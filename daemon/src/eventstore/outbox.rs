//! Transactional outbox (§7 / §12 / §15 / §17, DATA_MODEL §2.5).
//!
//! Delivery-intent rows are written **in the event-commit transaction** alongside
//! the event + its projections (the transactional-outbox pattern: a fact is never
//! recorded without its delivery intents, never delivered without being recorded),
//! and an async drainer delivers them at-least-once with backoff + retryable/terminal
//! classification + a dead-letter terminal. The outbox is the ONLY path an event
//! reaches an external destination.
//!
//! §15 sync sink: every outbox payload derives from the **already-redacted** event
//! (the writer never re-fetches raw); per-destination filtering only REMOVES fields,
//! so a secret in the source event reaches no outbox row.
//!
//! **Boundary (load-bearing, §7.2):** the outbox write happens ONLY in `append` (the
//! event-commit txn). `catch_up_replay` / `rebuild` reconstruct READ MODELS only and
//! MUST NOT re-emit outbox rows — a rebuild must never resurrect delivery intents
//! (no re-delivery of historical events). `outbox` is deliberately absent from
//! `projections::schema::REBUILD_TABLES`.

// L2 — drainer (drain_once / classify / backoff / dead-letter) + destinations.

use std::path::PathBuf;

use rusqlite::{params, Connection, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;

use crate::clock::Clock;
use crate::idgen::IdGen;

use super::EventStoreError;

/// The destinations subscribed to `event_type` — the daemon-internal routing map
/// (NOT a `shared/` contract). 1.3: `jsonl_mirror` mirrors every event; the real
/// adapters (brain_mcp/github/linear/notifier) re-home to their producing phases.
fn destinations_for(_event_type: &str) -> &'static [&'static str] {
    &["jsonl_mirror"]
}

/// The per-destination outbox payload, derived from the ALREADY-REDACTED event
/// (§15 sync sink — the writer never re-fetches raw). Per-destination filtering only
/// REMOVES fields, never re-introduces unredacted content, so a secret in the source
/// event reaches no row. 1.3: `jsonl_mirror` mirrors the whole redacted envelope.
fn build_payload(destination: &str, env: &EventEnvelope) -> Result<String, EventStoreError> {
    match destination {
        "jsonl_mirror" => {
            serde_json::to_string(env).map_err(|e| EventStoreError::Reconstruct(e.to_string()))
        }
        // unreachable in 1.3 (destinations_for + build_payload must agree); a mismatch
        // is an internal routing/config error, NOT a stored-row decode failure.
        other => Err(EventStoreError::Reconstruct(format!(
            "internal: outbox destination '{other}' has no payload builder"
        ))),
    }
}

/// Write one `pending` outbox row per subscribed destination, **in the caller's
/// append transaction** (transactional-outbox: recorded-iff-intended). Mints `out_`
/// ids + `created_at` from the injected seams. A `build_payload`/INSERT failure
/// propagates → the whole append (event + projections + outbox) rolls back together.
pub(crate) fn write_for_event(
    tx: &Transaction,
    env: &EventEnvelope,
    idgen: &dyn IdGen,
    clock: &dyn Clock,
) -> Result<(), EventStoreError> {
    let created_at = clock.now_rfc3339();
    for destination in destinations_for(&env.event_type) {
        let payload = build_payload(destination, env)?;
        tx.execute(
            "INSERT INTO outbox \
             (outbox_id, destination, event_id, payload_json, status, retry_count, created_at) \
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5)",
            params![
                idgen.new_outbox_id(),
                destination,
                env.event_id.as_str(),
                payload,
                created_at,
            ],
        )
        .map_err(EventStoreError::Write)?;
    }
    Ok(())
}

// ---- L2: drainer + destinations (§12 / §17) ---------------------------------

/// The result of one delivery attempt — the §17 classification that drives the
/// status machine: success, transient (back off + retry), or permanent (dead).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryOutcome {
    Delivered,
    /// transient (429 Retry-After / 5xx / transport) — back off + retry
    Retryable(String),
    /// permanent (401/403/4xx) — dead immediately, no retry
    Terminal(String),
}

/// An outbox delivery sink. The drainer is the ONLY caller of `deliver`, and only
/// over rows read from the `outbox` table — so the outbox is the sole external path
/// (the destination invariant, an INV-SEC-1 analogue for side-effects).
pub trait Destination: Send + Sync {
    /// the `outbox.destination` key this sink drains
    fn name(&self) -> &str;
    /// Deliver one already-redacted payload, classifying the result (§17). MUST be
    /// idempotent-friendly: at-least-once means a payload may be delivered twice.
    fn deliver(&self, payload: &str) -> DeliveryOutcome;
}

/// The one Phase-1 real destination: appends each (already-redacted) payload as a
/// line to a JSONL mirror file (§10.4) — a local audit/debug sink. Real adapters
/// (brain_mcp/github/linear/notifier) re-home to their producing phases.
pub struct JsonlMirror {
    path: PathBuf,
}

impl JsonlMirror {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Destination for JsonlMirror {
    fn name(&self) -> &str {
        "jsonl_mirror"
    }
    fn deliver(&self, payload: &str) -> DeliveryOutcome {
        use std::io::Write;
        // an IO error is transient (disk full / transient FS) → retryable, not dead.
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            Ok(mut f) => match writeln!(f, "{payload}") {
                Ok(()) => DeliveryOutcome::Delivered,
                Err(e) => DeliveryOutcome::Retryable(e.to_string()),
            },
            Err(e) => DeliveryOutcome::Retryable(e.to_string()),
        }
    }
}

/// max RETRIES before dead-letter (§17 bounded retry budget) — so MAX_RETRIES=5 is
/// 6 total deliver attempts (1 initial + 5 retries).
const MAX_RETRIES: i64 = 5;

/// exponential backoff (seconds), capped — the delay before the `attempt`-th retry
/// (called with `retry_count + 1`, so the first retry waits `backoff_secs(1)` = 60s).
fn backoff_secs(attempt: i64) -> i64 {
    let shift = attempt.clamp(0, 12) as u32;
    30i64.saturating_mul(1i64 << shift).min(3600)
}

/// Crash recovery (§17): reset any `in_flight` rows to `pending` on startup — a daemon
/// crash AFTER the in_flight claim but BEFORE delivery confirmation leaves the row
/// re-deliverable (at-least-once; idempotency is the destination's responsibility).
/// Single-instance-safe (the 1.4 pidlock guarantees one daemon at a time).
pub(crate) fn reset_in_flight(conn: &Connection) -> Result<(), EventStoreError> {
    conn.execute(
        "UPDATE outbox SET status='pending' WHERE status='in_flight'",
        [],
    )
    .map_err(EventStoreError::Write)?;
    Ok(())
}

/// Summary of one drain pass (for the Tokio loop's logging).
#[derive(Debug, Default, Clone, Copy)]
pub struct DrainSummary {
    pub delivered: usize,
    pub retried: usize,
    pub dead: usize,
}

/// Drain one pass for `dest`: select its due rows, attempt delivery, and transition
/// the status machine (§12/§17). The `in_flight` claim is committed BEFORE delivery
/// so a crash mid-delivery leaves the row re-deliverable (reset on the next open).
/// Single-threaded select-then-claim is fine under the one-drainer-per-destination
/// model; a concurrent drainer would need an atomic claim.
pub(crate) fn drain_once(
    conn: &Connection,
    clock: &dyn Clock,
    dest: &dyn Destination,
) -> Result<DrainSummary, EventStoreError> {
    let now = clock.now_rfc3339();
    // due = pending (next_attempt_at NULL) or failed whose backoff has elapsed.
    let due: Vec<(String, String, i64)> = {
        let mut stmt = conn
            .prepare(
                "SELECT outbox_id, payload_json, retry_count FROM outbox \
                 WHERE destination = ?1 AND status IN ('pending','failed') \
                   AND (next_attempt_at IS NULL OR next_attempt_at <= ?2) \
                 ORDER BY next_attempt_at",
            )
            .map_err(EventStoreError::Write)?;
        let rows = stmt
            .query_map(params![dest.name(), now], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(EventStoreError::Write)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(EventStoreError::Write)?);
        }
        out
    };

    let mut summary = DrainSummary::default();
    for (outbox_id, payload, retry_count) in due {
        // claim: commit `in_flight` BEFORE delivering (autocommit) so a crash leaves
        // the row stuck in_flight → reset to pending on the next open (at-least-once).
        conn.execute(
            "UPDATE outbox SET status='in_flight' WHERE outbox_id = ?1",
            params![outbox_id],
        )
        .map_err(EventStoreError::Write)?;

        match dest.deliver(&payload) {
            DeliveryOutcome::Delivered => {
                conn.execute(
                    "UPDATE outbox SET status='delivered', last_error=NULL WHERE outbox_id = ?1",
                    params![outbox_id],
                )
                .map_err(EventStoreError::Write)?;
                summary.delivered += 1;
            }
            DeliveryOutcome::Retryable(err) => {
                // saturating: a corrupt/huge retry_count must still dead-letter, never
                // wrap to a negative that retries forever.
                let next = retry_count.saturating_add(1);
                if next > MAX_RETRIES {
                    // bounded budget exhausted → dead-letter
                    conn.execute(
                        "UPDATE outbox SET status='dead', retry_count=?2, last_error=?3 \
                         WHERE outbox_id = ?1",
                        params![outbox_id, next, err],
                    )
                    .map_err(EventStoreError::Write)?;
                    summary.dead += 1;
                } else {
                    conn.execute(
                        "UPDATE outbox SET status='failed', retry_count=?2, \
                         next_attempt_at=?3, last_error=?4 WHERE outbox_id = ?1",
                        params![
                            outbox_id,
                            next,
                            clock.now_plus_secs(backoff_secs(next)),
                            err
                        ],
                    )
                    .map_err(EventStoreError::Write)?;
                    summary.retried += 1;
                }
            }
            DeliveryOutcome::Terminal(err) => {
                conn.execute(
                    "UPDATE outbox SET status='dead', last_error=?2 WHERE outbox_id = ?1",
                    params![outbox_id, err],
                )
                .map_err(EventStoreError::Write)?;
                summary.dead += 1;
            }
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    // a genuinely-internal helper the public API can't reach: the backoff curve.
    use super::backoff_secs;

    #[test]
    fn backoff_is_monotonic_and_capped() {
        // strictly increasing until the cap, then clamped at 3600s
        assert_eq!(backoff_secs(0), 30);
        assert_eq!(backoff_secs(1), 60);
        assert_eq!(backoff_secs(2), 120);
        assert!(backoff_secs(3) > backoff_secs(2));
        assert_eq!(
            backoff_secs(100),
            3600,
            "capped (no overflow on a large retry_count)"
        );
    }
}

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

use rusqlite::{params, Transaction};

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

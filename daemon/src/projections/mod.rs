//! The projection engine (§7 / §7.2 / DATA_MODEL §2.2–§2.4).
//!
//! Read models are folded from the append-only `events` log **in-band, inside the
//! event-commit transaction** ([`apply_all`], called from `EventStore::append`),
//! advancing each projector's `projection_offsets.last_seq` in that same txn — so a
//! reader never sees an event whose projections haven't applied, and an offset is
//! never ahead of the rows it represents (§2.4). Recovery (startup catch-up replay,
//! full rebuild, degraded-skip) lands in L3. Raw `events` are NEVER mutated by a
//! projector — projection corruption must not corrupt the spine (§7.2).
//!
//! L1 ships the [`Projector`] trait + the typed error; L2 adds the registry +
//! `apply_all` + the projector bodies, recovery in L3.

mod activity;
mod approval_queue;
mod audit;
mod graph;
mod object_refs;
mod pull_request;
mod schema;
mod session;
mod usage;
mod worktree;

use rusqlite::{params, Connection, Transaction, TransactionBehavior};
use serde::Serialize;

use nexusops_shared::event_envelope::EventEnvelope;

use crate::eventstore::DegradableEvent;

// edges-026 (P5.2 follow-on) — the §7.2 worktree live-read cache: the public computed-values type
// (the runtime layer produces it; tests construct it via `compute_worktree_cache`) + the crate-internal
// refresh UPDATE (`EventStore::refresh_worktree_status` drives it through the single writer).
pub(crate) use worktree::refresh_git_cache;
pub use worktree::WorktreeGitCache;

/// Typed projection failure. A `Decode` (a payload/enum that won't bind to its
/// frozen §5.1 shape) degrades the offending projector (§7.2); a `Db` is an
/// infrastructure failure that fails the append closed.
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("projection db error: {0}")]
    Db(#[from] rusqlite::Error),
    /// payload / status value did not bind to its frozen contract shape (§5.1/§15
    /// reject-unknown). The reason MUST NOT echo (possibly sensitive) payload bytes.
    #[error("projection decode failed: {0}")]
    Decode(String),
    /// an offset write touched != 1 row — the projector's offset row is missing or
    /// duplicated (a §2.4 invariant break). Fail-closed: a silently-missing offset
    /// row would leave `last_seq` un-advanced, so catch-up re-folds it every reopen
    /// and the non-idempotent counters double-count.
    #[error("offset anomaly for {projector}: {rows} rows touched")]
    OffsetAnomaly { projector: String, rows: usize },
}

/// A single read-model fold. `apply` writes its rows for `env` **within the caller's
/// transaction** (the engine owns offset advancement + degraded-skip around it). A
/// projector NEVER touches the raw `events` table and is idempotent under replay
/// (upsert / INSERT-OR-IGNORE), so a rebuild reproduces identical state (§7.2).
pub trait Projector {
    /// the `projection_offsets.projection_name` key (stable; offsets bind to it)
    fn name(&self) -> &'static str;

    /// Fold `env` into this projector's read tables, in `tx`. Returning `Err`
    /// degrades this projector for this event (the engine handles the skip);
    /// it must not have committed partial rows the engine can't roll back.
    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError>;
}

/// The registered projectors, in apply order. **`object_refs` is first**: the graph
/// projector folds from the `object_refs` rows it writes (same txn). 1.2 ships the
/// Phase-1-feedable bodies; the later-phase projectors get their bodies with the
/// phase that emits their feeding events (their tables already exist, migration 3).
/// **`approval_queue` (2.1c)** folds the Gateway's approval events (the Human Input
/// Queue read model) — order-independent of the others (it sibling-reads the
/// `action_requests`/`approvals` registry tables, not another projection).
fn projectors() -> Vec<Box<dyn Projector>> {
    vec![
        Box::new(object_refs::ObjectRefsProjector),
        Box::new(session::SessionProjector),
        Box::new(graph::GraphProjector),
        Box::new(audit::AuditProjector),
        Box::new(activity::ActivityProjector),
        Box::new(approval_queue::ApprovalQueueProjector),
        // P3.1 — folds the §9.1 TelemetrySampled observation event into per-day usage rollups
        // (§18). Wired NOW; its branch fires once the 3.2/3.3 adapters emit the feeding events.
        Box::new(usage::UsageLedgerProjector),
        // P5.2 (edges-022) — folds the edges git.create_worktree WorktreeCreated event into the
        // proj_worktree read model (sibling-reads action_requests for repo_id, LESSON 17).
        Box::new(worktree::WorktreeProjector),
        // P7.1 (edges-025) — folds the edges github.create_pr PullRequestSynced event into the
        // proj_pull_request read cache (§7.2; sibling-reads action_requests for repo_id, LESSON 17;
        // pr_id = the {repo_id}#{pr_number} rebuild-safe composite). Closes the github read vertical.
        Box::new(pull_request::PullRequestProjector),
    ]
}

/// Fold `env` into every registered read model, **in the caller's transaction** (the
/// event-commit txn for the in-band path). This is the §7 fan-out: one event updates
/// multiple projections atomically. A `Db` error fails the append closed; a projector
/// logic failure is contained (degraded-skip), never propagated.
pub fn apply_all(tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
    for p in projectors() {
        apply_one(tx, p.as_ref(), env)?;
    }
    Ok(())
}

/// Apply one projector, owning offset advancement + degraded-skip. The fold runs
/// inside a SAVEPOINT so a logic failure rolls its partial rows AND its (un-advanced)
/// offset back together — the offset is never left ahead of the rows it represents
/// (§2.4). Success advances `last_seq` to `env.seq`; a logic failure (or an event
/// newer than this binary understands) marks the projector `degraded` (sticky) and
/// SKIPS without advancing, so a later event is still processed (§7.2). A `Db` error
/// is infrastructure failure → propagated (the append fails closed, §15/§17).
pub(crate) fn apply_one(
    tx: &Transaction,
    p: &dyn Projector,
    env: &EventEnvelope,
) -> Result<(), ProjectionError> {
    ensure_offset_row(tx, p.name())?;

    if env.event_version as i64 > crate::eventstore::MAX_SUPPORTED_EVENT_VERSION {
        mark_degraded(tx, p.name())?; // unfoldable by this binary → degrade, skip
        return Ok(());
    }

    // a per-projector savepoint (quoted identifier; the name is a fixed-registry
    // `&'static str`) brackets BOTH the projector's rows and its offset advance, so a
    // failure rolls them back together (offset never ahead of rows). Unique names
    // remove any reliance on release-before-reuse balancing across the loop.
    let name = p.name();
    tx.execute_batch(&format!("SAVEPOINT \"{name}\""))?;
    match p.apply(tx, env) {
        Ok(()) => {
            advance_offset(tx, name, env)?;
            tx.execute_batch(&format!("RELEASE \"{name}\""))?;
            Ok(())
        }
        Err(ProjectionError::Decode(_)) => {
            tx.execute_batch(&format!("ROLLBACK TO \"{name}\"; RELEASE \"{name}\""))?;
            mark_degraded(tx, name)?;
            Ok(())
        }
        // a Db / OffsetAnomaly (infrastructure or integrity) error fails closed: roll
        // the projector back and propagate so the whole append/replay txn aborts.
        Err(e) => {
            tx.execute_batch(&format!("ROLLBACK TO \"{name}\"; RELEASE \"{name}\""))?;
            Err(e)
        }
    }
}

/// lazily create a projector's offset row (the registry is the source of truth for
/// which projectors exist; offsets seed at `last_seq=0, healthy`).
fn ensure_offset_row(tx: &Transaction, name: &str) -> Result<(), ProjectionError> {
    tx.execute(
        "INSERT OR IGNORE INTO projection_offsets (projection_name, last_seq, state) \
         VALUES (?1, 0, 'healthy')",
        params![name],
    )?;
    Ok(())
}

/// advance the cursor in the SAME txn as the rows (§2.4). A projector already flagged
/// `degraded` STAYS degraded (sticky — a rebuild heals it) even as it folds later events.
fn advance_offset(
    tx: &Transaction,
    name: &str,
    env: &EventEnvelope,
) -> Result<(), ProjectionError> {
    let rows = tx.execute(
        "UPDATE projection_offsets SET \
           last_event_id = ?2, last_seq = ?3, last_processed_at = ?4, \
           state = CASE WHEN state = 'degraded' THEN 'degraded' ELSE 'healthy' END \
         WHERE projection_name = ?1",
        params![name, env.event_id.as_str(), env.seq, env.recorded_at],
    )?;
    expect_one_row(name, rows)
}

fn mark_degraded(tx: &Transaction, name: &str) -> Result<(), ProjectionError> {
    let rows = tx.execute(
        "UPDATE projection_offsets SET state = 'degraded' WHERE projection_name = ?1",
        params![name],
    )?;
    expect_one_row(name, rows)
}

/// Fail-closed if an offset write didn't touch exactly one row — a missing offset
/// row would silently strand `last_seq`, re-folding the log on every reopen (and
/// double-counting the non-idempotent counters). `ensure_offset_row` runs first, so
/// this is structurally unreachable in normal flow — it backstops a regression.
fn expect_one_row(projector: &str, rows: usize) -> Result<(), ProjectionError> {
    if rows == 1 {
        Ok(())
    } else {
        Err(ProjectionError::OffsetAnomaly {
            projector: projector.to_string(),
            rows,
        })
    }
}

/// A contract enum's canonical snake_case wire string — fail-closed: a value that
/// won't render to a JSON string is a `Decode` error, never a silent empty column.
pub(crate) fn wire_value<T: Serialize>(v: &T) -> Result<String, ProjectionError> {
    match serde_json::to_value(v).map_err(|e| ProjectionError::Decode(e.to_string()))? {
        serde_json::Value::String(s) => Ok(s),
        _ => Err(ProjectionError::Decode(
            "enum did not serialize to a string".into(),
        )),
    }
}

// ---- recovery: startup catch-up replay + full rebuild (§7.2) -----------------

/// Startup catch-up: fold every event a projector lags behind (`seq > its last_seq`),
/// per projector, in one txn. The strict `>` makes a current log a NO-OP — idempotent
/// on restart, so the non-idempotent counters never double-count. A crash mid-write
/// heals: the un-advanced offset re-folds its tail on the next open.
pub fn catch_up_replay(conn: &mut Connection) -> Result<(), ProjectionError> {
    replay(conn, false)
}

/// Full rebuild: truncate every derived table + drop offsets, then replay the entire
/// log from scratch. Raw `events` are untouched (§7.2 — corruption of a projection must
/// not corrupt the spine). Deterministic: the result is identical to the incremental
/// fold of the same log (rebuild-equivalence).
pub fn rebuild(conn: &mut Connection) -> Result<(), ProjectionError> {
    replay(conn, true)
}

fn replay(conn: &mut Connection, full_rebuild: bool) -> Result<(), ProjectionError> {
    let registry = projectors();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if full_rebuild {
        // table names are fixed compile-time constants (no injection surface)
        for table in schema::REBUILD_TABLES {
            tx.execute_batch(&format!("DELETE FROM {table}"))?;
        }
        tx.execute_batch("DELETE FROM projection_offsets")?;
    }
    // each projector replays its OWN pending tail independently. `object_refs` is first,
    // so by the time `graph` replays, the rows it folds from are all present — identical
    // to the in-band interleave (graph reads only the current event's refs, never others').
    // §17 (Option C): the tail is read DEGRADABLY — a healthy row folds; a Degraded (unknown
    // version) row marks the projector degraded + continues; a Quarantined (corrupt/unredacted)
    // row marks degraded AND records the quarantine (→ L2 audit-integrity event) + continues.
    // A skipped row never advances the offset (a later healthy row advances past it; a trailing
    // skipped row is re-read+re-recorded idempotently). `open` never aborts on a bad row.
    for p in &registry {
        ensure_offset_row(&tx, p.name())?;
        let from = offset_last_seq(&tx, p.name())?;
        let pending =
            crate::eventstore::read_events_after_degradable(&tx, from).map_err(replay_read_err)?;
        for de in &pending {
            match de {
                DegradableEvent::Ok(env) => apply_one(&tx, p.as_ref(), env)?,
                // unknown version → degrade quietly (no integrity violation, no quarantine).
                DegradableEvent::Degraded { .. } => mark_degraded(&tx, p.name())?,
                // corrupt/unredacted → degrade AND record the quarantine (the loud §17 gap).
                DegradableEvent::Quarantined { seq, reason } => {
                    mark_degraded(&tx, p.name())?;
                    record_quarantine(&tx, *seq, reason)?;
                }
            }
        }
    }
    tx.commit()?;
    Ok(())
}

/// Record a quarantined row (§17 Option C) — preserve + mark the gap for forensics + the L2
/// audit-integrity emission. **`ON CONFLICT(seq) DO NOTHING`** (never REPLACE): re-detection (a
/// catch-up re-read of a trailing row, or a full `rebuild` whose offsets reset) must NOT reset
/// `audit_emitted`, so the L2 event emits exactly once per seq. `detected_at` is a DB-side
/// timestamp — forensic only; `quarantine` is daemon-internal and not replay-deterministic.
fn record_quarantine(tx: &Transaction, seq: i64, reason: &str) -> Result<(), ProjectionError> {
    tx.execute(
        "INSERT INTO quarantine (seq, reason, detected_at) \
         VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')) \
         ON CONFLICT(seq) DO NOTHING",
        params![seq, reason],
    )?;
    Ok(())
}

fn offset_last_seq(tx: &Transaction, name: &str) -> Result<i64, ProjectionError> {
    Ok(tx.query_row(
        "SELECT last_seq FROM projection_offsets WHERE projection_name = ?1",
        params![name],
        |r| r.get(0),
    )?)
}

/// Map a store read error from the replay source onto the projection error.
fn replay_read_err(e: crate::eventstore::EventStoreError) -> ProjectionError {
    match e {
        crate::eventstore::EventStoreError::Write(err) => ProjectionError::Db(err),
        other => ProjectionError::Decode(other.to_string()),
    }
}

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
//! L1 ships the [`Projector`] trait + the typed error; the registry + `apply_all`
//! wiring + the projector bodies land in L2, recovery in L3.

use rusqlite::Transaction;

use nexusops_shared::event_envelope::EventEnvelope;

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

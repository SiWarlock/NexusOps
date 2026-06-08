//! `object_refs` projector (§2.2) — the normalized event→object edges that back the
//! ProjectGraph + every per-object timeline.
//!
//! Refs are **derived from the event** (its typed identity fields + type), NOT taken
//! from a caller-supplied list: §2.2 normalizes refs OUT of `events` and a rebuild
//! truncates this table, so the refs MUST be reconstructable from `events` alone.
//! Deriving makes the append path and the rebuild path produce identical rows. As the
//! event-type registry grows, each type contributes its own derivation (refs that
//! aren't envelope columns come from `payload_json`); 1.2 folds only `SessionStarted`.

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;

use super::{ProjectionError, Projector};

pub struct ObjectRefsProjector;

impl Projector for ObjectRefsProjector {
    fn name(&self) -> &'static str {
        "object_refs"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        for (object_type, object_id) in derive_refs(env) {
            // INSERT OR IGNORE → idempotent under replay/rebuild (same PK, no conflict).
            tx.execute(
                "INSERT OR IGNORE INTO object_refs (event_id, object_type, object_id) \
                 VALUES (?1, ?2, ?3)",
                params![env.event_id.as_str(), object_type, object_id],
            )?;
        }
        Ok(())
    }
}

/// The edges inherent to `env`, derived from its typed fields (rebuildable from the
/// stored event). 1.2: a `SessionStarted` relates its session + its project.
fn derive_refs(env: &EventEnvelope) -> Vec<(&'static str, String)> {
    let mut refs = Vec::new();
    if env.event_type == "SessionStarted" {
        if let Some(s) = &env.session_id {
            refs.push(("session", s.as_str().to_string()));
        }
        if let Some(p) = &env.project_id {
            refs.push(("project", p.as_str().to_string()));
        }
    }
    refs
}

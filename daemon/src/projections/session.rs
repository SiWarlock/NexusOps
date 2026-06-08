//! `proj_session` projector (§2.3) — the derived current state of every session.
//!
//! Identity comes from the envelope's typed fields; the type-specific attributes come
//! from the `SessionStarted` payload. `status` **binds to the frozen §5.1 `Session`
//! machine** — an unknown wire value fails closed (`ProjectionError::Decode` →
//! the engine degrades this projector, §15 reject-unknown), it is never stored raw.

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::SessionStarted;

use super::{wire_value, ProjectionError, Projector};

pub struct SessionProjector;

impl Projector for SessionProjector {
    fn name(&self) -> &'static str {
        "session"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        if env.event_type != "SessionStarted" {
            return Ok(()); // 1.2 folds only SessionStarted (lifecycle events → Phase 3)
        }
        // a SessionStarted without identity is not projectable — skip (healthy no-op,
        // not a degrade): proj_session.session_id/project_id are NOT NULL.
        let (Some(session_id), Some(project_id)) = (&env.session_id, &env.project_id) else {
            return Ok(());
        };

        // reject-unknown: the reason MUST NOT echo payload bytes (§15) — generic text.
        let payload: SessionStarted = serde_json::from_str(&env.payload_json)
            .map_err(|_| ProjectionError::Decode("SessionStarted payload did not bind".into()))?;
        let status = wire_value(&payload.status)?; // §5.1 Session enum → canonical wire string

        tx.execute(
            "INSERT INTO proj_session \
             (session_id, project_id, status, harness, model, display_name, started_at, updated_at_seq) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(session_id) DO UPDATE SET \
               status=excluded.status, harness=excluded.harness, model=excluded.model, \
               display_name=excluded.display_name, updated_at_seq=excluded.updated_at_seq",
            params![
                session_id.as_str(),
                project_id.as_str(),
                status,
                payload.harness,
                payload.model,
                payload.display_name,
                env.occurred_at,
                env.seq,
            ],
        )?;
        Ok(())
    }
}

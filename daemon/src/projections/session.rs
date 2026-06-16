//! `proj_session` projector (§2.3) — the derived current state of every session.
//!
//! Identity comes from the envelope's typed fields; the type-specific attributes come
//! from the `SessionStarted` payload. `status` **binds to the frozen §5.1 `Session`
//! machine** — an unknown wire value fails closed (`ProjectionError::Decode` →
//! the engine degrades this projector, §15 reject-unknown), it is never stored raw.

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::{SessionFailed, SessionRecovered, SessionStarted};
use nexusops_shared::status::Session;

use super::{wire_value, ProjectionError, Projector};

pub struct SessionProjector;

impl Projector for SessionProjector {
    fn name(&self) -> &'static str {
        "session"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        // 4.2 — a §17 supervised-child death: fold the row to status=Failed (+completed_at). The status
        // is derived from the EVENT TYPE (SessionFailed → Session::Failed), never a row's current value,
        // so a rebuild re-derives it identically (LESSON §17 rebuild-safety). A SessionFailed for an
        // unknown session is a healthy no-op (UPDATE affects 0 rows) — never a degrade.
        if env.event_type == SessionFailed::EVENT_TYPE {
            let Some(session_id) = &env.session_id else {
                return Ok(());
            };
            tx.execute(
                "UPDATE proj_session SET status=?1, completed_at=?2, updated_at_seq=?3 \
                 WHERE session_id=?4",
                params![
                    wire_value(&Session::Failed)?,
                    env.occurred_at,
                    env.seq,
                    session_id.as_str(),
                ],
            )?;
            return Ok(());
        }
        // D2/P4.4 — the §8.1/§11.4 survival-recovery DISPLAY fold: a (previously-dead) SessionRecovered
        // (emitted by 4.1b-1's recover_sessions_on_restart) records the recovery OUTCOME on the row. The
        // fields derive from the EVENT payload (rebuild-safe, LESSON §17). It does NOT change `status`
        // (owned by SessionStarted/SessionFailed — a relaunch re-emits SessionStarted; the recovery
        // fields are the §11.4 banner source, orthogonal). An unknown ResumeMode wire value fails the
        // payload bind → Decode (the projector degrades + skips; never stored raw — the SessionStarted
        // reject-unknown precedent). A SessionRecovered for an unknown session is a healthy no-op (UPDATE
        // affects 0 rows).
        if env.event_type == SessionRecovered::EVENT_TYPE {
            let Some(session_id) = &env.session_id else {
                return Ok(());
            };
            let payload: SessionRecovered =
                serde_json::from_str(&env.payload_json).map_err(|_| {
                    ProjectionError::Decode("SessionRecovered payload did not bind".into())
                })?;
            // fail-closed at the FOLD boundary (not silent-truncate-then-fail-at-read): an
            // unrepresentable count degrades here rather than storing a wrapped negative.
            let replayed = i64::try_from(payload.replayed_event_count).map_err(|_| {
                ProjectionError::Decode("replayed_event_count overflows i64".into())
            })?;
            // NB: the payload's `execution_profile_id` (§15 #8 — preserved-from-the-original-SessionStarted,
            // no account-hop) is INTENTIONALLY not folded: proj_session.execution_profile_id is already set
            // (identically) by that original SessionStarted, so the event's copy is the AUDIT record, not a
            // projection input. The fold writes ONLY the recovery metadata; `status` stays owned by
            // SessionStarted/SessionFailed (a relaunch re-emits SessionStarted).
            tx.execute(
                "UPDATE proj_session SET resume_mode=?1, replayed_event_count=?2, recovered_at=?3, \
                 updated_at_seq=?4 WHERE session_id=?5",
                params![
                    wire_value(&payload.mode)?, // §8.1 ResumeMode → canonical wire string
                    replayed,
                    env.occurred_at,
                    env.seq,
                    session_id.as_str(),
                ],
            )?;
            return Ok(());
        }
        if env.event_type != "SessionStarted" {
            return Ok(()); // 1.2 folds SessionStarted; 4.2 folds SessionFailed; D2 folds SessionRecovered
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

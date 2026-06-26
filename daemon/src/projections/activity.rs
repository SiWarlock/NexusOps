//! `proj_project_activity` projector (§2.3) — the per-project sidebar/Command-Center
//! counters.
//!
//! **091 — folds TWO event types:** `SessionStarted` (the counter) and `ProjectRescanned`
//! (ensure-row-exists, counters untouched) so a session-less freshly-rescanned project
//! surfaces in the cockpit grid (the add-project "Scanning… forever" fix; the row+nudge
//! is inert until the UI reads it — ui-082).
//!
//! **Increment-only until Phase 3.** 1.2 folds only `SessionStarted`, so this slice
//! can only raise `active_sessions`; the decrements (active→completed/failed/idle)
//! arrive with the session-lifecycle events in Phase 3. This is partial-by-design,
//! not half-finished. Unlike the upsert / INSERT-OR-IGNORE projectors this counter
//! is NOT idempotent under a re-fold, so re-applying a `SessionStarted` would
//! double-count. The in-band append path never re-folds (each event is appended
//! once); the recovery path's strict `seq > last_seq` replay guard (L3) is what
//! keeps a restart/catch-up from re-incrementing (pinned by the catch-up no-op test).

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::ProjectRescanned;

use super::{ProjectionError, Projector};

pub struct ActivityProjector;

impl Projector for ActivityProjector {
    fn name(&self) -> &'static str {
        "project_activity"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        // 091: the projector now folds TWO event types — SessionStarted (increment active_sessions) and
        // ProjectRescanned (ensure the row EXISTS so a session-less rescanned project enters the cockpit
        // grid). Both key on the envelope project_id; any other event (or a project_id-less one) is a no-op.
        let is_session_started = env.event_type == "SessionStarted";
        let is_rescanned = env.event_type == ProjectRescanned::EVENT_TYPE;
        if !is_session_started && !is_rescanned {
            return Ok(());
        }
        let Some(project_id) = &env.project_id else {
            return Ok(());
        };
        if is_session_started {
            tx.execute(
                "INSERT INTO proj_project_activity (project_id, active_sessions, updated_at_seq) \
                 VALUES (?1, 1, ?2) \
                 ON CONFLICT(project_id) DO UPDATE SET \
                   active_sessions = active_sessions + 1, updated_at_seq = excluded.updated_at_seq",
                params![project_id.as_str(), env.seq],
            )?;
        } else if is_rescanned {
            // ProjectRescanned — ensure the row exists (counters default 0 via the DDL); ON CONFLICT touches
            // ONLY updated_at_seq so a re-rescan after sessions started NEVER resets active_sessions.
            tx.execute(
                "INSERT INTO proj_project_activity (project_id, updated_at_seq) \
                 VALUES (?1, ?2) \
                 ON CONFLICT(project_id) DO UPDATE SET updated_at_seq = excluded.updated_at_seq",
                params![project_id.as_str(), env.seq],
            )?;
        }
        Ok(())
    }
}

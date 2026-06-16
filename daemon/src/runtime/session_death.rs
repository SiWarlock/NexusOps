//! P4.2 (brief 061) — the PRODUCTION supervised-child-death sink: bind the supervisor's `Failed` reap to
//! the single write-actor as a `SessionFailed` OBSERVATION event (§17 child-death surface head).
//!
//! A `SessionFailed` is a non-mutation OBSERVATION event (the daemon WITNESSES a child dying) — written
//! via the write-actor through the §15 redaction gate, **NOT the Gateway** (a death notification is not
//! a state mutation; INV-SEC-1 governs mutations; the `SessionRecovered`/`TerminalProcessExited`
//! precedent, LESSON §10/§23). Appended **fire-and-forget** (`try_append_observation`: soft-degrade,
//! never back-pressures the writer — a dropped death-event is a stale §11.4 bit, never a safety fault;
//! the `proj_session` row stays at its pre-death status until a re-emit / rebuild). Constructed in
//! `main.rs` (where the `WriteHandle` lives) + injected into the supervisor via the opaque
//! [`SessionDeathSink`] trait, so `session/` never imports `WriteHandle` (the cat-1 boundary, LESSON 28).

use std::sync::{Arc, OnceLock};

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
use nexusops_shared::events::SessionFailed;
use nexusops_shared::ids::{SessionId, WorkspaceId};

use crate::clock::Clock;
use crate::eventstore::AppendIntent;
use crate::runtime::WriteHandle;
use crate::session::SessionDeathSink;

/// The PRODUCTION [`SessionDeathSink`] — appends a `SessionFailed` OBSERVATION event via the single
/// write-actor (System-actor; the daemon's own §17 lifecycle observation, the `system_intent`/
/// `WriteActorRecoverySink` precedent; NOT the Gateway). `occurred_at` = the daemon-`Clock` UTC-Z; the
/// session identity rides the envelope; empty payload (the death fact suffices, Q1=(a)). The
/// `WriteHandle` sits behind a deferred `OnceLock` — the supervisor (holding this sink) is spawned
/// BEFORE the write-actor exists (the bootstrap cycle the telemetry factory also solves), so `main.rs`
/// [`bind`](SessionDeathHandleSlot::bind)s the handle once the actor is up.
pub struct WriteActorSessionDeathSink {
    handle: Arc<OnceLock<WriteHandle>>,
    clock: Arc<dyn Clock>,
}

impl WriteActorSessionDeathSink {
    /// Construct with the handle ALREADY available (tests + any non-cyclic caller) — a pre-filled cell.
    pub fn new(handle: WriteHandle, clock: Arc<dyn Clock>) -> Self {
        let cell = Arc::new(OnceLock::new());
        let _ = cell.set(handle);
        Self {
            handle: cell,
            clock,
        }
    }

    /// Construct DEFERRED for the production bootstrap cycle — returns the sink + the
    /// [`SessionDeathHandleSlot`] `main.rs` `bind`s the write-actor handle into once it exists.
    pub fn deferred(clock: Arc<dyn Clock>) -> (Self, SessionDeathHandleSlot) {
        let cell = Arc::new(OnceLock::new());
        (
            Self {
                handle: cell.clone(),
                clock,
            },
            SessionDeathHandleSlot(cell),
        )
    }

    /// Build the `SessionFailed` observation [`AppendIntent`] — `actor_type=System` (the daemon WITNESSES
    /// the death, the `system_intent` precedent), the session identity on the envelope, `occurred_at` =
    /// the daemon-`Clock` UTC-Z. Empty payload (`{}`).
    fn intent(&self, session_id: &SessionId, payload_json: String) -> AppendIntent {
        let sid = session_id.as_str().to_string();
        AppendIntent {
            event_type: SessionFailed::EVENT_TYPE.to_string(),
            event_version: 1,
            occurred_at: self.clock.now_rfc3339(),
            workspace_id: WorkspaceId::system(),
            actor_type: ActorType::System,
            // System-actor: the session_id stands in for actor/source/correlation (no external actor;
            // the `system_intent`/`WriteActorRecoverySink` convention).
            actor_id: sid.clone(),
            source_type: SourceType::LocalDaemon,
            source_id: sid.clone(),
            correlation_id: sid,
            sensitivity: Sensitivity::Internal,
            payload_json,
            schema_version: "event-envelope-v1".to_string(),
            idempotency_key: None,
            project_id: None,
            session_id: Some(session_id.clone()),
            agent_team_id: None,
            visibility: Some(Visibility::System),
            action_request_id: None,
            approval_id: None,
            causation_id: None,
        }
    }
}

impl SessionDeathSink for WriteActorSessionDeathSink {
    fn emit_failed(&self, session_id: &SessionId) {
        // resolve the bound handle; if unbound (a death before `bind` — not the production order), fall
        // back to a DISCONNECTED handle so the emit soft-degrades (the event is dropped) rather than
        // panicking (LESSON §30; the telemetry-factory precedent).
        let handle = self
            .handle
            .get()
            .cloned()
            .unwrap_or_else(WriteHandle::disconnected);
        // serialize the (empty) payload; a serialize failure (not expected today, but additive fields
        // could fail later) LOGS + drops the event rather than substituting a stale payload — the
        // WriteActorRecoverySink precedent (never a silent mis-record).
        let payload_json = match serde_json::to_string(&SessionFailed {}) {
            Ok(j) => j,
            Err(e) => {
                eprintln!(
                    "nexusopsd: child-death surface — dropped a SessionFailed (serialize failed): {e}"
                );
                return;
            }
        };
        // fire-and-forget: never back-pressures the writer (LESSON §9/§30; non-safety observation).
        let _ = handle.try_append_observation(self.intent(session_id, payload_json));
    }
}

/// Binds the write-actor [`WriteHandle`] into a deferred [`WriteActorSessionDeathSink`] — breaks the
/// bootstrap CYCLE: the supervisor (holding the sink) is spawned BEFORE the write-actor exists. `main.rs`
/// holds this slot + [`bind`](Self::bind)s the handle once the actor is up (well before any session can
/// reach a `Failed` reap). Shared with the sink via `Arc<OnceLock>`.
pub struct SessionDeathHandleSlot(Arc<OnceLock<WriteHandle>>);

impl SessionDeathHandleSlot {
    /// Supply the write-actor handle once it exists. Idempotent (a second `bind` is a no-op — `OnceLock`).
    pub fn bind(&self, handle: WriteHandle) {
        let _ = self.0.set(handle);
    }
}

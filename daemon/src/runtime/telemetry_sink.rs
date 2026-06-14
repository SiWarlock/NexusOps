//! P4.0c (brief 056) — the PRODUCTION telemetry sink: bind the Claude adapter's `TelemetryEventSink`
//! to the single write-actor.
//!
//! A `TelemetrySampled` is a non-mutation OBSERVATION event (write-actor, NOT the Gateway — LESSON
//! §23/§27; INV-SEC-1 routes *mutations*, a usage observation is none), appended **fire-and-forget**
//! (`try_append_observation`: soft-degrade, LESSON §9/§30 — never back-pressures the writer; a dropped
//! sample is a stale meter, never a safety fault). Constructed in `main.rs` (where the `WriteHandle`
//! lives) + injected into the `ClaudeAdapter` via the launcher's opaque [`TelemetrySinkFactory`], so
//! `session/` never imports `WriteHandle` (the cat-1 boundary, LESSON §28).

use std::sync::{Arc, OnceLock};

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};

use crate::clock::Clock;
use crate::eventstore::AppendIntent;
use crate::harness::claude::telemetry::{TelemetryEventSink, TelemetrySinkFactory};
use crate::runtime::WriteHandle;

/// The production [`TelemetryEventSink`] — appends each emitted [`TelemetrySampled`] as a non-mutation
/// OBSERVATION event via the single write-actor (`try_append_observation`: fire-and-forget, §15-gated;
/// the §7.1/§9.1 path). One sink per session: the session/project identity rides the envelope (the
/// `proj_usage_ledger` buckets by them); `occurred_at` is the injected daemon-`Clock` UTC-Z (gates
/// `bucket_day`, LESSON §27/§5). `Send` (Q6) — owned by the one drive-loop thread (`WriteHandle` is
/// `Clone + Send`; `Arc<dyn Clock>` is `Send + Sync`).
pub struct WriteActorTelemetrySink {
    handle: WriteHandle,
    session_id: SessionId,
    project_id: Option<ProjectId>,
    clock: Arc<dyn Clock>,
}

impl WriteActorTelemetrySink {
    pub fn new(
        handle: WriteHandle,
        session_id: SessionId,
        project_id: Option<ProjectId>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            handle,
            session_id,
            project_id,
            clock,
        }
    }

    /// Build the `TelemetrySampled` observation [`AppendIntent`] — `actor_type=SessionAdapter` (the
    /// harness adapter WITNESSES usage), `source_type=UsageMeter`, the session/project identity on the
    /// envelope, `occurred_at` = the daemon-`Clock` UTC-Z (the `telemetry_intent`/`system_intent`
    /// precedent). `workspace_id` = the reserved system sentinel (the projector buckets by session/
    /// project, NOT workspace; threading the session's real workspace is a P5 follow-on).
    fn intent(&self, payload_json: String) -> AppendIntent {
        let sid = self.session_id.as_str().to_string();
        AppendIntent {
            event_type: "TelemetrySampled".to_string(),
            event_version: 1,
            occurred_at: self.clock.now_rfc3339(),
            workspace_id: WorkspaceId::system(),
            actor_type: ActorType::SessionAdapter,
            actor_id: sid.clone(),
            source_type: SourceType::UsageMeter,
            source_id: sid.clone(),
            correlation_id: sid,
            sensitivity: Sensitivity::Internal,
            payload_json,
            schema_version: "event-envelope-v1".to_string(),
            idempotency_key: None,
            project_id: self.project_id.clone(),
            session_id: Some(self.session_id.clone()),
            agent_team_id: None,
            visibility: None,
            action_request_id: None,
            approval_id: None,
            causation_id: None,
        }
    }
}

impl TelemetryEventSink for WriteActorTelemetrySink {
    fn emit_telemetry(&self, event: TelemetrySampled) {
        // serialize the payload; a serialize failure (not expected for TelemetrySampled) drops the
        // sample (soft-degrade, non-safety — never panics the drive loop).
        let Ok(payload_json) = serde_json::to_string(&event) else {
            return;
        };
        // fire-and-forget: never back-pressures the writer; a full channel drops the sample (LESSON §30).
        let _ = self
            .handle
            .try_append_observation(self.intent(payload_json));
    }
}

/// Binds the write-actor [`WriteHandle`] into a deferred factory — breaks the production bootstrap
/// CYCLE: the `PtyLauncher` (which holds the factory) is consumed into the `SessionExecutor` →
/// `Gateway` → `WriteActor` BEFORE the handle is born, so the factory can't be constructed with a live
/// handle. `main.rs` holds this slot (a clone of the factory's shared cell) + [`bind`](Self::bind)s the
/// handle once the write-actor exists — well before the accept-loop serves any `session.create`
/// (→ `make_sink`). Shared with the factory via `Arc<OnceLock>`.
pub struct TelemetryHandleSlot(Arc<OnceLock<WriteHandle>>);

impl TelemetryHandleSlot {
    /// Supply the write-actor handle once it exists. Idempotent (a second `bind` is a no-op — `OnceLock`).
    /// A sink minted (via `make_sink`) BEFORE `bind` snapshots a disconnected handle PERMANENTLY — it
    /// does NOT retroactively upgrade once the slot is bound — so it drops every sample (soft-degrade,
    /// never a panic; LESSON §30). In the production order `bind` runs immediately after the write-actor
    /// spawns, BEFORE the accept-loop serves any `session.create` → `make_sink`, so no live session's
    /// sink is ever minted disconnected.
    pub fn bind(&self, handle: WriteHandle) {
        let _ = self.0.set(handle);
    }
}

/// The production [`TelemetrySinkFactory`] (4.0c) — `main.rs` builds ONE (over the write-actor
/// `WriteHandle` + the daemon `Clock`) and hands it to the `PtyLauncher`; the launcher mints a
/// per-session [`WriteActorTelemetrySink`] for each launched session. The launcher holds it as an
/// opaque `Box<dyn TelemetrySinkFactory>` → `session/` never imports `WriteHandle` (cat-1).
pub struct WriteActorTelemetrySinkFactory {
    handle: Arc<OnceLock<WriteHandle>>,
    clock: Arc<dyn Clock>,
}

impl WriteActorTelemetrySinkFactory {
    /// Construct with the handle ALREADY available (tests + any non-cyclic caller) — a pre-filled cell.
    pub fn new(handle: WriteHandle, clock: Arc<dyn Clock>) -> Self {
        let cell = Arc::new(OnceLock::new());
        let _ = cell.set(handle);
        Self {
            handle: cell,
            clock,
        }
    }

    /// Construct DEFERRED for the production bootstrap cycle — returns the factory + the
    /// [`TelemetryHandleSlot`] `main.rs` `bind`s the handle into once the write-actor exists.
    pub fn deferred(clock: Arc<dyn Clock>) -> (Self, TelemetryHandleSlot) {
        let cell = Arc::new(OnceLock::new());
        let factory = Self {
            handle: cell.clone(),
            clock,
        };
        (factory, TelemetryHandleSlot(cell))
    }
}

impl TelemetrySinkFactory for WriteActorTelemetrySinkFactory {
    fn make_sink(
        &self,
        session_id: &SessionId,
        project_id: Option<&ProjectId>,
    ) -> Box<dyn TelemetryEventSink> {
        // resolve the bound handle; if unbound (a session somehow launched before bind — not the
        // production order), fall back to a DISCONNECTED handle so the sink soft-degrades (every
        // append fails → sample dropped) rather than panicking (LESSON §30).
        let handle = self
            .handle
            .get()
            .cloned()
            .unwrap_or_else(WriteHandle::disconnected);
        Box::new(WriteActorTelemetrySink::new(
            handle,
            session_id.clone(),
            project_id.cloned(),
            Arc::clone(&self.clock),
        ))
    }
}

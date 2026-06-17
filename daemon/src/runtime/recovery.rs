//! P4.1b-1 (brief 059) C2 — the PRODUCTION restart-recovery wiring (the `main.rs` entry).
//!
//! Three production pieces behind the deterministic [`session::recovery`](crate::session::recovery)
//! orchestration: the proj_session READER ([`enumerate_recoverable_sessions`] — sources identity + the
//! §15 #8 profile from the COMMITTED `SessionStarted` the projector folded), the System-actor
//! `SessionRecovered` observation SINK ([`WriteActorRecoverySink`] — write-actor, NOT the Gateway;
//! Q1=(a), lead/user-ruled), and the conservative production `Broker`/dispatch. The REAL
//! detachable-terminal broker + the live launcher drive land **4.1b-2**; the LIVE survival is the
//! 0.1/0.3-HITL follow-on. [`run_restart_recovery`] is the reachable entry `main.rs` calls
//! post-supervisor — closing `decide_resume`'s Step-7.5 reachability.

use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
use nexusops_shared::events::{SessionRecovered, SessionStarted};
use nexusops_shared::ids::{ExecutionProfileId, SessionId, WorkspaceId};
use nexusops_shared::status::Session;

use crate::clock::Clock;
use crate::eventstore::AppendIntent;
use crate::runtime::WriteHandle;
use crate::session::broker::Broker;
use crate::session::recovery::{
    is_recoverable_status, recover_sessions_on_restart, RecoverableSession, RecoveryAction,
    RecoveryDispatch, RecoverySignal, RecoverySignalSink,
};
use crate::terminal::ScrollbackStore;

/// Read the sessions that were live at shutdown — the live-set + status come from the rebuilt
/// `proj_session` read model (Q3: authoritative post-rebuild), and the §15 #8 `execution_profile_id`
/// comes from the COMMITTED `SessionStarted` EVENT (the authoritative event store; Q1 condition #1 — the
/// safety-critical profile-preservation source reads the event, NEVER a derived cache). A `proj_session`
/// row exists ONLY because `SessionStarted` was committed → recovery never fabricates authority for a
/// never-started session. A row that fails to parse fail-closed (unknown id/status — should never occur
/// post-rebuild, reject-unknown) is SKIPPED, never fabricated. The `supports_resume`/`has_resume_handle`
/// affordances are the live-adapter inputs 4.1b-2 populates — `false` here (the resume-handle/live-broker
/// axis is 4.1b-2/HITL); `has_scrollback` is fed from the `store` as of 075c (see below).
///
/// NOTE (Step-9 flag): `proj_session` has an `execution_profile_id` column the 1.2 projector never folds
/// — so the profile is sourced from the event here, not the (always-NULL) column. Folding it in the
/// projector (for the §11.4 session-card profile badge) is a separate follow-on.
///
/// **075c — the scrollback axis is now LIVE-fed from the `store`.** For each session we `load` its VT
/// snapshot and set `has_scrollback`/`replayed_event_count` from it (replacing the hardcoded `false`/`0`):
/// `has_scrollback = snapshot.has_restorable_content()` (scrollback rows OR a non-blank replayable screen
/// — so a mid-alt-screen session still `Replayed`s, the 075b design-input), `replayed_event_count =
/// scrollback_rows()` (honest 0 for alt-active). With the production no-op store every `load` is `None` →
/// `false`/`0` → `Relaunched` exactly as before (075d's durable store flips this on). The
/// `supports_resume`/`has_resume_handle` axis stays `false` (the resume-handle/live-broker axis is
/// 4.1b-2/HITL — 075c owns ONLY the scrollback axis).
pub fn enumerate_recoverable_sessions(
    conn: &Connection,
    store: &dyn ScrollbackStore,
) -> rusqlite::Result<Vec<RecoverableSession>> {
    let mut stmt = conn.prepare("SELECT session_id, status FROM proj_session")?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for (sid, status_wire) in rows {
        let Ok(session_id) = SessionId::parse(&sid) else {
            continue;
        };
        let Ok(status) = serde_json::from_value::<Session>(serde_json::Value::String(status_wire))
        else {
            continue;
        };
        let execution_profile_id = committed_profile(conn, &session_id)?;
        // the scrollback axis (075c): keyed on has-restorable-content, not raw scrollback length.
        // Only LOAD for recoverable (non-terminal) sessions — terminal rows are discarded downstream
        // by `recover_sessions_on_restart`, so a store read for them is wasted I/O (matters once 075d's
        // durable store lands; harmless with the no-op store). `usize → u64` is lossless on every
        // supported target (both unsigned, u64 ≥ usize) → a plain `as` cast, not a fallible conversion.
        let (has_scrollback, replayed_event_count) = if is_recoverable_status(status) {
            match store.load(&session_id) {
                Some(snap) => (snap.has_restorable_content(), snap.scrollback_rows() as u64),
                None => (false, 0),
            }
        } else {
            (false, 0)
        };
        out.push(RecoverableSession {
            session_id,
            status,
            execution_profile_id,
            supports_resume: false,
            has_resume_handle: false,
            has_scrollback,
            replayed_event_count,
        });
    }
    Ok(out)
}

/// The §15 #8 `execution_profile_id` from the latest COMMITTED `SessionStarted` event for `session_id`
/// (the authoritative store; `None` if the session has none, or the event is somehow absent — recovery
/// then preserves "no profile" rather than fabricating one). Read-only.
fn committed_profile(
    conn: &Connection,
    session_id: &SessionId,
) -> rusqlite::Result<Option<ExecutionProfileId>> {
    let payload: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM events \
             WHERE session_id = ?1 AND event_type = 'SessionStarted' \
             ORDER BY seq DESC LIMIT 1",
            params![session_id.as_str()],
            |r| r.get(0),
        )
        .optional()?;
    Ok(payload
        .and_then(|p| serde_json::from_str::<SessionStarted>(&p).ok())
        .and_then(|s| s.execution_profile_id))
}

/// The PRODUCTION [`RecoverySignalSink`] — appends each [`RecoverySignal`] as a `SessionRecovered`
/// OBSERVATION event via the single write-actor (System-actor; the `DeviceRegistered`/`TelemetrySampled`
/// precedent, LESSON §10/§23; NOT the Gateway — Q1=(a)). Fire-and-forget (`try_append_observation`,
/// soft-degrade): a recovery is non-mutation + non-safety — a dropped signal is a stale §11.4 bit, never
/// a safety fault (the `proj_session` row + the §17 RecoveryState let the UI re-derive). The §15 #8
/// profile rides the payload, the session identity the envelope; `occurred_at` = the daemon-`Clock` UTC-Z
/// (the `system_intent` precedent). Still §15-redaction-gated (the same write-actor append path).
pub struct WriteActorRecoverySink {
    handle: WriteHandle,
    clock: Arc<dyn Clock>,
}

impl WriteActorRecoverySink {
    pub fn new(handle: WriteHandle, clock: Arc<dyn Clock>) -> Self {
        Self { handle, clock }
    }

    /// Build the `SessionRecovered` observation [`AppendIntent`] — `actor_type=System` (the daemon's OWN
    /// lifecycle, the `system_intent` precedent; Q1=(a)), the session identity on the envelope,
    /// `occurred_at` = the daemon-`Clock` UTC-Z. `project_id=None` (the session id suffices; project
    /// bucketing via the `proj_session` join is a P5 follow-on).
    fn intent(&self, signal: &RecoverySignal, payload_json: String) -> AppendIntent {
        let sid = signal.session_id.as_str().to_string();
        AppendIntent {
            event_type: SessionRecovered::EVENT_TYPE.to_string(),
            event_version: 1,
            occurred_at: self.clock.now_rfc3339(),
            workspace_id: WorkspaceId::system(),
            actor_type: ActorType::System,
            actor_id: sid.clone(),
            source_type: SourceType::LocalDaemon,
            source_id: sid.clone(),
            correlation_id: sid,
            sensitivity: Sensitivity::Internal,
            payload_json,
            schema_version: "event-envelope-v1".to_string(),
            idempotency_key: None,
            project_id: None,
            session_id: Some(signal.session_id.clone()),
            agent_team_id: None,
            visibility: Some(Visibility::System),
            action_request_id: None,
            approval_id: None,
            causation_id: None,
        }
    }
}

impl RecoverySignalSink for WriteActorRecoverySink {
    fn emit_recovered(&self, signal: RecoverySignal) {
        let payload = SessionRecovered {
            mode: signal.mode,
            replayed_event_count: signal.replayed_event_count,
            execution_profile_id: signal.execution_profile_id.clone(),
        };
        // a serialize failure (not expected — all fields are plainly serializable) drops the signal
        // (soft-degrade, non-safety — never panics the restart path), but is LOGGED so an operator can
        // tell a corrupt-payload drop from a clean fire-and-forget channel drop.
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                eprintln!(
                    "nexusopsd: restart recovery — dropped a SessionRecovered signal (serialize failed): {e}"
                );
                return;
            }
        };
        // fire-and-forget: never back-pressures the writer (LESSON §9/§30).
        let _ = self
            .handle
            .try_append_observation(self.intent(&signal, payload_json));
    }
}

/// The production recovery [`RecoveryDispatch`] — DEFERRED. The live re-materialization (the
/// identity-preserving launcher relaunch + the broker reattach) is the 4.1b-2 / 0.1-0.3-HITL follow-on;
/// at this slice the recovery EFFECT is the audited `SessionRecovered` signal the sink emits (for
/// `Relaunched` the signal IS the §17 "restart session" affordance the §11.4 UI renders — Q5: an
/// observation, NOT an auto-action). A no-op here is HONEST: nothing is re-spawned that the audit log
/// would misrepresent. 4.1b-2 replaces this with the real launcher/broker drive.
struct DeferredRecoveryDispatch;

impl RecoveryDispatch for DeferredRecoveryDispatch {
    fn dispatch(
        &self,
        _session_id: &SessionId,
        _action: RecoveryAction,
        _profile: Option<&ExecutionProfileId>,
    ) {
        // intentionally a no-op (the live drive is 4.1b-2 / HITL); the audited signal is the C2 effect.
    }
}

/// The reachable restart-recovery entry — `main.rs` calls this post-supervisor, BEFORE the accept-loop
/// serves clients (the §16 restart path). Enumerates the live-at-shutdown sessions from `proj_session`
/// (read-only `conn`) → drives the deterministic `recover_sessions_on_restart` over the SELECTED
/// survival `broker` (4.1b-2: `TmuxBroker` when tmux is present, else `NoSurvivorBroker` — graceful
/// degrade) + the deferred dispatch + the write-actor `SessionRecovered` sink. The PRODUCTION caller of
/// `decide_resume` (4.1a) — closes its Step-7.5 reachability. Returns the recovered-session count.
pub fn run_restart_recovery(
    conn: &Connection,
    broker: &dyn Broker,
    handle: &WriteHandle,
    clock: Arc<dyn Clock>,
    store: &dyn ScrollbackStore,
) -> usize {
    let sessions = match enumerate_recoverable_sessions(conn, store) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("nexusopsd: restart recovery — proj_session read failed (skipping): {e}");
            return 0;
        }
    };
    if sessions.is_empty() {
        return 0;
    }
    let dispatch = DeferredRecoveryDispatch;
    let sink = WriteActorRecoverySink::new(handle.clone(), clock);
    recover_sessions_on_restart(&sessions, broker, &dispatch, &sink).len()
}

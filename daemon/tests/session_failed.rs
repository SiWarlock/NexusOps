//! P4.2 (brief 061) — the §17 supervised-child-death surface HEAD: a `Failed` reap → `SessionFailed`
//! (System-actor observation, write-actor, NOT the Gateway) → `proj_session.status='failed'` (the §11.4
//! restart affordance). The cascade ARMS (fail-in-flight-action, release-lease, Codex distinction) are
//! DEFERRED to their producers (lead-ruled known-shape-vs-defer). Deterministic; cat-1 boundary held
//! (the supervisor emits via an INJECTED `SessionDeathSink` trait — never a `WriteHandle`).

use std::path::Path;
use std::sync::{Arc, Mutex};

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::events::{SessionFailed, SessionStarted};
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
use nexusops_shared::status::Session;

use nexusopsd::clock::FixedClock;
use nexusopsd::decisions::DecisionRegistry;
use nexusopsd::eventstore::{open_read_only, AppendIntent, EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::session_death::WriteActorSessionDeathSink;
use nexusopsd::runtime::WriteActor;
use nexusopsd::session::{handle_reaped, SessionDeathSink};

// ---- scaffold (the telemetry_pump.rs / recovery_restart_wiring.rs pattern) -----------------------

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open_store(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .unwrap()
}

fn stub_gateway() -> nexusopsd::gateway::Gateway {
    nexusopsd::gateway::Gateway::new(
        Box::new(nexusopsd::gateway::policy::StubPolicy),
        Box::new(nexusopsd::gateway::executor::StubExecutor),
    )
}

/// a recording `SessionDeathSink` — records each `emit_failed` (the `RecordingSink` precedent).
#[derive(Default)]
struct RecordingDeathSink {
    failed: Mutex<Vec<SessionId>>,
}

impl SessionDeathSink for RecordingDeathSink {
    fn emit_failed(&self, session_id: &SessionId) {
        self.failed.lock().unwrap().push(session_id.clone());
    }
}

/// commit a `SessionStarted` (the projector folds `proj_session` status=Starting).
fn commit_session_started(store: &mut EventStore, session_id: &SessionId, project_id: &ProjectId) {
    let payload = serde_json::to_string(&SessionStarted {
        status: Session::Active,
        harness: None,
        model: None,
        display_name: None,
        execution_profile_id: None,
    })
    .unwrap();
    store
        .append(envelope(
            "SessionStarted",
            session_id,
            Some(project_id),
            payload,
        ))
        .unwrap();
}

/// commit a `SessionFailed` (the projector folds `proj_session` status=Failed).
fn commit_session_failed(store: &mut EventStore, session_id: &SessionId) {
    let payload = serde_json::to_string(&SessionFailed {}).unwrap();
    store
        .append(envelope("SessionFailed", session_id, None, payload))
        .unwrap();
}

fn envelope(
    event_type: &str,
    session_id: &SessionId,
    project_id: Option<&ProjectId>,
    payload_json: String,
) -> AppendIntent {
    AppendIntent {
        event_type: event_type.to_string(),
        event_version: 1,
        occurred_at: "2026-06-08T00:00:00Z".to_string(),
        workspace_id: WorkspaceId::system(),
        actor_type: ActorType::System,
        actor_id: session_id.as_str().to_string(),
        source_type: SourceType::LocalDaemon,
        source_id: session_id.as_str().to_string(),
        correlation_id: session_id.as_str().to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json,
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: project_id.cloned(),
        session_id: Some(session_id.clone()),
        agent_team_id: None,
        visibility: None,
        action_request_id: None,
        causation_id: None,
        approval_id: None,
    }
}

// ---- RED #3 — a Failed reap emits SessionFailed via the injected sink -----------------------------

#[test]
fn test_failed_reap_emits_session_failed() {
    // spec(§17) — a `(id, Failed)` reap → the injected sink receives one SessionFailed for that session
    // (the supervisor's child-death surface; the registry's cancel still runs for every reap).
    let sink = RecordingDeathSink::default();
    let registry = DecisionRegistry::new();
    let failed = SessionId::new();
    handle_reaped(&[(failed.clone(), Session::Failed)], &registry, &sink);
    assert_eq!(sink.failed.lock().unwrap().as_slice(), &[failed]);
}

// ---- RED #4 — a clean (Killed/Completed) reap does NOT emit (not a failure) -----------------------

#[test]
fn test_clean_reap_does_not_emit() {
    // spec(§17 / Q3) — only a FAILURE is a failure: a clean `session.kill` (Killed) or a normal end
    // (Completed) reap is NOT a child-death → no SessionFailed.
    let sink = RecordingDeathSink::default();
    let registry = DecisionRegistry::new();
    handle_reaped(
        &[
            (SessionId::new(), Session::Killed),
            (SessionId::new(), Session::Completed),
        ],
        &registry,
        &sink,
    );
    assert!(
        sink.failed.lock().unwrap().is_empty(),
        "a clean terminal reap is not a failure"
    );
}

// ---- RED #5 — the production sink appends SessionFailed as a System-actor observation -------------

#[tokio::test]
async fn test_driver_emits_via_write_actor() {
    // spec(LESSON §10/§23) — the production `WriteActorSessionDeathSink` appends SessionFailed via the
    // single write-actor (System-actor; session on the envelope; UTC-Z occurred_at; fire-and-forget),
    // NOT the Gateway (no Action* in the log — INV-SEC-1 untouched).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T12:00:00Z")),
        stub_gateway(),
    );
    let sid = SessionId::new();
    let sink = WriteActorSessionDeathSink::new(
        actor.handle(),
        Arc::new(FixedClock::new("2026-06-08T12:00:00Z")),
    );
    sink.emit_failed(&sid);
    actor.shutdown().await;

    let conn = open_read_only(&path).unwrap();
    let (etype, evt_session, occurred): (String, Option<String>, String) = conn
        .query_row(
            "SELECT event_type, session_id, occurred_at FROM events ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        etype, "SessionFailed",
        "the observation landed via the write-actor"
    );
    assert_eq!(
        evt_session.as_deref(),
        Some(sid.as_str()),
        "session on the envelope"
    );
    assert_eq!(
        occurred, "2026-06-08T12:00:00Z",
        "the injected daemon-Clock UTC-Z occurred_at (exact, not just a Z-suffix)"
    );
    let action_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_type LIKE 'Action%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(action_events, 0, "an observation, never a Gateway action");
}

// ---- RED #7 — the proj_session projector folds SessionFailed → status='failed' --------------------

#[test]
fn test_projector_folds_session_failed() {
    // spec(§5.1/§7.2) — SessionFailed folds proj_session → status='failed' (+completed_at) WHERE the
    // envelope session_id. The status binds the frozen §5.1 Session::Failed wire value.
    let (_d, path) = temp_db();
    let mut store = open_store(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    commit_session_started(&mut store, &sid, &pid);
    commit_session_failed(&mut store, &sid);

    let conn = open_read_only(&path).unwrap();
    let (status, completed): (String, Option<String>) = conn
        .query_row(
            "SELECT status, completed_at FROM proj_session WHERE session_id = ?1",
            [sid.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "failed", "SessionFailed → status=failed");
    assert!(completed.is_some(), "completed_at recorded on failure");
    // bind the §5.1 SoT: the folded status is a TERMINAL state (a regression making Failed recoverable
    // — e.g. dropping it from the terminal set — would break the restart-affordance semantics).
    assert!(
        Session::Failed.is_terminal(),
        "the §5.1 Failed state the projector folds to is terminal"
    );
}

// ---- RED #8 — the fold is rebuild-equivalent (status from the EVENT TYPE) -------------------------

#[test]
fn test_session_failed_rebuild_equivalent() {
    // spec(LESSON §17) — status='failed' is derived from the EVENT TYPE (SessionFailed), never a row's
    // current value, so re-folding the log (rebuild) yields the SAME status (idempotent, rebuild-safe).
    let (_d, path) = temp_db();
    let mut store = open_store(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    commit_session_started(&mut store, &sid, &pid);
    commit_session_failed(&mut store, &sid);
    store.rebuild_projections().unwrap();

    let conn = open_read_only(&path).unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM proj_session WHERE session_id = ?1",
            [sid.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        status, "failed",
        "rebuild re-derives status=failed from the event type"
    );
}

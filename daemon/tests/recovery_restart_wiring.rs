//! P4.1b-1 (brief 059) C2 — the PRODUCTION restart-recovery wiring: the proj_session reader (sources
//! identity + the §15 #8 profile from the COMMITTED `SessionStarted`) + the System-actor
//! `SessionRecovered` observation sink (write-actor, NOT the Gateway — Q1=(a), lead/user-ruled).
//! Integration over a real `EventStore` + write-actor (the `telemetry_pump.rs` precedent).

use std::path::Path;
use std::sync::Arc;

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::events::{SessionRecovered, SessionStarted};
use nexusops_shared::harness::ResumeMode;
use nexusops_shared::ids::{ExecutionProfileId, ProjectId, SessionId, WorkspaceId};
use nexusops_shared::status::Session;

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, AppendIntent, EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::recovery::{enumerate_recoverable_sessions, WriteActorRecoverySink};
use nexusopsd::runtime::WriteActor;
use nexusopsd::session::recovery::{RecoverySignal, RecoverySignalSink};

// ---- integration scaffold (the telemetry_pump.rs pattern) ---------------------------------------

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

/// a no-side-effect stub Gateway for the write-actor (Append never touches the gateway).
fn stub_gateway() -> nexusopsd::gateway::Gateway {
    nexusopsd::gateway::Gateway::new(
        Box::new(nexusopsd::gateway::policy::StubPolicy),
        Box::new(nexusopsd::gateway::executor::StubExecutor),
    )
}

/// Commit a `SessionStarted` through the event store (the projector folds `proj_session` in-txn) — the
/// AUTHORITATIVE prior-session record recovery sources identity + the §15 #8 profile from.
fn commit_session_started(
    store: &mut EventStore,
    session_id: &SessionId,
    project_id: &ProjectId,
    status: Session,
    profile: Option<&ExecutionProfileId>,
) {
    let payload = serde_json::to_string(&SessionStarted {
        status,
        harness: None,
        model: None,
        display_name: None,
        execution_profile_id: profile.cloned(),
    })
    .unwrap();
    let intent = AppendIntent {
        // SessionStarted predates the per-type `EVENT_TYPE` const (the 1.2 projector greps the literal).
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: "2026-06-08T00:00:00Z".to_string(),
        workspace_id: WorkspaceId::new(),
        actor_type: ActorType::User,
        actor_id: "u_1".to_string(),
        source_type: SourceType::DesktopUi,
        source_id: "src_1".to_string(),
        correlation_id: "corr_1".to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json: payload,
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: Some(project_id.clone()),
        session_id: Some(session_id.clone()),
        agent_team_id: None,
        visibility: None,
        action_request_id: None,
        approval_id: None,
        causation_id: None,
    };
    store.append(intent).unwrap();
}

// ---- C2 RED #8 (the lead's ADD, condition #1) — authority + profile from the COMMITTED SessionStarted

#[test]
fn test_reader_sources_authority_from_committed_session_started() {
    // spec(§8.1 / §15 #8 / Q1 condition #1) — recovery's identity + §15 #8 profile come from the
    // COMMITTED prior `SessionStarted` (folded into `proj_session`). A fabricated never-started session
    // (no committed `SessionStarted` → no `proj_session` row) is NEVER recovered — recovery never
    // fabricates authority for a session the daemon never genuinely started.
    let (_d, path) = temp_db();
    let mut store = open_store(&path);
    let started = SessionId::new();
    let profile = ExecutionProfileId::new();
    let pid = ProjectId::new();
    commit_session_started(&mut store, &started, &pid, Session::Active, Some(&profile));
    let fabricated = SessionId::new(); // no committed SessionStarted → no authority

    let conn = open_read_only(&path).unwrap();
    // 075c — enumeration now reads the scrollback store; with the no-op store every load is None →
    // has_replayable_snapshot stays false (this test exercises identity/profile sourcing, not the scrollback axis).
    let recoverable =
        enumerate_recoverable_sessions(&conn, &nexusopsd::terminal::NoopScrollbackStore).unwrap();

    assert_eq!(recoverable.len(), 1, "only the genuinely-started session");
    assert_eq!(recoverable[0].session_id, started);
    assert_eq!(
        recoverable[0].execution_profile_id,
        Some(profile),
        "the §15 #8 profile is sourced from the COMMITTED SessionStarted (no fabrication)"
    );
    assert_eq!(recoverable[0].status, Session::Active);
    assert!(
        !recoverable.iter().any(|r| r.session_id == fabricated),
        "a never-started session is never recovered (never fabricate authority)"
    );
}

// ---- C2 RED #9 (condition #2 + #4) — the production sink appends a SessionRecovered OBSERVATION ------

#[tokio::test]
async fn test_production_sink_appends_session_recovered_observation() {
    // spec(§8.1 / §7.1 / Q1 conditions #2 + #4) — the production `WriteActorRecoverySink` appends a
    // `SessionRecovered` OBSERVATION event via the single write-actor (System-actor; NOT the Gateway —
    // Q1=(a) / LESSON §10/§23), carrying the session identity + the PRESERVED §15 #8 profile. No
    // `Action*` event in the log: recovery is never a 2nd mutator (INV-SEC-1 untouched).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T12:00:00Z")),
        stub_gateway(),
    );
    let sid = SessionId::new();
    let profile = ExecutionProfileId::new();
    let sink = WriteActorRecoverySink::new(
        actor.handle(),
        Arc::new(FixedClock::new("2026-06-08T12:00:00Z")),
    );
    sink.emit_recovered(RecoverySignal {
        session_id: sid.clone(),
        mode: ResumeMode::Relaunched,
        replayed_event_count: 0,
        execution_profile_id: Some(profile.clone()),
    });
    // fire-and-forget: the Append is enqueued ahead of Shutdown (FIFO) → it commits before the join.
    actor.shutdown().await;

    let conn = open_read_only(&path).unwrap();
    let (etype, evt_session, payload): (String, Option<String>, String) = conn
        .query_row(
            "SELECT event_type, session_id, payload_json FROM events ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        etype, "SessionRecovered",
        "the observation landed via the write-actor"
    );
    assert_eq!(
        evt_session.as_deref(),
        Some(sid.as_str()),
        "carries the session identity (the envelope column)"
    );
    let recovered: SessionRecovered = serde_json::from_str(&payload).unwrap();
    assert_eq!(recovered.mode, ResumeMode::Relaunched);
    assert_eq!(
        recovered.execution_profile_id,
        Some(profile),
        "the §15 #8 profile is preserved in the recovery record (no account-hop)"
    );
    let action_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_type LIKE 'Action%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        action_events, 0,
        "recovery is an observation event, never a Gateway action (Q1=(a), INV-SEC-1)"
    );
}

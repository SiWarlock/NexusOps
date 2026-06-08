//! Phase 1.6a L2 — cold-start bootstrap orchestration (RED first). ARCHITECTURE §16
//! (first-run ordering, single-instance pidlock, DB version floor, restart-resume),
//! §14 (injected base dir + Clock/IdGen/Redactor seams — no real `~/Library`).
//!
//! `cold_start()` is reachable from these tests only this slice; its production caller is
//! the 1.6c runtime (`main.rs` + the Tokio accept-loop). Device/LocalRunner registration
//! is L3 (held on the lead/user INV-SEC-1 ruling); the §17 degradable replay is 1.6b; the
//! UDS bind/accept + Tokio spawns are 1.6c — all OUT of scope here.

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::ids::WorkspaceId;
use nexusops_shared::ipc::{SUPPORTED_PROTOCOL_MAX, SUPPORTED_PROTOCOL_MIN};
use nexusopsd::bootstrap::{cold_start, BootstrapConfig, BootstrapError, DB_FILENAME};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    AppendIntent, EventStoreError, PrefixRedactor, SUPPORTED_USER_VERSION,
};
use nexusopsd::idgen::UlidGen;

use std::path::PathBuf;

/// A bootstrap config over an injected base dir (§14 seam — no real `~/Library`).
fn config(base_dir: PathBuf) -> BootstrapConfig {
    BootstrapConfig {
        base_dir,
        idgen: Box::new(UlidGen),
        clock: Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        redactor: Box::new(PrefixRedactor),
    }
}

/// A base dir path UNDER a fresh tempdir that does NOT yet exist — so cold_start must
/// create it (exercises the create-app-support-dir step). Returns (tempdir guard, dir).
fn fresh_base() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Application Support").join("NexusOps");
    (tmp, dir)
}

/// A minimal valid append intent (the store assigns event_id, seq, recorded_at) — used to
/// prove restart-resume writes survive a re-open WITHOUT depending on L3 registration.
fn test_intent() -> AppendIntent {
    AppendIntent {
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
        payload_json: "{}".to_string(),
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: None,
    }
}

#[test]
fn test_clean_first_run_creates_and_migrates() {
    // §16 first-run happy path: an empty base dir → Ok(context); the dir is created;
    // the DB is migrated to the supported floor.
    let (_tmp, dir) = fresh_base();
    assert!(!dir.exists(), "precondition: base dir absent");

    let ctx = cold_start(config(dir.clone())).expect("clean first run succeeds");

    assert!(dir.exists(), "cold_start creates the app-support dir");
    assert!(dir.join(DB_FILENAME).exists(), "the DB file exists");
    assert_eq!(
        ctx.store.user_version().unwrap(),
        SUPPORTED_USER_VERSION as u32,
        "DB migrated to the supported user_version floor (§16)"
    );
}

#[test]
fn test_second_instance_refused() {
    // §16 single-instance / forbidden #3: a second cold_start while the first holds the
    // pidlock is refused with AlreadyRunning (NOT a silent second writer).
    let (_tmp, dir) = fresh_base();
    let _ctx1 = cold_start(config(dir.clone())).expect("first instance starts");

    // DaemonContext holds a (non-Debug) EventStore, so extract the Err by match, not expect_err.
    let err = match cold_start(config(dir.clone())) {
        Ok(_) => panic!("second instance must be refused"),
        Err(e) => e,
    };
    assert!(
        matches!(err, BootstrapError::AlreadyRunning),
        "second instance → AlreadyRunning, got {err:?}"
    );
}

#[test]
fn test_db_newer_than_binary_refuses() {
    // §16 downgraded-binary refuse-safe: a DB at user_version > SUPPORTED → cold_start
    // refuses (the daemon does NOT start over a future-schema DB).
    let (_tmp, dir) = fresh_base();
    std::fs::create_dir_all(&dir).unwrap();
    {
        let conn = rusqlite::Connection::open(dir.join(DB_FILENAME)).unwrap();
        conn.pragma_update(None, "user_version", SUPPORTED_USER_VERSION + 1)
            .unwrap();
    }

    let err = match cold_start(config(dir.clone())) {
        Ok(_) => panic!("future-schema DB must be refused"),
        Err(e) => e,
    };
    assert!(
        matches!(
            err,
            BootstrapError::Store(EventStoreError::DbNewerThanSupported { .. })
        ),
        "DB-newer refusal is a typed DbNewerThanSupported, got {err:?}"
    );
}

#[test]
fn test_restart_resumes_existing_db() {
    // §16 restart-resume: cold_start, write an event through the bootstrapped store, release
    // (drop), cold_start again → Ok against the SAME DB; the prior event survives (the log is
    // the durable spine). Proven via a manual append — independent of L3 registration.
    let (_tmp, dir) = fresh_base();

    let mut ctx1 = cold_start(config(dir.clone())).expect("first start");
    let id = ctx1
        .store
        .append(test_intent())
        .expect("append through the bootstrapped store");
    let count_after_first = ctx1.store.read_all().unwrap().len();
    drop(ctx1); // releases the pidlock + closes the writer

    let ctx2 = cold_start(config(dir.clone())).expect("restart resumes");
    let events = ctx2.store.read_all().unwrap();
    assert_eq!(
        events.len(),
        count_after_first,
        "restart re-opens the SAME DB — no events lost or duplicated"
    );
    assert!(
        events.iter().any(|e| e.event_id == id),
        "the pre-restart event survived the restart (durable spine)"
    );
}

#[test]
fn test_app_support_dir_idempotent() {
    // re-run robustness: cold_start when the base dir already exists succeeds (exists-ok,
    // not an error) — the create step is idempotent.
    let (_tmp, dir) = fresh_base();
    std::fs::create_dir_all(&dir).unwrap();

    let ctx = cold_start(config(dir.clone()));
    assert!(ctx.is_ok(), "cold_start over an existing dir is exists-ok");
}

#[test]
fn test_version_info_reports_db_and_contract() {
    // §16 version-compat: a composed DaemonVersionInfo is available off the context for the
    // handshake/diagnostics. The ENFORCING floor is the DB user_version (asserted here +
    // pinned by test_db_newer_than_binary_refuses); the agent-CLI/SDK + sidecar-MCP matrix
    // rows are §9.1/§13.1 — out of scope, named as deferred.
    let (_tmp, dir) = fresh_base();
    let ctx = cold_start(config(dir.clone())).unwrap();

    assert_eq!(ctx.version.db_user_version, SUPPORTED_USER_VERSION as u32);
    assert_eq!(
        ctx.version.contract_version,
        nexusops_shared::CONTRACT_VERSION
    );
    assert_eq!(
        ctx.version.protocol_range,
        (SUPPORTED_PROTOCOL_MIN, SUPPORTED_PROTOCOL_MAX),
        "protocol_range mirrors the daemon-authored SUPPORTED_PROTOCOL_RANGE"
    );
    assert_eq!(ctx.version.app_version, env!("CARGO_PKG_VERSION"));
}

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
        action_request_id: None,
        approval_id: None,
        causation_id: None,
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
    // §16 restart-resume + L3: cold_start now registers identity each start (a fresh
    // LocalRunnerRegistered every start; the Device is register-if-absent), so a restart
    // RESUMES the SAME DB (every prior event survives — the durable spine) AND appends
    // exactly one new runner registration (the Device is NOT re-registered).
    let (_tmp, dir) = fresh_base();

    let mut ctx1 = cold_start(config(dir.clone())).expect("first start");
    let id = ctx1
        .store
        .append(test_intent())
        .expect("append through the bootstrapped store");
    // first start: DeviceRegistered + LocalRunnerRegistered + the manual SessionStarted.
    let count_after_first = ctx1.store.read_all().unwrap().len();
    drop(ctx1); // releases the pidlock + closes the writer

    let ctx2 = cold_start(config(dir.clone())).expect("restart resumes");
    let events = ctx2.store.read_all().unwrap();
    assert!(
        events.iter().any(|e| e.event_id == id),
        "the pre-restart event survived the restart (durable spine)"
    );
    assert_eq!(
        events.len(),
        count_after_first + 1,
        "restart re-opens the SAME DB and adds exactly one new LocalRunnerRegistered"
    );
    let device_events = events
        .iter()
        .filter(|e| e.event_type == "DeviceRegistered")
        .count();
    assert_eq!(
        device_events, 1,
        "the desktop host Device is register-if-absent — not re-registered on restart"
    );
}

#[test]
fn test_localrunner_minted_per_start() {
    // §5.3 "LocalRunner minted per daemon start": each cold_start appends exactly one
    // LocalRunnerRegistered with a FRESH lr_ id → two starts yield two distinct runners.
    let (_tmp, dir) = fresh_base();

    let ctx1 = cold_start(config(dir.clone())).expect("first start");
    let lr1 = ctx1.local_runner_id.clone();
    assert!(lr1.as_str().starts_with("lr_"), "runner id carries lr_");
    drop(ctx1);

    let ctx2 = cold_start(config(dir.clone())).expect("restart");
    let lr2 = ctx2.local_runner_id.clone();
    assert_ne!(lr1, lr2, "a fresh LocalRunner is minted each start");

    let runner_events = ctx2
        .store
        .read_all()
        .unwrap()
        .into_iter()
        .filter(|e| e.event_type == "LocalRunnerRegistered")
        .count();
    assert_eq!(runner_events, 2, "one LocalRunnerRegistered per start");
}

#[test]
fn test_device_stable_across_restarts() {
    // §5.3 Device = the stable desktop host: register-if-absent → two starts against the
    // same base dir REUSE one dev_ id and append exactly one DeviceRegistered total.
    let (_tmp, dir) = fresh_base();

    let ctx1 = cold_start(config(dir.clone())).expect("first start");
    let dev1 = ctx1.device_id.clone();
    assert!(dev1.as_str().starts_with("dev_"), "device id carries dev_");
    drop(ctx1);

    let ctx2 = cold_start(config(dir.clone())).expect("restart");
    assert_eq!(
        ctx2.device_id, dev1,
        "the desktop host Device is stable across restarts (register-if-absent)"
    );

    let device_events = ctx2
        .store
        .read_all()
        .unwrap()
        .into_iter()
        .filter(|e| e.event_type == "DeviceRegistered")
        .count();
    assert_eq!(
        device_events, 1,
        "register-if-absent: one DeviceRegistered across restarts"
    );
}

#[test]
fn test_registration_event_redacted_and_projected() {
    // §15 + Option B: the registration events are System-actor, pass the redaction-before-
    // persist gate (persist `redacted`), carry the reserved system-workspace sentinel, and
    // land in object_refs (the id sourced from the payload — dev_/lr_ aren't envelope columns).
    use nexusops_shared::actor::ActorType;
    use nexusops_shared::event_envelope::RedactionStatus;

    let (_tmp, dir) = fresh_base();
    let ctx = cold_start(config(dir.clone())).expect("first start");
    let dev_id = ctx.device_id.as_str().to_string();
    let lr_id = ctx.local_runner_id.as_str().to_string();

    let regs: Vec<_> = ctx
        .store
        .read_all()
        .unwrap()
        .into_iter()
        .filter(|e| e.event_type == "DeviceRegistered" || e.event_type == "LocalRunnerRegistered")
        .collect();
    assert_eq!(regs.len(), 2, "both registration events persisted");
    for e in &regs {
        assert_eq!(
            e.redaction_status,
            RedactionStatus::Redacted,
            "§15 redaction-before-persist gate ran for {}",
            e.event_type
        );
        assert_eq!(
            e.actor_type,
            ActorType::System,
            "Option B: daemon self-registration is a System-actor event, not a Gateway Action"
        );
        assert_eq!(
            e.workspace_id,
            WorkspaceId::system(),
            "a workspace-less System event carries the reserved sentinel"
        );
    }

    // object_refs: a ('device', dev_id) + a ('local_runner', lr_id) edge, payload-sourced.
    let conn = nexusopsd::eventstore::open_read_only(&dir.join(DB_FILENAME)).unwrap();
    let mut stmt = conn
        .prepare("SELECT object_type, object_id FROM object_refs")
        .unwrap();
    let refs: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(
        refs.contains(&("device".to_string(), dev_id)),
        "device object_ref written from the payload"
    );
    assert!(
        refs.contains(&("local_runner".to_string(), lr_id)),
        "local_runner object_ref written from the payload"
    );
}

#[test]
fn test_corrupt_device_registration_refuses() {
    // §16 fail-closed: a corrupt stored DeviceRegistered payload (the register-if-absent read)
    // makes cold_start refuse → the daemon does NOT start half-identified. (Surfaced by the
    // 1.6a-L3 code-quality review — the `BootstrapError::Registration` path now has a pin.)
    let (_tmp, dir) = fresh_base();
    // a first start records a VALID DeviceRegistered.
    drop(cold_start(config(dir.clone())).expect("first start registers a valid Device"));

    // simulate corruption of the stored DeviceRegistered payload via a test-only direct write
    // (the production append path can't produce this; this is the §17-adjacent corrupt-row case).
    {
        let conn = rusqlite::Connection::open(dir.join(DB_FILENAME)).unwrap();
        let n = conn
            .execute(
                "UPDATE events SET payload_json = '{}' WHERE event_type = 'DeviceRegistered'",
                [],
            )
            .unwrap();
        assert_eq!(n, 1, "exactly one DeviceRegistered row corrupted");
    }

    let err = match cold_start(config(dir.clone())) {
        Ok(_) => panic!("a corrupt stored DeviceRegistered must refuse start"),
        Err(e) => e,
    };
    assert!(
        matches!(err, BootstrapError::Registration(_)),
        "corrupt registration → fail-closed BootstrapError::Registration, got {err:?}"
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

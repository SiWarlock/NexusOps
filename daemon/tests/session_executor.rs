//! P4.0b-1 L3 (CAT-1) — the session.create/kill executor: risk-0 auto-execute + the audited
//! `SessionStarted` (PIN a), §15 #8 profile recorded-at-start (PIN b), the executor drives the
//! NON-LIVE launcher + routes session.kill, and the binding-condition structural test (test 9 — NO
//! `ClaudeAdapter` in the executor path; NOT wired into production `main.rs`).
//!
//! **Concurrency note (test-harness, NOT a production requirement):** the supervisor control channel is
//! UNBOUNDED → the executor's `spawn_session` is a SYNC, non-blocking enqueue that can never stall the
//! write-actor (cat-1). So `submit_action` (sync) never blocks on the supervisor, and a single-threaded
//! `#[tokio::test]` runtime does not deadlock. In production the executor runs on the write-actor's
//! dedicated `std::thread` (off the runtime), which is the load-bearing isolation.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::events::SessionStarted;
use nexusops_shared::harness::HarnessCapabilities;
use nexusops_shared::ids::{ActionRequestId, SessionId};
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::gateway::session_executor::SessionExecutor;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::session::{
    spawn_supervisor_task, FakeLauncher, LaunchedSession, SessionLauncher, SupervisorHandle,
};

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

fn full_caps() -> HarnessCapabilities {
    HarnessCapabilities {
        supports_terminal: true,
        supports_resume: true,
        supports_transcript_read: true,
        supports_tool_call_parsing: true,
        supports_usage_metadata: true,
        supports_context_metadata: true,
        supports_command_injection: true,
        supports_subagents: true,
        supports_hooks: true,
        supports_cloud_tasks: true,
    }
}

/// A launcher that COUNTS launches (proving the executor drove the NON-LIVE launcher) + delegates to
/// `FakeLauncher` (FakeHarness; NO `ClaudeAdapter`).
struct RecordingLauncher {
    inner: FakeLauncher,
    launches: Arc<AtomicUsize>,
}

impl SessionLauncher for RecordingLauncher {
    fn launch_session(&self) -> std::io::Result<LaunchedSession> {
        self.launches.fetch_add(1, Ordering::SeqCst);
        self.inner.launch_session()
    }
}

/// The production policy (catalog-driven) + the SessionExecutor over a recording NON-LIVE launcher +
/// a real (unbounded) supervisor handle. Returns the gateway + the launch-counter + the
/// shutdown/join keep-alives (drop them only at the end so the supervisor task stays up).
fn gateway_with_session_executor() -> (
    Gateway,
    Arc<AtomicUsize>,
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (join, handle): (_, SupervisorHandle) = spawn_supervisor_task(
        shutdown_rx,
        std::sync::Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
    );
    let launches = Arc::new(AtomicUsize::new(0));
    let launcher = RecordingLauncher {
        inner: FakeLauncher::new(full_caps()),
        launches: launches.clone(),
    };
    let executor = SessionExecutor::new(Box::new(launcher), handle);
    let gateway = Gateway::new(
        Box::new(nexusopsd::gateway::policy::CatalogPolicy),
        Box::new(executor),
    );
    (gateway, launches, shutdown_tx, join)
}

fn base_req(action_type: &str) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User, // UI/IPC (PIN e)
        requester_id: "u_local".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({}),
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
    }
}

/// A UI session.create with the project resource_ref (catalog `requires_resource_refs`) + an optional
/// requested ExecutionProfile id in the inputs (§15 #8).
fn session_create_req(profile: Option<&str>) -> ActionRequest {
    let mut req = base_req("session.create");
    req.resource_refs = vec![ResourceRef {
        resource_type: ResourceType::Project,
        id: "proj_session_ctx".to_string(),
        uri: None,
    }];
    if let Some(p) = profile {
        req.inputs = serde_json::json!({ "execution_profile_id": p });
    }
    req
}

/// the single appended `SessionStarted` event's payload, if one was emitted.
fn session_started_payload(store: &EventStore) -> Option<SessionStarted> {
    store
        .read_all()
        .unwrap()
        .iter()
        .find(|e| e.event_type == "SessionStarted")
        .map(|e| serde_json::from_str(&e.payload_json).expect("SessionStarted payload parses"))
}

#[tokio::test]
async fn test_session_create_auto_executes_and_audits() {
    // PIN (a) — spec(§6.2/§15) — a UI session.create (risk-0) AUTO-EXECUTES (no approval) AND a
    // `SessionStarted` is appended via the pipeline (INV-SEC-1 — audited even when auto-allowed).
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let (gw, _launches, _sd, _join) = gateway_with_session_executor();

    let ack = gw
        .submit_action(&mut store, session_create_req(None))
        .expect("session.create auto-executes");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "risk-0 session.create auto-executes to succeeded (no human approval)"
    );
    assert!(
        session_started_payload(&store).is_some(),
        "PIN a — SessionStarted is appended via the pipeline (audited)"
    );
}

#[tokio::test]
async fn test_session_started_records_profile() {
    // PIN (b) — spec(§15 #8) — the executor RESOLVES the ExecutionProfile + records its id in the
    // audited `SessionStarted` (profile recorded-at-start).
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let (gw, _launches, _sd, _join) = gateway_with_session_executor();

    let profile = nexusops_shared::ids::ExecutionProfileId::new();
    let profile_str = profile.as_str().to_string();
    gw.submit_action(&mut store, session_create_req(Some(&profile_str)))
        .expect("submit");

    let payload = session_started_payload(&store).expect("SessionStarted appended");
    assert_eq!(
        payload.execution_profile_id.as_ref().map(|p| p.as_str()),
        Some(profile_str.as_str()),
        "PIN b — SessionStarted records the resolved execution_profile_id (§15 #8)"
    );
}

#[tokio::test]
async fn test_session_create_drives_supervisor_non_live() {
    // spec(binding condition + 4.0a) — the executor drives the NON-LIVE launcher for session.create
    // (the recording launcher counts the launch; NO ClaudeAdapter), and session.kill routes a Kill
    // through the supervisor (submits Succeeded). The actor's terminal-state transition itself is
    // P4.0a-tested; here we pin that the EXECUTOR drives the launcher + supervisor.
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let (gw, launches, _sd, _join) = gateway_with_session_executor();

    gw.submit_action(&mut store, session_create_req(None))
        .expect("session.create");
    assert_eq!(
        launches.load(Ordering::SeqCst),
        1,
        "the executor launched ONE session over the NON-LIVE launcher (no ClaudeAdapter)"
    );

    // session.kill targets a session via its resource_ref (NaturalResourceRef-keyed).
    let mut kill = base_req("session.kill");
    kill.resource_refs = vec![ResourceRef {
        resource_type: ResourceType::Session,
        id: SessionId::new().as_str().to_string(),
        uri: None,
    }];
    let ack = gw.submit_action(&mut store, kill).expect("session.kill");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "session.kill routed a Kill through the supervisor (audited auto-allow)"
    );
}

#[test]
fn test_live_session_create_has_interception() {
    // 🔴 the call-5 ATOMICITY PIN (P4.0b-2 C2 — the binding-FLIP): a live agent becomes reachable (the
    // registered SessionExecutor + the live PtyLauncher) ONLY TOGETHER with the interception
    // (AgentMutationPolicy + the per-session decision_sink registry + the §17 alarm). Pinned
    // STRUCTURALLY by main.rs co-residency: the reachable session.create cannot exist in a shipped
    // state WITHOUT the interception, because they are wired in the SAME file (this commit). This is
    // the INVERSE of 4.0b-1's `test_no_reachable_live_caller` — the binding flips from
    // BY-CONSTRUCTION to ENFORCED-BY-THE-INTERCEPTION.
    let main_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
        .expect("main.rs present");

    // the live-REACHABLE wiring (session.create is dispatchable + the real ClaudeAdapter launches):
    assert!(
        main_src.contains("ExecutorKind::Session") && main_src.contains("SessionExecutor::new"),
        "main.rs registers the live SessionExecutor under ExecutorKind::Session (reachable session.create)"
    );
    assert!(
        main_src.contains("PtyLauncher::new") && main_src.contains("PortablePtySpawner"),
        "main.rs wires the LIVE launcher — the real ClaudeAdapter via PortablePtySpawner"
    );
    // ...co-landed WITH the interception (every one of these MUST be present alongside the live launch):
    assert!(
        main_src.contains("AgentMutationPolicy"),
        "the production policy is AgentMutationPolicy (the live INV-SEC-1 interception + deny-rules)"
    );
    assert!(
        main_src.contains("DecisionRegistry"),
        "the per-session decision_sink registry is wired (the intercept wait + the approve/deny resolve)"
    );
    assert!(
        main_src.contains("spawn_with_alarm_and_breaker")
            && main_src.contains("FileIntegrityAlarm")
            && main_src.contains("AuditBackboneBreaker"),
        "the §17 durable integrity alarm AND the daemon-wide audit-backbone circuit-breaker (P4.0b-2c) \
         are bound at the write-actor (call-2 per-incident alarm + the systemic quiesce-and-refuse gate)"
    );

    // the SessionExecutor itself stays LAUNCHER-AGNOSTIC — it constructs no ClaudeAdapter / live spawn
    // path; the real launcher is INJECTED (main.rs builds it). So the cat-1 live spawn lives at the
    // `PtyLauncher` (#10, Option A), not the executor — the executor can't smuggle an un-intercepted launch.
    let exec_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gateway/session_executor.rs"
    ))
    .expect("session_executor.rs present");
    for tok in ["ClaudeAdapter", "harness::claude", "PortablePtySpawner"] {
        assert!(
            !exec_src.contains(tok),
            "the session executor must stay launcher-agnostic (no `{tok}`) — the live launcher is \
             injected via the SessionLauncher seam (Option A; #10 lives at PtyLauncher)"
        );
    }
}

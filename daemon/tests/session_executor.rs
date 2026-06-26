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
    ActionPreview, ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::events::{PullRequestSynced, ReviewSynced, SessionStarted, WorktreeCreated};
use nexusops_shared::harness::HarnessCapabilities;
use nexusops_shared::ids::{ActionRequestId, ExecutionProfileId, ProjectId, SessionId, WorktreeId};
use nexusops_shared::ipc::{ProjectionDelta, ProjectionName};
use nexusops_shared::status::{ActionRequestStatus, PullRequest, ReviewState};
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::{ActionExecutor, EmittedEvent, ExecError, ExecutionOutcome};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::session_executor::SessionExecutor;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::WriteActor;
use nexusopsd::session::{
    spawn_supervisor_task, FakeLauncher, LaunchedSession, SessionCommand, SessionLauncher,
    SupervisorControl, SupervisorHandle,
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

/// An in-memory [`ProfileLookup`] double (P5.3a §15 #8): a known seeded default + a set of registered
/// ids. Lets the resolve cases (found / unknown-fail-closed / none-default) run deterministically with
/// NO real `execution_profiles` table — production injects the sqlite-backed lookup (`main.rs`).
struct FakeProfileLookup {
    default: ExecutionProfileId,
    known: std::collections::HashSet<String>,
}

impl FakeProfileLookup {
    fn new(default: ExecutionProfileId) -> Self {
        let mut known = std::collections::HashSet::new();
        known.insert(default.as_str().to_string());
        Self { default, known }
    }

    fn with_known(mut self, id: &ExecutionProfileId) -> Self {
        self.known.insert(id.as_str().to_string());
        self
    }
}

impl nexusopsd::profiles::ProfileLookup for FakeProfileLookup {
    fn default_id(&self) -> Result<ExecutionProfileId, nexusopsd::profiles::ProfileError> {
        Ok(self.default.clone())
    }
    fn exists(&self, id: &ExecutionProfileId) -> Result<bool, nexusopsd::profiles::ProfileError> {
        Ok(self.known.contains(id.as_str()))
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
    // a default fake with a known seeded default (the None / no-profile path resolves to it).
    gateway_with_profile_lookup(Box::new(FakeProfileLookup::new(ExecutionProfileId::new())))
}

fn gateway_with_profile_lookup(
    lookup: Box<dyn nexusopsd::profiles::ProfileLookup>,
) -> (
    Gateway,
    Arc<AtomicUsize>,
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (join, handle): (_, SupervisorHandle) = spawn_supervisor_task(
        shutdown_rx,
        std::sync::Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
        Box::new(nexusopsd::session::NullSessionDeathSink),
        std::sync::Arc::new(nexusopsd::terminal::NoopScrollbackStore),
    );
    let launches = Arc::new(AtomicUsize::new(0));
    let launcher = RecordingLauncher {
        inner: FakeLauncher::new(full_caps()),
        launches: launches.clone(),
    };
    let executor = SessionExecutor::new(Box::new(launcher), handle, lookup);
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
async fn resolve_uses_registered_profile_when_known() {
    // PIN (b) — spec(§15 #8) — a `Some(known_id)` present in the registry resolves to THAT id + is
    // recorded in the audited `SessionStarted` (profile recorded-at-start; registry-backed resolve).
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let profile = ExecutionProfileId::new();
    let lookup = FakeProfileLookup::new(ExecutionProfileId::new()).with_known(&profile);
    let (gw, _launches, _sd, _join) = gateway_with_profile_lookup(Box::new(lookup));

    let profile_str = profile.as_str().to_string();
    gw.submit_action(&mut store, session_create_req(Some(&profile_str)))
        .expect("submit");

    let payload = session_started_payload(&store).expect("SessionStarted appended");
    assert_eq!(
        payload.execution_profile_id.as_ref().map(|p| p.as_str()),
        Some(profile_str.as_str()),
        "PIN b — a known registered profile resolves to that id, recorded in SessionStarted (§15 #8)"
    );
}

#[tokio::test]
async fn resolve_fails_closed_on_unknown_profile() {
    // spec(§15 #8 — no silent account-hop; sub-decision 2) — a `Some(unknown_id)` (NOT in the registry)
    // FAILS CLOSED: session.create → Failed, NO mint, NO SessionStarted, NO session spawned (the resolve
    // happens BEFORE the launch, so no orphaned session-actor).
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    // the fake knows ONLY its default; the requested id is unknown.
    let lookup = FakeProfileLookup::new(ExecutionProfileId::new());
    let (gw, launches, _sd, _join) = gateway_with_profile_lookup(Box::new(lookup));

    let unknown = ExecutionProfileId::new();
    let ack = gw
        .submit_action(&mut store, session_create_req(Some(unknown.as_str())))
        .expect("submit returns an ack (Failed), not an Err");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Failed,
        "an unknown profile id fails the session.create closed (no silent mint)"
    );
    assert!(
        session_started_payload(&store).is_none(),
        "no SessionStarted is appended for a fail-closed profile resolution"
    );
    assert_eq!(
        launches.load(Ordering::SeqCst),
        0,
        "no session was spawned — the resolve fails BEFORE the launch (no orphaned actor)"
    );
}

#[tokio::test]
async fn resolve_uses_default_when_none_given() {
    // spec(§15 #8 — sub-decision 2) — `None` (no profile requested) resolves to the SEEDED DEFAULT id
    // (NOT a fresh per-session mint), recorded in SessionStarted.
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(&dir.path().join("nexusops.db"));
    let default = ExecutionProfileId::new();
    let lookup = FakeProfileLookup::new(default.clone());
    let (gw, _launches, _sd, _join) = gateway_with_profile_lookup(Box::new(lookup));

    gw.submit_action(&mut store, session_create_req(None))
        .expect("submit");

    let payload = session_started_payload(&store).expect("SessionStarted appended");
    assert_eq!(
        payload.execution_profile_id.as_ref().map(|p| p.as_str()),
        Some(default.as_str()),
        "None resolves to the seeded default profile id (not a fresh mint)"
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
        main_src.contains("select_survival_backend") && main_src.contains("PortablePtySpawner"),
        "main.rs wires the LIVE launcher — the 4.1b-2 survival backend (TmuxLauncher/PtyLauncher, the \
         real ClaudeAdapter) over the real PortablePtySpawner; the launcher constructor moved into \
         select_survival_backend, still co-resident with the interception below"
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

// =============================================================================
// W1-exec (094) — the session.send_message/pause/resume executor bodies. Each validates the catalog
// `requires_resource_refs` precondition FIRST, parses the target SessionId from the resource_ref, and
// routes a typed SessionCommand to the supervisor (the execute_kill precedent); `side_effect_applied =
// delivered`. An observable SupervisorHandle (from_sender) lets us assert the EXACT routed command.
// =============================================================================

/// A `SessionExecutor` over an OBSERVABLE supervisor: routes flow into `rx` so a test can assert the
/// exact `SupervisorControl::Route { session_id, command }` enqueued (the supervisor task is not
/// spawned — `route` enqueues into our channel, so `delivered == true` while `rx` is held).
fn executor_with_observable_supervisor() -> (
    SessionExecutor,
    tokio::sync::mpsc::UnboundedReceiver<SupervisorControl>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = SupervisorHandle::from_sender(tx);
    let launcher = FakeLauncher::new(full_caps());
    let lookup = Box::new(FakeProfileLookup::new(ExecutionProfileId::new()));
    let executor = SessionExecutor::new(Box::new(launcher), handle, lookup);
    (executor, rx)
}

/// a session-control request (session.pause/resume/send_message) keyed on the target session resource_ref.
fn session_ctrl_req(action_type: &str, target: &SessionId) -> ActionRequest {
    let mut req = base_req(action_type);
    req.resource_refs = vec![ResourceRef {
        resource_type: ResourceType::Session,
        id: target.as_str().to_string(),
        uri: None,
    }];
    req
}

fn send_message_req(target: &SessionId, message: &str) -> ActionRequest {
    let mut req = session_ctrl_req("session.send_message", target);
    req.inputs = serde_json::json!({ "message": message });
    req
}

#[test]
fn session_send_message_routes_to_supervisor() {
    // spec(§9.1) — a valid target + message → SessionCommand::SendMessage(text) routed; delivered honesty
    // (side_effect_applied == true while the supervisor channel is live, the execute_kill precedent).
    let (executor, mut rx) = executor_with_observable_supervisor();
    let target = SessionId::new();
    let outcome = executor.execute(&send_message_req(&target, "hello agent"));
    assert!(
        matches!(
            outcome,
            ExecutionOutcome::Succeeded {
                side_effect_applied: true,
                ..
            }
        ),
        "a routed send_message → Succeeded + delivered"
    );
    match rx.try_recv() {
        Ok(SupervisorControl::Route {
            session_id,
            command,
        }) => {
            assert_eq!(session_id, target, "routed to the target session");
            assert_eq!(
                command,
                SessionCommand::SendMessage("hello agent".to_string()),
                "routed the SendMessage command carrying the message text"
            );
        }
        _ => panic!("expected a SupervisorControl::Route(SendMessage)"),
    }
}

#[test]
fn session_send_message_empty_message_fails() {
    // spec(§9.1 — I::FromInputs required input) — an EMPTY or ABSENT inputs.message → Failed BEFORE any
    // route (a send with no message is invalid; no SessionCommand reaches the supervisor).
    let (executor, mut rx) = executor_with_observable_supervisor();
    let target = SessionId::new();
    // (a) empty string
    assert!(
        matches!(
            executor.execute(&send_message_req(&target, "")),
            ExecutionOutcome::Failed(_)
        ),
        "empty message → Failed"
    );
    // (b) absent inputs.message entirely
    let no_msg = session_ctrl_req("session.send_message", &target);
    assert!(
        matches!(executor.execute(&no_msg), ExecutionOutcome::Failed(_)),
        "absent message → Failed"
    );
    assert!(
        rx.try_recv().is_err(),
        "no command routed when the message is empty/absent"
    );
}

#[test]
fn session_send_message_missing_or_invalid_target_fails() {
    // spec(§6.3 precondition — the execute_kill precedent) — no resource_ref (catalog
    // requires_resource_refs) OR an unparseable SessionId → Failed BEFORE any route.
    let (executor, mut rx) = executor_with_observable_supervisor();
    // (a) no resource_ref → the requires_resource_refs precondition fails first.
    let mut no_ref = base_req("session.send_message");
    no_ref.inputs = serde_json::json!({ "message": "hi" });
    assert!(
        matches!(executor.execute(&no_ref), ExecutionOutcome::Failed(_)),
        "no resource_ref → Failed (requires_resource_refs)"
    );
    // (b) a resource_ref carrying an unparseable SessionId → Failed before route.
    let mut bad = base_req("session.send_message");
    bad.resource_refs = vec![ResourceRef {
        resource_type: ResourceType::Session,
        id: "not_a_session_id".to_string(),
        uri: None,
    }];
    bad.inputs = serde_json::json!({ "message": "hi" });
    assert!(
        matches!(executor.execute(&bad), ExecutionOutcome::Failed(_)),
        "invalid SessionId → Failed"
    );
    assert!(
        rx.try_recv().is_err(),
        "no command routed on a missing/invalid target"
    );
}

#[test]
fn session_pause_routes_pause() {
    // spec(§9.1) — session.pause routes SessionCommand::Pause to the target; delivered honesty.
    let (executor, mut rx) = executor_with_observable_supervisor();
    let target = SessionId::new();
    let outcome = executor.execute(&session_ctrl_req("session.pause", &target));
    assert!(matches!(
        outcome,
        ExecutionOutcome::Succeeded {
            side_effect_applied: true,
            ..
        }
    ));
    match rx.try_recv() {
        Ok(SupervisorControl::Route {
            session_id,
            command,
        }) => {
            assert_eq!(session_id, target);
            assert_eq!(command, SessionCommand::Pause, "routed Pause");
        }
        _ => panic!("expected Route(Pause)"),
    }
}

#[test]
fn session_resume_routes_resume() {
    // spec(§9.1) — session.resume routes SessionCommand::Resume (distinct from the survival decide_resume).
    let (executor, mut rx) = executor_with_observable_supervisor();
    let target = SessionId::new();
    let outcome = executor.execute(&session_ctrl_req("session.resume", &target));
    assert!(matches!(
        outcome,
        ExecutionOutcome::Succeeded {
            side_effect_applied: true,
            ..
        }
    ));
    match rx.try_recv() {
        Ok(SupervisorControl::Route {
            session_id,
            command,
        }) => {
            assert_eq!(session_id, target);
            assert_eq!(command, SessionCommand::Resume, "routed Resume");
        }
        _ => panic!("expected Route(Resume)"),
    }
}

#[test]
fn session_pause_validates_precondition_first() {
    // spec(§6.3) — requires_resource_refs validated FIRST: a pause with NO resource_ref → Failed, no route.
    let (executor, mut rx) = executor_with_observable_supervisor();
    let req = base_req("session.pause"); // no resource_refs
    assert!(
        matches!(executor.execute(&req), ExecutionOutcome::Failed(_)),
        "no resource_ref → Failed (precondition validated before any route)"
    );
    assert!(rx.try_recv().is_err(), "no command routed");
}

#[test]
fn session_control_bodies_only_via_gateway_execute() {
    // spec(§15 INV-SEC-1) — the send_message/pause/resume bodies grant NO new executor-side authority:
    // they ONLY route a SessionCommand to the supervisor (the execute_kill seam) + emit NO event, and
    // are reachable ONLY via the registered ExecutorKind::Session execute path. (a) behavioral: each
    // Succeeds with emitted_events EMPTY (no Gateway-bypass emission); (b) structural: the executor stays
    // launcher-agnostic (no live-adapter token), so a send_message feed can't smuggle an un-intercepted launch.
    let (executor, _rx) = executor_with_observable_supervisor();
    let target = SessionId::new();
    for outcome in [
        executor.execute(&send_message_req(&target, "hi")),
        executor.execute(&session_ctrl_req("session.pause", &target)),
        executor.execute(&session_ctrl_req("session.resume", &target)),
    ] {
        match outcome {
            ExecutionOutcome::Succeeded { emitted_events, .. } => assert!(
                emitted_events.is_empty(),
                "a session-control body emits NO event (route-only — no Gateway-bypass authority)"
            ),
            _ => panic!("expected Succeeded (routed)"),
        }
    }
    // structural: the executor constructs no live-adapter / launch path (the launcher-agnostic pin —
    // send_message routes to an EXISTING actor; it cannot create an un-intercepted agent).
    let exec_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gateway/session_executor.rs"
    ))
    .expect("session_executor.rs present");
    for tok in ["ClaudeAdapter", "harness::claude", "PortablePtySpawner"] {
        assert!(
            !exec_src.contains(tok),
            "the session-control executor stays launcher-agnostic (no `{tok}`)"
        );
    }
}

// =============================================================================
// D4b (P4.5) — the gateway-emitted-event delta COMPLETENESS SWEEP (production-path tests).
// Each drives the REAL gateway execute+publish (via a WriteActor), NOT a direct store.append — that's
// what closes the test-gap that let the Finding hide. The §51 mapping itself is unit-tested in
// `projections::delta_mapping_tests`; these pin the end-to-end gateway threading + the payload-id extract.
// =============================================================================

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn drain(rx: &mut tokio::sync::broadcast::Receiver<ProjectionDelta>) -> Vec<ProjectionDelta> {
    let mut out = Vec::new();
    while let Ok(d) = rx.try_recv() {
        out.push(d);
    }
    out
}

/// A test executor emitting ONE configured `Namespaced` event THROUGH the real gateway execute+publish
/// (the brief-allowed fake executor for the edges-emitted events). Paired with `CatalogPolicy` + a risk-0
/// auto-executing `session.create` as the trigger, so the emitted event flows execute→emitted_event_deltas
/// →publish without an approve round-trip.
struct EmitExecutor {
    event_type: &'static str,
    payload_json: String,
}
impl ActionExecutor for EmitExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        Ok(())
    }
    fn execute(&self, _req: &ActionRequest) -> ExecutionOutcome {
        ExecutionOutcome::Succeeded {
            changed_resources: vec![],
            detail: "test-emit".to_string(),
            side_effect_applied: false,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: self.event_type,
                payload_json: self.payload_json.clone(),
            }],
        }
    }
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: "test-emit".to_string(),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
}

#[tokio::test]
async fn test_gateway_session_create_publishes_session_activity_graph_deltas() {
    // THE Finding fix — production `SessionStarted` is emitted via the gateway emitted_events path (NOT
    // Command::Append), so it must nudge Session + ProjectActivity + ProjectGraph (all 3 fold it). Drives
    // the REAL gateway execute (SessionExecutor + FakeLauncher) through the WriteActor — NOT a direct append.
    let (_d, path) = temp_db();
    let (gw, _launches, _sd, _join) = gateway_with_session_executor();
    let actor = WriteActor::spawn(
        open(&path),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        gw,
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    let pid = ProjectId::new();
    let mut req = session_create_req(None);
    req.project_id = Some(pid.clone());
    let h = handle.clone();
    tokio::task::spawn_blocking(move || h.submit_action_blocking(req))
        .await
        .unwrap()
        .expect("write-actor reachable")
        .expect("session.create auto-executes");
    let deltas = drain(&mut rx);
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::Session
                && d.id.as_deref().is_some_and(|id| id.starts_with("sess_"))),
        "the gateway-emitted SessionStarted nudges Session keyed by the minted session_id (the \
         production Finding fix — a coarse id-less nudge would mean session_id wasn't threaded)"
    );
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::ProjectActivity
                && d.id.as_deref() == Some(pid.as_str())),
        "and ProjectActivity (keyed by project_id)"
    );
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::ProjectGraph
                && d.id.as_deref() == Some(pid.as_str())),
        "and ProjectGraph (keyed by project_id)"
    );
    actor.shutdown().await;
}

#[tokio::test]
async fn test_gateway_worktree_created_publishes_worktree_delta() {
    // a gateway-emitted WorktreeCreated → a Worktree Upsert keyed by worktree_id (parsed from the emitted
    // payload). Real gateway execute+publish via the test EmitExecutor.
    let (_d, path) = temp_db();
    let wt_payload = serde_json::to_string(&WorktreeCreated {
        worktree_id: WorktreeId::parse("wt_01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
        path: "/tmp/wt".to_string(),
        branch_name: "feature/x".to_string(),
        base_branch: None,
    })
    .unwrap();
    let gw = Gateway::new(
        Box::new(CatalogPolicy),
        Box::new(EmitExecutor {
            event_type: WorktreeCreated::EVENT_TYPE,
            payload_json: wt_payload,
        }),
    );
    let actor = WriteActor::spawn(
        open(&path),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        gw,
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    let h = handle.clone();
    tokio::task::spawn_blocking(move || h.submit_action_blocking(session_create_req(None)))
        .await
        .unwrap()
        .expect("write-actor reachable")
        .expect("auto-executes");
    let deltas = drain(&mut rx);
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::Worktree
                && d.id.as_deref() == Some("wt_01ARZ3NDEKTSV4RRFFQ69G5FAV")),
        "the gateway-emitted WorktreeCreated nudges Worktree (id = worktree_id from the payload)"
    );
    actor.shutdown().await;
}

#[tokio::test]
async fn test_gateway_pull_request_synced_publishes_pr_delta() {
    // a gateway-emitted PullRequestSynced → a PullRequest Upsert keyed by pr_id = `{repo_id}#{pr_number}`
    // (repo_id from the action's Repo resource_ref; pr_number from the emitted payload — the row PK).
    let (_d, path) = temp_db();
    let pr_payload = serde_json::to_string(&PullRequestSynced {
        pr_number: 42,
        status: PullRequest::Open,
        branch: "feature".to_string(),
        base: "main".to_string(),
        mergeable: None,
        checks_summary: None,
        additions: None,
        deletions: None,
        changed_files: None,
        commits: None,
        head_sha: None,
        pr_checked_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
    })
    .unwrap();
    let gw = Gateway::new(
        Box::new(CatalogPolicy),
        Box::new(EmitExecutor {
            event_type: PullRequestSynced::EVENT_TYPE,
            payload_json: pr_payload,
        }),
    );
    let actor = WriteActor::spawn(
        open(&path),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        gw,
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    // a session.create trigger carrying a Repo resource_ref so emitted_event_deltas computes pr_id.
    let mut req = session_create_req(None);
    req.resource_refs = vec![ResourceRef {
        resource_type: ResourceType::Repo,
        id: "repo_alpha".to_string(),
        uri: None,
    }];
    let h = handle.clone();
    tokio::task::spawn_blocking(move || h.submit_action_blocking(req))
        .await
        .unwrap()
        .expect("write-actor reachable")
        .expect("auto-executes");
    let deltas = drain(&mut rx);
    assert!(
        deltas.iter().any(|d| d.projection == ProjectionName::PullRequest
            && d.id.as_deref() == Some("repo_alpha#42")),
        "the gateway-emitted PullRequestSynced nudges PullRequest (id = pr_id = {{repo_id}}#{{pr_number}})"
    );
    actor.shutdown().await;
}

#[tokio::test]
async fn test_gateway_review_synced_publishes_review_delta() {
    // D5b-1 — a gateway-emitted ReviewSynced → a Review Upsert keyed by review_id (parsed from the emitted
    // payload — self-contained, NO sibling Repo ref needed for the nudge). Real gateway execute+publish via
    // the test EmitExecutor (the §51/§52 production-path discipline — the nudge rides emitted_event_deltas,
    // NOT a direct append).
    let (_d, path) = temp_db();
    let review_payload = serde_json::to_string(&ReviewSynced {
        review_id: 9001,
        pr_number: 42,
        reviewer: "octocat".to_string(),
        state: ReviewState::Approved,
        submitted_at: None,
        body: None,
        review_synced_at: Timestamp::parse("2026-06-16T00:00:00Z").unwrap(),
    })
    .unwrap();
    let gw = Gateway::new(
        Box::new(CatalogPolicy),
        Box::new(EmitExecutor {
            event_type: ReviewSynced::EVENT_TYPE,
            payload_json: review_payload,
        }),
    );
    let actor = WriteActor::spawn(
        open(&path),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        gw,
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    let h = handle.clone();
    tokio::task::spawn_blocking(move || h.submit_action_blocking(session_create_req(None)))
        .await
        .unwrap()
        .expect("write-actor reachable")
        .expect("auto-executes");
    let deltas = drain(&mut rx);
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::Review && d.id.as_deref() == Some("9001")),
        "the gateway-emitted ReviewSynced nudges Review (id = review_id from the payload)"
    );
    actor.shutdown().await;
}

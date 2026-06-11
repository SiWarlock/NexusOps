//! P2.4 — the Action Gateway's §17 failure-mode safety behaviors (the deterministic LOGIC + seams;
//! real external re-reads land with the adapters, Phase 3/5/7/8). Exercised via the §14
//! fault-injection hook (`nexusopsd::fault`, the `fault-injection` feature) + a fake clock + a fake
//! `PreconditionOracle` — NO real SIGKILL/threads (a real kill is flaky + un-assertable).
//!
//! **L2 (fail-closed on audit-write, §15/§17 INV-SEC-1):** an audit-required action whose terminal
//! event (`ActionSucceeded`/`ActionFailed`) cannot be written ABORTS — the completion txn rolls back,
//! the action stays `executing` (reconciled on restart by L5), and is NEVER acked succeeded. A side
//! effect APPLIED but its terminal event unwritable → `ActionPartiallySucceeded` (the loud,
//! consumer-visible audit-integrity record) + the action settles `partially_succeeded`, best-effort.

use nexusops_shared::actions::{ActionPreview, ActionRequest, RequesterType, RiskLevel};
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::fault::{arm, arm_n, FaultPoint};
use nexusopsd::gateway::executor::{ActionExecutor, ExecError, ExecutionOutcome, StubExecutor};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;

// ---- helpers (integration tests are separate crates → the small helpers are per-file, the codebase
// convention; mirrors policy.rs / gateway.rs) ----------------------------------------------------

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// the PRODUCTION Gateway (CatalogPolicy + the side-effect-free StubExecutor).
fn catalog_gateway() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(StubExecutor))
}

fn sample_request(action_type: &str, claimed_risk: RiskLevel) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({ "k": "v" }),
        risk_level: claimed_risk,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
    }
}

/// the `status` of the single action_requests row (each test submits EXACTLY ONE action to a fresh
/// temp DB → the unqualified SELECT is deterministic; the convention shared with policy.rs/gateway.rs).
fn action_status(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM action_requests", [], |r| r.get(0))
        .ok()
}

fn approval_id_of(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("approval_id")
}

fn action_event_types(store: &EventStore) -> Vec<String> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type.starts_with("Action"))
        .map(|e| e.event_type.clone())
        .collect()
}

/// a fake executor that reports a side effect WAS APPLIED (`side_effect_applied: true`) — the L2
/// case-2 lever: the real adapters (Phase 3/5/7/8) report `true` after a durable change; the 2.4
/// stub/catalog executors report `false` (no side effect). Drives the partial-success path.
struct AppliedExecutor;
impl ActionExecutor for AppliedExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        Ok(())
    }
    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "fake: side effect APPLIED".to_string(),
            side_effect_applied: true,
        }
    }
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: "applied".to_string(),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
}

// ---- L2 RED #1 — fail-closed: a risk>=1 action whose terminal event can't be written aborts -------

#[test]
fn audit_write_failure_aborts_risk1_action() {
    // spec(§15/§17 INV-SEC-1) — an audit-required action whose terminal event (ActionSucceeded)
    // cannot be written ABORTS: the completion txn rolls back, the action stays `executing` (the
    // executing-commit already landed; reconciled on restart by L5), NEVER acked succeeded, and no
    // ActionSucceeded persists. The StubExecutor reports NO side effect → clean fail-closed (no
    // ActionPartiallySucceeded). Fault: the §14 TerminalEventWrite checkpoint fails the terminal append.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();

    // risk-1 (session.attach_terminal) → awaiting_approval + an approval row.
    gw.submit_action(
        &mut store,
        sample_request("session.attach_terminal", RiskLevel::Level1),
    )
    .expect("submit");
    assert_eq!(action_status(&path).as_deref(), Some("awaiting_approval"));
    let appr = approval_id_of(&path);

    // arm the audit-write fault, then approve → drive execute → the terminal ActionSucceeded write fails.
    arm(FaultPoint::TerminalEventWrite);
    let err = gw
        .approve(&mut store, &appr)
        .expect_err("the terminal-event write fails → the action is NOT acked succeeded");

    // fail-closed: nothing acked succeeded; the action stays `executing` (txn-A committed it); the
    // success event never persisted (the terminal txn rolled back). The error is the §15/§17 signal.
    assert!(
        matches!(err, nexusopsd::gateway::GatewayError::AuditWriteFailed(_)),
        "fail-closed signal is AuditWriteFailed, got {err:?}"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("executing"),
        "the action stays executing (orphaned → L5 reconciles), NOT succeeded"
    );
    let types = action_event_types(&store);
    assert!(
        types.contains(&"ActionStarted".to_string()),
        "ActionStarted (txn-A) committed before the fault"
    );
    assert!(
        !types.contains(&"ActionSucceeded".to_string()),
        "NO ActionSucceeded persisted — the terminal write failed closed"
    );
    assert!(
        !types.contains(&"ActionPartiallySucceeded".to_string()),
        "the stub applied no side effect → clean rollback, no partial-success record"
    );
}

// ---- L2 RED #2 — side effect applied but the terminal event can't be written → partially_succeeded -

#[test]
fn side_effect_applied_event_unwritable_partially_succeeds() {
    // spec(§17 side-effect-applied record) — a fake executor reports the side effect WAS applied +
    // the terminal ActionSucceeded write fails (the §14 fault). The real-world change can't be rolled
    // back, so the gateway records the divergence BEST-EFFORT: emit ActionPartiallySucceeded (the
    // loud, consumer-visible audit-integrity record) + settle the action `partially_succeeded`.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    // brain.ask is catalog-risk-0 → auto-executes (no approval); the AppliedExecutor reports applied.
    // (Using the risk-0 auto-execute path proves the fail-closed/partial logic is risk-AGNOSTIC: the
    // gateway fail-closes UNIFORMLY across all execute paths — a deliberate over-satisfaction of §17's
    // "audit-required = risk≥1" scoping, NOT an INV-SEC-1 mandate (risk-0 is catalog mutation-free).)
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(AppliedExecutor));

    arm(FaultPoint::TerminalEventWrite);
    let ack = gw
        .submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0))
        .expect("partial success is a settled terminal outcome, acked (not an Err)");

    assert_eq!(
        ack.status,
        ActionRequestStatus::PartiallySucceeded,
        "a side-effect-applied + unwritable-terminal action settles partially_succeeded"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("partially_succeeded"),
        "the row settles partially_succeeded (not stuck executing — the side effect is real + recorded)"
    );
    let types = action_event_types(&store);
    assert!(
        types.contains(&"ActionPartiallySucceeded".to_string()),
        "the ActionPartiallySucceeded audit-integrity record is emitted, got {types:?}"
    );
    assert!(
        !types.contains(&"ActionSucceeded".to_string()),
        "NO ActionSucceeded — the terminal success write failed (that's the whole point)"
    );
}

// ---- L2 RED #3 — audit FULLY broken: even the partial-success record can't be written → fail closed -

#[test]
fn audit_fully_broken_stays_executing_never_partial() {
    // spec(§15/§17 ultimate fail-closed) — a side effect was applied AND both the terminal
    // ActionSucceeded write (txn-B) AND the best-effort ActionPartiallySucceeded record (txn-C) fail.
    // The gateway must NOT claim a partial-success it could not even record: it returns
    // AuditWriteFailed + leaves the action `executing` (orphaned → L5), NO ActionPartiallySucceeded.
    // (Never assert an outcome the audit log doesn't hold — the strongest form of the §17 invariant.)
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(AppliedExecutor));

    // count=2 → the fault fires on BOTH the ActionSucceeded (txn-B) AND the ActionPartiallySucceeded
    // (txn-C) terminal appends — models a fully-unavailable audit write.
    arm_n(FaultPoint::TerminalEventWrite, 2);
    let err = gw
        .submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0))
        .expect_err("audit fully broken → AuditWriteFailed, never a partial-success ack");

    assert!(
        matches!(err, nexusopsd::gateway::GatewayError::AuditWriteFailed(_)),
        "fully-broken audit returns AuditWriteFailed, got {err:?}"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("executing"),
        "stays executing (orphaned → L5) — never partially_succeeded with no record of it"
    );
    let types = action_event_types(&store);
    assert!(
        !types.contains(&"ActionSucceeded".to_string())
            && !types.contains(&"ActionPartiallySucceeded".to_string()),
        "neither terminal record persisted — no outcome is claimed that the log doesn't hold, got {types:?}"
    );
}

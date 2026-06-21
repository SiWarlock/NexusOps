//! P2.1c L2/L3 — bundled action plans (O-3). `submit_action_plan` + the plan schema (MIGRATION_8),
//! then the step-by-step / approve-all approval mechanics. The plan is a grouping over
//! `action_requests` — each step is a full ActionRequest emitting its own `Action*` events — and the
//! plan itself is a durable grouping (`action_plans` + `plan_id` FK), NOT a new event type (Q7). One
//! atomic gateway txn, whole-plan fail-closed (INV-SEC-1 / §15 / §17).

use nexusops_shared::actions::{
    ActionPlan, ActionPlanStep, ActionRequest, ApprovalMode, RequesterType, RiskLevel,
};
use nexusops_shared::gateway_ids::ActionPlanId;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor, RedactionOutcome, Redactor};
use nexusopsd::gateway::{Gateway, GatewayError};

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

fn open_with(path: &std::path::Path, redactor: Box<dyn Redactor>) -> EventStore {
    EventStore::open(
        path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        redactor,
    )
    .expect("open event store")
}

fn stub_gateway() -> Gateway {
    Gateway::new(
        Box::new(nexusopsd::gateway::policy::StubPolicy),
        Box::new(nexusopsd::gateway::executor::StubExecutor),
    )
}

/// a plan step wrapping a full ActionRequest at `risk`, with the given `inputs`.
fn step(
    step_id: &str,
    action_type: &str,
    risk: RiskLevel,
    inputs: serde_json::Value,
) -> ActionPlanStep {
    ActionPlanStep {
        step_id: step_id.to_string(),
        label: format!("step {step_id}"),
        action_request: ActionRequest {
            action_request_id: ActionRequestId::new(),
            project_id: None,
            action_type: action_type.to_string(),
            requester_type: RequesterType::User,
            requester_id: "u_local".to_string(),
            resource_refs: vec![],
            inputs,
            risk_level: risk,
            idempotency_key: None,
            fencing_token: None,
            status: ActionRequestStatus::Submitted,
            preview: None,
            created_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
        },
        required: true,
        can_skip: false,
        rollback_action_type: None,
        status: ActionRequestStatus::Submitted,
    }
}

/// a bundled plan with the given steps + approval mode.
fn plan(steps: Vec<ActionPlanStep>, mode: ApprovalMode) -> ActionPlan {
    let overall = steps
        .iter()
        .map(|s| s.action_request.risk_level)
        .max()
        .unwrap_or(RiskLevel::Level0);
    ActionPlan {
        plan_id: ActionPlanId::new(),
        title: "feature setup".to_string(),
        steps,
        dependencies: vec![],
        overall_risk: overall,
        approval_mode: mode,
    }
}

fn count(path: &std::path::Path, table: &str) -> i64 {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("count")
}

/// the `(name, notnull)` map of a table via PRAGMA table_info.
fn columns(path: &std::path::Path, table: &str) -> std::collections::BTreeMap<String, bool> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("table_info");
    stmt.query_map([], |r| {
        let name: String = r.get(1)?;
        let notnull: i64 = r.get(3)?;
        Ok((name, notnull != 0))
    })
    .expect("query")
    .map(|r| r.unwrap())
    .collect()
}

// ---- L2 RED #6 — MIGRATION_8: action_plans + the plan dimension ------------------------------

#[test]
fn test_migration_8_action_plans_and_plan_dimension() {
    // spec(§6.2 / DATA_MODEL §2.9) — MIGRATION_8 adds the action_plans table + action_requests.plan_id
    // + generalizes approvals (action_request_id NULLABLE, + plan_id) + the proj_approval_queue
    // plan dimension. (The store opens at the LATEST version — now 18 after MIGRATION_18, the P4.7/083
    // proj_integration_connection live_writes_enabled toggle.)
    let (_d, path) = temp_db();
    let store = open(&path);
    assert_eq!(
        store.user_version().unwrap(),
        18,
        "the store opens at the latest SUPPORTED_USER_VERSION (18 after MIGRATION_18, P4.7/083 live_writes)"
    );

    // action_plans — the thin grouping table + the plan-submit immutable context (Flag 2)
    let ap = columns(&path, "action_plans");
    for col in [
        "plan_id",
        "project_id",
        "requester_type",
        "requester_id",
        "title",
        "overall_risk",
        "approval_mode",
        "created_at",
    ] {
        assert!(ap.contains_key(col), "action_plans.{col} missing");
    }

    // action_requests gained a nullable plan_id FK
    let ar = columns(&path, "action_requests");
    assert!(ar.contains_key("plan_id"), "action_requests.plan_id added");
    assert!(
        !ar["plan_id"],
        "action_requests.plan_id is nullable (single action = NULL)"
    );

    // approvals generalized: action_request_id NULLABLE + plan_id added (the frozen §6.2 Approval
    // already has action_request_id: Option / plan_id — the table catches up)
    let appr = columns(&path, "approvals");
    assert!(
        !appr["action_request_id"],
        "approvals.action_request_id is now NULLABLE (a plan-level approve-all approval)"
    );
    assert!(appr.contains_key("plan_id"), "approvals.plan_id added");

    // proj_approval_queue plan dimension: action_request_id NULLABLE + plan_id
    let q = columns(&path, "proj_approval_queue");
    assert!(
        !q["action_request_id"],
        "proj_approval_queue.action_request_id is now NULLABLE"
    );
    assert!(
        q.contains_key("plan_id"),
        "proj_approval_queue.plan_id added"
    );
}

// ---- L2 RED #7 — submit_action_plan persists the plan + steps atomically (StepByStep) ---------

#[test]
fn test_submit_action_plan_persists_plan_and_steps_atomically() {
    // spec(§6.1 / AG §8 / O-3) — a 2-step StepByStep plan: 1 action_plans row + 2 action_requests
    // rows (plan_id set) + 2 ActionRequested + a per-step approval each + 2 ActionApprovalRequested;
    // returns a PlanAck with the 2 minted step action_request_ids.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.create_worktree",
                RiskLevel::Level2,
                serde_json::json!({"branch":"x"}),
            ),
            step(
                "s2",
                "github.create_pr",
                RiskLevel::Level3,
                serde_json::json!({"title":"y"}),
            ),
        ],
        ApprovalMode::StepByStep,
    );
    let plan_id = p.plan_id.as_str().to_string();
    let step_ids: Vec<String> = p
        .steps
        .iter()
        .map(|s| s.action_request.action_request_id.as_str().to_string())
        .collect();

    let ack = gw.submit_action_plan(&mut store, p).expect("submit plan");
    assert_eq!(ack.plan_id, plan_id, "PlanAck carries the aplan_ id");
    assert_eq!(ack.steps.len(), 2, "one PlanStepAck per step");
    for s in &ack.steps {
        assert!(
            step_ids.contains(&s.action_request_id),
            "PlanStepAck carries the minted step action_request_id"
        );
        assert_eq!(
            s.status,
            ActionRequestStatus::AwaitingApproval,
            "each step rests at awaiting_approval (stub policy)"
        );
    }

    assert_eq!(count(&path, "action_plans"), 1, "one plan row");
    assert_eq!(count(&path, "action_requests"), 2, "two step rows");
    assert_eq!(
        count(&path, "approvals"),
        2,
        "StepByStep → a per-step approval each"
    );

    // every step row carries plan_id; the plan groups them
    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let with_plan: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM action_requests WHERE plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(with_plan, 2, "both steps reference the plan");

    // 2 ActionRequested + 2 ActionApprovalRequested (one per step) — no plan-level event type (Q7)
    let events = store.read_all().unwrap();
    let n = |t: &str| events.iter().filter(|e| e.event_type == t).count();
    assert_eq!(n("ActionRequested"), 2, "one ActionRequested per step");
    assert_eq!(
        n("ActionApprovalRequested"),
        2,
        "a per-step approval request each"
    );
    assert!(
        !events.iter().any(|e| e.event_type.contains("Plan")),
        "no plan-level event type (Q7 — the plan is a durable grouping, not an event)"
    );
}

// ---- L2 RED #7b — ApproveAll opens ONE plan-level approval + per-critical (Q3 opening) --------

#[test]
fn test_submit_action_plan_approve_all_opens_plan_and_critical_approvals() {
    // spec(§6.2 / §11.5) — ApproveAll with a risk-4 step opens ONE plan-level approval
    // (action_request_id NULL, plan_id set) over the NON-critical steps + a SEPARATE per-step
    // approval for EACH critical (risk-4) step. The opening structure the L3 cascade resolves.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.create_worktree",
                RiskLevel::Level2,
                serde_json::json!({}),
            ),
            step(
                "s2",
                "workflow.command.invoke",
                RiskLevel::Level4,
                serde_json::json!({}),
            ), // critical
            step(
                "s3",
                "project.rescan",
                RiskLevel::Level1,
                serde_json::json!({}),
            ),
        ],
        ApprovalMode::ApproveAll,
    );
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit plan");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    // exactly ONE plan-level approval: action_request_id NULL, plan_id set
    let plan_level: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        plan_level, 1,
        "ONE plan-level approve-all approval over the non-critical steps"
    );
    // exactly ONE per-step approval — for the single critical (risk-4) step
    let per_step: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NOT NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        per_step, 1,
        "the risk-4 step gets its OWN per-step approval (never in approve-all)"
    );
    assert_eq!(
        count(&path, "approvals"),
        2,
        "1 plan-level + 1 per-critical"
    );

    // exactly 2 ActionApprovalRequested (1 plan-level + 1 per-critical) — pins the event count, not
    // just the rows (a 0-or-3 plan-level emission would slip past the row asserts).
    let n_appr_req = store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "ActionApprovalRequested")
        .count();
    assert_eq!(
        n_appr_req, 2,
        "1 plan-level + 1 per-critical approval request"
    );

    // the NEW projector branch: a plan-level approval (action_request_id NULL) folds a
    // proj_approval_queue row reading its risk/requester from action_plans (not action_requests).
    let (q_ar, q_plan, q_risk, q_status): (Option<String>, Option<String>, i64, String) = conn
        .query_row(
            "SELECT action_request_id, plan_id, risk_level, status FROM proj_approval_queue \
             WHERE action_request_id IS NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("a plan-level queue row");
    assert!(
        q_ar.is_none(),
        "plan-level queue row has NULL action_request_id"
    );
    assert_eq!(
        q_plan.as_deref(),
        Some(plan_id.as_str()),
        "plan_id sibling-read"
    );
    assert_eq!(q_risk, 4, "risk_level from action_plans.overall_risk");
    assert_eq!(q_status, "awaiting_approval", "open at awaiting_approval");
    // the queue holds 2 open rows: the plan-level approve-all + the per-critical step
    assert_eq!(
        count(&path, "proj_approval_queue"),
        2,
        "plan-level + per-critical queue rows"
    );
}

// ---- L2 RED #7c — Blocked is rejected (Mod 1 — no phantom awaiting_approval) ------------------

#[test]
fn test_submit_action_plan_blocked_rejected() {
    // spec(§6.2 / INV-SEC-1) — `Blocked` is a policy-ASSIGNED outcome (2.2 owns it), not a valid
    // proposer-submitted mode. submit_action_plan rejects it fail-closed (typed error) — never
    // leaves steps at awaiting_approval with no approval object (an inconsistent durable state).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let p = plan(
        vec![step(
            "s1",
            "git.create_worktree",
            RiskLevel::Level2,
            serde_json::json!({}),
        )],
        ApprovalMode::Blocked,
    );
    let err = gw
        .submit_action_plan(&mut store, p)
        .expect_err("Blocked is rejected fail-closed");
    assert!(
        matches!(err, GatewayError::UnsupportedApprovalMode(_)),
        "Blocked → typed fail-closed error, got {err:?}"
    );
    // nothing persisted (the txn rolled back / never ran)
    assert_eq!(count(&path, "action_plans"), 0, "no plan row");
    assert_eq!(count(&path, "action_requests"), 0, "no step rows");
    assert_eq!(store.read_all().unwrap().len(), 0, "no events");
}

// ---- L2 RED #7d — an empty plan is rejected fail-closed (no steps to group) -------------------

#[test]
fn test_submit_action_plan_empty_rejected() {
    // spec(§6.2 / INV-SEC-1) — a submitted plan must carry ≥1 step (nothing to group + no submitter
    // context to record). Rejected fail-closed before any durable write.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let p = plan(vec![], ApprovalMode::StepByStep);
    let err = gw
        .submit_action_plan(&mut store, p)
        .expect_err("an empty plan is rejected");
    assert!(
        matches!(err, GatewayError::UnsupportedApprovalMode(_)),
        "empty plan → typed fail-closed error, got {err:?}"
    );
    assert_eq!(count(&path, "action_plans"), 0, "no plan row");
    assert_eq!(store.read_all().unwrap().len(), 0, "no events");
}

// ---- L2 RED #8 — fail-closed: a step failure rolls back the WHOLE plan -----------------------

/// a Redactor that refuses any payload containing `sentinel` (delegates clean payloads to the real
/// PrefixRedactor) — drives a §15 gate failure on a SPECIFIC step to prove whole-plan rollback.
struct RefusesSentinel(&'static str);
impl Redactor for RefusesSentinel {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        if payload_json.contains(self.0) {
            RedactionOutcome {
                status: nexusops_shared::event_envelope::RedactionStatus::Unredacted,
                payload_json: payload_json.to_string(),
                engine_version: "refuses-sentinel".to_string(),
                quarantine: None,
            }
        } else {
            PrefixRedactor.redact(payload_json)
        }
    }
}

#[test]
fn test_submit_action_plan_fail_closed_rolls_back_whole_plan() {
    // spec(§15 / §17 INV-SEC-1) — if a step's authoritative event/row can't be written (the §15 gate
    // refuses step 2's inputs), the WHOLE plan aborts: no action_plans row, no partial step rows,
    // no events, no ack. The plan is ONE atomic gateway txn.
    let (_d, path) = temp_db();
    let mut store = open_with(&path, Box::new(RefusesSentinel("FAIL_STEP_2")));
    let gw = stub_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.create_worktree",
                RiskLevel::Level2,
                serde_json::json!({"ok":"clean"}),
            ),
            step(
                "s2",
                "github.create_pr",
                RiskLevel::Level3,
                serde_json::json!({"x":"FAIL_STEP_2"}),
            ),
        ],
        ApprovalMode::StepByStep,
    );
    let err = gw
        .submit_action_plan(&mut store, p)
        .expect_err("step 2 fails the §15 gate → whole plan rolls back");
    assert!(
        matches!(err, GatewayError::AuditWriteFailed(_)),
        "fail-closed → AuditWriteFailed, got {err:?}"
    );
    assert_eq!(
        count(&path, "action_plans"),
        0,
        "no plan row (atomic rollback)"
    );
    assert_eq!(
        count(&path, "action_requests"),
        0,
        "no step rows (incl. the clean step 1)"
    );
    assert_eq!(count(&path, "approvals"), 0, "no approvals");
    assert_eq!(store.read_all().unwrap().len(), 0, "no events");
}

// ---- L2 RED #9 — each step's inputs are §15-redacted at rest ---------------------------------

#[test]
fn test_plan_step_inputs_redacted() {
    // spec(§15 redaction-before-persist / rule #3/#4) — each step's inputs_json passes the §15
    // Redactor before the action_requests INSERT, exactly like a single action (the proposers are
    // untrusted). A 40-char high-entropy KEY=value secret is masked at rest.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let secret = "Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd";
    let p = plan(
        vec![
            step(
                "s1",
                "git.create_worktree",
                RiskLevel::Level2,
                serde_json::json!({"branch":"x"}),
            ),
            step(
                "s2",
                "brain.summarize_session",
                RiskLevel::Level3,
                serde_json::json!({ "env": format!("API_SECRET={secret}") }),
            ),
        ],
        ApprovalMode::StepByStep,
    );
    gw.submit_action_plan(&mut store, p).expect("submit plan");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let mut stmt = conn
        .prepare("SELECT inputs_json FROM action_requests")
        .unwrap();
    let blobs: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(
        blobs.iter().all(|b| !b.contains(secret)),
        "no step inputs leak the secret at rest: {blobs:?}"
    );
    assert!(
        blobs.iter().any(|b| b.contains("[REDACTED]")),
        "the secret step is masked in place: {blobs:?}"
    );
}

// =============================================================================================
// L3 — step-by-step + approve-all approval mechanics (the O-3 approval semantics).
// approve(approval_id): a per-step approval drives ONLY its step; a plan-level approve-all
// approval CASCADES to every non-critical step (each emitting its own Action* events tied to that
// step's action_request_id + the shared plan approval_id). Critical (risk-4) is NEVER cascaded.
// =============================================================================================

/// the per-step approval_id for a given step `action_request_id`.
fn step_approval_id(path: &std::path::Path, action_request_id: &str) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(
        "SELECT approval_id FROM approvals WHERE action_request_id = ?1",
        [action_request_id],
        |r| r.get(0),
    )
    .expect("a per-step approval exists")
}

/// the plan-level (action_request_id NULL) approval_id for a plan.
fn plan_approval_id(path: &std::path::Path, plan_id: &str) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(
        "SELECT approval_id FROM approvals WHERE action_request_id IS NULL AND plan_id = ?1",
        [plan_id],
        |r| r.get(0),
    )
    .expect("a plan-level approval exists")
}

/// a step's current `action_requests.status`.
fn step_status(path: &std::path::Path, action_request_id: &str) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(
        "SELECT status FROM action_requests WHERE action_request_id = ?1",
        [action_request_id],
        |r| r.get(0),
    )
    .expect("the step row exists")
}

/// the `Action*` event types correlated to a given step (by the action_request_id envelope FK).
fn step_event_types(store: &EventStore, action_request_id: &str) -> Vec<String> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| {
            e.event_type.starts_with("Action")
                && e.action_request_id.as_ref().map(|x| x.as_str()) == Some(action_request_id)
        })
        .map(|e| e.event_type.clone())
        .collect()
}

// ---- L3 RED #11 — StepByStep: approve one step drives ONLY that step -------------------------

#[test]
fn test_step_by_step_approve_one_step() {
    // spec(§6.1 / §11.5) — a StepByStep plan opens a per-step approval each; approve(step_approval)
    // drives ONLY that step to succeeded; the sibling step stays awaiting_approval.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let s1 = step(
        "s1",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    );
    let s2 = step(
        "s2",
        "github.create_pr",
        RiskLevel::Level3,
        serde_json::json!({}),
    );
    let ar1 = s1.action_request.action_request_id.as_str().to_string();
    let ar2 = s2.action_request.action_request_id.as_str().to_string();
    gw.submit_action_plan(&mut store, plan(vec![s1, s2], ApprovalMode::StepByStep))
        .expect("submit");

    gw.approve(&mut store, &step_approval_id(&path, &ar1))
        .expect("approve step 1");

    assert_eq!(
        step_status(&path, &ar1),
        "succeeded",
        "step 1 drove to succeeded"
    );
    assert_eq!(
        step_status(&path, &ar2),
        "awaiting_approval",
        "step 2 untouched — StepByStep approves each independently"
    );
    // step 2 emitted NO execution events
    let s2_events = step_event_types(&store, &ar2);
    for forbidden in ["ActionApproved", "ActionStarted", "ActionSucceeded"] {
        assert!(
            !s2_events.contains(&forbidden.to_string()),
            "step 2 must not {forbidden} — only step 1 was approved"
        );
    }
}

// ---- L3 RED #12 — ApproveAll cascade executes non-critical, leaves critical ------------------

#[test]
fn test_approve_all_excludes_critical() {
    // spec(§6.2/§11.5 "critical never in approve-all" — the safety pin) — approving the plan-level
    // approve-all approval executes EVERY non-critical step; the risk-4 step stays awaiting_approval
    // (covered by its OWN per-step approval, never by approve-all).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let s1 = step(
        "s1",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    );
    let s2 = step(
        "s2",
        "workflow.command.invoke",
        RiskLevel::Level4,
        serde_json::json!({}),
    ); // critical
    let s3 = step(
        "s3",
        "project.rescan",
        RiskLevel::Level1,
        serde_json::json!({}),
    );
    let (ar1, ar2, ar3) = (
        s1.action_request.action_request_id.as_str().to_string(),
        s2.action_request.action_request_id.as_str().to_string(),
        s3.action_request.action_request_id.as_str().to_string(),
    );
    let p = plan(vec![s1, s2, s3], ApprovalMode::ApproveAll);
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit");

    gw.approve(&mut store, &plan_approval_id(&path, &plan_id))
        .expect("approve the plan-level approve-all");

    assert_eq!(
        step_status(&path, &ar1),
        "succeeded",
        "non-critical s1 executed"
    );
    assert_eq!(
        step_status(&path, &ar3),
        "succeeded",
        "non-critical s3 executed"
    );
    assert_eq!(
        step_status(&path, &ar2),
        "awaiting_approval",
        "the critical (risk-4) step is NEVER in approve-all — stays awaiting (safety pin)"
    );
}

// ---- L3 RED #13 — the cascade emits each covered step's own Action* events -------------------

#[test]
fn test_approve_all_cascades_each_step_event() {
    // spec(§15 every-mutation-has-an-event / AG §17.1) — the cascade emits a per-step ActionApproved
    // + ActionStarted + ActionSucceeded for EACH covered step, each tied to that step's
    // action_request_id + the SHARED plan approval_id.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let s1 = step(
        "s1",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    );
    let s3 = step(
        "s3",
        "project.rescan",
        RiskLevel::Level1,
        serde_json::json!({}),
    );
    let (ar1, ar3) = (
        s1.action_request.action_request_id.as_str().to_string(),
        s3.action_request.action_request_id.as_str().to_string(),
    );
    let p = plan(vec![s1, s3], ApprovalMode::ApproveAll);
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit");
    let plan_appr = plan_approval_id(&path, &plan_id);

    gw.approve(&mut store, &plan_appr).expect("approve cascade");

    for ar in [&ar1, &ar3] {
        let ev = step_event_types(&store, ar);
        for t in ["ActionApproved", "ActionStarted", "ActionSucceeded"] {
            assert!(ev.contains(&t.to_string()), "step {ar} emitted {t}");
        }
    }
    // each covered step's ActionApproved carries the SHARED plan approval_id (envelope FK)
    let approved: Vec<_> = store
        .read_all()
        .unwrap()
        .into_iter()
        .filter(|e| e.event_type == "ActionApproved")
        .collect();
    assert_eq!(approved.len(), 2, "one ActionApproved per covered step");
    assert!(
        approved
            .iter()
            .all(|e| e.approval_id.as_deref() == Some(plan_appr.as_str())),
        "every covered step's ActionApproved ties to the shared plan approval_id"
    );
}

// ---- L3 RED #14 — plan deny + plan expiry never execute -------------------------------------

#[test]
fn test_plan_deny_and_expiry_no_execution() {
    // spec(§17 / AG §8.8) — deny(plan_approval, reason) → covered steps terminal (denied), the
    // executor is NEVER invoked; an expired plan approval → ActionExpired per covered step, none
    // executed (fake clock — a past expiry stamped on the plan-level approval).

    // deny path
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let s1 = step(
        "s1",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    );
    let s2 = step(
        "s2",
        "project.rescan",
        RiskLevel::Level1,
        serde_json::json!({}),
    );
    let (ar1, ar2) = (
        s1.action_request.action_request_id.as_str().to_string(),
        s2.action_request.action_request_id.as_str().to_string(),
    );
    let p = plan(vec![s1, s2], ApprovalMode::ApproveAll);
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit");
    gw.deny(
        &mut store,
        &plan_approval_id(&path, &plan_id),
        "not allowed",
    )
    .expect("deny the plan");
    let deny_appr = plan_approval_id(&path, &plan_id);
    for ar in [&ar1, &ar2] {
        assert_eq!(
            step_status(&path, ar),
            "denied",
            "covered step denied (terminal)"
        );
        let ev = step_event_types(&store, ar);
        assert!(
            ev.contains(&"ActionDenied".to_string()),
            "each covered step emits ActionDenied (not just a silent status flip)"
        );
        for forbidden in ["ActionStarted", "ActionSucceeded"] {
            assert!(
                !ev.contains(&forbidden.to_string()),
                "denied step {ar} must NOT execute — {forbidden} forbidden"
            );
        }
    }
    // every covered step's ActionDenied carries the SHARED plan approval_id (envelope FK)
    let denied: Vec<_> = store
        .read_all()
        .unwrap()
        .into_iter()
        .filter(|e| e.event_type == "ActionDenied")
        .collect();
    assert_eq!(denied.len(), 2, "one ActionDenied per covered step");
    assert!(
        denied
            .iter()
            .all(|e| e.approval_id.as_deref() == Some(deny_appr.as_str())),
        "every covered step's ActionDenied ties to the shared plan approval_id"
    );

    // expiry path — a past expiry stamped on the plan-level approval
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    let s1b = step(
        "s1",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    );
    let arb = s1b.action_request.action_request_id.as_str().to_string();
    let p2 = plan(vec![s1b], ApprovalMode::ApproveAll);
    let plan_id2 = p2.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store2, p2).expect("submit");
    {
        let c = rusqlite::Connection::open(&path2).expect("fixture conn");
        c.busy_timeout(std::time::Duration::from_secs(2)).unwrap();
        c.execute(
            "UPDATE approvals SET expires_at = '2020-01-01T00:00:00Z' WHERE action_request_id IS NULL",
            [],
        )
        .expect("stamp past expiry on the plan-level approval");
    }
    gw.approve(&mut store2, &plan_approval_id(&path2, &plan_id2))
        .expect("approve-expired plan");
    assert_eq!(
        step_status(&path2, &arb),
        "expired",
        "an expired plan approval expires its steps"
    );
    let ev = step_event_types(&store2, &arb);
    assert!(
        ev.contains(&"ActionExpired".to_string()),
        "ActionExpired emitted"
    );
    for forbidden in ["ActionStarted", "ActionSucceeded"] {
        assert!(
            !ev.contains(&forbidden.to_string()),
            "an expired plan approval must NOT execute — {forbidden} forbidden"
        );
    }
}

// ---- L3 RED #15 — re-resolving a cascaded plan-level approval is rejected (R-9) --------------

#[test]
fn test_reapprove_cascaded_plan_rejected() {
    // spec(§5.1 R-9) — once a plan-level approve-all approval is resolved (cascaded → approved), a
    // second approve/deny on it is rejected with a typed IllegalTransition (the approval is terminal,
    // its covered steps already executed) — the cascade can never re-execute already-resolved steps.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let s1 = step(
        "s1",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    );
    let p = plan(vec![s1], ApprovalMode::ApproveAll);
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit");
    let plan_appr = plan_approval_id(&path, &plan_id);
    gw.approve(&mut store, &plan_appr)
        .expect("first approve cascades");

    let err = gw
        .approve(&mut store, &plan_appr)
        .expect_err("re-approving a resolved plan-level approval is rejected");
    assert!(
        matches!(err, GatewayError::IllegalTransition { .. }),
        "second approve → IllegalTransition (the approval is terminal), got {err:?}"
    );
}

// ---- P4.0b-ui2 (②-mini) C1 — the plan-level approve-all approval persists NULL policy_decision ----

#[test]
fn test_plan_level_approval_persists_null_policy_decision() {
    // spec(§6.2 / Q2) — a plan-level approve-all approval (`action_request_id` NULL, `plan_id` set)
    // persists `policy_decision_json` = NULL (the single-action path carries the real decision;
    // plan-level sourcing is a follow-on) and the projector carries NULL without error. Pins that the
    // `approval::insert` `tx`→`gtx` + policy-param sig change did NOT regress the plan call site.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let s1 = step(
        "s1",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    );
    let s3 = step(
        "s3",
        "project.rescan",
        RiskLevel::Level1,
        serde_json::json!({}),
    );
    let p = plan(vec![s1, s3], ApprovalMode::ApproveAll);
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit");
    let appr = plan_approval_id(&path, &plan_id);

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let pd_appr: Option<String> = conn
        .query_row(
            "SELECT policy_decision_json FROM approvals WHERE approval_id = ?1",
            [&appr],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        pd_appr.is_none(),
        "a plan-level approve-all approval persists NULL policy_decision (Q2)"
    );
    let pd_proj: Option<String> = conn
        .query_row(
            "SELECT policy_decision_json FROM proj_approval_queue WHERE approval_id = ?1",
            [&appr],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        pd_proj.is_none(),
        "the projector carries NULL for the plan-level row (no error)"
    );
}

// ---- P4.0b-ui2 (②-mini) C2 — the ApprovalQueue projection is served TYPED (pin #2; Some + None) ----

#[test]
fn test_get_projection_serves_typed_approval_row() {
    // spec(§6.1 / pin #2) — the ApprovalQueue projection is served TYPED via `read_approval_queue_typed`
    // (no loose JSON on the human-approval path). It must handle BOTH a Some (single-action, the C1
    // persisted decision) AND a None (plan-level approve-all, NULL policy_decision) row — both
    // deserialize into the frozen `ApprovalQueueRow`, neither errors. Guards the DB-TEXT→
    // Option<PolicyDecision> mapping: a wrong NULL-unwrap would break the ENTIRE read whenever a
    // plan-level approval is present (an availability bug on the safety-critical approval path).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    // a single-action approval → Some(policy_decision) (C1 persists the StubPolicy decision).
    let single = step(
        "solo",
        "git.create_worktree",
        RiskLevel::Level2,
        serde_json::json!({}),
    )
    .action_request;
    gw.submit_action(&mut store, single)
        .expect("submit single action");
    // an ApproveAll plan (one non-critical step) → a plan-level approval with NULL policy_decision (Q2).
    let s1 = step(
        "s1",
        "project.rescan",
        RiskLevel::Level1,
        serde_json::json!({}),
    );
    gw.submit_action_plan(&mut store, plan(vec![s1], ApprovalMode::ApproveAll))
        .expect("submit plan");

    let rows = nexusopsd::ipc::read_approval_queue_typed(&path).expect("typed approval-queue read");
    assert_eq!(
        rows.len(),
        2,
        "both approvals served (single-action + plan-level)"
    );
    // BOTH deserialized into the frozen ApprovalQueueRow (the read never errored on the NULL).
    let some_count = rows.iter().filter(|r| r.policy_decision.is_some()).count();
    let none_count = rows.iter().filter(|r| r.policy_decision.is_none()).count();
    assert_eq!(
        some_count, 1,
        "the single-action row carries Some(PolicyDecision)"
    );
    assert_eq!(
        none_count, 1,
        "the plan-level row carries None (NULL policy_decision) — the read handled NULL"
    );
    for r in &rows {
        assert_eq!(
            r.status,
            nexusops_shared::status::Approval::AwaitingApproval,
            "typed Approval status (no loose JSON)"
        );
    }
}

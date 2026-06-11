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
    // plan dimension; SUPPORTED_USER_VERSION == 8.
    let (_d, path) = temp_db();
    let store = open(&path);
    assert_eq!(store.user_version().unwrap(), 8, "MIGRATION_8 raises to v8");

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
                "git.force_push",
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
                "brain.send",
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

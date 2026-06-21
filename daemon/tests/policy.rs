//! P2.2 — the catalog-driven policy engine (the policy half of the INV-SEC-1 chokepoint).
//!
//! **L2 (RED #4-7):** `CatalogPolicy` resolves each action's risk from the §6.3 `ActionTypeCatalog`
//! (AUTHORITATIVE — never the requester-supplied `risk_level`) → the §6.2 `PolicyDecision`; it is
//! wired into the `Gateway` (swapping `StubPolicy`, which stays test-only); and the recorded
//! `action_requests.risk_level` + the `ActionRequested` event are reconciled to the catalog risk at
//! submit (Q5 — the audit trail records the TRUE risk, not the proposer's claim).
//!
//! **L3 (RED #8-10, INV-SEC-1-critical, added after the L2 commit):** the risk-0 `allow` →
//! `policy_decided → queued` auto-execute path (the FIRST no-human-approval execution path, gated
//! STRICTLY on `allow` + catalog-risk-0); the "no non-zero / non-allow auto-queue" safety pin; and
//! the §11.5 approve-all critical-exclusion migrated onto the catalog-authoritative risk.

use nexusops_shared::actions::{
    ActionPlan, ActionPlanStep, ActionRequest, ApprovalMode, PolicyDecision, PolicyDecisionStatus,
    RequesterType, RiskLevel,
};
use nexusops_shared::gateway_ids::ActionPlanId;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::StubExecutor;
use nexusopsd::gateway::policy::{CatalogPolicy, PolicyEngine};
use nexusopsd::gateway::{Gateway, GatewayError};
use nexusopsd::idgen::UlidGen;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

/// open a store (runs migrations through MIGRATION_8 — action_requests/approvals/action_plans).
fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// the PRODUCTION Gateway: the catalog-driven policy engine + the stub executor (real executors are
/// 2.3). This is the wiring `main.rs` adopts (swap `StubPolicy` → `CatalogPolicy`).
fn catalog_gateway() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(StubExecutor))
}

/// a §6.2 ActionRequest fixture; `risk` is the requester's CLAIMED risk (recorded, never trusted —
/// the catalog is authoritative).
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

/// the `risk_level` integer of the single action_requests row, over a read-only connection.
fn persisted_risk(path: &std::path::Path) -> i64 {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT risk_level FROM action_requests", [], |r| r.get(0))
        .expect("risk_level")
}

// ---- L2 RED #4 — the policy resolves risk from the CATALOG, not the requester's claim -----------

#[test]
fn test_catalog_policy_resolves_risk_from_catalog_not_request() {
    // spec(§15 recorded-not-trusted) — CatalogPolicy::decide resolves risk from the §6.3 catalog,
    // NEVER the requester-supplied `risk_level`. `github.create_pr` is catalog-risk-3 → require_approval;
    // a proposer claiming risk-0 (which WOULD auto-allow if the claim were trusted) must still be gated.
    let policy = CatalogPolicy;
    let req = sample_request("github.create_pr", RiskLevel::Level0); // claim 0, catalog 3
    let d = policy.decide(&req);
    assert_eq!(
        d.status,
        PolicyDecisionStatus::RequireApproval,
        "catalog risk-3 → require_approval, NOT the claimed risk-0 → allow"
    );
}

// ---- L2 RED #5 — the risk → decision mapping (0→allow, 1-3→approval, 4→step, unknown→deny) ------

#[test]
fn test_catalog_policy_decision_per_risk() {
    // spec(§6.3 / AG§7 / AG§12) — the catalog-authoritative risk → PolicyDecision mapping: risk-0 →
    // allow; risk 1/2/3 → require_approval; risk-4 (critical) → require_step_approval; an action_type
    // absent from the catalog → deny (fail-closed, §15 — never a default-allow).
    let policy = CatalogPolicy;
    use PolicyDecisionStatus::*;
    let cases = [
        ("brain.ask", Allow),                             // risk-0 (read/propose-only)
        ("session.attach_terminal", RequireApproval),     // risk-1
        ("git.create_worktree", RequireApproval),         // risk-2
        ("github.create_pr", RequireApproval),            // risk-3
        ("workflow.command.invoke", RequireStepApproval), // risk-4 (critical)
        ("git.force_push", Deny),                         // not in the catalog → fail-closed deny
    ];
    for (action_type, expected) in cases {
        // the CLAIMED risk is deliberately a constant wrong value — decide must ignore it entirely.
        let d = policy.decide(&sample_request(action_type, RiskLevel::Level2));
        assert_eq!(d.status, expected, "{action_type} → {expected:?}");
    }

    // an unknown-type submit routes the `deny` decision to the HONEST GatewayError::PolicyDenied
    // (→ §6.4 `policy_denied`), NOT `UnsupportedPolicyDecision`/`precondition_stale` — a deny is a
    // routed policy outcome now, not an unrouted 2.1b decision (a deny must not read as "stale").
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let err = gw
        .submit_action(
            &mut store,
            sample_request("git.force_push", RiskLevel::Level0),
        )
        .expect_err("an uncatalogued action_type is denied fail-closed");
    assert!(
        matches!(err, GatewayError::PolicyDenied),
        "unknown-type deny → PolicyDenied (policy_denied), got {err:?}"
    );
    // fail-closed rollback: the deny returns INSIDE the gateway txn → nothing persists (no row, no
    // event). Pins the rollback, not just the error variant (the audit-integrity posture).
    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let rows: i64 = conn
        .query_row("SELECT count(*) FROM action_requests", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0, "a denied submit persists no action_requests row");
    assert_eq!(
        store.read_all().unwrap().len(),
        0,
        "a denied submit persists no event (txn rollback)"
    );
}

// =================================================================================================
// P4.0b-1 L2 — the risk-0 session-lifecycle relaxation + the 5 protective pins (CAT-1; away-ruled).
// =================================================================================================

/// `sample_request` with an explicit requester (PIN-e — the UI/IPC-only gate).
fn request_from(action_type: &str, requester: RequesterType) -> ActionRequest {
    ActionRequest {
        requester_type: requester,
        ..sample_request(action_type, RiskLevel::Level0)
    }
}

#[test]
fn test_session_create_kill_auto_allow() {
    // spec(§6.3 away-ruled) — session.create/kill are risk-0 → Allow (audited auto-allow); a UI/User
    // requester auto-executes (no approval). PINs (a)-(e) constrain HOW it stays safe (below).
    let policy = CatalogPolicy;
    assert_eq!(
        policy
            .decide(&sample_request("session.create", RiskLevel::Level0))
            .status,
        PolicyDecisionStatus::Allow
    );
    assert_eq!(
        policy
            .decide(&sample_request("session.kill", RiskLevel::Level0))
            .status,
        PolicyDecisionStatus::Allow
    );
}

#[test]
fn test_profile_change_requires_approval() {
    // PIN (c) — spec(§15 #8) — session.profile_change is risk-2 → RequireApproval; the no-silent-
    // account-hop gate lives on the CHANGE, never the routine start. Pin the DISCRETE PIN-c facts (the
    // catalog risk is 2 + it's NOT on the risk-0 auto-execute allowlist → never auto-executed), not
    // just the generic risk-2→approval arm any risk-2 type would satisfy.
    assert_eq!(
        nexusops_shared::catalog::lookup("session.profile_change")
            .unwrap()
            .locked_risk,
        RiskLevel::Level2,
        "session.profile_change is catalog-risk-2 (the §15 #8 account-hop gate)"
    );
    assert!(
        !nexusopsd::gateway::policy::risk0_auto_execute_permitted("session.profile_change"),
        "session.profile_change is NOT on the risk-0 auto-execute allowlist"
    );
    let policy = CatalogPolicy;
    assert_eq!(
        policy
            .decide(&sample_request("session.profile_change", RiskLevel::Level0))
            .status,
        PolicyDecisionStatus::RequireApproval,
        "profile-change is approval-gated (no silent account-hop)"
    );
}

#[test]
fn test_risk0_relaxation_is_narrow() {
    // PIN (d) — spec(LESSON 19) — the risk-0 relaxation is NARROW to session-lifecycle. A non-session
    // MUTATING type (git.create_worktree, catalog-risk-2) the requester CLAIMS is risk-0 is NOT
    // auto-allowed (the catalog authority resolves its real risk → RequireApproval).
    let policy = CatalogPolicy;
    assert_eq!(
        policy
            .decide(&sample_request("git.create_worktree", RiskLevel::Level0))
            .status,
        PolicyDecisionStatus::RequireApproval,
        "a non-session mutation can't ride the risk-0 relaxation (catalog-authoritative)"
    );
    // the explicit allowlist guard (lead-ruled, belt-and-suspenders over the catalog + LESSON 19
    // re-gate): only the allowlisted risk-0 types may auto-execute; a non-allowlisted one fails closed.
    assert!(nexusopsd::gateway::policy::risk0_auto_execute_permitted(
        "session.create"
    ));
    assert!(nexusopsd::gateway::policy::risk0_auto_execute_permitted(
        "session.kill"
    ));
    assert!(
        !nexusopsd::gateway::policy::risk0_auto_execute_permitted("some.future.risk0_mutation"),
        "a risk-0 type NOT on the allowlist fails closed — admitting one forces a deliberate edit"
    );
}

#[test]
fn test_risk0_allowlist_matches_catalog() {
    // PIN (d) consistency — the explicit auto-execute allowlist MUST equal the catalog's risk-0 set:
    // a type is allowlisted IFF it is catalog-risk-0. So (1) every catalog risk-0 type is allowlisted
    // (else it silently fail-closes — this is the LOUD dev-time catch) and (2) the allowlist can't
    // admit a non-risk-0 type. Admitting a NEW auto-executing type therefore forces a deliberate edit
    // to BOTH the catalog risk AND the allowlist — a future risk-0 mutation can't silently auto-execute.
    use nexusops_shared::catalog::{lookup, AGENT_MUTATION_ACTION_TYPES, MVP_ACTION_TYPES};
    // forward — every catalogued type is allowlisted IFF it is catalog-risk-0.
    for at in MVP_ACTION_TYPES.iter().chain(AGENT_MUTATION_ACTION_TYPES) {
        let is_catalog_risk0 = lookup(at).unwrap().locked_risk == RiskLevel::Level0;
        assert_eq!(
            is_catalog_risk0,
            nexusopsd::gateway::policy::risk0_auto_execute_permitted(at),
            "the risk-0 auto-execute allowlist must EXACTLY track the catalog risk-0 set: {at}"
        );
    }
    // reverse — every allowlist entry IS a catalog risk-0 type (catches a phantom / typo'd allowlist
    // entry, e.g. `session.creat`, that the forward sweep alone would never visit).
    for at in nexusopsd::gateway::policy::risk0_auto_execute_allowlist() {
        assert_eq!(
            lookup(at).map(|e| e.locked_risk),
            Some(RiskLevel::Level0),
            "every risk-0 allowlist entry must be a catalog risk-0 type: {at}"
        );
    }
}

#[test]
fn test_session_create_rejects_agent_brain_requester() {
    // PIN (e) — spec(§15 #8 / 043) — session.create/kill are UI/IPC-initiated only; an AgentSession /
    // ProjectBrain / WorkflowPack requester is DENIED (agents stay governed by the 043
    // AgentMutationPolicy, never spawning a session at risk-0).
    let policy = CatalogPolicy;
    for requester in [
        RequesterType::AgentSession,
        RequesterType::ProjectBrain,
        RequesterType::WorkflowPack,
    ] {
        assert_eq!(
            policy
                .decide(&request_from("session.create", requester))
                .status,
            PolicyDecisionStatus::Deny,
            "{requester:?} session.create is denied (UI/IPC-only)"
        );
        assert_eq!(
            policy
                .decide(&request_from("session.kill", requester))
                .status,
            PolicyDecisionStatus::Deny,
            "{requester:?} session.kill is denied (UI/IPC-only)"
        );
    }
    // a User (the desktop UI) is the permitted requester → auto-allows.
    assert_eq!(
        policy
            .decide(&request_from("session.create", RequesterType::User))
            .status,
        PolicyDecisionStatus::Allow,
        "a User (UI/IPC) session.create auto-allows"
    );
}

// ---- D9 (P4.7) — github.merge_pr is UI/IPC-requester-only (F2; cat-1) ----------------------------

#[test]
fn test_merge_pr_denied_for_non_ui_requester() {
    // spec(§15 #8 / F2 — the PIN-e precedent generalized to github mutations) — github.merge_pr is a
    // 🔴 cat-1 GitHub WRITE: it is UI/human-initiated ONLY. An AgentSession / ProjectBrain / WorkflowPack
    // requester is DENIED *before* risk resolution — no agent/Brain may merge a PR (F2, USER-steered).
    let policy = CatalogPolicy;
    for requester in [
        RequesterType::AgentSession,
        RequesterType::ProjectBrain,
        RequesterType::WorkflowPack,
        RequesterType::SystemPolicy,
    ] {
        assert_eq!(
            policy
                .decide(&request_from("github.merge_pr", requester))
                .status,
            PolicyDecisionStatus::Deny,
            "{requester:?} github.merge_pr is denied (UI/IPC-only, F2)"
        );
    }
}

#[test]
fn test_set_live_writes_denied_for_non_ui_requester() {
    // spec(§15 / 083 Q3 — the GITHUB_MUTATION gate extended) — integration.set_live_writes flips a
    // connection's live-writes AUTHORIZATION → UI/human-initiated ONLY. An agent/Brain/pack/system
    // requester is DENIED *before* risk resolution (no agent may self-enable live external writes).
    let policy = CatalogPolicy;
    for requester in [
        RequesterType::AgentSession,
        RequesterType::ProjectBrain,
        RequesterType::WorkflowPack,
        RequesterType::SystemPolicy,
    ] {
        assert_eq!(
            policy
                .decide(&request_from("integration.set_live_writes", requester))
                .status,
            PolicyDecisionStatus::Deny,
            "{requester:?} integration.set_live_writes is denied (UI/IPC-only)"
        );
    }
}

#[test]
fn test_merge_pr_require_approval_for_ui_requester() {
    // spec(§6.2 risk-3 / F1) — a User (the desktop UI) requester is the permitted initiator → the merge
    // routes to RequireApproval (risk-3: not auto-execute, not on the risk-0 allowlist) — NOT auto-allowed,
    // NOT denied. (RemoteClient is the other UI/IPC requester; deferred surface.)
    let policy = CatalogPolicy;
    assert_eq!(
        policy
            .decide(&request_from("github.merge_pr", RequesterType::User))
            .status,
        PolicyDecisionStatus::RequireApproval,
        "a User (UI/IPC) github.merge_pr → RequireApproval (risk-3, every merge a fresh approval)"
    );
}

// ---- D10 (P4.7) — github.submit_review is UI/IPC-requester-only (F2; the D9 gate EXTENDED) ----------

#[test]
fn test_submit_review_denied_for_non_ui_requester() {
    // spec(§15 #8 / F2) — github.submit_review joins D9's GITHUB_MUTATION_TYPES gate: an Agent/Brain/Pack/
    // System requester → Deny BEFORE risk resolution (a review verdict is a human attestation; an `approve`
    // carries merge-gate power → no agent/Brain-posted review). The D9 deny-before-risk gate REUSED.
    let policy = CatalogPolicy;
    for requester in [
        RequesterType::AgentSession,
        RequesterType::ProjectBrain,
        RequesterType::WorkflowPack,
        RequesterType::SystemPolicy,
    ] {
        assert_eq!(
            policy
                .decide(&request_from("github.submit_review", requester))
                .status,
            PolicyDecisionStatus::Deny,
            "{requester:?} github.submit_review is denied (UI/IPC-only, F2)"
        );
    }
}

#[test]
fn test_submit_review_require_approval_for_ui_requester() {
    // spec(§6.2 risk-3 / F1) — a User requester → RequireApproval (risk-3, not auto-execute, not on the
    // risk-0 allowlist) — every submit a fresh per-action approval.
    let policy = CatalogPolicy;
    assert_eq!(
        policy
            .decide(&request_from("github.submit_review", RequesterType::User))
            .status,
        PolicyDecisionStatus::RequireApproval,
        "a User (UI/IPC) github.submit_review → RequireApproval (risk-3)"
    );
}

// =================================================================================================
// L3 — the risk-0 `allow` auto-execute path (the FIRST no-human-approval execution path) + the
// §11.5 approve-all critical-exclusion migrated onto the catalog-authoritative risk. INV-SEC-1.
// =================================================================================================

/// COUNT(*) of a table over a read-only connection (sees committed rows).
fn count(path: &std::path::Path, table: &str) -> i64 {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("count")
}

/// the `status` of the single action_requests row, over a read-only connection.
fn action_status(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM action_requests", [], |r| r.get(0))
        .ok()
}

/// the `Action*` event types in the log (insertion order).
fn action_event_types(store: &EventStore) -> Vec<String> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type.starts_with("Action"))
        .map(|e| e.event_type.clone())
        .collect()
}

/// a plan step wrapping a full ActionRequest at the CLAIMED `risk`, with the given `inputs`.
fn step(
    step_id: &str,
    action_type: &str,
    claimed_risk: RiskLevel,
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
            risk_level: claimed_risk,
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

/// an ADVERSARIAL policy double: returns `allow` for EVERY action, even a non-risk-0 one. Pins the
/// pipeline's strict catalog-risk-0 re-gate (defense-in-depth) — a buggy/malicious policy must NEVER
/// open the auto-queue for a non-risk-0 action.
struct AllowAllPolicy;
impl PolicyEngine for AllowAllPolicy {
    fn decide(&self, _req: &ActionRequest) -> PolicyDecision {
        PolicyDecision {
            status: PolicyDecisionStatus::Allow,
            reasons: vec!["adversarial double: allow everything".to_string()],
            required_approvals: vec![],
            constraints: vec![],
            safer_alt: None,
        }
    }
}

// ---- L3 RED #8 — a risk-0 `allow` action auto-executes with no human gate ----------------------

#[test]
fn test_risk0_allow_auto_executes_without_approval() {
    // spec(§6.1/§5.1) — a risk-0 `allow` action drives submitted → policy_decided → queued →
    // executing → succeeded with NO approvals row + NO ActionApprovalRequested; the execution family
    // (ActionStarted + ActionSucceeded) emits. `brain.ask` is catalog-risk-0; the claimed risk-2 is
    // ignored (catalog-authoritative).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let ack = gw
        .submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level2))
        .expect("a risk-0 allow auto-executes");

    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "risk-0 allow → succeeded (auto-executed)"
    );
    assert_eq!(action_status(&path), Some("succeeded".to_string()));
    assert_eq!(
        count(&path, "approvals"),
        0,
        "NO approval row — no human gate"
    );

    // EXACTLY the auto-execute sequence, in order — pins presence AND ordering AND the absence of any
    // approval event (an ActionApprovalRequested/ActionApproved would make the vec differ).
    assert_eq!(
        action_event_types(&store),
        vec!["ActionRequested", "ActionStarted", "ActionSucceeded"],
        "the risk-0 allow path emits exactly requested→started→succeeded, no approval event"
    );
}

// ---- L3 RED #9 — no non-zero / non-allow action ever auto-queues (the INV-SEC-1 pin) -----------

#[test]
fn test_nonzero_risk_never_auto_queues() {
    // spec(§15 INV-SEC-1; §14 extends) — the policy_decided→queued auto-execute edge is gated STRICTLY
    // on `allow` + catalog-risk-0. No catalog-risk≥1 action and no non-allow decision ever reaches
    // queued without an approval.

    // (a) the REAL policy: every non-zero-risk action → require_approval / require_step_approval →
    //     rests at awaiting_approval, NEVER executes (an approval is opened, no ActionStarted).
    for action_type in [
        "session.attach_terminal", // risk-1
        "git.create_worktree",     // risk-2
        "github.create_pr",        // risk-3
        "workflow.command.invoke", // risk-4
    ] {
        let (_d, path) = temp_db();
        let mut store = open(&path);
        let gw = catalog_gateway();
        // under-claim risk-0 — the catalog risk still gates it.
        let ack = gw
            .submit_action(&mut store, sample_request(action_type, RiskLevel::Level0))
            .expect("submit");
        assert_eq!(
            ack.status,
            ActionRequestStatus::AwaitingApproval,
            "{action_type}: a non-zero-risk action rests at awaiting_approval (never auto-queues)"
        );
        assert_eq!(
            count(&path, "approvals"),
            1,
            "{action_type}: an approval was opened"
        );
        assert!(
            !action_event_types(&store).contains(&"ActionStarted".to_string()),
            "{action_type}: never executes without an approval"
        );
    }

    // (b) DEFENSE-IN-DEPTH: even a buggy policy returning `allow` for a non-risk-0 action must NOT
    //     auto-queue — the pipeline's strict catalog-risk-0 re-gate fails closed (nothing persists).
    //     Exercised across the risk range (risk-3 AND the critical risk-4) so the re-gate is pinned
    //     for more than a single "≥ 1" value.
    for action_type in [
        "github.create_pr",        // catalog risk-3
        "workflow.command.invoke", // catalog risk-4 (critical)
    ] {
        let (_d, path) = temp_db();
        let mut store = open(&path);
        let gw = Gateway::new(Box::new(AllowAllPolicy), Box::new(StubExecutor));
        let err = gw
            .submit_action(&mut store, sample_request(action_type, RiskLevel::Level0))
            .expect_err("a non-risk-0 `allow` is re-gated fail-closed");
        assert!(
            matches!(err, GatewayError::UnsupportedPolicyDecision(_)),
            "{action_type}: the strict re-gate refuses to auto-queue a non-risk-0 allow, got {err:?}"
        );
        assert_eq!(
            count(&path, "action_requests"),
            0,
            "{action_type}: the re-gate rolled the txn back — nothing persisted"
        );
        assert!(
            !action_event_types(&store).contains(&"ActionStarted".to_string()),
            "{action_type}: a re-gated action never executes"
        );
    }
}

// ---- L3 RED #10 — approve-all excludes a catalog-critical step (the §11.5 safety-pin migration) -

#[test]
fn test_approve_all_excludes_catalog_critical() {
    // spec(§6.2/§11.5 on AUTHORITATIVE risk; closes the 2.1c-L3 follow-on) — an ApproveAll plan step
    // claiming risk_level=0 whose CATALOG risk is 4 (workflow.command.invoke) is EXCLUDED from the
    // plan-level approve-all approval + gets its OWN per-step approval, even though the proposer
    // under-claimed (the §11.5 exclusion keys off catalog risk, not the claim).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.create_worktree",
                RiskLevel::Level2,
                serde_json::json!({}),
            ),
            // under-claims risk-0, but the catalog locks workflow.command.invoke at risk-4 (critical).
            step(
                "s2",
                "workflow.command.invoke",
                RiskLevel::Level0,
                serde_json::json!({}),
            ),
        ],
        ApprovalMode::ApproveAll,
    );
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit plan");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let plan_level: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        plan_level, 1,
        "ONE plan-level approve-all over the genuinely non-critical step"
    );
    let per_step: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NOT NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        per_step, 1,
        "the catalog-risk-4 step gets its OWN per-step approval DESPITE the risk-0 claim"
    );
}

// ---- L3 RED #11 — an uncatalogued plan step rejects the WHOLE plan fail-closed -----------------

#[test]
fn test_unknown_type_plan_step_fail_closed() {
    // spec(§15 fail-closed) — an uncatalogued action_type in ANY plan step → reject the WHOLE plan
    // (PolicyDenied), nothing persists. The §11.5 risk reconciliation NEVER falls back to the
    // untrusted claimed risk (the same claim-0-into-approve-all bypass #10 guards, via the
    // unknown-type door). Consistent with the single-action unknown→deny + whole-plan atomicity.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.create_worktree",
                RiskLevel::Level2,
                serde_json::json!({}),
            ),
            // uncatalogued, claiming risk-0 to try to slip through approve-all.
            step(
                "s2",
                "git.force_push",
                RiskLevel::Level0,
                serde_json::json!({}),
            ),
        ],
        ApprovalMode::ApproveAll,
    );
    let err = gw
        .submit_action_plan(&mut store, p)
        .expect_err("an uncatalogued plan step rejects the whole plan");
    assert!(
        matches!(err, GatewayError::PolicyDenied),
        "uncatalogued plan step → PolicyDenied, got {err:?}"
    );
    assert_eq!(count(&path, "action_plans"), 0, "no plan row persisted");
    assert_eq!(
        count(&path, "action_requests"),
        0,
        "no step rows persisted (whole-plan rollback)"
    );
    assert_eq!(count(&path, "approvals"), 0, "no approvals persisted");
}

// ---- L2 RED #6 — workflow.command.invoke (null schema) is approval-floored, never allow ---------

#[test]
fn test_workflow_command_invoke_null_schema_approval_floored() {
    // spec(§6.3 / OQ-WP-5) — workflow.command.invoke carries params_schema_present=false (the
    // null-schema floor): arbitrary pack-command execution can NEVER resolve to `allow` / be
    // standing-granted. The MVP catalog locks it at risk-4 → require_step_approval; the floor guard in
    // decide is the defense-in-depth that holds even if a future catalog edit lowered its risk.
    let policy = CatalogPolicy;
    let d = policy.decide(&sample_request(
        "workflow.command.invoke",
        RiskLevel::Level0,
    ));
    assert_ne!(
        d.status,
        PolicyDecisionStatus::Allow,
        "null-schema invoke → NEVER allow"
    );
    assert_eq!(
        d.status,
        PolicyDecisionStatus::RequireStepApproval,
        "critical/4 → require_step_approval"
    );
}

// ---- L2 RED #7 — the recorded risk is reconciled to the catalog risk at submit (Q5) ------------

#[test]
fn test_recorded_risk_reconciled_to_catalog() {
    // spec(audit-integrity / Q5) — at submit, the persisted action_requests.risk_level AND the
    // ActionRequested event payload carry the AUTHORITATIVE catalog risk, not the proposer's claim.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    // git.create_worktree is catalog-risk-2 (→ require_approval, so submit completes to
    // awaiting_approval); the proposer under-claims risk-0.
    let req = sample_request("git.create_worktree", RiskLevel::Level0);
    gw.submit_action(&mut store, req).expect("submit");

    // the persisted row carries the authoritative catalog risk (2), not the claimed 0.
    assert_eq!(
        persisted_risk(&path),
        2,
        "action_requests.risk_level reconciled to the catalog risk-2"
    );

    // the ActionRequested event payload likewise carries the authoritative risk (audit trail).
    let events = store.read_all().unwrap();
    let requested = events
        .iter()
        .find(|e| e.event_type == "ActionRequested")
        .expect("ActionRequested");
    let payload: serde_json::Value = serde_json::from_str(&requested.payload_json).unwrap();
    assert_eq!(
        payload["risk_level"], 2,
        "ActionRequested carries the catalog risk-2, not the claimed 0"
    );
}

// =================================================================================================
// P4.0b-ui1 (brief 052) — the per-hunk git.* catalog freeze + the non-standing-grant safety floor.
// The 3 git.* hunk action types + the `standing_grant_eligible` catalog field generalizing the
// risk-4 approve-all exclusion (USER-ruled: git.discard_hunk is destructive/irreversible →
// per-action approval ALWAYS, even under an approve-all). Executor bodies = stubs (Phase 5).
// =================================================================================================

// ---- 052 #1 — the 3 git.* hunk catalog entries (risk / executor / refs / preview) ---------------

#[test]
fn test_git_hunk_catalog_risks() {
    // spec(§6.3) — git.stage_hunk/unstage_hunk = risk-2 (the git.* tier); git.discard_hunk = risk-3
    // (destructive); ALL executor_kind=git, requires_resource_refs=yes (the file/hunk is the target),
    // and discard's preview_class=diff (show the content lost). The 3 types resolve in the catalog.
    use nexusops_shared::catalog::{lookup, ExecutorKind, PreviewClass};
    for t in ["git.stage_hunk", "git.unstage_hunk"] {
        let e = lookup(t).unwrap_or_else(|| panic!("{t} must be catalogued"));
        assert_eq!(
            e.locked_risk,
            RiskLevel::Level2,
            "{t} = risk-2 (git.* tier)"
        );
        assert_eq!(e.executor, ExecutorKind::Git, "{t} executor_kind=git");
        assert!(e.requires_resource_refs, "{t} requires the file/hunk ref");
    }
    let discard = lookup("git.discard_hunk").expect("git.discard_hunk catalogued");
    assert_eq!(
        discard.locked_risk,
        RiskLevel::Level3,
        "git.discard_hunk = risk-3 (destructive, irreversible content loss)"
    );
    assert_eq!(discard.executor, ExecutorKind::Git);
    assert!(discard.requires_resource_refs);
    assert_eq!(
        discard.preview_class,
        PreviewClass::Diff,
        "discard's preview shows EXACTLY the hunk content lost (USER-ruled)"
    );
}

// ---- 052 #2 — git.discard_hunk is NON-standing-grantable (adversarial; the load-bearing pin) -----

#[test]
fn test_discard_hunk_non_standing_grantable() {
    // spec(§6.2 / §11.5) — the USER-ruled safety floor: a destructive risk-3 action that is
    // standing_grant_eligible=false is EXCLUDED from a plan-level approve-all (it gets its OWN per-step
    // approval, ALWAYS), generalizing the risk-4 critical-exclusion (LESSON 19). A standing-grant-
    // eligible git.* step (stage_hunk) IS covered by the plan-level approve-all. Mirrors
    // test_approve_all_excludes_catalog_critical, but the exclusion keys off the new field, not risk-4.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.stage_hunk",
                RiskLevel::Level2,
                serde_json::json!({}),
            ),
            step(
                "s2",
                "git.discard_hunk",
                RiskLevel::Level3,
                serde_json::json!({}),
            ),
        ],
        ApprovalMode::ApproveAll,
    );
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit plan");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let plan_level: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        plan_level, 1,
        "ONE plan-level approve-all over the standing-grant-eligible step (stage_hunk)"
    );
    let per_step: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NOT NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        per_step, 1,
        "git.discard_hunk gets its OWN per-step approval — NEVER folded into approve-all (USER-ruled)"
    );
}

// ---- 052 #3 — the standing_grant_eligible catalog field (the floor's input) ----------------------

#[test]
fn test_standing_grant_eligible_field() {
    // spec(§6.2/§6.3) — the catalog carries `standing_grant_eligible`: false for the destructive
    // git.discard_hunk AND the risk-4 floor type workflow.command.invoke (reconciled to ONE mechanism,
    // Step-2.5 #1 unified-field); true for the normal git.* tier + ordinary types. The policy
    // approve-all floor reads this field (refuse standing-grant for risk-4 OR !standing_grant_eligible).
    use nexusops_shared::catalog::lookup;
    assert!(
        !lookup("git.discard_hunk").unwrap().standing_grant_eligible,
        "git.discard_hunk is NOT standing-grant-eligible (destructive)"
    );
    assert!(
        !lookup("workflow.command.invoke")
            .unwrap()
            .standing_grant_eligible,
        "workflow.command.invoke reconciles to the same field (one mechanism, not two)"
    );
    for t in ["git.stage_hunk", "git.unstage_hunk", "git.status"] {
        assert!(
            lookup(t).unwrap().standing_grant_eligible,
            "{t} IS standing-grant-eligible (the normal tier)"
        );
    }
}

// ---- D9 (P4.7) — github.merge_pr is NEVER in an approve-all (F1; the discard_hunk precedent) ------

#[test]
fn test_merge_pr_never_in_approve_all() {
    // spec(§6.2 / F1 / LESSON 32) — a plan-level approve-all can NEVER cover github.merge_pr: it is
    // standing_grant_eligible=false → EXCLUDED from the bulk approval, getting its OWN per-step approval
    // ALWAYS (every merge a fresh per-action human approval, USER-steered F1). A standing-grant-eligible
    // step (git.stage_hunk) IS covered by the plan-level approve-all. Mirrors
    // test_discard_hunk_non_standing_grantable — the exclusion keys off the catalog field, not risk-4.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.stage_hunk",
                RiskLevel::Level2,
                serde_json::json!({}),
            ),
            step(
                "s2",
                "github.merge_pr",
                RiskLevel::Level3,
                serde_json::json!({}),
            ),
        ],
        ApprovalMode::ApproveAll,
    );
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit plan");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let plan_level: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        plan_level, 1,
        "ONE plan-level approve-all over the standing-grant-eligible step (stage_hunk)"
    );
    let per_step: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NOT NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        per_step, 1,
        "github.merge_pr gets its OWN per-step approval — NEVER folded into approve-all (F1)"
    );
}

#[test]
fn test_submit_review_never_in_approve_all() {
    // spec(§6.2 / F1 / LESSON 32) — a plan approve-all can NEVER cover github.submit_review (non-standing-
    // grantable): it gets its OWN per-step approval ALWAYS. Pinned alongside github.merge_pr (also excluded)
    // + a standing-grant-eligible step (git.stage_hunk) that IS covered — so a 3-step plan yields ONE
    // plan-level approval (over stage_hunk) + TWO per-step approvals (merge_pr + submit_review).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let p = plan(
        vec![
            step(
                "s1",
                "git.stage_hunk",
                RiskLevel::Level2,
                serde_json::json!({}),
            ),
            step(
                "s2",
                "github.merge_pr",
                RiskLevel::Level3,
                serde_json::json!({}),
            ),
            step(
                "s3",
                "github.submit_review",
                RiskLevel::Level3,
                serde_json::json!({}),
            ),
        ],
        ApprovalMode::ApproveAll,
    );
    let plan_id = p.plan_id.as_str().to_string();
    gw.submit_action_plan(&mut store, p).expect("submit plan");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let plan_level: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        plan_level, 1,
        "ONE plan-level approve-all over the standing-grant-eligible step (stage_hunk)"
    );
    let per_step: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE action_request_id IS NOT NULL AND plan_id = ?1",
            [&plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        per_step, 2,
        "BOTH github.submit_review AND github.merge_pr get their OWN per-step approval (never approve-all, F1)"
    );
}

// ---- 052 #4 — git.discard_hunk preview_class=diff (the destructive-action preview) ---------------

#[test]
fn test_discard_hunk_preview_class_diff() {
    // spec(§6.3) — discard's preview renders the hunk content (the diff) so the human sees EXACTLY what
    // is irreversibly lost before approving (USER-ruled). stage/unstage = git (index ops), Step-2.5 #2.
    use nexusops_shared::catalog::{lookup, PreviewClass};
    assert_eq!(
        lookup("git.discard_hunk").unwrap().preview_class,
        PreviewClass::Diff
    );
}

// ---- 052 ADD — the two non-standing-grant disjuncts can't DRIFT (orch-requested invariant) -------

#[test]
fn test_risk4_implies_non_standing_grantable() {
    // spec(§6.2) — the approve-all exclusion keeps BOTH disjuncts (`risk==Level4 OR
    // !standing_grant_eligible`, defense-in-depth). This invariant pins they can't drift: EVERY
    // catalogued risk-4 (critical) entry MUST also be standing_grant_eligible=false, so a future risk-4
    // addition that forgets the field is still excluded from approve-all (and the unification holds).
    use nexusops_shared::catalog::{lookup, AGENT_MUTATION_ACTION_TYPES, MVP_ACTION_TYPES};
    for t in MVP_ACTION_TYPES
        .iter()
        .chain(AGENT_MUTATION_ACTION_TYPES.iter())
    {
        let e = lookup(t).unwrap_or_else(|| panic!("{t} catalogued"));
        if e.locked_risk == RiskLevel::Level4 {
            assert!(
                !e.standing_grant_eligible,
                "risk-4 (critical) '{t}' MUST be non-standing-grantable (the disjuncts must not drift)"
            );
        }
    }
}

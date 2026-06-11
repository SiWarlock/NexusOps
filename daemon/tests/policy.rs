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

use nexusops_shared::actions::{ActionRequest, PolicyDecisionStatus, RequesterType, RiskLevel};
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

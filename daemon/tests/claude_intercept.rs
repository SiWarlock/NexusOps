//! Brief 043 — the Claude `MutationIntercept`→Gateway interception (INV-SEC-1), tests.
//!
//! L2 (this file's first block): the daemon **hook-receiver ingress** — parsing a `PreToolUse`
//! payload + the **pure `tool_name (+ permission_mode) → agent-mutation action_type`** mapping,
//! **fail-closed** (malformed / unmapped / non-`default` mode → Deny; never an un-adjudicated allow).
//! L3 (the routing → Gateway + the adjudication verdict) + L4 (the coverage-gap crux + deny-rules)
//! land in their own blocks. Per-call interception is sound (the `*`-matcher PreToolUse hook fires on
//! EVERY tool call — claude-code-guide / hooks-guide.md); the coverage-gap compensation (mcp/Task/bg
//! DENY two-layer + the `permissions.deny` baseline) is L4.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::status::ActionRequestStatus as AR;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::fault::{arm, FaultPoint};
use nexusopsd::gateway::executor::{ActionExecutor, ExecError, ExecutionOutcome, StubExecutor};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::harness::claude::intercept::{
    map_to_action_type, parse_payload, route_intercept, verdict_for_status, DenyReason,
    InterceptOutcome,
};
use nexusopsd::harness::MutationVerdict;
use nexusopsd::idgen::UlidGen;

// ---- L2 helpers ---------------------------------------------------------------------------------

/// a well-formed `PreToolUse` hook payload JSON (the daemon-wired hook pipes this on stdin).
fn payload_json(tool_name: &str, permission_mode: &str) -> String {
    serde_json::json!({
        "tool_name": tool_name,
        "tool_input": { "command": "ls -la" },
        "session_id": "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "permission_mode": permission_mode,
    })
    .to_string()
}

// ---- 043 L2 RED #5 — the tool_name → agent-mutation action_type mapping (DirectToolUse) ----------

#[test]
fn test_hook_payload_maps_tool_to_action_type() {
    // spec(§9.1) — the receiver maps each interceptable DIRECT Claude tool (default mode) to its
    // agent-mutation action_type: Bash→agent.bash, the file-write family→agent.file_edit, the
    // read family→agent.file_read. (MCP/Task are the L4 coverage-gap DENYs — NOT mapped here.)
    let cases: &[(&str, &str)] = &[
        ("Bash", "agent.bash"),
        ("Write", "agent.file_edit"),
        ("Edit", "agent.file_edit"),
        ("MultiEdit", "agent.file_edit"),
        ("NotebookEdit", "agent.file_edit"),
        ("Read", "agent.file_read"),
        ("Glob", "agent.file_read"),
        ("Grep", "agent.file_read"),
    ];
    for (tool, expected) in cases {
        let payload = parse_payload(&payload_json(tool, "default")).expect("well-formed payload");
        assert_eq!(
            map_to_action_type(&payload),
            Ok(*expected),
            "{tool} → {expected}"
        );
    }
}

// ---- 043 L2 RED #6 — an unmapped tool fails closed (never an un-adjudicated allow) ---------------

#[test]
fn test_unknown_tool_fail_closed_deny() {
    // spec(§15 #1 / §9.1) — a tool the receiver does NOT recognize as an interceptable direct
    // mutation → Deny (UnmappedTool), never a silent allow. Covers a genuinely-unknown tool AND the
    // not-Direct categories L2 denies conservatively (Task subagent, MCP) — L4 formalizes WHY via the
    // §9.1 coverage matrix; here the invariant is "anything not a covered direct tool is denied".
    for tool in [
        "Task",
        "mcp__codegraph__search",
        "WebFetch",
        "SomeBrandNewTool",
        "",
    ] {
        let payload = parse_payload(&payload_json(tool, "default")).expect("well-formed payload");
        assert_eq!(
            map_to_action_type(&payload),
            Err(DenyReason::UnmappedTool),
            "an unmapped tool `{tool}` must fail closed (Deny), never an un-adjudicated allow"
        );
    }
}

// ---- 043 L2 RED #7 — a non-default permission mode fails closed (O-13 #10) -----------------------

#[test]
fn test_non_default_mode_fail_closed_deny() {
    // spec(§9.1 / O-13 #10) — interception is only guaranteed in `default` permission mode; ANY other
    // mode (acceptEdits/bypassPermissions/plan) → Deny, even for an otherwise-interceptable tool. The
    // mode check precedes the tool map (a non-default mode is a deny regardless of the tool).
    for mode in ["acceptEdits", "bypassPermissions", "plan", "", "Default"] {
        let payload = parse_payload(&payload_json("Bash", mode)).expect("well-formed payload");
        assert_eq!(
            map_to_action_type(&payload),
            Err(DenyReason::NonDefaultMode),
            "permission_mode `{mode}` (≠ default) must fail closed (Deny) — O-13 #10"
        );
    }
}

// ---- 043 L2 RED #8 — a malformed hook payload fails closed -----------------------------------

#[test]
fn test_malformed_payload_deny() {
    // spec(§15 fail-closed) — a hook payload that does not parse (truncated JSON, missing the required
    // fields, wrong types) → Deny (Malformed), never construct an un-adjudicated allow from garbage.
    for bad in [
        "{ not valid json",           // truncated
        "{}",                         // missing every field
        r#"{ "tool_name": "Bash" }"#, // missing session_id / permission_mode
        r#"{ "tool_name": 42, "tool_input": {}, "session_id": "s", "permission_mode": "default" }"#, // wrong type
        "", // empty
    ] {
        assert_eq!(
            parse_payload(bad).err(),
            Some(DenyReason::Malformed),
            "a malformed payload `{bad}` must fail closed (Deny)"
        );
    }
}

// =================================================================================================
// L3 — the MutationIntercept→Gateway routing + the adjudication-only verdict + audit-before-verdict
// (the INV-SEC-1 CORE). An intercepted tool call routes through the EXISTING `submit_action` pipeline
// (Option A — one chokepoint) as an adjudication-only ActionRequest that TERMINATES at the verdict:
// risk-0 read → PolicyDecided (no human) → Allow; a mutating tool → AwaitingApproval → (human) →
// Approved → Allow / Denied → Deny. NO daemon executor ever runs (the agent runs the tool). An Allow
// is gated on the authoritative event committing FIRST (§15 #5): any submit/approve error → Deny.
// Driven via the REAL Gateway `submit_action` (no live Claude).
// =================================================================================================

fn temp_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-12T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// the PRODUCTION Gateway (CatalogPolicy + the side-effect-free StubExecutor).
fn catalog_gw() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(StubExecutor))
}

/// a parsed, well-formed payload for `tool` in default mode (the L2 ingress is already pinned).
fn pp(tool: &str) -> nexusopsd::harness::claude::intercept::HookPayload {
    parse_payload(&payload_json(tool, "default")).expect("well-formed payload")
}

/// the single action_requests row's status (the temp DB has exactly one unless noted).
fn action_status(path: &Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM action_requests", [], |r| r.get(0))
        .ok()
}

fn approval_id_of(path: &Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("approval_id")
}

/// an executor that RECORDS whether `execute` was ever called — pins "no daemon executor runs for an
/// adjudication action" (the INV-SEC-1 crux of #13).
#[derive(Clone, Default)]
struct RecordingExecutor {
    ran: Arc<AtomicBool>,
}
impl RecordingExecutor {
    fn ran(&self) -> bool {
        self.ran.load(Ordering::SeqCst)
    }
}
impl ActionExecutor for RecordingExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        Ok(())
    }
    fn execute(&self, _req: &ActionRequest) -> ExecutionOutcome {
        self.ran.store(true, Ordering::SeqCst);
        ExecutionOutcome::Succeeded {
            changed_resources: vec![],
            detail: "RECORDED — an executor ran (this must NOT happen for an adjudication action)"
                .to_string(),
            side_effect_applied: false,
        }
    }
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: "recording".to_string(),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
}

// ---- 043 L3 RED #9 — a risk-0 (read-only) tool auto-allows: PolicyDecided terminal → Allow --------

#[test]
fn test_risk0_tool_auto_allow() {
    // spec(§9.1/§6.2) — agent.file_read is risk-0 → the policy `allow` path, but (adjudication-only)
    // the action TERMINATES at PolicyDecided (NO queued/executing) and the verdict is Allow — no human,
    // no executor. The audit ActionRequested (committed) gates the Allow (§15 #5).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    let outcome = route_intercept(&gw, &mut store, &pp("Read"));
    assert!(
        matches!(outcome, InterceptOutcome::Resolved(MutationVerdict::Allow)),
        "a risk-0 read auto-allows (no human)"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("policy_decided"),
        "the adjudication action rests at PolicyDecided — NEVER queued/executing/succeeded"
    );
}

// ---- 043 L3 RED #10 — a mutating tool requires approval, then (approved) → Allow -----------------

#[test]
fn test_mutating_tool_requires_approval_then_allow() {
    // spec(§6.2/§5.1) — agent.bash is risk-2 → require_approval → AwaitingApproval (a proj_approval_queue
    // item for the human). When the human approves, the action terminates at Approved (NO queued/
    // executing — adjudication) and verdict_for_status(Approved) → Allow.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    let outcome = route_intercept(&gw, &mut store, &pp("Bash"));
    let action_request_id = match outcome {
        InterceptOutcome::AwaitingApproval { action_request_id } => action_request_id,
        _ => panic!("a mutating tool awaits the human's decision"),
    };
    assert!(
        !action_request_id.is_empty(),
        "the AwaitingApproval outcome carries the action id (the P4 transport polls it)"
    );
    assert_eq!(action_status(&path).as_deref(), Some("awaiting_approval"));
    // the human clears the queue item.
    let appr = approval_id_of(&path);
    gw.approve(&mut store, &appr).expect("approve");
    assert_eq!(
        action_status(&path).as_deref(),
        Some("approved"),
        "an approved adjudication rests at Approved — NEVER queued/executing/succeeded"
    );
    assert!(
        matches!(verdict_for_status(AR::Approved), MutationVerdict::Allow),
        "the approved adjudication → Allow"
    );
}

// ---- 043 L3 RED #11 — a denied approval → Deny verdict -------------------------------------------

#[test]
fn test_deny_returns_deny_verdict() {
    // spec(§6.2) — the human denies the queue item → the action is Denied → verdict_for_status → Deny.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    let _ = route_intercept(&gw, &mut store, &pp("Bash"));
    let appr = approval_id_of(&path);
    gw.deny(&mut store, &appr, "operator declined")
        .expect("deny");
    assert_eq!(action_status(&path).as_deref(), Some("denied"));
    assert!(
        matches!(verdict_for_status(AR::Denied), MutationVerdict::Deny { .. }),
        "a denied adjudication → Deny"
    );
}

// ---- 043 L3 RED #12 — audit-write BEFORE verdict, fail-closed (§15 #5) ---------------------------

#[test]
fn test_audit_write_before_verdict_fail_closed() {
    // spec(§15 #5) — an Allow is gated on the authoritative event being durably committed FIRST. Inject
    // a fault on the ActionRequested append (the risk-0 allow-gate) → the submit txn rolls back → the
    // intercept resolves to Deny, NEVER Allow, and NO action row persists (fail-closed).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    arm(FaultPoint::AuditEventWrite); // the next ActionRequested/ActionApproved append fails
    let outcome = route_intercept(&gw, &mut store, &pp("Read")); // risk-0 — WOULD be Allow
    assert!(
        matches!(
            outcome,
            InterceptOutcome::Resolved(MutationVerdict::Deny { .. })
        ),
        "an audit-write fault on the allow-gate → Deny, never Allow (§15 #5)"
    );
    assert_eq!(
        action_status(&path),
        None,
        "the txn rolled back — no un-audited action persisted (fail-closed)"
    );
}

// ---- 043 L3 RED #13 — NO daemon executor runs for an adjudication action (INV-SEC-1) -------------

#[test]
fn test_adjudication_no_executor_runs() {
    // spec(INV-SEC-1) — an adjudication action terminates at the verdict; the AGENT runs the tool, NOT
    // the daemon. A RecordingExecutor proves `execute` is never called — for the risk-0 auto-allow path
    // AND the human-approved path — and the row never enters queued→executing.
    let rec = RecordingExecutor::default();

    // risk-0 auto-allow.
    {
        let (_d, path) = temp_db();
        let mut store = open(&path);
        let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(rec.clone()));
        route_intercept(&gw, &mut store, &pp("Read"));
        assert_eq!(action_status(&path).as_deref(), Some("policy_decided"));
    }
    assert!(!rec.ran(), "a risk-0 adjudication runs NO executor");

    // human-approved mutating.
    {
        let (_d, path) = temp_db();
        let mut store = open(&path);
        let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(rec.clone()));
        route_intercept(&gw, &mut store, &pp("Bash"));
        let appr = approval_id_of(&path);
        gw.approve(&mut store, &appr).expect("approve");
        assert_eq!(action_status(&path).as_deref(), Some("approved"));
    }
    assert!(!rec.ran(), "an approved adjudication runs NO executor");
}

// ---- 043 L3 RED #14 — verdict_for_status: Allow ONLY for the adjudication-allow terminals --------

#[test]
fn test_no_decision_timeout_denies() {
    // spec(§15 fail-closed) — verdict_for_status is the verdict authority: Allow ONLY for PolicyDecided
    // (risk-0 auto-allow) and Approved (human-approved); EVERY other status — incl. an UNRESOLVED
    // AwaitingApproval (the wait timed out — the live wall-clock wait is P4) — → Deny (fail-closed).
    assert!(matches!(
        verdict_for_status(AR::PolicyDecided),
        MutationVerdict::Allow
    ));
    assert!(matches!(
        verdict_for_status(AR::Approved),
        MutationVerdict::Allow
    ));
    // EXHAUSTIVE over the 13 non-allow ActionRequest(15) statuses (the 2 allow-terminals asserted
    // above) — a future variant that accidentally slips into the Allow arm fails here.
    for s in [
        AR::AwaitingApproval, // the no-decision / timeout case — fail-closed Deny
        AR::Submitted,
        AR::Previewed,
        AR::Denied,
        AR::Expired,
        AR::Queued,
        AR::Executing,
        AR::Succeeded,
        AR::Failed,
        AR::PartiallySucceeded,
        AR::RolledBack,
        AR::RollbackFailed,
        AR::Cancelled,
    ] {
        assert!(
            matches!(verdict_for_status(s), MutationVerdict::Deny { .. }),
            "{s:?} → Deny (Allow is reserved for the adjudication-allow terminals)"
        );
    }
}

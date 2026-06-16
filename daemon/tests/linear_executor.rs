//! P7.1 (edges-024) — the `linear` sync executor: the second edges EXTERNAL-NETWORK mutator, mirroring
//! `GithubExecutor` (edges-023).
//!
//! `LinearExecutor` (`ExecutorKind::Linear`) handles `linear.link_issue` (risk-2, requires_resource_refs)
//! and `linear.create_issue` (risk-2, NO resource_ref) by driving an injected async `LinearWriteClient`
//! (Linear GraphQL) seam from the SYNC executor trait over the edges-023 captured-`Handle`/`block_on`/
//! 30s-timeout 3a mechanism.
//!
//! **Q1 asymmetry (load-bearing):** success → `ActionSucceeded` ONLY — NO Linear domain event (the frozen
//! contract has none, unlike github's `PullRequestSynced`). Terminal non-auth → `LinearSyncFailed` via the
//! landed `ExecutionOutcome::FailedWithEvents`; auth → `Failed("auth_failed")` (auth_expired deferred);
//! transient → `Failed`. The §15 `reason` is a structural class-name, never raw API/GraphQL text.
//!
//! **Test strategy:** the `LinearGraphqlWriteClient` live HTTP round-trip is the non-deterministic edge
//! (fake-covered per CLAUDE.md; the `FakeLinearReadClient`/`OctocrabGithubWriteClient` precedent). Tests
//! call `execute()` directly (plain `#[test]` + a built `Runtime` handle — NOT `#[tokio::test]`; `block_on`
//! inside a runtime context panics, the edges-023 pin); #14 drives the full submit→approve pipeline.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::events::{LinearSyncFailed, Provider};
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::{ActionExecutor, CatalogExecutor, ExecutionOutcome};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrations::classifier::IntegrationOutcomeClass;
use nexusopsd::integrations::executor::LinearExecutor;
use nexusopsd::integrations::linear::classify_linear_write_response;
use nexusopsd::integrations::linear_write::{
    CreateIssueArgs, FakeLinearWriteClient, LinearWriteCall, LinearWriteError, LinkIssueArgs,
};

const FIXED_TS: &str = "2026-06-13T00:00:00Z";

// ---- harness helpers -------------------------------------------------------

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(FIXED_TS)),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("nexusops.db");
    (d, p)
}

/// A `linear.link_issue` ActionRequest: `issue_id`/`target` in `inputs`, the issue identity in a
/// resource_ref (the catalog `requires_resource_refs` precondition). `with_ref=false` omits the ref.
fn link_req(issue_id: &str, target: &str, with_ref: bool) -> ActionRequest {
    base_req(
        "linear.link_issue",
        serde_json::json!({ "issue_id": issue_id, "target": target }),
        with_ref,
        RiskLevel::Level2,
    )
}

/// A `linear.create_issue` ActionRequest: `team_id`/`title`/`description?` in `inputs`; NO resource_ref
/// required (requires_resource_refs=false). `with_ref` lets a test add one anyway (must still proceed).
fn create_req(
    team_id: &str,
    title: &str,
    description: Option<&str>,
    with_ref: bool,
) -> ActionRequest {
    let mut inputs = serde_json::json!({ "team_id": team_id, "title": title });
    if let Some(d) = description {
        inputs["description"] = serde_json::json!(d);
    }
    base_req("linear.create_issue", inputs, with_ref, RiskLevel::Level2)
}

fn base_req(
    action_type: &str,
    inputs: serde_json::Value,
    with_ref: bool,
    risk_level: RiskLevel,
) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: if with_ref {
            vec![ResourceRef {
                resource_type: ResourceType::LinearIssue,
                id: "lin_issue".to_string(),
                uri: None,
            }]
        } else {
            vec![]
        },
        inputs,
        risk_level,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// Build a `LinearExecutor` over a `FakeLinearWriteClient::ok()` + the recorded-calls handle. The
/// `Runtime` is returned so it outlives the captured `Handle` (the executor's `block_on` source).
fn ok_executor() -> (
    tokio::runtime::Runtime,
    LinearExecutor,
    Arc<Mutex<Vec<LinearWriteCall>>>,
) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let fake = FakeLinearWriteClient::ok();
    let calls = fake.calls();
    let exec = LinearExecutor::new(
        Box::new(fake),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
    );
    (rt, exec, calls)
}

fn err_executor(error: LinearWriteError) -> (tokio::runtime::Runtime, LinearExecutor) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = LinearExecutor::new(
        Box::new(FakeLinearWriteClient::err(error)),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
    );
    (rt, exec)
}

fn approval_id_of(path: &std::path::Path) -> String {
    let conn = open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("an approval")
}

fn linear_sync_failed_events(store: &EventStore) -> Vec<LinearSyncFailed> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "LinearSyncFailed")
        .map(|e| serde_json::from_str(&e.payload_json).expect("LinearSyncFailed parses"))
        .collect()
}

fn event_type_count(store: &EventStore, event_type: &str) -> usize {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == event_type)
        .count()
}

// ---- 1/2. dispatch + operational-param plumbing ----------------------------

#[test]
fn test_link_issue_invokes_write_client() {
    // spec(§6.3): linear.link_issue calls LinearWriteClient::link_issue with the inputs' issue_id+target.
    let (_rt, exec, calls) = ok_executor();
    let _ = exec.execute(&link_req("ENG-42", "https://app/session/s1", true));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "exactly one write-client call");
    assert_eq!(
        calls[0],
        LinearWriteCall::LinkIssue(LinkIssueArgs {
            issue_id: "ENG-42".to_string(),
            target: "https://app/session/s1".to_string(),
        })
    );
}

#[test]
fn test_create_issue_invokes_write_client() {
    // spec(§6.3): linear.create_issue calls create_issue with team_id/title/description (the FromInputs arm).
    let (_rt, exec, calls) = ok_executor();
    let _ = exec.execute(&create_req(
        "team_eng",
        "Fix the bug",
        Some("steps to repro"),
        false,
    ));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0],
        LinearWriteCall::CreateIssue(CreateIssueArgs {
            team_id: "team_eng".to_string(),
            title: "Fix the bug".to_string(),
            description: Some("steps to repro".to_string()),
        })
    );
}

// ---- 3/4. success → NO domain event (Q1) -----------------------------------

#[test]
fn test_link_issue_success_no_domain_event() {
    // spec(Q1): success → Succeeded{side_effect_applied:true, emitted_events:[]} — the frozen contract has
    // NO Linear success event; ActionSucceeded is the audit record. NO LinearSyncFailed, NO other event.
    let (_rt, exec, _calls) = ok_executor();
    match exec.execute(&link_req("ENG-42", "https://app/session/s1", true)) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            emitted_events,
            changed_resources,
            ..
        } => {
            assert!(
                side_effect_applied,
                "a linked issue is a durable external change"
            );
            assert!(
                emitted_events.is_empty(),
                "Linear success emits NO domain event (Q1 asymmetry vs PR)"
            );
            // the success carries the issue resource_ref as the audit record of what changed (the
            // GithubExecutor precedent — NOT silently dropped).
            assert_eq!(
                changed_resources.len(),
                1,
                "link_issue reports the issue resource_ref in changed_resources"
            );
            assert_eq!(changed_resources[0].id, "lin_issue");
        }
        _ => panic!("expected Succeeded with no events"),
    }
}

#[test]
fn test_create_issue_success_no_domain_event() {
    // spec(Q1): same — create success → Succeeded{side_effect_applied:true, emitted_events:[]}.
    let (_rt, exec, _calls) = ok_executor();
    match exec.execute(&create_req("team_eng", "Fix the bug", None, false)) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            emitted_events,
            ..
        } => {
            assert!(side_effect_applied);
            assert!(emitted_events.is_empty(), "no Linear success domain event");
        }
        _ => panic!("expected Succeeded with no events"),
    }
}

// ---- 5. terminal non-auth → LinearSyncFailed -------------------------------

#[test]
fn test_link_issue_terminal_non_auth_emits_linear_sync_failed() {
    // spec(§17/§15): a terminal non-auth write error (ClientError/NotFound — incl. a GraphQL errors[]
    // mapped to a terminal class) → the action FAILS AND emits LinearSyncFailed{Linear, reason, failed_at};
    // reason is the EXACT structural class-name, carrying NO raw API/GraphQL text (§15).
    for (class, raw_message, expected_reason) in [
        (
            IntegrationOutcomeClass::ClientError { status: 422 },
            "422: a validation failed for team team_eng secret=lin_api_LEAK",
            "client_error",
        ),
        (
            IntegrationOutcomeClass::NotFound,
            "issue ENG-999 not found at https://api.linear.app/graphql",
            "not_found",
        ),
    ] {
        let (_rt, exec) = err_executor(LinearWriteError {
            class,
            message: raw_message.to_string(),
        });
        let outcome = exec.execute(&link_req("ENG-42", "https://app/session/s1", true));
        assert!(
            matches!(outcome, ExecutionOutcome::FailedWithEvents { .. }),
            "terminal non-auth → FailedWithEvents (failure + a durable sync-failed event)"
        );
        let events = match &outcome {
            ExecutionOutcome::FailedWithEvents { emitted_events, .. } => emitted_events,
            _ => unreachable!(),
        };
        assert_eq!(events.len(), 1);
        let failed: LinearSyncFailed = match &events[0] {
            nexusopsd::gateway::executor::EmittedEvent::Namespaced {
                event_type,
                payload_json,
            } => {
                assert_eq!(*event_type, "LinearSyncFailed");
                serde_json::from_str(payload_json).expect("parses")
            }
            _ => panic!("expected a Namespaced LinearSyncFailed"),
        };
        assert_eq!(failed.provider, Provider::Linear);
        assert_eq!(failed.failed_at, Timestamp::parse(FIXED_TS).unwrap());
        assert_eq!(
            failed.reason, expected_reason,
            "the reason is the exact structural class-name (§15/LESSON 31)"
        );
        assert!(
            !failed.reason.contains("lin_api_LEAK")
                && !failed.reason.contains("api.linear.app")
                && !failed.reason.contains("team_eng")
                && !failed.reason.contains("ENG-999"),
            "the reason carries NO raw API/GraphQL text (§15), got: {}",
            failed.reason
        );
    }
}

// ---- 6. AuthFailed → no sync event (auth_expired deferred) -----------------

#[test]
fn test_linear_auth_failed_no_sync_event() {
    // spec(§17): AuthFailed → Failed("auth_failed"), NO LinearSyncFailed (auth_expired DEFERRED).
    let (_rt, exec) = err_executor(LinearWriteError {
        class: IntegrationOutcomeClass::AuthFailed,
        message: "401 unauthorized".to_string(),
    });
    match exec.execute(&link_req("ENG-42", "https://app/session/s1", true)) {
        ExecutionOutcome::Failed(reason) => assert!(
            reason.contains("auth_failed"),
            "auth → a structural auth_failed reason, got: {reason}"
        ),
        _ => panic!("expected Failed (no event)"),
    }
}

// ---- 7. transient → no sync event (LinearSyncFailed = terminal-non-auth only)

#[test]
fn test_linear_transient_no_sync_event() {
    // spec(§17): a transient class (ServerError/RateLimited/TransportError) → Failed, NO LinearSyncFailed.
    for class in [
        IntegrationOutcomeClass::ServerError,
        IntegrationOutcomeClass::RateLimited { retry_after: None },
        IntegrationOutcomeClass::TransportError,
    ] {
        let (_rt, exec) = err_executor(LinearWriteError {
            class,
            message: "transient".to_string(),
        });
        assert!(
            matches!(
                exec.execute(&link_req("ENG-42", "t", true)),
                ExecutionOutcome::Failed(_)
            ),
            "a transient class → plain Failed, never FailedWithEvents/Succeeded"
        );
    }
}

// ---- 8/9. the catalog precondition asymmetry -------------------------------

#[test]
fn test_link_issue_requires_resource_ref() {
    // spec(§6.3): linear.link_issue requires_resource_refs=true → no ref → Failed, client never called.
    let (_rt, exec, calls) = ok_executor();
    let outcome = exec.execute(&link_req("ENG-42", "t", false)); // NO resource_ref
    assert!(matches!(outcome, ExecutionOutcome::Failed(_)));
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "the precondition fails BEFORE the write client is called"
    );
}

#[test]
fn test_create_issue_no_resource_ref_ok() {
    // spec(§6.3): linear.create_issue requires_resource_refs=FALSE → proceeds with NO resource_ref (do not
    // over-require). The write client is called → Succeeded.
    let (_rt, exec, calls) = ok_executor();
    let outcome = exec.execute(&create_req("team_eng", "Fix the bug", None, false));
    assert!(
        matches!(outcome, ExecutionOutcome::Succeeded { .. }),
        "create_issue does not require a resource_ref"
    );
    assert_eq!(
        calls.lock().unwrap().len(),
        1,
        "the write client IS called for create_issue without a ref"
    );
}

// ---- 10. fail-closed operand guard (LESSON 31/32) --------------------------

#[test]
fn test_linear_missing_inputs_failed_no_call() {
    // spec(LESSON 31/32 analog): a blank/absent required operand → Failed BEFORE the GraphQL call.
    // link_issue: blank issue_id or target; create_issue: blank team_id or title.
    let cases: &[ActionRequest] = &[
        link_req("", "t", true),
        link_req("ENG-42", "", true),
        create_req("", "title", None, false),
        create_req("team_eng", "  ", None, false),
    ];
    for req in cases {
        let (_rt, exec, calls) = ok_executor();
        assert!(
            matches!(exec.execute(req), ExecutionOutcome::Failed(_)),
            "blank required operand → Failed ({})",
            req.action_type
        );
        assert_eq!(
            calls.lock().unwrap().len(),
            0,
            "the write client is NEVER called on a blank operand"
        );
    }
}

// ---- 11. the write-actor bound: a hung call times out ----------------------

#[test]
fn test_linear_timeout_is_failed() {
    // spec(LESSON 32): a never-resolving future → the captured-Handle block_on timeout fires → Failed
    // (structural), bounded — a Linear hang can never wedge the single write-actor.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = LinearExecutor::with_timeout(
        Box::new(FakeLinearWriteClient::hanging()),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
        Duration::from_millis(50),
    );
    match exec.execute(&link_req("ENG-42", "t", true)) {
        ExecutionOutcome::Failed(reason) => assert!(
            reason.to_lowercase().contains("time"),
            "a timeout → a structural timeout reason, got: {reason}"
        ),
        _ => panic!("expected Failed (timeout)"),
    }
}

// ---- 12. delegation: non-linear.* → inner stub -----------------------------

#[test]
fn test_linear_executor_delegates_other_actions_to_stub() {
    // spec(§6.3): an action this executor does not handle delegates to the inner CatalogExecutor stub
    // (no-op success, NO event) — the GitExecutor delegation precedent.
    let (_rt, exec, calls) = ok_executor();
    // a non-Linear-kind catalogued type WITH a resource_ref (plan.link_task requires_resource_refs=true) →
    // the inner stub's precondition passes → it stub-succeeds (the delegation contract, not a precondition
    // failure).
    let mut req = create_req("team_eng", "x", None, true);
    req.action_type = "plan.link_task".to_string();
    match exec.execute(&req) {
        ExecutionOutcome::Succeeded { emitted_events, .. } => {
            assert!(emitted_events.is_empty(), "delegated action emits no event")
        }
        _ => panic!("expected a delegated stub Succeeded"),
    }
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "a delegated action never reaches the write client"
    );
}

// ---- 13. structural pin: the 3a mechanism ----------------------------------

#[test]
fn test_linear_executor_uses_captured_handle_not_current() {
    // spec(3a / LESSON 32, load-bearing): the executor drives the async client via the CAPTURED Handle's
    // block_on, NEVER Handle::current() (panics on the write-actor std::thread) and NEVER spawn_blocking.
    // Structural pin (the edges-023 #14 idiom) so a regression is caught at test time.
    let src = std::fs::read_to_string(format!(
        "{}/src/integrations/executor.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read integrations/executor.rs");
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("block_on"),
        "the 3a mechanism drives via block_on"
    );
    assert!(
        !code.contains("Handle::current"),
        "execute() must use the CAPTURED handle, NEVER Handle::current() (write-actor-thread panic)"
    );
    assert!(
        !code.contains("spawn_blocking"),
        "the captured-Handle block_on is the mechanism, NOT spawn_blocking"
    );
}

// ---- 14. e2e reachability: submit (risk-2) → approve → ActionSucceeded ------

#[test]
fn test_link_issue_e2e_via_submit_action_approve() {
    // spec(§6.3 reachability): submit linear.link_issue (risk-2) → AwaitingApproval (gate holds, NO event)
    // → approve → ActionSucceeded (NO LinearSyncFailed) through the REAL pipeline (the 3a block_on runs on
    // the real execute call site — the integration proof).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut catalog = CatalogExecutor::new();
    catalog.register(
        ExecutorKind::Linear,
        Arc::new(LinearExecutor::new(
            Box::new(FakeLinearWriteClient::ok()),
            rt.handle().clone(),
            Box::new(FixedClock::new(FIXED_TS)),
        )),
    );
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog));

    let ack = gw
        .submit_action(
            &mut store,
            link_req("ENG-42", "https://app/session/s1", true),
        )
        .expect("submit");
    assert_eq!(
        ack.status,
        ActionRequestStatus::AwaitingApproval,
        "risk-2 holds at the approval gate (no auto-execute)"
    );

    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");
    assert_eq!(
        event_type_count(&store, "ActionSucceeded"),
        1,
        "approve → execute → ActionSucceeded"
    );
    assert_eq!(
        linear_sync_failed_events(&store).len(),
        0,
        "a successful link emits NO LinearSyncFailed (and no Linear success event — Q1)"
    );
}

// ---- 15. the pure write-response classifier (live-client core) -------------

#[test]
fn test_classify_linear_write_response() {
    // spec(§17): the pure write-response → §17-class mapping (the LinearGraphqlWriteClient's core; the
    // map_linear_response error-layering reused for a mutation, success = unit not an extracted issue):
    // GraphQL errors[] → the coded class (over HTTP); 2xx + no errors → Ok(()); non-2xx + no errors →
    // classify(status).
    // GraphQL errors[] take precedence (even on HTTP 200): RATELIMITED → RateLimited (retryable).
    assert!(matches!(
        classify_linear_write_response(
            200,
            None,
            None,
            r#"{"errors":[{"message":"slow down","extensions":{"code":"RATELIMITED"}}]}"#
        ),
        Err(IntegrationOutcomeClass::RateLimited { .. })
    ));
    // an auth GraphQL error → AuthFailed.
    assert!(matches!(
        classify_linear_write_response(
            200,
            None,
            None,
            r#"{"errors":[{"message":"no","extensions":{"code":"AUTHENTICATION_ERROR"}}]}"#
        ),
        Err(IntegrationOutcomeClass::AuthFailed)
    ));
    // a clean 2xx mutation response with no errors → success (Ok).
    assert!(classify_linear_write_response(
        200,
        None,
        None,
        r#"{"data":{"issueCreate":{"success":true}}}"#
    )
    .is_ok());
    // non-2xx, no GraphQL errors → classify(status): 403 → AuthFailed, 500 → ServerError, 422 → ClientError.
    assert!(matches!(
        classify_linear_write_response(403, None, None, "{}"),
        Err(IntegrationOutcomeClass::AuthFailed)
    ));
    assert!(matches!(
        classify_linear_write_response(500, None, None, "{}"),
        Err(IntegrationOutcomeClass::ServerError)
    ));
    assert!(matches!(
        classify_linear_write_response(422, None, None, "{}"),
        Err(IntegrationOutcomeClass::ClientError { status: 422 })
    ));
}

// ---- 16/17. the create_issue arm — terminal-non-auth + e2e (matrix completeness) -------------------

#[test]
fn test_create_issue_terminal_non_auth_emits_linear_sync_failed() {
    // spec(§17/§15): create_issue ALSO routes a terminal non-auth error through finish→classify_failure →
    // FailedWithEvents emitting LinearSyncFailed (completes the §17 matrix for the create arm — the link
    // arm is #5; both share the disposition, this pins the create dispatch reaches it).
    let (_rt, exec) = err_executor(LinearWriteError {
        class: IntegrationOutcomeClass::ClientError { status: 422 },
        message: "422 validation failed for team team_eng".to_string(),
    });
    let outcome = exec.execute(&create_req("team_eng", "Fix the bug", None, false));
    let events = match &outcome {
        ExecutionOutcome::FailedWithEvents { emitted_events, .. } => emitted_events,
        _ => panic!("create_issue terminal non-auth → FailedWithEvents"),
    };
    assert_eq!(events.len(), 1);
    let failed: LinearSyncFailed = match &events[0] {
        nexusopsd::gateway::executor::EmittedEvent::Namespaced {
            event_type,
            payload_json,
        } => {
            assert_eq!(*event_type, "LinearSyncFailed");
            serde_json::from_str(payload_json).expect("parses")
        }
        _ => panic!("expected a Namespaced LinearSyncFailed"),
    };
    assert_eq!(failed.provider, Provider::Linear);
    assert_eq!(failed.reason, "client_error");
}

#[test]
fn test_create_issue_e2e_via_submit_action_approve() {
    // spec(§6.3 reachability): submit linear.create_issue (risk-2, requires_resource_refs=FALSE — NO ref)
    // → AwaitingApproval → approve → ActionSucceeded (NO LinearSyncFailed) via the REAL pipeline (the
    // create arm's e2e counterpart to #14; proves the no-ref precondition profile + finish() on the real
    // CatalogPolicy/Gateway path).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut catalog = CatalogExecutor::new();
    catalog.register(
        ExecutorKind::Linear,
        Arc::new(LinearExecutor::new(
            Box::new(FakeLinearWriteClient::ok()),
            rt.handle().clone(),
            Box::new(FixedClock::new(FIXED_TS)),
        )),
    );
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog));
    let ack = gw
        .submit_action(
            &mut store,
            create_req("team_eng", "Fix the bug", None, false),
        )
        .expect("submit");
    assert_eq!(
        ack.status,
        ActionRequestStatus::AwaitingApproval,
        "risk-2 holds the approval gate"
    );
    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");
    assert_eq!(
        event_type_count(&store, "ActionSucceeded"),
        1,
        "create_issue → ActionSucceeded (no domain event, Q1)"
    );
    assert_eq!(
        linear_sync_failed_events(&store).len(),
        0,
        "no LinearSyncFailed on a successful create"
    );
}

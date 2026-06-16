//! P7.1 (edges-023) — the `github` sync executor: the FIRST real edges EXTERNAL-NETWORK mutator.
//!
//! `GithubExecutor` (`ExecutorKind::Github`) handles `github.create_pr` (risk-3) + `github.create_pr_draft`
//! (risk-2) by driving an injected async `GithubWriteClient` (octocrab) seam from the SYNC executor trait
//! over a **captured `tokio::runtime::Handle`** (`handle.block_on`, with a hard timeout — NOT
//! `Handle::current()`, NOT `spawn_blocking`; the as-built `execute()` runs on the write-actor's dedicated
//! `std::thread`, a non-runtime non-worker thread → `block_on` there does NOT panic), and emits
//! `PullRequestSynced` (success) / `GithubSyncFailed` (TERMINAL non-auth failure) via the edges-019
//! `EmittedEvent::Namespaced` bridge through the in-txn §15 gate.
//!
//! **Failure taxonomy (§17 classifier):** terminal non-auth (`ClientError`/`NotFound`) → the action FAILS
//! AND emits `GithubSyncFailed` (the new `ExecutionOutcome::FailedWithEvents`; `reason` = a §15
//! structural class-name, NEVER raw API text); `AuthFailed` → plain `Failed("auth_failed")`, NO event
//! (the `auth_expired` variant is DEFERRED); transient (`ServerError`/`RateLimited`/`TransportError`) →
//! plain `Failed`, NO event (`GithubSyncFailed` is the terminal-non-auth class ONLY).
//!
//! **Test strategy:** the octocrab live HTTP round-trip is the non-deterministic edge (fake-covered per
//! CLAUDE.md — only the test module + `FakeGithubWriteClient` reference the trait; Step 7.5 confirms the
//! real `OctocrabGithubWriteClient` is reachable from `main.rs`). Tests 1-10/13/14 call `execute()`
//! directly; 11/12 drive the full submit→approve Gateway path (the `block_on` runs on the real pipeline
//! call site — the 3a integration proof).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::events::{GithubSyncFailed, Provider, PullRequestSynced};
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::{ActionRequestStatus, PullRequest};
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecutionOutcome,
};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrations::classifier::IntegrationOutcomeClass;
use nexusopsd::integrations::executor::GithubExecutor;
use nexusopsd::integrations::github_write::{
    CreatePrArgs, CreatedPr, FakeGithubWriteClient, GithubWriteError,
};
use nexusopsd::integrations::pull_request::{derive_pull_request_status, PullRequestSignals};

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

/// A canned `CreatedPr` — a just-created PR: default (Open) signals, the given coords.
fn canned_pr(pr_number: u64, branch: &str, base: &str) -> CreatedPr {
    CreatedPr {
        pr_number,
        signals: PullRequestSignals::default(),
        branch: branch.to_string(),
        base: base.to_string(),
    }
}

/// A `github.create_pr` / `github.create_pr_draft` ActionRequest: operational params in `inputs`, the
/// repo identity in a `resource_ref` (the catalog `requires_resource_refs` precondition).
fn pr_req(
    action_type: &str,
    owner: &str,
    repo: &str,
    head: &str,
    base: &str,
    title: &str,
    body: Option<&str>,
) -> ActionRequest {
    let mut inputs = serde_json::json!({
        "owner": owner,
        "repo": repo,
        "head": head,
        "base": base,
        "title": title,
    });
    if let Some(b) = body {
        inputs["body"] = serde_json::json!(b);
    }
    let risk_level = if action_type == "github.create_pr" {
        RiskLevel::Level3
    } else {
        RiskLevel::Level2
    };
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: "repo_x".to_string(),
            uri: None,
        }],
        inputs,
        risk_level,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// a `github.create_pr` req with body, the default coords.
fn create_pr_req() -> ActionRequest {
    pr_req(
        "github.create_pr",
        "acme",
        "widget",
        "feature/x",
        "main",
        "Add widget",
        Some("Closes #42."),
    )
}

/// Build a `GithubExecutor` over a `FakeGithubWriteClient::ok(created)` + the recorded-calls handle.
/// The `Runtime` is returned so it outlives the captured `Handle` (the executor's `block_on` source).
fn ok_executor(
    created: CreatedPr,
) -> (
    tokio::runtime::Runtime,
    GithubExecutor,
    Arc<Mutex<Vec<CreatePrArgs>>>,
) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let fake = FakeGithubWriteClient::ok(created);
    let calls = fake.calls();
    let exec = GithubExecutor::new(
        Box::new(fake),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
    );
    (rt, exec, calls)
}

/// Build a `GithubExecutor` over a `FakeGithubWriteClient::err(error)`.
fn err_executor(error: GithubWriteError) -> (tokio::runtime::Runtime, GithubExecutor) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = GithubExecutor::new(
        Box::new(FakeGithubWriteClient::err(error)),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
    );
    (rt, exec)
}

/// the single approvals row's approval_id (the submit→approve flow, mirrors tests/git_executor.rs).
fn approval_id_of(path: &std::path::Path) -> String {
    let conn = open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("an approval")
}

fn pull_request_synced_events(store: &EventStore) -> Vec<PullRequestSynced> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "PullRequestSynced")
        .map(|e| serde_json::from_str(&e.payload_json).expect("PullRequestSynced parses"))
        .collect()
}

fn github_sync_failed_events(store: &EventStore) -> Vec<GithubSyncFailed> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "GithubSyncFailed")
        .map(|e| serde_json::from_str(&e.payload_json).expect("GithubSyncFailed parses"))
        .collect()
}

fn action_failed_count(store: &EventStore) -> usize {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "ActionFailed")
        .count()
}

/// extract the single emitted Namespaced event (event_type, payload_json) from a Succeeded/FailedWithEvents.
fn single_namespaced(outcome: &ExecutionOutcome) -> (&'static str, String) {
    let events = match outcome {
        ExecutionOutcome::Succeeded { emitted_events, .. } => emitted_events,
        ExecutionOutcome::FailedWithEvents { emitted_events, .. } => emitted_events,
        ExecutionOutcome::Failed(e) => panic!("expected emitted events, got Failed: {e}"),
    };
    assert_eq!(events.len(), 1, "exactly one emitted event");
    match &events[0] {
        EmittedEvent::Namespaced {
            event_type,
            payload_json,
        } => (event_type, payload_json.clone()),
        _ => panic!("expected a Namespaced event"),
    }
}

// ---- 1. dispatch + operational-param plumbing ------------------------------

#[test]
fn test_create_pr_invokes_write_client() {
    // spec(§6.3): github.create_pr calls GithubWriteClient::create_pull_request with the inputs'
    // owner/repo/head/base/title/body, draft=false.
    let (_rt, exec, calls) = ok_executor(canned_pr(7, "feature/x", "main"));
    let _ = exec.execute(&create_pr_req());
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "exactly one create_pull_request call");
    let a = &calls[0];
    assert_eq!(a.owner, "acme");
    assert_eq!(a.repo, "widget");
    assert_eq!(a.head, "feature/x");
    assert_eq!(a.base, "main");
    assert_eq!(a.title, "Add widget");
    assert_eq!(a.body.as_deref(), Some("Closes #42."));
    assert!(!a.draft, "github.create_pr → draft=false");
}

// ---- 2. the draft action type ---------------------------------------------

#[test]
fn test_create_pr_draft_sets_draft_true() {
    // spec(§6.3): github.create_pr_draft → the same create arm with draft=true.
    let (_rt, exec, calls) = ok_executor(canned_pr(8, "feature/y", "main"));
    let _ = exec.execute(&pr_req(
        "github.create_pr_draft",
        "acme",
        "widget",
        "feature/y",
        "main",
        "Draft widget",
        None,
    ));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].draft, "github.create_pr_draft → draft=true");
    assert_eq!(
        calls[0].body, None,
        "no body input → None (not empty string)"
    );
}

// ---- 3. emission: PullRequestSynced ---------------------------------------

#[test]
fn test_create_pr_emits_pull_request_synced() {
    // spec(§7.2/§5.1): success → exactly one PullRequestSynced whose pr_number/branch/base come from the
    // create result + status == derive_pull_request_status(signals) (the §5.1 machine, no fork) +
    // pr_checked_at == the injected Clock's UTC-Z stamp.
    let created = canned_pr(101, "feature/x", "main");
    let expected_status = derive_pull_request_status(&created.signals);
    let (_rt, exec, _calls) = ok_executor(created);
    let outcome = exec.execute(&create_pr_req());
    let (event_type, payload_json) = single_namespaced(&outcome);
    assert_eq!(event_type, "PullRequestSynced");
    let synced: PullRequestSynced = serde_json::from_str(&payload_json).expect("parses");
    assert_eq!(synced.pr_number, 101);
    assert_eq!(synced.branch, "feature/x");
    assert_eq!(synced.base, "main");
    assert_eq!(synced.status, expected_status);
    assert_eq!(synced.status, PullRequest::Open, "a fresh open PR → Open");
    assert_eq!(synced.pr_checked_at, Timestamp::parse(FIXED_TS).unwrap());
}

// ---- 4. side_effect_applied (a real external change) -----------------------

#[test]
fn test_create_pr_side_effect_applied_true() {
    // spec(LESSON 21): a created PR is a durable EXTERNAL change → side_effect_applied: true (so a lost
    // terminal write yields the honest ActionPartiallySucceeded, never a clean rollback).
    let (_rt, exec, _calls) = ok_executor(canned_pr(9, "feature/x", "main"));
    match exec.execute(&create_pr_req()) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            ..
        } => assert!(
            side_effect_applied,
            "a created PR is a durable external change"
        ),
        _ => panic!("expected Succeeded {{ side_effect_applied }}"),
    }
}

// ---- 5. terminal non-auth → GithubSyncFailed -------------------------------

#[test]
fn test_create_pr_terminal_non_auth_emits_github_sync_failed() {
    // spec(§17/§15): a terminal non-auth write error (ClientError/NotFound) → the action FAILS AND emits
    // GithubSyncFailed{provider: Github, reason: <structural class>, failed_at}; the reason carries NO raw
    // API response text (§15 — a structural class-name ONLY).
    for (class, raw_message, expected_reason) in [
        (
            IntegrationOutcomeClass::ClientError { status: 422 },
            "422 Unprocessable Entity: a validation failed for path /repos/acme/widget secret=ghp_LEAK",
            "client_error",
        ),
        (
            IntegrationOutcomeClass::NotFound,
            "404 Not Found: https://api.github.com/repos/acme/widget/pulls",
            "not_found",
        ),
    ] {
        let (_rt, exec) = err_executor(GithubWriteError {
            class,
            message: raw_message.to_string(),
        });
        let outcome = exec.execute(&create_pr_req());
        // the action is a FAILURE that carries the structured event (FailedWithEvents).
        assert!(
            matches!(outcome, ExecutionOutcome::FailedWithEvents { .. }),
            "terminal non-auth → FailedWithEvents (failure + a durable sync-failed event)"
        );
        let (event_type, payload_json) = single_namespaced(&outcome);
        assert_eq!(event_type, "GithubSyncFailed");
        let failed: GithubSyncFailed = serde_json::from_str(&payload_json).expect("parses");
        assert_eq!(failed.provider, Provider::Github);
        assert_eq!(failed.failed_at, Timestamp::parse(FIXED_TS).unwrap());
        // §15 / LESSON 31: the reason is the classifier's STRUCTURAL class-name (a stable spec-carrying
        // value) — pin it EXACTLY (an accidental rename is a contract drift), and confirm it carries NO
        // raw API text.
        assert_eq!(
            failed.reason, expected_reason,
            "the reason is the exact structural class-name"
        );
        assert!(
            !failed.reason.contains("ghp_LEAK")
                && !failed.reason.contains("api.github.com")
                && !failed.reason.contains("Unprocessable")
                && !failed.reason.contains("acme/widget"),
            "the reason must carry NO raw API text (§15), got: {}",
            failed.reason
        );
    }
}

// ---- 6. AuthFailed → no sync event (auth_expired deferred) -----------------

#[test]
fn test_create_pr_auth_failed_no_sync_event() {
    // spec(§17): an AuthFailed write error → plain Failed("auth_failed"), NO GithubSyncFailed (the
    // auth_expired *SyncFailed variant is DEFERRED — its 0.5b gate lifted but needs a §17/INV-SEC re-review).
    let (_rt, exec) = err_executor(GithubWriteError {
        class: IntegrationOutcomeClass::AuthFailed,
        message: "401 Unauthorized".to_string(),
    });
    match exec.execute(&create_pr_req()) {
        ExecutionOutcome::Failed(reason) => assert!(
            reason.contains("auth_failed"),
            "auth → a structural auth_failed reason, got: {reason}"
        ),
        _ => panic!("expected Failed (no event)"),
    }
}

// ---- 7. transient → no sync event (GithubSyncFailed is terminal-only) ------

#[test]
fn test_create_pr_transient_no_sync_event() {
    // spec(§17): a transient write error (ServerError/RateLimited/TransportError) → plain Failed, NO
    // GithubSyncFailed (the event is the TERMINAL non-auth class ONLY — a transient retries/queues).
    for class in [
        IntegrationOutcomeClass::ServerError,
        IntegrationOutcomeClass::RateLimited { retry_after: None },
        IntegrationOutcomeClass::TransportError,
    ] {
        let (_rt, exec) = err_executor(GithubWriteError {
            class,
            message: "transient".to_string(),
        });
        assert!(
            matches!(exec.execute(&create_pr_req()), ExecutionOutcome::Failed(_)),
            "a transient class → plain Failed, never FailedWithEvents/Succeeded"
        );
    }
}

// ---- 8. fail-closed input guard (param-injection analog, LESSON 31) --------

#[test]
fn test_create_pr_missing_inputs_failed_no_call() {
    // spec(LESSON 31 analog): a blank/absent required operand (owner/repo/head/base/title) → Failed BEFORE
    // the network call (octocrab is a typed API → the leading-`-` CLI vector does not apply; the analog is
    // fail-closed non-empty validation of every required operand).
    let blanks: &[(&str, &str, &str, &str, &str)] = &[
        ("", "widget", "feature/x", "main", "T"),
        ("acme", "  ", "feature/x", "main", "T"),
        ("acme", "widget", "", "main", "T"),
        ("acme", "widget", "feature/x", "  ", "T"),
        ("acme", "widget", "feature/x", "main", ""),
    ];
    for (owner, repo, head, base, title) in blanks {
        let (_rt, exec, calls) = ok_executor(canned_pr(1, "feature/x", "main"));
        let outcome = exec.execute(&pr_req(
            "github.create_pr",
            owner,
            repo,
            head,
            base,
            title,
            None,
        ));
        assert!(
            matches!(outcome, ExecutionOutcome::Failed(_)),
            "blank operand ({owner:?},{repo:?},{head:?},{base:?},{title:?}) → Failed"
        );
        assert_eq!(
            calls.lock().unwrap().len(),
            0,
            "the write client is NEVER called when an operand is rejected"
        );
    }
}

// ---- 9. the write-actor bound: a hung call times out -----------------------

#[test]
fn test_create_pr_timeout_is_failed() {
    // spec(write-actor bound): a write-client future that never resolves → the captured-Handle block_on's
    // timeout fires → Failed (structural reason), bounded — an octocrab hang can never wedge the single
    // write-actor indefinitely.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = GithubExecutor::with_timeout(
        Box::new(FakeGithubWriteClient::hanging()),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
        Duration::from_millis(50),
    );
    let outcome = exec.execute(&create_pr_req());
    match outcome {
        ExecutionOutcome::Failed(reason) => assert!(
            reason.to_lowercase().contains("time"),
            "a timeout → a structural timeout reason, got: {reason}"
        ),
        _ => panic!("expected Failed (timeout)"),
    }
}

// ---- 10. delegation: non-github.create_pr* → inner stub --------------------

#[test]
fn test_github_executor_delegates_other_actions_to_stub() {
    // spec(§6.3): an action this executor does not handle delegates to the inner CatalogExecutor stub
    // (no-op success, NO event) — the GitExecutor delegation precedent. linear.link_issue is a
    // non-Github-kind catalogued type that must never be create-arm'd here.
    let (_rt, exec, calls) = ok_executor(canned_pr(1, "feature/x", "main"));
    let mut req = create_pr_req();
    req.action_type = "linear.link_issue".to_string();
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

// ---- 11. e2e reachability: submit (risk-3) → approve → PullRequestSynced ----

#[test]
fn test_create_pr_e2e_via_submit_action_approve() {
    // spec(§6.3 reachability): submit github.create_pr (risk-3) → AwaitingApproval (gate holds, NO event)
    // → approve → execute → PullRequestSynced persisted through the REAL pipeline. The 3a block_on runs on
    // the real pipeline execute call site here (the integration proof the captured Handle works in situ).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut catalog = CatalogExecutor::new();
    catalog.register(
        ExecutorKind::Github,
        Arc::new(GithubExecutor::new(
            Box::new(FakeGithubWriteClient::ok(canned_pr(
                55,
                "feature/x",
                "main",
            ))),
            rt.handle().clone(),
            Box::new(FixedClock::new(FIXED_TS)),
        )),
    );
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog));

    let ack = gw
        .submit_action(&mut store, create_pr_req())
        .expect("submit");
    assert_eq!(
        ack.status,
        ActionRequestStatus::AwaitingApproval,
        "risk-3 holds at the approval gate (no auto-execute)"
    );
    assert_eq!(
        pull_request_synced_events(&store).len(),
        0,
        "no PullRequestSynced before approval"
    );

    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");
    let synced = pull_request_synced_events(&store);
    assert_eq!(synced.len(), 1, "approve → execute → PullRequestSynced");
    assert_eq!(synced[0].pr_number, 55);
}

// ---- 12. FailedWithEvents: ActionFailed + the event, atomic (Q2) -----------

#[test]
fn test_failed_with_events_records_action_failed_and_appends() {
    // spec(Q2/§17): a FailedWithEvents outcome records ActionFailed AND appends the emitted_events in the
    // SAME txn-B (atomic); side_effect_applied()==false (the create failed → no durable change). Driven
    // e2e through the github terminal-non-auth path.
    // side_effect_applied() is false for FailedWithEvents (the fail-closed contract keys off it).
    let probe = ExecutionOutcome::FailedWithEvents {
        detail: "x".to_string(),
        emitted_events: vec![],
    };
    assert!(
        !probe.side_effect_applied(),
        "FailedWithEvents applied no durable change → false (clean rollback on a txn-B fault)"
    );

    let (_d, path) = temp_db();
    let mut store = open(&path);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut catalog = CatalogExecutor::new();
    catalog.register(
        ExecutorKind::Github,
        Arc::new(GithubExecutor::new(
            Box::new(FakeGithubWriteClient::err(GithubWriteError {
                class: IntegrationOutcomeClass::ClientError { status: 422 },
                message: "422 secret=ghp_LEAK".to_string(),
            })),
            rt.handle().clone(),
            Box::new(FixedClock::new(FIXED_TS)),
        )),
    );
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog));
    gw.submit_action(&mut store, create_pr_req())
        .expect("submit");
    let ack = gw
        .approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Failed,
        "a terminal non-auth failure → the action is Failed"
    );
    assert_eq!(
        action_failed_count(&store),
        1,
        "the terminal event is ActionFailed"
    );
    let failed = github_sync_failed_events(&store);
    assert_eq!(
        failed.len(),
        1,
        "the GithubSyncFailed event is appended atomic with ActionFailed"
    );
    assert!(
        !failed[0].reason.contains("ghp_LEAK"),
        "§15: the persisted reason carries no raw API text"
    );
}

// ---- 13. catalog precondition: requires_resource_refs FIRST ----------------

#[test]
fn test_create_pr_requires_resource_ref() {
    // spec(§6.3): the catalog `requires_resource_refs` precondition (the repo IDENTITY) is enforced by the
    // create arm FIRST (the GitExecutor precedent) — no resource_ref → Failed, the write client never called.
    let (_rt, exec, calls) = ok_executor(canned_pr(1, "feature/x", "main"));
    let mut req = pr_req(
        "github.create_pr",
        "acme",
        "widget",
        "feature/x",
        "main",
        "Add widget",
        None,
    );
    req.resource_refs.clear(); // NO resource_ref — the precondition must fail-close
    let outcome = exec.execute(&req);
    assert!(matches!(outcome, ExecutionOutcome::Failed(_)));
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "the precondition fails BEFORE the write client is called"
    );
}

// ---- 14. structural pin: the 3a mechanism (captured Handle, not current) ----

#[test]
fn test_github_executor_uses_captured_handle_not_current() {
    // spec(3a / Q1, lead-flagged load-bearing): the executor drives the async client via the CAPTURED
    // Handle's `block_on`, NEVER `Handle::current()` (panics on the write-actor std::thread) and NEVER
    // `spawn_blocking` (awkward off-runtime). Structural pin (the forbidden-#6 grep idiom) so a regression
    // to `Handle::current()` — a production write-actor panic — is caught at test time.
    let src = std::fs::read_to_string(format!(
        "{}/src/integrations/executor.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read integrations/executor.rs");
    // strip full-line comments so the pin checks ACTUAL CODE, not the doc comments that EXPLAIN the rule
    // (which legitimately name `Handle::current()`/`spawn_blocking` to say why they're avoided).
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("block_on"),
        "the 3a mechanism drives the async client via block_on"
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

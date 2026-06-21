//! D10 (P4.7) — `github.submit_review`: a 🔴 cat-1, risk-3, NON-standing-grantable GitHub WRITE that
//! submits a PR review VERDICT (`approve`/`request_changes`/`comment`) to a remote PR, SHA-pinned to the
//! reviewed head (`commit_id`), emits `ReviewSubmitted`, and folds `proj_review`. The SECOND GitHub write
//! (after D9 `github.merge_pr`); mirrors the merge_pr executor precedent + the review-specific knobs
//! (conditional body rule, the explicit fail-closed review event).
//!
//! **Test strategy (mirrors `github_merge_pr.rs`):** the octocrab live HTTP round-trip is the
//! non-deterministic edge (fake-covered per CLAUDE.md; Step 7.5 confirms the real
//! `OctocrabGithubWriteClient` is reachable via `main.rs`'s `ExecutorKind::Github` registration). These
//! tests call `execute()` directly + drive the txn-B fault through the real submit→approve gateway path.
//!
//! **Cross-file coverage (cat-1 — review the whole D10 diff):** the catalog entry + the `ReviewSubmitted`
//! §2.5-seam snapshot live in `shared/tests/contract.rs`; the F2 requester-deny (the D9
//! `GITHUB_MUTATION_TYPES` gate EXTENDED) + the F1 approve-all exclusion in `daemon/tests/policy.rs`; the
//! projector fold + rebuild in `daemon/tests/projections.rs`; the production-path delta nudge in
//! `daemon/tests/runtime.rs`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::events::ReviewSubmitted;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::{ActionRequestStatus, ReviewState};
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, EventStore, PrefixRedactor};
use nexusopsd::fault::{arm, FaultPoint};
use nexusopsd::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecutionOutcome,
};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrations::classifier::IntegrationOutcomeClass;
use nexusopsd::integrations::executor::GithubExecutor;
use nexusopsd::integrations::github_write::{
    map_review_event, FakeGithubWriteClient, GithubWriteError, SubmitReviewArgs, SubmittedReview,
};
use nexusopsd::integrations::repo_resolve::FakeRepoResolver;
use octocrab::models::pulls::ReviewAction;

const FIXED_TS: &str = "2026-06-20T00:00:00Z";

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

/// a `github.submit_review` ActionRequest: operational params in `inputs`, the repo identity in a
/// `resource_ref` (the catalog `requires_resource_refs` precondition).
fn submit_review_req(
    owner: &str,
    repo: &str,
    pr_number: u64,
    commit_id: &str,
    event: &str,
    body: Option<&str>,
) -> ActionRequest {
    let mut inputs = serde_json::json!({
        "owner": owner,
        "repo": repo,
        "pr_number": pr_number,
        "commit_id": commit_id,
        "event": event,
    });
    if let Some(b) = body {
        inputs["body"] = serde_json::json!(b);
    }
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "github.submit_review".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: "repo_x".to_string(),
            uri: None,
        }],
        inputs,
        risk_level: RiskLevel::Level3,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// the default: request_changes with a body, a pinned reviewed head.
fn default_submit_req() -> ActionRequest {
    submit_review_req(
        "acme",
        "widget",
        55,
        "9fceb02d0ae598e95dc970b74767f19372d61af8",
        "request_changes",
        Some("Please address the inline notes."),
    )
}

/// a canned `SubmittedReview` (the normalized create-review result the executor maps → ReviewSubmitted).
fn canned_review(review_id: u64, state: ReviewState, body: Option<&str>) -> SubmittedReview {
    SubmittedReview {
        review_id,
        reviewer: "octocat".to_string(),
        state,
        submitted_at: Some(Timestamp::parse(FIXED_TS).unwrap()),
        body: body.map(|s| s.to_string()),
        commit_id: Some("9fceb02d0ae598e95dc970b74767f19372d61af8".to_string()),
    }
}

fn submitted_executor(
    canned: SubmittedReview,
) -> (
    tokio::runtime::Runtime,
    GithubExecutor,
    Arc<Mutex<Vec<SubmitReviewArgs>>>,
) {
    submitted_executor_with_resolver(canned, FakeRepoResolver::ok("acme", "widget"))
}

/// P4.7 — like [`submitted_executor`] but with an explicit resolver (resolve / ignore-divergent /
/// fail-closed). owner/repo on the submit call come from the RESOLVED audited repo_id, not inputs.
fn submitted_executor_with_resolver(
    canned: SubmittedReview,
    resolver: FakeRepoResolver,
) -> (
    tokio::runtime::Runtime,
    GithubExecutor,
    Arc<Mutex<Vec<SubmitReviewArgs>>>,
) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let fake = FakeGithubWriteClient::submitted(canned);
    let calls = fake.submit_calls();
    let exec = GithubExecutor::new(
        Box::new(fake),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
        Box::new(resolver),
    );
    (rt, exec, calls)
}

fn submit_err_executor(error: GithubWriteError) -> (tokio::runtime::Runtime, GithubExecutor) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = GithubExecutor::new(
        Box::new(FakeGithubWriteClient::submit_err(error)),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
        Box::new(FakeRepoResolver::ok("acme", "widget")),
    );
    (rt, exec)
}

fn approval_id_of(path: &std::path::Path) -> String {
    let conn = open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("an approval")
}

fn event_types(store: &EventStore) -> Vec<String> {
    store
        .read_all()
        .unwrap()
        .iter()
        .map(|e| e.event_type.clone())
        .collect()
}

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

// ---- 6. success: ReviewSubmitted + the SHA-pinned, explicit-event submit -----

#[test]
fn test_submit_review_success_emits_review_submitted() {
    // spec(§7.1 / LESSON 46 / F1) — a successful submit → exactly one ReviewSubmitted{review_id, pr_number,
    // reviewer, state, body, submitted_at, commit_id}; side_effect_applied=true (a real review was POSTed);
    // and the fake recorded the SHA-pinned, explicit-event call {owner,repo,pr_number,commit_id,event,body}.
    // review_id/reviewer/state/submitted_at/body come from the create-response; pr_number from the inputs.
    let (_rt, exec, calls) = submitted_executor(canned_review(
        9100,
        ReviewState::ChangesRequested,
        Some("Please address the inline notes."),
    ));
    let outcome = exec.execute(&default_submit_req());

    {
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "exactly one submit_review call");
        let a = &calls[0];
        assert_eq!(a.owner, "acme");
        assert_eq!(a.repo, "widget");
        assert_eq!(a.pr_number, 55);
        assert_eq!(
            a.commit_id, "9fceb02d0ae598e95dc970b74767f19372d61af8",
            "the review is SHA-pinned to the reviewed head (audit-integrity/anti-race)"
        );
        assert_eq!(
            a.event,
            ReviewAction::RequestChanges,
            "the explicit approved review verdict drives the call"
        );
        assert_eq!(a.body, "Please address the inline notes.");
    }

    match &outcome {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            ..
        } => assert!(
            *side_effect_applied,
            "a POSTed review is a durable external change"
        ),
        _ => panic!("expected Succeeded {{ side_effect_applied }}"),
    }
    let (event_type, payload_json) = single_namespaced(&outcome);
    assert_eq!(event_type, "ReviewSubmitted");
    let submitted: ReviewSubmitted = serde_json::from_str(&payload_json).expect("parses");
    assert_eq!(
        submitted.review_id, 9100,
        "review_id from the create-response"
    );
    assert_eq!(submitted.pr_number, 55, "pr_number from the inputs");
    assert_eq!(submitted.reviewer, "octocat");
    assert_eq!(submitted.state, ReviewState::ChangesRequested);
    assert_eq!(
        submitted.body.as_deref(),
        Some("Please address the inline notes.")
    );
    assert_eq!(
        submitted.commit_id.as_deref(),
        Some("9fceb02d0ae598e95dc970b74767f19372d61af8")
    );
}

// ---- 7. fail-closed operand guard + the conditional-body rule --------------

#[test]
fn test_submit_review_missing_operand_fails_closed() {
    // spec(LESSON 31 analog + GitHub's body rule) — a blank/absent required operand → Failed BEFORE the
    // network call. pr_number(>0)/commit_id/event always required; **body required-non-empty when event ∈
    // {request_changes, comment}** (GitHub 422s otherwise). P4.7: owner/repo are NO LONGER inputs operands
    // — they're resolved from the audited repo_id (blank inputs owner/repo ignored), so those cases drop.
    let blanks: &[serde_json::Value] = &[
        serde_json::json!({ "owner": "a", "repo": "w", "commit_id": "s", "event": "approve" }), // no pr_number
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 0, "commit_id": "s", "event": "approve" }), // pr_number=0
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "event": "approve" }), // no commit_id
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "commit_id": "  ", "event": "approve" }), // blank commit_id
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "commit_id": "s" }), // no event
        // the conditional-body rule: request_changes / comment REQUIRE a non-empty body.
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "commit_id": "s", "event": "request_changes" }), // no body
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "commit_id": "s", "event": "request_changes", "body": "  " }), // blank body
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "commit_id": "s", "event": "comment" }), // comment, no body
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "commit_id": "s", "event": "comment", "body": "  " }), // comment, blank body
    ];
    for inputs in blanks {
        let (_rt, exec, calls) = submitted_executor(canned_review(1, ReviewState::Approved, None));
        let mut req = default_submit_req();
        req.inputs = inputs.clone();
        assert!(
            matches!(exec.execute(&req), ExecutionOutcome::Failed(_)),
            "a missing/invalid operand ({inputs}) → Failed"
        );
        assert_eq!(
            calls.lock().unwrap().len(),
            0,
            "the submit client is NEVER called when an operand is rejected"
        );
    }
}

#[test]
fn test_submit_review_approve_allows_empty_body() {
    // spec(GitHub's conditional-body rule) — `approve` does NOT require a body: an approve with no body
    // reaches the client (NOT rejected); the recorded call carries an empty body.
    let (_rt, exec, calls) = submitted_executor(canned_review(9101, ReviewState::Approved, None));
    let req = submit_review_req("acme", "widget", 55, "deadbeef", "approve", None);
    let outcome = exec.execute(&req);
    assert!(
        matches!(outcome, ExecutionOutcome::Succeeded { .. }),
        "approve + no body → reaches the client + Succeeds"
    );
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "approve-no-body still calls the client");
    assert_eq!(
        calls[0].body, "",
        "approve with no body → an empty body string"
    );
    assert_eq!(calls[0].event, ReviewAction::Approve);
    drop(calls);

    // approve WITH a non-empty body is also allowed (body is OPTIONAL for approve, not forbidden) — the
    // text is passed through to the client.
    let (_rt2, exec2, calls2) =
        submitted_executor(canned_review(9102, ReviewState::Approved, Some("LGTM")));
    let req2 = submit_review_req("acme", "widget", 55, "deadbeef", "approve", Some("LGTM"));
    assert!(matches!(
        exec2.execute(&req2),
        ExecutionOutcome::Succeeded { .. }
    ));
    assert_eq!(
        calls2.lock().unwrap()[0].body,
        "LGTM",
        "approve with a body → the body text is passed through"
    );
}

#[test]
fn test_submit_review_unknown_event_fails_closed_no_call() {
    // spec(fail-closed mapping) — an unknown review event → Failed BEFORE the network call (NEVER a silent
    // default; the approved+audited verdict must execute exactly).
    let (_rt, exec, calls) = submitted_executor(canned_review(1, ReviewState::Approved, None));
    let req = submit_review_req("acme", "widget", 55, "deadbeef", "lgtm", Some("x"));
    assert!(
        matches!(exec.execute(&req), ExecutionOutcome::Failed(_)),
        "an unknown event → Failed (fail-closed, no silent default)"
    );
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "the submit client is NEVER called for an unmappable event"
    );
}

#[test]
fn test_submit_review_requires_resource_ref() {
    // spec(§6.3 / cat-1 fail-closed) — execute_submit_review validates the catalog `requires_resource_refs`
    // precondition (the Repo IDENTITY) FIRST: NO/empty resource_refs → Failed, the client NEVER called.
    let (_rt, exec, calls) = submitted_executor(canned_review(1, ReviewState::Approved, None));
    let mut req = default_submit_req();
    req.resource_refs.clear();
    assert!(
        matches!(exec.execute(&req), ExecutionOutcome::Failed(_)),
        "no resource_ref → Failed (precondition fails BEFORE the network call)"
    );
    assert_eq!(calls.lock().unwrap().len(), 0, "the client is never called");
}

// ---- 8. the write-actor bound: a hung submit times out ---------------------

#[test]
fn test_submit_review_timeout_is_structural_failure() {
    // spec(LESSON 46 liveness) — a submit future that never resolves → the captured-Handle block_on's
    // timeout fires → Failed with the §15 STRUCTURAL reason (no raw API text), bounded.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = GithubExecutor::with_timeout(
        Box::new(FakeGithubWriteClient::submit_hanging()),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
        Box::new(FakeRepoResolver::ok("acme", "widget")),
        Duration::from_millis(50),
    );
    match exec.execute(&default_submit_req()) {
        ExecutionOutcome::Failed(reason) => assert_eq!(
            reason, "github.submit_review timed out (structural)",
            "the timeout reason is the §15 structural class-name (no raw API text)"
        ),
        _ => panic!("expected Failed (timeout)"),
    }
}

// ---- 9. §17 failure classes (shared classify_sync_failure) -----------------

#[test]
fn test_submit_review_failure_classes() {
    // spec(§17 / §15 / LESSON 32) — the submit reuses the SHARED classify_sync_failure: terminal non-auth
    // (ClientError/NotFound) → FailedWithEvents emitting GithubSyncFailed (a §15 STRUCTURAL reason, NEVER
    // raw API text); AuthFailed → plain Failed; transient → plain Failed.
    let (_rt, exec) = submit_err_executor(GithubWriteError {
        class: IntegrationOutcomeClass::ClientError { status: 422 },
        message: "422 Unprocessable: body required secret=ghp_LEAK".to_string(),
    });
    let outcome = exec.execute(&default_submit_req());
    assert!(
        matches!(outcome, ExecutionOutcome::FailedWithEvents { .. }),
        "terminal non-auth → FailedWithEvents (failure + a durable sync-failed event)"
    );
    let (event_type, payload_json) = single_namespaced(&outcome);
    assert_eq!(event_type, "GithubSyncFailed");
    assert!(
        !payload_json.contains("ghp_LEAK") && !payload_json.contains("Unprocessable"),
        "§15: the persisted GithubSyncFailed carries a structural reason, no raw API text: {payload_json}"
    );

    let (_rt, exec) = submit_err_executor(GithubWriteError {
        class: IntegrationOutcomeClass::AuthFailed,
        message: "401 Unauthorized".to_string(),
    });
    assert!(
        matches!(
            exec.execute(&default_submit_req()),
            ExecutionOutcome::Failed(_)
        ),
        "AuthFailed → plain Failed (no event)"
    );

    for class in [
        IntegrationOutcomeClass::ServerError,
        IntegrationOutcomeClass::RateLimited { retry_after: None },
        IntegrationOutcomeClass::TransportError,
    ] {
        let (_rt, exec) = submit_err_executor(GithubWriteError {
            class,
            message: "transient".to_string(),
        });
        assert!(
            matches!(
                exec.execute(&default_submit_req()),
                ExecutionOutcome::Failed(_)
            ),
            "a transient class → plain Failed, never FailedWithEvents"
        );
    }
}

// ---- 10. side-effect-applied + txn-B fault → partially_succeeded (LESSON 21) -

#[test]
fn test_submit_review_txnb_fault_is_partially_succeeded() {
    // spec(§17 / LESSON 21) — a real review was POSTed (side_effect_applied=true) but the terminal
    // ActionSucceeded write faults (the §14 TerminalEventWrite checkpoint). The GitHub review can't be
    // un-posted, so the gateway records ActionPartiallySucceeded (the loud audit-integrity record), NOT a
    // clean rollback — settled partially_succeeded, never claimed succeeded; ReviewSubmitted rolls back.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut catalog = CatalogExecutor::new();
    catalog.register(
        ExecutorKind::Github,
        Arc::new(GithubExecutor::new(
            Box::new(FakeGithubWriteClient::submitted(canned_review(
                9100,
                ReviewState::ChangesRequested,
                Some("x"),
            ))),
            rt.handle().clone(),
            Box::new(FixedClock::new(FIXED_TS)),
            Box::new(FakeRepoResolver::ok("acme", "widget")),
        )),
    );
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog));
    gw.submit_action(&mut store, default_submit_req())
        .expect("submit");

    arm(FaultPoint::TerminalEventWrite);
    let ack = gw
        .approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute; a partial success is a settled terminal outcome, acked");
    assert_eq!(
        ack.status,
        ActionRequestStatus::PartiallySucceeded,
        "side-effect-applied + unwritable terminal → partially_succeeded"
    );
    let types = event_types(&store);
    assert!(
        types.contains(&"ActionPartiallySucceeded".to_string()),
        "the ActionPartiallySucceeded record is emitted, got {types:?}"
    );
    assert!(
        !types.contains(&"ActionSucceeded".to_string()),
        "NO ActionSucceeded — the terminal success write failed"
    );
    assert!(
        !types.contains(&"ReviewSubmitted".to_string()),
        "ReviewSubmitted was in the rolled-back txn-B → not persisted on a partial, got {types:?}"
    );
}

// ---- 11. map_review_event — fail-closed mapping ----------------------------

#[test]
fn test_map_review_event() {
    // spec(fail-closed mapping) — approve/request_changes/comment map 1:1 to octocrab's ReviewAction
    // (case-insensitive + trimmed, the map_merge_method precedent); any unknown value → a fail-closed Err
    // (NEVER a silent default — the approved+audited verdict must execute exactly).
    assert_eq!(map_review_event("approve"), Ok(ReviewAction::Approve));
    assert_eq!(
        map_review_event("request_changes"),
        Ok(ReviewAction::RequestChanges)
    );
    assert_eq!(map_review_event("comment"), Ok(ReviewAction::Comment));
    // case-insensitive + trimmed.
    assert_eq!(map_review_event("APPROVE"), Ok(ReviewAction::Approve));
    assert_eq!(
        map_review_event("  request_changes "),
        Ok(ReviewAction::RequestChanges)
    );
    for unknown in ["lgtm", "", "  ", "reject", "approved", "changes_requested"] {
        assert!(
            map_review_event(unknown).is_err(),
            "an unknown event `{unknown}` must fail closed (Err), never a silent default"
        );
    }
    // pin the WIRE token the submit POST sends (the lower-level body relies on ReviewAction's Serialize):
    // GitHub's reviews endpoint requires exactly APPROVE/REQUEST_CHANGES/COMMENT — an octocrab minor-bump
    // that changed this serialization would otherwise silently 422 the live submit (no compile error).
    assert_eq!(
        serde_json::to_value(map_review_event("approve").unwrap()).unwrap(),
        serde_json::json!("APPROVE")
    );
    assert_eq!(
        serde_json::to_value(map_review_event("request_changes").unwrap()).unwrap(),
        serde_json::json!("REQUEST_CHANGES")
    );
    assert_eq!(
        serde_json::to_value(map_review_event("comment").unwrap()).unwrap(),
        serde_json::json!("COMMENT")
    );
}

// =============================================================================
// P4.7 — confused-deputy closure (submit_review): owner/repo resolved from the
// AUDITED resource_ref repo_id (the D10 mirror of the merge_pr closure).
// =============================================================================

/// a submit_review req whose inputs["owner"/"repo"] DIVERGE from the audited resource_ref repo_id.
fn submit_req_divergent_inputs() -> ActionRequest {
    submit_review_req(
        "attacker",
        "evil",
        55,
        "9fceb02d0ae598e95dc970b74767f19372d61af8",
        "request_changes",
        Some("notes"),
    )
}

#[test]
fn submit_review_resolves_owner_repo_from_repo_id() {
    // spec(P4.7 (b) — D10 mirror): the review targets the RESOLVED owner/repo (from the audited repo_id).
    // DISTINCT resolver values (rowner/rrepo ≠ the inputs' acme/widget) prove the call came from RESOLUTION.
    let (_rt, exec, calls) = submitted_executor_with_resolver(
        canned_review(9100, ReviewState::ChangesRequested, Some("x")),
        FakeRepoResolver::ok("rowner", "rrepo"),
    );
    let outcome = exec.execute(&default_submit_req());
    assert!(matches!(outcome, ExecutionOutcome::Succeeded { .. }));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].owner, "rowner",
        "owner is the RESOLVED value, not inputs"
    );
    assert_eq!(
        calls[0].repo, "rrepo",
        "repo is the RESOLVED value, not inputs"
    );
}

#[test]
fn submit_review_ignores_divergent_inputs_owner_repo() {
    // spec(P4.7 (c) — audited==executed): divergent inputs[owner/repo] IGNORED; resolved audited target wins.
    let (_rt, exec, calls) = submitted_executor_with_resolver(
        canned_review(9100, ReviewState::ChangesRequested, Some("x")),
        FakeRepoResolver::ok("acme", "widget"),
    );
    let _ = exec.execute(&submit_req_divergent_inputs());
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].owner, "acme",
        "divergent inputs[owner]=attacker IGNORED"
    );
    assert_eq!(
        calls[0].repo, "widget",
        "divergent inputs[repo]=evil IGNORED"
    );
}

#[test]
fn submit_review_fails_closed_on_unresolvable_repo_id() {
    // spec(P4.7 (b) fail-closed): an unresolvable repo_id → Failed BEFORE the network call (no submit).
    let (_rt, exec, calls) = submitted_executor_with_resolver(
        canned_review(9100, ReviewState::ChangesRequested, Some("x")),
        FakeRepoResolver::not_found(),
    );
    match exec.execute(&default_submit_req()) {
        ExecutionOutcome::Failed(_) => {}
        _ => panic!("expected Failed on an unresolvable repo_id"),
    }
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "fail-closed BEFORE the network call — no review submitted on an unresolvable target"
    );
}

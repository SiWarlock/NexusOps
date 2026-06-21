//! D9 (P4.7) — `github.merge_pr`: a 🔴 cat-1, risk-3, NON-standing-grantable GitHub WRITE mutation that
//! merges a remote PR (head→base) via octocrab `pulls().merge()`, SHA-pinned to the approved head, emits
//! `PullRequestMerged`, and folds `proj_pull_request` → terminal `Merged`. The FIRST GitHub *write* beyond
//! `create_pr`; mirrors the `github.create_pr` executor precedent + the merge-specific safety knobs
//! (SHA-pin anti-race, explicit merge_method, F1/F2 USER-steer).
//!
//! **Test strategy (mirrors `github_executor.rs`):** the octocrab live HTTP round-trip is the
//! non-deterministic edge (fake-covered per CLAUDE.md — only this test module + `FakeGithubWriteClient`
//! reference the trait; Step 7.5 confirms the real `OctocrabGithubWriteClient` is reachable via
//! `main.rs`'s `ExecutorKind::Github` registration). These tests call `execute()` directly + drive the
//! txn-B fault through the real submit→approve gateway path.
//!
//! **Cross-file coverage (this is a cat-1 slice — review the whole D9 diff):** the catalog entry + the
//! `PullRequestMerged` §2.5-seam snapshot live in `shared/tests/contract.rs`; the F2 requester-deny + the
//! F1 approve-all exclusion in `daemon/tests/policy.rs`; the projector fold + rebuild in
//! `daemon/tests/projections.rs`; the production-path delta nudge in `daemon/tests/runtime.rs`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::events::PullRequestMerged;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
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
    map_merge_method, FakeGithubWriteClient, GithubWriteError, MergePrArgs, MergedPr,
};
use octocrab::params::pulls::MergeMethod;

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

/// a `github.merge_pr` ActionRequest: operational params in `inputs`, the repo identity in a
/// `resource_ref` (the catalog `requires_resource_refs` precondition).
fn merge_pr_req(
    owner: &str,
    repo: &str,
    pr_number: u64,
    sha: &str,
    merge_method: &str,
) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "github.merge_pr".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: "repo_x".to_string(),
            uri: None,
        }],
        inputs: serde_json::json!({
            "owner": owner,
            "repo": repo,
            "pr_number": pr_number,
            "sha": sha,
            "merge_method": merge_method,
        }),
        risk_level: RiskLevel::Level3,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// the default merge req: acme/widget#55, squash, a pinned head SHA.
fn default_merge_req() -> ActionRequest {
    merge_pr_req(
        "acme",
        "widget",
        55,
        "9fceb02d0ae598e95dc970b74767f19372d61af8",
        "squash",
    )
}

/// Build a `GithubExecutor` over a `FakeGithubWriteClient::merged(canned)` + the recorded merge-calls
/// handle. The `Runtime` is returned so it outlives the captured `Handle` (the executor's block_on source).
fn merged_executor(
    canned: MergedPr,
) -> (
    tokio::runtime::Runtime,
    GithubExecutor,
    Arc<Mutex<Vec<MergePrArgs>>>,
) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let fake = FakeGithubWriteClient::merged(canned);
    let calls = fake.merge_calls();
    let exec = GithubExecutor::new(
        Box::new(fake),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
    );
    (rt, exec, calls)
}

/// Build a `GithubExecutor` over a `FakeGithubWriteClient::merge_err(error)`.
fn merge_err_executor(error: GithubWriteError) -> (tokio::runtime::Runtime, GithubExecutor) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = GithubExecutor::new(
        Box::new(FakeGithubWriteClient::merge_err(error)),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
    );
    (rt, exec)
}

/// the single approvals row's approval_id (the submit→approve flow; the github_executor precedent).
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

// ---- 6. success: PullRequestMerged + the SHA-pinned merge call --------------

#[test]
fn test_merge_pr_success_emits_pull_request_merged() {
    // spec(§7.1 / LESSON 46 / F1) — a successful merge → exactly one PullRequestMerged{pr_number,
    // merge_commit_sha, merged_at}; side_effect_applied=true (a real merge happened); and the fake
    // recorded the SHA-pinned, explicit-method call {owner,repo,pr_number,sha,method}. The pr_number is
    // from the inputs; merge_commit_sha from the merge result; merged_at from the injected Clock.
    let (_rt, exec, calls) = merged_executor(MergedPr {
        merge_commit_sha: Some("abc123def".to_string()),
    });
    let outcome = exec.execute(&default_merge_req());

    // the executor drove the SHA-pinned merge with the mapped method (audit-integrity F1/F2).
    {
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "exactly one merge_pull_request call");
        let a = &calls[0];
        assert_eq!(a.owner, "acme");
        assert_eq!(a.repo, "widget");
        assert_eq!(a.pr_number, 55);
        assert_eq!(
            a.sha, "9fceb02d0ae598e95dc970b74767f19372d61af8",
            "the merge is SHA-pinned to the approved head (anti-race)"
        );
        assert_eq!(
            a.merge_method,
            MergeMethod::Squash,
            "the explicit approved merge_method drives the call (audit-integrity)"
        );
    }

    // side_effect_applied=true → a lost terminal write yields ActionPartiallySucceeded, not a clean rollback.
    match &outcome {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            ..
        } => assert!(
            *side_effect_applied,
            "a merged PR is a durable external change"
        ),
        _ => panic!("expected Succeeded {{ side_effect_applied }}"),
    }
    let (event_type, payload_json) = single_namespaced(&outcome);
    assert_eq!(event_type, "PullRequestMerged");
    let merged: PullRequestMerged = serde_json::from_str(&payload_json).expect("parses");
    assert_eq!(merged.pr_number, 55, "pr_number from the inputs");
    assert_eq!(
        merged.merge_commit_sha.as_deref(),
        Some("abc123def"),
        "merge_commit_sha from the merge result"
    );
    assert_eq!(
        merged.merged_at,
        Timestamp::parse(FIXED_TS).unwrap(),
        "merged_at from the injected Clock (UTC-Z)"
    );
}

// ---- 7. fail-closed operand guard (param-injection analog, LESSON 31) ------

#[test]
fn test_merge_pr_missing_operand_fails_closed() {
    // spec(LESSON 31 analog) — a blank/absent required operand (owner/repo/pr_number/sha/merge_method) →
    // Failed BEFORE the network call (octocrab is a typed API; the analog is fail-closed non-empty/typed
    // validation of EVERY required operand). pr_number=0 is rejected (a GitHub PR number is >= 1).
    let blanks: &[serde_json::Value] = &[
        serde_json::json!({ "repo": "w", "pr_number": 55, "sha": "s", "merge_method": "merge" }), // no owner
        serde_json::json!({ "owner": "  ", "repo": "w", "pr_number": 55, "sha": "s", "merge_method": "merge" }), // blank owner
        serde_json::json!({ "owner": "a", "pr_number": 55, "sha": "s", "merge_method": "merge" }), // no repo
        serde_json::json!({ "owner": "a", "repo": "  ", "pr_number": 55, "sha": "s", "merge_method": "merge" }), // blank repo
        serde_json::json!({ "owner": "a", "repo": "w", "sha": "s", "merge_method": "merge" }), // no pr_number
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 0, "sha": "s", "merge_method": "merge" }), // pr_number=0
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "merge_method": "merge" }), // no sha
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "sha": "  ", "merge_method": "merge" }), // blank sha
        serde_json::json!({ "owner": "a", "repo": "w", "pr_number": 55, "sha": "s" }), // no merge_method
    ];
    for inputs in blanks {
        let (_rt, exec, calls) = merged_executor(MergedPr {
            merge_commit_sha: None,
        });
        let mut req = default_merge_req();
        req.inputs = inputs.clone();
        let outcome = exec.execute(&req);
        assert!(
            matches!(outcome, ExecutionOutcome::Failed(_)),
            "a missing/invalid operand ({inputs}) → Failed"
        );
        assert_eq!(
            calls.lock().unwrap().len(),
            0,
            "the merge client is NEVER called when an operand is rejected"
        );
    }
}

#[test]
fn test_merge_pr_unknown_method_fails_closed_no_call() {
    // spec(F2 fail-closed mapping) — an unknown merge_method → Failed BEFORE the network call (NEVER a
    // silent server-side default; the approved+audited Action must name the exact method that executes).
    let (_rt, exec, calls) = merged_executor(MergedPr {
        merge_commit_sha: None,
    });
    let req = merge_pr_req("acme", "widget", 55, "deadbeef", "fast_forward");
    assert!(
        matches!(exec.execute(&req), ExecutionOutcome::Failed(_)),
        "an unknown merge_method → Failed (fail-closed, no silent default)"
    );
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "the merge client is NEVER called for an unmappable method"
    );
}

#[test]
fn test_merge_pr_requires_resource_ref() {
    // spec(§6.3 / cat-1 fail-closed) — execute_merge_pr validates the catalog `requires_resource_refs`
    // precondition (the Repo IDENTITY) FIRST: NO/empty resource_refs → Failed, the merge client NEVER
    // called. Pinned DIRECTLY for this cat-1 action (not just transitively via the shared inner.validate)
    // so the audit trail is explicit — a merge with no auditable repo identity must never reach GitHub.
    let (_rt, exec, calls) = merged_executor(MergedPr {
        merge_commit_sha: None,
    });
    let mut req = default_merge_req();
    req.resource_refs.clear(); // NO Repo resource_ref — the precondition must fail-close
    assert!(
        matches!(exec.execute(&req), ExecutionOutcome::Failed(_)),
        "no resource_ref → Failed (the precondition fails BEFORE the network call)"
    );
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "the merge client is NEVER called when the resource_ref precondition fails"
    );
}

// ---- 8. the write-actor bound: a hung merge times out ----------------------

#[test]
fn test_merge_pr_timeout_is_structural_failure() {
    // spec(LESSON 46 liveness) — a merge future that never resolves → the captured-Handle block_on's
    // timeout fires → Failed (structural reason), bounded: an octocrab hang can never wedge the single
    // write-actor indefinitely.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let exec = GithubExecutor::with_timeout(
        Box::new(FakeGithubWriteClient::merge_hanging()),
        rt.handle().clone(),
        Box::new(FixedClock::new(FIXED_TS)),
        Duration::from_millis(50),
    );
    match exec.execute(&default_merge_req()) {
        ExecutionOutcome::Failed(reason) => assert_eq!(
            reason, "github.merge_pr timed out (structural)",
            "the timeout reason is the §15 STRUCTURAL class-name (no raw API text), not an arbitrary string"
        ),
        _ => panic!("expected Failed (timeout)"),
    }
}

// ---- 9. §17 failure classes (shared classify_sync_failure) -----------------

#[test]
fn test_merge_pr_failure_classes() {
    // spec(§17 / §15 / LESSON 32) — the merge reuses the SHARED classify_sync_failure: terminal non-auth
    // (ClientError/NotFound) → FailedWithEvents emitting GithubSyncFailed (a §15 STRUCTURAL reason, NEVER
    // raw API text); AuthFailed → plain Failed (the auth_expired variant is deferred); transient → plain
    // Failed (GithubSyncFailed is the terminal-non-auth class ONLY).
    // terminal non-auth → FailedWithEvents[GithubSyncFailed]; the reason carries no raw API text.
    let (_rt, exec) = merge_err_executor(GithubWriteError {
        class: IntegrationOutcomeClass::ClientError { status: 405 },
        message: "405 Method Not Allowed: head sha mismatch secret=ghp_LEAK".to_string(),
    });
    let outcome = exec.execute(&default_merge_req());
    assert!(
        matches!(outcome, ExecutionOutcome::FailedWithEvents { .. }),
        "terminal non-auth → FailedWithEvents (failure + a durable sync-failed event)"
    );
    let (event_type, payload_json) = single_namespaced(&outcome);
    assert_eq!(event_type, "GithubSyncFailed");
    assert!(
        !payload_json.contains("ghp_LEAK") && !payload_json.contains("Method Not Allowed"),
        "§15: the persisted GithubSyncFailed carries a structural reason, no raw API text: {payload_json}"
    );

    // AuthFailed → plain Failed, NO event.
    let (_rt, exec) = merge_err_executor(GithubWriteError {
        class: IntegrationOutcomeClass::AuthFailed,
        message: "401 Unauthorized".to_string(),
    });
    assert!(
        matches!(
            exec.execute(&default_merge_req()),
            ExecutionOutcome::Failed(_)
        ),
        "AuthFailed → plain Failed (no event; auth_expired deferred)"
    );

    // transient → plain Failed, NO event.
    for class in [
        IntegrationOutcomeClass::ServerError,
        IntegrationOutcomeClass::RateLimited { retry_after: None },
        IntegrationOutcomeClass::TransportError,
    ] {
        let (_rt, exec) = merge_err_executor(GithubWriteError {
            class,
            message: "transient".to_string(),
        });
        assert!(
            matches!(
                exec.execute(&default_merge_req()),
                ExecutionOutcome::Failed(_)
            ),
            "a transient class → plain Failed, never FailedWithEvents"
        );
    }
}

// ---- 10. side-effect-applied + txn-B fault → partially_succeeded (LESSON 21) -

#[test]
fn test_merge_pr_txnb_fault_is_partially_succeeded() {
    // spec(§17 / LESSON 21) — a real merge happened (side_effect_applied=true) but the terminal
    // ActionSucceeded write faults (the §14 TerminalEventWrite checkpoint). The GitHub merge can't be
    // rolled back, so the gateway records the divergence BEST-EFFORT: ActionPartiallySucceeded (the loud
    // audit-integrity record), NOT a clean rollback — settled partially_succeeded, never claimed succeeded.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut catalog = CatalogExecutor::new();
    catalog.register(
        ExecutorKind::Github,
        Arc::new(GithubExecutor::new(
            Box::new(FakeGithubWriteClient::merged(MergedPr {
                merge_commit_sha: Some("abc".to_string()),
            })),
            rt.handle().clone(),
            Box::new(FixedClock::new(FIXED_TS)),
        )),
    );
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(catalog));
    gw.submit_action(&mut store, default_merge_req())
        .expect("submit");

    arm(FaultPoint::TerminalEventWrite); // fail the terminal ActionSucceeded append (txn-B)
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
        "the ActionPartiallySucceeded audit-integrity record is emitted, got {types:?}"
    );
    assert!(
        !types.contains(&"ActionSucceeded".to_string()),
        "NO ActionSucceeded — the terminal success write failed (the whole point)"
    );
    // the merge's PullRequestMerged rode txn-B with ActionSucceeded → both rolled back (NOT persisted).
    assert!(
        !types.contains(&"PullRequestMerged".to_string()),
        "PullRequestMerged was in the rolled-back txn-B → not persisted on a partial, got {types:?}"
    );
}

// ---- 11. map_merge_method — fail-closed mapping ----------------------------

#[test]
fn test_map_merge_method() {
    // spec(F2 fail-closed mapping) — merge/squash/rebase map 1:1 to octocrab's MergeMethod (case-
    // insensitive + trimmed for API resilience); any unknown value → a fail-closed Err (NEVER a silent
    // default — the approved+audited method must execute exactly).
    assert_eq!(map_merge_method("merge"), Ok(MergeMethod::Merge));
    assert_eq!(map_merge_method("squash"), Ok(MergeMethod::Squash));
    assert_eq!(map_merge_method("rebase"), Ok(MergeMethod::Rebase));
    // case-insensitive + trimmed (the parse_* precedent).
    assert_eq!(map_merge_method("SQUASH"), Ok(MergeMethod::Squash));
    assert_eq!(map_merge_method("  rebase "), Ok(MergeMethod::Rebase));
    // unknown / empty → fail-closed Err.
    for unknown in ["fast_forward", "", "  ", "merge_commit", "rebase-merge"] {
        assert!(
            map_merge_method(unknown).is_err(),
            "an unknown merge_method `{unknown}` must fail closed (Err), never a silent default"
        );
    }
}

// =============================================================================
// P4.7 — head_sha safety pins (the SHA-pin axis; security-reviewer `invariant`).
// Source-level structural pins (the `test_live_session_create_has_interception`
// precedent): they assert a PROPERTY of github_write.rs that no value-test can.
// =============================================================================

/// The github_write.rs (D9/D10 mutation executors) source.
fn github_write_src() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/integrations/github_write.rs"
    ))
    .expect("github_write.rs present")
}

#[test]
fn head_sha_sourced_only_from_producer() {
    // spec(P4.7 safety (a)): `head_sha` is a producer-sourced, read-only PROJECTION field — it has NO
    // inbound path into the cat-1 mutation executors. The mutation file never references `head_sha`, so a
    // proposer/UI value can never reach the merge/review via this field (it flows only
    // extract_pr_signals → PullRequestSynced → fold → typed serve → UI). Unspoofable by construction.
    assert!(
        !github_write_src().contains("head_sha"),
        "github_write.rs must NOT reference head_sha — the projection field has no path into the cat-1 \
         mutation; the SHA-pin reads the requester-supplied sha/commit_id (safety (a))"
    );
}

#[test]
fn d9_d10_executors_unchanged() {
    // spec(P4.7 safety (b)): the daemon's anti-race stays the LIVE GitHub 409 on the REQUESTER-supplied
    // sha/commit_id (from req.inputs), NOT this projection field. This slice does NOT touch github_write.rs;
    // the pin mechanism (D9 `args.sha` → octocrab merge .sha(); D10 `args.commit_id`) is intact. The
    // existing D9/D10 value-tests in this suite stay green (the full-suite non-regression).
    // Tight substrings unique to the ACTUAL executor pin sites (not comments) — a greedy `contains`
    // would pass even if the call were deleted and only a comment survived. The real non-regression
    // guarantee is the D9/D10 value-tests in this suite staying green; these pin the live call shape.
    let src = github_write_src();
    assert!(
        src.contains(".sha(args.sha"),
        "D9 still SHA-pins the live merge to the requester-supplied args.sha (the 409 anti-race, safety (b))"
    );
    assert!(
        src.contains("\"commit_id\": args.commit_id"),
        "D10 still pins the live review verdict to the requester-supplied commit_id (safety (b))"
    );
}

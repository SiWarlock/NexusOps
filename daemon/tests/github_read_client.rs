//! P7.1 — the GitHub PR read client (edges-009). Closes the GitHub-PR read vertical: live GitHub →
//! signals → §5.1, all in-lane (the `github` executor + `proj_pull_request` projector + the
//! `PullRequestSynced` event stay gated on the R1 daemon seam).
//!
//! The DETERMINISTIC core — `extract_pr_signals(pr, reviews, check_runs)` — is fixture-driven
//! test-first: recorded public GitHub JSON deserializes through octocrab's models, then the
//! extraction stringifies octocrab's typed fields into edges-008's `signals_from_github_response`.
//! The live HTTP round-trip is the non-deterministic edge, covered by `FakeGithubReadClient`
//! (CLAUDE.md: real GitHub calls → recorded fixtures/fakes).
//!
//! octocrab 0.53.1 reality (confirmed against the pinned source): `state`/`mergeable_state`/review
//! `state` are TYPED enums (not strings) → stringified into edges-008's decoders; `CheckRun` exposes
//! ONLY `conclusion: Option<String>` (NO `status` field) → completed-ness derived from
//! `conclusion.is_some()` (GitHub sets `conclusion` only once a run completes). Fixtures are public
//! PR/review/check API shapes — never real secrets/tokens.

use nexusops_shared::status::PullRequest;
use nexusopsd::integrations::classifier::IntegrationOutcomeClass;
use nexusopsd::integrations::github::{
    extract_pr_signals, FakeGithubReadClient, GithubReadClient, GithubReadError,
};
use nexusopsd::integrations::pull_request::{
    derive_pull_request_status, ChecksConclusion, GitHubMergeableState, Mergeability, PrState,
    PullRequestSignals, ReviewDecision, ReviewState,
};
use octocrab::models::checks::CheckRun;
use octocrab::models::pulls::{PullRequest as OctoPr, Review};

// ---- recorded public-shape fixtures (inline const — hermetic, no fixture-file IO) --------------

const PR_OPEN_DIRTY: &str = r#"{"locked":false,"number":7,"state":"open","mergeable_state":"dirty","draft":false,"merged":false}"#;
const PR_OPEN_CLEAN: &str = r#"{"locked":false,"number":7,"state":"open","mergeable_state":"clean","draft":false,"merged":false}"#;
const PR_DRAFT: &str = r#"{"locked":false,"number":7,"state":"open","mergeable_state":"clean","draft":true,"merged":false}"#;
// mergeable_state ABSENT → octocrab Option<MergeableState> = None → conservative Unknown.
const PR_OPEN_NO_MERGEABLE: &str =
    r#"{"locked":false,"number":7,"state":"open","draft":false,"merged":false}"#;
// terminal state: IssueState::Closed + merged=true → the Merged plumbing path.
const PR_CLOSED_MERGED: &str = r#"{"locked":false,"number":7,"state":"closed","mergeable_state":"clean","draft":false,"merged":true}"#;

const REVIEW_APPROVED: &str = r#"{"id":1,"node_id":"PRR_1","html_url":"https://github.com/o/r/pull/7#pullrequestreview-1","state":"APPROVED"}"#;

const CHECK_SUCCESS: &str = r#"{"id":1,"node_id":"CR_1","head_sha":"abc123","url":"https://api.github.com/repos/o/r/check-runs/1","conclusion":"success","output":{"title":null,"summary":null,"text":null,"annotations_count":0,"annotations_url":"https://api.github.com/x"},"name":"build","pull_requests":[]}"#;
const CHECK_FAILURE: &str = r#"{"id":2,"node_id":"CR_2","head_sha":"abc123","url":"https://api.github.com/repos/o/r/check-runs/2","conclusion":"failure","output":{"title":null,"summary":null,"text":null,"annotations_count":0,"annotations_url":"https://api.github.com/x"},"name":"test","pull_requests":[]}"#;

fn pr(json: &str) -> OctoPr {
    serde_json::from_str(json).expect("PR fixture deserializes through octocrab")
}
fn review(json: &str) -> Review {
    serde_json::from_str(json).expect("Review fixture deserializes through octocrab")
}
fn check(json: &str) -> CheckRun {
    serde_json::from_str(json).expect("CheckRun fixture deserializes through octocrab")
}

// ---- extraction fixtures → derived §5.1 status (the deterministic core) -------------------------

#[test]
fn extract_open_dirty_approved_passing_is_conflict() {
    // spec(§7.2/§5.1): octocrab models → extract → derive == Conflict (Conflict > review, end-to-end
    // through the octocrab-model layer).
    let s = extract_pr_signals(
        &pr(PR_OPEN_DIRTY),
        &[review(REVIEW_APPROVED)],
        &[check(CHECK_SUCCESS)],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::Conflict);
}

#[test]
fn extract_clean_approved_failing_check_is_checks_failing() {
    // spec(§5.1): a HARD failing check overrides an approving review through the octocrab extraction.
    let s = extract_pr_signals(
        &pr(PR_OPEN_CLEAN),
        &[review(REVIEW_APPROVED)],
        &[check(CHECK_FAILURE)],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::ChecksFailing);
}

#[test]
fn extract_clean_approved_passing_is_mergeable() {
    // spec(§5.1): approved + clean + checks-success → Mergeable (fully ready).
    let s = extract_pr_signals(
        &pr(PR_OPEN_CLEAN),
        &[review(REVIEW_APPROVED)],
        &[check(CHECK_SUCCESS)],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::Mergeable);
}

#[test]
fn extract_closed_merged_is_merged() {
    // spec(§5.1): the terminal-state plumbing — octocrab IssueState::Closed → PrState::Closed + the
    // merged=true bool → derive == Merged (the most-salient §5.1 value, unexercised by the open fixtures).
    let s = extract_pr_signals(&pr(PR_CLOSED_MERGED), &[], &[]);
    assert_eq!(s.state, PrState::Closed);
    assert!(s.merged);
    assert_eq!(derive_pull_request_status(&s), PullRequest::Merged);
}

#[test]
fn extract_draft_is_draft() {
    // spec(§5.1): a draft PR → Draft (draft precedence survives the octocrab extraction).
    let s = extract_pr_signals(&pr(PR_DRAFT), &[], &[]);
    assert_eq!(derive_pull_request_status(&s), PullRequest::Draft);
}

#[test]
fn extract_option_none_fields_degrade_conservative() {
    // spec(§5.1): octocrab Option-None fields degrade to the conservative floor — absent
    // mergeable_state → Unknown, no reviews → None, no checks → None → derive Open (nothing fabricated).
    let s = extract_pr_signals(&pr(PR_OPEN_NO_MERGEABLE), &[], &[]);
    assert_eq!(s.mergeable, Mergeability::Unknown);
    assert_eq!(s.review, ReviewDecision::None);
    assert_eq!(s.checks, ChecksConclusion::None);
    assert_eq!(derive_pull_request_status(&s), PullRequest::Open);
}

#[test]
fn extract_empty_reviews_and_checks_lists() {
    // spec(§9): empty reviews/check-runs → None/None aggregates (edges-006 aggregate semantics).
    let s = extract_pr_signals(&pr(PR_OPEN_CLEAN), &[], &[]);
    assert_eq!(s.review, ReviewDecision::None);
    assert_eq!(s.checks, ChecksConclusion::None);
}

#[test]
fn extract_running_check_is_pending_not_failure() {
    // spec(§9): a check-run with conclusion ABSENT (still running) → Pending — octocrab's CheckRun has
    // NO `status` field, so completed-ness is derived from conclusion.is_some(); a running check can
    // never read Failure. Pins the no-status synthesis.
    let running = r#"{"id":3,"node_id":"CR_3","head_sha":"abc123","url":"https://api.github.com/repos/o/r/check-runs/3","conclusion":null,"output":{"title":null,"summary":null,"text":null,"annotations_count":0,"annotations_url":"https://api.github.com/x"},"name":"slow","pull_requests":[]}"#;
    let s = extract_pr_signals(&pr(PR_OPEN_CLEAN), &[], &[check(running)]);
    assert_eq!(s.checks, ChecksConclusion::Pending);
}

// ---- the trait seam (the gated projector/executor consume this) --------------------------------

#[tokio::test]
async fn fake_client_returns_canned_signals() {
    // spec(§7.2): FakeGithubReadClient is the seam the gated proj_pull_request projector + github
    // executor consume in tests — it returns the canned Ok(signals).
    let signals = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[ReviewState::Approved],
        &[],
    );
    let fake = FakeGithubReadClient::new(Ok(signals.clone()));
    let got = fake.fetch_pr_signals("o", "r", 7).await;
    assert_eq!(got, Ok(signals));
}

#[tokio::test]
async fn fake_client_returns_canned_error() {
    // spec(§17): the error carries IntegrationOutcomeClass (NOT the collapsed DeliveryOutcome) so the
    // gated SyncFailed/auth_expired path can branch on AuthFailed vs ClientError.
    let err = GithubReadError {
        class: IntegrationOutcomeClass::AuthFailed,
        message: "401 Unauthorized".to_owned(),
    };
    let fake = FakeGithubReadClient::new(Err(err.clone()));
    let got = fake.fetch_pr_signals("o", "r", 7).await;
    assert_eq!(got, Err(err));
    // the §17 forward-constraint: the carried class is reachable + distinct (AuthFailed, not a
    // collapsed terminal) — the field the gated SyncFailed/auth_expired path branches on.
    assert_eq!(got.unwrap_err().class, IntegrationOutcomeClass::AuthFailed);
}

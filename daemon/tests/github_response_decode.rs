//! P7.1 — GitHub raw-API-response → daemon-enum decode layer (edges-008). One layer BELOW
//! edges-006's `from_github` aggregation: maps GitHub's raw REST string values (top-level `state`,
//! `mergeable_state`, each review's `state`, each check-run's `status`+`conclusion`) into the
//! daemon-internal enums `from_github` consumes, plus a `signals_from_github_response` composer that
//! runs the full raw → signals decode in one call. Pure — NO octocrab/async/network (the gated
//! octocrab client calls these parsers on its response strings).
//!
//! Decode is TOTAL + CONSERVATIVE: an unrecognized API string maps to the least-salient variant so
//! it can never fabricate a more-blocking / terminal §5.1 status (pr_state→Open, mergeable→Unknown,
//! review→Commented, check→Neutral). Case-insensitive (normalized before match) for API-casing drift.

use nexusops_shared::status::PullRequest;
use nexusopsd::integrations::pull_request::{
    derive_pull_request_status, parse_check_conclusion, parse_mergeable_state, parse_pr_state,
    parse_review_state, signals_from_github_response, CheckConclusion, GitHubMergeableState,
    PrState, PullRequestSignals, ReviewState,
};

// ---- parse_pr_state --------------------------------------------------------

#[test]
fn parse_pr_state_open_closed_unknown() {
    // spec(§7.2): top-level PR state; an unrecognized string must NOT fabricate the terminal Closed.
    assert_eq!(parse_pr_state("open"), PrState::Open);
    assert_eq!(parse_pr_state("closed"), PrState::Closed);
    assert_eq!(parse_pr_state("weird"), PrState::Open);
    // case-insensitive (the module-header claim) — pins it for parse_pr_state.
    assert_eq!(parse_pr_state("CLOSED"), PrState::Closed);
}

// ---- parse_mergeable_state -------------------------------------------------

#[test]
fn parse_mergeable_state_all_known() {
    // spec(§9): each documented GitHub mergeable_state decodes to its variant.
    use GitHubMergeableState as G;
    assert_eq!(parse_mergeable_state("clean"), G::Clean);
    assert_eq!(parse_mergeable_state("dirty"), G::Dirty);
    assert_eq!(parse_mergeable_state("blocked"), G::Blocked);
    assert_eq!(parse_mergeable_state("behind"), G::Behind);
    assert_eq!(parse_mergeable_state("unstable"), G::Unstable);
    assert_eq!(parse_mergeable_state("unknown"), G::Unknown);
    // case-insensitive (the module-header claim) — pins it for parse_mergeable_state.
    assert_eq!(parse_mergeable_state("DIRTY"), G::Dirty);
}

#[test]
fn parse_mergeable_state_unrecognized_is_unknown() {
    // spec(§9): conservative "not a conflict" floor — unrecognized/empty → Unknown (map_mergeable doc).
    use GitHubMergeableState as G;
    assert_eq!(parse_mergeable_state("draft"), G::Unknown);
    assert_eq!(parse_mergeable_state("has_hooks"), G::Unknown);
    assert_eq!(parse_mergeable_state(""), G::Unknown);
}

// ---- parse_review_state ----------------------------------------------------

#[test]
fn parse_review_state_uppercase_set() {
    // spec(§9): GitHub's UPPERCASE review states decode; an unrecognized string → Commented
    // (carries no decision in aggregate_reviews).
    assert_eq!(parse_review_state("APPROVED"), ReviewState::Approved);
    assert_eq!(
        parse_review_state("CHANGES_REQUESTED"),
        ReviewState::ChangesRequested
    );
    assert_eq!(parse_review_state("COMMENTED"), ReviewState::Commented);
    assert_eq!(parse_review_state("DISMISSED"), ReviewState::Dismissed);
    assert_eq!(parse_review_state("PENDING"), ReviewState::Pending);
    assert_eq!(parse_review_state("FOO"), ReviewState::Commented);
}

#[test]
fn parse_review_state_case_insensitive() {
    // spec(§9): case-normalized before match — defensive against GitHub casing drift (the Q2 default).
    assert_eq!(parse_review_state("approved"), ReviewState::Approved);
    assert_eq!(
        parse_review_state("Changes_Requested"),
        ReviewState::ChangesRequested
    );
}

// ---- parse_check_conclusion (two-field status + conclusion) ----------------

#[test]
fn parse_check_pending_when_not_completed() {
    // spec(§9): a running build is SOFT-pending; conclusion is ignored until status=="completed".
    assert_eq!(
        parse_check_conclusion("queued", None),
        CheckConclusion::Pending
    );
    assert_eq!(
        parse_check_conclusion("in_progress", None),
        CheckConclusion::Pending
    );
    // The load-bearing conservative-floor guard: a non-completed status with a failure conclusion
    // STILL reads Pending — a running check can never fabricate Failure (the early-return precedes
    // the conclusion match). Pins the invariant against a future reorder.
    assert_eq!(
        parse_check_conclusion("in_progress", Some("failure")),
        CheckConclusion::Pending
    );
}

#[test]
fn parse_check_completed_conclusions() {
    // spec(§9): completed-by-conclusion collapse (the CheckConclusion doc, verbatim).
    assert_eq!(
        parse_check_conclusion("completed", Some("success")),
        CheckConclusion::Success
    );
    // case-insensitive on both fields (the module-header claim) — pins it for parse_check_conclusion.
    assert_eq!(
        parse_check_conclusion("COMPLETED", Some("SUCCESS")),
        CheckConclusion::Success
    );
    for fail in ["failure", "timed_out", "action_required"] {
        assert_eq!(
            parse_check_conclusion("completed", Some(fail)),
            CheckConclusion::Failure,
            "{fail}"
        );
    }
    for neutral in ["neutral", "cancelled", "skipped", "stale"] {
        assert_eq!(
            parse_check_conclusion("completed", Some(neutral)),
            CheckConclusion::Neutral,
            "{neutral}"
        );
    }
}

#[test]
fn parse_check_completed_missing_or_unknown_conclusion_is_neutral() {
    // spec(§9): total + conservative — a completed check with no/unknown conclusion → Neutral (ignored).
    assert_eq!(
        parse_check_conclusion("completed", None),
        CheckConclusion::Neutral
    );
    assert_eq!(
        parse_check_conclusion("completed", Some("bogus")),
        CheckConclusion::Neutral
    );
}

// ---- signals_from_github_response (the composer) ---------------------------

#[test]
fn signals_from_github_response_composes_to_from_github() {
    // spec(§9): the composer is exactly the parser-composition over from_github — no extra logic.
    let decoded = signals_from_github_response(
        "open",
        false,
        false,
        "dirty",
        &["APPROVED", "COMMENTED"],
        &[("completed", Some("success")), ("in_progress", None)],
    );
    let hand_built = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Dirty,
        &[ReviewState::Approved, ReviewState::Commented],
        &[CheckConclusion::Success, CheckConclusion::Pending],
    );
    // The composer is exactly parser-composition over from_github — whole-struct equality (PartialEq).
    assert_eq!(decoded, hand_built);
}

// ---- end-to-end: decode → from_github → derive (§5.1) ----------------------

#[test]
fn decode_to_derive_dirty_dominates_approved() {
    // spec(§5.1): the full raw → signals → status chain — a dirty open PR is Conflict even with an
    // approving review + passing checks (edges-004/006 precedence: Conflict > review).
    let s = signals_from_github_response(
        "open",
        false,
        false,
        "dirty",
        &["APPROVED"],
        &[("completed", Some("success"))],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::Conflict);
}

#[test]
fn decode_to_derive_failing_check_dominates_approved() {
    // spec(§5.1): a HARD failing check overrides an approving review through the raw decode →
    // ChecksFailing (pins the HARD-vs-SOFT distinction survives the decode).
    let s = signals_from_github_response(
        "open",
        false,
        false,
        "clean",
        &["APPROVED"],
        &[("completed", Some("failure"))],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::ChecksFailing);
}

#[test]
fn decode_to_derive_empty_slices_is_open() {
    // spec(§5.1): the steady-state freshly-opened PR — no reviews, no check-runs (empty slices) →
    // None review + None checks → the Open baseline (the composer handles empty slices).
    let s = signals_from_github_response("open", false, false, "clean", &[], &[]);
    assert_eq!(
        s,
        PullRequestSignals::from_github(
            PrState::Open,
            false,
            false,
            GitHubMergeableState::Clean,
            &[],
            &[],
        )
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::Open);
}

#[test]
fn decode_to_derive_closed_merged() {
    // spec(§5.1): the state/merged params wire through the composer — a closed+merged PR → Merged
    // (a terminal path through the raw decode, complementing the open-state pins above).
    let s = signals_from_github_response("closed", false, true, "unknown", &[], &[]);
    assert_eq!(derive_pull_request_status(&s), PullRequest::Merged);
}

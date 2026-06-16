//! P7.1 — `PullRequestSignals::from_github` (aggregation). Closes the GitHub-PR read chain: raw
//! GitHub facts (state/draft/merged/mergeable-state + the reviews list + the check-runs list) →
//! daemon-internal `PullRequestSignals` (edges-004) → `derive_pull_request_status` → the §5.1
//! `PullRequest` status. Pure — NO octocrab/async/network (the octocrab client that populates these
//! daemon-internal inputs is the gated follow-on).
//!
//! Aggregation rules: review = any ChangesRequested > any Approved > None (ReviewRequired is a
//! branch-protection signal not derivable from reviews[] alone — deferred to the octocrab client);
//! checks = any Failure > any Pending > any Success > None (Neutral ignored); mergeable = dirty →
//! Conflicting, clean → Clean, else → Unknown.

use nexusops_shared::status::PullRequest;
use nexusopsd::integrations::pull_request::{
    derive_pull_request_status, CheckConclusion, ChecksConclusion, GitHubMergeableState,
    Mergeability, PrState, PullRequestSignals, ReviewDecision, ReviewState,
};

// ---- review aggregation ----------------------------------------------------

#[test]
fn review_changes_requested_wins() {
    // spec(§9): any ChangesRequested dominates an Approved in the reviews list.
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Unknown,
        &[ReviewState::Approved, ReviewState::ChangesRequested],
        &[],
    );
    assert_eq!(s.review, ReviewDecision::ChangesRequested);
}

#[test]
fn review_approved() {
    // spec(§9): an Approved review (no changes-requested) → Approved; Commented is ignored.
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[ReviewState::Commented, ReviewState::Approved],
        &[],
    );
    assert_eq!(s.review, ReviewDecision::Approved);
}

#[test]
fn review_none_when_undecided() {
    // spec(§9): no decisive review (empty, or only Commented) → None. ReviewRequired is a branch-
    // protection signal not derivable from reviews[] alone (deferred to the octocrab client).
    let empty = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Unknown,
        &[],
        &[],
    );
    assert_eq!(empty.review, ReviewDecision::None);
    let commented = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Unknown,
        &[ReviewState::Commented],
        &[],
    );
    assert_eq!(commented.review, ReviewDecision::None);
    // the other non-decisive review states carry no decision either
    for non_decisive in [ReviewState::Dismissed, ReviewState::Pending] {
        let s = PullRequestSignals::from_github(
            PrState::Open,
            false,
            false,
            GitHubMergeableState::Unknown,
            &[non_decisive],
            &[],
        );
        assert_eq!(s.review, ReviewDecision::None, "{non_decisive:?}");
    }
}

// ---- checks aggregation ----------------------------------------------------

#[test]
fn checks_failing_wins() {
    // spec(§9): any Failure dominates → Failing.
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[],
        &[CheckConclusion::Success, CheckConclusion::Failure],
    );
    assert_eq!(s.checks, ChecksConclusion::Failing);
}

#[test]
fn checks_failure_dominates_pending() {
    // spec(§9): Failure outranks Pending directly (pins the Failure>Pending tier).
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[],
        &[CheckConclusion::Failure, CheckConclusion::Pending],
    );
    assert_eq!(s.checks, ChecksConclusion::Failing);
}

#[test]
fn checks_pending() {
    // spec(§9): a Pending check (no Failure) → Pending.
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[],
        &[CheckConclusion::Success, CheckConclusion::Pending],
    );
    assert_eq!(s.checks, ChecksConclusion::Pending);
}

#[test]
fn checks_all_success() {
    // spec(§9): all-success → Success.
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[],
        &[CheckConclusion::Success, CheckConclusion::Success],
    );
    assert_eq!(s.checks, ChecksConclusion::Success);
}

#[test]
fn checks_empty_none() {
    // spec(§9): no check-runs → None.
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[],
        &[],
    );
    assert_eq!(s.checks, ChecksConclusion::None);
}

#[test]
fn checks_neutral_ignored() {
    // spec(§9): Neutral checks are ignored — [Neutral] → None; [Neutral, Success] → Success.
    let only_neutral = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[],
        &[CheckConclusion::Neutral],
    );
    assert_eq!(only_neutral.checks, ChecksConclusion::None);
    let neutral_plus_success = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[],
        &[CheckConclusion::Neutral, CheckConclusion::Success],
    );
    assert_eq!(neutral_plus_success.checks, ChecksConclusion::Success);
}

// ---- mergeable mapping + carry-through -------------------------------------

#[test]
fn mergeable_state_mapping() {
    // spec(§7.2): the MVP collapse — dirty→Conflicting, clean→Clean, blocked/behind/unstable/unknown→Unknown.
    use GitHubMergeableState as G;
    let cases = [
        (G::Dirty, Mergeability::Conflicting),
        (G::Clean, Mergeability::Clean),
        (G::Blocked, Mergeability::Unknown),
        (G::Behind, Mergeability::Unknown),
        (G::Unstable, Mergeability::Unknown),
        (G::Unknown, Mergeability::Unknown),
    ];
    for (gh, expected) in cases {
        let s = PullRequestSignals::from_github(PrState::Open, false, false, gh, &[], &[]);
        assert_eq!(s.mergeable, expected, "mergeable {gh:?}");
    }
}

#[test]
fn state_draft_merged_carried() {
    // spec(§9): state/draft/merged pass through to the signals unchanged.
    let s = PullRequestSignals::from_github(
        PrState::Closed,
        true,
        true,
        GitHubMergeableState::Clean,
        &[],
        &[],
    );
    assert_eq!(s.state, PrState::Closed);
    assert!(s.draft);
    assert!(s.merged);
}

// ---- end-to-end chain (compose with edges-004's derivation) ----------------

#[test]
fn chain_merged_to_status() {
    // spec(§5.1): a merged PR — from_github → derive_pull_request_status → Merged (end-to-end).
    let s = PullRequestSignals::from_github(
        PrState::Closed,
        false,
        true,
        GitHubMergeableState::Unknown,
        &[],
        &[],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::Merged);
}

#[test]
fn chain_approved_clean_success_to_mergeable() {
    // spec(§5.1): approved + clean + all-checks-success → Mergeable (the happy chain end-to-end).
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Clean,
        &[ReviewState::Approved],
        &[CheckConclusion::Success],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::Mergeable);
}

#[test]
fn chain_conflict_to_status() {
    // spec(§5.1): a dirty (conflicting) open PR → from_github → derive → Conflict — a non-happy chain
    // end-to-end (the conflict outranks an Approved review + passing checks).
    let s = PullRequestSignals::from_github(
        PrState::Open,
        false,
        false,
        GitHubMergeableState::Dirty,
        &[ReviewState::Approved],
        &[CheckConclusion::Success],
    );
    assert_eq!(derive_pull_request_status(&s), PullRequest::Conflict);
}

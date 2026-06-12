//! P7.1 — the §5.1 PullRequest status-derivation fn. `derive_pull_request_status` maps daemon-internal
//! GitHub PR signals (state / merged / draft / mergeability / review-decision / checks-conclusion) to
//! the FROZEN 11-state §5.1 `PullRequest` enum via a precedence — the deterministic core of the
//! `proj_pull_request` cache (§7.2, GitHub-authoritative). Same pure-fn pattern as the worktree
//! precedence (edges-002). The octocrab adapter (GitHub API → signals) + the projector that call it
//! are gated/deferred.
//!
//! Precedence (most → least salient): `Merged`/`Closed` (terminal) > `Draft` > `Conflict` >
//! `ChecksFailing` > {review: `ChangesRequested` | `Mergeable` | `Approved` | `NeedsReview`} >
//! `ChecksPending` > `Open`. `ChecksFailing` is a HARD blocker (outranks the review decision); a
//! PENDING build is SOFT (shown only when there is no review decision — so an approved-but-pending PR
//! reads `Approved`, not `ChecksPending`).

use nexusops_shared::status::PullRequest;
use nexusopsd::integrations::pull_request::{
    derive_pull_request_status, ChecksConclusion, Mergeability, PrState, PullRequestSignals,
    ReviewDecision,
};

#[test]
fn pr_merged_terminal() {
    // spec(§5.1): Closed + merged → Merged (terminal; dominates conflict/failing-checks).
    let s = PullRequestSignals {
        state: PrState::Closed,
        merged: true,
        mergeable: Mergeability::Conflicting,
        checks: ChecksConclusion::Failing,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::Merged);
}

#[test]
fn pr_closed_terminal() {
    // spec(§5.1): Closed + !merged → Closed (terminal; dominates open signals).
    let s = PullRequestSignals {
        state: PrState::Closed,
        review: ReviewDecision::Approved,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::Closed);
}

#[test]
fn pr_draft() {
    // spec(§5.1): Open + draft → Draft (precedes the review flow; outranks conflict/review).
    let s = PullRequestSignals {
        draft: true,
        mergeable: Mergeability::Conflicting,
        review: ReviewDecision::ChangesRequested,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::Draft);
}

#[test]
fn pr_conflict() {
    // spec(§5.1): a merge conflict → Conflict (blocks merge; outranks checks + review).
    let s = PullRequestSignals {
        mergeable: Mergeability::Conflicting,
        checks: ChecksConclusion::Failing,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::Conflict);
}

#[test]
fn pr_conflict_over_approved() {
    // spec(§5.1): a conflicting merge outranks an Approved review (step 3 before the review block) —
    // even with checks passing, the review never bleeds through.
    let s = PullRequestSignals {
        mergeable: Mergeability::Conflicting,
        review: ReviewDecision::Approved,
        checks: ChecksConclusion::Success,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::Conflict);
}

#[test]
fn pr_checks_failing() {
    // spec(§5.1): failing checks → ChecksFailing (a HARD blocker — outranks the review decision).
    let s = PullRequestSignals {
        checks: ChecksConclusion::Failing,
        review: ReviewDecision::Approved,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::ChecksFailing);
}

#[test]
fn pr_checks_pending() {
    // spec(§5.1): pending checks + no review decision → ChecksPending.
    let s = PullRequestSignals {
        checks: ChecksConclusion::Pending,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::ChecksPending);
}

#[test]
fn pr_changes_requested() {
    // spec(§5.1): review = ChangesRequested → ChangesRequested.
    let s = PullRequestSignals {
        review: ReviewDecision::ChangesRequested,
        ..Default::default()
    };
    assert_eq!(
        derive_pull_request_status(&s),
        PullRequest::ChangesRequested
    );
}

#[test]
fn pr_mergeable() {
    // spec(§5.1): Approved + clean + checks-pass → Mergeable (ready to merge).
    let s = PullRequestSignals {
        review: ReviewDecision::Approved,
        mergeable: Mergeability::Clean,
        checks: ChecksConclusion::Success,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::Mergeable);
}

#[test]
fn pr_approved_not_clean() {
    // spec(§5.1): Approved but mergeable-unknown / checks-pending → Approved (NOT Mergeable, NOT
    // ChecksPending) — the approved-vs-mergeable distinction (review decision outranks a pending build).
    let s = PullRequestSignals {
        review: ReviewDecision::Approved,
        mergeable: Mergeability::Unknown,
        checks: ChecksConclusion::Pending,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::Approved);
}

#[test]
fn pr_needs_review() {
    // spec(§5.1): review required, open, no blocker → NeedsReview.
    let s = PullRequestSignals {
        review: ReviewDecision::ReviewRequired,
        checks: ChecksConclusion::Success,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::NeedsReview);
}

#[test]
fn pr_open_baseline() {
    // spec(§5.1): an open PR with no signals → Open (baseline).
    assert_eq!(
        derive_pull_request_status(&PullRequestSignals::default()),
        PullRequest::Open
    );
}

#[test]
fn pr_unknown_mergeable_not_conflict() {
    // spec(§7.2): Unknown mergeability (GitHub still computing) is NOT Conflict — it falls through to
    // the review/checks signals (conservative: never show Conflict on a not-yet-computed merge).
    let s = PullRequestSignals {
        mergeable: Mergeability::Unknown,
        review: ReviewDecision::ReviewRequired,
        ..Default::default()
    };
    assert_eq!(derive_pull_request_status(&s), PullRequest::NeedsReview);
}

#[test]
fn pr_full_precedence() {
    // spec(§7.2): the derived-cache total precedence — terminal > Draft > Conflict > ChecksFailing >
    // {review decision} > ChecksPending > Open. Each row = (signals → expected most-salient status).
    let cases: &[(PullRequestSignals, PullRequest)] = &[
        // terminal dominates every open signal
        (
            PullRequestSignals {
                state: PrState::Closed,
                merged: true,
                mergeable: Mergeability::Conflicting,
                checks: ChecksConclusion::Failing,
                ..Default::default()
            },
            PullRequest::Merged,
        ),
        (
            PullRequestSignals {
                state: PrState::Closed,
                ..Default::default()
            },
            PullRequest::Closed,
        ),
        // Draft over conflict/checks/review
        (
            PullRequestSignals {
                draft: true,
                mergeable: Mergeability::Conflicting,
                checks: ChecksConclusion::Failing,
                review: ReviewDecision::ChangesRequested,
                ..Default::default()
            },
            PullRequest::Draft,
        ),
        // Conflict over ChecksFailing + review
        (
            PullRequestSignals {
                mergeable: Mergeability::Conflicting,
                checks: ChecksConclusion::Failing,
                review: ReviewDecision::Approved,
                ..Default::default()
            },
            PullRequest::Conflict,
        ),
        // ChecksFailing (hard) over the review decision
        (
            PullRequestSignals {
                checks: ChecksConclusion::Failing,
                review: ReviewDecision::Approved,
                ..Default::default()
            },
            PullRequest::ChecksFailing,
        ),
        (
            PullRequestSignals {
                checks: ChecksConclusion::Failing,
                review: ReviewDecision::ChangesRequested,
                ..Default::default()
            },
            PullRequest::ChecksFailing,
        ),
        // review decision over a PENDING build (the approved-not-clean class)
        (
            PullRequestSignals {
                review: ReviewDecision::ChangesRequested,
                checks: ChecksConclusion::Pending,
                ..Default::default()
            },
            PullRequest::ChangesRequested,
        ),
        (
            PullRequestSignals {
                review: ReviewDecision::Approved,
                mergeable: Mergeability::Clean,
                checks: ChecksConclusion::Success,
                ..Default::default()
            },
            PullRequest::Mergeable,
        ),
        (
            PullRequestSignals {
                review: ReviewDecision::Approved,
                checks: ChecksConclusion::Pending,
                ..Default::default()
            },
            PullRequest::Approved,
        ),
        (
            PullRequestSignals {
                review: ReviewDecision::ReviewRequired,
                checks: ChecksConclusion::Pending,
                ..Default::default()
            },
            PullRequest::NeedsReview,
        ),
        // ChecksPending (soft) over Open — only when there is no review decision
        (
            PullRequestSignals {
                checks: ChecksConclusion::Pending,
                ..Default::default()
            },
            PullRequest::ChecksPending,
        ),
        (PullRequestSignals::default(), PullRequest::Open),
    ];
    for (signals, expected) in cases {
        assert_eq!(
            derive_pull_request_status(signals),
            *expected,
            "signals: {signals:?}"
        );
    }
}

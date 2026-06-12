//! §5.1 PullRequest status-derivation — pure + deterministic.
//!
//! `derive_pull_request_status` maps daemon-internal GitHub PR signals to the FROZEN 11-state §5.1
//! `PullRequest` enum (the §7.2 GitHub-authoritative `proj_pull_request` cache value). The octocrab
//! adapter (GitHub API → `PullRequestSignals`) + the projector that call it are gated/deferred. Same
//! pure-precedence pattern as the worktree fn (edges-002).
//!
//! Precedence (most → least salient):
//!
//! ```text
//! Merged > Closed > Draft > Conflict > ChecksFailing
//!   > { review-block: ChangesRequested | Mergeable | Approved | NeedsReview } > ChecksPending > Open
//! ```
//!
//! `ChecksFailing` is a HARD blocker (overrides the review decision — even an approved PR shows it);
//! a PENDING build is SOFT (yields to the review decision — an approved-but-CI-running PR reads
//! `Approved`, not `ChecksPending`).

use nexusops_shared::status::PullRequest;

/// PR top-level state (GitHub `state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrState {
    #[default]
    Open,
    Closed,
}

/// Merge-readiness — GitHub's `mergeable_state` collapsed for the MVP (Blocked/Behind/Unstable refine
/// at the octocrab adapter). `Unknown` = GitHub still computing → NOT treated as a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mergeability {
    Clean,
    Conflicting,
    #[default]
    Unknown,
}

/// Aggregate review decision (GitHub `reviewDecision`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    ReviewRequired,
    #[default]
    None,
}

/// Aggregate CI / checks conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChecksConclusion {
    Success,
    Failing,
    Pending,
    #[default]
    None,
}

/// Daemon-internal GitHub PR signals — the octocrab adapter maps a GitHub API response into these.
#[derive(Debug, Clone, Default)]
pub struct PullRequestSignals {
    pub state: PrState,
    pub merged: bool,
    pub draft: bool,
    pub mergeable: Mergeability,
    pub review: ReviewDecision,
    pub checks: ChecksConclusion,
}

/// Derive the single most-salient §5.1 `PullRequest` status from the signals (§7.2 derived cache).
/// Total + deterministic — every signal combination resolves to exactly one frozen variant.
pub fn derive_pull_request_status(signals: &PullRequestSignals) -> PullRequest {
    // 1. Terminal — a closed PR dominates every open-state signal.
    if signals.state == PrState::Closed {
        return if signals.merged {
            PullRequest::Merged
        } else {
            PullRequest::Closed
        };
    }

    // 2. Draft precedes the review flow.
    if signals.draft {
        return PullRequest::Draft;
    }

    // 3. A merge conflict is a structural blocker (Unknown mergeability is NOT a conflict).
    if signals.mergeable == Mergeability::Conflicting {
        return PullRequest::Conflict;
    }

    // 4. A failing build is a HARD blocker — it overrides the review decision below.
    if signals.checks == ChecksConclusion::Failing {
        return PullRequest::ChecksFailing;
    }

    // 5. The review decision (mutually exclusive values).
    match signals.review {
        ReviewDecision::ChangesRequested => return PullRequest::ChangesRequested,
        ReviewDecision::Approved => {
            // Fully ready (clean + checks-pass) → Mergeable; approved-but-not-yet → Approved.
            return if signals.mergeable == Mergeability::Clean
                && signals.checks == ChecksConclusion::Success
            {
                PullRequest::Mergeable
            } else {
                PullRequest::Approved
            };
        }
        ReviewDecision::ReviewRequired => return PullRequest::NeedsReview,
        ReviewDecision::None => {}
    }

    // 6. No review decision — surface a SOFT pending build, else the open baseline.
    if signals.checks == ChecksConclusion::Pending {
        return PullRequest::ChecksPending;
    }

    PullRequest::Open
}

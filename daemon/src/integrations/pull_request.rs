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

// ---- GitHub → signals aggregation (edges-006) ------------------------------

/// GitHub's `mergeable_state` string as a daemon-internal enum (the octocrab client maps the raw
/// string into this). Collapsed to the MVP `Mergeability` by `from_github`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHubMergeableState {
    Clean,
    Dirty,
    Blocked,
    Behind,
    Unstable,
    Unknown,
}

/// A single GitHub review's state. Only `Approved` / `ChangesRequested` count toward the aggregate
/// review decision; `Commented` / `Dismissed` / `Pending` carry no decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    Commented,
    Dismissed,
    Pending,
}

/// A single GitHub check-run conclusion, collapsed for the MVP (the octocrab client maps GitHub's
/// `{success, failure, neutral, cancelled, timed_out, action_required, skipped, stale}` + the
/// queued/in_progress status into these): failure/timed_out/action_required → `Failure`; queued/
/// in_progress → `Pending`; success → `Success`; the rest → `Neutral` (ignored in the aggregate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckConclusion {
    Success,
    Failure,
    Pending,
    Neutral,
}

impl PullRequestSignals {
    /// Aggregate GitHub PR facts into `PullRequestSignals` (the input to `derive_pull_request_status`).
    /// Pure — no octocrab/async/network; the gated octocrab client populates the daemon-internal
    /// inputs. Review: any `ChangesRequested` > any `Approved` > `None` (`ReviewRequired` is a branch-
    /// protection signal NOT in `reviews[]` — the octocrab client layers it in via `reviewDecision`).
    /// Checks: any `Failure` > any `Pending` > any `Success` > `None` (Neutral ignored). Mergeable:
    /// `Dirty` → `Conflicting`, `Clean` → `Clean`, else → `Unknown`.
    pub fn from_github(
        state: PrState,
        draft: bool,
        merged: bool,
        mergeable: GitHubMergeableState,
        reviews: &[ReviewState],
        checks: &[CheckConclusion],
    ) -> Self {
        PullRequestSignals {
            state,
            merged,
            draft,
            mergeable: map_mergeable(mergeable),
            review: aggregate_reviews(reviews),
            checks: aggregate_checks(checks),
        }
    }
}

/// Collapse GitHub's `mergeable_state` to the MVP `Mergeability`. `Unknown` (incl. blocked/behind/
/// unstable) is conservatively NOT a conflict. Exhaustive — a new variant forces a reconcile here.
fn map_mergeable(mergeable: GitHubMergeableState) -> Mergeability {
    match mergeable {
        GitHubMergeableState::Dirty => Mergeability::Conflicting,
        GitHubMergeableState::Clean => Mergeability::Clean,
        GitHubMergeableState::Blocked
        | GitHubMergeableState::Behind
        | GitHubMergeableState::Unstable
        | GitHubMergeableState::Unknown => Mergeability::Unknown,
    }
}

/// Aggregate the reviews list → a single `ReviewDecision`: any `ChangesRequested` > any `Approved` >
/// `None`. Commented/Dismissed/Pending carry no decision.
fn aggregate_reviews(reviews: &[ReviewState]) -> ReviewDecision {
    if reviews.contains(&ReviewState::ChangesRequested) {
        ReviewDecision::ChangesRequested
    } else if reviews.contains(&ReviewState::Approved) {
        ReviewDecision::Approved
    } else {
        ReviewDecision::None
    }
}

/// Aggregate the check-runs list → a single `ChecksConclusion`: any `Failure` > any `Pending` > any
/// `Success` > `None` (empty or all-`Neutral`).
fn aggregate_checks(checks: &[CheckConclusion]) -> ChecksConclusion {
    if checks.contains(&CheckConclusion::Failure) {
        ChecksConclusion::Failing
    } else if checks.contains(&CheckConclusion::Pending) {
        ChecksConclusion::Pending
    } else if checks.contains(&CheckConclusion::Success) {
        ChecksConclusion::Success
    } else {
        ChecksConclusion::None
    }
}

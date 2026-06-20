//! P7.1 — the GitHub PR read client (edges-009). The live-GitHub → signals read path, all in-lane:
//! the `github` executor arm (`ExecutorKind::Github`, R1 seam), the `proj_pull_request` projector,
//! and the `PullRequestSynced` event stay GATED/deferred.
//!
//! Two layers:
//!   - `extract_pr_signals` — the **pure, deterministic core** (fixture-driven test-first): octocrab's
//!     typed PR/review/check-run models → `PullRequestSignals`. octocrab 0.53.1 exposes `state`,
//!     `mergeable_state`, and review `state` as TYPED enums (`IssueState`/`MergeableState`/`ReviewState`,
//!     all `#[non_exhaustive]`), so the extraction **stringifies** each into edges-008's
//!     `signals_from_github_response` — keeping edges-008 the SINGLE decode authority (its conservative
//!     floor degrades octocrab's extra variants, e.g. `MergeableState::HasHooks` → Unknown). octocrab's
//!     `CheckRun` carries ONLY `conclusion: Option<String>` (no `status` field): completed-ness is
//!     derived from `conclusion.is_some()` (GitHub sets `conclusion` only once a run completes).
//!   - `GithubReadClient` (trait) + `FakeGithubReadClient` (the seam the gated projector/executor
//!     consume in tests) + `OctocrabGithubReadClient` — the **live HTTP round-trip**, the
//!     non-deterministic edge, covered by the fake (CLAUDE.md: real GitHub calls → fixtures/fakes).
//!     The real client takes an **injected `octocrab::Octocrab`** — auth bootstrap (reuse `gh auth
//!     token`, else Device Flow, §9) is deferred; this client never builds the handle / reads the
//!     keychain. Failures map through edges-003's `classify`, and `GithubReadError` carries the
//!     `IntegrationOutcomeClass` (NOT the collapsed `DeliveryOutcome`) so the gated `SyncFailed`/
//!     `auth_expired` path can branch on `AuthFailed` vs `ClientError` (the §17 forward-constraint).
//!
//! `ReviewRequired` is NOT produced here — the REST `reviews[]` only yields ChangesRequested/Approved/
//! None (edges-006 `aggregate_reviews`); the GraphQL `reviewDecision → ReviewRequired` layering is the
//! next in-lane follow-up (edges-010).

use async_trait::async_trait;
use octocrab::models::checks::CheckRun;
use octocrab::models::pulls::{MergeableState, PullRequest, Review, ReviewState};
use octocrab::models::IssueState;
use serde::Deserialize;

use super::classifier::{classify, IntegrationOutcomeClass};
use super::pull_request::{signals_from_github_response, PullRequestSignals, ReviewDecision};

// ---- the deterministic extraction core (fixture-driven) ----------------------------------------

/// Plumb octocrab's typed PR/review/check-run models into `PullRequestSignals` (pure — no IO/Clock).
/// Stringifies octocrab's typed enums into edges-008's `signals_from_github_response` (the single
/// decode authority); `Option`-None fields degrade to the conservative floor (absent state → `open`,
/// absent mergeable_state → Unknown, absent draft/merged → false); empty review/check lists aggregate
/// to None/None.
pub fn extract_pr_signals(
    pr: &PullRequest,
    reviews: &[Review],
    check_runs: &[CheckRun],
) -> PullRequestSignals {
    let review_states: Vec<&str> = reviews.iter().map(|r| review_state_str(&r.state)).collect();
    let check_inputs: Vec<(&str, Option<&str>)> = check_runs
        .iter()
        .map(|c| {
            // octocrab's CheckRun has no `status` field — `conclusion` is set only once the run
            // completes (GitHub's contract), so `is_some()` IS the completed signal.
            let status = if c.conclusion.is_some() {
                "completed"
            } else {
                "in_progress"
            };
            (status, c.conclusion.as_deref())
        })
        .collect();
    let mut signals = signals_from_github_response(
        issue_state_str(&pr.state),
        pr.draft.unwrap_or(false),
        pr.merged.unwrap_or(false),
        mergeable_state_str(&pr.mergeable_state),
        &review_states,
        &check_inputs,
    );
    // D6 — capture the GET-only diff-stats from the octocrab PR (all `Option<u64>`; `None` when GitHub
    // omitted them, e.g. the list endpoint). NOT status signals → set here, not in the status aggregator.
    signals.additions = pr.additions;
    signals.deletions = pr.deletions;
    signals.changed_files = pr.changed_files;
    signals.commits = pr.commits;
    signals
}

/// octocrab `IssueState` → the canonical GitHub `state` string for edges-008's `parse_pr_state`.
/// None or a future `#[non_exhaustive]` variant → `"open"` (the conservative floor — never fabricate
/// the terminal `closed`).
fn issue_state_str(state: &Option<IssueState>) -> &'static str {
    match state {
        Some(IssueState::Closed) => "closed",
        _ => "open",
    }
}

/// octocrab `MergeableState` → the canonical GitHub `mergeable_state` string. octocrab's extra
/// variants (`Draft`/`HasHooks`) stringify faithfully and edges-008's `parse_mergeable_state` collapses
/// them (+ None / future variants → `""`) to the conservative `Unknown`.
fn mergeable_state_str(state: &Option<MergeableState>) -> &'static str {
    match state {
        Some(MergeableState::Clean) => "clean",
        Some(MergeableState::Dirty) => "dirty",
        Some(MergeableState::Blocked) => "blocked",
        Some(MergeableState::Behind) => "behind",
        Some(MergeableState::Unstable) => "unstable",
        Some(MergeableState::Draft) => "draft",
        Some(MergeableState::HasHooks) => "has_hooks",
        Some(MergeableState::Unknown) => "unknown",
        _ => "",
    }
}

/// octocrab `ReviewState` → the canonical GitHub review `state` wire string (SCREAMING_SNAKE_CASE,
/// GitHub's actual casing) for edges-008's `parse_review_state`, which case-folds before matching.
/// octocrab's extra `Open` (+ None / future variants) → a no-decision value (`parse_review_state`
/// maps the unrecognized form to `Commented`, which carries no review decision).
fn review_state_str(state: &Option<ReviewState>) -> &'static str {
    match state {
        Some(ReviewState::Approved) => "APPROVED",
        Some(ReviewState::ChangesRequested) => "CHANGES_REQUESTED",
        Some(ReviewState::Commented) => "COMMENTED",
        Some(ReviewState::Dismissed) => "DISMISSED",
        Some(ReviewState::Pending) => "PENDING",
        Some(ReviewState::Open) => "OPEN",
        _ => "",
    }
}

// ---- edges-010: GraphQL reviewDecision layering ------------------------------------------------

/// Decode GitHub's GraphQL `reviewDecision` string → `ReviewDecision`. `REVIEW_REQUIRED` is the
/// GraphQL-only branch-protection signal (the REST `reviews[]` aggregate can never yield it). None /
/// null / unrecognized → `None` (the least-salient floor — an unknown value never fabricates a
/// decision). Case-insensitive (edges-008/009 convention).
pub fn parse_review_decision(decision: Option<&str>) -> ReviewDecision {
    match decision.map(|d| d.trim().to_ascii_lowercase()).as_deref() {
        Some("approved") => ReviewDecision::Approved,
        Some("changes_requested") => ReviewDecision::ChangesRequested,
        Some("review_required") => ReviewDecision::ReviewRequired,
        _ => ReviewDecision::None,
    }
}

/// Overlay the GraphQL `reviewDecision` onto the REST-derived signals. GraphQL `reviewDecision` is
/// GitHub's branch-protection-aware aggregate → authoritative WHEN PRESENT (non-`None`); a `None`
/// decision (no protection / not reachable) keeps the REST `reviews[]` aggregate. This is the ONLY
/// path that can set `ReviewRequired` (the REST `aggregate_reviews` is unchanged).
pub fn layer_review_decision(
    mut signals: PullRequestSignals,
    graphql_decision: ReviewDecision,
) -> PullRequestSignals {
    if graphql_decision != ReviewDecision::None {
        signals.review = graphql_decision;
    }
    signals
}

// ---- the read-client trait + fake + live octocrab impl -----------------------------------------

/// A read-failure carrying the §17 `IntegrationOutcomeClass` (NOT the collapsed `DeliveryOutcome`) so
/// the gated `SyncFailed`/`auth_expired` path can branch on `AuthFailed` vs a payload `ClientError`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("github read failed [{class:?}]: {message}")]
pub struct GithubReadError {
    pub class: IntegrationOutcomeClass,
    pub message: String,
}

/// Read a PR's `PullRequestSignals` from GitHub. `async` + dyn-compatible (via `async_trait`) so the
/// gated `github` executor can inject an `Arc<dyn GithubReadClient>` (the codebase's `dyn`-handler idiom).
#[async_trait]
pub trait GithubReadClient: Send + Sync {
    async fn fetch_pr_signals(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestSignals, GithubReadError>;

    /// D7 — fetch a PR's head-vs-base diff as a **unified-diff string** (the
    /// `application/vnd.github.diff` media type; octocrab `pulls().get_diff(pr)`). The `get_pr_diff` read
    /// RPC parses it into `DiffResult`. Read-only; the daemon bounds it with a MANDATORY timeout (§46).
    async fn fetch_pr_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<String, GithubReadError>;
}

/// Test double — returns a canned `Ok(signals)` / `Err(GithubReadError)` for `fetch_pr_signals` and a
/// canned diff for `fetch_pr_diff` (D7; default `Ok("")`, set via [`with_diff`](Self::with_diff)). The
/// seam the gated projector / executor / `get_pr_diff` handler consume in tests (the live HTTP edge is
/// non-deterministic — fake-covered per CLAUDE.md).
pub struct FakeGithubReadClient {
    result: Result<PullRequestSignals, GithubReadError>,
    diff: Result<String, GithubReadError>,
}

impl FakeGithubReadClient {
    pub fn new(result: Result<PullRequestSignals, GithubReadError>) -> Self {
        Self {
            result,
            diff: Ok(String::new()),
        }
    }

    /// Set the canned `fetch_pr_diff` result (D7) — builder over [`new`](Self::new).
    #[must_use]
    pub fn with_diff(mut self, diff: Result<String, GithubReadError>) -> Self {
        self.diff = diff;
        self
    }
}

#[async_trait]
impl GithubReadClient for FakeGithubReadClient {
    async fn fetch_pr_signals(
        &self,
        _owner: &str,
        _repo: &str,
        _pr_number: u64,
    ) -> Result<PullRequestSignals, GithubReadError> {
        self.result.clone()
    }

    async fn fetch_pr_diff(
        &self,
        _owner: &str,
        _repo: &str,
        _pr_number: u64,
    ) -> Result<String, GithubReadError> {
        self.diff.clone()
    }
}

/// The live client over an **injected** `octocrab::Octocrab` (auth bootstrap deferred — never builds
/// the handle/reads the keychain). Fetches the PR + its reviews + its head-ref check-runs, then runs
/// the deterministic `extract_pr_signals`. The HTTP round-trip is the fake-covered non-deterministic
/// edge — no unit test (Step 7.5: only the test module + `FakeGithubReadClient` reference the trait).
pub struct OctocrabGithubReadClient {
    octocrab: octocrab::Octocrab,
}

impl OctocrabGithubReadClient {
    pub fn new(octocrab: octocrab::Octocrab) -> Self {
        Self { octocrab }
    }
}

#[async_trait]
impl GithubReadClient for OctocrabGithubReadClient {
    async fn fetch_pr_signals(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequestSignals, GithubReadError> {
        let pr = self
            .octocrab
            .pulls(owner, repo)
            .get(pr_number)
            .await
            .map_err(|e| to_read_error(&e))?;
        let reviews = self
            .octocrab
            .pulls(owner, repo)
            .list_reviews(pr_number)
            .send()
            .await
            .map_err(|e| to_read_error(&e))?;
        // §7.2: check-runs are keyed on the PR head SHA (the checks SoT). No head, or an empty head
        // SHA (GitHub's "no commits yet" signal — an empty ref would 422/404) → no checks.
        let check_runs = match pr.head.as_ref() {
            Some(head) if !head.sha.is_empty() => {
                self.octocrab
                    .checks(owner, repo)
                    .list_check_runs_for_git_ref(head.sha.clone().into())
                    .send()
                    .await
                    .map_err(|e| to_read_error(&e))?
                    .check_runs
            }
            _ => Vec::new(),
        };
        let signals = extract_pr_signals(&pr, &reviews.items, &check_runs);
        // Best-effort GraphQL enrichment: overlay reviewDecision (the only source of ReviewRequired).
        // Any GraphQL failure degrades to None → the REST aggregate stands (never fails the read).
        let decision = fetch_review_decision(&self.octocrab, owner, repo, pr_number).await;
        Ok(layer_review_decision(signals, decision))
    }

    async fn fetch_pr_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<String, GithubReadError> {
        // the `application/vnd.github.diff` media type → a unified-diff string (octocrab 0.53.1
        // `pulls().get_diff(pr)`). The daemon parses it via `git::parse_unified_diff`.
        self.octocrab
            .pulls(owner, repo)
            .get_diff(pr_number)
            .await
            .map_err(|e| to_read_error(&e))
    }
}

/// Map an `octocrab::Error` → `GithubReadError`, classifying via edges-003's `classify` (§17).
fn to_read_error(err: &octocrab::Error) -> GithubReadError {
    GithubReadError {
        class: classify_octocrab_error(err),
        message: err.to_string(),
    }
}

/// `octocrab::Error` → `IntegrationOutcomeClass` via edges-003's `classify`. The HTTP-status variant
/// carries the GitHub status; hyper/service variants are transport failures. `octocrab::Error` is
/// `#[non_exhaustive]` (not externally constructible → unit-tested via `classify` itself, edges-003);
/// the remaining variants (serde/uri/...) conservatively classify as a transient `ServerError` (retry,
/// never dead-letter). `retry_after` is unreachable from the typed error (no raw headers) → `None`.
pub(crate) fn classify_octocrab_error(err: &octocrab::Error) -> IntegrationOutcomeClass {
    match err {
        octocrab::Error::GitHub { source, .. } => {
            classify(Some(source.status_code.as_u16()), None, false)
        }
        octocrab::Error::Hyper { .. } | octocrab::Error::Service { .. } => {
            classify(None, None, true)
        }
        _ => classify(None, None, false),
    }
}

/// Best-effort GraphQL fetch of a PR's `reviewDecision`. Uses GraphQL VARIABLES (NOT string
/// interpolation) for the caller-supplied coords — injection-safe. ANY failure degrades to
/// `ReviewDecision::None` → the REST aggregate stands (the enrichment never fails the whole read).
/// Note: octocrab's high-level `graphql::<R>()` already folds a logical GraphQL error
/// (`GraphqlResponse::Err`) into `Err(octocrab::Error::Graphql)`, so transport AND logical failures
/// arrive as `Err(_)` — one degrade arm covers both. Not unit-tested (live HTTP — fake-covered, like
/// the REST fetch); the deterministic `layer_review_decision(.., None)` degrade is pinned in tests.
async fn fetch_review_decision(
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> ReviewDecision {
    const QUERY: &str = "query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){pullRequest(number:$number){reviewDecision}}}";
    let payload = serde_json::json!({
        "query": QUERY,
        "variables": { "owner": owner, "name": repo, "number": pr_number },
    });
    match octocrab.graphql::<ReviewDecisionData>(&payload).await {
        Ok(data) => parse_review_decision(
            data.repository
                .and_then(|r| r.pull_request)
                .and_then(|p| p.review_decision)
                .as_deref(),
        ),
        Err(_) => ReviewDecision::None,
    }
}

/// The `data` payload of the reviewDecision query (octocrab's `graphql()` returns the inner `data`).
/// Every node is optional → a missing field degrades to `None` (best-effort).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewDecisionData {
    repository: Option<RepositoryNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryNode {
    pull_request: Option<PullRequestNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestNode {
    review_decision: Option<String>,
}

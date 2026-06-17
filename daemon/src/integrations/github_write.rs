//! P7.1 (edges-023) — the GitHub PR **WRITE** client seam (the create-PR network MUTATION).
//!
//! The write mirror of `github.rs`'s read client:
//!   - `GithubWriteClient` (trait) + `FakeGithubWriteClient` (the seam the `GithubExecutor` consumes in
//!     tests) + `OctocrabGithubWriteClient` — the **live HTTP round-trip**, the non-deterministic edge,
//!     covered by the fake (CLAUDE.md: real GitHub calls → fixtures/fakes). The real client takes an
//!     **injected `octocrab::Octocrab`** — auth bootstrap (reuse `gh auth token`, else Device Flow, §9)
//!     is deferred; this client never builds the handle / reads the keychain.
//!   - `CreatePrArgs` (the operational params the executor reads from `req.inputs`) + the DOMAIN result
//!     `CreatedPr` (NOT an octocrab `PullRequest`, which is `#[non_exhaustive]`/unbuildable in tests —
//!     the `FakeGithubReadClient`/`CreatedPr` precedent) + `GithubWriteError` (carries the §17
//!     `IntegrationOutcomeClass`, mirroring `GithubReadError`, so the executor branches terminal-non-auth
//!     vs `AuthFailed` vs transient). Failures map through the SAME `classify_octocrab_error` (§17).

use async_trait::async_trait;

use nexusops_shared::status::ReviewState;
use nexusops_shared::time::Timestamp;

use super::classifier::IntegrationOutcomeClass;
use super::github::{classify_octocrab_error, extract_pr_signals};
use super::pull_request::PullRequestSignals;

/// The operational parameters for a create-PR call — read from `req.inputs` by the `GithubExecutor`
/// (the resource_ref stays the audit/policy IDENTITY; the inputs carry the operation — the `GitExecutor`
/// precedent). `body` is optional; `draft` distinguishes `github.create_pr_draft` from `github.create_pr`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatePrArgs {
    pub owner: String,
    pub repo: String,
    pub head: String,
    pub base: String,
    pub title: String,
    pub body: Option<String>,
    pub draft: bool,
}

/// The DOMAIN result of a created PR — NOT an octocrab `PullRequest` (which is `#[non_exhaustive]` and
/// unbuildable in tests). `signals` feed the §5.1 `derive_pull_request_status`; the live client extracts
/// them from the create response (a just-created PR has no reviews/checks yet → `Open`/`Draft`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedPr {
    pub pr_number: u64,
    pub signals: PullRequestSignals,
    pub branch: String,
    pub base: String,
}

/// (D5b-2) The operational params for a `github.sync_reviews` call — owner/repo/pr_number read from
/// `req.inputs` by the `GithubExecutor` (the Repo resource_ref stays the audit/policy IDENTITY).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncReviewsArgs {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
}

/// (D5b-2) The DOMAIN result of ONE PR review — NOT an octocrab `Review` (`#[non_exhaustive]`/unbuildable
/// in tests, the `CreatedPr` precedent). The executor maps each → a `ReviewSynced` event (adding the
/// PR's `pr_number` from the inputs + `review_synced_at` from the daemon Clock). `state` is already the
/// frozen shared [`ReviewState`] (mapped from octocrab via [`map_review_state`] at the client boundary).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewData {
    pub review_id: u64,
    pub reviewer: String,
    pub state: ReviewState,
    pub submitted_at: Option<Timestamp>,
    pub body: Option<String>,
}

/// (D5b-2) octocrab `ReviewState` → the frozen shared [`ReviewState`]. A PURE deterministic mapping (NOT
/// network I/O — the live octocrab CALL is the fake-covered edge; this is unit-tested): the 5 GitHub
/// states map 1:1; octocrab's extra `Open` + `None` + any future `#[non_exhaustive]` variant floor to
/// `Commented` (a no-decision review) — NEVER a panic or a fabricated verdict (the `review_state_str`
/// conservative-floor precedent applied to the shared enum).
pub(crate) fn map_review_state(
    state: &Option<octocrab::models::pulls::ReviewState>,
) -> ReviewState {
    use octocrab::models::pulls::ReviewState as O;
    match state {
        Some(O::Approved) => ReviewState::Approved,
        Some(O::ChangesRequested) => ReviewState::ChangesRequested,
        Some(O::Commented) => ReviewState::Commented,
        Some(O::Dismissed) => ReviewState::Dismissed,
        Some(O::Pending) => ReviewState::Pending,
        // octocrab's `Open` + None + a future variant → the conservative no-decision floor.
        _ => ReviewState::Commented,
    }
}

/// A write-failure carrying the §17 `IntegrationOutcomeClass` (mirrors `GithubReadError`) so the executor
/// can branch terminal-non-auth (`ClientError`/`NotFound` → `GithubSyncFailed`) vs `AuthFailed` (the
/// deferred `auth_expired` path) vs transient (retry/queue). The `message` is for daemon LOGS ONLY —
/// it is NEVER surfaced into a persisted event (§15 — the event's `reason` is a structural class-name).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("github write failed [{class:?}]: {message}")]
pub struct GithubWriteError {
    pub class: IntegrationOutcomeClass,
    pub message: String,
}

/// Create a PR on GitHub. `async` + dyn-compatible (`async_trait`) so the SYNC `GithubExecutor` injects a
/// `Box<dyn GithubWriteClient>` and drives it via a CAPTURED `tokio::runtime::Handle` (`block_on`, the 3a
/// mechanism — the trait stays async; the executor bridges sync↔async on the write-actor std::thread).
#[async_trait]
pub trait GithubWriteClient: Send + Sync {
    async fn create_pull_request(&self, args: &CreatePrArgs)
        -> Result<CreatedPr, GithubWriteError>;

    /// (D5b-2) List a PR's reviews → normalized [`ReviewData`] (the executor maps each → `ReviewSynced`).
    /// A READ on this client (the executor already holds it; the live HTTP round-trip is the fake-covered
    /// edge). Additive — no change to `create_pull_request`.
    async fn list_reviews(
        &self,
        args: &SyncReviewsArgs,
    ) -> Result<Vec<ReviewData>, GithubWriteError>;
}

/// The live client over an **injected** `octocrab::Octocrab` (auth bootstrap deferred — never builds the
/// handle / reads the keychain). Creates the PR, then derives signals from the create response. The HTTP
/// round-trip is the fake-covered non-deterministic edge — no unit test (Step 7.5: only the test module +
/// `FakeGithubWriteClient` reference the trait; the real client is reachable from `main.rs`).
pub struct OctocrabGithubWriteClient {
    octocrab: octocrab::Octocrab,
}

impl OctocrabGithubWriteClient {
    pub fn new(octocrab: octocrab::Octocrab) -> Self {
        Self { octocrab }
    }
}

#[async_trait]
impl GithubWriteClient for OctocrabGithubWriteClient {
    async fn create_pull_request(
        &self,
        args: &CreatePrArgs,
    ) -> Result<CreatedPr, GithubWriteError> {
        let handler = self.octocrab.pulls(&args.owner, &args.repo);
        let mut builder = handler.create(&args.title, &args.head, &args.base);
        if let Some(body) = &args.body {
            builder = builder.body(body.as_str());
        }
        builder = builder.draft(args.draft);
        let pr = builder.send().await.map_err(|e| GithubWriteError {
            class: classify_octocrab_error(&e),
            message: e.to_string(),
        })?;
        // octocrab models `number` as `Option<u64>` for response resilience; a successful create
        // response ALWAYS carries the PR number. Treat a numberless 201 (a malformed GitHub response) as
        // a TRANSIENT failure (→ retry/queue via `classify_failure`) rather than silently persisting a
        // bogus `pr_number: 0` into the event log — fail-safe over fail-silent. NOT unit-tested (live edge).
        let pr_number = pr.number.ok_or_else(|| GithubWriteError {
            class: IntegrationOutcomeClass::ServerError,
            message: "github create response missing PR number".to_string(),
        })?;
        // a just-created PR has no reviews/checks yet → empty aggregates → Open (or Draft).
        let signals = extract_pr_signals(&pr, &[], &[]);
        Ok(CreatedPr {
            pr_number,
            signals,
            branch: args.head.clone(),
            base: args.base.clone(),
        })
    }

    async fn list_reviews(
        &self,
        args: &SyncReviewsArgs,
    ) -> Result<Vec<ReviewData>, GithubWriteError> {
        // first page (per_page=100) — pagination of >100 reviews on one PR is a rare follow-on. The live
        // HTTP round-trip is the fake-covered edge (no unit test); the per-review NORMALIZATION (the state
        // map) is the unit-tested pure `map_review_state`.
        let page = self
            .octocrab
            .pulls(&args.owner, &args.repo)
            .list_reviews(args.pr_number)
            .per_page(100)
            .send()
            .await
            .map_err(|e| GithubWriteError {
                class: classify_octocrab_error(&e),
                message: e.to_string(),
            })?;
        let reviews = page
            .items
            .into_iter()
            .map(|r| ReviewData {
                review_id: r.id.0,
                // a review without an attributed user (rare) → an empty reviewer string (display-tolerant).
                reviewer: r.user.map(|u| u.login).unwrap_or_default(),
                state: map_review_state(&r.state),
                // GitHub's ISO timestamp → the daemon Timestamp; an unparseable value → None (defensive).
                submitted_at: r
                    .submitted_at
                    .and_then(|dt| Timestamp::parse(&dt.to_rfc3339()).ok()),
                body: r.body,
            })
            .collect();
        Ok(reviews)
    }
}

/// Test double — records each call's args + returns a canned `Ok`/`Err`, or HANGS (the timeout test).
/// The seam the `GithubExecutor` tests consume (the live HTTP edge is non-deterministic — fake-covered
/// per CLAUDE.md). Gated behind `test-support` (the `FakeGitCli` precedent; the daemon dev-dep
/// self-enables it for test/bench targets — LESSON 21).
///
/// **Constructed for exactly ONE operation** (create-PR via `::ok`/`::err`/`::hanging`, OR list-reviews via
/// `::with_reviews`/`::reviews_err`/`::reviews_hanging`): the unused op's mode stays `None` and its impl
/// `expect`-panics if invoked — a deliberate fail-LOUD so a mis-targeted dispatch is caught in tests.
#[cfg(feature = "test-support")]
pub struct FakeGithubWriteClient {
    calls: std::sync::Arc<std::sync::Mutex<Vec<CreatePrArgs>>>,
    mode: Option<FakeWriteMode>,
    // D5b-2 — the list_reviews half. A fake is constructed for EITHER op (create OR reviews); the unused
    // op's mode stays `None` (its impl `expect`s configuration, so a mis-targeted call fails loud in tests).
    review_calls: std::sync::Arc<std::sync::Mutex<Vec<SyncReviewsArgs>>>,
    reviews_mode: Option<FakeReviewsMode>,
}

#[cfg(feature = "test-support")]
enum FakeWriteMode {
    Ok(CreatedPr),
    Err(GithubWriteError),
    /// a future that never resolves — exercises the executor's `block_on` timeout.
    Hang,
}

#[cfg(feature = "test-support")]
enum FakeReviewsMode {
    Ok(Vec<ReviewData>),
    Err(GithubWriteError),
    /// a future that never resolves — exercises the executor's `block_on` timeout.
    Hang,
}

#[cfg(feature = "test-support")]
impl FakeGithubWriteClient {
    /// records the call; returns the canned `CreatedPr`.
    pub fn ok(created: CreatedPr) -> Self {
        Self {
            calls: Default::default(),
            mode: Some(FakeWriteMode::Ok(created)),
            review_calls: Default::default(),
            reviews_mode: None,
        }
    }
    /// records the call; returns the canned `GithubWriteError`.
    pub fn err(error: GithubWriteError) -> Self {
        Self {
            calls: Default::default(),
            mode: Some(FakeWriteMode::Err(error)),
            review_calls: Default::default(),
            reviews_mode: None,
        }
    }
    /// records the call; then NEVER resolves (the write-actor timeout bound is the only escape).
    pub fn hanging() -> Self {
        Self {
            calls: Default::default(),
            mode: Some(FakeWriteMode::Hang),
            review_calls: Default::default(),
            reviews_mode: None,
        }
    }
    /// (D5b-2) records the call; returns the canned `Vec<ReviewData>`.
    pub fn with_reviews(reviews: Vec<ReviewData>) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: Some(FakeReviewsMode::Ok(reviews)),
        }
    }
    /// (D5b-2) records the call; returns the canned `GithubWriteError` from `list_reviews`.
    pub fn reviews_err(error: GithubWriteError) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: Some(FakeReviewsMode::Err(error)),
        }
    }
    /// (D5b-2) records the call; then NEVER resolves (the `list_reviews` timeout bound).
    pub fn reviews_hanging() -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: Some(FakeReviewsMode::Hang),
        }
    }
    /// a handle to the recorded create-PR call args — clone it BEFORE the client is boxed.
    pub fn calls(&self) -> std::sync::Arc<std::sync::Mutex<Vec<CreatePrArgs>>> {
        self.calls.clone()
    }
    /// (D5b-2) a handle to the recorded list_reviews call args.
    pub fn review_calls(&self) -> std::sync::Arc<std::sync::Mutex<Vec<SyncReviewsArgs>>> {
        self.review_calls.clone()
    }
}

#[cfg(feature = "test-support")]
#[async_trait]
impl GithubWriteClient for FakeGithubWriteClient {
    async fn create_pull_request(
        &self,
        args: &CreatePrArgs,
    ) -> Result<CreatedPr, GithubWriteError> {
        // `.unwrap()` (daemon no-bare-unwrap convention): test-only double; the Mutex is uncontended
        // (single-threaded test use) and only poisons if a test already panicked while holding it — no
        // new failure mode (the FakeGitCli Mutex-lock precedent).
        self.calls.lock().unwrap().push(args.clone());
        match self
            .mode
            .as_ref()
            .expect("FakeGithubWriteClient create mode not configured (use ::ok/::err/::hanging)")
        {
            FakeWriteMode::Ok(created) => Ok(created.clone()),
            FakeWriteMode::Err(e) => Err(e.clone()),
            // never resolves → the executor's `tokio::time::timeout` fires (the write-actor bound).
            FakeWriteMode::Hang => std::future::pending().await,
        }
    }

    async fn list_reviews(
        &self,
        args: &SyncReviewsArgs,
    ) -> Result<Vec<ReviewData>, GithubWriteError> {
        self.review_calls.lock().unwrap().push(args.clone());
        match self.reviews_mode.as_ref().expect(
            "FakeGithubWriteClient reviews mode not configured (use ::with_reviews/::reviews_err/::reviews_hanging)",
        ) {
            FakeReviewsMode::Ok(reviews) => Ok(reviews.clone()),
            FakeReviewsMode::Err(e) => Err(e.clone()),
            FakeReviewsMode::Hang => std::future::pending().await,
        }
    }
}

#[cfg(test)]
mod review_state_mapping_tests {
    use super::map_review_state;
    use nexusops_shared::status::ReviewState;
    use octocrab::models::pulls::ReviewState as O;

    #[test]
    fn maps_known_states_one_to_one_and_floors_unknown() {
        // D5b-2 — the 5 GitHub review states map 1:1 (deterministic, NOT network I/O — the live octocrab
        // CALL is the fake-covered edge; this pure mapping is unit-tested).
        assert_eq!(map_review_state(&Some(O::Approved)), ReviewState::Approved);
        assert_eq!(
            map_review_state(&Some(O::ChangesRequested)),
            ReviewState::ChangesRequested
        );
        assert_eq!(map_review_state(&Some(O::Commented)), ReviewState::Commented);
        assert_eq!(map_review_state(&Some(O::Dismissed)), ReviewState::Dismissed);
        assert_eq!(map_review_state(&Some(O::Pending)), ReviewState::Pending);
        // the conservative floor — octocrab's extra `Open` + None (+ a future non_exhaustive variant) →
        // Commented (a no-decision review), NEVER a panic / a fabricated verdict (the review_state_str
        // precedent applied to the shared enum).
        assert_eq!(map_review_state(&Some(O::Open)), ReviewState::Commented);
        assert_eq!(map_review_state(&None), ReviewState::Commented);
    }
}

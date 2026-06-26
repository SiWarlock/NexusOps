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

use super::auth::GithubAuthResolver;
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

/// (D9) The operational params for a `github.merge_pr` call — owner/repo/pr_number/sha read from
/// `req.inputs` by the `GithubExecutor` (the Repo resource_ref stays the audit/policy IDENTITY). `sha` is
/// the head SHA the merge is PINNED to (the anti-race guard — octocrab 409s if the head moved; the human
/// approved THIS head). `merge_method` is the already-mapped octocrab enum (validated fail-closed by
/// [`map_merge_method`] at the executor BEFORE the call — the approved+audited method executes exactly).
/// No `Eq` derive — octocrab's `MergeMethod` is `PartialEq`-only + `#[non_exhaustive]`.
#[derive(Clone, Debug, PartialEq)]
pub struct MergePrArgs {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
    pub sha: String,
    pub merge_method: octocrab::params::pulls::MergeMethod,
}

/// (D9) The DOMAIN result of a merged PR — NOT an octocrab `Merge` (the `CreatedPr` precedent: keep the
/// daemon types decoupled from octocrab's response models). `merge_commit_sha` feeds the
/// `PullRequestMerged` event; `None` when the API response omitted it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedPr {
    pub merge_commit_sha: Option<String>,
}

/// (D9) map a `merge_method` string → octocrab's `MergeMethod`. A PURE deterministic mapping (NOT network
/// I/O — the live octocrab CALL is the fake-covered edge; this is unit-tested). `merge`/`squash`/`rebase`
/// map 1:1 (case-insensitive + trimmed for API resilience, the `parse_*` precedent); any unknown value →
/// a **fail-closed `Err`** — NEVER a silent default (the approved+audited method must execute EXACTLY; a
/// silent `Merge` default would diverge the on-disk result from the audited Action, F2 audit-integrity).
/// §15: the `Err` is a STRUCTURAL reason, NEVER echoes the raw (possibly attacker-chosen) input value.
pub fn map_merge_method(method: &str) -> Result<octocrab::params::pulls::MergeMethod, String> {
    use octocrab::params::pulls::MergeMethod as M;
    match method.trim().to_ascii_lowercase().as_str() {
        "merge" => Ok(M::Merge),
        "squash" => Ok(M::Squash),
        "rebase" => Ok(M::Rebase),
        _ => Err("unknown merge_method (expected merge|squash|rebase)".to_string()),
    }
}

/// (D10) The operational params for a `github.submit_review` call — read from `req.inputs` by the
/// `GithubExecutor` (the Repo resource_ref stays the audit/policy IDENTITY). `commit_id` is the reviewed
/// head SHA the verdict is PINNED to (audit-integrity/anti-race; the merge_pr `sha` precedent). `event` is
/// the already-mapped octocrab review verb (validated fail-closed by [`map_review_event`] at the executor
/// BEFORE the call). `body` is the resolved review text — `""` for an `approve` with no body, the
/// validated non-empty text for `request_changes`/`comment` (GitHub 422s on an empty body there). No `Eq`
/// derive — octocrab's `ReviewAction` is `PartialEq`-only + `#[non_exhaustive]`.
#[derive(Clone, Debug, PartialEq)]
pub struct SubmitReviewArgs {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
    pub commit_id: String,
    pub event: octocrab::models::pulls::ReviewAction,
    pub body: String,
}

/// (D10) The DOMAIN result of a submitted review — NOT an octocrab `Review` (`#[non_exhaustive]`; the
/// `ReviewData`/`MergedPr` precedent). Mirrors [`ReviewData`] (+ `commit_id`): the executor maps it → a
/// `ReviewSubmitted` event. `state` is already the frozen shared [`ReviewState`] (mapped via
/// [`map_review_state`] at the client boundary).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedReview {
    pub review_id: u64,
    pub reviewer: String,
    pub state: ReviewState,
    pub submitted_at: Option<Timestamp>,
    pub body: Option<String>,
    pub commit_id: Option<String>,
}

/// (D10) map a review `event` string → octocrab's `ReviewAction`. PURE + deterministic (NOT network I/O —
/// the live octocrab CALL is the fake-covered edge; this is unit-tested). `approve`/`request_changes`/
/// `comment` map 1:1 (case-insensitive + trimmed, the `map_merge_method`/`parse_*` precedent); any unknown
/// value → a **fail-closed `Err`** — NEVER a silent default (the approved+audited verdict must execute
/// EXACTLY — a silent default would diverge the posted review from the audited Action). §15: the `Err` is a
/// STRUCTURAL reason, NEVER echoes the raw (possibly attacker-chosen) input value.
pub fn map_review_event(event: &str) -> Result<octocrab::models::pulls::ReviewAction, String> {
    use octocrab::models::pulls::ReviewAction as A;
    match event.trim().to_ascii_lowercase().as_str() {
        "approve" => Ok(A::Approve),
        "request_changes" => Ok(A::RequestChanges),
        "comment" => Ok(A::Comment),
        _ => Err("unknown review event (expected approve|request_changes|comment)".to_string()),
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

    /// (D9) Merge a PR on GitHub — the cat-1 WRITE. SHA-pinned to `args.sha` (octocrab 409s on a head
    /// move) with the explicit `args.merge_method`. The live HTTP round-trip is the fake-covered edge.
    async fn merge_pull_request(&self, args: &MergePrArgs) -> Result<MergedPr, GithubWriteError>;

    /// (D10) Submit a PR review verdict on GitHub — the cat-1 WRITE. SHA-pinned to `args.commit_id` with
    /// the explicit `args.event`. The live HTTP round-trip is the fake-covered edge.
    async fn submit_review(
        &self,
        args: &SubmitReviewArgs,
    ) -> Result<SubmittedReview, GithubWriteError>;
}

/// The live client over a [`GithubAuthResolver`] (P4.7/083 — the C3 live-auth wiring). Each call builds a
/// PER-OWNER `octocrab` handle: AUTHED with the repo owner's keychain token when the connection's
/// live-writes toggle is ON + a token is present; else UNAUTHENTICATED (fail-closed — the existing
/// 401→AuthFailed path). The per-`owner` token selection is the confused-deputy-safe axis (a write to
/// repo X uses X's owner's token, never another account's). The HTTP round-trip is the fake-covered
/// non-deterministic edge — no unit test (only the deterministic `resolve_token_for` decision is pinned;
/// `FakeGithubWriteClient` covers the executor path).
pub struct OctocrabGithubWriteClient {
    auth: GithubAuthResolver,
}

impl OctocrabGithubWriteClient {
    pub fn new(auth: GithubAuthResolver) -> Self {
        Self { auth }
    }
}

#[async_trait]
impl GithubWriteClient for OctocrabGithubWriteClient {
    async fn create_pull_request(
        &self,
        args: &CreatePrArgs,
    ) -> Result<CreatedPr, GithubWriteError> {
        // per-owner authed-or-unauth handle (the C3 live-auth wiring; toggle-gated, fail-closed unauth).
        let octocrab = self.auth.octocrab_for(&args.owner);
        let handler = octocrab.pulls(&args.owner, &args.repo);
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
        let octocrab = self.auth.octocrab_for(&args.owner);
        let page = octocrab
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

    async fn merge_pull_request(&self, args: &MergePrArgs) -> Result<MergedPr, GithubWriteError> {
        let octocrab = self.auth.octocrab_for(&args.owner);
        let merge = octocrab
            .pulls(&args.owner, &args.repo)
            .merge(args.pr_number)
            // SHA-pin: GitHub refuses (409) if the head moved off this SHA — the anti-race guard so the
            // merge applies to EXACTLY the head the human approved (audit-integrity, F1).
            .sha(args.sha.clone())
            .method(args.merge_method)
            .send()
            .await
            .map_err(|e| GithubWriteError {
                class: classify_octocrab_error(&e),
                message: e.to_string(),
            })?;
        // a 200 with `merged: false` (rare — e.g. a server-side merge precondition not met) must NEVER
        // fabricate a `PullRequestMerged` event: treat it as a TRANSIENT failure (→ retry/queue via
        // classify_failure), the create_pr numberless-201 fail-safe-over-fail-silent precedent. NOT
        // unit-tested (live edge).
        if !merge.merged {
            return Err(GithubWriteError {
                class: IntegrationOutcomeClass::ServerError,
                message: "github merge response reported not merged".to_string(),
            });
        }
        Ok(MergedPr {
            merge_commit_sha: merge.sha,
        })
    }

    async fn submit_review(
        &self,
        args: &SubmitReviewArgs,
    ) -> Result<SubmittedReview, GithubWriteError> {
        // octocrab 0.53 DEPRECATED the high-level create path (`pull_number().reviews().create_review()`):
        // only `pr_review_actions(pr, review_id)` survives — for actions on an EXISTING review, no CREATE
        // path. So drive the create directly via the typed lower-level POST — BYTE-EQUIVALENT to what
        // `create_review` does internally (octocrab `pr_reviews.rs`): POST /repos/{o}/{r}/pulls/{n}/reviews
        // with {commit_id, body, event, comments}. SAME URL interpolation as every other github write
        // (`pulls(&owner,&repo)`) → no new injection surface; pr_number is a u64. `event` serializes via
        // its `ReviewAction` Serialize (SCREAMING_SNAKE: APPROVE/REQUEST_CHANGES/COMMENT). comments=[] —
        // the per-hunk inline surface is the deferred §4.7 follow-on (D10 Step-2.5 Q1). commit_id PINS the
        // verdict to the reviewed head (audit-integrity).
        let route = format!(
            "/repos/{}/{}/pulls/{}/reviews",
            args.owner, args.repo, args.pr_number
        );
        let request_body = serde_json::json!({
            "commit_id": args.commit_id,
            "body": args.body,
            "event": args.event,
            "comments": [],
        });
        let octocrab = self.auth.octocrab_for(&args.owner);
        let review: octocrab::models::pulls::Review = octocrab
            .post(route, Some(&request_body))
            .await
            .map_err(|e| GithubWriteError {
                class: classify_octocrab_error(&e),
                message: e.to_string(),
            })?;
        Ok(SubmittedReview {
            // the create-response's review id is the proj_review PK + the ReviewSubmitted identity.
            review_id: review.id.0,
            // the reviewer is the authenticated submitter (response `user.login`); empty if unattributed.
            reviewer: review.user.map(|u| u.login).unwrap_or_default(),
            // octocrab's response ReviewState → the frozen shared enum (the list_reviews precedent).
            state: map_review_state(&review.state),
            // GitHub's ISO timestamp → the daemon Timestamp; an unparseable/absent value → None (defensive).
            submitted_at: review
                .submitted_at
                .and_then(|dt| Timestamp::parse(&dt.to_rfc3339()).ok()),
            body: review.body,
            commit_id: review.commit_id,
        })
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
    // D9 — the merge_pull_request half (the cat-1 WRITE). Same one-op-per-fake discipline: the unused op's
    // mode stays `None` (its impl `expect`s configuration → a mis-targeted call fails LOUD in tests).
    merge_calls: std::sync::Arc<std::sync::Mutex<Vec<MergePrArgs>>>,
    merge_mode: Option<FakeMergeMode>,
    // D10 — the submit_review half (the cat-1 WRITE). Same one-op-per-fake discipline.
    submit_calls: std::sync::Arc<std::sync::Mutex<Vec<SubmitReviewArgs>>>,
    submit_mode: Option<FakeSubmitMode>,
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
enum FakeMergeMode {
    Ok(MergedPr),
    Err(GithubWriteError),
    /// a future that never resolves — exercises the executor's `block_on` timeout.
    Hang,
}

#[cfg(feature = "test-support")]
enum FakeSubmitMode {
    Ok(SubmittedReview),
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
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// records the call; returns the canned `GithubWriteError`.
    pub fn err(error: GithubWriteError) -> Self {
        Self {
            calls: Default::default(),
            mode: Some(FakeWriteMode::Err(error)),
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// records the call; then NEVER resolves (the write-actor timeout bound is the only escape).
    pub fn hanging() -> Self {
        Self {
            calls: Default::default(),
            mode: Some(FakeWriteMode::Hang),
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// (D5b-2) records the call; returns the canned `Vec<ReviewData>`.
    pub fn with_reviews(reviews: Vec<ReviewData>) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: Some(FakeReviewsMode::Ok(reviews)),
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// (D5b-2) records the call; returns the canned `GithubWriteError` from `list_reviews`.
    pub fn reviews_err(error: GithubWriteError) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: Some(FakeReviewsMode::Err(error)),
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// (D5b-2) records the call; then NEVER resolves (the `list_reviews` timeout bound).
    pub fn reviews_hanging() -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: Some(FakeReviewsMode::Hang),
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// (D9) records the call; returns the canned `MergedPr` (a successful merge).
    pub fn merged(merged: MergedPr) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: Some(FakeMergeMode::Ok(merged)),
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// (D9) records the call; returns the canned `GithubWriteError` from `merge_pull_request`.
    pub fn merge_err(error: GithubWriteError) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: Some(FakeMergeMode::Err(error)),
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// (D9) records the call; then NEVER resolves (the merge `block_on` timeout bound).
    pub fn merge_hanging() -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: Some(FakeMergeMode::Hang),
            submit_calls: Default::default(),
            submit_mode: None,
        }
    }
    /// (D10) records the call; returns the canned `SubmittedReview` (a successful submit).
    pub fn submitted(submitted: SubmittedReview) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: Some(FakeSubmitMode::Ok(submitted)),
        }
    }
    /// (D10) records the call; returns the canned `GithubWriteError` from `submit_review`.
    pub fn submit_err(error: GithubWriteError) -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: Some(FakeSubmitMode::Err(error)),
        }
    }
    /// (D10) records the call; then NEVER resolves (the submit `block_on` timeout bound).
    pub fn submit_hanging() -> Self {
        Self {
            calls: Default::default(),
            mode: None,
            review_calls: Default::default(),
            reviews_mode: None,
            merge_calls: Default::default(),
            merge_mode: None,
            submit_calls: Default::default(),
            submit_mode: Some(FakeSubmitMode::Hang),
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
    /// (D9) a handle to the recorded merge_pull_request call args — clone it BEFORE the client is boxed.
    pub fn merge_calls(&self) -> std::sync::Arc<std::sync::Mutex<Vec<MergePrArgs>>> {
        self.merge_calls.clone()
    }
    /// (D10) a handle to the recorded submit_review call args — clone it BEFORE the client is boxed.
    pub fn submit_calls(&self) -> std::sync::Arc<std::sync::Mutex<Vec<SubmitReviewArgs>>> {
        self.submit_calls.clone()
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

    async fn merge_pull_request(&self, args: &MergePrArgs) -> Result<MergedPr, GithubWriteError> {
        self.merge_calls.lock().unwrap().push(args.clone());
        match self.merge_mode.as_ref().expect(
            "FakeGithubWriteClient merge mode not configured (use ::merged/::merge_err/::merge_hanging)",
        ) {
            FakeMergeMode::Ok(merged) => Ok(merged.clone()),
            FakeMergeMode::Err(e) => Err(e.clone()),
            FakeMergeMode::Hang => std::future::pending().await,
        }
    }

    async fn submit_review(
        &self,
        args: &SubmitReviewArgs,
    ) -> Result<SubmittedReview, GithubWriteError> {
        self.submit_calls.lock().unwrap().push(args.clone());
        match self.submit_mode.as_ref().expect(
            "FakeGithubWriteClient submit mode not configured (use ::submitted/::submit_err/::submit_hanging)",
        ) {
            FakeSubmitMode::Ok(submitted) => Ok(submitted.clone()),
            FakeSubmitMode::Err(e) => Err(e.clone()),
            FakeSubmitMode::Hang => std::future::pending().await,
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
        assert_eq!(
            map_review_state(&Some(O::Commented)),
            ReviewState::Commented
        );
        assert_eq!(
            map_review_state(&Some(O::Dismissed)),
            ReviewState::Dismissed
        );
        assert_eq!(map_review_state(&Some(O::Pending)), ReviewState::Pending);
        // the conservative floor — octocrab's extra `Open` + None (+ a future non_exhaustive variant) →
        // Commented (a no-decision review), NEVER a panic / a fabricated verdict (the review_state_str
        // precedent applied to the shared enum).
        assert_eq!(map_review_state(&Some(O::Open)), ReviewState::Commented);
        assert_eq!(map_review_state(&None), ReviewState::Commented);
    }
}

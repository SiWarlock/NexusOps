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
}

/// Test double — records every `create_pull_request` call's args + returns a canned `Ok`/`Err`, or HANGS
/// (the timeout test). The seam the `GithubExecutor` tests consume (the live HTTP edge is
/// non-deterministic — fake-covered per CLAUDE.md). Gated behind `test-support` (the `FakeGitCli`
/// precedent; the daemon dev-dep self-enables it for test/bench targets — LESSON 21).
#[cfg(feature = "test-support")]
pub struct FakeGithubWriteClient {
    calls: std::sync::Arc<std::sync::Mutex<Vec<CreatePrArgs>>>,
    mode: FakeWriteMode,
}

#[cfg(feature = "test-support")]
enum FakeWriteMode {
    Ok(CreatedPr),
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
            mode: FakeWriteMode::Ok(created),
        }
    }
    /// records the call; returns the canned `GithubWriteError`.
    pub fn err(error: GithubWriteError) -> Self {
        Self {
            calls: Default::default(),
            mode: FakeWriteMode::Err(error),
        }
    }
    /// records the call; then NEVER resolves (the write-actor timeout bound is the only escape).
    pub fn hanging() -> Self {
        Self {
            calls: Default::default(),
            mode: FakeWriteMode::Hang,
        }
    }
    /// a handle to the recorded call args — clone it BEFORE the client is boxed.
    pub fn calls(&self) -> std::sync::Arc<std::sync::Mutex<Vec<CreatePrArgs>>> {
        self.calls.clone()
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
        match &self.mode {
            FakeWriteMode::Ok(created) => Ok(created.clone()),
            FakeWriteMode::Err(e) => Err(e.clone()),
            // never resolves → the executor's `tokio::time::timeout` fires (the write-actor bound).
            FakeWriteMode::Hang => std::future::pending().await,
        }
    }
}

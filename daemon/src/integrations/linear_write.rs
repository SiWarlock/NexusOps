//! P7.1 (edges-024) — the Linear GraphQL **WRITE** client seam (the mutation network path).
//!
//! The write mirror of `linear.rs`'s read client (the `github_write.rs` ↔ `github.rs` analog):
//!   - `LinearWriteClient` (trait) + `FakeLinearWriteClient` (the seam `LinearExecutor` consumes in
//!     tests) + `LinearGraphqlWriteClient` — the **live HTTP round-trip**, the non-deterministic edge,
//!     fake-covered (CLAUDE.md). The real client takes an **injected `reqwest::Client` + endpoint +
//!     api_key**; auth bootstrap (OAuth/PKCE 24h refresh, §9) is deferred — it never builds the key /
//!     reads the keychain. §15 rule #5: the api_key reaches ONLY the `Authorization` header.
//!   - `LinkIssueArgs`/`CreateIssueArgs` (the operands the executor reads from `req.inputs`) + the
//!     `LinearWriteCall` recording enum (the fake's assertion surface) + `LinearWriteError{class, message}`
//!     (mirrors `LinearReadError` — the §17 class so the executor branches terminal-non-auth vs auth vs
//!     transient). Both mutations return `()` on success — Linear emits NO domain event (Q1); the success
//!     is recorded by `ActionSucceeded`. Response classification reuses `classify_linear_write_response`.

use async_trait::async_trait;

use super::classifier::{classify, IntegrationOutcomeClass};
use super::linear::classify_linear_write_response;

/// `link_issue` operands (read from `req.inputs` by `LinearExecutor`): the Linear `issue_id` + the
/// `target` (the URL/ref being linked). The resource_ref stays the audit/policy IDENTITY (the
/// `GitExecutor`/`GithubExecutor` operational-params precedent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkIssueArgs {
    pub issue_id: String,
    pub target: String,
}

/// `create_issue` operands: `team_id` + `title` + optional `description`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateIssueArgs {
    pub team_id: String,
    pub title: String,
    pub description: Option<String>,
}

/// A recorded write-client call — the fake's assertion surface (two methods → one enum).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinearWriteCall {
    LinkIssue(LinkIssueArgs),
    CreateIssue(CreateIssueArgs),
}

/// A write-failure carrying the §17 `IntegrationOutcomeClass` (mirrors `LinearReadError`) so the executor
/// can branch terminal-non-auth (`ClientError`/`NotFound` → `LinearSyncFailed`) vs `AuthFailed` (the
/// deferred `auth_expired` path) vs transient (retry/queue). The `message` is for daemon LOGS ONLY — it is
/// NEVER surfaced into a persisted event (§15 — the event's `reason` is a structural class-name).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("linear write failed [{class:?}]: {message}")]
pub struct LinearWriteError {
    pub class: IntegrationOutcomeClass,
    pub message: String,
}

/// Mutate a Linear issue. `async` + dyn-compatible (`async_trait`) so the SYNC `LinearExecutor` injects a
/// `Box<dyn LinearWriteClient>` and drives it via a CAPTURED `tokio::runtime::Handle` (`block_on`, the 3a
/// mechanism). Both methods return `()` on success — Linear emits NO domain event (Q1); `ActionSucceeded`
/// is the audit record.
#[async_trait]
pub trait LinearWriteClient: Send + Sync {
    async fn link_issue(&self, args: &LinkIssueArgs) -> Result<(), LinearWriteError>;
    async fn create_issue(&self, args: &CreateIssueArgs) -> Result<(), LinearWriteError>;
}

/// The live client over an **injected** `reqwest::Client` + endpoint + api_key (auth bootstrap deferred —
/// never builds the key / reads the keychain; the `LinearGraphqlReadClient` precedent). Builds the GraphQL
/// mutation with TYPED VARIABLES (never interpolation — injection-safe), POSTs, and classifies via
/// `classify_linear_write_response`. The HTTP round-trip is the fake-covered non-deterministic edge — no
/// unit test (Step 7.5: only the test module + `FakeLinearWriteClient` reference the trait).
pub struct LinearGraphqlWriteClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: String,
}

impl LinearGraphqlWriteClient {
    /// `endpoint` is normally `https://api.linear.app/graphql`; `api_key` is INJECTED (auth bootstrap
    /// deferred; this client never reads the keychain / mints the key).
    pub fn new(http: reqwest::Client, endpoint: String, api_key: String) -> Self {
        Self {
            http,
            endpoint,
            api_key,
        }
    }

    /// POST a GraphQL mutation body, read (status, the two rate-limit headers, body), and classify.
    /// Shared by both mutations — success = unit (no domain object; Linear emits no success event, Q1).
    async fn run_mutation(&self, body: serde_json::Value) -> Result<(), LinearWriteError> {
        let response = self
            .http
            .post(&self.endpoint)
            .header("Authorization", &self.api_key) // §15: the key reaches ONLY this header
            .json(&body)
            .send()
            .await
            .map_err(|e| to_write_error(&e))?;
        let status = response.status().as_u16();
        // the two rate-limit backoff hints (precedence resolved in classify_linear_write_response →
        // classify_graphql_error_code, edges-016): Linear's X-RateLimit-Requests-Reset (epoch-ms) wins,
        // RFC-7231 Retry-After is the fallback. Here we only read the header values.
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let retry_after = header("Retry-After");
        let rate_limit_reset = header("X-RateLimit-Requests-Reset");
        let text = response.text().await.map_err(|e| to_write_error(&e))?;
        classify_linear_write_response(
            status,
            retry_after.as_deref(),
            rate_limit_reset.as_deref(),
            &text,
        )
        // §15: the message is logs-only + STRUCTURAL (no body/key content) — the persisted event's
        // `reason` is the executor's `structural_reason(class)`, never this string.
        .map_err(|class| LinearWriteError {
            class,
            message: format!("linear write [http {status}]"),
        })
    }
}

/// The Linear `attachmentLinkURL` mutation — links a URL/ref to an issue. TYPED variables ($issueId,
/// $url) — never interpolated (injection-safe, the `build_issue_query` precedent).
const LINK_MUTATION: &str = "mutation Attach($issueId: String!, $url: String!) { attachmentLinkURL(issueId: $issueId, url: $url) { success } }";
/// The Linear `issueCreate` mutation. TYPED variables ($teamId/$title/$description) — never interpolated.
const CREATE_MUTATION: &str = "mutation Create($teamId: String!, $title: String!, $description: String) { issueCreate(input: { teamId: $teamId, title: $title, description: $description }) { success } }";

#[async_trait]
impl LinearWriteClient for LinearGraphqlWriteClient {
    async fn link_issue(&self, args: &LinkIssueArgs) -> Result<(), LinearWriteError> {
        let body = serde_json::json!({
            "query": LINK_MUTATION,
            "variables": { "issueId": args.issue_id, "url": args.target },
        });
        self.run_mutation(body).await
    }

    async fn create_issue(&self, args: &CreateIssueArgs) -> Result<(), LinearWriteError> {
        let body = serde_json::json!({
            "query": CREATE_MUTATION,
            "variables": {
                "teamId": args.team_id,
                "title": args.title,
                "description": args.description,
            },
        });
        self.run_mutation(body).await
    }
}

/// Map a `reqwest::Error` (a transport/connection/decode failure — NOT an HTTP 4xx/5xx, which reqwest
/// returns as `Ok(response)`) → `LinearWriteError`. **Key-free by construction**: takes ONLY the error,
/// never the api_key (§15 rule #5; a reqwest error's Display carries the URL/kind, never request headers).
/// A transport failure is transient → `TransportError` (writes queue, not fail; §17 line-452).
fn to_write_error(err: &reqwest::Error) -> LinearWriteError {
    LinearWriteError {
        class: classify(None, None, true),
        message: err.to_string(),
    }
}

/// Test double — records each call (`LinearWriteCall`) + returns a canned `Ok`/`Err`, or HANGS (the
/// timeout test). The seam `LinearExecutor`'s tests consume (the live HTTP edge is non-deterministic —
/// fake-covered per CLAUDE.md). Gated behind `test-support` (the `FakeGithubWriteClient`/`FakeGitCli`
/// precedent; the daemon dev-dep self-enables it for test/bench targets — LESSON 21).
#[cfg(feature = "test-support")]
pub struct FakeLinearWriteClient {
    calls: std::sync::Arc<std::sync::Mutex<Vec<LinearWriteCall>>>,
    mode: FakeWriteMode,
}

#[cfg(feature = "test-support")]
enum FakeWriteMode {
    Ok,
    Err(LinearWriteError),
    /// a future that never resolves — exercises the executor's `block_on` timeout.
    Hang,
}

#[cfg(feature = "test-support")]
impl FakeLinearWriteClient {
    /// records the call; returns `Ok(())` (a successful mutation — no domain object, Q1).
    pub fn ok() -> Self {
        Self {
            calls: Default::default(),
            mode: FakeWriteMode::Ok,
        }
    }
    /// records the call; returns the canned `LinearWriteError`.
    pub fn err(error: LinearWriteError) -> Self {
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
    /// a handle to the recorded calls — clone it BEFORE the client is boxed.
    pub fn calls(&self) -> std::sync::Arc<std::sync::Mutex<Vec<LinearWriteCall>>> {
        self.calls.clone()
    }

    async fn respond(&self) -> Result<(), LinearWriteError> {
        match &self.mode {
            FakeWriteMode::Ok => Ok(()),
            FakeWriteMode::Err(e) => Err(e.clone()),
            // never resolves → the executor's `tokio::time::timeout` fires (the write-actor bound).
            FakeWriteMode::Hang => std::future::pending().await,
        }
    }
}

#[cfg(feature = "test-support")]
#[async_trait]
impl LinearWriteClient for FakeLinearWriteClient {
    async fn link_issue(&self, args: &LinkIssueArgs) -> Result<(), LinearWriteError> {
        // `.unwrap()` (daemon no-bare-unwrap convention): test-only double; the Mutex is uncontended
        // (single-threaded test use) and only poisons on a prior in-lock panic — no new failure mode
        // (the FakeGithubWriteClient/FakeGitCli precedent).
        self.calls
            .lock()
            .unwrap()
            .push(LinearWriteCall::LinkIssue(args.clone()));
        self.respond().await
    }

    async fn create_issue(&self, args: &CreateIssueArgs) -> Result<(), LinearWriteError> {
        self.calls
            .lock()
            .unwrap()
            .push(LinearWriteCall::CreateIssue(args.clone()));
        self.respond().await
    }
}

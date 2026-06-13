//! The P7.1 (edges-023) GitHub sync executor (`ExecutorKind::Github`) — the FIRST real edges
//! EXTERNAL-NETWORK mutator.
//!
//! `GithubExecutor` handles `github.create_pr` (risk-3) + `github.create_pr_draft` (risk-2) by driving an
//! injected async [`GithubWriteClient`](crate::integrations::github_write::GithubWriteClient) (octocrab)
//! from the SYNC `ActionExecutor` trait. On success it emits `PullRequestSynced` through the in-txn §15
//! gate via the edges-019 `EmittedEvent::Namespaced` bridge; `side_effect_applied: true` (a real PR was
//! created → honest `ActionPartiallySucceeded` on a txn-B fault, LESSON 21). Every other action delegates
//! to the inner side-effect-free stub (the `GitExecutor` precedent).
//!
//! **3a (LOAD-BEARING).** `execute()` runs on the write-actor's dedicated `std::thread` (a non-worker
//! with NO entered runtime — `runtime/writer.rs` → `gateway/pipeline.rs`). So `Handle::current()` there
//! PANICS and `spawn_blocking` is awkward; the mechanism is a **captured `tokio::runtime::Handle`** (the
//! daemon captures `Handle::current()` in `main.rs`'s async `run()` and passes it here) + a
//! `handle.block_on(timeout(fut))`. `#[tokio::main]` (multi-thread) runs the reactor on the runtime's
//! workers, so `block_on` from this non-worker thread completes the octocrab I/O without panicking. A
//! hard timeout bounds it so an octocrab hang can NEVER wedge the single write-actor indefinitely.
//!
//! **§17 failure taxonomy.** A TERMINAL non-auth write error (`ClientError`/`NotFound`) → the action
//! FAILS AND emits `GithubSyncFailed` (the `ExecutionOutcome::FailedWithEvents` extension; `reason` = a
//! §15 STRUCTURAL class-name, NEVER raw API text). `AuthFailed` → plain `Failed` (the `auth_expired`
//! `*SyncFailed` variant is DEFERRED — needs a §17/INV-SEC re-review). A transient class
//! (`ServerError`/`RateLimited`/`TransportError`) → plain `Failed` (retry/queue — `GithubSyncFailed` is
//! the terminal-non-auth class ONLY).

use std::time::Duration;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::events::{GithubSyncFailed, Provider, PullRequestSynced};
use nexusops_shared::time::Timestamp;

use crate::clock::Clock;
use crate::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecError, ExecutionOutcome,
};
use crate::integrations::classifier::IntegrationOutcomeClass;
use crate::integrations::github_write::{CreatePrArgs, GithubWriteClient, GithubWriteError};
use crate::integrations::pull_request::derive_pull_request_status;

/// The action types this executor handles directly (`ExecutorKind::Github`); the rest delegate.
const GITHUB_CREATE_PR: &str = "github.create_pr";
const GITHUB_CREATE_PR_DRAFT: &str = "github.create_pr_draft";

/// The default network timeout — generous enough for a slow API, short enough that a hung call can't
/// wedge the single write-actor for long. Tests inject a short one via [`GithubExecutor::with_timeout`].
const NETWORK_TIMEOUT: Duration = Duration::from_secs(30);

/// Runs `github.create_pr*` via the injected async write-client seam, driven from the SYNC trait over a
/// CAPTURED `tokio::runtime::Handle` (the 3a mechanism). Holds an inner [`CatalogExecutor`] for the
/// catalog `requires_resource_refs` precondition + delegation of the non-handled `github.*` actions, and
/// an injected [`Clock`] for the deterministic `pr_checked_at`/`failed_at` stamps (golden-log replay).
pub struct GithubExecutor {
    client: Box<dyn GithubWriteClient>,
    handle: tokio::runtime::Handle,
    clock: Box<dyn Clock>,
    timeout: Duration,
    inner: CatalogExecutor,
}

impl GithubExecutor {
    /// Build with the default [`NETWORK_TIMEOUT`].
    pub fn new(
        client: Box<dyn GithubWriteClient>,
        handle: tokio::runtime::Handle,
        clock: Box<dyn Clock>,
    ) -> Self {
        Self::with_timeout(client, handle, clock, NETWORK_TIMEOUT)
    }

    /// Build with an explicit network timeout (tests inject a short one to exercise the bound fast).
    pub fn with_timeout(
        client: Box<dyn GithubWriteClient>,
        handle: tokio::runtime::Handle,
        clock: Box<dyn Clock>,
        timeout: Duration,
    ) -> Self {
        Self {
            client,
            handle,
            clock,
            timeout,
            inner: CatalogExecutor::new(),
        }
    }

    fn execute_create_pr(&self, req: &ActionRequest, draft: bool) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition (the repo IDENTITY) FIRST — this
        // path runs its own side effect, never reaching `inner.execute`'s validation (the GitExecutor/
        // SessionExecutor precedent).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }

        // Operational params from `req.inputs` (the GitExecutor precedent; the resource_ref is the repo
        // IDENTITY for audit/policy). §7.2/§15 redacted-op-inputs MVP-accept (edges-020 finding, lead-
        // confirmed for Wave-D): owner/repo/head/base/title are LOW-entropy identifiers → survive the
        // approve-path §15 inputs-redaction (flag back if any field is genuinely high-entropy — none here).
        //
        // PARAM-INJECTION guard (LESSON 31 adaptation): octocrab is a TYPED API (no shell / no CLI arg
        // parsing) → the leading-`-` CLI vector does NOT apply; the analogous guard is fail-closed
        // non-empty validation of EVERY required operand BEFORE the network call (a blank operand never
        // reaches GitHub; the network call is never invoked on a malformed request).
        let Some(owner) = string_input(req, "owner") else {
            return ExecutionOutcome::Failed(
                "github.create_pr requires a non-empty inputs[\"owner\"]".to_string(),
            );
        };
        let Some(repo) = string_input(req, "repo") else {
            return ExecutionOutcome::Failed(
                "github.create_pr requires a non-empty inputs[\"repo\"]".to_string(),
            );
        };
        let Some(head) = string_input(req, "head") else {
            return ExecutionOutcome::Failed(
                "github.create_pr requires a non-empty inputs[\"head\"]".to_string(),
            );
        };
        let Some(base) = string_input(req, "base") else {
            return ExecutionOutcome::Failed(
                "github.create_pr requires a non-empty inputs[\"base\"]".to_string(),
            );
        };
        let Some(title) = string_input(req, "title") else {
            return ExecutionOutcome::Failed(
                "github.create_pr requires a non-empty inputs[\"title\"]".to_string(),
            );
        };
        // body is OPTIONAL — a blank/absent body → None (GitHub's create treats an empty and an absent
        // PR body identically; consistent with the other operands' `string_input` handling).
        let body = string_input(req, "body");

        let args = CreatePrArgs {
            owner,
            repo,
            head,
            base,
            title,
            body,
            draft,
        };

        // 3a (LOAD-BEARING): drive the async client via the CAPTURED Handle's `block_on` (NEVER
        // `Handle::current()` — `execute()` runs on the write-actor's dedicated std::thread, a non-worker
        // with no entered runtime → `Handle::current()` would PANIC). A hard `timeout` bounds it so an
        // octocrab hang can never wedge the single write-actor.
        let result = self.handle.block_on(async {
            tokio::time::timeout(self.timeout, self.client.create_pull_request(&args)).await
        });
        let created = match result {
            // timed out — the write-actor must never hang unbounded. STRUCTURAL reason only (§15).
            Err(_elapsed) => {
                return ExecutionOutcome::Failed(
                    "github.create_pr timed out (structural)".to_string(),
                )
            }
            Ok(Err(write_err)) => return self.classify_failure(write_err),
            Ok(Ok(created)) => created,
        };

        // success — stamp `pr_checked_at` via the injected Clock (UTC-Z; the Clock contract guarantees
        // valid RFC3339-Z, but fail CLOSED rather than emit a malformed audit event).
        let pr_checked_at = match Timestamp::parse(&self.clock.now_rfc3339()) {
            Ok(ts) => ts,
            Err(e) => return ExecutionOutcome::Failed(format!("invalid clock timestamp: {e}")),
        };
        let payload = PullRequestSynced {
            pr_number: created.pr_number,
            // status REUSES the frozen §5.1 PullRequest machine (no fork — adapters map INTO it).
            status: derive_pull_request_status(&created.signals),
            branch: created.branch,
            base: created.base,
            // a just-created PR has no computed mergeability/checks yet → None (a status-refresh sync is
            // the deferred proj_pull_request follow-on).
            mergeable: None,
            checks_summary: None,
            pr_checked_at,
        };
        // serialize the FROZEN event struct (the Namespaced bridge, Q1=B); a serialize fault fails CLOSED.
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => return ExecutionOutcome::Failed(format!("serialize PullRequestSynced: {e}")),
        };
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "github.create_pr — created a PR via octocrab".to_string(),
            // a real PR was created on GitHub BEFORE txn-B → a lost terminal write yields the honest
            // ActionPartiallySucceeded (the PR exists; the event didn't commit), NOT a clean rollback.
            side_effect_applied: true,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: PullRequestSynced::EVENT_TYPE,
                payload_json,
            }],
        }
    }

    /// Map a write failure → the §17 outcome. TERMINAL non-auth (`ClientError`/`NotFound`) →
    /// `FailedWithEvents` emitting `GithubSyncFailed` (the action FAILS + a durable §17 record).
    /// `AuthFailed` → plain `Failed` (the `auth_expired` variant is DEFERRED). A transient class →
    /// plain `Failed` (retry/queue; `GithubSyncFailed` is the terminal-non-auth class ONLY). The event
    /// `reason` is the classifier's STRUCTURAL class-name ONLY — NEVER raw API text (§15).
    fn classify_failure(&self, err: GithubWriteError) -> ExecutionOutcome {
        match &err.class {
            // the deferred auth_expired path: plain Failed, structural reason, NO event.
            IntegrationOutcomeClass::AuthFailed => {
                ExecutionOutcome::Failed("github.create_pr failed: auth_failed".to_string())
            }
            // terminal non-auth → the action fails AND emits GithubSyncFailed atomic with ActionFailed.
            IntegrationOutcomeClass::ClientError { .. } | IntegrationOutcomeClass::NotFound => {
                let reason = structural_reason(&err.class);
                let failed_at = match Timestamp::parse(&self.clock.now_rfc3339()) {
                    Ok(ts) => ts,
                    Err(e) => {
                        return ExecutionOutcome::Failed(format!("invalid clock timestamp: {e}"))
                    }
                };
                let payload = GithubSyncFailed {
                    provider: Provider::Github,
                    // §15: a STRUCTURAL class-name ONLY — never raw API response text.
                    reason: reason.clone(),
                    failed_at,
                };
                let payload_json = match serde_json::to_string(&payload) {
                    Ok(j) => j,
                    Err(e) => {
                        return ExecutionOutcome::Failed(format!("serialize GithubSyncFailed: {e}"))
                    }
                };
                ExecutionOutcome::FailedWithEvents {
                    detail: format!("github.create_pr failed: {reason}"),
                    emitted_events: vec![EmittedEvent::Namespaced {
                        event_type: GithubSyncFailed::EVENT_TYPE,
                        payload_json,
                    }],
                }
            }
            // transient — retry/queue, never a terminal *SyncFailed.
            IntegrationOutcomeClass::ServerError
            | IntegrationOutcomeClass::TransportError
            | IntegrationOutcomeClass::RateLimited { .. } => ExecutionOutcome::Failed(format!(
                "github.create_pr failed (transient): {}",
                structural_reason(&err.class)
            )),
            // Success never reaches classify_failure (it's the Ok arm); fail closed defensively.
            IntegrationOutcomeClass::Success => ExecutionOutcome::Failed(
                "github.create_pr: a Success was classified as a failure (unreachable)".to_string(),
            ),
        }
    }
}

impl ActionExecutor for GithubExecutor {
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError> {
        self.inner.validate(req)
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            GITHUB_CREATE_PR => self.execute_create_pr(req, false),
            GITHUB_CREATE_PR_DRAFT => self.execute_create_pr(req, true),
            // any other Github-kind action delegates to the inner side-effect-free stub (no event).
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}

/// The §15 STRUCTURAL class-name for a failure class — NEVER raw API text. Drives the persisted
/// `GithubSyncFailed.reason` + the structured `Failed` detail.
fn structural_reason(class: &IntegrationOutcomeClass) -> String {
    match class {
        IntegrationOutcomeClass::Success => "success",
        IntegrationOutcomeClass::RateLimited { .. } => "rate_limited",
        IntegrationOutcomeClass::ServerError => "server_error",
        IntegrationOutcomeClass::TransportError => "transport_error",
        IntegrationOutcomeClass::AuthFailed => "auth_failed",
        IntegrationOutcomeClass::ClientError { .. } => "client_error",
        IntegrationOutcomeClass::NotFound => "not_found",
    }
    .to_string()
}

/// a non-blank string input — `None` if absent or whitespace-only (fail-closed for a required input;
/// the natural optionality for `body`). The `GitExecutor::string_input` precedent.
fn string_input(req: &ActionRequest, key: &str) -> Option<String> {
    req.inputs
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

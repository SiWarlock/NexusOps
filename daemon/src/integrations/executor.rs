//! The P7.1 edges EXTERNAL-NETWORK sync executors — `GithubExecutor` (edges-023, `ExecutorKind::Github`)
//! and `LinearExecutor` (edges-024, `ExecutorKind::Linear`). Both share the captured-`Handle`/`block_on`/
//! `NETWORK_TIMEOUT` 3a mechanism + the exhaustive §17 [`classify_sync_failure`] disposition + the
//! `EmittedEvent::Namespaced` / `FailedWithEvents` emit path; they differ only in the provider client +
//! the per-provider success/failure event (github emits `PullRequestSynced`; linear emits NO success
//! event [Q1], only `LinearSyncFailed` on terminal-non-auth).
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
use nexusops_shared::events::{
    GithubSyncFailed, LinearSyncFailed, Provider, PullRequestMerged, PullRequestSynced,
    ReviewSynced,
};
use nexusops_shared::time::Timestamp;

use crate::clock::Clock;
use crate::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecError, ExecutionOutcome,
};
use crate::integrations::classifier::IntegrationOutcomeClass;
use crate::integrations::github_write::{
    map_merge_method, CreatePrArgs, GithubWriteClient, GithubWriteError, MergePrArgs,
    SyncReviewsArgs,
};
use crate::integrations::linear_write::{
    CreateIssueArgs, LinearWriteClient, LinearWriteError, LinkIssueArgs,
};
use crate::integrations::pull_request::derive_pull_request_status;

/// The action types `GithubExecutor` handles directly (`ExecutorKind::Github`); the rest delegate.
const GITHUB_CREATE_PR: &str = "github.create_pr";
const GITHUB_CREATE_PR_DRAFT: &str = "github.create_pr_draft";
const GITHUB_SYNC_REVIEWS: &str = "github.sync_reviews";
/// D9 — the cat-1 GitHub WRITE that merges a remote PR (head→base).
const GITHUB_MERGE_PR: &str = "github.merge_pr";
/// The action types `LinearExecutor` handles directly (`ExecutorKind::Linear`); the rest delegate.
const LINEAR_LINK_ISSUE: &str = "linear.link_issue";
const LINEAR_CREATE_ISSUE: &str = "linear.create_issue";

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
            Ok(Err(write_err)) => return self.classify_failure("github.create_pr", write_err),
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
            // D6 — the diff-stats captured from the create response's PR (the §11.2 PR card render data);
            // `extract_pr_signals` populated them on `created.signals`. `None` if GitHub omitted them.
            additions: created.signals.additions,
            deletions: created.signals.deletions,
            changed_files: created.signals.changed_files,
            commits: created.signals.commits,
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

    /// Map a write failure → the §17 outcome via the SHARED [`classify_sync_failure`] disposition (so
    /// github + linear can never route the same class differently — LESSON 32). TERMINAL non-auth
    /// (`ClientError`/`NotFound`) → `FailedWithEvents` emitting `GithubSyncFailed` (the action FAILS + a
    /// durable §17 record; `reason` = the §15 STRUCTURAL class-name, NEVER raw API text). `AuthFailed` →
    /// plain `Failed` (the `auth_expired` variant DEFERRED). Transient → plain `Failed` (retry/queue).
    fn classify_failure(&self, op: &str, err: GithubWriteError) -> ExecutionOutcome {
        match classify_sync_failure(&err.class) {
            SyncFailure::Auth => ExecutionOutcome::Failed(format!("{op} failed: auth_failed")),
            SyncFailure::Transient { reason } => {
                ExecutionOutcome::Failed(format!("{op} failed (transient): {reason}"))
            }
            SyncFailure::TerminalNonAuth { reason } => {
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
                    detail: format!("{op} failed: {reason}"),
                    emitted_events: vec![EmittedEvent::Namespaced {
                        event_type: GithubSyncFailed::EVENT_TYPE,
                        payload_json,
                    }],
                }
            }
        }
    }

    fn execute_sync_reviews(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition (the repo IDENTITY) FIRST.
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // fail-closed non-empty/typed validation of EVERY required operand BEFORE the network call (the
        // execute_create_pr param-injection-guard precedent; octocrab is a TYPED API, no shell vector).
        let Some(owner) = string_input(req, "owner") else {
            return ExecutionOutcome::Failed(
                "github.sync_reviews requires a non-empty inputs[\"owner\"]".to_string(),
            );
        };
        let Some(repo) = string_input(req, "repo") else {
            return ExecutionOutcome::Failed(
                "github.sync_reviews requires a non-empty inputs[\"repo\"]".to_string(),
            );
        };
        let Some(pr_number) = u64_input(req, "pr_number") else {
            return ExecutionOutcome::Failed(
                "github.sync_reviews requires inputs[\"pr_number\"] (a positive integer)"
                    .to_string(),
            );
        };

        let args = SyncReviewsArgs {
            owner,
            repo,
            pr_number,
        };
        // 3a (LESSON 46): drive the async client via the CAPTURED Handle's `block_on` + a hard timeout so
        // an octocrab hang can never wedge the single write-actor.
        let result = self.handle.block_on(async {
            tokio::time::timeout(self.timeout, self.client.list_reviews(&args)).await
        });
        let reviews = match result {
            Err(_elapsed) => {
                return ExecutionOutcome::Failed(
                    "github.sync_reviews timed out (structural)".to_string(),
                )
            }
            Ok(Err(write_err)) => return self.classify_failure("github.sync_reviews", write_err),
            Ok(Ok(reviews)) => reviews,
        };

        // stamp `review_synced_at` ONCE via the injected Clock (UTC-Z; fail CLOSED on a malformed stamp).
        let review_synced_at = match Timestamp::parse(&self.clock.now_rfc3339()) {
            Ok(ts) => ts,
            Err(e) => return ExecutionOutcome::Failed(format!("invalid clock timestamp: {e}")),
        };
        // one ReviewSynced per review (zero reviews → zero events, a clean empty Succeeded). pr_number from
        // the inputs (the PR being synced); state is the already-mapped frozen ReviewState.
        let mut emitted_events = Vec::with_capacity(reviews.len());
        for r in reviews {
            let payload = ReviewSynced {
                review_id: r.review_id,
                pr_number,
                reviewer: r.reviewer,
                state: r.state,
                submitted_at: r.submitted_at,
                body: r.body,
                review_synced_at: review_synced_at.clone(),
            };
            let payload_json = match serde_json::to_string(&payload) {
                Ok(j) => j,
                Err(e) => return ExecutionOutcome::Failed(format!("serialize ReviewSynced: {e}")),
            };
            emitted_events.push(EmittedEvent::Namespaced {
                event_type: ReviewSynced::EVENT_TYPE,
                payload_json,
            });
        }
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: format!(
                "github.sync_reviews — synced {} review(s) via octocrab",
                emitted_events.len()
            ),
            // a READ — no GitHub mutation; a lost terminal write is a CLEAN rollback (NOT the
            // ActionPartiallySucceeded create_pr emits — nothing was applied on GitHub).
            side_effect_applied: false,
            emitted_events,
        }
    }

    /// D9 — the cat-1 `github.merge_pr` WRITE. Validates the catalog `requires_resource_refs` precondition
    /// (the Repo IDENTITY) FIRST, then fail-closed non-empty/typed validation of EVERY operand BEFORE the
    /// network call (the `execute_create_pr` param-injection-guard precedent; octocrab is a TYPED API), then
    /// drives the SHA-pinned merge via the captured-`Handle` `block_on`+timeout (LESSON 46). Success → a
    /// `PullRequestMerged` event with `side_effect_applied: true` (a real merge happened BEFORE txn-B → a
    /// lost terminal write yields the honest `ActionPartiallySucceeded`, LESSON 21). Failure → the SHARED
    /// `classify_failure` (terminal-non-auth → `GithubSyncFailed`; auth/transient → plain `Failed`; §17).
    fn execute_merge_pr(&self, req: &ActionRequest) -> ExecutionOutcome {
        // catalog precondition FIRST — a merge with no auditable Repo identity must never reach GitHub.
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        let Some(owner) = string_input(req, "owner") else {
            return ExecutionOutcome::Failed(
                "github.merge_pr requires a non-empty inputs[\"owner\"]".to_string(),
            );
        };
        let Some(repo) = string_input(req, "repo") else {
            return ExecutionOutcome::Failed(
                "github.merge_pr requires a non-empty inputs[\"repo\"]".to_string(),
            );
        };
        let Some(pr_number) = u64_input(req, "pr_number") else {
            return ExecutionOutcome::Failed(
                "github.merge_pr requires inputs[\"pr_number\"] (a positive integer)".to_string(),
            );
        };
        // the SHA-pin (F1 anti-race): the merge is bound to the head the human approved; a missing/blank
        // sha fails closed (the merge MUST be SHA-pinned — never an unpinned "merge whatever is on top").
        let Some(sha) = string_input(req, "sha") else {
            return ExecutionOutcome::Failed(
                "github.merge_pr requires a non-empty inputs[\"sha\"] (the approved head SHA — anti-race pin)"
                    .to_string(),
            );
        };
        // the merge_method (F2 audit-integrity): the approved+audited method executes EXACTLY — an
        // absent/blank or unmappable method fails closed (NEVER a silent server-side default).
        let Some(merge_method_str) = string_input(req, "merge_method") else {
            return ExecutionOutcome::Failed(
                "github.merge_pr requires a non-empty inputs[\"merge_method\"] (merge|squash|rebase)"
                    .to_string(),
            );
        };
        let merge_method = match map_merge_method(&merge_method_str) {
            Ok(m) => m,
            Err(reason) => return ExecutionOutcome::Failed(format!("github.merge_pr: {reason}")),
        };

        let args = MergePrArgs {
            owner,
            repo,
            pr_number,
            sha,
            merge_method,
        };

        // 3a (LESSON 46): drive the async client via the CAPTURED Handle's `block_on` + a hard timeout so
        // an octocrab hang can never wedge the single write-actor.
        let result = self.handle.block_on(async {
            tokio::time::timeout(self.timeout, self.client.merge_pull_request(&args)).await
        });
        let merged = match result {
            Err(_elapsed) => {
                return ExecutionOutcome::Failed(
                    "github.merge_pr timed out (structural)".to_string(),
                )
            }
            Ok(Err(write_err)) => return self.classify_failure("github.merge_pr", write_err),
            Ok(Ok(merged)) => merged,
        };

        // success — stamp `merged_at` via the injected Clock (UTC-Z; fail CLOSED on a malformed stamp).
        // This is the DAEMON-clock observation time (when the daemon recorded the merge), NOT GitHub's
        // authoritative server merge timestamp — octocrab's `Merge` response exposes only `sha`/`merged`,
        // no `merged_at` (the `pr_checked_at` create_pr precedent). Close enough for the §11.2 PR card.
        let merged_at = match Timestamp::parse(&self.clock.now_rfc3339()) {
            Ok(ts) => ts,
            Err(e) => return ExecutionOutcome::Failed(format!("invalid clock timestamp: {e}")),
        };
        let payload = PullRequestMerged {
            pr_number,
            merge_commit_sha: merged.merge_commit_sha,
            merged_at,
        };
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => return ExecutionOutcome::Failed(format!("serialize PullRequestMerged: {e}")),
        };
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "github.merge_pr — merged a PR via octocrab".to_string(),
            // a real merge was applied on GitHub BEFORE txn-B → a lost terminal write yields the honest
            // ActionPartiallySucceeded (the merge happened; the event didn't commit), NOT a clean rollback.
            side_effect_applied: true,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: PullRequestMerged::EVENT_TYPE,
                payload_json,
            }],
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
            GITHUB_SYNC_REVIEWS => self.execute_sync_reviews(req),
            GITHUB_MERGE_PR => self.execute_merge_pr(req),
            // any other Github-kind action delegates to the inner side-effect-free stub (no event).
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}

/// The P7.1 (edges-024) Linear sync executor (`ExecutorKind::Linear`) — the second edges external-network
/// mutator, mirroring [`GithubExecutor`]. Handles `linear.link_issue` (risk-2, requires_resource_refs) +
/// `linear.create_issue` (risk-2, NO resource_ref) by driving an injected async [`LinearWriteClient`]
/// (Linear GraphQL) over the captured-`Handle` `block_on` + [`NETWORK_TIMEOUT`] 3a mechanism (shared with
/// github). **Success → `ActionSucceeded` ONLY — NO Linear domain event** (the frozen contract has none,
/// unlike github's `PullRequestSynced`; Linear is read on-demand via `fetch_issue`, §7.3 — the intentional
/// asymmetry). Terminal non-auth → `LinearSyncFailed` via [`ExecutionOutcome::FailedWithEvents`] (the
/// SHARED §17 [`classify_sync_failure`] disposition); auth/transient → plain `Failed`.
pub struct LinearExecutor {
    client: Box<dyn LinearWriteClient>,
    handle: tokio::runtime::Handle,
    clock: Box<dyn Clock>,
    timeout: Duration,
    inner: CatalogExecutor,
}

impl LinearExecutor {
    /// Build with the default [`NETWORK_TIMEOUT`] (shared with github).
    pub fn new(
        client: Box<dyn LinearWriteClient>,
        handle: tokio::runtime::Handle,
        clock: Box<dyn Clock>,
    ) -> Self {
        Self::with_timeout(client, handle, clock, NETWORK_TIMEOUT)
    }

    /// Build with an explicit network timeout (tests inject a short one to exercise the bound fast).
    pub fn with_timeout(
        client: Box<dyn LinearWriteClient>,
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

    fn execute_link_issue(&self, req: &ActionRequest) -> ExecutionOutcome {
        // catalog precondition FIRST (link_issue requires_resource_refs=true — the issue IDENTITY).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // operands from req.inputs (§7.2/§15 redacted-op-inputs MVP-accept — issue_id/target are
        // low-entropy identifiers). PARAM-INJECTION guard (LESSON 31/32): fail-closed non-empty
        // validation BEFORE the call; Linear GraphQL uses TYPED VARIABLES (never interpolation) →
        // injection-safe by construction (the `build_issue_query` precedent).
        let Some(issue_id) = string_input(req, "issue_id") else {
            return ExecutionOutcome::Failed(
                "linear.link_issue requires a non-empty inputs[\"issue_id\"]".to_string(),
            );
        };
        let Some(target) = string_input(req, "target") else {
            return ExecutionOutcome::Failed(
                "linear.link_issue requires a non-empty inputs[\"target\"]".to_string(),
            );
        };
        let args = LinkIssueArgs { issue_id, target };
        // 3a: drive the async client via the CAPTURED Handle's block_on + a hard timeout (NEVER
        // Handle::current() — write-actor-thread panic; the edges-023 mechanism).
        let result = self.handle.block_on(async {
            tokio::time::timeout(self.timeout, self.client.link_issue(&args)).await
        });
        self.finish(req, result)
    }

    fn execute_create_issue(&self, req: &ActionRequest) -> ExecutionOutcome {
        // catalog precondition FIRST. create_issue requires_resource_refs=FALSE → resolve() passes with
        // NO ref (do not over-require); the precondition check still runs (uniform with link_issue).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        let Some(team_id) = string_input(req, "team_id") else {
            return ExecutionOutcome::Failed(
                "linear.create_issue requires a non-empty inputs[\"team_id\"]".to_string(),
            );
        };
        let Some(title) = string_input(req, "title") else {
            return ExecutionOutcome::Failed(
                "linear.create_issue requires a non-empty inputs[\"title\"]".to_string(),
            );
        };
        let description = string_input(req, "description"); // optional
        let args = CreateIssueArgs {
            team_id,
            title,
            description,
        };
        let result = self.handle.block_on(async {
            tokio::time::timeout(self.timeout, self.client.create_issue(&args)).await
        });
        self.finish(req, result)
    }

    /// Shared tail for both arms (both return `Result<(), LinearWriteError>`): map the
    /// `block_on(timeout(...))` result → the outcome. **Success → `Succeeded { side_effect_applied: true,
    /// changed_resources: req.resource_refs, emitted_events: [] }`** — NO Linear domain event (Q1;
    /// `ActionSucceeded` is the audit record). A timeout → `Failed` (structural). A write error → the
    /// SHARED §17 [`LinearExecutor::classify_failure`].
    fn finish(
        &self,
        req: &ActionRequest,
        result: Result<Result<(), LinearWriteError>, tokio::time::error::Elapsed>,
    ) -> ExecutionOutcome {
        match result {
            // timed out — the write-actor must never hang unbounded. STRUCTURAL reason only (§15).
            Err(_elapsed) => {
                ExecutionOutcome::Failed("linear sync timed out (structural)".to_string())
            }
            Ok(Err(err)) => self.classify_failure(err),
            Ok(Ok(())) => ExecutionOutcome::Succeeded {
                // the resources this action touched — the audit record of what changed (the
                // GithubExecutor precedent: link_issue's issue resource_ref; create_issue carries none →
                // empty). Dropping it would leave a successful link's ActionSucceeded ref-less.
                changed_resources: req.resource_refs.clone(),
                detail: "linear sync — mutation applied via Linear GraphQL".to_string(),
                // a real Linear mutation was applied BEFORE txn-B → a lost terminal write yields the
                // honest ActionPartiallySucceeded (LESSON 21), NOT a clean rollback.
                side_effect_applied: true,
                // Q1: the frozen 11-event family has NO Linear success event (unlike PullRequestSynced)
                // — ActionSucceeded is the audit record; Linear is read on-demand via fetch_issue (§7.3).
                emitted_events: vec![],
            },
        }
    }

    /// The §17 disposition via the SHARED [`classify_sync_failure`] (mirrors github — terminal non-auth →
    /// `LinearSyncFailed`; auth/transient → plain `Failed`). `reason` = the §15 STRUCTURAL class-name.
    fn classify_failure(&self, err: LinearWriteError) -> ExecutionOutcome {
        match classify_sync_failure(&err.class) {
            SyncFailure::Auth => {
                ExecutionOutcome::Failed("linear sync failed: auth_failed".to_string())
            }
            SyncFailure::Transient { reason } => {
                ExecutionOutcome::Failed(format!("linear sync failed (transient): {reason}"))
            }
            SyncFailure::TerminalNonAuth { reason } => {
                let failed_at = match Timestamp::parse(&self.clock.now_rfc3339()) {
                    Ok(ts) => ts,
                    Err(e) => {
                        return ExecutionOutcome::Failed(format!("invalid clock timestamp: {e}"))
                    }
                };
                let payload = LinearSyncFailed {
                    provider: Provider::Linear,
                    // §15: a STRUCTURAL class-name ONLY — never raw API/GraphQL response text.
                    reason: reason.clone(),
                    failed_at,
                };
                let payload_json = match serde_json::to_string(&payload) {
                    Ok(j) => j,
                    Err(e) => {
                        return ExecutionOutcome::Failed(format!("serialize LinearSyncFailed: {e}"))
                    }
                };
                ExecutionOutcome::FailedWithEvents {
                    detail: format!("linear sync failed: {reason}"),
                    emitted_events: vec![EmittedEvent::Namespaced {
                        event_type: LinearSyncFailed::EVENT_TYPE,
                        payload_json,
                    }],
                }
            }
        }
    }
}

impl ActionExecutor for LinearExecutor {
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError> {
        self.inner.validate(req)
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            LINEAR_LINK_ISSUE => self.execute_link_issue(req),
            LINEAR_CREATE_ISSUE => self.execute_create_issue(req),
            // any other Linear-kind action delegates to the inner side-effect-free stub (no event).
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}

/// The §17 disposition of an external-sync write failure — SHARED by every edges external-network mutator
/// (github/linear) so the Auth / TerminalNonAuth / Transient routing can NEVER diverge between providers
/// (a §17 correctness guard, not just DRY — LESSON 32). The executor maps each variant to its
/// provider-specific outcome (the `*SyncFailed` event type differs; the disposition does not).
enum SyncFailure {
    /// `AuthFailed` → a plain `Failed("…auth_failed")`, NO event (the `auth_expired` `*SyncFailed` variant
    /// is DEFERRED — needs a §17/INV-SEC re-review).
    Auth,
    /// terminal non-auth (`ClientError`/`NotFound`) → the action FAILS AND emits the provider's
    /// `*SyncFailed` event (`FailedWithEvents`). `reason` = the §15 structural class-name.
    TerminalNonAuth { reason: String },
    /// transient (`ServerError`/`RateLimited`/`TransportError`) → a plain `Failed`, NO event (retry/queue;
    /// `*SyncFailed` is the terminal-non-auth class ONLY). `reason` = the §15 structural class-name.
    Transient { reason: String },
}

/// Classify a write-failure class → its §17 disposition. **Exhaustive** (no `_`) — a new
/// `IntegrationOutcomeClass` variant forces a reconcile here (the `map_mergeable` precedent), so github +
/// linear can never route the same class differently. `Success` is unreachable from a failure path (the
/// executor calls this only on `Err`); fold it conservatively to `Transient` (a Success-classified-as-a-
/// failure is a client bug → retry is the safe disposition).
fn classify_sync_failure(class: &IntegrationOutcomeClass) -> SyncFailure {
    match class {
        IntegrationOutcomeClass::AuthFailed => SyncFailure::Auth,
        IntegrationOutcomeClass::ClientError { .. } | IntegrationOutcomeClass::NotFound => {
            SyncFailure::TerminalNonAuth {
                reason: structural_reason(class),
            }
        }
        IntegrationOutcomeClass::ServerError
        | IntegrationOutcomeClass::TransportError
        | IntegrationOutcomeClass::RateLimited { .. }
        | IntegrationOutcomeClass::Success => SyncFailure::Transient {
            reason: structural_reason(class),
        },
    }
}

/// The §15 STRUCTURAL class-name for a failure class — NEVER raw API text. Drives the persisted
/// `{Github,Linear}SyncFailed.reason` + the structured `Failed` detail (shared by both executors).
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

/// a POSITIVE-integer input — `None` if absent, non-numeric, or zero (fail-closed; a GitHub PR number is
/// `>= 1`, so `0` is never valid — the `string_input` non-empty-guard analogue). Accepts a JSON number OR
/// a numeric string (the ui/IPC may send `pr_number` as either).
fn u64_input(req: &ActionRequest, key: &str) -> Option<u64> {
    let v = req.inputs.get(key)?;
    v.as_u64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        .filter(|&n| n > 0)
}

//! P7.1 — Linear issue-state derivation (edges-013). The deterministic foundation of the Linear read
//! vertical (the analog of edges-004's GitHub PR-status derivation). A Linear issue is an
//! **external_task** (§5.1 R-8): its status derives from the issue's `WorkflowState.type` — the 6-value
//! closed set Linear's API exposes — NOT the team's custom state NAME. Two stages, mirroring the GitHub
//! `derive_pull_request_status` pattern:
//!   - `parse_linear_state_type` decodes the raw GraphQL `WorkflowState.type` string → the daemon enum
//!     (conservative floor + case-insensitive, the edges-008 convention);
//!   - `derive_task_status_from_linear` maps the state-type → the frozen §5.1 `Task` status (the
//!     external-task subset), an **exhaustive** match so a new state-type forces a reconcile (LESSON-2).
//!
//! The Linear → `Task` mapping is DAEMON-DEFINED (the architecture lists the Task machine + the R-8
//! subset rule but does not pin the Linear mapping — recorded as a §5.1/§9 arch-note, like the §7.2
//! PR-derivation precedence). Pure — no network (the Linear read client that fetches the issue is the
//! next slice; review-flavored Task states like NeedsReview/PrOpened come from a PR, not an issue's
//! state-type, so they are correctly unreachable here).

use async_trait::async_trait;
use serde::Deserialize;

use nexusops_shared::status::Task;
use nexusops_shared::time::Timestamp;

use super::classifier::{
    classify, parse_rate_limit_reset, parse_retry_after, IntegrationOutcomeClass,
};

/// Linear's `WorkflowState.type` — the closed 6-value lifecycle set (confirmed against the Linear
/// GraphQL API: `state: { type: { eq: "started" } }` etc.). The daemon-internal decode of the raw
/// API string; NOT the `agentSession.plan` status enum (a different, unrelated field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinearStateType {
    Triage,
    /// The conservative parked floor (also `parse_linear_state_type`'s unknown default) — matches the
    /// `Mergeability::Unknown` / `ReviewDecision::None` floor-as-`#[default]` convention so the gated
    /// Linear read client can put this in a `#[derive(Default)]` signals struct.
    #[default]
    Backlog,
    Unstarted,
    Started,
    Completed,
    Canceled,
}

/// Decode Linear's GraphQL `WorkflowState.type` string → `LinearStateType` (case-insensitive, the
/// edges-008 convention). None / null / unrecognized → `Backlog` — the conservative parked floor:
/// Linear's 6 types are a stable closed set, so an unknown is a parse-miss API anomaly that parks
/// (never fabricating human-attention from a miss).
pub fn parse_linear_state_type(state_type: Option<&str>) -> LinearStateType {
    match state_type.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("triage") => LinearStateType::Triage,
        Some("backlog") => LinearStateType::Backlog,
        Some("unstarted") => LinearStateType::Unstarted,
        Some("started") => LinearStateType::Started,
        Some("completed") => LinearStateType::Completed,
        Some("canceled") => LinearStateType::Canceled,
        _ => LinearStateType::Backlog,
    }
}

/// Map a Linear `WorkflowState.type` → the frozen §5.1 `Task` status (the external-task subset). The
/// daemon-defined mapping (§5.1/§9 arch-note): `Triage→NeedsClarification` (untriaged → needs a human
/// look), `Backlog→Queued`, `Unstarted→Ready` (todo), `Started→InProgress`, `Completed→Done` (work
/// complete — `Done` is non-terminal in the Task machine, so a completed issue can reopen; the terminal
/// set is Merged/Closed/Abandoned, which are PR/issue-close outcomes), `Canceled→Abandoned` (terminal
/// won't-do). **Exhaustive** (no `_`) — a new `LinearStateType` variant forces a reconcile here (LESSON-2).
pub fn derive_task_status_from_linear(state_type: LinearStateType) -> Task {
    match state_type {
        LinearStateType::Triage => Task::NeedsClarification,
        LinearStateType::Backlog => Task::Queued,
        LinearStateType::Unstarted => Task::Ready,
        LinearStateType::Started => Task::InProgress,
        LinearStateType::Completed => Task::Done,
        LinearStateType::Canceled => Task::Abandoned,
    }
}

// ---- the read client: model + extraction core + seam (edges-014) --------------------------------
//
// Mirrors the edges-009 GitHub read client (trait + fake + an injected-handle live client), split:
// this slice = the deterministic extraction + the seam; the real **`LinearGraphqlReadClient`**
// (reqwest POST + auth header + the GraphQL-errors-as-200 mapping via edges-003 `classify` + the HTTP
// dep) is **edges-015** — Linear has no octocrab-equivalent Rust client, so the live HTTP path is a
// heavier, separate slice. edges-015 sniffs the raw response body for `errors[]`/HTTP/auth → an
// `IntegrationOutcomeClass` before/around `extract_issue`, so no §17 fidelity is lost here.

/// The fetched Linear issue (daemon-internal). A Linear issue is an **external_task** (§5.1 R-8):
/// `status` is the derived §5.1 `Task` (via edges-013 — the single status authority), `state_name` is
/// the team's free-form workflow-state NAME (display only, NOT the status source), `assignee` is the
/// optional assignee display name (`None` = unassigned). The minimal MVP-chip set (identifier/title/
/// url/status) plus the richer secondary signals the gated §7.3 Task Inbox consumes — `description`,
/// `priority` (0–4 urgency), `team`, and `created_at`/`updated_at` (edges-018). Every richer field is
/// `Option` (absent / unparseable → `None`; tolerant of the minimal fixtures); `status` stays
/// `state.type`-derived regardless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearIssue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub url: String,
    pub status: Task,
    pub state_name: String,
    pub assignee: Option<String>,
    /// The issue body (markdown); `None` if absent.
    pub description: Option<String>,
    /// Linear's urgency 0–4 (0 = none … 4 = urgent); `None` if absent or out-of-range.
    pub priority: Option<u8>,
    /// The owning team's identity; `None` if absent.
    pub team: Option<LinearTeam>,
    /// Issue creation time (RFC3339-Z); `None` only for an absent/unparseable instant.
    pub created_at: Option<Timestamp>,
    /// Last-update time (RFC3339-Z); `None` only for an absent/unparseable instant.
    pub updated_at: Option<Timestamp>,
}

/// A Linear issue's owning team (daemon-internal). The nested identity (`key` is the human prefix,
/// e.g. `ENG`; `name` the display name) the gated §7.3 Task Inbox groups + labels by. Present iff the
/// issue carries a team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearTeam {
    pub id: String,
    pub name: String,
    pub key: String,
}

// Private wire structs — the Linear GraphQL issue response (`{ data: { issue: {...} } }`). Tolerant
// reads (serde ignores unselected fields). Kept PRIVATE: the GraphQL response shape is an internal
// detail, so the public seam is `extract_issue(&str)` (the raw body edges-015 already holds), never a
// public node type.
#[derive(Deserialize)]
struct IssueResponse {
    data: Option<IssueData>,
}

#[derive(Deserialize)]
struct IssueData {
    issue: Option<IssueNode>,
}

#[derive(Deserialize)]
struct IssueNode {
    id: String,
    identifier: String,
    title: String,
    url: String,
    state: StateNode,
    assignee: Option<AssigneeNode>,
    // edges-018 richer signals — tolerant (`Option` + `#[serde(default)]`) so the minimal fixtures
    // (which omit these) still deserialize → the field reads `None`.
    #[serde(default)]
    description: Option<String>,
    // Linear's `priority` is a `Float` (an integer 0–4); mapped to `Option<u8>` in `extract_issue`
    // (keeps `LinearIssue: Eq` — `f64` isn't `Eq`).
    #[serde(default)]
    priority: Option<f64>,
    #[serde(default)]
    team: Option<TeamNode>,
    #[serde(default, rename = "createdAt")]
    created_at: Option<String>,
    #[serde(default, rename = "updatedAt")]
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct StateNode {
    /// `type` is a Rust keyword — renamed from the GraphQL `WorkflowState.type` field.
    #[serde(rename = "type")]
    type_: String,
    name: String,
}

#[derive(Deserialize)]
struct AssigneeNode {
    name: String,
}

#[derive(Deserialize)]
struct TeamNode {
    id: String,
    name: String,
    key: String,
}

/// Parse a Linear GraphQL issue response (`{ data: { issue: {...} } }`) → `LinearIssue`, deriving the
/// §5.1 `Task` status from `state.type` via edges-013 (`derive_task_status_from_linear ∘
/// parse_linear_state_type` — the single status authority; the free-form `state.name` is preserved but
/// never the status source). Pure + total over the input string: a malformed body, an absent `data`,
/// or a null/absent `issue` all yield `None` (the gated edges-015 client classifies a GraphQL-errors
/// body separately); within a valid node, an absent assignee → `None` and an unknown `state.type` →
/// the edges-013 conservative floor. The richer secondary signals (edges-018) each map total/no-panic:
/// `description` passthrough, `priority` via `map_priority` (out-of-range → `None`), `team` → `LinearTeam`,
/// and the timestamps via `Timestamp::parse` (unparseable/absent → `None`, mirroring `CommitInfo.timestamp`).
pub fn extract_issue(response_json: &str) -> Option<LinearIssue> {
    let node = serde_json::from_str::<IssueResponse>(response_json)
        .ok()?
        .data?
        .issue?;
    let status = derive_task_status_from_linear(parse_linear_state_type(Some(&node.state.type_)));
    Some(LinearIssue {
        id: node.id,
        identifier: node.identifier,
        title: node.title,
        url: node.url,
        status,
        state_name: node.state.name,
        assignee: node.assignee.map(|a| a.name),
        description: node.description,
        priority: node.priority.and_then(map_priority),
        team: node.team.map(|t| LinearTeam {
            id: t.id,
            key: t.key,
            name: t.name,
        }),
        created_at: node
            .created_at
            .as_deref()
            .and_then(|s| Timestamp::parse(s).ok()),
        updated_at: node
            .updated_at
            .as_deref()
            .and_then(|s| Timestamp::parse(s).ok()),
    })
}

/// Map Linear's `priority` Float (an integer 0–4) → `u8`; out-of-range → `None` (total/no-panic — keeps
/// `LinearIssue.priority` an `Option<u8>` so `LinearIssue: Eq` holds). A JSON number is always finite
/// (no NaN/Inf), and Linear emits an integral 0–4, so the truncating `as i64` is exact for real data;
/// the range check rejects anything else.
fn map_priority(p: f64) -> Option<u8> {
    let n = p as i64; // float→int `as` saturates (no UB on a huge value); range-checked next
    (0..=4).contains(&n).then_some(n as u8)
}

/// A read-failure carrying the §17 `IntegrationOutcomeClass` (NOT the collapsed `DeliveryOutcome`) so
/// the gated `SyncFailed`/`auth_expired` path can branch on `AuthFailed` vs a payload `ClientError`.
/// Mirrors `GithubReadError` (the §17 forward-constraint).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("linear read failed [{class:?}]: {message}")]
pub struct LinearReadError {
    pub class: IntegrationOutcomeClass,
    pub message: String,
}

/// Read a Linear issue. `async` + dyn-compatible (via `async_trait`) so the gated `tasks`(external_task)
/// projector + the §7.3 Task Inbox can inject an `Arc<dyn LinearReadClient>` (the codebase's dyn-handler
/// idiom). The live impl (`LinearGraphqlReadClient`) is edges-015.
#[async_trait]
pub trait LinearReadClient: Send + Sync {
    async fn fetch_issue(&self, issue_id: &str) -> Result<LinearIssue, LinearReadError>;
}

/// Test double — returns a canned `Ok(LinearIssue)` / `Err(LinearReadError)`. The seam the gated
/// `tasks` projector + §7.3 Task Inbox consume in tests (the live HTTP fetch is the non-deterministic
/// edge, edges-015 — fake-covered per CLAUDE.md).
pub struct FakeLinearReadClient {
    result: Result<LinearIssue, LinearReadError>,
}

impl FakeLinearReadClient {
    pub fn new(result: Result<LinearIssue, LinearReadError>) -> Self {
        Self { result }
    }
}

#[async_trait]
impl LinearReadClient for FakeLinearReadClient {
    async fn fetch_issue(&self, _issue_id: &str) -> Result<LinearIssue, LinearReadError> {
        self.result.clone()
    }
}

// ---- the real Linear GraphQL network adapter: L1 pure core (edges-015) ---------------------------
//
// L1 = the deterministic core, test-first, NO new dep: `build_issue_query` (typed-variable query body)
// + `map_linear_response` (the response mapper layering the GraphQL `errors[].extensions.code` OVER
// edges-003's HTTP-status `classify`). L2 = the reqwest `LinearGraphqlReadClient` IO shell (next commit).

/// The GraphQL query for one issue. The id is a TYPED variable (`$id`) — never interpolated into the
/// query text (injection-safe, edges-010). The `id` arg accepts the `BLA-123` identifier or the UUID.
const ISSUE_QUERY: &str = "query Issue($id: String!) { issue(id: $id) { id identifier title url state { type name } assignee { id name } description priority team { id name key } createdAt updatedAt } }";

/// Build the Linear GraphQL request body `{ query, variables: { id } }` for `fetch_issue`. The
/// `issue_id` is a typed variable (injection-safe — never interpolated into the query, edges-010).
pub fn build_issue_query(issue_id: &str) -> serde_json::Value {
    serde_json::json!({
        "query": ISSUE_QUERY,
        "variables": { "id": issue_id },
    })
}

// Private wire structs for the GraphQL error envelope (`{ errors: [{ message, extensions: { code } }] }`).
// Tolerant reads (serde ignores the `data` key + any unselected fields). `#[serde(default)]` lets a body
// with no `errors` key deserialize to an empty Vec.
#[derive(Deserialize)]
struct GraphqlErrorEnvelope {
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: Option<String>,
    extensions: Option<GraphqlErrorExtensions>,
}

#[derive(Deserialize)]
struct GraphqlErrorExtensions {
    code: Option<String>,
}

/// Map a Linear GraphQL response → `Result<LinearIssue, LinearReadError>` (pure, total). Layers the
/// GraphQL `errors[].extensions.code` OVER edges-003's HTTP-status `classify`:
///   1. **`errors[]` present** → ALWAYS an error (even on HTTP 200 — Linear can partial-succeed with
///      `data` AND `errors`; check `errors[]` first). The primary code drives the class: `RATELIMITED`
///      → `RateLimited` (retryable — overrides the HTTP-400 Linear returns rate-limits as, which
///      `classify` alone would call a terminal `ClientError`); `AUTHENTICATION_ERROR`/`FORBIDDEN` (or
///      an HTTP 401/403 fold) → `AuthFailed`; any other code → the conservative fold (an error body is
///      NEVER `Success`).
///   2. **no `errors[]`, HTTP 2xx** → `extract_issue` (edges-014): `Some` → `Ok`; `None` (issue:null /
///      absent / malformed-on-2xx) → a TERMINAL `NotFound` (the issue isn't there — retrying won't
///      conjure it).
///   3. **no `errors[]`, non-2xx** → `classify(status)` (a 401 with no GraphQL body, a 5xx HTML page, …).
///
/// `rate_limit_reset` carries Linear's `X-RateLimit-Requests-Reset` (epoch-ms) when present; on a
/// `RATELIMITED` it takes precedence over `retry_after` (see `classify_graphql_error_code`).
pub fn map_linear_response(
    http_status: u16,
    retry_after: Option<&str>,
    rate_limit_reset: Option<&str>,
    body: &str,
) -> Result<LinearIssue, LinearReadError> {
    if let Ok(env) = serde_json::from_str::<GraphqlErrorEnvelope>(body) {
        if let Some(first) = env.errors.first() {
            let code = first.extensions.as_ref().and_then(|e| e.code.as_deref());
            let class =
                classify_graphql_error_code(code, http_status, retry_after, rate_limit_reset);
            let message = first
                .message
                .clone()
                .unwrap_or_else(|| "linear graphql error".to_owned());
            return Err(LinearReadError { class, message });
        }
    }
    if (200..=299).contains(&http_status) {
        return match extract_issue(body) {
            Some(issue) => Ok(issue),
            // Terminal: the issue isn't present (issue:null / absent / malformed 2xx body) → the
            // dedicated §17 `NotFound` terminal (edges-016; was a synthetic `ClientError{404}`).
            None => Err(LinearReadError {
                class: IntegrationOutcomeClass::NotFound,
                message: "linear issue not found or empty response".to_owned(),
            }),
        };
    }
    let class = classify(Some(http_status), retry_after, false);
    Err(LinearReadError {
        class,
        message: format!("linear http {http_status}"),
    })
}

/// Resolve a GraphQL `errors[].extensions.code` → `IntegrationOutcomeClass`, layered over the HTTP
/// status. RATELIMITED / auth recognized case-insensitively; an unrecognized code folds via `classify`,
/// but a `Success` fold (the HTTP-200+errors case) is forced to a conservative transient `ServerError`
/// — an error body is never a success. The auth set is `AUTHENTICATION_ERROR`/`FORBIDDEN` plus the
/// HTTP 401/403 fold (the backstop catches auth regardless of the GraphQL code string).
///
/// The RATELIMITED backoff hint resolves with **precedence (edges-016 Q2)**: Linear's epoch-ms
/// `X-RateLimit-Requests-Reset` (`rate_limit_reset`, → `Until`) WINS; it falls back to the standard
/// RFC-7231 `retry_after` only when the reset is absent. (The reset is Linear's authoritative,
/// Linear-specific signal — this is the Linear adapter path; the shared `classify` stays reset-unaware.)
fn classify_graphql_error_code(
    code: Option<&str>,
    http_status: u16,
    retry_after: Option<&str>,
    rate_limit_reset: Option<&str>,
) -> IntegrationOutcomeClass {
    // Normalize to lowercase (the edges-008/`parse_linear_state_type` convention) before matching.
    match code.map(|c| c.trim().to_ascii_lowercase()).as_deref() {
        Some("ratelimited") => IntegrationOutcomeClass::RateLimited {
            retry_after: parse_rate_limit_reset(rate_limit_reset)
                .or_else(|| parse_retry_after(retry_after)),
        },
        Some("authentication_error") | Some("forbidden") => IntegrationOutcomeClass::AuthFailed,
        _ => match classify(Some(http_status), retry_after, false) {
            IntegrationOutcomeClass::Success => IntegrationOutcomeClass::ServerError,
            other => other,
        },
    }
}

// ---- the real Linear GraphQL network adapter: L2 reqwest IO shell (edges-015) -------------------
//
// The thin non-deterministic edge: build_issue_query → reqwest POST → map_linear_response. Takes an
// INJECTED reqwest::Client + endpoint + api_key (auth bootstrap — keychain/OAuth 24h refresh, §9 — is
// deferred; this client never builds the key / reads the keychain). The live HTTP round-trip is
// fake-covered at the consumer level (edges-014's FakeLinearReadClient) per the edges-009 posture (no
// mock-server dep). §15 rule #5: the api_key reaches ONLY the Authorization header — never a message /
// log / row (map_linear_response + to_read_error are key-free by construction).

/// The real Linear read client over an INJECTED `reqwest::Client` + endpoint + api_key.
pub struct LinearGraphqlReadClient {
    http: reqwest::Client,
    endpoint: String,
    api_key: String,
}

impl LinearGraphqlReadClient {
    /// `endpoint` is normally `https://api.linear.app/graphql`; `api_key` is a personal API key (used as
    /// the `Authorization` value as-is) or an OAuth `Bearer <token>` — INJECTED by the caller (auth
    /// bootstrap deferred; this client never reads the keychain / mints the key).
    pub fn new(http: reqwest::Client, endpoint: String, api_key: String) -> Self {
        Self {
            http,
            endpoint,
            api_key,
        }
    }
}

#[async_trait]
impl LinearReadClient for LinearGraphqlReadClient {
    async fn fetch_issue(&self, issue_id: &str) -> Result<LinearIssue, LinearReadError> {
        let query = build_issue_query(issue_id);
        let response = self
            .http
            .post(&self.endpoint)
            .header("Authorization", &self.api_key) // §15: the key reaches ONLY this header
            .json(&query)
            .send()
            .await
            .map_err(|e| to_read_error(&e))?;
        let status = response.status().as_u16();
        // Two rate-limit backoff hints, in precedence order (edges-016): Linear's authoritative
        // X-RateLimit-Requests-Reset (epoch-ms → an absolute `Until` via `parse_rate_limit_reset`) wins;
        // the standard RFC-7231 Retry-After is the fallback. `map_linear_response`/`classify_graphql_error_code`
        // resolve the precedence in the pure, tested core; here we only read the two header values.
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let retry_after = header("Retry-After");
        let rate_limit_reset = header("X-RateLimit-Requests-Reset");
        let body = response.text().await.map_err(|e| to_read_error(&e))?;
        map_linear_response(
            status,
            retry_after.as_deref(),
            rate_limit_reset.as_deref(),
            &body,
        )
    }
}

/// Map a `reqwest::Error` (a transport/connection/decode failure — NOT an HTTP 4xx/5xx, which reqwest
/// returns as `Ok(response)`) → `LinearReadError`. **Key-free by construction**: takes ONLY the error,
/// never the api_key, so no key can reach the message (§15 rule #5; a reqwest error's Display carries
/// the URL/kind, never request headers). A transport failure is transient → `classify(None, None,
/// true)` = `TransportError` (writes queue, not fail; §17 line-452).
fn to_read_error(err: &reqwest::Error) -> LinearReadError {
    LinearReadError {
        class: classify(None, None, true),
        message: err.to_string(),
    }
}

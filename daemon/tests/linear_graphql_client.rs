//! P7.1 — the real Linear GraphQL read client (edges-015). Completes the Linear read vertical's
//! network side: live Linear → `LinearIssue` → §5.1 `Task`. All in-lane (the `tasks`(external_task)
//! projector + the `linear.*` executors + the keychain/OAuth auth bootstrap stay gated/deferred).
//!
//! L1 (this file's coverage) = the **pure deterministic core**, fixture-driven test-first, NO new dep:
//!   - `build_issue_query(issue_id)` — the GraphQL body with the id as a TYPED variable (injection-safe,
//!     edges-010 — never interpolated into the query text);
//!   - `map_linear_response(http_status, retry_after, body)` — the response mapper that layers the
//!     GraphQL `errors[].extensions.code` OVER edges-003's HTTP-status `classify`, then folds the happy
//!     body through edges-014's `extract_issue`. The load-bearing Linear quirk: a rate-limit is **HTTP
//!     400 + `RATELIMITED`** (NOT a clean 429), so the GraphQL code override flips the 400 that
//!     `classify` would call a terminal `ClientError` into a retryable `RateLimited`.
//!
//! L2 (the `LinearGraphqlReadClient` reqwest shell + the §15 no-key-leak + round-trip tests) lands in
//! the next commit (keeps the `reqwest` dep out of this pure-core commit). Fixtures are public GraphQL
//! shapes — never a real API key.

use nexusops_shared::status::Task;
use nexusopsd::integrations::classifier::{IntegrationOutcomeClass, RetryAfter};
use nexusopsd::integrations::linear::{build_issue_query, map_linear_response};

// ---- recorded public-shape Linear GraphQL response bodies (inline const) ------------------------

// HTTP 200 happy issue (state.name "In Review" ≠ type "started" — §B-#5 carries through extract_issue).
const STARTED_200: &str = r#"{"data":{"issue":{"id":"a1b2c3d4-0000-0000-0000-000000000001","identifier":"BLA-123","title":"Update documentation for API","url":"https://linear.app/acme/issue/BLA-123","state":{"type":"started","name":"In Review"},"assignee":{"id":"u_001","name":"Ada Lovelace"}}}}"#;
// HTTP 400 + RATELIMITED (the Linear quirk — rate-limit-as-400, not 429).
const RATELIMITED_400: &str =
    r#"{"errors":[{"message":"Rate limit exceeded","extensions":{"code":"RATELIMITED"}}]}"#;
// The auth-error body (code confirmed-best-effort = AUTHENTICATION_ERROR; matched case-insensitively).
const AUTH_ERROR: &str = r#"{"errors":[{"message":"Authentication required","extensions":{"code":"AUTHENTICATION_ERROR"}}]}"#;
// HTTP 200 + data.issue null (issue not found / not visible) — terminal, not retryable.
const ISSUE_NULL_200: &str = r#"{"data":{"issue":null}}"#;
// HTTP 500 with no GraphQL errors[] (a server/proxy error page).
const SERVER_500: &str = r#"<html><body>500 Internal Server Error</body></html>"#;
// errors[] with an unrecognized code at HTTP 200 (an error body is ALWAYS an error — never Ok/Success).
const UNKNOWN_CODE_200: &str =
    r#"{"errors":[{"message":"Something odd","extensions":{"code":"SOME_FUTURE_CODE"}}]}"#;

// ---- build_issue_query — typed-variable, injection-safe (edges-010) ------------------------------

#[test]
fn build_issue_query_uses_typed_variable() {
    // spec(§9/edges-010): the id is a TYPED GraphQL variable, never string-interpolated into the query
    // text (injection-safe) — variables.id carries it; the query selects `issue(id: $id)`.
    let body = build_issue_query("BLA-123");
    assert_eq!(body["variables"]["id"], serde_json::json!("BLA-123"));
    let query = body["query"].as_str().expect("query is a string");
    assert!(
        query.contains("issue(id: $id)"),
        "query selects the issue by the $id variable"
    );
    assert!(
        !query.contains("BLA-123"),
        "the id is NEVER interpolated into the query text"
    );
}

// ---- map_linear_response — the deterministic response mapper -------------------------------------

#[test]
fn map_started_200_extracts_in_progress() {
    // spec(§9/§5.1): the happy chain — HTTP 200 + a valid started issue → Ok(LinearIssue) with the
    // status derived through edges-014's extract_issue (state.type "started" → InProgress).
    let issue = map_linear_response(200, None, STARTED_200).expect("a valid issue maps to Ok");
    assert_eq!(issue.status, Task::InProgress);
    assert_eq!(issue.identifier, "BLA-123");
}

#[test]
fn map_ratelimited_400_is_retryable() {
    // spec(§17): the load-bearing Linear quirk — HTTP 400 + RATELIMITED is RETRYABLE; the GraphQL code
    // overrides the HTTP-400 that `classify` alone would call a terminal ClientError{400}.
    let err = map_linear_response(400, None, RATELIMITED_400).expect_err("RATELIMITED is an error");
    assert_eq!(
        err.class,
        IntegrationOutcomeClass::RateLimited { retry_after: None }
    );
    assert_ne!(
        err.class,
        IntegrationOutcomeClass::ClientError { status: 400 }
    );
}

#[test]
fn map_ratelimited_carries_retry_after() {
    // spec(§17): a backoff hint survives — a Retry-After header parses (via edges-003) into the
    // RateLimited class. (Linear's PRIMARY hint is X-RateLimit-Requests-Reset (epoch-ms) — a deferred
    // refinement flagged at Step-2.5; this pins the generic Retry-After capability.)
    let err = map_linear_response(400, Some("30"), RATELIMITED_400).expect_err("an error");
    assert_eq!(
        err.class,
        IntegrationOutcomeClass::RateLimited {
            retry_after: Some(RetryAfter::Delta(30))
        }
    );
}

#[test]
fn map_auth_error_is_authfailed() {
    // spec(§17): the auth-error code → AuthFailed — distinct/reachable for the gated auth_expired path
    // to branch on (the §17 forward-constraint), NOT collapsed into a generic client error.
    let err = map_linear_response(400, None, AUTH_ERROR).expect_err("auth error");
    assert_eq!(err.class, IntegrationOutcomeClass::AuthFailed);
}

#[test]
fn map_issue_null_is_terminal_error() {
    // spec(§8): a fetch for a nonexistent/invisible issue (HTTP 200 + data.issue null) is TERMINAL —
    // an Err with a non-retryable class (retrying won't conjure the issue), never Ok.
    let err = map_linear_response(200, None, ISSUE_NULL_200).expect_err("issue:null is terminal");
    assert_eq!(
        err.class,
        IntegrationOutcomeClass::ClientError { status: 404 }
    );
}

#[test]
fn map_server_500_is_retryable() {
    // spec(§17): HTTP 500 with no GraphQL errors[] → a transient ServerError (retry, never dead-letter).
    let err = map_linear_response(500, None, SERVER_500).expect_err("5xx is an error");
    assert_eq!(err.class, IntegrationOutcomeClass::ServerError);
}

#[test]
fn map_unknown_graphql_code_floors_conservatively() {
    // spec(§17): an errors[] body with an unrecognized code is ALWAYS an error — never Ok, never
    // silently Success. The 200+errors case (which `classify` alone would call Success) folds to the
    // conservative transient ServerError (Q2 default).
    let err =
        map_linear_response(200, None, UNKNOWN_CODE_200).expect_err("an error body is an error");
    assert_eq!(err.class, IntegrationOutcomeClass::ServerError);
}

#[test]
fn map_partial_success_errors_win_over_data() {
    // spec(§17): Linear can return data AND errors[] together (partial success) — errors[] is checked
    // FIRST, so a populated `data.issue` alongside a RATELIMITED error is still Err(RateLimited),
    // never Ok. Pins the Context7-flagged "check errors[] before assuming success" precedence.
    let body = r#"{"data":{"issue":{"id":"x","identifier":"BLA-1","title":"t","url":"u","state":{"type":"started","name":"In Progress"},"assignee":null}},"errors":[{"message":"Rate limit exceeded","extensions":{"code":"RATELIMITED"}}]}"#;
    let err = map_linear_response(200, None, body).expect_err("errors[] wins over a present data");
    assert_eq!(
        err.class,
        IntegrationOutcomeClass::RateLimited { retry_after: None }
    );
}

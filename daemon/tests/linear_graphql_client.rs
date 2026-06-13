//! P7.1 — the real Linear GraphQL read client (edges-015). Completes the Linear read vertical's
//! network side: live Linear → `LinearIssue` → §5.1 `Task`. All in-lane (the `tasks`(external_task)
//! projector + the `linear.*` executors + the keychain/OAuth auth bootstrap stay gated/deferred).
//!
//! L1 (this file's coverage) = the **pure deterministic core**, fixture-driven test-first, NO new dep:
//!   - `build_issue_query(issue_id)` — the GraphQL body with the id as a TYPED variable (injection-safe,
//!     edges-010 — never interpolated into the query text);
//!   - `map_linear_response(http_status, retry_after, rate_limit_reset, body)` — the response mapper that layers the
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
use nexusopsd::integrations::linear::{
    build_issue_query, map_linear_response, LinearGraphqlReadClient, LinearReadClient,
};

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

#[test]
fn build_issue_query_selects_richer_fields() {
    // spec(§9/edges-018): the query↔mapping sync guard. extract_issue reads description/priority/team/
    // createdAt/updatedAt off the response, but Linear only RETURNS a field the query REQUESTS — the
    // canned fixtures carry the fields regardless of the query, so they can't catch an omission (the
    // field would silently always-None in production). ISSUE_QUERY is a deterministic const → pin that
    // each richer field is actually selected, alongside the existing id/identifier/title/url/state/assignee.
    let body = build_issue_query("BLA-123");
    let query = body["query"].as_str().expect("query is a string");
    for field in [
        "description",
        "priority",
        "team { id name key }",
        "createdAt",
        "updatedAt",
    ] {
        assert!(
            query.contains(field),
            "ISSUE_QUERY must select `{field}` (else extract_issue's mapping is silently always-None)"
        );
    }
}

// ---- map_linear_response — the deterministic response mapper -------------------------------------

#[test]
fn map_started_200_extracts_in_progress() {
    // spec(§9/§5.1): the happy chain — HTTP 200 + a valid started issue → Ok(LinearIssue) with the
    // status derived through edges-014's extract_issue (state.type "started" → InProgress).
    let issue =
        map_linear_response(200, None, None, STARTED_200).expect("a valid issue maps to Ok");
    assert_eq!(issue.status, Task::InProgress);
    assert_eq!(issue.identifier, "BLA-123");
}

#[test]
fn map_ratelimited_400_is_retryable() {
    // spec(§17): the load-bearing Linear quirk — HTTP 400 + RATELIMITED is RETRYABLE; the GraphQL code
    // overrides the HTTP-400 that `classify` alone would call a terminal ClientError{400}.
    let err =
        map_linear_response(400, None, None, RATELIMITED_400).expect_err("RATELIMITED is an error");
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
    // RateLimited class. (Linear's PRIMARY hint, X-RateLimit-Requests-Reset (epoch-ms), is honored with
    // precedence by edges-016 — see the *_carries_epoch_ms_reset_winning_over_retry_after /
    // *_falls_back_to_retry_after pair; this pins the generic Retry-After capability via the fallback.)
    let err = map_linear_response(400, Some("30"), None, RATELIMITED_400).expect_err("an error");
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
    let err = map_linear_response(400, None, None, AUTH_ERROR).expect_err("auth error");
    assert_eq!(err.class, IntegrationOutcomeClass::AuthFailed);
}

#[test]
fn map_forbidden_code_is_authfailed() {
    // spec(§17): the second auth-terminal code arm — FORBIDDEN also → AuthFailed (the gated auth path
    // branches on AuthFailed regardless of which auth code Linear returns).
    let body = r#"{"errors":[{"message":"Forbidden","extensions":{"code":"FORBIDDEN"}}]}"#;
    let err = map_linear_response(400, None, None, body).expect_err("forbidden is an error");
    assert_eq!(err.class, IntegrationOutcomeClass::AuthFailed);
}

#[test]
fn map_http_401_no_body_is_authfailed() {
    // spec(§17): the HTTP-status backstop — a bare 401 with no GraphQL errors[] body folds via
    // edges-003's classify → AuthFailed (auth is caught regardless of a GraphQL code; robust for the
    // gated auth_expired path even if the GraphQL code string is absent/unrecognized).
    let err = map_linear_response(401, None, None, "Unauthorized").expect_err("401 is an error");
    assert_eq!(err.class, IntegrationOutcomeClass::AuthFailed);
}

#[test]
fn map_issue_null_is_not_found() {
    // spec(§8/§D): a fetch for a nonexistent/invisible issue (HTTP 200 + data.issue null) is TERMINAL
    // and now carries the dedicated NotFound class (was the synthetic ClientError{404}) — retrying won't
    // conjure the issue. NotFound→Terminal is pinned in integration_classifier::not_found_to_terminal.
    let err =
        map_linear_response(200, None, None, ISSUE_NULL_200).expect_err("issue:null is terminal");
    assert_eq!(err.class, IntegrationOutcomeClass::NotFound);
}

#[test]
fn map_server_500_is_retryable() {
    // spec(§17): HTTP 500 with no GraphQL errors[] → a transient ServerError (retry, never dead-letter).
    let err = map_linear_response(500, None, None, SERVER_500).expect_err("5xx is an error");
    assert_eq!(err.class, IntegrationOutcomeClass::ServerError);
}

#[test]
fn map_unknown_graphql_code_floors_conservatively() {
    // spec(§17): an errors[] body with an unrecognized code is ALWAYS an error — never Ok, never
    // silently Success. The 200+errors case (which `classify` alone would call Success) folds to the
    // conservative transient ServerError (Q2 default).
    let err = map_linear_response(200, None, None, UNKNOWN_CODE_200)
        .expect_err("an error body is an error");
    assert_eq!(err.class, IntegrationOutcomeClass::ServerError);
}

#[test]
fn map_partial_success_errors_win_over_data() {
    // spec(§17): Linear can return data AND errors[] together (partial success) — errors[] is checked
    // FIRST, so a populated `data.issue` alongside a RATELIMITED error is still Err(RateLimited),
    // never Ok. Pins the Context7-flagged "check errors[] before assuming success" precedence.
    let body = r#"{"data":{"issue":{"id":"x","identifier":"BLA-1","title":"t","url":"u","state":{"type":"started","name":"In Progress"},"assignee":null}},"errors":[{"message":"Rate limit exceeded","extensions":{"code":"RATELIMITED"}}]}"#;
    let err =
        map_linear_response(200, None, None, body).expect_err("errors[] wins over a present data");
    assert_eq!(
        err.class,
        IntegrationOutcomeClass::RateLimited { retry_after: None }
    );
}

#[test]
fn map_linear_response_ratelimited_carries_epoch_ms_reset_winning_over_retry_after() {
    // spec(§17/§D — Q2 precedence): Linear's authoritative reset (X-RateLimit-Requests-Reset, epoch-ms)
    // WINS over a present Retry-After. Both are threaded; the result must be the reset's ABSOLUTE Until,
    // NOT Delta(120). (A Retry-After of "120" alone would parse to Delta(120) — never an Until — so an
    // Until result unambiguously proves the reset path won the precedence.) The exact instant is pinned
    // in integration_classifier::parse_rate_limit_reset_epoch_ms.
    let err = map_linear_response(400, Some("120"), Some("1700000000500"), RATELIMITED_400)
        .expect_err("RATELIMITED is an error");
    let IntegrationOutcomeClass::RateLimited {
        retry_after: Some(RetryAfter::Until(_)),
    } = err.class
    else {
        panic!(
            "expected the reset's absolute Until to win over the present Retry-After, got {:?}",
            err.class
        );
    };
}

#[test]
fn map_linear_response_ratelimited_falls_back_to_retry_after() {
    // spec(§17/§D — Q2 precedence, the fallback branch): when the reset is ABSENT, the RateLimited hint
    // falls back to the standard Retry-After (`.or_else`). RATELIMITED + reset=None + Retry-After "120"
    // → Delta(120). Pins the middle precedence state a `.or_else`-dropping refactor would silently break.
    let err = map_linear_response(400, Some("120"), None, RATELIMITED_400)
        .expect_err("RATELIMITED is an error");
    assert_eq!(
        err.class,
        IntegrationOutcomeClass::RateLimited {
            retry_after: Some(RetryAfter::Delta(120))
        }
    );
}

#[test]
fn map_linear_response_ratelimited_no_reset_is_none() {
    // spec(§17/§D regression guard): with NO reset AND NO Retry-After, the RateLimited hint stays None
    // (never a bogus value) — preserves edges-015 behavior under the new threading.
    let err = map_linear_response(400, None, None, RATELIMITED_400).expect_err("an error");
    assert_eq!(
        err.class,
        IntegrationOutcomeClass::RateLimited { retry_after: None }
    );
}

// ---- L2: LinearGraphqlReadClient (the reqwest IO shell) -----------------------------------------
//
// The live HTTP round-trip (test 10, build_issue_query → POST → map_linear_response against a mock
// endpoint) is NOT tested here — not-tested-because: it is the non-deterministic edge (edges-009
// posture, no mock-server dep). Its two halves ARE pinned deterministically: build_issue_query +
// map_linear_response above (the request body + the response mapping), and the consumer fake-covers
// `fetch_issue` via edges-014's FakeLinearReadClient. The §15 no-key-leak invariant IS exercised
// against the real client below (a true transport-error path).

#[tokio::test]
async fn error_message_never_contains_api_key() {
    // spec(§15 rule #5): the injected api_key reaches ONLY the Authorization header — NEVER an error
    // message / log / row. Drive a real transport failure (a guaranteed-closed local port) and assert
    // the sentinel key is absent from LinearReadError.message. (map_linear_response + to_read_error are
    // structurally key-free — they never receive the key; this pins the client's own error arm too.)
    const SENTINEL_KEY: &str = "lin_api_SENTINEL_DO_NOT_LEAK_0123456789";
    // bind→drop to claim a port that is now closed → the POST gets connection-refused (deterministic).
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .build()
        .expect("build reqwest client");
    let client = LinearGraphqlReadClient::new(
        http,
        format!("http://127.0.0.1:{port}/graphql"),
        SENTINEL_KEY.to_owned(),
    );
    let err = client
        .fetch_issue("BLA-123")
        .await
        .expect_err("a closed port → transport error");
    assert_eq!(err.class, IntegrationOutcomeClass::TransportError);
    assert!(
        !err.message.contains(SENTINEL_KEY),
        "the api_key must NEVER appear in an error message (§15 rule #5)"
    );
}

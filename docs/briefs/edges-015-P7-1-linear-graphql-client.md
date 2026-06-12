# /tdd brief — linear_graphql_client

## Feature
The **real `LinearGraphqlReadClient`** — the network adapter that implements edges-014's `LinearReadClient` trait over a live Linear GraphQL POST. Splits into a **pure deterministic core (test-first, no new dep)** + a **thin reqwest IO shell (the non-deterministic edge, fake/mockito-covered)**:

- **L1 (test-first, no new dep):** `build_issue_query(issue_id) -> serde_json::Value` (the GraphQL query + **typed `variables.id`** — injection-safe, edges-010 lesson) and `map_linear_response(http_status, retry_after, body) -> Result<LinearIssue, LinearReadError>` — the response mapper that layers the GraphQL `errors[].extensions.code` **over** edges-003's HTTP-status `classify`, then folds the happy body through edges-014's `extract_issue`.
- **L2 (new dep `reqwest`; fake/mockito-covered):** `LinearGraphqlReadClient { http, endpoint, api_key }` impl `LinearReadClient::fetch_issue` — composes `build_issue_query` → reqwest POST (Authorization header) → `map_linear_response`, plus the transport-error arm. Auth bootstrap (keychain/OAuth) stays deferred — the client takes an **injected api_key + reqwest::Client** (mirrors edges-009's injected `octocrab::Octocrab` handle).

Completes the Linear read vertical's network side: **live Linear → `LinearIssue` → §5.1 `Task`**, all in-lane (the `tasks`(external_task) projector + the `linear.*` executors + the auth bootstrap stay gated/deferred).

## Use case + traceability
- **Task ID:** P7.1 (in-lane, Approach A — the Linear network adapter; the gated `tasks`(external_task) projector + `linear.link_issue`/`linear.create_issue` executors + the keychain/OAuth auth bootstrap stay deferred on the R1 daemon seam).
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (Linear = GraphQL; `POST https://api.linear.app/graphql`; auth = personal API key / OAuth token — **bootstrap deferred, injected key**; the integration-failure §17 contract), `§8` (the Linear intake flow `integrations(Linear read) → tasks(external_task rows)` — this is the read half), `§17` (integration-failure contract — transient 429/5xx + Linear's **400+`RATELIMITED`** quirk vs auth-terminal vs client-terminal; via edges-003's `classify` + the GraphQL-code override layer).
- **Widens phase scope because** `fetch_issue` returns edges-014's `LinearIssue` whose `status: Task` is the derived §5.1 value — this client **consumes** the frozen `Task` enum read-only (transitively, via edges-014's `extract_issue`) and does not modify it. (Same §5.1 waiver posture as edges-009/013/014.)
- **Related context:** edges-014 (`6ebdc4e` — the predecessor; `LinearReadClient` trait + `extract_issue(&str) -> Option<LinearIssue>` + `LinearIssue` + `LinearReadError{class, message}` + `FakeLinearReadClient`). edges-003 `classify(status, retry_after, transport_error)` / `IntegrationOutcomeClass` / `parse_retry_after` / `RetryAfter` (the §17 classifier this layers the GraphQL code over). edges-009 (`2eec8f2`) `OctocrabGithubReadClient` = the structural mirror (injected handle, auth deferred, error carries `IntegrationOutcomeClass`). edges-010 = the precedent for layering a GraphQL signal over a base classification (here: `errors[].code` over `classify`) + typed GraphQL variables (injection-safe). **Linear error semantics CONFIRMED (Context7 /websites/linear_app_developers):** rate-limits are returned as **HTTP 400** with `errors[].extensions.code == "RATELIMITED"` (NOT a clean 429); logical errors arrive in the `errors[]` array (a query can partially succeed → `data` AND `errors` both present — check `errors[]` before assuming success); server errors via HTTP status. **The exact auth-error `extensions.code` (likely `AUTHENTICATION_ERROR`) the impl confirms at Step-2.5** (spike it like edges-014's shape confirmation).

## Acceptance criteria (what "done" means)
- [ ] **`build_issue_query(issue_id: &str) -> serde_json::Value`** (pure): the GraphQL request body `{ query, variables: { id } }` for `issue(id:$id){ id identifier title url state{ type name } assignee{ id name } }` — the `id` passed as a **typed variable** (never string-interpolated into the query — injection-safe, edges-010).
- [ ] **`map_linear_response(http_status: u16, retry_after: Option<&str>, body: &str) -> Result<LinearIssue, LinearReadError>`** (pure, total — the deterministic core):
  - **`errors[]` present (non-empty)** → it is an error, NEVER `Ok` (even on HTTP 200): inspect the primary `extensions.code` —
    - `RATELIMITED` → `Err(class = RateLimited{ retry_after: parse_retry_after(retry_after) })` (**retryable — overrides the HTTP-400 that `classify` would call a terminal `ClientError`** — the load-bearing Linear quirk);
    - the auth code (confirmed at Step-2.5) → `Err(class = AuthFailed)`;
    - any other / unrecognized code → the conservative fold (Step-2.5 Q2: `classify(http_status)`, with the 200-floor decision).
  - **no `errors[]`, HTTP 2xx** → `extract_issue(body)`: `Some(issue) → Ok(issue)`; `None` (issue:null / absent / malformed-on-2xx) → `Err` with a **terminal** class (Step-2.5 Q3 — the issue isn't there; not retryable).
  - **no `errors[]`, non-2xx** (e.g. a 401 with no GraphQL body, a 5xx HTML page) → `classify(Some(http_status), retry_after, false)` → `Err(class, message)`.
- [ ] **`LinearGraphqlReadClient`** implements `LinearReadClient` over an **injected `reqwest::Client` + endpoint + api_key** (auth bootstrap deferred — never builds the key / reads the keychain): `fetch_issue` = `build_issue_query` → POST (`Authorization: <api_key>`, `Content-Type: application/json`) → read status + `Retry-After` header + body text → `map_linear_response`. A reqwest **transport error** (no response) → `Err(class = TransportError)` via `classify(None, None, true)`.
- [ ] **§15 no-key-leak (security-load-bearing):** the `api_key` is NEVER logged, NEVER placed in `LinearReadError.message`, and NEVER in any event/row (rule #5 — keychain-refs-only; the key is injected, held only for the header). `LinearReadError.message` carries Linear's response/error text only — pin a test that an error path's message does **not** contain the key.
- [ ] **Fixture pins (`map_linear_response`, deterministic):** at minimum — (a) HTTP 200 + a valid `{data:{issue:{state.type:"started"…}}}` → `Ok(LinearIssue{ status: InProgress })`; (b) **HTTP 400 + `{errors:[{extensions:{code:"RATELIMITED"}}]}` → `Err(RateLimited{..})`** (the quirk); (c) the auth-error body → `Err(AuthFailed)`; (d) HTTP 200 + `{data:{issue:null}}` → `Err(terminal)`; (e) HTTP 500 (no errors[]) → `Err(ServerError)`. (Fixtures are **public GraphQL shapes — never a real API key.**)
- [ ] All tests pass; `/preflight` clean (`cargo fmt --check && clippy -D warnings && check && test`). Cross-doc invariant: **none** (daemon-internal over edges-014/013/003 + the frozen `Task`; no `shared/` surface, no CONTRACT bump). **Adds the `reqwest` dep** (`default-features = false, features = ["json", "rustls-tls"]` — reuse the in-tree rustls, **no native-tls/OpenSSL**; Step-2.5 Q4) → `cargo audit` at the P7.1 `/phase-exit`.

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated `tasks`(external_task) projector + the `linear.*` executors + the auth-bootstrap slice.** `LinearGraphqlReadClient` is the concrete `LinearReadClient` the gated consumers inject (the gated `linear` executor arm + the `tasks` projector). Tested-but-unwired **by design** (Approach A) — Step 7.5 grep-confirms only the module + its tests reference the new symbols; `pub mod linear` already exports the trait to the gated consumers. Same posture as edges-009/014. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**Modified:**
- `daemon/src/integrations/linear.rs` (extend edges-014) — `build_issue_query`, `map_linear_response`, `LinearGraphqlReadClient`, the reqwest impl + the transport-error arm + the GraphQL-error-code override helper. `use` edges-014's `extract_issue`/`LinearIssue`/`LinearReadError`/`LinearReadClient` + edges-003's `classify`/`IntegrationOutcomeClass`/`parse_retry_after`. *(Split into `linear/mod.rs` + `linear/client.rs` only if it grows past readability — Step-2.5 Q5.)*
- `daemon/tests/linear_graphql_client.rs` (NEW) — the fixture-driven `build_issue_query` + `map_linear_response` tests (inline-const JSON, per edges-009/014 hygiene). *(Optional: a `mockito` round-trip `#[tokio::test]` for `fetch_issue` — Step-2.5 Q1.)*
- `daemon/Cargo.toml` + `Cargo.lock` — add `reqwest` (L2 only; the L1 pure functions need no new dep). *(Optional dev-dep: `mockito`/`wiremock` — only if Q1 takes the round-trip route.)*

If implementation needs files beyond this list, flag at Step 2.5.

## RED test outline (Step 2)
Tests in `daemon/tests/linear_graphql_client.rs`:

1. **`build_issue_query_uses_typed_variable`** — Asserts: `build_issue_query("BLA-123")` → a body whose `variables.id == "BLA-123"` and whose `query` string contains the `issue(id:$id)` field selection but **never** interpolates the id into the query text (injection-safe). Why: §9/edges-010 typed-variables.
2. **`map_started_200_extracts_in_progress`** — Asserts: HTTP 200 + valid started-issue body → `Ok(LinearIssue{ status: InProgress, identifier, … })`. Why: §9/§5.1 the happy chain through `extract_issue`.
3. **`map_ratelimited_400_is_retryable`** — Asserts: **HTTP 400** + `{errors:[{extensions:{code:"RATELIMITED"}}]}` → `Err(class = RateLimited{..})` (NOT `ClientError{400}`). Why: §17 — the Linear rate-limit-as-400 quirk; the GraphQL code overrides the HTTP-status classify.
4. **`map_ratelimited_carries_retry_after`** — Asserts: a `RATELIMITED` response with a `Retry-After` header → `RateLimited{ retry_after: Some(..) }` (parsed via edges-003). Why: §17 backoff hint survives.
5. **`map_auth_error_is_authfailed`** — Asserts: the auth-error body (confirmed code) → `Err(class = AuthFailed)` — distinct/reachable for the gated `auth_expired` path. Why: §17 forward-constraint (branch on `IntegrationOutcomeClass::AuthFailed`).
6. **`map_issue_null_is_terminal_error`** — Asserts: HTTP 200 + `{data:{issue:null}}` → `Err(terminal class)` (not `Ok`, not retryable). Why: §8 — a fetch for a nonexistent issue is terminal.
7. **`map_server_500_is_retryable`** — Asserts: HTTP 500 (no `errors[]`) → `Err(class = ServerError)`. Why: §17 transient.
8. **`map_unknown_graphql_code_floors_conservatively`** — Asserts: `errors[]` with an unrecognized `code` → the Q2 conservative fold (never `Ok`, never silently `Success`). Why: §17 — an error body is always an error.
9. **`error_message_never_contains_api_key`** — Asserts: an error built around a response that echoes a header does NOT surface the injected `api_key` in `LinearReadError.message`. Why: §15 rule #5 (keychain-refs-only; no secret in logs/messages).
10. *(optional, Q1)* **`fetch_issue_round_trip_via_mock`** (`#[tokio::test]`, `mockito`) — Asserts: the client POSTs the typed query to the mock endpoint with the Authorization header and maps a canned started-issue 200 → `Ok(LinearIssue)`. Why: pins request construction + the L1↔L2 composition. *(If skipped: `fetch_issue` is fake-covered at the consumer level per edges-009 posture — state the not-tested-because in a doc comment.)*

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — `build_issue_query`/`map_linear_response`/`LinearGraphqlReadClient` are daemon-internal over edges-014's types + the frozen `Task` (read-only).
- **Orchestrator doc rows to write hot (Step 9 routing):** none for `daemon/CLAUDE.md`/Appendix A. **Anticipated (integration-owned — FLAG, I route into the round PLAN-DELTA §B/§C):** a §9 arch-note (the Linear network adapter: injected key/auth-deferred; **the GraphQL-`errors[].code` override layered over `classify` — Linear rate-limit = 400+RATELIMITED, not 429**; transport→TransportError) + a C-list lesson (the errors-as-400/200 code-override mirrors edges-010's GraphQL-over-REST layering; reqwest with rustls-tls reuses the in-tree stack).
- **Shared-contract seam model touched?** **NO** — daemon-internal; no `shared/` surface → no schema-snapshot, no CONTRACT_VERSION.

## Things to flag at Step 2.5
1. **Test strategy for `fetch_issue` (the reqwest IO).** Pure `map_linear_response` + `build_issue_query` are required test-first (the deterministic core). The live reqwest call is the non-deterministic edge. **Default lean:** test the pure core exhaustively + fake-cover the client at the consumer level (edges-009 posture, no mock-server dep); add the `mockito` round-trip (test 10) **only** if you want request-construction coverage and accept the dev-dep. State your choice + the not-tested-because if you skip the round-trip.
2. **Unknown GraphQL `errors[].code` fold.** An `errors[]` body with a code that isn't RATELIMITED/auth — map via `classify(http_status)`, but **if `classify` returns `Success` (the 200+errors case), it must NOT stay `Success`** (an error body is an error). **Default lean:** fold an unrecognized-code 200 to a conservative transient `ServerError` (retry-safe) — and note the §17 taxonomy gap (a true terminal-non-auth-non-client variant is the deferred §17 refinement, PLAN-DELTA §D). Confirm or pick the terminal-floor instead.
3. **`issue:null` / extract-None on 2xx → which class?** A fetch for a nonexistent/invisible issue. **Default lean:** a terminal class (`ClientError{404}`-style or a dedicated message) — not retryable (retrying won't conjure the issue). Confirm the exact class/message.
4. **reqwest dep config (TLS-backend review — PLAN-DELTA §D carry).** rustls is already in-tree (via octocrab's hyper-rustls) → **Default lean: `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`** — reuse rustls, no native-tls/OpenSSL, no credential-auto-discovery features. Confirm the current reqwest version + that no extra default features creep in (`cargo tree` check).
5. **Module placement.** Extend `linear.rs`, or split to `linear/{mod,client}.rs` if it grows past readability. **Default lean:** extend `linear.rs` (mirrors `github.rs` holding both the derivation + the client) unless it crosses ~readability — your call at Step 1.

## Dependencies + sequencing
- **Depends on:** **edges-014** (the `LinearReadClient` trait + `extract_issue` + `LinearIssue` + `LinearReadError` this implements/composes — MUST be landed first); edges-003 `classify`/`IntegrationOutcomeClass`/`parse_retry_after`; edges-009 the structural mirror.
- **Blocks:** the gated `tasks`(external_task) projector + the §7.3 Task Inbox + the `linear.link_issue`/`linear.create_issue` executors + the auth-bootstrap slice (keychain/OAuth — the injected api_key's real source).

## Estimated commit count
**2** — **L1** (the pure `build_issue_query` + `map_linear_response`, test-first, NO new dep) then **L2** (the `LinearGraphqlReadClient` reqwest impl + the `reqwest` dep, fake/optional-mockito-covered). Could collapse to **1** if the impl judges the L2 shell thin enough to land atomically with L1; the L1/L2 split keeps the new dep out of the pure-core commit. ~120–180 lines + tests. **Security-touching (handles a credential) → `security-reviewer` runs at Step 8 (the `invariant` policy fires — §15 no-key-leak + the §17 auth-class surface).**

## Lessons-logged candidates anticipated
- **Convention candidate** — "The real Linear client layers the GraphQL `errors[].extensions.code` **over** edges-003's HTTP-status `classify`: Linear signals rate-limits as **HTTP 400 + `RATELIMITED`** (not 429), so the code override is load-bearing — mirrors edges-010's GraphQL-`reviewDecision`-over-REST layering. The pure `map_linear_response` (status+body→Result) + `build_issue_query` (typed-variable, injection-safe) are test-first; the reqwest POST is the thin non-deterministic shell (fake/mockito-covered). reqwest uses `rustls-tls` to reuse the in-tree stack (no native-tls). The injected api_key never leaves the Authorization header (§15 rule #5)." (extends the edges-009/014 line.)
- **Architecture-doc note candidate** — the Linear network-adapter boundary under §9 (injected key, auth deferred; the errors-as-400/200 code-override mapping).
- **Future TODO — next-brief working set** — the auth bootstrap (keychain/OAuth, 24h refresh — §9) as the injected-key's real source; richer `LinearIssue` fields (description/team/priority/timestamps — secondary signals); the §17 terminal-non-auth taxonomy variant (Q2).

## How to invoke
1. **Read this brief end-to-end** — esp. the Step-2.5 test-strategy (Q1) + the unknown-code fold (Q2) + the reqwest TLS config (Q4).
2. **Run `/tdd linear_graphql_client`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line (L1 pure core + L2 reqwest shell).
4. **Step 1 (files)** — confirm the `linear.rs` extension + the new test file + the L2-only `reqwest` dep.
5. **Step 2.5** — send the test-design write-up + the confirmed auth-error `extensions.code` + the Q1/Q2/Q3/Q4 decisions before GREEN; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` runs (credential-handling slice); fold its findings.
7. **Step 9** — surface cross-doc "none" + the anticipated §9 arch-note + C-list lesson + the auth-bootstrap/§17-taxonomy carries (integration-owned — flag, I route into the round PLAN-DELTA).

# edges-005 — R3 implementer close-out: the Linear read vertical's client + network adapter

- **Date:** 2026-06-12
- **Track / area:** `edges` / `daemon` (worktree `NexusOps-edges`, branch `track/edges`)
- **Phase:** 7.1 (Integrations — Linear read vertical), in-lane / Approach A
- **Predecessor:** [edges-003](edges-003-2026-06-12-r2-github-read-chain-diff-backend-linear-opened.md) (R2 implementer close-out) — _(edges-004 = the orchestrator's R2 round seal in between)_
- **Successor:** _(next implementer session — TBD)_
- **Round:** R3 (fresh implementer after the 2nd context cycle; sealed at edges-015, the clean boundary — wholesale closeout, user exiting iTerm + restarting each track)

## Why this session existed

R3 opened the **client + network side** of the Linear read vertical that R2's edges-013 issue-state derivation foundation set up. Two in-lane slices: edges-014 (the read-client core + seam) then edges-015 (the real GraphQL network adapter over reqwest). Mirrors the edges-009/010 GitHub PR read client, applied to Linear. All wiring stays deferred (Approach A — the daemon registration seam + event types aren't delivered yet).

## What was built

### edges-014 — Linear read-client core + seam (`6ebdc4e`)
**Files modified:**
- `daemon/src/integrations/linear.rs` — extended edges-013 with: `LinearIssue` (daemon-internal external_task model: id/identifier/title/url/`status: Task`/state_name/`assignee: Option<String>`), private GraphQL wire structs (`{data:{issue:{…}}}`), `extract_issue(&str) -> Option<LinearIssue>` (folds deserialize → maps the node → derives §5.1 `Task` via edges-013), `LinearReadError{class: IntegrationOutcomeClass, message}`, the `LinearReadClient` async trait, `FakeLinearReadClient`.

**Files created:**
- `daemon/tests/linear_read_client.rs` — 8 integration tests (started→InProgress, completed→Done, missing-assignee→None, identity-fields + §B-#5 status-from-type-not-name, unknown-state→floor, absent/null/errors[]/non-JSON→None, fake Ok/Err).

### edges-015 — real `LinearGraphqlReadClient` over reqwest (L1 `7445ae7` + L2 `581fa61`)
**Files modified:**
- `daemon/src/integrations/linear.rs` — **L1:** `build_issue_query(issue_id) -> serde_json::Value` (typed `$id` variable, injection-safe — edges-010), `map_linear_response(http_status, retry_after, body) -> Result<LinearIssue, LinearReadError>` (layers the GraphQL `errors[].extensions.code` OVER edges-003's `classify`), private GraphQL error wire structs, `classify_graphql_error_code` (lowercase-normalized). **L2:** `LinearGraphqlReadClient{http, endpoint, api_key}` impl `LinearReadClient::fetch_issue` (build_issue_query → reqwest POST + Authorization header → map_linear_response), key-free `to_read_error`.
- `daemon/Cargo.toml` — `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }` in `[dependencies]` + `[dev-dependencies]` (the dev mirror lets the §15 test construct a `Client`).
- `Cargo.lock` — reqwest + transitive deps (rustls reused; no native-tls/openssl).

**Files created:**
- `daemon/tests/linear_graphql_client.rs` — 12 integration tests: L1 (build_issue_query typed-var; map_* for started/RATELIMITED-400/retry-after/auth/issue-null/500/unknown-code/partial-success-errors-win) + L2 (§15 no-key-leak via a closed-port transport error; FORBIDDEN code; bare-401 HTTP backstop).

**Suite:** daemon **381/0** (369 inherited + 9 L1 + 3 L2, plus edges-014's 8 already in the 369... see note¹). fmt clean · clippy `-D warnings` clean · cargo check clean.

> ¹ Count bookkeeping: edges-014's 8 tests took the daemon suite 369→ (the 361 R2 + edges-014). edges-015 L1 added 9 (→378), L2 added 3 (→381). All daemon-package counts (cwd `daemon/`); a worktree-root `cargo test` reports 433 = daemon 381 + the shared crate's 54.

## Decisions made

- **edges-014 signature = Form A** (`extract_issue(&str) -> Option<LinearIssue>` + PRIVATE wire structs) over the brief body's literal `extract_issue(node) -> LinearIssue`. Rationale: "private wire structs" means a test can't name a node type, so the public entry takes the raw `{data:{issue}}` JSON and folds deserialize (None on malformed/absent); Linear has no octocrab-equivalent public model; edges-015's HTTP body IS a `&str`. (Orch-ratified at Step 2.5.)
- **§B-#5 independence pinned** — status derives from `state.type` (the closed 6-value set), NEVER the free-form `state.name`; pinned on one fixture (`name="In Review"` ≠ `type="started"`, both asserted). (Step-2.5 TWEAK.)
- **edges-015 errors-over-classify layering** — Linear signals rate-limits as **HTTP 400 + `errors[].extensions.code == "RATELIMITED"`** (NOT 429), so `map_linear_response` checks `errors[]` first (always an error, even on 200) and the code override flips the 400 that bare `classify` would call terminal `ClientError` into a retryable `RateLimited`. Mirrors edges-010's GraphQL-over-REST layering.
- **Auth recognition** = `AUTHENTICATION_ERROR`/`FORBIDDEN` (case-insensitive) + the HTTP 401/403→classify backstop (catches auth regardless of the GraphQL code string). The exact auth code is best-effort (Context7 pins RATELIMITED firmly but not the auth code); confirm-against-live deferred to auth-bootstrap.
- **§15 no-key-leak** structurally guaranteed — the injected `api_key` reaches ONLY the `Authorization` header; `map_linear_response` + `to_read_error` are signature-key-free; no `Debug` on the key-holding struct. Pinned by a closed-port TransportError test asserting the key is absent from the message. (security-reviewer CLEAN.)
- **reqwest = rustls-tls, default-features off** — reuses the in-tree rustls (hyper-rustls via octocrab); no native-tls/OpenSSL, no credential/proxy auto-discovery; `cargo tree` verified.
- **L2 passes `retry_after = None`** — Linear's rate-limit hint is `X-RateLimit-Requests-Reset` (epoch-ms), which edges-003's `parse_retry_after` would misread as `Delta(seconds)` (≈ infinite backoff); epoch-ms-aware parsing is a deferred §17 refinement. The `RateLimited` class is correct meanwhile.
- **issue:null → terminal `ClientError{404}`** (synthetic) — the §17 taxonomy has no dedicated not-found terminal; a `NotFound` variant is the deferred refinement.
- **Q1 test strategy = edges-009 posture** (no mock-server dep) — the live POST is fake-covered at the consumer level; build_issue_query + map_linear_response are tested in isolation; the §15 test drives a real transport error. Round-trip (test 10) skipped → not-tested-because doc note.

## Decisions explicitly NOT made (deferred)

- The real auth bootstrap (keychain/OAuth 24h refresh, §9) — the injected api_key's real source; + the auth-code confirm-against-live. → future slice.
- `X-RateLimit-Requests-Reset` (epoch-ms) backoff-hint parsing + threading the real header. → §17 refinement.
- A dedicated `NotFound` terminal class in the §17 `IntegrationOutcomeClass` taxonomy. → §17 refinement.
- Richer `LinearIssue` fields (description/team/priority/timestamps). → later.
- A `test-support` cargo feature to gate `FakeLinearReadClient`/`FakeGithubReadClient` out of the release binary (they ship ungated because integration tests link the lib as an external crate; cf. LESSON §21). → cross-cutting hardening.

## TDD compliance

**Clean.** Every slice was RED-first:
- edges-014: 8 tests written first; RED confirmed (5 missing symbols); then GREEN. A `data:null`/`errors[]` strengthening was folded into an already-approved test (same code path; no RED/GREEN change).
- edges-015 L1: 9 tests written first; RED confirmed (`build_issue_query`/`map_linear_response` missing); then GREEN.
- edges-015 L2: the §15 no-key-leak test written first; RED confirmed (`LinearGraphqlReadClient` missing); then GREEN. The reqwest IO shell is the non-deterministic edge (exempt from strict unit test-first) — covered via the §15 transport test + the structural key-free argument + the consumer fake (the project's non-deterministic-coverage path).
- The 2 auth tests (FORBIDDEN code, bare-401 backstop) added at the Step-8 review **strengthen coverage of an approved-design behavior** (the auth-terminal path was test-first via `map_auth_error`/AUTHENTICATION_ERROR at Step 2.5); the additional auth representatives were pinned at review — coverage strengthening, not behavior-before-test.

## Cross-doc invariant audit

**No model field changes this session.** Both slices are daemon-internal over edges-014/013/003 + the frozen `Task` (consumed read-only); no `shared/` surface, no CONTRACT_VERSION bump. Both flagged `Cross-doc invariant change: NONE` at Step 9 (orch confirmed). Multi-track memory check: nothing to verify in `ARCHITECTURE.md`.

## Reachability

**All new symbols are tested-but-unwired BY DESIGN (Approach A)** — confirmed at each slice's Step 7.5:
- `extract_issue`/`LinearReadClient`/`FakeLinearReadClient`/`LinearReadError`/`LinearIssue` (edges-014) and `LinearGraphqlReadClient`/`build_issue_query`/`map_linear_response` (edges-015) have **no production caller** outside `linear.rs`; `pub mod linear` exports them. Within the module, `LinearGraphqlReadClient::fetch_issue` reaches `build_issue_query` + `map_linear_response` (L1 reached by L2).
- **Gated consumers (the wiring entry points, not in this round):** the real `LinearGraphqlReadClient` is the concrete `LinearReadClient` injected by the gated `tasks`(external_task) projector + the §7.3 Task Inbox + the `linear.link_issue`/`linear.create_issue` executors + the auth-bootstrap slice. Same posture as edges-009/014.

## Open follow-ups

**Step-9 items routed hot to the orchestrator's round PLAN-DELTA (orch writes; not from track/edges):**
- §B (§9 arch-notes): the Linear read-client boundary (edges-014 — injected key/auth-deferred; extract_issue derives §5.1 Task; real fetch=edges-015) + the Linear network-adapter boundary (edges-015 — the errors[].code-over-classify override, rate-limit=400+RATELIMITED, transport→TransportError, rustls-tls reuse).
- §C (C-list lessons): edges-014 thin-glue mirror (§26) + edges-015 errors-as-400/200 code-override mirrors edges-010's GraphQL-over-REST layering (§27).
- §D (carry-forward): the §17 epoch-ms `X-RateLimit-Requests-Reset` refinement · the §17 not-found taxonomy variant · the auth-bootstrap (keychain/OAuth) + auth-code confirm-against-live · richer LinearIssue fields · the `test-support` cargo-feature to gate the read-client fakes out of release · **reqwest `cargo audit` at the P7.1 `/phase-exit`** (new dep).

**Deferred code-quality lows (accepted in-slice):** the `url:"u"` minimal-fixture cosmetic; the `assignee { id name }` over-fetch (matches the brief query; `AssigneeNode` projects only `name`); the bind→drop port-race in the §15 test (brief-sanctioned posture, 2s-bounded, = edges-009).

**Next in-lane work (orch was scoping):** the registry + `integration_connections` migration slice (P5.1/P7.1 — projects/repositories registry + the keychain-backed connection rows).

## How to use what was built

The Linear read vertical is now complete on the read side: a gated consumer constructs `LinearGraphqlReadClient::new(reqwest_client, "https://api.linear.app/graphql".into(), api_key)` and calls `fetch_issue(issue_id).await` → `Result<LinearIssue, LinearReadError>`, where `LinearIssue.status` is the derived §5.1 `Task`. Errors carry the `IntegrationOutcomeClass` for the gated `auth_expired`/`SyncFailed` path. `FakeLinearReadClient` covers the trait in consumer tests.

# /tdd brief — linear_read_client

## Feature
The **Linear read client — deterministic core + seam.** A `LinearReadClient` trait + `FakeLinearReadClient` + the `LinearIssue` model + `extract_issue` (a Linear GraphQL issue-response → `LinearIssue`, deriving the §5.1 `Task` status via edges-013). The deterministic extraction is fixture-driven test-first; the **real reqwest/GraphQL fetch is DEFERRED to edges-015** (Linear has no ready-made Rust client — unlike octocrab — so the live HTTP path + its dep is a heavier, separate slice). Mirrors the edges-009 octocrab-client shape, split: this slice = the extraction + the seam; edges-015 = the network adapter.

## Use case + traceability
- **Task ID:** P7.1 (in-lane, Approach A — the Linear read client foundation; the real GraphQL fetch + the `tasks`(external_task) projector + the `linear.link_issue`/`linear.create_issue` executors stay gated/deferred)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (Linear = GraphQL; `@linear/sdk`/GraphQL; the integration-failure §17 contract), `§8` (the Linear intake flow: `integrations(Linear read) → tasks(external_task rows)`).
- **Widens phase scope because** `extract_issue` derives a §5.1 `Task` status (via edges-013) — it **consumes** the frozen `Task` enum read-only (does not modify it). (Widens to §5.1; same waiver posture as edges-009/013.)
- **Related context:** edges-013 (`d7a9458`) `LinearStateType`/`parse_linear_state_type`/`derive_task_status_from_linear` (the status derivation `extract_issue` calls). edges-009 (`2eec8f2`) the octocrab client = the structural mirror (trait + fake + injected-handle + `GithubReadError` carrying `IntegrationOutcomeClass`). edges-003 `classify`/`IntegrationOutcomeClass` (the error type the gated real client maps into). **Linear GraphQL confirmed (Context7):** `POST https://api.linear.app/graphql`, auth header `Authorization: <API_KEY>` (personal) / `Bearer <token>` (OAuth2); `issue(id){ id identifier title url state{ type name } assignee{ id name } }` → `{ "data": { "issue": {…} } }`; `state.type` ∈ the 6-value set (edges-013). The impl confirms the exact query/response against the Linear API at Step-2.5 (like the octocrab/git2 spikes).

## Acceptance criteria (what "done" means)
- [ ] `LinearIssue` (daemon-internal): the fetched-issue representation — at least `id`, `identifier`, `title`, `url`, `status: Task` (the derived §5.1 status), `state_name: String` (the team's custom workflow-state name), `assignee: Option<String>` (the field set is Step-2.5 Q3).
- [ ] `extract_issue(node) -> LinearIssue` plumbs the Linear GraphQL issue node → `LinearIssue`, deriving `status` via `derive_task_status_from_linear(parse_linear_state_type(Some(&node.state.type)))` (edges-013) — the single status authority. Pure, total (a `None`/absent assignee → `None`; an unknown state.type → the edges-013 floor).
- [ ] The Linear GraphQL response Deserialize structs (`{ data: { issue: { id, identifier, title, url, state: { type, name }, assignee? } } }`) — `#[serde(...)]` as needed; deserialize from recorded **public** Linear JSON (never a real key).
- [ ] **Fixture pins:** a recorded Linear issue response deserializes + extracts to the expected `LinearIssue` — at minimum: (a) a `state.type="started"` issue → `status == Task::InProgress`; (b) a `state.type="completed"` issue → `Task::Done`; (c) a missing assignee → `assignee: None`.
- [ ] `LinearReadClient` trait: async `fetch_issue(&self, issue_id) -> Result<LinearIssue, LinearReadError>` — the seam the gated `tasks`(external_task) projector + the §7.3 Task Inbox consume.
- [ ] `FakeLinearReadClient` implements the trait returning canned `Ok(LinearIssue)` / `Err(LinearReadError)`.
- [ ] `LinearReadError { class: IntegrationOutcomeClass, message: String }` (mirrors `GithubReadError` — the §17 forward-constraint: the error carries the classifier's class, NOT a collapsed `DeliveryOutcome`, so the gated `auth_expired` path can branch on `AuthFailed`).
- [ ] **The real `LinearGraphqlReadClient` (reqwest POST + auth + the GraphQL-errors-as-200 mapping via edges-003 `classify`) is NOT in this slice** — named edges-015 (the network adapter; adds the HTTP/GraphQL dep). State it in a doc comment.
- [ ] All tests pass; `/preflight` clean. Cross-doc invariant: **none** (daemon-internal over edges-013 + the frozen `Task`). No new dep (the real reqwest client + its dep land in edges-015).

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated real client (edges-015) + the `tasks`(external_task) projector + the `linear.*` executors.** The trait + `extract_issue` are consumed by (a) edges-015's `LinearGraphqlReadClient` (fetch → `extract_issue`) and (b) the gated `tasks` projector + §7.3 Task Inbox. Tested-but-unwired **by design** (Approach A) — Step 7.5 grep-confirms only the module + test + `FakeLinearReadClient` reference the new symbols. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**Modified:**
- `daemon/src/integrations/linear.rs` (extend edges-013) — `LinearIssue`, the response Deserialize structs, `extract_issue`, `LinearReadClient` trait, `FakeLinearReadClient`, `LinearReadError`. `use` the edges-013 derive + edges-003 `IntegrationOutcomeClass`.
- `daemon/tests/linear_read_client.rs` (NEW) — the fixture-driven `extract_issue` tests + the fake/trait tests (inline-const Linear JSON, per the edges-009 hygiene).
- *(maybe)* `daemon/Cargo.toml` — `async-trait` is already in (edges-009); confirm no new dep (the real reqwest client is edges-015).

If implementation needs files beyond this list, flag at Step 2.5.

## RED test outline (Step 2)
Tests in `daemon/tests/linear_read_client.rs`:

1. **`extract_started_issue_is_in_progress`** — Asserts: a `state.type="started"` fixture → `extract_issue` → `LinearIssue{ status: InProgress, identifier, title, url, … }`. Why: §9/§5.1 the response→issue→derive chain.
2. **`extract_completed_issue_is_done`** — Asserts: `state.type="completed"` → `status: Done`. Why: §5.1 terminal/complete via the derivation.
3. **`extract_issue_missing_assignee_is_none`** — Asserts: an issue with `assignee: null` → `assignee: None`. Why: total over the optional field.
4. **`extract_issue_preserves_identity_fields`** — Asserts: `id`/`identifier`/`title`/`url`/`state_name` map through verbatim. Why: §8 the external_task row fields.
5. **`extract_unknown_state_type_floors`** — Asserts: a `state.type="weird"` fixture → `status` = the edges-013 floor (Backlog→Queued). Why: the conservative floor survives the issue extraction.
6. **`fake_client_returns_canned_issue`** (`#[tokio::test]`) — Asserts: `FakeLinearReadClient::new(Ok(issue)).fetch_issue(..).await == Ok(issue)`. Why: the seam the gated projector consumes.
7. **`fake_client_returns_canned_error`** (`#[tokio::test]`) — Asserts: a fake `Err(LinearReadError{class: AuthFailed,…})` surfaces it. Why: the §17 forward-constraint (the error carries `IntegrationOutcomeClass`, AuthFailed distinct).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — `LinearIssue`/`LinearReadError` are daemon-internal; the frozen `Task` is consumed read-only.
- **Orchestrator doc rows to write hot (Step 9 routing):** none for `daemon/CLAUDE.md`/Appendix A. **Anticipated (integration-owned — FLAG, I route):** a §9 arch-note (the Linear read client = injected-key handle [auth deferred] + `extract_issue` deriving the §5.1 Task; the real reqwest/GraphQL fetch is edges-015) + a C-list lesson (the Linear client mirrors the GitHub thin-glue pattern; the error carries `IntegrationOutcomeClass`).
- **Shared-contract seam model touched?** **NO** — daemon-internal over edges-013 + the frozen `Task`; no `shared/` surface → no schema-snapshot, no CONTRACT_VERSION.

## Things to flag at Step 2.5
1. **Confirm the Linear GraphQL issue query + response shape** against the Linear API (spike it like the octocrab/git2 ones): `issue(id){ id identifier title url state{ type name } assignee{ id name } }` → `{ data: { issue: {…} } }`; the `id` arg accepts the issue UUID or the `identifier` (`BLA-123`). **Default lean:** the response structs as above; derive `status` from `state.type`.
2. **Defer the real `LinearGraphqlReadClient`?** The real reqwest POST + auth header + the GraphQL-errors-as-200 mapping (edges-010 taught: a GraphQL logical error is HTTP-200 with an `errors[]` body — NOT a `classify`-able status) + the HTTP dep is heavier. **Default vote: DEFER to edges-015** (this slice = extraction + seam; edges-015 = the network adapter). Confirm `async-trait` is the only dep needed here (already in). If you judge a thin real client fits, flag it — but the GraphQL-error mapping + the reqwest dep argue for the split.
3. **`LinearIssue` field set.** Minimal = `id, identifier, title, url, status, state_name, assignee`. Add `description`/`team`/`priority`/timestamps? **Default vote: the minimal set** — the §7.3 Task Inbox external-task chip needs identifier/title/url/status; richer fields are a later refinement (secondary signals, like the GitHub-009 deferral). Keep it lean.
4. **`assignee` shape.** `Option<String>` (the assignee name) vs a struct `{ id, name }`. **Default vote: `Option<String>` (name)** for the MVP chip; promote to a struct if §7.3 needs the assignee id. (Linear's `assignee` is nullable — unassigned issues.)
5. **Module / trait placement.** Extend `integrations/linear.rs` (the trait + client live with the derivation — mirrors `github.rs` holding both). **Default vote: extend `linear.rs`.**

## Dependencies + sequencing
- **Depends on:** edges-013 (`d7a9458`) `parse_linear_state_type`/`derive_task_status_from_linear`; edges-003 `IntegrationOutcomeClass`; edges-009 the structural mirror.
- **Blocks:** edges-015 = the real `LinearGraphqlReadClient` (reqwest GraphQL fetch + auth + error mapping + the HTTP dep) — impls this trait via `extract_issue`. Then the gated `tasks`(external_task) projector + the §7.3 Task Inbox + the `linear.link_issue`/`linear.create_issue` executors.

## Estimated commit count
**1** — the deterministic Linear extraction + the seam in one module-extension; fixture-driven core, the live fetch deferred, no safety invariant, no cross-doc change, no new dep. ~90–130 lines + tests.

## Lessons-logged candidates anticipated
- **Convention candidate** — "The Linear read client mirrors the GitHub thin-glue pattern: `extract_issue` (Linear GraphQL response → `LinearIssue`, deriving the §5.1 `Task` via edges-013) is fixture-driven test-first; the live GraphQL fetch is a fake-covered edge (a separate slice, since Linear has no ready-made Rust client). The client takes an **injected key/handle** (auth bootstrap deferred); the §17 error carries `IntegrationOutcomeClass` (NOT a collapsed `DeliveryOutcome`)." (extends the edges-009/013 line).
- **Future TODO — next-brief working set** — edges-015 the real `LinearGraphqlReadClient` (reqwest GraphQL + auth + the GraphQL-errors-as-200 mapping + the HTTP dep); richer `LinearIssue` fields (secondary signals).
- **Architecture-doc note candidate** — the Linear read-client boundary (injected key, auth deferred; the real fetch in edges-015) under §9.

## How to invoke
1. **Read this brief end-to-end** — esp. the Step-2.5 Linear-query confirmation + the defer-the-real-client question.
2. **Run `/tdd linear_read_client`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (files)** — confirm the `linear.rs` extension + the new test file; confirm no new dep.
5. **Step 2.5** — send the test-design write-up + the confirmed Linear query/response shape + the defer-real-client decision before GREEN; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 9** — surface cross-doc "none" + the anticipated §9 arch-note + C-list lesson (integration-owned — flag, I route).

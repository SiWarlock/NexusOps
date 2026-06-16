# /tdd brief — linear_sync_executor

## Feature
The P7.1 **`LinearExecutor`** (`ExecutorKind::Linear`) — the second edges external-network mutator,
mirroring the landed `GithubExecutor` (edges-023). Handles `linear.link_issue` + `linear.create_issue`:
runs the Linear GraphQL mutation via an injected async write-client seam, driven from the SYNC executor
trait over the **captured `tokio::runtime::Handle` + `block_on` + mandatory timeout** (edges-023 / LESSON
32). **Success → `ActionSucceeded` only (no Linear domain event — see Q1).** Terminal non-auth failure →
`LinearSyncFailed` via the landed `ExecutionOutcome::FailedWithEvents`.

## Use case + traceability
- **Task ID:** P7.1 (Wave-D slice 6 of `docs/planning/edges-R5-wiring-plan.md` — the last Wave-D mutator).
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (catalog / executor dispatch), `§17`
  (the integration-failure classifier → `*SyncFailed`), `§7.3` (the Linear Task Inbox read surface this
  write path complements) — all within P7's phase scope.
- **Widens phase scope because** `§15` (the in-txn redaction gate + the `reason` = structural-class-name
  discipline) is a cross-cutting invariant every event-emitting mutator respects; cited for traceability,
  not redefined (pinned elsewhere).
- **Related context:**
  - **edges-023 `GithubExecutor`** (`integrations/executor.rs`, `498bd21`) — the EXACT mirror: inner
    `CatalogExecutor` for precondition + delegation; captured-`Handle` `block_on` + `NETWORK_TIMEOUT` (30s)
    + the `with_timeout()` test ctor; `ExecutionOutcome::FailedWithEvents` for the failure+event; the
    `classify_*` → terminal-non-auth → `*SyncFailed` (structural `reason`) / AuthFailed → `Failed` /
    transient → `Failed` taxonomy. **Reuse all of it.**
  - **edges-014/015 `integrations/linear.rs`** — `LinearReadClient`/`LinearGraphqlReadClient`/
    `FakeLinearReadClient` (read = `fetch_issue`); `map_linear_response` (the GraphQL-errors-`extensions.code`-
    over-HTTP-`classify` mapper, §17) + `build_issue_query` (the **typed-variable, never-interpolated** GraphQL
    pattern — the injection-safe precedent the write mutations follow). The write client mirrors this shape.
  - **The catalog (authoritative):** `linear.link_issue` = **risk-2, NaturalResourceRef, requires_resource_refs
    true**; `linear.create_issue` = **risk-2, FromInputs, requires_resource_refs FALSE** (catalog.rs:215/223).
- **Standing requirement (LESSON 31/32):** fail-closed validate every required operand BEFORE the network
  call. Linear GraphQL uses typed variables (not interpolation) → injection-safe by construction (the
  `build_issue_query` precedent); the guard here = reject blank/absent required operands.

## Acceptance criteria (what "done" means)
- [ ] `LinearExecutor::execute` routes `linear.link_issue` + `linear.create_issue` to their arms; every
      other action delegates to the inner `CatalogExecutor` stub (no event).
- [ ] Each arm validates the catalog precondition FIRST: `link_issue` requires a resource_ref (→ `Failed`
      if absent); `create_issue` does NOT (requires_resource_refs=false — do not over-require).
- [ ] Reads operands from `req.inputs`: `link_issue` → `issue_id` (+ the link target per Q5); `create_issue`
      → `team_id`/`title`/`description?`. Required operands blank/absent → `Failed`, **GraphQL never sent**.
- [ ] **3a mechanism (reuse edges-023):** the async mutation runs via the captured `Handle` +
      `handle.block_on(tokio::time::timeout(NETWORK_TIMEOUT, …))` — NOT `Handle::current()`, NOT
      `spawn_blocking`; a timeout → `Failed` (structural reason). Tests are plain `#[test]` + a built
      `Runtime` handle (NOT `#[tokio::test]` — `block_on` inside a runtime context panics; the edges-023 pin).
- [ ] **Success** → `Succeeded { side_effect_applied: true, emitted_events: [] }` — **no Linear domain
      event** (the frozen contract has none; `ActionSucceeded` is the audit record — see Q1).
- [ ] **Terminal non-auth failure** (classifier `ClientError`/`NotFound`, incl. a GraphQL `errors[]` mapped
      to a terminal class) → `FailedWithEvents` emitting `LinearSyncFailed{ provider: Linear, reason:
      <structural class>, failed_at }`; `reason` carries NO raw API/GraphQL text (§15).
- [ ] **AuthFailed** → `Failed("auth_failed")`, no event (auth_expired deferred). **Transient**
      (`ServerError`/`RateLimited`/transport) → `Failed`, no event.
- [ ] `ExecutorKind::Linear` registered in `main.rs` with the live `LinearGraphqlWriteClient` (auth bootstrap
      deferred — injected handle, never reads the keychain here).
- [ ] All tests pass; `/preflight` clean; `security-reviewer` run (external mutator, INV-SEC-1).

## Wiring / entry point (Step 7.5)
**Production entry point:** register `ExecutorKind::Linear` → `LinearExecutor` in `main.rs` (alongside the
edges-019 Project + edges-020 Git + edges-023 Github registrations), with the captured `Handle`, the daemon
`SystemClock` (for `failed_at`), and `LinearGraphqlWriteClient`. Path: `submit_action` IPC → Gateway →
**approval** (risk-2) → `CatalogExecutor` dispatch by `ExecutorKind::Linear` → `LinearExecutor::execute_*`.
**No projector** (Linear success emits no domain event; the §7.3 Task Inbox reads via `fetch_issue`).

## Files expected to touch
**New:**
- `daemon/src/integrations/linear_write.rs` — `LinearWriteClient` (async_trait, `link_issue`/`create_issue`)
  + `LinkIssueArgs`/`CreateIssueArgs` + `LinearWriteError{class, message}` (mirror `LinearReadError`) +
  `FakeLinearWriteClient` (`#[cfg(feature="test-support")]`) + `LinearGraphqlWriteClient` (reqwest GraphQL).
- `daemon/tests/linear_executor.rs` — the executor tests.

**Modified:**
- `daemon/src/integrations/executor.rs` — add `LinearExecutor` next to `GithubExecutor` (reuse the shared
  `NETWORK_TIMEOUT` const + the `with_timeout()` ctor pattern + the failure-taxonomy helper if edges-023
  extracted one — Step-2.5 Q2).
- `daemon/src/integrations/linear.rs` — if `map_linear_response`/`classify` need `pub(crate)` for the write
  module (the `classify_octocrab_error → pub(crate)` precedent).
- `daemon/src/main.rs` — register `ExecutorKind::Linear`.
- `daemon/src/integrations/mod.rs` — export the new module.

(No `gateway/` edit — `FailedWithEvents` already landed in edges-023.) If implementation needs files beyond
this, **flag at Step 2.5**.

## RED test outline (Step 2 — `daemon/tests/linear_executor.rs`)
1. **`test_link_issue_invokes_write_client`** — Asserts: `linear.link_issue` calls `link_issue` with the
   inputs' `issue_id` (+ target). Why: §6.3 dispatch + op-param plumbing.
2. **`test_create_issue_invokes_write_client`** — Asserts: `linear.create_issue` calls `create_issue` with
   `team_id`/`title`/`description`. Why: §6.3 (the FromInputs arm).
3. **`test_link_issue_success_no_domain_event`** — Asserts: success → `Succeeded{ side_effect_applied:true,
   emitted_events: [] }` (NO `LinearSyncFailed`, NO other event). Why: Q1 — the contract has no Linear
   success event; `ActionSucceeded` is the record.
4. **`test_create_issue_success_no_domain_event`** — Asserts: same for create. Why: Q1.
5. **`test_link_issue_terminal_non_auth_emits_linear_sync_failed`** — Asserts: `ClientError`/`NotFound`
   (incl. a GraphQL `errors[]` terminal class) → `FailedWithEvents` emitting `LinearSyncFailed{Linear,
   reason, failed_at}`; `reason` carries NO raw API/GraphQL text [a leaked token/url/title absent]. Why: §17/§15.
6. **`test_linear_auth_failed_no_sync_event`** — Asserts: AuthFailed → `Failed("auth_failed")`, NO event. Why: §17 (auth_expired deferred).
7. **`test_linear_transient_no_sync_event`** — Asserts: ServerError/RateLimited/transport → `Failed`, NO
   event. Why: §17 (LinearSyncFailed = terminal-non-auth only).
8. **`test_link_issue_requires_resource_ref`** — Asserts: no resource_ref → `Failed`, client never called.
   Why: §6.3 (link_issue requires_resource_refs=true).
9. **`test_create_issue_no_resource_ref_ok`** — Asserts: `create_issue` with NO resource_ref proceeds (does
   NOT fail on the precondition; requires_resource_refs=false). Why: the catalog asymmetry — don't over-require.
10. **`test_linear_missing_inputs_failed_no_call`** — Asserts: blank `issue_id` (link) / blank
    `team_id`/`title` (create) → `Failed`, client never called. Why: fail-closed operand validation (LESSON 31/32).
11. **`test_linear_timeout_is_failed`** — Asserts: a never-resolving future → captured-Handle `block_on`
    timeout → `Failed` (structural, bounded). Why: the write-actor must not wedge (LESSON 32 mandatory timeout).
12. **`test_linear_executor_delegates_other_actions_to_stub`** — Asserts: a non-`linear.*` action → inner
    stub, no event. Why: §6.3 delegation precedent.
13. **`test_linear_executor_uses_captured_handle_not_current`** — Asserts: structural pin — the executor
    source uses `block_on`, NOT `Handle::current()`, NOT `spawn_blocking` (the edges-023 #14 idiom; a
    `Handle::current()` regression = a production write-actor panic). Why: the load-bearing 3a pin.
14. **`test_link_issue_e2e_via_submit_action_approve`** — Asserts: submit (risk-2) → AwaitingApproval (no
    event) → approve → `ActionSucceeded` (no `LinearSyncFailed`) via the REAL pipeline. Why: §6.3 reachability
    (the 3a `block_on` runs on the real execute call site — integration proof).

> **Non-deterministic edge — NOT unit-tested:** the `LinearGraphqlWriteClient` live HTTP round-trip
> (fake-covered, the `FakeLinearReadClient`/`OctocrabGithubReadClient` precedent; #13 pins the mechanism,
> #14 the e2e). Step 7.5 confirms the real client is reachable from `main.rs`.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes (shared/):** NONE — `LinearSyncFailed`/`Provider` frozen @ CONTRACT 0.26.0; no bump;
  no schema-snapshot test.
- **Daemon-internal contract:** NO new gateway extension — `FailedWithEvents` already landed (edges-023);
  this slice REUSES it. (If `map_linear_response`/`classify` go `pub(crate)`, note it — same-crate, benign.)
- **Orchestrator doc rows (held for the final merge — cross-track rule):** §6.3 note (`linear.link_issue`/
  `create_issue` live); §17 note (`LinearSyncFailed` emit path live); LESSON 32 already covers the pattern
  (this slice confirms it generalizes from github to linear — no new lesson). **Possible arch-doc flag (Q1):**
  the Linear-success-emits-no-event asymmetry — if §7.3/§8 implies a Linear write-event/projection is needed,
  that's a contract gap to ESCALATE (not improvise); if intentional, an arch note records the asymmetry.
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **No Linear success event (load-bearing — architecture-as-contract).** The frozen 11-event family has
   `LinearSyncFailed` but NO Linear success event (unlike github's `PullRequestSynced`). Options: (a) success
   emits NO domain event — only `ActionSucceeded` (the audit record); (b) escalate a contract gap if the
   architecture expects a Linear write-event/projection. **My default vote: (a) — no domain event.** The
   asymmetry is intentional: GitHub PRs have the `proj_pull_request` derived cache (`PullRequestSynced` feeds
   it); Linear issues are read on-demand via `fetch_issue` (§7.3 Task Inbox) — no write-event-fed projection
   in the MVP. **Do NOT improvise a new event** (LOCKED contract; a new event = a CONTRACT bump = cross-track
   escalation). Flagging it for the arch-note ledger; escalate ONLY if §7.3/§8 review shows a real gap.
2. **Reuse vs. extract the edges-023 shared machinery.** `NETWORK_TIMEOUT`/`with_timeout()`/the
   captured-`Handle` `block_on` wrapper/the failure-taxonomy mapping. My default vote: **reuse in place**
   (both executors in `integrations/executor.rs`); if edges-023 left the taxonomy inline, extract a tiny
   shared helper (`classify → outcome`) used by both — keep it pure + pinned. Avoid premature abstraction.
3. **`link_issue` semantics + operands.** What does `link_issue` link (issue ↔ session/PR/task)? The
   resource_ref = the NaturalResourceRef identity; `issue_id` from inputs. My default vote: the write client's
   `link_issue(issue_id, target)` takes the issue + the link target from inputs/resource_ref; keep the GraphQL
   mutation behind the seam (variables, never interpolation — the `build_issue_query` precedent). Pin the
   client-call contract, not Linear's exact mutation shape (fake-covered).
4. **`LinearWriteError` shape.** Mirror `LinearReadError{class: IntegrationOutcomeClass, message}` + reuse
   `map_linear_response`'s error layering (GraphQL `errors[].extensions.code` over HTTP `classify`). My
   default vote: **mirror + reuse** the existing mapper (make it `pub(crate)` if needed).
5. **`failed_at` source.** The injected daemon `Clock` (UTC-Z), like edges-023's `pr_checked_at`. My default
   vote: **yes** — inject `Box<dyn Clock>`.
6. **`NETWORK_TIMEOUT` reuse.** Share the 30s const with github (one const). My default vote: **reuse.**

## Dependencies + sequencing
- **Depends on:** edges-023 (`498bd21`) — the captured-`Handle`/`block_on`/timeout machinery + the
  `ExecutionOutcome::FailedWithEvents` extension + the executor-module pattern. The merged R1 `LinearSyncFailed`/
  `Provider` (0.26.0); the edges-014/015 Linear GraphQL adapter + `classify`/`map_linear_response`.
- **Blocks:** completes the Wave-D external mutators. Then: the MIGRATION_9-deferred Wave-C
  `integration_connections` + the P5.1 registry projector · `proj_pull_request` projector · the §7.2/
  subscribe-delta/live-read hardening · P5.4 bench · cargo audit · `/phase-exit 5`+`7`.

## Estimated commit count
**1.** A focused INV-SEC-1 external-network mutator (its OWN commit). `security-reviewer` REQUIRED (external
mutation + §15 `reason` discipline + the operand guard + the write-actor-thread `block_on`). Smaller than
edges-023 (no success-event derivation; `FailedWithEvents` already landed) — mostly the mirror + the Linear
GraphQL write seam.

## Lessons-logged candidates anticipated
- **Convention candidate** — confirms LESSON 32 (the external-network-mutator pattern) generalizes github→linear;
  no NEW lesson (the pattern is the same). Possibly a one-line LESSON-32 extension if the success-event
  asymmetry is worth recording.
- **Architecture-doc note candidate** — Linear write success emits no domain event (intentional asymmetry vs
  PR — Q1); `linear.link_issue`/`create_issue` live (§6.3/§17).
- **Future TODO — operational (SPREAD, reuse edges-023's)** — the write-actor execute-phase offload (a slow
  Linear call blocks the write-actor ≤NETWORK_TIMEOUT; bounded) + the Linear auth bootstrap + the deferred
  `auth_expired` variant. `last-consumer-slice:` as edges-023.

## How to invoke
1. **Read this brief end-to-end** — Q1 (no Linear success event) is the load-bearing design call; the rest
   mirror edges-023.
2. **Run `/tdd linear_sync_executor`.**
3. **Step 2.5** — test-design write-up + coverage map + answers to Q1-Q6. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 7.5** — confirm `ExecutorKind::Linear` is registered in `main.rs`.
5. **Step 8** — `security-reviewer` + `code-quality-reviewer` (external mutator → both).
6. **Step 9** — categorized flags: the Linear-success-asymmetry arch note, the LESSON-32 generalization, the
   reused SPREADs.

# /tdd brief — github_sync_executor

## Feature
The P7.1 **`GithubExecutor`** (`ExecutorKind::Github`) — the FIRST real edges external-network mutator.
Handles `github.create_pr` + `github.create_pr_draft`: creates the PR via an injected async octocrab
write-client seam, driven from the SYNC executor trait over a **captured `tokio::runtime::Handle`**
(`handle.block_on`, with a timeout), and emits `PullRequestSynced` (success) / `GithubSyncFailed`
(terminal non-auth failure) via the `EmittedEvent::Namespaced` bridge through the §15 gate.

## Use case + traceability
- **Task ID:** P7.1 (Wave-D slice 5 of `docs/planning/edges-R5-wiring-plan.md`).
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (catalog / executor dispatch), `§7.2`
  (the GitHub-authoritative `proj_pull_request` cache value), `§17` (the integration-failure classifier →
  `*SyncFailed`) — all within P7's phase scope.
- **Widens phase scope because** two cross-cutting invariants are load-bearing for any event-emitting
  mutator and this slice respects (not redefines) them: `§5.1` (the frozen 11-state `PullRequest` machine —
  reused verbatim as `PullRequestSynced.status` via `derive_pull_request_status`, no fork) and `§15` (the
  in-txn redaction gate every emit passes + the `reason` = structural-class-name discipline). Neither is
  a new contract; both are pinned elsewhere — cited here for traceability.
- **Related context:**
  - **edges-020/021 `GitExecutor`** (`git/executor.rs`) — the mirror pattern: an executor holding an
    inner `CatalogExecutor` for the `requires_resource_refs` precondition + delegation of the
    non-handled `<ns>.*` actions; registered in `main.rs`; emits via `EmittedEvent::Namespaced`.
  - **edges-009 `integrations/github.rs`** — the existing `GithubReadClient`/`OctocrabGithubReadClient`
    + `FakeGithubReadClient` + `GithubReadError{class: IntegrationOutcomeClass, message}` + the
    `extract_pr_signals` / classifier (`classify_octocrab_error`) — the WRITE client mirrors this shape.
  - **edges-004/006/010 `integrations/pull_request.rs`** — `derive_pull_request_status(&PullRequestSignals)
    -> PullRequest` (§5.1) — the executor reuses this to fill `PullRequestSynced.status`.
  - **The 3a finding (load-bearing, lead-flagged):** `execute()` runs on the **write-actor's dedicated
    `std::thread`** (`runtime/writer.rs:273-326` → `gateway/pipeline.rs:969`), which is NOT a tokio worker
    and has NO entered runtime. So `Handle::current()` PANICS there and `spawn_blocking` is awkward; the
    correct mechanism is a **captured `Handle` + `handle.block_on(...)`** (see Step-2.5 Q1). `#[tokio::main]`
    (multi-thread) → the reactor runs on the runtime's workers, so `block_on` from this non-worker thread
    completes I/O futures without panicking.
- **Standing requirement (edges-020 security HIGH → LESSON 31):** every external mutator guards against
  parameter injection BEFORE the call. octocrab is a typed API (no shell / no CLI arg parsing) so the
  leading-`-` CLI vector does NOT apply; the analogous guard here = **fail-closed non-empty/well-formed
  validation of every required operand** (owner/repo/head/base/title), reject before the network call.
  (The Linear GraphQL path already uses variables, not interpolation — injection-safe.)

## Acceptance criteria (what "done" means)
- [ ] `GithubExecutor::execute` routes `github.create_pr` + `github.create_pr_draft` to a `create_pr` arm
      (`draft` = true for `_draft`, false otherwise); every other action delegates to the inner
      `CatalogExecutor` stub (no event).
- [ ] The create arm validates the catalog `requires_resource_refs` precondition FIRST (the
      `GitExecutor`/`SessionExecutor` precedent — this path runs its own side effect, never reaching
      `inner.execute`'s validation).
- [ ] Reads `owner`/`repo`/`head`(branch)/`base`/`title`/`body?` from `req.inputs` (the operational params;
      the resource_ref is the repo IDENTITY for audit/policy — the `GitExecutor` precedent). Required
      operands fail-closed if absent/blank → `Failed`, **network call never invoked**.
- [ ] **3a mechanism:** the async `create_pull_request` runs via the captured `tokio::runtime::Handle`
      (`handle.block_on(...)`), NOT `Handle::current()`, NOT `spawn_blocking`. Wrapped in a
      `tokio::time::timeout(NETWORK_TIMEOUT, …)` — a timeout → `Failed` (structural reason), so an
      octocrab hang can never freeze the single write-actor indefinitely.
- [ ] **Success** → `Succeeded { side_effect_applied: true, emitted_events: [PullRequestSynced{ pr_number,
      status: derive_pull_request_status(&signals), branch, base, mergeable, checks_summary, pr_checked_at }] }`
      via `EmittedEvent::Namespaced { event_type: PullRequestSynced::EVENT_TYPE, payload_json }`, landing
      through the §15 gate ATOMIC with `ActionSucceeded`. `pr_checked_at` = the injected daemon Clock (UTC-Z).
      `side_effect_applied: true` — a real PR was created on GitHub (honest `ActionPartiallySucceeded` on a
      txn-B fault, LESSON 21).
- [ ] **Terminal non-auth failure** (classifier `ClientError`/`NotFound`) → emits `GithubSyncFailed{
      provider: Github, reason: <structural class-name>, failed_at }` AND the action terminates as a
      FAILURE (see Step-2.5 Q2 — the `FailedWithEvents` extension). `reason` is the classifier's structural
      class-name ONLY (`client_error` / `not_found`), **never raw API response text** (§15).
- [ ] **AuthFailed** → `Failed` with a structural `"auth_failed"` reason, **no `GithubSyncFailed`**
      (the `auth_expired` variant is DEFERRED — its 0.5b gate lifted but needs a §17/INV-SEC re-review).
- [ ] **Transient** (`ServerError`/`RateLimited`) → `Failed` (structural reason); **no `GithubSyncFailed`**
      (the contract says `GithubSyncFailed` is the TERMINAL non-auth class only).
- [ ] `ExecutorKind::Github` registered in `main.rs` with the live `OctocrabGithubWriteClient` (auth bootstrap
      stays deferred — the client takes an injected `octocrab::Octocrab`, never reads the keychain here).
- [ ] All tests pass; `/preflight` clean; `security-reviewer` run (INV-SEC-1 external mutator).

## Wiring / entry point (Step 7.5)
**Production entry point:** register `ExecutorKind::Github` → `GithubExecutor` in `main.rs` (alongside the
edges-019 `Project` + edges-020 `Git` registrations) with the captured `Handle` (`tokio::runtime::Handle::current()`
inside the async `run()`), the daemon `SystemClock`, and `OctocrabGithubWriteClient`. Path:
`submit_action` IPC → Gateway → **approval** (risk-3 for `create_pr`, risk-2 for `_draft`) →
`CatalogExecutor` dispatch by `ExecutorKind::Github` → `GithubExecutor::execute_create_pr`.
**Deferred:** the `proj_pull_request` read-model projector folding `PullRequestSynced` → the cache is a
SEPARATE follow-on slice (the `WorktreeCreated`→edges-022 `proj_worktree` precedent; the event is durable
+ replayable meanwhile) — `none for the read-model projection this slice`.

## Files expected to touch
**New:**
- `daemon/src/integrations/github_write.rs` (or a section of `github.rs` — Step-2.5 Q3) — the
  `GithubWriteClient` trait + `CreatePrArgs` + `CreatedPr` + `FakeGithubWriteClient` +
  `OctocrabGithubWriteClient` + `GithubWriteError`.
- `daemon/src/git/` … NO — the executor lives in `integrations/`: **`daemon/src/integrations/executor.rs`**
  — `GithubExecutor` (the `ActionExecutor` impl). (Confirm module placement at Step-2.5 Q3.)
- `daemon/tests/github_executor.rs` — the executor tests.

**Modified:**
- `daemon/src/gateway/executor.rs` — **(Step-2.5 Q2)** the additive `ExecutionOutcome::FailedWithEvents
  { detail, emitted_events }` variant (edges-owned bridge territory; daemon-internal; existing
  `Failed(String)` sites untouched) + extend `side_effect_applied()` (returns false for it).
- `daemon/src/gateway/pipeline.rs` — the txn-B outcome match gains a `FailedWithEvents` arm: record
  `ActionFailed` + append the emitted_events, ATOMIC (same §15-gated append the `Succeeded` arm uses).
- `daemon/src/main.rs` — register `ExecutorKind::Github`.
- `daemon/src/integrations/mod.rs` — export the new module(s).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2 — `daemon/tests/github_executor.rs`, + a few in the client module)
1. **`test_create_pr_invokes_write_client`** — Asserts: `github.create_pr` calls
   `GithubWriteClient::create_pull_request` with the inputs' owner/repo/head/base/title/body, `draft=false`.
   Why: §6.3 dispatch + operational-param plumbing.
2. **`test_create_pr_draft_sets_draft_true`** — Asserts: `github.create_pr_draft` → `draft=true`. Why: the
   two action types differ only by the draft flag.
3. **`test_create_pr_emits_pull_request_synced`** — Asserts: success → exactly one `PullRequestSynced` with
   `pr_number`/`branch`/`base` from the result + `status == derive_pull_request_status(signals)` +
   `pr_checked_at` = the (fake) Clock's UTC-Z stamp. Why: §7.2/§5.1 emission + status reuse.
4. **`test_create_pr_side_effect_applied_true`** — Asserts: `Succeeded { side_effect_applied: true }`.
   Why: a created PR is a durable external change → honest partial on a txn-B fault (LESSON 21).
5. **`test_create_pr_terminal_non_auth_emits_github_sync_failed`** — Asserts: a `ClientError`/`NotFound`
   write-client error → the action FAILS **and** emits `GithubSyncFailed{ provider: Github, reason:
   <structural class>, failed_at }`; `reason` contains NO raw API text. Why: §17/§15.
6. **`test_create_pr_auth_failed_no_sync_event`** — Asserts: `AuthFailed` → `Failed`("auth_failed"), NO
   `GithubSyncFailed`. Why: the `auth_expired` variant is deferred; non-auth-only this round.
7. **`test_create_pr_transient_no_sync_event`** — Asserts: `ServerError`/`RateLimited` → `Failed`, NO
   `GithubSyncFailed`. Why: `GithubSyncFailed` is the TERMINAL non-auth class only.
8. **`test_create_pr_missing_inputs_failed_no_call`** — Asserts: blank `owner`/`repo`/`head`/`base`/`title`
   → `Failed`, write-client never called. Why: fail-closed input guard (LESSON 31 analog).
9. **`test_create_pr_timeout_is_failed`** — Asserts: a write-client future that never resolves →
   `timeout` → `Failed` (structural reason), bounded. Why: the write-actor must never hang unbounded.
10. **`test_github_executor_delegates_other_actions_to_stub`** — Asserts: a non-`github.create_pr*` action
    delegates to the inner stub (no event). Why: the delegation contract (the `GitExecutor` precedent).
11. **`test_create_pr_e2e_via_submit_action_approve`** — Asserts: submit (risk-3) → AwaitingApproval (no
    event) → approve → `PullRequestSynced` persisted through the real pipeline. Why: approve-path
    reachability (the 3a `block_on` runs on the real write-actor thread here — the integration proof).
12. **`test_failed_with_events_records_action_failed_and_appends`** *(gateway test, `daemon/tests/`)* —
    Asserts: a `FailedWithEvents` outcome records `ActionFailed` **and** appends the emitted_events in the
    same txn-B; `side_effect_applied()==false`. Why: the Q2 extension's atomicity + honest-failure pin.

> **Non-deterministic edge — NOT unit-tested:** the `OctocrabGithubWriteClient` live HTTP round-trip
> (fake-covered, per CLAUDE.md + the `OctocrabGithubReadClient` precedent — only the test module +
> `FakeGithubWriteClient` reference the trait; Step 7.5 confirms the real client is reachable from `main.rs`).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes (shared/):** NONE — `PullRequestSynced`/`GithubSyncFailed`/`Provider` frozen @ CONTRACT
  0.26.0; no bump; **no schema-snapshot test** (no cross-track shared-contract model touched in `shared/`).
- **Daemon-internal contract:** the `ExecutionOutcome::FailedWithEvents` variant is a **daemon-internal
  gateway type** (NOT a `shared/` contract) — edges-owned bridge territory (lead-confirmed R5: edges owns
  the additive `gateway/executor.rs`+`pipeline.rs`+`request.rs` bridge edits as phase-exit integration).
  Additive (existing `Failed(String)` sites untouched). **Flag at Step 9** for the held-for-merge ledger.
- **Orchestrator doc rows (held for the final merge — cross-track rule):** §6.3 note (`github.create_pr*`
  live); §7.2/§17 note (the `PullRequestSynced`/`GithubSyncFailed` emit path live); a LESSON-32 candidate
  (the external-network-mutator pattern: write-actor-thread `block_on` + timeout + the §17→`*SyncFailed`
  emit + the param-injection-fail-closed adaptation).
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **The 3a async mechanism (LOAD-BEARING — lead-flagged highest-risk).** Options: (a) capture a
   `tokio::runtime::Handle` at construction (`main.rs` `run()` is async → `Handle::current()` works THERE)
   + `handle.block_on(timeout(fut))` in `execute()`; (b) `handle.spawn(fut)` + a `oneshot`/`std::mpsc`
   `blocking_recv()`. **My default vote: (a) — captured `Handle` + `block_on`.** The as-built `execute()`
   runs on the write-actor's dedicated `std::thread` (non-worker, no entered runtime) → `block_on` from
   there does NOT panic (the lead's "spawn_blocking + block_on" framing assumed a worker thread, which is
   NOT the as-built context — see Related context). **(b) is the fallback** if `block_on`-from-a-non-runtime-
   thread proves problematic for hyper I/O — pin whichever you choose RED-first; I review the asserted
   mechanism. Either way: the captured Handle (NOT `Handle::current()` in `execute()`) + a hard timeout.
2. **Emitting `GithubSyncFailed` on a FAILED action.** `ExecutionOutcome::Failed(String)` carries no
   `emitted_events`. Options: (a) **add an additive `ExecutionOutcome::FailedWithEvents { detail,
   emitted_events }` variant** (edges-owned bridge; one new txn-B pipeline arm records `ActionFailed` +
   appends atomically; existing `Failed(String)` sites untouched); (b) descope `GithubSyncFailed` to its own
   later slice (slice = success/`PullRequestSynced` only); (c) emit it on a `Succeeded{side_effect_applied:
   false}` (REJECT — dishonest: the action did not succeed). **My default vote: (a)** — keeps the action's
   failure honest (`ActionFailed` is still the terminal event) AND makes the §17 sync-failure event durable
   atomic; it's the edges-owned bridge extension done once (Linear reuses it). Confirm you're comfortable
   touching `gateway/{executor,pipeline}.rs` (lead-confirmed edges-owned).
3. **Module placement + the write-client seam.** `GithubExecutor` in `integrations/executor.rs`; the write
   client in `integrations/github_write.rs` (or fold into `github.rs`). The seam: `GithubWriteClient`
   (async, `async_trait`) with `create_pull_request(&CreatePrArgs) -> Result<CreatedPr, GithubWriteError>`,
   where `CreatedPr { pr_number: u64, signals: PullRequestSignals, branch, base }` is a DOMAIN type (the
   Fake returns a canned one — NOT an octocrab `PullRequest`, which is `#[non_exhaustive]`/unbuildable in
   tests; the `FakeGithubReadClient` precedent). `OctocrabGithubWriteClient` builds it via `extract_pr_signals`.
   My default vote: **new `integrations/executor.rs` + `integrations/github_write.rs`; domain `CreatedPr`;
   `GithubWriteError{class, message}` mirroring `GithubReadError`** (or reuse a shared `GithubError`).
4. **owner/repo source.** From `req.inputs` (mirrors `GitExecutor`'s operational-params-from-inputs) vs.
   parsed from the repo resource_ref. My default vote: **`req.inputs`** (the `GitExecutor` precedent;
   the resource_ref stays the audit/policy IDENTITY). Apply the §7.2/§15 redacted-operational-inputs
   MVP-accept (lead-confirmed for Wave-D: owner/repo/branch identifiers are low-entropy → survive
   redaction; flag back ONLY if a field is genuinely high-entropy — none here).
5. **`status` from the create response.** A just-created PR has no checks/reviews yet → `extract_pr_signals`
   on the create response yields `Open` (or `Draft` if draft). My default vote: **derive from the create
   response directly** (one round-trip; no follow-up fetch) — `mergeable`/`checks_summary` = `None`
   initially. A status-refresh sync is a later concern (the deferred `proj_pull_request` + a refresh slice).
6. **`NETWORK_TIMEOUT` value.** My default vote: a generous bound (e.g. **30s**) — enough for a slow API,
   short enough that a hung call can't wedge the write-actor for long. Confirm the value.

## Dependencies + sequencing
- **Depends on:** the merged R1 contract (`PullRequestSynced`/`GithubSyncFailed`/`Provider` @ 0.26.0); the
  `EmittedEvent::Namespaced` bridge (edges-019); the `GithubReadClient`/classifier/`derive_pull_request_status`
  (edges-009/004/006/010); the `GitExecutor` registration pattern (edges-020).
- **Blocks:** the linear sync executor (next slice — reuses the 3a mechanism + the `FailedWithEvents`
  extension + the param-injection adaptation); the `proj_pull_request` projector (the read vertical close);
  `/phase-exit 7`.

## Estimated commit count
**1.** A focused INV-SEC-1 external-network mutator (its OWN commit per the lead's "each external mutator is
its own auditable, security-reviewed slice"). `security-reviewer` REQUIRED (external mutation + §15 `reason`
discipline + the param-injection guard + the new write-actor-thread `block_on` surface). The `FailedWithEvents`
gateway extension rides this slice (it's the mechanism the emission needs; not separately bisectable).

## Lessons-logged candidates anticipated
- **Convention candidate (LESSON 32?)** — the external-network-mutator pattern: a SYNC `ActionExecutor`
  driving an async client via a captured `Handle::block_on` + a hard timeout on the write-actor thread; the
  §17 classifier → `*SyncFailed` (terminal-non-auth-only, structural `reason` §15); auth/transient → plain
  `Failed`; the param-injection guard adapts to "fail-closed validate required operands" for a typed API.
- **Architecture-doc note candidate** — `ExecutionOutcome::FailedWithEvents` (a failed action may emit
  structured observation events atomic with `ActionFailed`); the github sync path is live (§6.3/§7.2/§17).
- **Future TODO — operational (SPREAD)** — a slow external executor blocks the single write-actor for the
  network duration (same class as the git-CLI subprocess; bounded by the timeout). Hardening: move slow
  external executors off the write-actor thread (an execute-phase worker pool). `last-consumer-slice: a
  gateway execute-phase-offload hardening slice`.

## How to invoke
1. **Read this brief end-to-end** — the 6 Step-2.5 questions have default votes; Q1 (the 3a mechanism) +
   Q2 (the `FailedWithEvents` extension) are the load-bearing ones — pin them RED-first.
2. **Run `/tdd github_sync_executor`.**
3. **Step 2.5** — test-design write-up + coverage map + answers to Q1-Q6. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 7.5** — confirm `ExecutorKind::Github` is registered in `main.rs` (reachable, not test-only).
5. **Step 8** — `security-reviewer` + `code-quality-reviewer` (external mutator → both run).
6. **Step 9** — categorized flags: the `FailedWithEvents` daemon-internal contract extension (held-for-merge
   ledger), the LESSON-32 candidate, the §6.3/§7.2/§17 arch notes, the write-actor-offload SPREAD.

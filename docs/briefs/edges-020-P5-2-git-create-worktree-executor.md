# /tdd brief — git_create_worktree_executor

## Feature
The first real edges **FS/git mutation** Action: a `GitExecutor` (`ExecutorKind::Git`) that handles
`git.create_worktree` by shelling out to the **git CLI** (`git worktree add`, forbidden #6 — never git2 for
mutations), mints a `WorktreeId`, and emits `WorktreeCreated` through the in-txn §15 gate via the
`EmittedEvent::Namespaced` bridge (from edges-019). Registers `ExecutorKind::Git` into the seam; the other
`git.*` action types (`git.status`/`git.diff`/`git.create_branch`) delegate to the inner stub for now.

## Use case + traceability
- **Task ID:** P5.2
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (catalog / executor dispatch), `§7.2`
  (the worktree read model the event feeds), `§5.1` (Worktree status machine), `§15` (the in-txn redaction
  gate the emission rides). Forbidden #6 (git CLI for mutations, never git2) is the load-bearing daemon rule.
- **Related context:** `docs/planning/edges-R5-wiring-plan.md` (Wave-B); the `EmittedEvent::Namespaced`
  bridge + the in-txn emission pattern landed in edges-019 (`c739278`). git2 read backend is in-lane
  (`daemon/src/git/reads.rs`). The R1 packet's design: `GitExecutor::new(cli_runner)` — an injected git-CLI
  seam. `WorktreeCreated { worktree_id, path, branch_name, base_branch? }` is frozen (CONTRACT 0.26.0).
  **Scope note:** `git.status`/`git.diff` need NO executor — no consumer submits them; the reads are served
  via the read path (`get_projection(Worktree)→proj_worktree` + the in-lane diff backend). They dispatch to
  `ExecutorKind::Git` too, so `GitExecutor` delegates them (+ `create_branch`, → edges-021) to the inner stub.

## Acceptance criteria (what "done" means)
- [ ] `GitExecutor` implements `ActionExecutor`; `execute` for `git.create_worktree` validates the catalog
      `requires_resource_refs` precondition, reads the operational params from `req.inputs`, runs
      `git worktree add <path> -b <branch> [<base>]` via the injected git-CLI seam, and on success mints a
      `WorktreeId` + returns `Succeeded { side_effect_applied: true, emitted_events: [WorktreeCreated] }`.
- [ ] **Forbidden #6 pin:** the worktree is created via the **git CLI** (the injected runner), NEVER a git2
      mutating API — pinned by a test (the fake CLI runner records the `git worktree add …` invocation) AND a
      scoped grep that `git/executor.rs` calls no git2 mutating API.
- [ ] `WorktreeCreated` is emitted via `EmittedEvent::Namespaced { event_type: WorktreeCreated::EVENT_TYPE,
      payload_json }` (the edges-019 bridge) and lands in the audit log through the §15 gate, ATOMIC with
      `ActionSucceeded`.
- [ ] **Partial-success (fail-closed):** `side_effect_applied: true` (a real FS mutation) → if the txn-B append fails
      after the worktree was created, the pipeline records `ActionPartiallySucceeded` (the honest divergence —
      the worktree exists but the event didn't commit), NOT a clean rollback.
- [ ] A git-CLI failure (non-zero exit / spawn error) → `ExecutionOutcome::Failed` with the structural reason
      BEFORE any event is emitted (no `WorktreeCreated` on a failed create).
- [ ] Missing/blank required inputs (worktree path / branch) → `Failed`, never a partial git invocation.
- [ ] `git.status` / `git.diff` / `git.create_branch` dispatched to `GitExecutor` delegate to the inner stub
      (no-op success, no event) — they are not handled by this slice.
- [ ] All unit + integration tests pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
**Production entry point:** `daemon/src/main.rs` — `catalog_executor.register(ExecutorKind::Git,
Arc::new(GitExecutor::new(SystemGitCli, idgen)))` before the Gateway is built. Reachable via the existing
`submit_action` IPC: a `git.create_worktree` `ActionRequest` (risk-2) → Gateway → **approval** → approved
path → `CatalogExecutor` dispatch → `GitExecutor::execute`. (risk-2 requires approval — not auto-execute; the
Gateway handles the approval transition, the executor runs on the approved path.)

**Deferred (NOT this slice):** the `proj_worktree` **projector** that consumes `WorktreeCreated` (the gated
5.2-remainder) — the event lands in the audit log (replayable); the projector lands in a follow-on
(`proj_worktree` table exists, so NO migration). `none for the read-model projection — lands in the
proj_worktree-projector slice`.

## Files expected to touch
**New:**
- `daemon/src/git/cli.rs` — the git-CLI seam: a `GitCli` trait (`run(args, cwd) -> Result<GitCliOutput,
  GitCliError>`), a real `SystemGitCli` (`std::process::Command::new("git")`), and a
  `#[cfg(feature = "test-support")] FakeGitCli` test double (records invocations + returns canned output).
- `daemon/src/git/executor.rs` — `GitExecutor` (`impl ActionExecutor`); handles `create_worktree`, delegates
  the rest to the inner `CatalogExecutor` stub.
- `daemon/tests/git_executor.rs` — integration (submit_action end-to-end, with approval) + unit tests.

**Modified:**
- `daemon/src/git/mod.rs` — `pub mod cli; pub mod executor;`.
- `daemon/src/main.rs` — `register(ExecutorKind::Git, …)`.
- (No `gateway/` edit — the `EmittedEvent::Namespaced` bridge from edges-019 already exists.)

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2 — `daemon/tests/git_executor.rs`)
1. **`test_create_worktree_invokes_git_cli_add`** — Asserts: the fake CLI runner received `git worktree add
   <path> -b <branch>` (+ base if given), run in the repo cwd. Why: forbidden #6 — mutation via the CLI.
2. **`test_create_worktree_emits_worktree_created`** — Asserts: success → exactly one `WorktreeCreated` whose
   `path`/`branch_name`/`base_branch` match inputs + a freshly-minted `wt_` `worktree_id`. Why: §6.3/§7.2 emission.
3. **`test_create_worktree_side_effect_applied_true`** — Asserts: `Succeeded { side_effect_applied: true }`
   (so a txn-B fault → ActionPartiallySucceeded, not rollback). Why: the honest partial-success path for a real FS change.
4. **`test_create_worktree_cli_failure_is_failed_no_event`** — Asserts: a non-zero git exit → `Failed`, no
   `WorktreeCreated` emitted. Why: fail-before-event (no phantom worktree record).
5. **`test_create_worktree_missing_inputs_failed`** — Asserts: absent/blank worktree path OR branch → `Failed`,
   the CLI runner is NEVER invoked. Why: fail-closed input guard before the side effect.
6. **`test_git_executor_no_git2_mutation`** — Asserts (scoped grep / structural): `git/executor.rs` uses the
   injected CLI runner, calls no git2 mutating API. Why: forbidden #6 structural pin.
7. **`test_create_worktree_requires_resource_ref`** — Asserts: the catalog `requires_resource_refs`
   precondition is enforced (no resource_ref → `Failed`). Why: §6.3 precondition (the repo identity).
8. **`test_git_status_diff_branch_delegate_to_stub`** — Asserts: `git.status`/`git.diff`/`git.create_branch`
   dispatched to `GitExecutor` → the no-op stub outcome, no event. Why: shared `ExecutorKind::Git` dispatch;
   these are not handled here (create_branch → edges-021; status/diff → read path).
9. **`test_create_worktree_e2e_via_submit_action`** — Asserts: submit `git.create_worktree` (risk-2) →
   approve → executes → `WorktreeCreated` persisted; over a real hermetic temp repo with `SystemGitCli`. Why:
   end-to-end reachability through the real Gateway + approval path.
10. **`test_create_worktree_remote_safe`** *(if any URL/path field risks creds)* — Asserts: no credential
    leaks into the payload. Why: §15 backstop (likely N/A — path/branch only; confirm at 2.5).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none to a `shared/` contract — `WorktreeCreated` frozen @ 0.26.0; no CONTRACT bump;
  no schema-snapshot test (existing `shared/tests/contract.rs` pins it).
- **Orchestrator doc rows to write hot (Step 9):** held in PLAN-DELTA for the phase-exit merge (edges
  cross-track pattern) — §6.3 row note (`ExecutorKind::Git` registered, `create_worktree` live),
  EventTypeRegistry note (`WorktreeCreated` live emitter), and a LESSON candidate for the git-CLI-mutation
  seam (the forbidden-#6 enforcement pattern + the partial-on-real-FS-change handling) → next-free LESSON slot.
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **git-CLI seam shape.** A `GitCli` trait + real `SystemGitCli` + `#[cfg(feature="test-support")] FakeGitCli`.
   My default vote: **yes, this shape** — mirrors the `SessionLauncher`/`PtySpawner` injected-seam precedent
   (LESSON 25/28) for the non-deterministic spawn; gate the fake behind `test-support` from creation (the
   feature exists post-merge — new fakes gate from day one, no retro-gate in Wave-E).
2. **Operational params source.** Read `worktree_path` / `branch_name` / `base_branch?` from `req.inputs`, and
   the **repo cwd** (where `git worktree add` runs) from inputs too (`repo_path`)? My default vote:
   **all from `req.inputs`**; the `resource_ref` is the repo IDENTITY (audit/policy), inputs carry the
   operational params (the session.create precedent). A projection-lookup of the repo path by id is a later
   refinement — don't couple this slice to a projector.
3. **Partial-success semantics.** `side_effect_applied: true` (real FS change) → confirm a txn-B fault yields
   `ActionPartiallySucceeded`, not rollback. My default vote: **true** (the worktree on disk can't be un-done
   by a rollback; the honest partial-success path — LESSON 21).
4. **`WorktreeId` minting.** The executor mints a fresh `wt_` id via an injected `IdGen` (the worktree is
   created by this action; no pre-existing id). My default vote: **inject `IdGen`** (deterministic-replay, LESSON 3).
5. **Branch-already-exists / path-occupied.** `git worktree add -b <branch>` fails if the branch exists or the
   path is occupied → surfaces as the CLI-failure `Failed` path (test 4). My default vote: **let the CLI be the
   source of truth** (don't pre-check; the structural failure reason is recorded) — a pre-check races the CLI anyway.

## Dependencies + sequencing
- **Depends on:** edges-019 (the `EmittedEvent::Namespaced` bridge + the in-txn emission pattern);
  `WorktreeCreated` frozen on main (merged).
- **Blocks:** edges-021 (`git.create_branch` extends `GitExecutor`); the `proj_worktree` projector slice
  (consumes `WorktreeCreated`).

## Estimated commit count
**1.** One focused security-load-bearing slice — the first real edges FS/git mutation through the Gateway. Its
OWN commit; do NOT bundle. `security-reviewer` required (INV-SEC-1, forbidden #6, the partial-success path).

## Lessons-logged candidates anticipated
- **Convention candidate (→ next LESSON slot)** — "edges git mutators run via an injected git-CLI seam
  (forbidden #6 — never git2 for mutations; structural grep-pin), report `side_effect_applied: true` so a
  txn-B fault yields the honest ActionPartiallySucceeded, and mint the `wt_` id via injected IdGen."
- **Architecture-doc note candidate** — `ExecutorKind::Git` registered; `git.status`/`git.diff` served via the
  read path (no executor needed); `WorktreeCreated` live emitter.
- **Future TODO — phase** — the `proj_worktree` projector (gated 5.2-remainder); `git.create_branch` (edges-021).

## How to invoke
1. **Read this brief end-to-end** (5 Step-2.5 questions have default votes).
2. **Run `/tdd git_create_worktree_executor`.**
3. **Step 2.5** — send the test-design write-up (one `Asserts: <invariant> (§anchor)` line per test + the
   acceptance-bullet coverage map) + your answers to the 5 questions. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 9** — surface categorized flags (esp. the forbidden-#6 enforcement + the next LESSON candidate).

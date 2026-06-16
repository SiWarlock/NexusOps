# /tdd brief — git_create_branch_executor

## Feature
Extend the landed `GitExecutor` (edges-020) with the `git.create_branch` arm: shell out to the **git CLI**
(`git branch <name> [<start-point>]`, forbidden #6) and emit `BranchCreated` via the `EmittedEvent::Namespaced`
bridge through the §15 gate. Completes the P5.2 git mutators (`ExecutorKind::Git`).

## Use case + traceability
- **Task ID:** P5.2
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (catalog / executor dispatch), `§7.2`
  (the worktree/branch read model), `§5.1` (Worktree status machine), `§15` (the in-txn redaction gate).
  Forbidden #6 (git CLI for mutations, never git2) is the load-bearing daemon rule.
- **Related context:** edges-020 (`7dabab5`) landed `GitExecutor { cli, inner }` + the git-CLI seam
  (`git/cli.rs`: `GitCli`/`SystemGitCli`/`FakeGitCli`) + the `execute_create_worktree` arm + the
  **leading-`-` argument-injection guard** (inline at `git/executor.rs:92`). This slice adds the sibling
  `create_branch` arm. `BranchCreated { branch_name, base: Option<String> }` is frozen (CONTRACT 0.26.0).
  **Standing requirement (edges-020 security HIGH):** every external mutator guards against arg-injection.

## Acceptance criteria (what "done" means)
- [ ] `GitExecutor::execute` routes `git.create_branch` to a new `execute_create_branch` arm (the
      `create_worktree` arm + the stub-delegation for `git.status`/`git.diff` are unchanged).
- [ ] `execute_create_branch` validates the catalog `requires_resource_refs` precondition, reads
      `branch_name` (+ optional `base`/start-point) + the repo cwd from `req.inputs`, runs
      `git branch <name> [<start-point>]` via the injected git-CLI seam, and on success returns
      `Succeeded { side_effect_applied: true, emitted_events: [BranchCreated{branch_name, base}] }`.
- [ ] **Argument-injection guard (standing requirement):** reject any `branch_name`/`base`/repo-path operand
      starting with `-` fail-closed BEFORE the CLI runs (reuse the edges-020 guard — see Step-2.5 Q1) +
      canonical option-before-operand arg order. Regression-pinned.
- [ ] **Forbidden #6 pin:** the branch is created via the git CLI, never a git2 mutating API (the existing
      `test_git_executor_no_git2_mutation` structural pin already covers `git/executor.rs`; extend if needed).
- [ ] `BranchCreated` emitted via `EmittedEvent::Namespaced { event_type: BranchCreated::EVENT_TYPE,
      payload_json }`, landing through the §15 gate, ATOMIC with `ActionSucceeded`.
- [ ] A git-CLI failure → `Failed` with a STRUCTURAL reason (no raw stderr → §15), no `BranchCreated`.
- [ ] Missing/blank `branch_name` → `Failed`, CLI never invoked.
- [ ] `side_effect_applied: true` (real git mutation → honest `ActionPartiallySucceeded` on a txn-B fault).
- [ ] All tests pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
**Production entry point:** `ExecutorKind::Git` is already registered in `main.rs` (edges-020) → the new
`create_branch` arm is reachable the moment `GitExecutor::execute` matches `git.create_branch`. Path:
`submit_action` IPC → Gateway → **approval** (risk-2) → `CatalogExecutor` → `GitExecutor::execute_create_branch`.
**No main.rs change** (registration already done). **Deferred:** a `BranchCreated` read-model projector
(no consumer table yet; the event is in the audit log, replayable) — `none for the read-model projection`.

## Files expected to touch
**Modified:**
- `daemon/src/git/executor.rs` — add `GIT_CREATE_BRANCH` const + `execute_create_branch` + the match arm;
  (Step-2.5 Q1) extract the leading-`-` guard into a shared helper used by both arms.
- `daemon/tests/git_executor.rs` — add the `create_branch` tests.

(No new files; no `gateway/`/`main.rs` edit.) If implementation needs files beyond this, flag at Step 2.5.

## RED test outline (Step 2 — `daemon/tests/git_executor.rs`)
1. **`test_create_branch_invokes_git_cli_branch`** — Asserts: `git branch <name> [<start-point>]` via the
   injected CLI runner in the repo cwd. Why: forbidden #6 — mutation via the CLI.
2. **`test_create_branch_emits_branch_created`** — Asserts: success → exactly one `BranchCreated` with
   `branch_name`/`base` matching inputs. Why: §6.3/§7.2 emission.
3. **`test_create_branch_side_effect_applied_true`** — Asserts: `Succeeded { side_effect_applied: true }`.
   Why: real git mutation → honest partial on a txn-B fault.
4. **`test_create_branch_cli_failure_is_failed_no_event`** — Asserts: non-zero git exit → `Failed`
   (STRUCTURAL reason), no `BranchCreated`. Why: fail-before-event.
5. **`test_create_branch_missing_inputs_failed`** — Asserts: blank `branch_name` → `Failed`, CLI never run.
   Why: fail-closed input guard.
6. **`test_create_branch_rejects_dash_leading_operand`** — Asserts: `branch_name`/`base` starting with `-`
   (e.g. `--force`) → `Failed`, CLI never run. Why: argument-injection guard (the edges-020 standing requirement).
7. **`test_create_branch_e2e_via_submit_action_approve`** — Asserts: submit (risk-2) → AwaitingApproval (no
   event) → approve → `BranchCreated` persisted. Why: approve-path reachability.
8. **`test_git_status_diff_still_delegate_to_stub`** — Asserts: `git.status`/`git.diff` still delegate to the
   stub (no event) after adding the `create_branch` arm. Why: the delegation regression isn't broken.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — `BranchCreated` frozen @ 0.26.0; no CONTRACT bump; no schema-snapshot test.
- **Orchestrator doc rows (held for the merge):** §6.3 row note (`git.create_branch` now live); a small
  LESSON-31 extension (the guard is now a shared helper) if the extraction warrants it.
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **Extract the leading-`-` guard into a shared helper?** edges-020 has it inline in `execute_create_worktree`.
   My default vote: **YES — extract a `reject_dash_operands(&[&str]) -> Result<(), …>` helper** used by both
   arms (DRY; it's now a standing cross-mutator requirement). Keep it tiny + pure; the regression tests pin both arms.
2. **`base` semantics.** `git branch <name> <start-point>` (base = the start ref). My default vote: pass
   `base` as the start-point operand when present, else `git branch <name>` (current HEAD). Guard `base` for `-`.
3. **`BranchCreated` projector — deferred?** My default vote: **defer** (no consumer table; the event is in the
   audit log, replayable) — same pattern as `WorktreeCreated`.
4. **`side_effect_applied: true`.** Confirm — a created branch is a real git mutation. My default vote: **true.**
5. **repo cwd source.** Same as edges-020 (`req.inputs["repo_path"]`). My default vote: **yes, mirror edges-020.**

## Dependencies + sequencing
- **Depends on:** edges-020 (`GitExecutor` + the git-CLI seam + the Namespaced bridge + the guard).
- **Blocks:** completes the P5.2 git mutators. (The `proj_worktree`/branch read-model projector is a separate
  follow-on; Wave-C `integration_connections` is MIGRATION_9-deferred per D8; Wave-D = github/linear.)

## Estimated commit count
**1.** A focused security-load-bearing mutator (its OWN commit per the lead's "each INV-SEC-1 mutation is its
own auditable, security-reviewed slice"). `security-reviewer` required (forbidden #6 + the injection guard).

## Lessons-logged candidates anticipated
- **Convention candidate** — extends LESSON 31 (the git-mutator pattern); confirms the arg-injection guard
  generalizes (now a shared helper across both git mutators).
- **Architecture-doc note candidate** — `git.create_branch` live; the P5.2 git mutators are complete.

## How to invoke
1. **Read this brief end-to-end** (5 Step-2.5 questions have default votes).
2. **Run `/tdd git_create_branch_executor`.**
3. **Step 2.5** — test-design write-up + coverage map + answers. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 9** — categorized flags (the guard extraction + completion of the P5.2 mutators).

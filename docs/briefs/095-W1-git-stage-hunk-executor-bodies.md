# /tdd brief — git_stage_unstage_hunk_executor_bodies

## Feature
Implement the real `git.stage_hunk` / `git.unstage_hunk` Gateway-executor bodies in `GitExecutor` — re-derive the targeted hunk from the live diff (the frozen position-only resource-ref) and apply it to the git INDEX via the git CLI, with `git apply --check` fail-closed as the read↔mutate race guard. Today these two action types are R1-A registry stubs (side-effect-free) → the existing hunk-staging UI (ui-6.3e) is wired-but-non-functional. (`git.discard_hunk` — risk-3 DESTRUCTIVE — is a SEPARATE safety-pinned slice **W1-git-discard**, not this one.)

## Use case + traceability
- **Task ID:** W1-git-stage
- **Architecture sections it implements:** `ARCHITECTURE.md §6.2`/`§6.3` (the Gateway executor body + the catalog entry it realizes), `§17` (read↔mutate consistency — fail-closed when the hunk no longer matches), `§15` (the git-CLI mutation guards + structural-reason redaction), `§6.1` (the `get_diff` / `daemon/src/git` read the hunk re-derives from).
- **Related context:** the body precedents in `daemon/src/git/executor.rs` — `execute_create_worktree`/`execute_create_branch` (validate-precondition-first → `self.cli.run(args, repo_path)` → structural-reason-on-nonzero-exit). The frozen **position-only hunk resource-ref** (§6.3, 4.0b-ui1): `ResourceRef{resource_type=File, id="{worktree_id}\x1f{file}\x1f{old_start},{old_lines},{new_start},{new_lines}"}` — the UI sends ONLY the LOCATION (`ui/src/intent/hunk-resource-ref.ts`), the daemon re-derives the content. The read-only `daemon/src/git` `read_diff` (LESSON 33) gives the live `DiffResult{hunks: [Hunk{header, old_start, old_lines, new_start, new_lines, lines}]}`. Guards: `reject_dash_operands` (LESSON 45), forbidden #6 (git CLI for mutations, NEVER git2 mutating), LESSON 63 (resolve the target from the AUDITED resource_ref, never `inputs`), LESSON 47 (the worktree git-axis is a live-read cache → no domain event).

## Acceptance criteria (what "done" means)
- [ ] **`git.stage_hunk`** decodes the resource_ref `{worktree_id, file, positions}` (the `\x1f`-split), resolves `worktree_id → proj_worktree.path` over read-only WAL (the `get_diff` precedent; unresolvable → `Failed`/NotFound-class), re-derives the matching hunk from the live diff, builds a minimal one-hunk patch, and applies it to the INDEX via `git apply --cached` — `side_effect_applied=true` on a clean apply.
- [ ] **`git.unstage_hunk`** is the reverse-apply to the index: `git apply --cached -R` (the hunk must currently be staged → `--check` fails-closed if not).
- [ ] **Read↔mutate race guard (§17):** `git apply --check` runs FIRST; a non-clean check (the file/hunk changed since the UI read it → the context no longer matches) → `Failed` with a STRUCTURAL "hunk no longer applies (re-approve)" reason, **no apply**. ALSO: if no hunk in the live diff matches the resource-ref positions → `Failed` (the displayed hunk is gone), no apply.
- [ ] **Validate `requires_resource_refs` FIRST** (`inner.validate`) + the resource_ref decode (malformed id → `Failed` before any git call). The `file` operand is `reject_dash_operands`-guarded (LESSON 45); the path is resolved from the AUDITED resource_ref, never `inputs` (LESSON 63).
- [ ] **§15:** raw git stderr is NEVER surfaced into the persisted `ActionFailed` (structural class-names only — a path/diff can carry secrets); the temp patch file is written 0600 + cleaned up.
- [ ] **No domain event** — the worktree git-axis is a live-read cache (LESSON 47); the UI re-reads `get_diff` to see the new staged/unstaged state. `emitted_events` empty (the Gateway `ActionSucceeded` is the audit trail). Contract-neutral (the catalog + resource-ref froze at 4.0b-ui1).
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`daemon/src/git/executor.rs::GitExecutor::execute` — add `GIT_STAGE_HUNK`/`GIT_UNSTAGE_HUNK` consts + match arms (alongside `create_worktree`/`create_branch`) → `execute_stage_hunk`/`execute_unstage_hunk` (or one `execute_apply_hunk(req, reverse: bool)`). Reachable from the production gateway execute via the registered `ExecutorKind::Git` (main.rs registers `GitExecutor`; the create_worktree `/wired` precedent). The hunk re-derivation reads `daemon/src/git` `read_diff`.

## Files expected to touch
**Modified:**
- `daemon/src/git/executor.rs` — the 2 bodies + consts + match arms + the patch-build + resolve-worktree-path helper.
- `daemon/src/git/` (read module) — possibly a small `hunk_patch`/`find_hunk` helper if the patch build lives there.
- `daemon/tests/git_executor.rs` — extend (decode / resolve / re-derive-match / apply / --check-race-fail / reject-dash / structural-reason / no-event).

**New:** none expected (extends the existing GitExecutor + git read module).

If implementation needs files beyond this list (e.g. a worktree-path resolver seam), **flag at Step 2.5**.

## RED test outline (Step 2)
Tests in `daemon/tests/git_executor.rs` (real git repo fixtures — the `create_worktree` 9a real-CLI precedent):

1. **`git_stage_hunk_applies_to_index`** — a real worktree with an unstaged hunk → stage_hunk stages exactly that hunk (the index now has it; other hunks untouched). Why: §6.3 body.
2. **`git_unstage_hunk_reverse_applies`** — a staged hunk → unstage_hunk removes it from the index. Why: §6.3.
3. **`git_stage_hunk_race_changed_file_fails_closed`** — the file changed so the resource-ref positions no longer match / `--check` fails → `Failed` (structural), no index change. Why: §17 read↔mutate.
4. **`git_stage_hunk_no_matching_hunk_fails`** — the resource-ref positions match no live hunk → `Failed`, no apply. Why: §17.
5. **`git_stage_hunk_malformed_ref_or_missing_target_fails`** — malformed `\x1f` id / no resource_ref → `Failed` before any git call. Why: §6.3 precondition + LESSON 63.
6. **`git_stage_hunk_rejects_dash_file_operand`** — a leading-`-` file → rejected fail-closed. Why: LESSON 45.
7. **`git_stage_hunk_structural_reason_no_stderr`** — a forced git failure → the `ActionFailed` reason is a structural class, NOT raw git stderr. Why: §15.
8. **`git_stage_hunk_emits_no_event`** — a successful stage emits NO domain event (`emitted_events` empty); `side_effect_applied=true`. Why: LESSON 47 live-read cache.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (contract-neutral — the catalog + resource-ref encoding froze at 4.0b-ui1).
- **Orchestrator doc rows to write hot (Step-9):** a §6.3 note that the `git.stage_hunk`/`unstage_hunk` executor bodies are now LIVE (the R1-A stubs realized). Minor; orchestrator writes it.
- **§2.5-seam:** NO (no shared model).

## Things to flag at Step 2.5
1. **Patch construction** — reconstruct the one-hunk patch from the parsed `read_diff` `Hunk.lines`, vs slice the raw `git diff -- <file>` output by matching the `@@` header. My default vote: **slice the raw `git diff` output** — git's own unified-diff format is the source of truth (no DiffLine→patch round-trip fidelity risk); match the hunk by its `@@ -old +new @@` header against the resource-ref positions.
2. **Worktree-path resolution** — resolve `worktree_id → proj_worktree.path` over read-only WAL (the `get_diff` precedent). My default vote: **yes, reuse the get_diff resolution path** (NotFound until proj_worktree populates — the same mechanism-live/populated-gated posture as get_diff; the hunk UI is downstream of get_diff working anyway).
3. **One body or two** — `execute_stage_hunk` + `execute_unstage_hunk` vs one `execute_apply_hunk(reverse)`. My default vote: **one `execute_apply_hunk(req, reverse)`** — stage/unstage differ only by `-R`; DRY + one race-guard path.
4. **`--check` then apply, or apply-with-rollback** — My default vote: **`apply --check` FIRST, then `apply`** (the dry-run gate; cleaner than apply-then-rollback for the index).

## Dependencies + sequencing
- **Depends on:** the `GitExecutor` (create_worktree/create_branch landed) + `daemon/src/git` `read_diff` (get_diff, landed) + the frozen hunk resource-ref (4.0b-ui1).
- **Blocks:** the hunk-staging UI (ui-6.3e) becoming functional.
- **Sibling (NOT this slice):** **W1-git-discard** = `git.discard_hunk` (risk-3 DESTRUCTIVE → its own safety-pinned slice; the read↔mutate-race-for-a-destructive-op safety-design surfaces to the lead before I author it).

## Estimated commit count
**1.** The stage+unstage bodies are one cohesive contract-neutral unit (they differ only by `-R`). **NOT safety-pinned** — stage/unstage mutate the INDEX (recoverable; re-stage/unstage undoes it), so the destructive-discard floor does not apply here. `security-reviewer` (the `invariant` policy) still runs (a git-CLI mutation executor touching §15/LESSON 45/LESSON 63); the race-guard (test 3) + the structural-reason (test 7) are the load-bearing pins. (The DESTRUCTIVE discard is the separate W1-git-discard slice.)

## Lessons-logged candidates anticipated
- **Convention candidate** — "a per-hunk git mutation re-derives the hunk from the live diff at execute-time (the frozen position-only resource-ref carries LOCATION not content) + `git apply --check` fail-closed as the read↔mutate race guard (§17); no domain event (the worktree git-axis is a live-read cache LESSON 47); the structural-reason / reject-dash / resolve-from-audited-ref CLI-mutation guard family (LESSON 45/LESSON 63)."
- **Architecture-doc note candidate** — the `git.stage_hunk`/`unstage_hunk` bodies join the live §6.3 executor set (the R1-A stubs realized).

## How to invoke
1. Read this brief end-to-end (esp. Step-2.5 Q1 patch-construction + the §17 race guard).
2. Run `/tdd git_stage_unstage_hunk_executor_bodies`.
3. Step 2.5 — ping back with answers (or take defaults).
4. Step 9 — surface the §6.3 doc note + confirm the discard slice is correctly deferred.

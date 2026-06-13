# /tdd brief — proj_worktree_projector

## Feature
The `proj_worktree` read-model projector (the "gated 5.2-remainder"): fold `WorktreeCreated` into a
`proj_worktree` row so the worktree the P5.2 mutators emit becomes a live read model. Completes the P5.2
read vertical (the existing `proj_worktree` TABLE has had no producer until now).

## Use case + traceability
- **Task ID:** P5.2
- **Architecture sections it implements:** `ARCHITECTURE.md §7.2` (the worktree read model + live-read
  cache), `§5.1` (the Worktree status machine the `status` column binds), `§15` (the projector reads the
  §15-redacted sibling `action_requests` row — repo identity, a non-secret, survives redaction).
- **Related context:** edges-020/021 emit `WorktreeCreated`/`BranchCreated` (audit log) but nothing
  consumes them yet (`git/reads.rs` names "the `proj_worktree` projector (the gated 5.2-remainder)").
  Projector pattern: `daemon/src/projections/session.rs` (SessionStarted → proj_session) + the sibling-read
  precedent `daemon/src/projections/graph.rs` (reads `object_refs` written earlier in-txn). Registration:
  `projections/mod.rs::projectors()`. `proj_worktree` DDL: `worktree_id PK, project_id NN, repo_id NN, path,
  branch_name, base_branch, owner_session_id?, owner_team_id?, linked_task_id?, status NN, dirty_state?,
  ahead_count?, behind_count?, last_commit_sha?, pr_status?, git_checked_at?, updated_at_seq NN`.

## Acceptance criteria (what "done" means)
- [ ] A `WorktreeProjector` (`impl Projector`, `name()="worktree"`) registered in `projectors()`; folds ONLY
      `WorktreeCreated` (the overlay lifecycle events have no emitter yet → not handled).
- [ ] On `WorktreeCreated`: INSERT a `proj_worktree` row — `worktree_id`/`path`/`branch_name`/`base_branch`
      from the payload; `project_id` from `env.project_id`; `repo_id` from the immutable sibling read (see
      Step-2.5 Q1); `status` = the initial Worktree §5.1 value (Q2); `updated_at_seq = env.seq`. ON CONFLICT
      (worktree_id) DO UPDATE the event-sourced columns.
- [ ] The live-read columns (`dirty_state`/`ahead_count`/`behind_count`/`last_commit_sha`/`git_checked_at`/
      `owner_*`/`linked_task_id`/`pr_status`) are inserted NULL — they are the §7.2 live-read cache, populated
      by a separate refresh (NOT this projector). State that explicitly.
- [ ] An identity-less event (no `project_id`, or no repo ref) → **healthy no-op skip** (NOT a degrade) —
      `proj_worktree.project_id`/`repo_id` are NOT NULL; a non-projectable event is skipped, the session.rs precedent.
- [ ] `status` binds the frozen §5.1 Worktree machine via `wire_value` (reject-unknown → `ProjectionError::Decode`,
      generic reason, never raw payload bytes — §15); never stored raw.
- [ ] **Rebuild-equivalence:** `proj_worktree` is in `REBUILD_TABLES`; a `rebuild()` reproduces byte-identical
      rows. The sibling `action_requests` read takes IMMUTABLE fields only (resource_refs) so rebuild (which
      reads action_requests at FINAL state) is deterministic — the LESSON 17 immutable-sibling-read rule.
- [ ] All tests pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
**Production entry point:** `projections/mod.rs::projectors()` — add `Box::new(worktree::WorktreeProjector)`.
The projector is folded **in-band in the event-commit txn** (after the redaction gate, like every projector,
LESSON 4) for any `WorktreeCreated` the Gateway appends. Reachable end-to-end: `git.create_worktree` action →
`WorktreeCreated` event → in-txn projector fold → `proj_worktree` row → `get_projection(Worktree)` IPC. No new
event/IPC. **This closes the P5.2 read vertical** (mutator → event → projection → IPC read).

## Files expected to touch
**New:**
- `daemon/src/projections/worktree.rs` — `WorktreeProjector`.
- `daemon/tests/projections.rs` additions (or a new test fn block) — the projector tests.

**Modified:**
- `daemon/src/projections/mod.rs` — `mod worktree;` + register in `projectors()`.
- (If `proj_worktree` is not already in `REBUILD_TABLES` — confirm at Step 1 — add it.)

If implementation needs files beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2 — `daemon/tests/projections.rs`)
1. **`test_worktree_created_inserts_proj_worktree_row`** — Asserts: a real `git.create_worktree` (submit→
   approve→execute, or a direct append) → a `proj_worktree` row with the payload + sibling-sourced
   `project_id`/`repo_id` + initial `status` + `updated_at_seq=seq`. Why: §7.2 read-model fold.
2. **`test_worktree_projector_repo_id_from_sibling`** — Asserts: `repo_id` is sourced per Q1 (the action's
   repository resource_ref), present + correct. Why: LESSON 17 immutable-sibling-read.
3. **`test_worktree_projector_live_read_columns_null`** — Asserts: `dirty_state`/`ahead`/`behind`/
   `git_checked_at` are NULL on insert (live-read cache, not event-sourced). Why: §7.2 split.
4. **`test_worktree_projector_skips_identity_less`** — Asserts: a `WorktreeCreated` with no `project_id` (or
   no repo ref) → no row, no error (healthy skip). Why: NOT-NULL columns; session.rs precedent.
5. **`test_worktree_projector_status_binds_5_1`** — Asserts: `status` is a canonical §5.1 Worktree wire value
   (not raw). Why: §5.1 reject-unknown binding.
6. **`test_worktree_projector_rebuild_equivalent`** — Asserts: `rebuild()` reproduces the same `proj_worktree`
   rows byte-identically (the immutable-sibling-read is rebuild-safe). Why: LESSON 4/17 rebuild-determinism.
7. **`test_worktree_projector_ignores_other_events`** — Asserts: a non-`WorktreeCreated` event → no
   `proj_worktree` write. Why: the projector folds only its event.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none to a `shared/` contract (`proj_worktree` is a DDL table, not a `shared/`
  model; `WorktreeCreated` frozen @ 0.26.0). No CONTRACT bump; no schema-snapshot test.
- **Orchestrator doc rows (held for the merge):** §7.2 note (`proj_worktree` now has a producer — the P5.2
  read vertical is closed); possibly a LESSON note (the projector sources non-payload identity from the
  immutable action_requests sibling — a generalization of LESSON 17 to the gateway-event-emitting case).
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **`repo_id` source (the central design question).** `WorktreeCreated` payload has no `repo_id`, and the
   envelope has no `repo_id` column. My default vote: **read it from the immutable sibling `action_requests`
   row via `env.action_request_id`** (the create_worktree action's repository `resource_ref` → its id) — the
   LESSON 17 immutable-sibling-read (resource_refs are immutable → rebuild-safe). **Confirm at Step 1 that
   (a) `env.action_request_id` is set on the `WorktreeCreated` event** (the Namespaced bridge rides the
   action envelope — the as-built comment says it carries `action_request_id`), **and (b) the create_worktree
   `resource_ref` is a Repository-typed ref carrying a `repo_id`.** If either fails → FINDING (the worktree
   read model can't be keyed to a repo from the event alone) → surface before GREEN; do NOT guess a scheme.
2. **Initial `status`.** What §5.1 Worktree value does a just-created worktree take? My default vote: the
   §5.1 Worktree machine's initial/clean state (confirm the exact wire value against the frozen enum). The
   git-sync axis (`dirty_state`/ahead/behind) is the live-read cache (NULL here), so `status` is the
   overlay-axis lifecycle state (created → its initial value).
3. **Live-read columns NULL now, refresh deferred?** My default vote: **yes** — insert NULL; the
   `read_worktree_status`→`proj_worktree` live-read refresh is a separate §7.2 concern (its own slice; the
   read backend exists in `git/reads.rs` but the write-to-`proj_worktree` path is not this projector).
4. **Subscribe-delta?** Gateway-event projector writes may need a `ProjectionDelta` for live subscribers
   (LESSON 17 — thread the delta, publish post-commit). My default vote: **emit a `Worktree` delta keyed by
   `worktree_id`** if the gateway-event delta path is wired for projector writes (confirm; if it's not yet
   wired for non-`Command::Append` events, flag — the read still works via `get_projection`).

## Dependencies + sequencing
- **Depends on:** edges-020 (`WorktreeCreated` emitter); the projector framework (LESSON 4).
- **Blocks:** closes the P5.2 read vertical. (The live-read status refresh is a follow-on; Wave-D = P7.1.)

## Estimated commit count
**1.** A focused read-model projector. `security-reviewer` per the `invariant` policy (it touches the
rebuild-determinism rule [LESSON 17 immutable-sibling-read] + reads the §15-redacted sibling row); `code-quality` every-slice.

## Lessons-logged candidates anticipated
- **Convention candidate** — a gateway-event projector sourcing non-payload identity from the IMMUTABLE
  sibling `action_requests` row via `env.action_request_id` (LESSON 17 generalized from `object_refs`).
- **Architecture-doc note candidate** — `proj_worktree` has a producer; the §7.2 worktree read vertical closes;
  the live-read status refresh is the named remaining §7.2 follow-on.

## How to invoke
1. **Read this brief end-to-end** (Q1 is load-bearing — verify the repo_id source at Step 1 before tests).
2. **Run `/tdd proj_worktree_projector`.**
3. **Step 2.5** — test-design write-up + coverage map + answers; flag immediately if Q1's repo_id source doesn't hold.
4. **Step 9** — categorized flags (the sibling-read generalization + the P5.2-read-vertical closure).

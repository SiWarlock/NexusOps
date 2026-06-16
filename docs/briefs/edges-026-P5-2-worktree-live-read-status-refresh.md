# /tdd brief — worktree_live_read_status_refresh

## Feature
The §7.2 worktree-status **live-read cache refresh** (P5.2 follow-on) — reads a worktree's live git
truth via `read_worktree_status` (git2, read-only) and writes the git-axis cache columns of
`proj_worktree` (`dirty_state`/`ahead_count`/`behind_count`/`last_commit_sha`/`git_checked_at` + the
recomputed `status`) through a NEW **non-Gateway, non-event** write-actor command, triggered by a
**git-watcher** interval task. **NO `WorktreeStatusRefreshed` event** (§7.1 — the git-axis is a
live-read projection cache, not event-sourced).

## Use case + traceability
- **Task ID:** P5.2 (the live-read status-refresh follow-on, named in the R5 wiring-plan TODO).
- **Architecture sections it implements:** `ARCHITECTURE.md §7.2` (the worktree-status live-read cache +
  `git_checked_at` staleness), `§5.1` (the `WorktreeGit` git-sync axis + `derive_worktree_status`
  precedence) — within scope.
- **Widens phase scope because** `§7.1` is cited only to confirm a NEGATIVE (no status-refresh event — the
  git-axis is a live-read cache, ARCHITECTURE.md:576) and `§16`/runtime for the git-watcher task wiring
  (the drainer/reaper precedent); neither is redefined.
- **Related context:**
  - **edges-022 `proj_worktree` projector** (`c666dc0`) — left the git-axis cache columns NULL + ABSENT
    from the `ON CONFLICT DO UPDATE` set (so a re-fold/rebuild preserves any refresh values). This slice
    fills them. The edges-022 doc flagged "the rebuild-compare coverage boundary (5 always-NULL columns)
    revisits here."
  - **`git/reads.rs::read_worktree_status(path, base) -> Option<WorktreeGitState>`** — `WorktreeGitState {
    git_axis: WorktreeGit, branch, last_commit_sha, ahead_count, behind_count }`; git2 read-only; `None`
    on a non-git/inaccessible path. The NON-DETERMINISTIC edge (real git2) — temp-repo-fixture-covered
    (the edges-002/005 git2-read test pattern).
  - **`git/precedence.rs::derive_worktree_status(git: Option<WorktreeGit>, overlay: Option<WorktreeOverlay>)
    -> DerivedWorktreeStatus`** — the §5.1 precedence (`locked/conflicts > dirty > ahead/behind > creating
    > clean`); serializes to the `WorktreeGit ∪ WorktreeOverlay` wire string via `wire_value`.
  - **The write-actor** (`runtime/writer.rs`) — `Command::{Append, DrainOnce, ReapLeases, Gateway*}`. There
    is NO projection-update command yet; this slice adds one (the DrainOnce/ReapLeases precedent — a
    non-Gateway, non-event write-actor operation on the single writer, forbidden #3).
  - **The git-watcher** — `ARCHITECTURE.md:340` names a daemon Tokio task: "git watcher (refresh
    worktree/PR caches via git2 + git hooks)" — the drainer/reaper interval-task precedent (`main.rs`
    `spawn_drainer`/`spawn_reaper`).

## Acceptance criteria (what "done" means)
- [ ] A NEW write-actor command (`RefreshWorktreeStatus { worktree_id, path, base }`) + a `WriteHandle`
      method — runs on the single write-actor thread (forbidden #3), NOT the Gateway, emits NO event.
- [ ] The refresh: `read_worktree_status(path, base)` → on `Some(state)` UPDATE `proj_worktree` SET
      `dirty_state` (= the `git_axis` wire value), `ahead_count`, `behind_count`, `last_commit_sha`,
      `git_checked_at` (= the injected daemon Clock, UTC-Z) WHERE `worktree_id`; the event-sourced columns
      (path/branch_name/base_branch/project_id/repo_id) are UNTOUCHED.
- [ ] `status` recompute (Q2): `status` = `wire_value(derive_worktree_status(Some(git_axis),
      Some(WorktreeOverlay::Creating)))` — the live git-axis re-derived against the overlay. (Overlay =
      `Creating` is the only EMITTED overlay in the MVP — see Q2; the clean overlay-source model is a
      flagged follow-on.)
- [ ] **`None` read** (non-git/inaccessible path) → stamp `git_checked_at` only (we checked; no git truth),
      leave the git-axis cols as-is/NULL, `status` unchanged (Q3). No panic.
- [ ] A git-watcher interval task (Q4) enumerates `proj_worktree` rows + issues a refresh per worktree
      (the drainer/reaper precedent; stopped by the shutdown watch) — wired in `main.rs`. **Reachable**
      (Step 7.5), not mechanism-only.
- [ ] **Rebuild-equivalence preserved:** `proj_worktree` is in `REBUILD_TABLES`; a rebuild truncates +
      replays from events → the git-axis cache cols reset to NULL + `status` to the event-derived
      `creating` (the live-read cache is NOT event-sourced; the watcher repopulates post-rebuild). The
      edges-022 rebuild test still passes (it compares event-sourced state).
- [ ] All tests pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
**Production entry point:** `main.rs` spawns the git-watcher interval task (alongside `spawn_drainer`/
`spawn_reaper`), which issues `RefreshWorktreeStatus` per `proj_worktree` row each interval → the
write-actor runs the refresh. Path: git-watcher tick → `WriteHandle::refresh_worktree_status` →
write-actor `RefreshWorktreeStatus` handler → git2 read + `proj_worktree` UPDATE. Reads served via the
existing `get_projection(Worktree)` (the cache columns now populated). **If the watcher balloons, the
fallback is the command reachable via a thin trigger** — but the default is a reachable watcher.

## Files expected to touch
**New:**
- `daemon/src/runtime/` (git-watcher task — a new `git_watcher.rs` or fold into `runtime/`) — the interval
  task (the `spawn_drainer`/`spawn_reaper` precedent).
- `daemon/tests/` — refresh + watcher tests (temp-repo fixtures for the git2 edge).

**Modified:**
- `daemon/src/runtime/writer.rs` — the `RefreshWorktreeStatus` command + handler + the `WriteHandle` method.
- `daemon/src/main.rs` — spawn the git-watcher task.
- (possibly) `daemon/src/git/reads.rs` — only if a small helper is needed (read_worktree_status exists).

(NO `shared/` / `gateway/` edit; NO new event; NO schema change — uses the existing `proj_worktree` DDL.)
If implementation needs files beyond this, **flag at Step 2.5**.

## RED test outline (Step 2)
1. **`test_refresh_fills_git_axis_cache`** — Asserts: a refresh on a temp worktree → `proj_worktree`
   `dirty_state`/`ahead_count`/`behind_count`/`last_commit_sha`/`git_checked_at` populated from the git
   read; event-sourced cols untouched. Why: §7.2 cache fill.
2. **`test_refresh_recomputes_status_from_live_git_axis`** — Asserts: a dirty worktree → `status` becomes
   `dirty` (the live git-axis ⟶ precedence over `creating`); a clean one stays `creating`. Why: §5.1
   precedence (Q2).
3. **`test_refresh_stamps_git_checked_at_utc_z`** — Asserts: `git_checked_at` = the injected Clock's UTC-Z.
   Why: §7.2 freshness stamp (LESSON 5 UTC-Z).
4. **`test_refresh_non_git_path_stamps_checked_only`** — Asserts: a `None` read → `git_checked_at` stamped,
   git cols/status unchanged, no panic. Why: typed-absence handling (Q3).
5. **`test_refresh_is_write_actor_not_gateway_no_event`** — Asserts: the refresh writes via the write-actor
   command, emits NO event (no `events` row, no `WorktreeStatusRefreshed`). Why: §7.1 (live-read cache,
   not event-sourced); forbidden #3 single-writer.
6. **`test_refresh_unknown_worktree_id_noop`** — Asserts: a refresh for a `worktree_id` with no
   `proj_worktree` row → no-op (0 rows updated), no error. Why: fail-safe.
7. **`test_git_watcher_refreshes_known_worktrees`** — Asserts: the watcher tick enumerates `proj_worktree`
   + issues a refresh per row (reachability — the watcher is the production trigger). Why: Step 7.5.
8. **`test_proj_worktree_rebuild_resets_live_read_cache`** — Asserts: after a refresh populates the cache,
   a `rebuild()` resets the git-axis cols to NULL + `status` to `creating` (event-derived), and the
   edges-022 rebuild-equivalence (event-sourced cols) still holds. Why: REBUILD_TABLES + live-read-cache
   semantics.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes (shared/):** NONE — no contract change; no new event; no schema change (uses the
  existing DDL); CONTRACT 0.26.0 held.
- **Orchestrator doc rows (held for the final merge — cross-track rule):** an arch note (the §7.2 worktree
  live-read cache is LIVE; the git-watcher task wired; `WorktreeStatusRefreshed`-is-not-an-event confirmed
  as-built); a LESSON candidate (the live-read-cache refresh pattern — a non-Gateway, non-event write-actor
  command + a git-watcher trigger + read-time staleness).
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **The write path (load-bearing).** A NEW write-actor command (`RefreshWorktreeStatus`), non-Gateway,
   non-event (§7.1: the git-axis is a live-read cache, NOT event-sourced), on the single writer (forbidden
   #3 — the DrainOnce/ReapLeases precedent). **My default vote: yes** — a `Command::RefreshWorktreeStatus`
   + a `WriteHandle::refresh_worktree_status`, mirroring `reap_leases`/`drain_once`. NOT a Gateway Action
   (no external mutation; a derived-cache write). NOT an event (no `WorktreeStatusRefreshed`).
2. **`status` recompute + the overlay source (load-bearing — flag the follow-on).** `status` =
   `derive_worktree_status(Some(git_axis), overlay)` needs the overlay; `proj_worktree` stores only the
   DERIVED `status`, not the overlay, and the overlay events (`WorktreeMerged`/`Locked`/…) have NO emitter
   yet → **the only overlay any worktree has is `Creating`** (from `WorktreeCreated`). **My default vote:
   recompute with `overlay = Some(WorktreeOverlay::Creating)`** (correct for the MVP — it's the only
   emitted overlay) + **FLAG**: when overlay-event emitters land (a post-merge slice), a clean overlay
   source is needed — either an `overlay` column (a schema change → **MIGRATION_9-deferred**, D8) or an
   event-sourced overlay-status read. Note this in the code + the held-for-merge ledger. (Alternative:
   fill ONLY the git-axis cache cols + leave `status` event-derived — REJECT: defeats the §7.2 "status
   reflects live git truth" point.)
3. **`None` read handling.** Non-git path → stamp `git_checked_at` only (we checked; no truth) vs no-op
   entirely. My default vote: **stamp `git_checked_at`, leave git cols/status unchanged** (records the
   check happened — staleness derivation needs it).
4. **The git-watcher trigger.** Include a minimal interval task (reachable — the drainer/reaper precedent,
   ARCHITECTURE.md:340) vs defer the trigger (mechanism-only). My default vote: **include a minimal
   watcher** (enumerate `proj_worktree` + refresh each on an interval; shutdown-watch-stopped) → Step-7.5
   reachable. Keep the interval generous (worktree git reads are cheap but not free). If it balloons,
   defer the watcher with a Step-7.5 "trigger lands in <slice>" note + flag back.
5. **Test harness for the git2 edge.** Temp-repo fixtures (the edges-002/005 `read_worktree_status` test
   pattern). My default vote: **reuse that pattern** (a temp git repo with a dirty/clean/ahead state).

## Dependencies + sequencing
- **Depends on:** edges-022 (`c666dc0`, the `proj_worktree` projector + the NULL/ON-CONFLICT-preserved
  cache cols); `read_worktree_status`/`derive_worktree_status` (edges-002/005); the write-actor runtime.
- **Blocks:** completes the P5.2 §7.2 live-read story. R7 continues: P5.4 bench · cargo audit → then edges
  PAUSES for the user-gated `/phase-exit` + edges→main merge.

## Estimated commit count
**1.** A focused §7.2 live-read-cache slice. NOT an INV-SEC-1 mutator (a derived-cache write, no external
mutation, no event) → `security-reviewer` per the `invariant` policy is **not required**;
`code-quality-reviewer` (every-slice). (If the impl judges the single-writer/forbidden-#3 surface warrants
it, run security too — orch lean: the no-Gateway/no-event/no-external-mutation shape is read-cache, not a
mutation path.)

## Lessons-logged candidates anticipated
- **Convention candidate** — the live-read-cache refresh pattern: a non-Gateway, non-event write-actor
  command (the DrainOnce/ReapLeases family) + a git-watcher interval trigger + read-time `git_checked_at`
  staleness; rebuild resets the cache (live-read, not event-sourced).
- **Architecture-doc note candidate** — §7.2 worktree live-read cache LIVE; the overlay-source follow-on
  (a clean overlay model for `status` recompute when overlay-event emitters land — MIGRATION_9/post-merge).

## How to invoke
1. **Read this brief end-to-end** — Q1 (the non-Gateway write path) + Q2 (the `status` recompute + the
   overlay follow-on) are the load-bearing calls.
2. **Run `/tdd worktree_live_read_status_refresh`.**
3. **Step 2.5** — test-design write-up + coverage map + answers to Q1-Q5. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 7.5** — confirm the git-watcher task is spawned in `main.rs` (reachable).
5. **Step 9** — categorized flags: the §7.2-cache-LIVE arch note, the overlay-source follow-on, the
   live-read-cache-refresh LESSON candidate.

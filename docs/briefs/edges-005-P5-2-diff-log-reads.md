# /tdd brief — git_diff_and_log_reads

## Feature
The git2 **read-only diff + log** functions (`git/reads.rs` extension) — completing Phase 5.2's named `status/diff/log/branch` read backend (status/branch/worktree-list landed in edges-002). File-level diff (working-tree-vs-ref or ref-vs-ref) + a bounded commit log, as daemon-internal structs. Pure read primitives for the §7.2 re-read-before-mutate path + the Code/Diff surface; their consumers (the gated git executor + the Code/Diff UI) stay deferred.

## Use case + traceability
- **Task ID:** P5.2 (the diff/log portion of the read backend)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (git2 for **status/diff/log**/branch/worktree-list reads), `§7.2` (Git/FS SoT = the repo; read git2 live).
- **Related context:** edges-002 (`git/reads.rs` — the worktree-status reads this extends; the same hermetic `git2::init` fixture pattern); the `git2` dep (read-only, forbidden #6).

## Acceptance criteria (what "done" means)
**Diff (`daemon/src/git/reads.rs`):**
- [ ] File-level diff returns a list of `FileChange { path, change_kind, additions, deletions }` over the changed files.
- [ ] `change_kind` covers Added / Modified / Deleted (Renamed optional — Step-2.5 Q).
- [ ] The diff target is parameterized: working-tree-vs-HEAD (uncommitted changes) AND ref-vs-ref (e.g. a PR/branch diff) — Step-2.5 Q on the exact signature.
- [ ] A clean tree → an empty diff (no changes), not an error.

**Log (`daemon/src/git/reads.rs`):**
- [ ] A bounded commit log returns `CommitInfo { sha, summary, author, timestamp }` for the most recent N commits from a ref (default HEAD).
- [ ] `limit` caps the walk (no unbounded revwalk).
- [ ] An empty repo (unborn HEAD) → an empty log, not an error/panic.

**General:**
- [ ] git2 READ-ONLY (forbidden #6); the reads do not mutate the repo (before/after HEAD-oid pin).
- [ ] Degraded-not-panic on a non-git / inaccessible path (consistent with edges-002's posture; no `unwrap`/`expect` in non-test).
- [ ] Unit/integration tests pass; `/preflight` clean. **No `shared/` touch, no migration, no `gateway/` touch, no new Cargo dep** (git2 already present).

## Wiring / entry point (Step 7.5)
**`none — wiring lands in the gated 5.2-remainder / 7.2 slices.`** Pure read primitives; consumers are the gated git executor (the §7.2 re-read-before-mutate diff) + the Code/Diff + PR-review UI (7.2, ui-track). Reachability intentionally deferred (named).

## Files expected to touch
**New:**
- (none — extends an existing module)

**Modified:**
- `daemon/src/git/reads.rs` — add `read_diff(...)` + `read_log(...)` + the `FileChange` / `CommitInfo` / `ChangeKind` daemon-internal types
- Test file: `daemon/tests/worktree_reads.rs` (extend) or a new `daemon/tests/git_diff_log.rs` — Step-1 choice

No `Cargo.toml` change. **Do NOT touch `gateway/`, `shared/`, `eventstore/`, or any migration.**

## RED test outline (Step 2)
**Diff (hermetic `git2::init` fixtures):**
1. **`diff_modified_file`** — modify a tracked file → `FileChange{Modified, +/-counts}`. Why: §9 diff.
2. **`diff_added_file`** — stage/commit then add a new file (working-tree) → `Added`. Why: change-kind.
3. **`diff_deleted_file`** — delete a tracked file → `Deleted`. Why: change-kind.
4. **`diff_clean_empty`** — clean tree → empty diff (not an error). Why: edge.
5. **`diff_ref_vs_ref`** — diff two commits/branches → the changed files between them. Why: §7.2 / PR-diff use.
**Log:**
6. **`log_recent_commits`** — a repo with N commits → `CommitInfo` list (sha/summary/author/timestamp), newest-first. Why: §9 log.
7. **`log_limit_caps`** — `limit=2` on a 5-commit repo → 2 entries. Why: bounded walk.
8. **`log_empty_repo`** — unborn HEAD → empty log, no panic. Why: edge.
**General:**
9. **`diff_log_do_not_mutate`** — HEAD-oid before==after. Why: **forbidden #6**.
10. **`diff_log_non_git_degraded`** — non-git/missing path → degraded (empty/None), no panic. Why: robustness.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none.** `FileChange`/`CommitInfo`/`ChangeKind` are **daemon-internal**.
- **Shared-contract seam model touched?** **NO** — no envelope/ID/status-machine/catalog/`EventTypeRegistry` change → **no schema-snapshot, no CONTRACT_VERSION**.
- **Orchestrator doc rows to write hot:** none new this slice.

## Things to flag at Step 2.5
1. **Diff signature / target.** My default vote: `read_diff(repo_path, from: Option<&str>, to: Option<&str>)` — `(None, None)` = working-tree-vs-HEAD; `(Some(base), Some(head))` = ref-vs-ref; `(Some(base), None)` = base-vs-working-tree. Confirm (vs. two separate fns `diff_worktree` + `diff_refs`).
2. **Diff granularity.** My default vote: **file-level** (`FileChange` with path/kind/line-counts) now; per-**hunk** diff (for the diff UI's hunk view) is deferred to a 7.2-adjacent slice. Confirm.
3. **Rename detection.** My default vote: **off for MVP** (a rename reads as Delete+Add) — `git2` `find_similar` is opt-in; add it only when the diff UI needs rename arrows. Confirm.
4. **Log fields + default limit.** My default vote: `CommitInfo { sha, summary (first line), author (name<email>), timestamp: Timestamp }`, default `limit` ~50, newest-first. Confirm the field set + whether `timestamp` uses the frozen `Timestamp` newtype (consumed read-only, like edges-003).

## Dependencies + sequencing
- **Depends on:** edges-002 (`git/reads.rs` + the `git2` dep). No Gateway / `shared/` dependency.
- **Blocks:** the gated 5.2-remainder (the §7.2 re-read-before-mutate executor path) + the 7.2 Code/Diff UI (the diff/log render).

## Estimated commit count
**1–2.** Bundle diff + log (same `git/reads.rs` read concern, same fixture pattern, no safety pin). Split into a diff-commit + a log-commit only if the diff grows.

## Lessons-logged candidates anticipated
- **Convention candidate** — covered by the existing git2-read-only convention (before/after HEAD-oid pin; hermetic `git2::init` fixtures) — no new lesson expected unless diff/log surface a fixture gotcha.
- **Future TODO — operational** — per-hunk diff (the diff UI's hunk view) + rename detection — deferred to a 7.2-adjacent slice.

## How to invoke
1. **Read this brief end-to-end** — Step-2.5 Q1 (the diff signature) is the main design choice.
2. **Run `/tdd git_diff_and_log_reads`.**
3. **Step 0 (Restate)** — confirm: diff + log read primitives only; per-hunk/rename/UI deferred.
4. **Step 1 (files)** — confirm; do NOT touch `gateway/`, `shared/`, `eventstore/`, migrations.
5. **Step 2.5** — send the test-design + the 4 design answers; wait for `APPROVED.`
6. **Step 9** — surface anything beyond the anticipated candidates.

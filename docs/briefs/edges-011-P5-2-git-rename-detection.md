# /tdd brief — git_rename_detection

## Feature
git2 **rename (and copy) detection** in `read_diff` — turn on `git2::Diff::find_similar` so a renamed file reads as a single `Renamed` change carrying its source path, instead of the current `Delete + Add` pair. Closes the named MVP deferral (`git/reads.rs:164` — "Rename/copy detection is OFF for the MVP (no git2 `find_similar`) → a rename reads as Delete + Add"). Pure, deterministic, git2 **read-only** (forbidden #6); a P5.2 read-backend refinement every diff consumer benefits from.

## Use case + traceability
- **Task ID:** P5.2 (in-lane — the git read backend; the `proj_worktree` projector + git-CLI mutations + watcher stay gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (git2 for hot structured reads — status/diff/log/branch), `§7.2` (Git SoT = the repo/worktree; diff is part of the live git read the §7.2 re-read-before-mutate + the §7.2 Code/Diff review consume).
- **Related context:** edges-005 (`857694d`) landed `read_diff` + `FileChange`/`ChangeKind` (file-level, rename detection OFF). The §7.2 PR Review Workspace (ui-track, gated) + the diff-open §18 bench (6.7, gated) are the downstream consumers of accurate diffs. Hermetic `git2::init` fixtures per edges-001/005 (LESSON: no shelling to git, no committed fixtures).

## Acceptance criteria (what "done" means)
- [ ] `read_diff` calls `git2::Diff::find_similar` (rename detection ON) before mapping deltas, so an add+delete pair above git's similarity threshold collapses to one `Renamed` delta.
- [ ] `ChangeKind` gains a `Renamed` variant; `ChangeKind::from_delta` maps `git2::Delta::Renamed → Renamed`.
- [ ] `FileChange` gains `old_path: Option<String>` — the rename source path (the `old_file` path) for a `Renamed` change; `None` for Added/Modified/Deleted/Other.
- [ ] A rename (commit `a.txt`, then `a.txt`→`b.txt` same content) → one `FileChange { path: "b.txt", change_kind: Renamed, old_path: Some("a.txt") }` (NOT a separate Delete + Add).
- [ ] A rename **with a small edit** (above the similarity threshold) → still `Renamed`, with `additions`/`deletions` reflecting the edit.
- [ ] A **low-similarity** "rename" (content wholly different) stays **Delete + Add** (below threshold — find_similar does not pair them); pins the threshold semantics so it isn't over-eager.
- [ ] A pure add and a pure delete keep `old_path: None` (no false rename pairing).
- [ ] **Copy detection** — included **iff** git2's copy detection (`FindOptions::copies`) tests cleanly (Step-2.5 Q1); if so, `ChangeKind::Copied` + `Delta::Copied → Copied` + `old_path = source`; else copy is a named follow-up and `Copied` is omitted (don't ship a never-produced variant).
- [ ] Existing `read_diff`/`read_log` tests in `daemon/tests/git_diff_log.rs` stay green (the find_similar addition reclassifies only true rename/copy pairs; non-paired add/delete cases are unchanged).
- [ ] git2 stays **read-only** (forbidden #6 — `find_similar` mutates only the in-memory `Diff`, never the repo; the existing `diff_log_do_not_mutate` HEAD-oid guard still holds).
- [ ] All tests pass; `/preflight` clean. Cross-doc invariant: **none** (daemon-internal git2 read types).

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated `proj_worktree` projector + the §7.2 Code/Diff review consumers.** `read_diff` is consumed by the gated worktree projector (re-read-before-mutate, §7.2) + the §7.2 PR Review Workspace (ui-track) + the diff-open §18 bench (6.7). Tested-but-unwired **by design** (Approach A) — Step 7.5 grep-confirms only the module + test reference the new `Renamed`/`old_path`. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**Modified:**
- `daemon/src/git/reads.rs` — `FindOptions`/`find_similar` in `read_diff`; `ChangeKind::Renamed` (+ `Copied` per Q1); `FileChange.old_path`; the delta mapping picks `old_path` from `old_file()` for a rename/copy. Refresh the doc comments (the `:164` "rename = Delete+Add" note is now stale).
- `daemon/tests/git_diff_log.rs` — the rename/threshold/copy tests; add `old_path: None` (or field reads) where existing `FileChange` expectations need it.

If implementation needs files beyond this list, flag at Step 2.5.

## RED test outline (Step 2)
Tests in `daemon/tests/git_diff_log.rs` (hermetic `init_repo` + `commit_file` fixtures):

1. **`diff_detects_rename`** — Asserts: commit `a.txt`, move → `b.txt` (same content), `read_diff(None,None)` → one change `{path:"b.txt", Renamed, old_path:Some("a.txt")}`, no separate Delete/Add. Why: §9 — the named limitation closed.
2. **`diff_rename_with_small_edit_still_rename`** — Asserts: rename + a one-line edit (above threshold) → `Renamed` with non-zero additions/deletions. Why: §9 similarity threshold tolerates edits.
3. **`diff_low_similarity_is_add_delete_not_rename`** — Asserts: `a.txt` deleted + `b.txt` (wholly different content) added → two changes (Deleted + Added), neither `Renamed`. Why: §9 — find_similar isn't over-eager (threshold floor).
4. **`diff_pure_add_has_no_old_path`** — Asserts: a new file → `Added`, `old_path:None`. Why: no false rename pairing.
5. **`diff_pure_delete_has_no_old_path`** — Asserts: a removed tracked file (no matching add) → `Deleted`, `old_path:None`. Why: no false pairing.
6. **`diff_detects_copy`** *(only if Q1 includes copy)* — Asserts: commit `a.txt`, add `b.txt` (same content) keeping `a.txt` → `{path:"b.txt", Copied, old_path:Some("a.txt")}`. Why: §9 copy detection. (Omit the test + the `Copied` variant if Q1 defers copy.)
7. **`rename_detection_read_only`** — Asserts: `read_diff` over a rename leaves HEAD oid unchanged (extends/parallels the existing `diff_log_do_not_mutate`). Why: forbidden #6 — find_similar must not mutate.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `FileChange` (+`old_path`) + `ChangeKind` (+`Renamed`[,`Copied`]) are **daemon-internal** (not a `shared/` contract — UI/Brain don't read `git::reads` types) → no Appendix-A row, no CONTRACT bump. Confirm "none" at Step 9.
- **Orchestrator doc rows to write hot (Step 9 routing):** none for `daemon/CLAUDE.md`/Appendix A. **Anticipated (integration-owned — FLAG, I route):** a small §9 arch-note (rename/copy detection is ON in the git2 diff read; the threshold semantics) + a possible C-list lesson if find_similar surfaces a gotcha (e.g. the workdir-diff rename-pairing needs `include_untracked` on, or copy detection's option fiddliness).
- **Shared-contract seam model touched?** **NO** — daemon-internal git2 read types only; no envelope/ID/status-machine/catalog/`EventTypeRegistry` change → no schema-snapshot, no CONTRACT_VERSION.

## Things to flag at Step 2.5
1. **Copy detection — include or defer?** git2 rename detection (`FindOptions::renames(true)`) is reliable; **copy** detection (`.copies(true)`) is finicky (git only detects copies from files touched in the same diff unless `.copies_from_unmodified(true)`, and it's costlier). **Default vote: include copy IF a clean `.copies(true)` test passes; otherwise ship rename-only this slice + a named copy follow-up** (and DON'T add a `Copied` variant that's never produced). Spike git2 0.21's copy behavior and decide.
2. **find_similar options / threshold.** Default git threshold (50% similarity) vs. an explicit `FindOptions` tuning. **Default vote: git's default** (matches `git diff -M` behavior the user sees in their terminal — the §7.2 "matches the user's terminal git" intent); pin #2/#3 against the default. Confirm `find_similar` needs no special flags for the **workdir** diff (the existing `read_diff` already sets `include_untracked` + `show_untracked_content`, so the add side is visible to pair).
3. **Unconditional vs opt-in.** Turn rename detection ON for all `read_diff` calls (default) vs. a `detect_renames: bool` param. **Default vote: unconditional** — accurate renames are always wanted, find_similar is cheap for typical diffs, and it matches terminal `git diff -M`/rename-aware tools; avoids API churn. (A perf note for huge diffs → a future-TODO, not a param now.)
4. **`old_path` only on rename/copy.** `old_path: Some(_)` only for `Renamed`/`Copied`; `None` otherwise (a Modified/Deleted file's path is already in `path`). **Default vote: as stated.**

## Dependencies + sequencing
- **Depends on:** edges-005 (`857694d`) `read_diff`/`FileChange`/`ChangeKind`.
- **Blocks:** the gated `proj_worktree` projector (accurate diffs in the worktree read) + the §7.2 PR Review Workspace per-hunk/per-file actions (ui-track) + the 6.7 diff-open §18 bench. **Named follow-up (NOT this slice):** per-hunk diff granularity (`git/reads.rs:151` — a `DiffHunk` read for the §7.2 per-hunk-actions UI) — defer until its §7.2 consumer is closer.

## Estimated commit count
**1** — a focused rename(/copy) refinement of `read_diff` in one module; deterministic git2, hermetic fixtures, no safety invariant, no cross-doc change. ~40–70 lines + tests.

## Lessons-logged candidates anticipated
- **Convention candidate** *(if a gotcha surfaces)* — git2 rename detection = `Diff::find_similar(FindOptions::renames(true))` post-diff (mutates only the in-memory `Diff`, stays read-only/forbidden-#6); the workdir add-side must be visible (`include_untracked`/`show_untracked_content`, already on) to pair a rename; the similarity threshold gates over-eager pairing (low-similarity stays Add+Delete). Copy detection is opt-in + finicky (flag if deferred).
- **Architecture-doc note candidate** — rename/copy detection is ON in the §9 git2 diff read (matches the user's terminal `git diff -M`); the threshold semantics.

## How to invoke
1. **Read this brief end-to-end** — esp. the Step-2.5 copy-vs-defer + threshold questions.
2. **Run `/tdd git_rename_detection`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (files)** — confirm `git/reads.rs` + `tests/git_diff_log.rs`.
5. **Step 2.5** — send the test-design write-up + the copy-include/defer decision (spike git2 0.21 copy) before GREEN; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 9** — surface cross-doc "none", and any find_similar gotcha → the §9 arch-note / C-list lesson (integration-owned — flag, I route).

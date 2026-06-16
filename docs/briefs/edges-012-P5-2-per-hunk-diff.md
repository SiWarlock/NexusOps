# /tdd brief — per_hunk_diff

## Feature
A git2 **per-hunk diff read** — `read_file_hunks` returns the structured hunks (range + header + per-line content) for a single file's change, completing the §9 diff read backend below `read_diff`'s file-level summary. This is the daemon-side data for the **locked O-6 §7.2 PR Review Workspace** per-hunk actions (approve/comment/ask-agent per hunk) + diff rendering. Closes the named deferral (`git/reads.rs:151` — "per-hunk diffs are a 7.2-adjacent slice"). Pure, deterministic, git2 **read-only** (forbidden #6).

## Use case + traceability
- **Task ID:** P5.2 (in-lane — the git read backend; the §7.2 Code/Diff review UI consumer is ui-track + gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (git2 for hot structured reads — diff), `§7.2` (git SoT; the Code/Diff review reads git2 live; O-6 PR Review Workspace per-hunk actions).
- **Related context:** edges-005 (`857694d`) `read_diff` (file-level `FileChange` — `additions`/`deletions` from `Patch::line_stats`; the per-file `Patch` this slice iterates for hunks). edges-011 (`fcf3ba9`) rename detection (a renamed-with-edit file's hunks reflect the content edit). The §7.2 PR Review Workspace (6.3e, ui-track/parked) + the 6.7 diff-open §18 bench are the downstream consumers. Hermetic `git2::init` fixtures per edges-005/011.

## Acceptance criteria (what "done" means)
- [ ] `read_file_hunks(path, file, from, to) -> Vec<DiffHunk>` returns the hunks for the named `file`'s change in the selected diff (`(from,to)` selects the target exactly as `read_diff`: `(None,None)` workdir-vs-HEAD, `(Some,Some)` ref-vs-ref, `(Some,None)` base-vs-workdir).
- [ ] `DiffHunk` carries the hunk geometry — `old_start`, `old_lines`, `new_start`, `new_lines` (from git2 `DiffHunk`) + the `header` string + the per-line content.
- [ ] `DiffLine` carries `kind` (Addition `+` / Deletion `-` / Context ` `), `content` (the line text), and `old_lineno`/`new_lineno` (`Option<u32>` — an addition has `new_lineno` Some + `old_lineno` None; a deletion the reverse; context both).
- [ ] A single-hunk modification → one `DiffHunk` with the correct geometry + lines; a **multi-hunk** modification (edits in two far-apart regions) → multiple `DiffHunk`s in order.
- [ ] A **binary** file → an empty hunk list (git2 `Patch::from_diff` is `None` for binary — no panic, typed absence).
- [ ] The `file` filter scopes the read to that one file (a 2-file diff → `read_file_hunks(.., "a.txt", ..)` returns only `a.txt`'s hunks).
- [ ] A non-git path / a clean tree / an unresolvable ref / a file with no change → an empty list (degrade-not-panic, matching `read_diff`).
- [ ] git2 stays **read-only** (forbidden #6 — `Patch`/`DiffHunk`/`DiffLine` are read APIs; a `read_file_hunks` over a change leaves HEAD oid unchanged).
- [ ] Every extraction path is **total** (no `unwrap` on git2 results — `Option`/`Result`-guarded; a hunk/line read error degrades that hunk/line, never panics).
- [ ] All tests pass; `/preflight` clean. Cross-doc invariant: **none** (daemon-internal git2 read types).

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated §7.2 Code/Diff review (6.3e, ui-track) + the gated `proj_worktree`/PR-review read path.** `read_file_hunks` is consumed by the gated §7.2 PR Review Workspace (per-hunk actions + diff render) + the 6.7 diff-open bench. Tested-but-unwired **by design** (Approach A) — Step 7.5 grep-confirms only the module + test reference the new symbols. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**Modified:**
- `daemon/src/git/reads.rs` — `DiffHunk` + `DiffLine` + `DiffLineKind` types + `read_file_hunks` (iterate the per-file `Patch`'s hunks + lines). Refresh the `:151` doc comment (per-hunk now exists).
- `daemon/tests/git_diff_log.rs` (extend, reusing `init_repo`/`commit_file`) **or** a new `daemon/tests/git_hunks.rs` (Step-2.5 Q3) — the hunk-geometry / multi-hunk / line-kind / binary / file-filter / read-only tests.

If implementation needs files beyond this list, flag at Step 2.5.

## RED test outline (Step 2)
Tests (hermetic `init_repo` + `commit_file` fixtures):

1. **`hunks_single_modification`** — Asserts: commit a multi-line `a.txt`, edit one line, `read_file_hunks(None,None,"a.txt")` → one `DiffHunk` whose `new_start`/`new_lines` cover the edit + a `Deletion` line (old) and an `Addition` line (new). Why: §9 hunk geometry + line kinds.
2. **`hunks_multiple_regions`** — Asserts: edit two far-apart regions of `a.txt` → ≥2 `DiffHunk`s, in file order. Why: §9 per-hunk granularity (the point of the slice).
3. **`hunk_line_kinds_and_linenos`** — Asserts: an addition line has `kind:Addition, new_lineno:Some, old_lineno:None`; a deletion `kind:Deletion, old_lineno:Some, new_lineno:None`; a context line both Some. Why: §7.2 the UI maps lines to gutter line numbers.
4. **`hunks_binary_file_is_empty`** — Asserts: a binary file change → empty hunk list (no panic). Why: §9 typed absence (`Patch::from_diff` None for binary).
5. **`hunks_file_filter_scopes`** — Asserts: a diff touching `a.txt` + `b.txt` → `read_file_hunks(.., "a.txt", ..)` returns only `a.txt`'s hunks. Why: the single-file targeting.
6. **`hunks_non_git_and_clean_empty`** — Asserts: a non-git path and a clean tree → empty list. Why: degrade-not-panic (read_diff parity).
7. **`read_file_hunks_read_only`** — Asserts: `read_file_hunks` over a change leaves HEAD oid unchanged. Why: forbidden #6.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `DiffHunk`/`DiffLine`/`DiffLineKind` are **NEW daemon-internal** git2 read types (not a `shared/` contract — UI/Brain don't read `git::reads` types) → no Appendix-A row, no CONTRACT bump.
- **Orchestrator doc rows to write hot (Step 9 routing):** none for `daemon/CLAUDE.md`/Appendix A. **Anticipated (integration-owned — FLAG, I route):** a small §9/§7.2 arch-note (the per-hunk read shape feeding the O-6 PR Review Workspace) + a C-list lesson if the git2 `Patch`/`line_in_hunk` iteration surfaces a gotcha (e.g. EOFNL line origins, the header `&[u8]` UTF-8 handling).
- **Shared-contract seam model touched?** **NO** — daemon-internal git2 read types; no envelope/ID/status-machine/catalog/`EventTypeRegistry` change → no schema-snapshot, no CONTRACT_VERSION.

## Things to flag at Step 2.5
1. **git2 API confirmation (spike it like find_similar).** Confirm against git2 0.21: `Patch::from_diff(&diff, idx) -> Result<Option<Patch>>`; `Patch::num_hunks()`; `Patch::hunk(i) -> Result<(DiffHunk, usize)>`; `Patch::num_lines_in_hunk(i)` + `Patch::line_in_hunk(hunk_i, line_i) -> Result<DiffLine>`; `DiffHunk::{old_start,old_lines,new_start,new_lines,header}` (header is `&[u8]`); `DiffLine::{origin()/origin_value(), content() -> &[u8], old_lineno()/new_lineno() -> Option<u32>}`. **Default lean:** iterate hunks 0..num_hunks, lines 0..num_lines_in_hunk; map origin `+`/`-`/` ` → Addition/Deletion/Context; UTF-8-lossy the `&[u8]` content/header.
2. **Line-origin mapping for non +/-/space.** git2 `DiffLine` origins also include EOFNL markers (`<`/`>`/`=`) + (in full diffs) file/hunk headers — but `line_in_hunk` yields only in-hunk lines (so mostly `+`/`-`/` `, plus the EOFNL "\ No newline at end of file" markers). **Default vote:** map `+`→Addition, `-`→Deletion, everything else (` `, EOFNL) → Context (the EOFNL marker rides as a context-ish line; it's display text, not a +/- change). Confirm at the spike + pin the chosen mapping.
3. **Test-file location.** Extend `tests/git_diff_log.rs` (reuse `init_repo`/`commit_file`) vs. a new `tests/git_hunks.rs` (own copies). **Default vote: extend `git_diff_log.rs`** — same fixtures, the hunk read is the file-level read's sibling; keep the git-diff read tests together.
4. **Scope — lines now, or ranges-only?** Include the per-line content (`DiffLine`) now (the UI needs lines to render), or ship hunk geometry (ranges + header) only and defer lines? **Default vote: include lines** — ranges-without-lines is half a feature (the §7.2 review can't render the diff from geometry alone). If the slice balloons, the natural split is geometry (this slice) / lines (a follow-up) — your call, but lines is the intended complete read.
5. **Single-file vs all-files.** `read_file_hunks(path, file, ..)` (one file — the UI opens one file's diff at a time, after `read_diff` gives the file tree) vs. an all-files `Vec<(file, Vec<DiffHunk>)>`. **Default vote: single-file** — targeted, avoids loading every file's hunks for a large PR; `read_diff` already supplies the file list.

## Dependencies + sequencing
- **Depends on:** edges-005 (`857694d`) `read_diff` + the per-file `Patch` path. edges-011 (`fcf3ba9`) rename detection (a renamed-with-edit file's hunks).
- **Blocks:** the gated §7.2 PR Review Workspace per-hunk actions (6.3e, ui-track) + the 6.7 diff-open §18 bench. **Completes the P5.2 git diff read backend** (status/diff/log/branch + rename + per-hunk). **Next in-lane after this:** the Linear read client (the second integration vertical — a multi-slice greenfield: a Linear issue-state derivation foundation, then the Linear GraphQL read client) is the headline.

## Estimated commit count
**1** — a focused per-hunk read in one module; deterministic git2, hermetic fixtures, no safety invariant, no cross-doc change. ~90–130 lines + tests. Split to geometry/lines only if Step-2.5 Q4 fires.

## Lessons-logged candidates anticipated
- **Convention candidate** *(if a gotcha surfaces)* — the git2 per-hunk read = iterate `Patch::hunk(i)` (geometry) + `Patch::line_in_hunk(i,j)` (lines); `Patch::from_diff` is `None` for a binary file (→ empty, typed absence); `DiffHunk::header`/`DiffLine::content` are `&[u8]` (UTF-8-lossy); the EOFNL/non-+/- origins map to Context. Read-only (forbidden #6).
- **Architecture-doc note candidate** — the per-hunk read shape (`DiffHunk`/`DiffLine`) under §9/§7.2, feeding the O-6 PR Review Workspace.

## How to invoke
1. **Read this brief end-to-end** — esp. the Step-2.5 git2-API spike + the lines-vs-geometry scope question.
2. **Run `/tdd per_hunk_diff`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (files)** — confirm `git/reads.rs` + the test file.
5. **Step 2.5** — send the test-design write-up + the confirmed git2 `Patch`/`DiffHunk`/`DiffLine` API + the scope (lines-vs-geometry) decision before GREEN; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 9** — surface cross-doc "none" + any git2 iteration gotcha → the §9 arch-note / C-list lesson (integration-owned — flag, I route).

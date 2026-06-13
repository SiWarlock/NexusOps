# /tdd brief — open_diff_dry_refactor

## Feature
A **behavior-preserving DRY refactor** of `git/reads.rs`: extract the diff-construction block duplicated
between `read_diff` and `read_file_hunks` (`Repository`-based `DiffOptions` setup → `resolve_tree` →
the `(from,to)` tree/workdir branching → rename-aware `find_similar`) into one private `open_diff`
helper. No behavior change — the existing diff-read suite is the guard. Closes the named follow-up at
`git/reads.rs:368`.

## Use case + traceability
- **Task ID:** P5.2 (in-lane — the git diff read backend; the §7.2 Code/Diff review consumer is ui-track + gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (git2 for hot structured reads — diff), `§7.2` (git SoT; the Code/Diff review reads git2 live). *Refactor — no spec behavior change; same anchors as the code it consolidates.*
- **Related context:** the two functions being DRY'd — `read_diff` (edges-005 `857694d`, file-level `FileChange`) + `read_file_hunks` (edges-012, per-hunk), both rename-aware via edges-011 (`fcf3ba9`). The shared construction was flagged a follow-up at `git/reads.rs:368`.

## Acceptance criteria (what "done" means)
- [ ] A private `open_diff` helper exists in `git/reads.rs` that builds the rename-aware `git2::Diff`
      for `(from, to)` and returns `None` on any early-out (unresolvable ref / unborn-HEAD-with-no-base /
      diff failure) — i.e. the current inline block's behavior, verbatim.
- [ ] `read_diff` and `read_file_hunks` both delegate diff-construction to `open_diff`; the duplicated
      inline block is gone from both; each keeps only its divergent tail (delta→`FileChange` map vs
      target-delta→hunk extraction).
- [ ] **Behavior-preserving:** the full `daemon/tests/git_diff_log.rs` suite (28 tests) stays GREEN with
      byte-identical results — incl. the forbidden-#6 read-only guards (`rename_detection_read_only` /
      `diff_log_do_not_mutate`) and the rename/per-hunk/`(from,to)`-mode cases. No new behavior, so **no
      new RED test** (see RED section).
- [ ] `/preflight` clean (`cargo fmt --check && clippy -D warnings && check && test`).
- [ ] No `shared/` change, no `CONTRACT_VERSION` bump (private helper; daemon-internal).

## Wiring / entry point (Step 7.5)
**No new entry point.** `read_diff` + `read_file_hunks` remain the production read entries (the §7.2
git-SoT read surface); they now delegate the shared diff-construction to `open_diff`. The §7.2 Code/Diff
review consumer (ui-track, gated) is unchanged — this is an internal extraction behind the existing
public reads. `/wired read_diff` / `/wired read_file_hunks` are unchanged from edges-005/012.

## Files expected to touch
**Modified:**
- `daemon/src/git/reads.rs` — add the private `open_diff` helper; rewire `read_diff` (211-249 block) +
  `read_file_hunks` (346-371 block) to call it; consolidate the richer `find_similar` doc comment onto
  the helper (drop the terser duplicate). No test-file change expected.

If implementation needs files beyond this list, **flag at Step 2.5** before proceeding.

## RED test outline (Step 2)
**This is a behavior-preserving refactor — there is NO new RED test.** The "test-first" guard is the
**already-green** `daemon/tests/git_diff_log.rs` (28 tests): they pin the exact diff-read behavior
(file-level counts, per-hunk extraction, rename detection, the three `(from,to)` modes, the read-only
invariant). The refactor is correct iff they stay byte-identically green before → after.

> If the implementer judges the "the two sibling reads agree on the same diff" property under-pinned,
> an **optional** belt-and-suspenders assertion (`read_diff` + `read_file_hunks` over one fixture both
> reflect the same renamed-with-edit delta) is a fine ADD — but it's likely already covered by edges-012's
> cases; flag at Step 2.5 rather than padding.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NONE. `open_diff` is a private helper; no public signature changes (`read_diff`/
  `read_file_hunks` signatures unchanged); no `shared/` surface; no `CONTRACT_VERSION` bump.
- **Shared-contract (cross-track) seam model touched?** NO → no schema-snapshot test.
- **Orchestrator doc rows to write hot:** none. (A clean refactor; no arch-note unless the helper surfaces
  something — unlikely.)

## Things to flag at Step 2.5
1. **`open_diff` signature.** (A) `fn open_diff<'r>(repo: &'r Repository, from: Option<&str>, to: Option<&str>)
   -> Option<git2::Diff<'r>>` — the caller keeps its one-line `Repository::discover(path)` (the `Diff`
   borrows the caller's `repo`, so the repo must outlive it — the helper can't own+return). (B) a closure
   form `with_diff(path, from, to, |diff| …)` that owns repo+diff. My default vote: **A** — idiomatic
   lifetime (`Diff<'r>` tied to `&'r Repository`); the 1-line `discover` staying per-caller is trivial;
   B adds HRTB-closure complexity for no real gain. Both callers drop their `let mut diff` (the helper
   runs `find_similar` internally and returns the post-find Diff used immutably).
2. **Scope of the extraction.** Extract ONLY the shared construction (discover-result in, rename-aware
   `Diff` out); keep each function's divergent tail in place (`read_diff`'s delta→`FileChange` map;
   `read_file_hunks`'s target-find + `Patch::from_diff`). Default: **yes, construction-only** — don't
   over-DRY the tails (they genuinely differ).
3. **Doc-comment consolidation.** Carry the richer `read_diff` `find_similar` comment (for_untracked
   rationale + similarity-threshold + copy-detection-off note) onto the helper; drop the terser
   `read_file_hunks` duplicate. Default: **yes.**

## Dependencies + sequencing
- **Depends on:** edges-005 (`read_diff`), edges-011 (rename detection), edges-012 (`read_file_hunks`). All LANDED.
- **Blocks:** nothing. Pure internal hygiene. (Note: copy-detection — the other `find_similar` follow-up —
  is NOT in scope; it's a git2-0.21 limitation captured separately as a finding-doc.)

## Estimated commit count
**1.** A single contained refactor in one file (`git/reads.rs`); behavior-preserving; no safety invariant.

## Reviewer posture (Step 8)
- **security-reviewer:** policy `invariant` → **SKIP** (no safety invariant; git2 stays read-only — forbidden
  #6 — `find_similar` mutates only the in-memory `Diff`; the existing read-only guard tests stay green). Note the skip.
- **code-quality-reviewer:** policy `every-slice` → runs on the slice diff (a refactor is exactly its wheelhouse —
  watch for an accidental behavior change in the extracted lifetimes / early-returns).

## Lessons-logged candidates anticipated
- None expected (clean refactor). Possible low-confidence convention candidate only if the `open_diff`
  borrow-the-repo pattern reads as worth restating for future git2 reads — flag at Step 9 if so.

# /tdd brief — worktree_status_reads_and_precedence

## Feature
The git2 **read-only worktree-status backend** (`git/reads.rs`) + the **§5.1-R7 two-axis worktree-status precedence fn** (`git/precedence.rs`) — the deterministic in-lane core of Phase 5.2's git read side. The `proj_worktree` projector + the git-CLI worktree/branch mutation executors + the git watcher that *consume* these are **deferred to a later wiring slice** (cross-track-gated — same Finding as the 5.1 wiring: worktree event types in `shared/` + the sealed `gateway/` executor seam).

> **Numbering note:** this is `edges-002` = the **5.2 read-backend**. The 5.1 *wiring* slice (executor arm + detection event + registry migration + projector) referenced in edges-001's Step-9 flags remains **gated** and gets its own number when the cross-track Finding resolves — those flags fold into *that* slice, not this one.

## Use case + traceability
- **Task ID:** P5.2 (the read-backend + precedence portion; the projector/mutations/watcher are the gated remainder)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.1` (Worktree = derived 2-axis status, R-7), `§7.2` (Git/FS SoT = the repo; re-read git2 live before any mutation; `git_checked_at` staleness on cache), `§9` (git2 for status/branch/worktree reads — relative-worktree repos are git2-readable per OQ-INT-SPIKE-6). _(The precedence ordering's salience rationale draws on the project's status→attention-rank ordering — a ui-side concept, referenced here only as design rationale, not implemented by this slice.)_
- **Related context:** edges-001 (`daemon/src/git/` module + the `git2` dep this extends); the frozen `WorktreeGit` / `WorktreeOverlay` enums (`shared/src/status.rs:67`/`:75`, **consumed read-only**); the `proj_worktree` DDL (`daemon/src/eventstore/schema.rs:161`, M3 — the derived `status` + `dirty_state`/`ahead_count`/`behind_count`/`last_commit_sha`/`git_checked_at` fields a later projector will populate from these reads).

## Acceptance criteria (what "done" means)
**git2 worktree-status reads (`daemon/src/git/reads.rs`):**
- [ ] Working-tree status maps to `WorktreeGit`: clean→`Clean`, modified/staged tracked changes→`Dirty`, untracked-only→`UntrackedFiles`, unmerged index→`Conflicts`.
- [ ] Ahead/behind vs a base branch → `ahead_count`/`behind_count` + the `AheadOfBase`/`BehindBase` signal.
- [ ] Current branch name + HEAD commit sha (`last_commit_sha`) read.
- [ ] Linked worktree-list read (git2 `worktrees()`).
- [ ] git2 is **READ-ONLY** (forbidden #6); the reads **do not mutate** the repo (before/after HEAD-oid pin).
- [ ] A **relative-worktree** repo reads via git2 (NOT a CLI fallback) — per §9 / OQ-INT-SPIKE-6. (The plan's 5.2 "relative-worktrees → CLI-read fallback" text predates the spike resolution; **reconcile-flagged** — CLI fallback is reserved for the sparse-checkout misreport gap only.)
- [ ] Degraded-not-panic on a non-git / inaccessible path (consistent with edges-001's infallible-degraded posture; no `unwrap`/`expect` in non-test).

**§5.1-R7 precedence fn (`daemon/src/git/precedence.rs`):**
- [ ] `derive_worktree_status(git: WorktreeGit, overlay: Option<WorktreeOverlay>) -> <derived>` returns the single most-salient §5.1 status from the two axes per a precedence order. **Pure** — both axes are params (the overlay source is event-driven + gated; the fn is testable now with constructed values).
- [ ] **Load-bearing precedence pins (must hold):** `Conflicts` (git) AND `Locked` (overlay) both **outrank `Dirty`** (the plan's "precedence collapse locked/conflicts>dirty"); `Deleted` (overlay terminal) **dominates all**.
- [ ] Baseline cases (`Clean` + no overlay / `Creating`) resolve deterministically.
- [ ] The derived value serializes to a **frozen `WorktreeGit` ∪ `WorktreeOverlay` wire string** (no new enum, no contract).

**General:**
- [ ] Unit/integration tests pass; `/preflight` clean. **No `shared/` touch, no migration, no `gateway/` touch** (git2 dep already present from edges-001).

## Wiring / entry point (Step 7.5)
**`none — wiring lands in the gated 5.2 remainder slice.`** The reads + precedence fn are a pure library; their consumers — the `proj_worktree` projector (folds worktree lifecycle events → rows, *calling* these reads + the precedence fn), the git-CLI `git.create_worktree`/`create_branch` mutation executors (`gateway/`, sealed), and the git watcher — are all **cross-track-gated** (worktree event types in `shared/` + the executor seam — the same Finding raised for the 5.1 wiring). Reachability is **intentionally deferred** (named), not an oversight.

## Files expected to touch
**New:**
- `daemon/src/git/reads.rs` — git2 read-only worktree-status reads → a daemon-internal `WorktreeGitState { git_axis: WorktreeGit, ahead_count, behind_count, branch, last_commit_sha }` (final shape is a Step-1 detail)
- `daemon/src/git/precedence.rs` — the §5.1-R7 `derive_worktree_status` precedence fn (pure)
- Test file(s): `daemon/tests/worktree_reads.rs` (or extend `tests/detect.rs` — impl's Step-1 choice)

**Modified:**
- `daemon/src/git/mod.rs` — `pub mod reads;` + `pub mod precedence;`

No `Cargo.toml` change (git2 added in edges-001). If implementation needs files beyond this, **flag at Step 2.5**. **Do NOT touch `gateway/`, `shared/`, or any migration.**

## RED test outline (Step 2)
**Reads (hermetic `tempfile` + `git2::Repository::init` fixtures, per edges-001):**
1. **`worktree_read_clean`** — fresh commit, clean tree → `WorktreeGit::Clean`. Why: §5.1 git-axis.
2. **`worktree_read_dirty`** — modified tracked file → `Dirty`. Why: §7.2 git-state.
3. **`worktree_read_untracked`** — untracked-only → `UntrackedFiles`. Why: §5.1 (distinct from Dirty).
4. **`worktree_read_conflicts`** — unmerged/conflicted index → `Conflicts`. Why: §5.1; the precedence-critical state.
5. **`worktree_read_ahead_behind`** — commits ahead + behind a base → `ahead_count`/`behind_count` + the Ahead/Behind signal. Why: §5.1 divergence axis.
6. **`worktree_read_branch_and_head`** — branch name + HEAD sha. Why: §7.2 git-state.
7. **`worktree_read_worktree_list`** — linked worktrees enumerated. Why: §9 worktree-list read.
8. **`worktree_read_does_not_mutate`** — HEAD-oid before==after. Why: **forbidden #6**.
9. **`worktree_read_relative_worktrees`** — a `extensions.relativeWorktrees` repo reads via git2 (no CLI). Why: §9 / OQ-INT-SPIKE-6.
10. **`worktree_read_non_git_degraded`** — non-git/missing path → degraded, no panic. Why: robustness (edges-001 posture).

**Precedence (pure — constructed enum inputs):**
11. **`precedence_conflicts_over_dirty`** — `derive(Conflicts, None)` outranks a dirty result. Why: plan pin.
12. **`precedence_locked_over_dirty`** — `derive(Dirty, Some(Locked))` → `locked`. Why: plan pin.
13. **`precedence_deleted_dominates`** — `derive(<any git>, Some(Deleted))` → `deleted`. Why: terminal dominance.
14. **`precedence_clean_baseline`** — `derive(Clean, None)` → `clean`; `derive(Clean, Some(Creating))` → deterministic. Why: baseline.
15. **`precedence_full_order`** — table test over the agreed ordering (asserts the total precedence). Why: §5.1 R-7 (the derived 2-axis status).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none.** `WorktreeGitState` is daemon-internal; the precedence fn **consumes** frozen `WorktreeGit`/`WorktreeOverlay` read-only.
- **Shared-contract seam model touched?** **NO** — consuming a frozen enum ≠ changing it; no new field/value, no envelope/ID/status-machine/catalog/`EventTypeRegistry` change → **no schema-snapshot test, no CONTRACT_VERSION**.
- **Orchestrator doc rows to write hot:** one **§9 reconcile note** (the plan's "relative-worktrees → CLI fallback" is stale vs §9's post-spike position — git2 reads them; CLI fallback = sparse-checkout only). Multi-track: ARCHITECTURE.md is integration-owned, so I route this at the edges round close-out, not hot in the worktree. Flag it at Step 9 so I capture it.

## Things to flag at Step 2.5
1. **The precedence ordering (THE design question).** **[CORRECTED 2026-06-12 — the original draft default inverted the LOCKED §5.1 R-7 (`ARCHITECTURE.md:53`), which pins `locked/conflicts > dirty > ahead/behind > clean`; the implementer caught it at Step 2.5 and correctly honored §5.1. Architecture is authoritative.]** The **§5.1-faithful** total order (most-salient first → the derived status): `deleted` > `conflicts` > `locked` > `merged` > `pr_open` > `prunable` > `dirty` > `untracked_files` > `behind_base` > `ahead_of_base` > `creating` > `clean`. **Locked §5.1 R-7 pins (must hold):** `locked`/`conflicts` > `dirty` > `ahead`/`behind` > `clean`; `deleted` dominates (terminal). The **unranked** values (`untracked_files`/`merged`/`pr_open`/`prunable`/`creating`) are the daemon's total-order **extension** of §5.1's partial spec — low-stakes (the derived status is the headline; `proj_worktree` stores `dirty_state`/`ahead_count`/`behind_count`/`pr_status` separately, so no signal is lost) → record the full order as a §5.1-R7/§7.2 arch note. The within-git-axis read resolution mirrors the same pin (`Conflicts > Dirty > UntrackedFiles > BehindBase > AheadOfBase > Clean`).
2. **Derived-status representation.** My default vote: a daemon-internal `enum DerivedWorktreeStatus { Git(WorktreeGit), Overlay(WorktreeOverlay) }` with an `.as_wire_str()` → the frozen §5.1 snake_case value (type-safe + obviously frozen-enum-sourced). Alt: return the winning enum serialized to `String`. **Default vote: the wrapper enum.**
3. **Base-branch source for ahead/behind.** The read needs a base ref to compute divergence. My default vote: **an explicit `base: &str` param** the caller (the later projector, from `proj_worktree.base_branch`) supplies — the read doesn't guess a base. Confirm.
4. **Conflict/unmerged detection mechanism.** My default vote: git2 `Status` flags (`CONFLICTED`/unmerged entries) → `Conflicts`. Confirm git2 surfaces unmerged reliably in your fixtures (the spike validated reads, but pin it with test #4).

## Dependencies + sequencing
- **Depends on:** edges-001 (the `git/` module + the `git2` dep). **No Gateway / `shared/` dependency** — pure read + logic.
- **Blocks:** the gated 5.2-remainder slice (the `proj_worktree` projector + git-CLI worktree/branch mutation executors + git watcher) — cross-track-gated on worktree event types + the executor seam (same Finding).

## Estimated commit count
**1–2.** Bundle the reads + the precedence fn (same worktree-status concern, same `git/` module, **no safety-invariant pin** — forbidden #6 is enforced by a behavior test). Split into a reads-commit + a precedence-commit only if the diff grows large.

## Lessons-logged candidates anticipated
- **Convention candidate** — "the §5.1-R7 two-axis worktree precedence is a pure fn (both axes as params, table-tested); git2 worktree reads are read-only (forbidden #6, before/after HEAD-oid pin) over hermetic `git2::init` fixtures."
- **Architecture-doc note candidate** — the §9 relative-worktrees-via-git2 reconcile (the plan's 5.2 "→CLI fallback" text is stale post-OQ-INT-SPIKE-6).
- **Future TODO — operational** — `diff`/`log` reads (the other half of 5.2's "status/diff/log/branch") serve the Code/Diff + PR surfaces (7.2) — defer to a diff-read slice unless bundled later.

## How to invoke
1. **Read this brief end-to-end** — especially Step-2.5 Q1 (the precedence ordering needs your scrutiny before GREEN).
2. **Run `/tdd worktree_status_reads_and_precedence`.**
3. **Step 0 (Restate)** — confirm: reads + precedence fn only; projector/mutations/watcher deferred.
4. **Step 1 (files)** — confirm against the list; do NOT touch `gateway/`, `shared/`, migrations.
5. **Step 2.5** — send the test-design write-up + the 4 design answers (esp. the precedence ordering); wait for `APPROVED.` before GREEN.
6. **Step 9** — surface anything beyond the anticipated candidates (incl. the §9 reconcile note so I capture it).

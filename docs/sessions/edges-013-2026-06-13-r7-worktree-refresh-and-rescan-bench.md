# edges-013 — R7 impl: §7.2 worktree live-read refresh + §18 project.rescan bench

**Date:** 2026-06-13
**Role:** edges-daemon-implementer (R7 — the thin final in-lane drain before the user-gated phase-exit pause)
**Predecessor:** [edges-012-2026-06-13-r6-orchestrator-round-seal.md](edges-012-2026-06-13-r6-orchestrator-round-seal.md) (R6 orch round-seal, `c131803`)
**Successor:** _(filled at the next session doc)_
**Slice commits:** `c195c7f` (edges-026) · `d800ef1` (style fmt fixup) · `44ce907` (edges-027) · branch `track/edges` · **NOT pushed, NOT merged** (the edges→main merge is the user-gated phase-exit, not this round)

> **Companion (the authoritative accumulated cross-track ledger):** `docs/planning/edges-R5-wiring-plan.md` — the R7 round-progress block + the held-for-merge PLAN-DELTA. This doc is the impl-side narrative.

## Why this session existed
R7 is the **short final in-lane drain** before the edges track PAUSES for the user-gated phase-exit (`/phase-exit 5`+`7` + the edges→main merge). The R6 seal (`c131803`) closed P7.1 Wave-D (external mutators) + the github read vertical; R7 mops up the remaining P5 §7.2 live-read story + the deferred §18 perf budget. Two in-lane slices + an orch-run `cargo audit`:
- **edges-026** — the §7.2 worktree-status live-read cache refresh (the P5.2 follow-on TODO).
- **edges-027** — the §18 `project.rescan` detection-latency benchmark (P5.4, deferred to the phase-exit cadence in R3/R4).

## What was built

### edges-026 — §7.2 worktree-status live-read cache refresh + git-watcher (`c195c7f`)
A NON-Gateway, NON-event write-actor command (`RefreshWorktreeStatus`, the DrainOnce/ReapLeases family) that reads a worktree's live git truth (`read_worktree_status`, git2 read-only) and writes `proj_worktree`'s git-axis cache columns (`dirty_state`/`ahead_count`/`behind_count`/`last_commit_sha`/`git_checked_at` + the recomputed `status`), triggered by a new git-watcher interval task. No `WorktreeStatusRefreshed` event — the git-axis is a live-read projection cache (§7.1), not event-sourced.

**Files created:**
- `daemon/src/runtime/git_watcher.rs` — the `spawn_git_watcher` interval task (30s; the drainer/reaper precedent) + `list_worktrees` (read-only enumeration via `open_read_only`, off the async worker via `spawn_blocking`).
- `daemon/tests/worktree_refresh.rs` — 8 integration tests (fill-cache · status-recompute · UTC-Z stamp · None-read · no-event/write-actor · unknown-id no-op · watcher reachability · rebuild-resets-cache).

**Files modified:**
- `daemon/src/runtime/writer.rs` — `Command::RefreshWorktreeStatus{worktree_id,path,base}` + the handler + `WriteHandle::refresh_worktree_status` (async) + `compute_worktree_cache` (the git read + `derive_worktree_status`, runtime layer).
- `daemon/src/eventstore/mod.rs` — `EventStore::refresh_worktree_status` (a thin IMMEDIATE-txn method delegating to `projections::refresh_git_cache`; stamps `git_checked_at` from the injected Clock UTC-Z).
- `daemon/src/projections/worktree.rs` — `WorktreeGitCache` (plain computed-values struct) + `refresh_git_cache` (the UPDATE; `git=None` → stamp `git_checked_at` only).
- `daemon/src/projections/mod.rs` — re-exports (`pub WorktreeGitCache`, `pub(crate) refresh_git_cache`).
- `daemon/src/runtime/mod.rs` — exports (`spawn_git_watcher`, `compute_worktree_cache`).
- `daemon/src/main.rs` — spawns the git-watcher (production entry, shutdown-watch-stopped).

### style fmt fixup (`d800ef1`)
Pure `cargo fmt` reflow of the 5 edges-026 files — `/tdd` Step 8 ran check+clippy but not `cargo fmt`, so edges-026 shipped unformatted. Zero behavior change.

### edges-027 — §18 project.rescan detection-latency bench (`44ce907`)
**Files created:**
- `daemon/benches/project_rescan.rs` — a `[[bench]] harness=false` target (`fn main()`) driving the AS-BUILT detection core (`detect_git` + `detect_workflow`) over a representative committed temp repo (~15 tracked files + origin remote + all 4 workflow signals), 1000 warm-cache iters → **median 0.44ms** (p95 0.50 / p99 0.60 / max 1.07). CI guard: **median < 50ms** (LESSON 22 — tighter than the §18 3s SLO).

**Files modified:**
- `daemon/Cargo.toml` — the `[[bench]] name="project_rescan" harness=false` entry.

## Decisions made
- **edges-026 Q1 (write path):** a NON-Gateway, NON-event `RefreshWorktreeStatus` write-actor command (the DrainOnce/ReapLeases precedent; forbidden #3 single-writer). The git-axis is a live-read cache (§7.1), so no event is appended.
- **edges-026 layer-clean:** persistence core stays git-free — the UPDATE (`projections/worktree.rs`) takes plain computed values; the git read + `derive_worktree_status` live in the runtime layer (`compute_worktree_cache`). Applies the edges-022 LESSON-17 layer rule (persistence must not import the `git/` edge).
- **edges-026 Q2 (status recompute):** `derive_worktree_status(live_git_axis, Some(Creating))` — Creating is the only emitted overlay in the MVP. (Real `derive_worktree_status` sig is `(git: WorktreeGit, overlay: Option<WorktreeOverlay>)`; status via `DerivedWorktreeStatus::as_wire_str()` — the brief Q2's `Some(git_axis)` was imprecise.)
- **edges-026 Q3 (None read):** a non-git/inaccessible path stamps `git_checked_at` only, leaving the git-axis cols + status unchanged.
- **edges-026 Q4 (watcher):** a minimal 30s interval task (reaper cadence — git reads cheap-not-free), shutdown-stopped, enumerating `proj_worktree` over a read-only conn; spawned in `main.rs` (reachable).
- **edges-026 review fixes (in-slice):** `usize`→`i64` narrowing guarded (`i64::try_from().unwrap_or(i64::MAX)`); the git-watcher now LOGS the enumeration read-error + panic arms (was silently swallowed; the drainer precedent logs).
- **edges-027 Q1 (entry):** drive the detection composition `detect_git`+`detect_workflow` directly (the §18-SLO-heavy scan core); the executor's emit (serialize + strip_userinfo + timestamp) is microseconds, not the SLO concern.
- **edges-027 Q3 (guard):** **median < 50ms**, MEDIAN-gated (a single-shot op's tail is OS-scheduling jitter on a shared CI box → the central tendency is the stable regression signal; p95/p99/max reported, not gated); gate-last so a breach still prints the baseline. ~112× margin over the measured baseline, ~60× under the 3s SLO (LESSON 22 — tighter than the SLO).

## Decisions explicitly NOT made (deferred)
- **The overlay-source clean model (MIGRATION_9-deferred):** when overlay-lifecycle event emitters (`WorktreeMerged`/`Locked`/`Prunable`/…) land, `status` recompute needs a real overlay source (an `overlay` column = MIGRATION_9, or an event-sourced overlay read) — else a merged/locked worktree's status would wrongly re-derive to a git-axis value each tick. Flagged in code + the wiring-plan ledger; not testable now (no overlay emitter exists).
- **The unified write-actor-I/O-offload hardening (SPREAD):** the git2 read runs on the write-actor std::thread (a bounded LOCAL read — not the unbounded-network-hang class the edges-023/024 timeouts guard). Deferred as ONE post-merge item covering git-watcher reads + drain_once + the external executors. `last-consumer-slice: a write-actor-I/O-offload hardening slice`.
- **A single watcher→real-repo e2e bench/test:** the watcher→real-repo cache-fill is composition-covered (test #7 watcher reachability + tests #1-#3 the refresh with a real path); a single e2e is fragile because the seeded `proj_worktree.path` is §15-redaction-masked for a high-entropy tempdir (the edges-020 over-redaction FP class).
- **The octocrab feature-prune** (the cargo-audit `rsa` follow-up) — orchestrator-owned, held-for-merge.

## TDD compliance
**Clean.** edges-026 ran the full `/tdd` discipline — 8 tests written RED-first, RED confirmed (compile failure for missing refresh APIs, the right reason), GREEN, code-quality review (no security-reviewer — no INV-SEC-1 surface). edges-027 is a **NON-TDD bench slice** under the §18 benchmark waiver (the bench IS the coverage; no RED→GREEN; the `event_write.rs` precedent + LESSON 22) — sanctioned, not a violation. **Process note (not a TDD violation):** edges-026 shipped without `cargo fmt` (`/tdd` Step 8 runs check+clippy but not fmt) — fixed in `d800ef1`; the gap is routed as a convention candidate (add `cargo fmt --check` to the per-slice gate).

## Cross-doc invariant audit
**No model field changes this session.** No `shared/` change, no contract bump (CONTRACT 0.26.0 held), no new event type, no schema change. Multi-track memory check: nothing to flag (no paired `ARCHITECTURE.md` edit owed). The arch-doc *notes* (below) are descriptive (a behavior is now LIVE), not model-field changes — routed to the orch hot.

## Reachability
- **edges-026:** `main.rs run()` [`#[tokio::main]` entry] → `spawn_git_watcher` → watcher tick → `WriteHandle::refresh_worktree_status` → `Command::RefreshWorktreeStatus` → handler → `compute_worktree_cache` + `EventStore::refresh_worktree_status` → `projections::refresh_git_cache` → UPDATE `proj_worktree`. **Reachable** (Step 7.5 traced; test #7 drives the watcher directly).
- **edges-027:** reached via `cargo bench --bench project_rescan` (the `/phase-exit` perf row + nightly) — NOT production wiring; the bench IS the coverage (the §18 bench waiver). Invisible to `cargo test --workspace`.
- No tested-but-unwired gaps.

## Open follow-ups (Step-9 categorized; already routed hot to the orch / wiring-plan ledger)
- **Architecture-doc note:** §7.2 worktree live-read cache is LIVE (git-watcher wired, ARCHITECTURE.md:340); `WorktreeStatusRefreshed`-is-NOT-an-event confirmed as-built. §18 `project.rescan` perf budget benched + guarded (median ~0.44ms ≪ 3s; guard 50ms tighter than the SLO).
- **Convention candidate (LESSON):** the live-read-cache refresh pattern — a non-Gateway/non-event write-actor command + a git-watcher interval trigger + read-time `git_checked_at` staleness; a rebuild RESETS the cache (live-read, not event-sourced); persistence-core stays git-free.
- **Convention candidate (process):** `/tdd` Step 8 should run `cargo fmt --check` (or `/preflight`) per-slice — clippy doesn't catch formatting (the edges-026 fmt-gap).
- **Future TODO (belongs-to-a-phase):** the overlay-source MIGRATION_9-deferred follow-on.
- **SPREAD (held-for-merge):** the unified write-actor-I/O-offload hardening.
- **Held-for-merge:** register `project_rescan` in the `/phase-exit` perf row + `.github/nightly.yml` at the edges→main merge.
- **Security (cargo audit, orch-run):** RUSTSEC-2023-0071 (rsa 0.9.10 Marvin Attack, MEDIUM, no fix) transitive via octocrab→jsonwebtoken→rsa — **exposure LOW** (edges never exercises GitHub-App RS256-JWT; auth deferred + the planned model is bearer-token). Accept-and-document; octocrab feature-prune follow-up. Recorded in `docs/audits/edges-P5-P7-cargo-audit.md` (orch); surfaced to the lead at the seal.

## Test count
612 (R6 seal) → **620 / 0** workspace (edges-026 +8; edges-027 bench invisible to the suite). Bench median 0.44ms ≪ 3s SLO. `clippy -D warnings` + `cargo fmt --check` clean.

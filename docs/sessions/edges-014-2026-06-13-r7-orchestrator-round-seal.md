# edges-014 — R7 orchestrator round-seal (the thin in-lane drain → PAUSE at the in-lane ceiling)

**Date:** 2026-06-13
**Role:** edges-daemon-orchestrator (R7 — STAYED through the R6 impl-only cycle)
**Predecessor:** `edges-013-2026-06-13-r7-worktree-refresh-and-rescan-bench.md` (R7 impl session doc, `58d90eb`)
**Successor:** _(filled at the next /orchestrate-end — likely the user-driven phase-exit)_
**Round-seal commit:** _(this commit)_ · branch `track/edges` · **NOT pushed, NOT merged to main**

> **Companion (the authoritative accumulated cross-track ledger):** `docs/planning/edges-R5-wiring-plan.md` R7 block + the R7 PLAN-DELTA. This doc is the round's orchestrator framing. **`docs/audits/edges-P5-P7-cargo-audit.md`** is the cargo-audit finding record.

## What R7 was
The **thin in-lane drain** before edges' true in-lane ceiling: the §7.2 live-read follow-on + the two phase-exit-checklist items (P5.4 bench, cargo audit) that don't need the gated/MIGRATION_9-deferred wiring. Opened with a fresh impl (the R6 impl-only cycle); orch stayed.

## What landed (orchestrator framing) — 2 slices + 1 ops task, 612 → 620 tests / 0 failed
- **edges-026 `c195c7f`** — **§7.2 worktree-status live-read cache refresh** (P5.2 follow-on). `read_worktree_status` (git2) → `proj_worktree` git-axis cache via a NEW **non-Gateway, non-event** write-actor command `RefreshWorktreeStatus` (the drain_once/reap_leases family; §7.1 — the git-axis is a live-read cache, NO `WorktreeStatusRefreshed` event), triggered by a 30s **git-watcher** task (the drainer/reaper precedent, ARCHITECTURE.md:340). **Layer-clean** (persistence-core stays git-free; the git read + derive in the runtime layer — the edges-022 LESSON-17 rule). Rebuild resets the live-read cache. +8 tests; code-quality 2-fixed (usize→i64 saturating; watcher error/panic logging). Full `/tdd` RED→GREEN.
- **style `d800ef1`** — `cargo fmt` fixup of the 5 edges-026 files (the fmt-gap finding — `/tdd` Step 8 ran check+clippy but not fmt; the 407be7c precedent). Zero behavior change.
- **edges-027 `44ce907`** — **§18 `project.rescan` detection-latency benchmark** (P5.4; NON-TDD bench waiver — the bench IS the coverage, the event_write.rs precedent). `[[bench]] harness=false` driving the AS-BUILT detection core (`detect_git`+`detect_workflow`) over a representative temp repo → **median 0.44 ms ≪ 3 s SLO**; CI guard median < 50 ms (LESSON 22 — tighter than the SLO). Invisible to `cargo test --workspace`.
- **`cargo audit` (orch ops task)** — vs the P2 0-finding baseline: **1 NEW MEDIUM** — RUSTSEC-2023-0071 (`rsa` 0.9.10, Marvin Attack, no fix), transitive via **octocrab → jsonwebtoken → rsa** (GitHub-App JWT auth). **Exposure LOW** (edges never exercises it — auth deferred; planned `gh auth token`/OAuth is bearer-token not GitHub-App-JWT; local boundary). **Disposition: accept-and-document → human return-review.** Report: `docs/audits/edges-P5-P7-cargo-audit.md`.

Preflight clean (clippy/fmt/check; 620 workspace). No `shared/` change (CONTRACT 0.26.0 held); no event, no schema change.

## Decisions made / ratified this round
- **§7.2 live-read = a non-Gateway/non-event write-actor command + a git-watcher trigger (edges-026).** The architecture (§7.1, ARCHITECTURE.md:576/340) confirms: the git-axis is a live-read projection cache (NOT event-sourced; no `WorktreeStatusRefreshed`), refreshed by the git-watcher task. The write-actor git read is a BOUNDED-throughput concern (local I/O — returns), NOT the unbounded-network-hang liveness concern that mandated the edges-023/024 timeout → folded into the unified write-actor-I/O-offload SPREAD (not a mandatory fix).
- **`status` recompute uses a hardcoded `Creating` overlay (edges-026).** The only emitted overlay (overlay-event emitters have no producer). FLAGGED: a clean overlay source (a schema `overlay` column = MIGRATION_9-deferred, or event-sourced) is needed when Merged/Locked/etc. emitters land.
- **P5.4 bench: drive the detection core (`detect_git`+`detect_workflow`), median-gated guard < 50 ms (edges-027).** The §18 SLO governs the scan latency; the guard is calibrated tighter than the SLO (LESSON 22); single-shot tail is OS jitter → median-gated.
- **cargo-audit RUSTSEC-2023-0071: accept-and-document (orch recommendation → human return-review).** Low exposure; preferred fix = an octocrab feature-prune (drop the unused jsonwebtoken/rsa app-auth path) in a follow-up slice.

## Decisions explicitly NOT made (deferred)
- The overlay-source model (MIGRATION_9-deferred — when overlay emitters land).
- The octocrab feature-prune (the cargo-audit preferred fix — a follow-up hardening slice).
- All R5/R6 deferrals UNCHANGED (auth_expired · §7.2 redacted-operational-inputs · the write-actor-I/O offload · Linear `success:false`).

## Held-for-merge PLAN-DELTA (applied at the user-gated edges→main phase-exit merge)
All in `docs/planning/edges-R5-wiring-plan.md` R7 block:
- **Arch notes:** §7.2 worktree live-read cache LIVE (git-watcher wired); `WorktreeStatusRefreshed`-not-an-event confirmed; §18 `project.rescan` benched+guarded.
- **LESSON candidate (33):** the live-read-cache refresh pattern (non-Gateway/non-event write-actor command + git-watcher trigger + read-time `git_checked_at` staleness; rebuild resets the cache; persistence-core git-free).
- **Convention candidate:** `/tdd` Step 8 should add `cargo fmt --check` (the fmt-gap; extend the daemon/CLAUDE.md "fmt-check is FIRST" note to the per-slice gate).
- **Future TODO:** the overlay-source follow-on (MIGRATION_9-deferred).
- **SPREAD (unified):** write-actor-I/O-offload (git-watcher reads + drain_once + the external executors as ONE item).
- **FINDING:** cargo-audit RUSTSEC-2023-0071 (rsa, LOW, accept-and-document) → human return-review; CI ignore + the octocrab feature-prune at the merge.
- **Held-for-merge CI:** register `project_rescan` in the `/phase-exit` perf row + `.github/nightly.yml`.
- **Completed-work ticks (held):** P5.2 §7.2 live-read COMPLETE; P5.4 bench DONE; cargo audit DONE.

## EDGES IN-LANE CEILING REACHED → PAUSE
After R7, edges has **exhausted clean in-lane runway** (the R4 pattern). Everything remaining needs the user + the daemon track:
- **`/phase-exit 5` + `/phase-exit 7`** — the phase-close + the edges→main merge (the user drives; needs the daemon track for `main`, the MIGRATION_9 number, the §5.0 contract reconcile).
- **D8/MIGRATION_9-deferred:** Wave-C `integration_connections` migration + `IntegrationConnectionRegistered` + the P5.1 registry projector (`projects`/`repositories`).
- The cross-cutting return-review items (§7.2 redacted-operational-inputs · the write-actor-I/O offload · the subscribe-delta daemon-owned fix · the cargo-audit disposition · the auth bootstrap + `auth_expired`).

**Edges PAUSES at the seal.** The orchestrator does NOT run `/phase-exit` or the merge (lead-instructed: the user drives those with the daemon track).

## Seal mechanics
Round terminal commit on `track/edges` (this commit) — folds the 2 R7 briefs (edges-026/027) + the wiring-plan R7 ledger + the cargo-audit report + this doc. **NO push, NO merge.** HEAD at seal: `58d90eb` + this commit; 620/0; tree clean post-commit. Edges is **ready for the user-gated edges→main phase-exit merge**.

# edges-010 — R5 orchestrator round-seal (the R1-gated P5/P7.1 phase-exit BEGINS)

**Date:** 2026-06-13
**Role:** edges-daemon-orchestrator (R5, fresh — resumed from the R4 pause)
**Predecessor:** `edges-009-2026-06-13-r5-p5-p7-wiring.md` (impl session doc, `09d003b`)
**Successor:** [edges-011-2026-06-13-r6-p7-1-wave-d-e.md](edges-011-2026-06-13-r6-p7-1-wave-d-e.md)
**Round-seal commit:** _(this commit)_ · branch `track/edges` · **NOT pushed, NOT merged to main** (round close-out only; the `bd3ee31` merge stays local — the edges→main merge is the eventual phase-exit, not this round)

> **Companion (READ THIS for the full cross-track ledger + findings + TODOs):** `docs/planning/edges-R5-wiring-plan.md` (committed `c50db99`) — the PLAN-DELTA the integration owner applies at the final merge. This doc is the round's orchestrator framing; the wiring-plan doc is the authoritative accumulated delta.

## What R5 was
The pause is over: **R1 was delivered on main** (the per-namespace `ActionExecutor` registration seam + the 11-type Phase-5/7 event contract + the test-support feature + the frozen ExecutionProfile), and the user authorized the merge + the R1-gated phase-exit. R5 = **merge main's R1 seal into `track/edges`, then wire the real P5 executors** against the delivered seam.

## What landed (orchestrator framing)
- **MERGE `bd3ee31`** — merged the LAST SEALED main commit `0c637a8` ("seal the 4.0b-T + edges-R1 round"), NOT the in-flight 4.0b-2 above it. Absorbed CONTRACT **0.20→0.26**. 3 additive conflicts (lib.rs mods · Cargo.toml deps · Cargo.lock), kept-both-sides. Green-verified: cargo check + 530 tests + clippy -D + fmt. `shared/` was edges-untouched → bumps came clean. **R1-completeness verified COMPLETE, no Finding** (all 4 deliverables + the catalog entries + the 3 design-choice resolutions; design-choice 3a = SYNC trait kept, frozen 2.3 trait NOT reopened; H1 ExecutionProfile frozen-on-main).
- **4 wiring slices** (each its own TDD cycle, each security-reviewed):
  - **edges-019** `c739278` — P5.1 `project.rescan` executor (ExecutorKind::Project); emits `ProjectRescanned` via the NEW generic `EmittedEvent::Namespaced` bridge (Q1=B) through the §15 gate; `remote_url` userinfo stripped at the emit source. Registry projector MIGRATION_9-deferred.
  - **edges-020** `7dabab5` — P5.2 `git.create_worktree` executor (ExecutorKind::Git); git-CLI seam (forbidden #6); emits `WorktreeCreated`. **Security HIGH caught + closed in-slice:** git argument-injection (leading-`-` operand → audit-integrity divergence) → fail-closed guard + canonical arg order + regression test.
  - **edges-021** `51f5586` — P5.2 `git.create_branch` executor; extends GitExecutor; `BranchCreated`; the arg-injection guard extracted to a shared `reject_dash_operands` helper. **P5.2 git mutators complete.**
  - **edges-022** `c666dc0` — P5.2 `proj_worktree` projector; folds `WorktreeCreated`→`proj_worktree` (repo_id via the LESSON-17 immutable sibling-read; `wire_value` status, layer-correct). **P5.2 read vertical CLOSED** (mutator→event→projection→IPC).
- **Tests 530→571, 0 failed.** No `shared/`/CONTRACT change (consumed the merged 0.26.0).

## Decisions made / ratified this round
- **Q1 = B** (one generic `EmittedEvent::Namespaced{event_type, payload_json}` bridge for all ~11 edges events; SessionStarted stays typed) — minimizes the cross-track gateway/ surface.
- **D8 (lead): MIGRATION_9 DEFERRED** to the final edges→main merge (daemon's in-flight 4.0b-2 may hold v9; consumer-less forward-laying → deferral free). Wave-C sequencing (build-vs-defer) = orch's call.
- **Bridge ownership (lead):** edges OWNS the additive `gateway/executor.rs`+`request.rs` bridge edits as phase-exit integration (NOT an R1-style re-route).
- **§7.2/§15 over-redaction (lead-ENDORSED MVP-accept):** the approve-path runs executors off §15-redacted inputs → a high-entropy operational path is over-masked. Invariant HOLDS (over-redaction, not a leak); production-low. Proper fix (non-redacted operational channel vs path-field exemption) = a human return-review hardening slice.
- **Slice-plan revision (orch):** the separate "git read executors" slice DISSOLVED (git.status/diff have no consumer; GitExecutor delegates them to the stub).

## Open follow-ups / carry-forward (full detail in the wiring-plan PLAN-DELTA)
- **§7.2 redacted-operational-inputs** → human return-review hardening slice (lead ledger).
- **Arg-injection guard** → STANDING requirement for every external mutator (Wave-D github/linear must fold it) → LESSON 31.
- **Subscribe-delta gap** → the emitted_events append loop threads no ProjectionDelta (affects edges projectors AND the daemon's own SessionStarted→proj_session) → daemon-owned gateway/pipeline fix (lead ledger).
- **Live-read status refresh** (P5.2 follow-on) → read_worktree_status→proj_worktree live-read columns.
- **MIGRATION_9-deferred:** P5.1 registry projector (projects/repositories) + Wave-C `integration_connections`.
- **LESSON 30** (strip-at-source) + **LESSON 31** (git-mutator pattern + arg-injection guard) + arch-notes (ExecutorKind Project/Git registered; proj_worktree producer; P5.2 read vertical closed) → apply to daemon/LESSONS.md + daemon/CLAUDE.md + ARCHITECTURE.md at the final merge (held — cross-track rule).

## NEXT (R6 target)
**P7.1 Wave-D — github/linear sync executors.** The load-bearing piece is **design-choice 3a**: the `ActionExecutor` trait is SYNC + octocrab/reqwest are async → run on a **dedicated blocking context** (`spawn_blocking` + `Handle::block_on`); `block_on` on a tokio worker thread PANICS. Emit `PullRequestSynced` + `GithubSyncFailed`/`LinearSyncFailed` (non-auth only; `auth_expired` deferred). Fold the arg/param-injection guard. Then: proj_pull_request projector · the MIGRATION_9-deferred items · the hardening TODOs · P5.4 bench · cargo audit · `/phase-exit 5`+`7`.

## Seal mechanics
Round terminal commit on `track/edges` (this commit) — folds the lead's decision-log R5 edits (D8 + the §15-redacted-inputs + arg-injection return-review entries) + this doc. **NO push, NO merge** (lead-confirmed). Cycle = fresh R6 orch+impl pair (the lead carries the R5 PLAN-DELTA into the spawn prompts). HEAD at seal: `09d003b` + this commit; 571/0; tree clean.

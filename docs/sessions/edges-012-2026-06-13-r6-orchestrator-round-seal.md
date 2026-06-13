# edges-012 — R6 orchestrator round-seal (P7.1 Wave-D external mutators + github read vertical closed)

**Date:** 2026-06-13
**Role:** edges-daemon-orchestrator (R6, fresh — opened post-R5-cycle; STAYS into R7 per the lead's impl-only cycle)
**Predecessor:** `edges-011-2026-06-13-r6-p7-1-wave-d-e.md` (impl session doc, `fd30065`)
**Successor:** _(filled at the next /orchestrate-end)_
**Round-seal commit:** _(this commit)_ · branch `track/edges` · **NOT pushed, NOT merged to main** (round close-out only; the edges→main merge is the user-gated phase-exit, not this round)

> **Companion (the authoritative accumulated cross-track ledger):** `docs/planning/edges-R5-wiring-plan.md` — the R6 round-progress block + the R6 PLAN-DELTA applied at the final merge. This doc is the round's orchestrator framing.

## What R6 was
The R1-gated phase-exit's highest-risk piece: **P7.1 Wave-D — the two external-network sync executors** (the first edges code that mutates external services through the Gateway), then **closing the github read vertical**. Opened from the R5 PLAN-DELTA with the fresh impl on full runway.

## What landed (orchestrator framing) — 3 slices, 571 → 612 tests / 0 failed
- **edges-023 `498bd21`** — **github sync executor** (`ExecutorKind::Github`, registered main.rs): `github.create_pr`(risk-3)/`_draft`(risk-2) via an injected async octocrab WRITE-client seam, driven from the SYNC `ActionExecutor` trait over the **as-built 3a mechanism** (captured `tokio::runtime::Handle` + `handle.block_on` + a mandatory 30s timeout; execute() runs on the write-actor's raw std::thread — non-worker, no entered runtime). Success → `PullRequestSynced` (Namespaced bridge, §15-gated); terminal-non-auth → `GithubSyncFailed` via the new additive **`ExecutionOutcome::FailedWithEvents`**; auth/transient → `Failed`. security-reviewer PASS (0 findings, 4 load-bearing axes adversarially verified).
- **edges-024 `f908424`** — **linear sync executor** (`ExecutorKind::Linear`): `link_issue`/`create_issue`; success = `ActionSucceeded` only (no Linear domain event — the contract has none, intentional asymmetry); terminal-non-auth → `LinearSyncFailed`. **Extracted the shared exhaustive `classify_sync_failure`/`SyncFailure` §17 disposition** used by BOTH executors (github + linear can't diverge — a correctness guard); refactored edges-023's `classify_failure` to call it (behavior-preserving — all github tests green). security PASS. **Wave-D external mutators COMPLETE.**
- **edges-025 `8db6cc7`** — **`proj_pull_request` projector**: folds `PullRequestSynced` → the §7.2 cache (`pr_id={repo_id}#{pr_number}` rebuild-safe composite, LESSON-17 sibling-read, `wire_value` status, edges-022 3-case taxonomy, rebuild-equivalent). **GITHUB READ VERTICAL CLOSED** (create_pr → PullRequestSynced → proj_pull_request → get_projection). code-quality 1-med fixed.

No `shared/` change (CONTRACT 0.26.0 held); no schema-snapshot, no bump. TDD-clean throughout; Step-2.5-reviewed every slice; reviewer policy honored (security on both mutators, code-quality on the projector).

## Decisions made / ratified this round
- **3a mechanism CORRECTED to the as-built (orch trace, lead-endorsed — evidence wins):** `execute()` runs on the write-actor's **raw `std::thread`** (writer.rs:273-326 → pipeline.rs:969) — NOT a tokio worker, NO entered runtime. So the pre-merge "spawn_blocking + Handle::block_on, block_on-on-a-worker-panics" framing was the OPPOSITE footgun: `Handle::current()` there PANICS. Correct = capture a `Handle` in main.rs's #[tokio::main] runtime → `handle.block_on(...)`. NO spawn_blocking. (The carried 3a note in the lead's decision-log + the wiring-plan Wave-D block are refined to this.)
- **Mandatory timeout (lead-mandated):** every external-call executor wraps its network future in `tokio::time::timeout` — the single write-actor serializes ALL mutations, so an unbounded hang is a liveness break. Not optional.
- **`ExecutionOutcome::FailedWithEvents` (Q2, edges-023):** an additive daemon-internal gateway-bridge variant (edges-owned per the lead's R5 ruling) so a FAILED action can emit a structured observation event (`*SyncFailed`) atomic with `ActionFailed`; existing `Failed(String)` sites untouched.
- **`classify_sync_failure` EXTRACTED (Q2, edges-024):** the §17 failure→outcome disposition in ONE exhaustive place (correctness > DRY — providers can't diverge). The edges-023 refactor is behavior-preserving (full suite green).
- **Linear-success-no-event (Q1, edges-024):** the frozen contract has no Linear success event (unlike github's `PullRequestSynced`). Confirmed intentional (Linear read on-demand via `fetch_issue`, §7.3; no write-event projection) — architecture-as-contract HELD (no improvised event); no §7.3/§8 gap found.
- **`pr_id` composite (Q1, edges-025):** `{repo_id}#{pr_number}`, rebuild-safe (`proj_pull_request` ∈ REBUILD_TABLES; `#` ULID-safe).

## Decisions explicitly NOT made (deferred)
- `auth_expired` `*SyncFailed` variant (0.5b gate lifted; needs a §17/INV-SEC re-review) — non-auth-only this round.
- The §7.2 redacted-operational-inputs proper fix (a §15 cross-cutting hardening — human return-review, carried from R5).
- The Linear live-client mutation-payload `success:false` parsing (MVP-accept; SPREAD).
- The slow-external-executor-on-the-write-actor offload (cross-track, daemon-core write-actor territory; lead routed to return-review).

## Cycle-gate (the round-seal trigger)
Surfaced a cycle-gate recommendation at the D3/D4 intersection: **WARN (impl 71%)** + a clean arc boundary (Wave-D complete + github read vertical closed) + thinning in-lane runway + the next slice would push `/session-end` to ~78%. Lead ruled **SEAL + cycle the IMPL ONLY** (orch stays — 39%, full round context, continuity into the short R7 drain). Held all dispatch from the rec until the ruling (cycle-gate protocol — no race).

## Held-for-merge PLAN-DELTA (applied at the user-gated edges→main phase-exit merge)
All in `docs/planning/edges-R5-wiring-plan.md` R6 block (+ the lead's decision-log R6 section, committed in this seal):
- **Arch notes:** github read vertical CLOSED · §6.3 github/linear actions LIVE · §7.2/§17 `PullRequestSynced`/`*SyncFailed` emit paths LIVE · `ExecutionOutcome::FailedWithEvents` (edges-owned daemon-internal gateway extension) · the Linear-success-no-event asymmetry · the Success→Transient unreachable-path message change.
- **Lessons:** **LESSON 32** (the external-network-mutator pattern: captured-`Handle` `block_on` + mandatory timeout on the write-actor std::thread + the §17→`*SyncFailed` disposition + the typed-API operand guard + `FailedWithEvents`; the test-harness pin: plain `#[test]` + a built `Runtime` handle, never `#[tokio::test]`) · **LESSON 17 generalization** (3rd gateway-event projector).
- **Completed-work ticks (held):** P7.1 = github + linear sync executors LANDED + proj_pull_request projector LANDED (github read vertical complete); P5/P7.1 in-lane surfaces complete.
- **SPREADs (consumer-marked):** auth bootstrap + `auth_expired` · Linear `success:false` parsing · write-actor execute-phase offload · (carried) §7.2 redacted-operational-inputs.

## Remaining for the phase-exit (R7 thin in-lane drain → then PAUSE)
R7 (fresh impl, orch stays): §7.2 live-read status refresh (proj_worktree dirty/ahead/behind) · P5.4 `project.rescan` bench (re-author w/ the known 1.029 ms) · `cargo audit` (reqwest/octocrab/async-trait vs the P2 baseline). Then edges hits its TRUE in-lane ceiling (the R4 pattern) → **PAUSE** for the user-gated `/phase-exit 5`+`7` + the edges→main merge (needs the daemon track + the D8/MIGRATION_9-deferred Wave-C `integration_connections` + P5.1 registry projector).

## Seal mechanics
Round terminal commit on `track/edges` (this commit) — folds the 3 briefs (edges-023/024/025) + the wiring-plan R6 ledger + the lead's decision-log R6 section + this doc. **NO push, NO merge** (R5 precedent, lead-confirmed). Cycle = impl-only (lead spins down + respawns the impl; orch stays). HEAD at seal: `fd30065` + this commit; 612/0; tree clean post-commit.

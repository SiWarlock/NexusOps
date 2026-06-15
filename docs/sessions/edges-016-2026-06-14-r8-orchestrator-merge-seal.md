# edges-016 — R8 orchestrator merge-round seal (main→edges re-sync; CONTRACT 0.26→0.32; INV-SEC-1 PASS)

**Date:** 2026-06-14
**Role:** edges-daemon-orchestrator (R8 — the phase-exit/merge round, STEP 1: re-sync)
**Predecessor:** `edges-015-2026-06-14-r8-main-to-edges-merge.md` (R8 impl merge session doc, `ae1106e`)
**Successor:** _(R9 — the fresh phase-exit pair, post-cycle)_
**Round-seal commit:** _(this commit)_ · branch `track/edges` · **NOT pushed, NOT merged to main** (`536ac04` stays local; the edges→main merge is the USER's later cross-track coordination)

> **Companion (the authoritative accumulated cross-track ledger):** `docs/planning/edges-R5-wiring-plan.md` R8 block + the R5–R8 PLAN-DELTA. This doc is the merge-round orchestrator framing.

## What R8 was
The user chose **topology A** (finish the edges phase-exit → then the user coordinates edges→main → then main→ui) and gave the merge go. R8 STEP 1 = **re-sync edges with the daemon track's latest** (main advanced past the R5 merge base) before the phase-exit. The phase-exit slices (Wave-C + P5.1 + `/phase-exit`) are R9 (a fresh post-cycle pair).

## What landed — the merge (1 merge commit + the impl session doc)
- **Merge `536ac04`** (2 parents: `1f1f14f` track/edges + `df19f89` main, 47 main commits) — absorbed **CONTRACT 0.26.0 → 0.32.0** (the daemon's full Phase-4 + Codex-3.3 arc: 4.0b-2 live interception · 4.0c telemetry pump · 4.1a/b survival + tmux broker · 4.2 SessionFailed · 4.3 background jobs · 3.3a/b Codex). Edges' `shared/` was untouched → clean absorb (no edges bump), the R5 pattern.
- **7 conflicts** — additive unions (lib.rs `integrations`+`integrity` · runtime/mod.rs git_watcher+jobs+telemetry+wait_class · Cargo.toml dev-deps) + 2 reconciliations: **git/mod.rs add/add** (edges' git submodules cli/detect/executor/precedence/reads + main's `read_diff` ui-get_diff backend — both `read_diff`s COEXIST, `git::read_diff` vs `git::reads::read_diff`) · **main.rs CAT-1** (edges' Project/Git/Github/Linear executor registrations folded into main's LIVE INV-SEC-1 drive loop under `AgentMutationPolicy` + `SessionExecutor` + alarm/breaker). **2 resolutions beyond the orch plan (impl-caught, both sound):** (1) deduped a `[dependencies]` git2 TOML dup-key the auto-merge silently left (kept edges' vendored entry; main's read_diff satisfied by the vendored build); (2) Cargo.lock via `--theirs`+`cargo check` regen.
- **Green:** cargo check + clippy -D + fmt + the full suite **760 / 0 / 0** (edges 620 + main's Phase-4/Codex arc). Both cat-1 pins pass (`test_live_session_create_has_interception` + `test_no_reachable_live_caller`). No semantic fixes needed (edges' executors compiled clean against main's unchanged `ActionExecutor`/`ExecutionOutcome` — no drift).

## The load-bearing safety gate — INV-SEC-1: PASS
The merge's load-bearing gate (lead-flagged) was the **main.rs CAT-1 fold** — edges' 4 mutators entering main's live INV-SEC-1 drive loop. **security-reviewer verdict: `INV-SEC-1: PASS (no-bypass confirmed)`** — all 6 criteria cleared with file:line evidence:
- **#1 no-bypass:** edges' 4 mutators reachable ONLY via the Gateway `policy→approval→execute→audit` pipeline (sole executor call `pipeline.rs:976`, downstream of a committed txn; edges executors hold NO WriteHandle/eventstore/SQL, emit only via `emitted_events`; CatalogExecutor dispatch runs AFTER `requires_resource_refs` + the Adjudication guard).
- **#2 single-mutator / policy governs all:** `AgentMutationPolicy` only raises `agent.*`→Deny, else falls through to the catalog-authoritative `CatalogPolicy` → edges' `git.*`/`linear.*`=risk-2, `github.create_pr`=risk-3 STILL approval-gated (NOT weakened); `project.rescan`=risk-0 auto is intentional (read-only, side_effect=false, allowlisted). FailedWithEvents audited atomic + breaker-gated; §15 secrets clean.

## Decisions made / ratified
- **D8 RESOLVED:** main claimed **MIGRATION_9 = `MIGRATION_9_POLICY_DECISION`** (the ②-mini work, 0.30.0) — exactly the collision the R5 deferral avoided. **Edges' Wave-C `integration_connections` = MIGRATION_10** (next-free; no edges migration to renumber — Wave-C unbuilt). The R5 D8-deferral was vindicated.
- **Merge mechanics:** the merge commit (`536ac04`) is the integration artifact; **NO push, NO edges→main merge** (the user coordinates edges→main later with the daemon track — `/phase-exit 5`+`7` + the §5.0 reconcile).
- **Cargo audit on the merged tree:** same single finding (RUSTSEC-2023-0071, rsa) — NO new advisories from main's P4/Codex deps; the R7 accept-and-document disposition still covers it.

## Held-for-merge PLAN-DELTA (UNCHANGED — applied at the user-gated edges→main merge)
The R5–R7 PLAN-DELTA (LESSONs 30/31/32 + 33-candidate · the arch-notes · the SPREADs · the completed-work ticks · the cargo-audit FINDING · the fmt-gap convention) STAYS HELD in `docs/planning/edges-R5-wiring-plan.md`. **The R8 main→edges merge did NOT apply it** — the cross-track rule still holds (the daemon track is live on main at 3.3c; edges editing the now-merged-in shared root docs would re-conflict at edges→main). The integration owner applies the PLAN-DELTA at the edges→main merge.

## R9 (the fresh phase-exit pair) — the round target
1. **Wave-C `integration_connections` (MIGRATION_10)** + `IntegrationConnectionRegistered` (keychain_ref = pointer-ONLY, §15 #4) + the connect-path mutator (security-reviewer + INV-SEC-1 + the LESSON-31 arg guard; consumer-less forward-laying migration).
2. **P5.1 registry projector** (projects/repositories — folds `ProjectRescanned`).
3. **`/phase-exit 5` + `/phase-exit 7`** (verify-only — arch-drift + reachability auditors + spec-coverage; the gated §6.3/§15/§8 anchors are now LIVE post-merge).
4. **SEAL + HOLD** for the user's edges→main coordination. Do NOT run edges→main.

## Cycle (the round-seal trigger)
Surfaced a cycle-gate rec at the clean merge-verified boundary: orch 67% [OK, climbing] + impl stale/likely-high (post-merge) + the FINAL phase-exit round is the heaviest (a new migration/mutator + the /phase-exit auditors + the seal). Lead ruled **CYCLE BOTH** (the D4 clean-boundary-before-a-heavy-round pattern; R6's impl-only was a short drain w/ a 39% orch — different case). Fresh runway for both. The integration context is durably captured (the wiring-plan R5–R8 PLAN-DELTA + the decision-log + the memory) → the R9 orch re-orients via /orchestrate-start.

## Seal mechanics
Round terminal commit on `track/edges` (this commit) — the orch merge-round doc + the wiring-plan R8 ledger entry. **NO push, NO merge.** The merge `536ac04` + the impl doc `ae1106e` already committed; this is the orch seal. Cycle = fresh R9 pair (the lead carries the context into the spawn prompts).

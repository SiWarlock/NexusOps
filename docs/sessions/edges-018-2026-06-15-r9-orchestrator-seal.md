# edges-018 — R9 orchestrator round-seal (P5/P7.1 Wave-C wiring + phase-exit COMPLETE; CONTRACT 0.33)

**Date:** 2026-06-15
**Role:** edges-daemon-orchestrator (R9 — the FINAL phase-exit round)
**Predecessor:** `edges-017-2026-06-15-r9-p5-p7-wave-c-wiring.md` (R9 implementer session doc, `074465f`)
**Successor:** _(the user-gated edges→main merge — not an edges round)_
**Round-seal commit:** _(this commit)_ · branch `track/edges` · **NOT pushed, NOT merged** (edges HOLDS at COMPLETE for the user's edges→main coordination)

> **Companion (the authoritative accumulated cross-track ledger):** `docs/planning/edges-R5-wiring-plan.md` **R9 block** + the R5–R9 PLAN-DELTA. This doc is the orchestrator's round framing.

## What R9 was
The FINAL edges in-lane round: complete the P5.1/P7.1 phase-exit wiring (the MIGRATION-deferred registry projectors + the Wave-C integration-connection mutator) then run `/phase-exit 5`+`7` (verify-only) and seal. Per USER topology A (finish the edges phase-exit → the user coordinates edges→main → then main→ui).

## What landed — 3 TDD slices (all test-first, Step-2.5-reviewed)
- **edges-028** `8788210` — P5.1 project-registry projector (`proj_project`+`proj_repository` ← `ProjectRescanned`; MIGRATION_10; CONTRACT-neutral; 9 tests; security SKIP).
- **edges-029** `355eddf` — Wave-C `integration.connect` mutator (`ExecutorKind::Integration`; **registration-only/§15 #4**; **CONTRACT 0.32→0.33**; `standing_grant_eligible=FALSE`; 8 tests; **security-reviewer full invariant PASS**; 1 commit — the §15 #4 pin is structural/inseparable).
- **edges-030** `25e0833` — Wave-C integration-connections projector (`proj_integration_connection` ← `IntegrationConnectionRegistered`; MIGRATION_11; keyed by payload.connection_id; CONTRACT-neutral; 8 tests; security SKIP).

`SUPPORTED_USER_VERSION` 9→11. **P5.1 + P7.1 Wave-C verticals CLOSED.** Briefs edges-028/029/030 (all spec-lint PASS).

## `/phase-exit 5` + `/phase-exit 7` — VERDICT: CLEAR (verify-only); NO BLOCKED
- **arch-drift (both): CLEAR — 0 drift** (`docs/audits/P5-arch-drift.md`, `P7-arch-drift.md`); known-deferred items confirmed deferred-NOT-drift; stale-doc notes only (§6.3 count ~21→28 · §18 bench figure · §9 auth-bootstrap · §11.2 fields).
- **reachability (both): CLEAR** (`docs/audits/P5-reachability.md` 20/2/9 · `P7-reachability.md` 53/0/7); the forward-laid `git/reads.rs` helpers + the deferred IPC reads are intentional gating, not gaps.
- **spec-coverage:** tests 5 PASS; tests 7 PASS except §11.2 (the ui-track waiver — held-for-merge).
- **dependency:** unchanged from R8 (RUSTSEC-2023-0071 accept-and-documented; R9 added no deps).
- **test-count verification (lead-requested):** workspace **785/0** = R8's 760 + 25 R9 tests; the impl's "707" was daemon-crate-only — BENIGN, coverage UP.

The phases stay **OPEN by design** (5.3 ExecutionProfile = daemon-side/H1-gated; P7.2/7.3 = ui-track; the deferrals) → the phase checkboxes do NOT tick (held-for-merge).

## Decisions made / ratified this round
- **§15 + LESSON-20-forced registration-only** (lead+user-APPROVED deferment): the connect mutator structurally cannot carry the token (a risk-2 action executes off the §15-redacted durable row); the token→keychain WRITE is a deferred non-Gateway/HITL mechanism (folds with H1). §15 #4 holds by construction.
- **§6.2-floor `standing_grant_eligible=FALSE`** (security-reviewer-recommended, lead-ENDORSED; for the user's return-review ratification): a credential/authorization-establishing action is non-grantable (the discard_hunk precedent; LESSON 32 axis = irreversibility/blast-radius, not risk). Shipped conservative; user ratifies keep-FALSE/relax.
- **Migration numbering** (orch away-mode authority): P5.1 registry = MIGRATION_10, Wave-C = MIGRATION_11 (Wave-C held first → P5.1 took the contiguous slot; the D8 "next-free" still holds).
- **DATA_MODEL §3/§2.8 reconciliation:** MVP projects/repositories/integration_connections = event-fed projections (proj_*), NOT durable registries; the fuller model deferred. Resolved by the frozen `ProjectRescanned` contract (lead-ruled at R1b).
- **Modeling A** (user-ruled): generic `integration.connect` + `ExecutorKind::Integration` (keychain-write/registration, not network-sync).

## Held-for-merge PLAN-DELTA
UNCHANGED routing posture — the full R9 PLAN-DELTA (the CONTRACT 0.33 ratification · the ticks · the §11.2 waiver · the standing_grant Finding · the arch-notes + stale-doc notes · the 3 lesson candidates [renumber §44+] · MIGRATION_10/11 · the carry-forwards) accumulates in `docs/planning/edges-R5-wiring-plan.md` **R9 block**. Edges does NOT edit the shared root docs in-worktree (cross-track rule — the daemon track is live on main; the integration owner applies the PLAN-DELTA at the edges→main merge).

## Seal mechanics
Round terminal commit on `track/edges` (this commit): the wiring-plan R9 block + this orch doc + the 3 R9 briefs + the 4 phase-exit audit reports + the lead's folded decision-log edits (R8/R9 section + D9/D10/D11 + reconciliation-ledger). **NO push, NO edges→main.** This is the FINAL edges in-lane seal — edges HOLDS at COMPLETE for the user's edges→main coordination (driven with the daemon track: the §5.0 0.33 contract reconcile + the held PLAN-DELTA + the migration numbers).

# edges-017 — R9: P5.1 registry projector + P7.1 Wave-C (mutator + projector) — the final in-lane wiring

**Date:** 2026-06-15
**Role:** edges-daemon-implementer (R9 — the FINAL phase-exit round; three TDD slices)
**Predecessor:** [edges-016-2026-06-14-r8-orchestrator-merge-seal.md](edges-016-2026-06-14-r8-orchestrator-merge-seal.md) (R8 orch merge-round seal, `8db62bb`)
**Successor:** _(R9 orchestrator round-seal — `/orchestrate-end` + `/phase-exit 5`+`7`)_
**Slice commits:** `8788210` (edges-028) · `355eddf` (edges-029) · `25e0833` (edges-030) · branch `track/edges` · **NOT pushed, NOT merged to main** (edges HOLDS at COMPLETE for the user's edges→main coordination)

> **Companion (the authoritative accumulated cross-track ledger):** `docs/planning/edges-R5-wiring-plan.md`. The orchestrator folds this round's PLAN-DELTA into its R9 block at the round seal.

## Why this session existed
R9 is the final edges phase-exit round (post-R8 main→edges re-sync). It lands the last P5/P7.1 wiring remainders so the orchestrator can run `/phase-exit 5`+`7` and seal the edges track at COMPLETE: the P5.1 registry read vertical (the MIGRATION-deferred projector), and the Wave-C integration-connection vertical (the new mutator + its projector). All three are TDD slices (test-first, RED→2.5→GREEN), each its own commit.

## What was built

### edges-028 — P5.1 project-registry projector (`8788210`, MIGRATION_10, CONTRACT-neutral)
**Files created:** `daemon/src/projections/project_registry.rs` (the `ProjectRegistryProjector` — folds `ProjectRescanned` → `proj_project` [identity axis] + `proj_repository` [git-detection axis], both keyed by `env.project_id`, 1:1 MVP) · `daemon/tests/project_registry.rs` (9 tests).
**Files modified:** `daemon/src/eventstore/schema.rs` (`MIGRATION_10_PROJECT_REGISTRY` — the two CREATE TABLEs) · `daemon/src/eventstore/migrations.rs` (`SUPPORTED_USER_VERSION` 9→10 + array) · `daemon/src/projections/schema.rs` (`proj_project`+`proj_repository` → `REBUILD_TABLES`) · `daemon/src/projections/mod.rs` (mod + register).
Closes the P5.1 read vertical (executor edges-019 → event → projection). NO LESSON-17 sibling-read (`ProjectRescanned` is self-contained). security-reviewer SKIP (read-model projector).

### edges-029 — Wave-C `integration.connect` mutator (`355eddf`, CONTRACT 0.32→0.33, SAFETY)
**Files created:** `daemon/src/integrations/connect.rs` (the `IntegrationExecutor`, `ExecutorKind::Integration`) · `daemon/tests/integration_connect.rs` (7 tests).
**Files modified:** `shared/src/catalog.rs` (`ExecutorKind::Integration` + `integration.connect` in `MVP_ACTION_TYPES` + the lookup arm via `entry_no_standing_grant`) · `shared/src/lib.rs` (CONTRACT 0.32→0.33) · `shared/tests/contract.rs` (1 new test + 4 updated count/version/ExecutorKind pins) · `shared/contracts/schema/nexusops-contract.schema.json` (regenerated) · `daemon/src/idgen.rs` (`new_connection_id()` + FixedIdGen counter) · `daemon/src/integrations/mod.rs` (pub mod) · `daemon/src/main.rs` (register `ExecutorKind::Integration` on the live CatalogExecutor) · `daemon/src/gateway/preview.rs` (two `ExecutorKind` match arms).
REGISTRATION-ONLY (§15 + LESSON 20 forced): inputs `{provider, keychain_ref pointer, account?}` — NO token; the event has no token slot → §15 #4 holds by construction. Defense-in-depth: a secret-shaped `keychain_ref` is rejected via the public `PrefixRedactor` (the canonical LESSON-13 detector, read-only). INV-SEC-1 no-bypass: the executor holds only `Box<dyn IdGen>`, emits via `emitted_events` only, approval-gated (NOT on the risk-0 auto-execute allowlist). **security-reviewer: full invariant PASS.**

### edges-030 — Wave-C integration-connections projector (`25e0833`, MIGRATION_11, CONTRACT-neutral)
**Files created:** `daemon/src/projections/integration_connections.rs` (the `IntegrationConnectionProjector` — folds `IntegrationConnectionRegistered` → `proj_integration_connection`, keyed by **payload.connection_id**) · `daemon/tests/integration_connections_proj.rs` (8 tests).
**Files modified:** `daemon/src/eventstore/schema.rs` (`MIGRATION_11_INTEGRATION_CONNECTIONS`) · `daemon/src/eventstore/migrations.rs` (`SUPPORTED_USER_VERSION` 10→11 + array) · `daemon/src/projections/schema.rs` (`proj_integration_connection` → `REBUILD_TABLES`) · `daemon/src/projections/mod.rs` (mod + register) · `daemon/tests/gateway_plan.rs` (runtime version pin 10→11) · `daemon/tests/project_registry.rs` (relaxed edges-028's `test_migration_10_applies` to a `>=10` floor).
Closes the Wave-C connection vertical. NO sibling-read (self-contained); `provider` via `wire_value`; `status='connected'` literal resting state; security-reviewer SKIP.

## Decisions made
- **edges-028 Q1–Q4 (all defaults):** two tables (`proj_project`/`proj_repository` mirror DATA_MODEL 2.8) · `project_id` PK for `proj_repository` (1:1 MVP; no `repo_id` in the payload) · DEFER the IPC read (a `ProjectionName` variant would bump CONTRACT) · store BOTH `scanned_at` + `updated_at_seq` (distinct axes).
- **edges-029 Q1–Q4 + the security flip:** preview_class=Api · idempotency=FromInputs · token→keychain write DEFERRED · **`standing_grant_eligible` flipped true→FALSE on the security-reviewer's ruling** — a credential/authorization-ESTABLISHING action is never folded into a plan-level approve-all (the blast-radius axis, LESSON 32; the discard_hunk precedent; fail-safe). Confirmed by the orchestrator; escalated to lead→user for §6.2-floor ratification.
- **edges-030 Q1–Q4 (all defaults):** `status='connected'` literal (no frozen §5.1 Connection machine) · OMIT the deferred DATA_MODEL 2.8 fields · DEFER the IPC read · `proj_integration_connection` (proj_ = rebuildable) naming.
- **Commit posture:** edges-029 folded to 1 commit (the §15 #4 no-token property is structural/inseparable — LESSON 39 carve-out, orch-ruled).
- **Migration-floor test convention (collateral of MIGRATION_11):** a per-migration `test_migration_N_applies` asserts its FLOOR (`user_version >= N`) + table existence, NOT the exact-latest (else every migration breaks the prior's test); `gateway_plan` remains the single exact-latest runtime pin. `assert!(CONST >= N)` trips clippy's const-assertion lint → use the runtime `store.user_version() >= N`.

## Decisions explicitly NOT made (deferred)
- The token→keychain WRITE (the `keyring` crate + a non-Gateway secret-store path + the live macOS-keychain write) — a HITL/live-integration follow-on (folds with H1).
- The IPC read RPCs (`ProjectionName::Project` / `::IntegrationConnection` + `get_projection`) — CONTRACT-bumping; land with a ui consumer.
- The DATA_MODEL 2.8 durable-registry fuller models (canonical rows + `register_project` mutator; `workspace_id`/`scopes_json`/`expires_at` + disconnect/refresh lifecycle).
- `workspace_id` on `proj_integration_connection` (available on the envelope; a cheap follow-on if workspace-scoping is needed).

## TDD compliance
**Clean — all three slices test-first** (RED confirmed for the right reason → Step-2.5 orch-approved → GREEN). No TDD violations. The only post-GREEN test edits were the code-quality fixes (folded into the slice) and the MIGRATION_11-collateral version-pin updates (test-maintenance, not back-filled behavior).

## Cross-doc invariant audit
- **edges-029 changed `shared/` (a contract):** `ExecutorKind` += `Integration`; `MVP_ACTION_TYPES` += `integration.connect`; CONTRACT 0.32→0.33. **Multi-track memory check: flagged at edges-029 Step 9** (the PROMINENT merge-ledger item; orchestrator confirmed). The paired `daemon/CLAUDE.md` §6.3 ActionTypeCatalog row + `ARCHITECTURE.md` Appendix A row are **HELD-for-merge** (cross-track rule — edges does not edit the shared root docs in-worktree; the daemon ratifies the action_type + final CONTRACT version at the edges→main merge, like the MIGRATION numbers). **No discipline violation.**
- edges-028 + edges-030 = CONTRACT-neutral (private daemon projection tables; no `shared/` surface).

## Reachability
- **edges-028:** reachable — real `project.rescan` Action → `ProjectExecutor` (live, main.rs) → `ProjectRescanned` → `EventStore::append` → `apply_all` → `ProjectRegistryProjector`. IPC read intentionally deferred.
- **edges-029:** reachable — `submit_action` → `CatalogPolicy` (risk-2, approval-gated) → approval → `CatalogExecutor` dispatch by `ExecutorKind::Integration` → `IntegrationExecutor` (registered live, main.rs:251) → emits `IntegrationConnectionRegistered`.
- **edges-030:** reachable — the live `integration.connect` executor emits `IntegrationConnectionRegistered` → `apply_all` → `IntegrationConnectionProjector` (mod.rs). IPC read intentionally deferred.
- No tested-but-unwired gaps (the deferred IPC reads are intentional gating, declared for /phase-exit 5+7).

## Open follow-ups (Step-9 categorized — all routed hot; HELD-for-merge per cross-track rule)
- **🔴 FINDING (lead→user ratification):** edges-029 `standing_grant_eligible=FALSE` — the §6.2-floor decision; the user may ratify or relax to true at the held edges review.
- **🟡 Cross-doc (merge-ledger, PROMINENT):** CONTRACT 0.33 + `ExecutorKind::Integration` + `integration.connect` — daemon ratifies at edges→main merge.
- **Arch-notes (HELD):** MVP `projects`/`repositories`/`integration_connections` are event-fed projections, NOT the DATA_MODEL 2.8 durable registries (deferred); the §15 #4 registration-only data-flow.
- **Completed-work ticks:** P5.1 read vertical CLOSED; P7.1 Wave-C connection vertical CLOSED; MIGRATION_10 + MIGRATION_11; SUPPORTED_USER_VERSION 9→11.
- **Lesson candidates (renumber 44+ at merge):** the event-fed-registry-projection convention (key by envelope OR payload identity; proj_ naming; defer the durable model) · the credential-registration-mutator-is-registration-only convention · the migration-floor-test convention.
- **Future TODO (consumer-gated):** the IPC read RPCs · the token→keychain credential-storage mechanism (+ a fresh cargo audit when `keyring` lands) · the durable-registry fuller models.
- **Deferred code-quality minors:** account whitespace-padding/idempotency nuance · `{provider:?}` Debug-vs-wire in the human detail string · the redundant (intentional) `event_type` test assignment.

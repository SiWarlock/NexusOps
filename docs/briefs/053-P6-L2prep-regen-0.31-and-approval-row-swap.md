# /tdd brief — regen_0.31_and_approval_row_swap

## Feature
**L2-prep consolidated regen (NON-cat-1) — the 0.28→0.31 contract sync + the approval-card
fixture→real swap.** The 0.31.0 boundary merge (`aa74674`) left the ui Zod at 0.28.0 → the §5.0
`CONTRACT_VERSION` drift sentinel is RED. This slice clears it in three layers: **(A)** regen
`generated.ts` 0.28.0→**0.31.0** (the new value-sets — the 4-value `ResumeMode`, `RecoveryState`,
any 0.29–0.31 enum additions) + the version/member-set/drift pins go GREEN; **(B)** reconcile the
now-frozen provisional shadows → frozen form (`ApprovalQueueRow` [the FIRST typed projection-row,
@0.30] + the survival enums `ResumeMode`/`RecoveryState` [@0.29] — remove the PROVISIONAL banners,
field-set/type drift-pin to the frozen `shared/src/projections.rs` + the survival schema, delegate
enum fields to the generated value-sets); **(C)** swap the approval-card **fixture** enrichment
(`enrichHunkAction`/`gatewayApprovalEnrichment` in `display-meta.ts`) → the **REAL `ApprovalQueueRow`
read** (the card sources `risk_level` + `policy_decision` from the daemon-served `proj_approval_queue`
projection, not a fixture side-map) — **resolves the 044 [med]** (no real human approves against
fixture risk). **NON-cat-1** (reads + a read-shape swap; the mutation submit stays L2-HELD), but
**`security-reviewer` REQUIRED on layer C** (the approval-card/§11.5 surface).

> **Layered, multi-commit (the impl drives layer→layer, LESSON ui-7):** A (mechanical regen → drift
> GREEN) → B (the provisional→frozen reconciles) → C (the approval-card real-read swap — its OWN
> commit + `security-reviewer`). If C proves to warrant full isolation (security-sensitivity), flag
> at Step 2.5 to split it to its own slice (053b); default is one consolidated slice per the lead.

## Use case + traceability
- **Task ID:** P6.8 L2-prep (the cross-track 0.31.0 reconcile + the 044 [med] approval-enrichment resolution; NON-cat-1; precedes the connection-state reconcile + L2)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (the generated contract layer — regen + drift pins), `§5.1` (the status/risk enums the card delegates), `§11.5` (the human-approval card renders the daemon's risk/policy), `§11.4` (the survival/recovery shapes), `§6.1` (the `get_projection("ApprovalQueue")` read the card sources from).
- **Upstream contract source (daemon-frozen, consumed not implemented here):** the daemon's `PolicyDecision` pipeline + the keychain-redacted persistence (②-mini `4e9579d`/`8c4b948`, CONTRACT 0.30.0), the survival freeze (`ResumeMode`/`RecoveryState`/`ResumeResult`, 4.1a `60b3919`, 0.29.0), the `SessionRecovered` event (4.1b-1 `1e68f20`, 0.31.0).
- **Reference:**
  - **The frozen `ApprovalQueueRow`** (`shared/src/projections.rs`) — 14 fields: `approval_id`, `action_request_id?`, `plan_id?`, `project_id?`, `session_id?`, `agent_team_id?`, `risk_level: RiskLevel`, `status: Approval`, `requester_type: RequesterType`, `requester_id`, `preview_summary?`, `requested_at`, `expires_at?`, `policy_decision: PolicyDecision?`. `deny_unknown_fields`; optionals serialize as explicit `null` (no `skip_serializing_if` — stable cross-area snapshot).
  - **The ui provisionals to reconcile** (`ui/src/contracts/provisional.ts`): `ApprovalQueueRow` (`:174`, an object shadow) · `ResumeMode` (`:30`, **STALE — only `["resumed","replayed"]` vs the frozen 4-value `resumed|replayed|relaunched|reattached_live`**) · `RecoveryState` (`:26`). `RecoveryStatus` (`:34`) is a wrapper the daemon did NOT freeze → **stays provisional** (re-base it over the generated `RecoveryState`). `ApprovalQueuePage` = `z.array(ApprovalQueueRow)` (`:184`).
  - **The regen script** `ui/scripts/gen-contracts.mjs` — emits **flat `.enum` value-sets only** (the MetricQuality `oneOf`-const limitation); it does NOT generate OBJECT shapes → see Step-2.5 Q1.
  - **The swap site** `ui/src/shell/display-meta.ts` (`enrichHunkAction`/`gatewayApprovalEnrichment` — the daemon-SHAPED fixture side-maps) + the consumer `GatewayModal` (renders `PolicyDecision`/risk, LESSON 17); the read path = the live `UdsGatewayPort.get_projection` (L1 ✅).
  - LESSON 14 (contract-bump regen discipline — `validators` = the generated bundle, never hand-list; shared-contract-seam field-*type* snapshot pin), LESSON 2 (provisional→generated reconcile), LESSON 15 (required-discriminator), LESSON 17 (the approval-card renders the daemon's decision).

## Acceptance criteria (what "done" means)
**Layer A — the regen (drift → GREEN):**
- [ ] `pnpm gen-contracts` regenerates `ui/src/contracts/generated.ts` to **`CONTRACT_VERSION 0.31.0`** (never hand-edited, LESSON 1/14); `validators` derives from the generated bundle (`= shape`), never hand-listed. The new flat-enum value-sets (the 4-value `ResumeMode`, `RecoveryState`, + any 0.29–0.31 additions) land.
- [ ] The §5.0 drift pins GREEN: `generated_contract_version_matches_frozen_schema` (the 1 current RED) + the member-set / accept-all / reject-unknown / `.options` drift tests. **`generated.test.ts` self-maintaining** (member-set + validators-keys==$defs).

**Layer B — the provisional→frozen reconciles:**
- [ ] `ResumeMode` reconciles to the **frozen 4-value** enum (delegate to the generated value-set; drop the stale 2-value provisional) — and the downstream `describeResumeMode` / the sidebar resume-mode indicator (LESSON 8) render the 2 NEW values (`relaunched`, `reattached_live`) without a fall-through gap (pin it). `RecoveryState` reconciles to the frozen enum.
- [ ] `ApprovalQueueRow` reconciles from PROVISIONAL → a **frozen-shadow** (banner removed): the field-set + field-types **drift-pinned** to `shared/src/projections.rs` (the shared-contract-seam snapshot, field-name + type per LESSON 14); enum fields (`risk_level`/`status`/`requester_type`) **delegate** to the generated value-sets; `policy_decision` delegates to the `PolicyDecision` shadow; `.strict()` per `deny_unknown_fields`; optionals present-and-nullable. `ApprovalQueuePage` uses it. (See Step-2.5 Q1 — generated vs drift-pinned-shadow.)
- [ ] `RecoveryStatus` STAYS a provisional wrapper, re-based over the generated `RecoveryState` (the daemon did not freeze the wrapper — note it). `SessionRecovered`/the recovery-event **UI consumption is DEFERRED** to a follow-on (the §11.4 recovery-UX slice) — this regen brings the generated types only (Step-2.5 Q4).
- [ ] The boundary parser (`parseProjectionPage` for `ApprovalQueue`) parses the reconciled `ApprovalQueueRow` (parse-don't-trust holds).

**Layer C — the approval-card fixture→real swap (own commit + `security-reviewer`):**
- [ ] The approval card sources `risk_level` + `policy_decision` from the **real `ApprovalQueueRow`** (read via `get_projection("ApprovalQueue")` → matched by `approval_id`), NOT the `enrichHunkAction`/`gatewayApprovalEnrichment` fixture side-map. The card renders the daemon's authoritative risk/decision (LESSON 17 — never UI-derived/invented). **Resolves the 044 [med].**
- [ ] No fixture risk/policy reaches the card on the real path; a missing/absent `ApprovalQueueRow` row → an honest pending/absent treatment (never a fabricated risk, forbidden #4/#2). The **mutation submit stays L2-HELD** (the swap is read-only — the card displays real risk; submitting an action is still cat-1 L2).
- [ ] **`security-reviewer` REQUIRED** (layer C): the card shows real daemon risk (not fixture) before any approval; no UI-derived risk; the read is parse-don't-trust; the swap adds no mutation reach.
- [ ] Whole suite green (328 + the reconcile pins); `/preflight` clean (tsc/oxlint/vitest); cross-doc flagged at Step 9.

## Wiring / entry point (Step 7.5)
**REAL — the approval card reads real risk on the live path.** `GatewayModal` (Code/Diff per-hunk
approval + any approval render) → sources `risk_level`/`policy_decision` from
`UdsGatewayPort.get_projection("ApprovalQueue")` → the daemon's `proj_approval_queue` (the typed
`ApprovalQueueRow`). `/wired`: the card's risk/policy now traces to the live projection read, not the
`display-meta.ts` fixture. The generated/reconciled types are consumed by the boundary parsers + the
card. (Layers A/B are exposed-ahead-of/under the existing consumers; C is the live wiring.)

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` (regen → 0.31.0; never hand-edited) + `generated.test.ts` (drift pins) · `ui/src/contracts/index.ts` (re-exports if new value-sets).
- `ui/src/contracts/provisional.ts` (`ApprovalQueueRow`→frozen-shadow + banner removal; `ResumeMode`/`RecoveryState`→delegate; `RecoveryStatus` re-base) + `provisional.test.ts` (the shared-contract-seam field/type snapshot pins).
- `ui/src/gateway-client/boundary.ts` (the `ApprovalQueue` page parse, if the shape moved).
- `ui/src/shell/display-meta.ts` (layer C — the fixture→real `ApprovalQueueRow` read) + `ui/src/overlays/GatewayModal.tsx` (sourcing) + their tests.
- Possibly `ui/src/status/descriptors.ts` / `describeResumeMode` (the 2 new `ResumeMode` values).

If beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2)
**A:** 1. `generated_contract_version_matches_frozen_schema` (0.31.0 — the current RED → GREEN). 2. member-set/`.options` drift for the new value-sets.
**B:** 3. `resume_mode_has_frozen_four_values` (delegates to generated; the 2 new values covered — no fall-through in `describeResumeMode`). 4. `approval_queue_row_field_set_and_types_match_frozen` (the shared-contract-seam snapshot vs `shared/src/projections.rs`; enum-field delegation; `.strict()` reject-extra; optionals nullable). 5. `recovery_state_delegates_to_generated` + `recovery_status_rebased_over_generated`.
**C:** 6. `approval_card_sources_real_risk_policy_from_projection` (the card reads `ApprovalQueueRow.risk_level`/`policy_decision`, not the fixture). 7. `no_fixture_risk_on_real_path` + `absent_row_is_honest_pending_not_fabricated`. 8. `swap_adds_no_mutation_reach` (submit stays L2-HELD/not-wired).
Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** the ui adopts the frozen `ApprovalQueueRow` (provisional→frozen-shadow) + the frozen `ResumeMode`(4)/`RecoveryState` — consuming the daemon's frozen schema (no ui-authored shared model).
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` Generated-contract row (34→… value-sets, 0.28.0→**0.31.0**) + the GatewayPort/approval-card row (the fixture→real `ApprovalQueueRow` swap — 044 [med] RESOLVED) + a provisional-shapes row update (`ApprovalQueueRow`/`ResumeMode`/`RecoveryState` reconciled; `RecoveryStatus` still provisional). **No `ARCHITECTURE.md` edit** (consumes the frozen daemon contract).
- **Shared-contract (cross-area) model touched?** YES — `ApprovalQueueRow` is a frozen cross-area projection-row; the RED outline includes the **schema-snapshot test** (field-name + type set vs `shared/src/projections.rs`, tagged `spec(§5.0)`), authored this cycle.

## Things to flag at Step 2.5
1. **`ApprovalQueueRow`/`ResumeResult` (objects): generated vs frozen-shadow.** `gen-contracts.mjs` emits flat enums only. Default: keep `ApprovalQueueRow` a **hand-declared frozen-shadow** (banner removed, field-set/type drift-pinned to the schema, enum fields delegated) — the existing object pattern; do NOT extend the generator to objects in this slice (that's a separate generator change, carry-forward). Confirm, or extend the generator if cheaper.
2. **The C swap scope + isolation.** Default: land C in this slice as its OWN commit with `security-reviewer` (approval-card/§11.5). Flag to split C → 053b if the impl judges the security surface warrants full isolation from the mechanical regen.
3. **`ResumeMode` 2→4 values — downstream coverage.** The stale 2-value provisional grows to 4 (`relaunched`/`reattached_live`). Ensure `describeResumeMode` + the sidebar indicator (LESSON 8) render the new values (glyph+label, never color-alone, §11) — pin no fall-through.
4. **`SessionRecovered`/recovery-UX = DEFER.** This regen brings the generated types; the `SessionRecovered`-event→`RecoveryStatus`-aggregate→§11.4-recovery-UI consumption is the daemon's carry-forward (3) follow-on (a later ui-feeding slice), NOT this slice. Confirm the scope line.

## Dependencies + sequencing
- **Depends on:** the 0.31.0 boundary merge (`aa74674` — DONE) + the live `UdsGatewayPort` read transport (L1 ✅, 049–052 — the card's real read rides it).
- **Blocks:** the **connection-state single-authority reconcile** slice (next; the 052 Finding fix, lead-RULED) → then **L2 (CAT-1)** — its own file-based cat-1 checkpoint. L2's live `submit_action`/`approve`/`deny` consumes this slice's real `ApprovalQueueRow` card.

## Estimated commit count
**2–4** (A regen → B reconciles → C the swap [own commit + `security-reviewer`]). **NON-cat-1** (reads + a read-shape swap; the mutation submit stays L2-HELD) — but **`security-reviewer` REQUIRED on layer C** (the approval-card surface; resolves the 044 [med]).

## Lessons-logged candidates anticipated
- **Convention candidate** — possibly: "a frozen projection-row (`ApprovalQueueRow`) reconciles provisional→frozen-shadow (banner removed, cross-area field/type snapshot-pinned to `shared/src/projections.rs`, enum fields delegated) — objects stay hand-declared-but-drift-pinned while `gen-contracts.mjs` is enum-only; the approval card sources real `risk_level`/`policy_decision` from the served projection (never fixture/UI-derived, LESSON 17), resolving the fixture-risk [med]." Surface at Step 9.
- **Architecture-doc note candidate** — the approval-enrichment is now real (the 044 [med] gate cleared); the only L2 blocker left is the connection-state reconcile + the cat-1 checkpoint.
- **Future TODO** — the `SessionRecovered`→recovery-UX consumption; the remaining projection-row freezes (SessionRow/etc); the `gen-contracts.mjs` object-generation extension (+ the MetricQuality `oneOf`-const support).

## How to invoke
1. **Read this brief end-to-end** — the 3 layers + the 4 Step-2.5 questions (esp. Q1 object-shape + Q2 the C swap).
2. Pre-flight: `track/ui` @ `aa74674` (0.31.0 merged); the drift is RED (expected).
3. **Run `/session-start`** (this is a new round after the L1 `/session-end`), then **`/tdd regen_0.31_and_approval_row_swap`**.
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 4 questions + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED on layer C (the approval-card real-risk swap).
7. Step 9 — the cross-doc flags (the contract row → 0.31.0 + the 044 [med] RESOLVED) + the reconcile lesson candidate.

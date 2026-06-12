# /tdd brief — contract_regen_0190_and_bounded_provisional_reconcile

## Feature
Regenerate the UI's Zod contract layer against the now-frozen `shared/` schema
**0.19.0** (the Phase-2 Action-Gateway freeze), clear the §5.0 drift tripwire,
and perform the **bounded** provisional→generated reconcile: adopt the 13 new
Gateway enum value-sets, handle the two enum renames
(`ActionRequest`→`ActionRequestStatus`, `Approval`→`ApprovalStatus`), and adopt
the §6.4 frame/error surface (`IpcErrorCode` codes `fencing_conflict` /
`internal_error`, `DeltaKind`, `ProjectionDelta`, `ServerFrame`). **Projection-row
object shapes that 0.19.0 does NOT freeze stay provisional** (SessionRow / PR /
Usage / survival / §17-safety), and the `ProjectionName`→`ProjectionNameEnum`
bare-name retirement stays a flagged follow-up.

> **This re-cuts the §5.0 generated contract layer that the paused ui track built
> at 0.5.0/0.12.0, now against the 0.19.0 freeze. It is the FIRST slice of the ui
> resume; the intent seam (slice 2) builds on the contract this slice adopts.**

## Use case + traceability
- **Task ID:** P6.2
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (generated,
  drift-caught Zod consumer), `§5.1` (status-machine enum strings), `§6.4`
  (IPC frame-mux + error codes).
- **Widens phase scope because** the ui-track resume re-cuts the **§5.0** generated
  contract layer against the Phase-2 freeze and adopts the **§6.4** IPC frame/error
  surface — both are P2-contract surfaces the paused ui track (whose Phase-6
  `**Spec anchors:**` line predates the P2 freeze) now consumes. §5.1 is already
  in the Phase-6 anchor set; §5.0/§6.4 are the contract-resume widen.
- **Related context:** carry-forward "Reconcile UI provisional object types →
  generated" + "ui ↔ daemon-1.5 integration" (the `UsageLedger` rename + the
  `ServerFrame`/`ProjectionDelta`/`protocol_error` adoption); `docs/sessions/ui-007`
  (the 0.8.0/0.12.0 regen mechanics + the `ProjectionName` bare-name follow-up);
  `ui/CLAUDE.md` cross-doc rows (Generated Zod contract layer; Status→attention-rank
  table); Lessons §1 (generated, never hand-declared) + §2 (provisional shapes,
  enum fields delegated) + §5 (descriptor completeness drift-pin).

## Current state (RED already present)
`ui/src/contracts/generated.test.ts` is **already failing** — three drift checks:
1. `generated_zod_member_sets_equal_frozen_schema` — generated has **20** value-sets;
   the frozen 0.19.0 schema has **33**.
2. `generated_contract_version_matches_frozen_schema` — `CONTRACT_VERSION` `"0.12.0"`
   ≠ schema `"0.19.0"`.
3. `generated_zod_accepts_every_canonical_enum_value` — the new value-sets have no
   generated validators yet.

That RED **is** this slice's RED — the regen + reconcile drives it GREEN. The two
renames will additionally turn `descriptors.test.ts`, `safety/model.test.ts`, and
`two-surface.test.tsx` RED until the reconcile lands; making the **whole** suite
green is the GREEN bar.

## The delta (verified against `shared/contracts/schema/nexusops-contract.schema.json`)
**2 renames** (0.12.0 → 0.19.0): `ActionRequest`→`ActionRequestStatus`,
`Approval`→`ApprovalStatus`. `ActionRequestStatus` **retains** `partially_succeeded`
+ `rollback_failed` (the `AuditOutcomeStatus.extract` survives — retarget its source).

**13 new value-sets** (all Phase-2 Gateway enums, no UI consumer yet):
`ActionResultStatus`, `ApprovalMode`, `ApprovalScope`, `EvidenceConfidence`,
`EvidenceType`, `ExecutorKind`, `GatewayObjectKind`, `IdempotencyFormula`,
`PolicyDecisionStatus`, `PreviewClass`, `RequesterType`, `RequiredApproverKind`,
`ResourceType`.

**§6.4:** `IpcErrorCode` gains `fencing_conflict`, `internal_error`,
`precondition_stale`, `protocol_error` (regen brings them automatically).
`ProjectionDelta` frozen shape `{id,kind,projection,row}` == the current provisional
shape (only `row`'s typing differs). `ServerFrame` is **new** (oneOf
`rpc_response` | `subscription_push`). `ProjectionName` now has 10 members
(provisional registry covers 6 — the 4 unbuilt projections stay out, bare-name
retirement flagged).

## Acceptance criteria (what "done" means)
- [ ] `ui/src/contracts/generated.ts` is **regenerated via `npm run gen:contracts`**
      (NOT hand-edited — Lesson §1): `CONTRACT_VERSION === "0.19.0"`, all 33 value-sets present.
- [ ] All three `generated.test.ts` drift checks pass.
- [ ] `index.ts`: contract exports + `validators` keys use the schema `$def` names
      (`ActionRequestStatus`/`ApprovalStatus`, not the old names); all 33 value-sets
      are reachable for the drift test (per Q5 — derived from `bundle.shape`, or hand-listed).
- [ ] `provisional.ts`: the `Approval`/`ActionRequest` delegations + the
      `AuditOutcomeStatus = …extract(["partially_succeeded","rollback_failed"])` source
      retarget to the renamed enums; **no enum re-declared** (Lesson §2).
- [ ] Projection-row provisional shapes (SessionRow / PR / ApprovalQueue / AuditEvent /
      Usage / RecoveryState / §17-safety) **remain provisional** — only their
      already-delegated enum fields follow the renames. No row object swapped to a
      "generated" object (none exists — the generator emits enums only).
- [ ] `descriptors.test.ts` completeness sweep is green with the rename (per Q1).
- [ ] `safety/model.ts` + `safety/model.test.ts` import/assert the renamed
      `ActionRequestStatus`; the §17 audit-integrity `.extract` still pins
      `partially_succeeded`/`rollback_failed`.
- [ ] `ui/src/gateway-client/mock.test.ts` tripwire bumped `0.12.0`→`0.19.0` (+ comment).
- [ ] (Per Q3) `ServerFrame` adopted as decided at Step 2.5 — if defined, a
      schema-pinned drift test pins its variant field-sets (§2.5-seam).
- [ ] Whole suite green (the 3 RED → green; **no regressions** — target 214/214).
- [ ] `/preflight` clean (oxlint + tsc + test:run).
- [ ] Cross-doc invariant flagged at Step 9 (the Generated-contract row: "13 enum
      value-sets"→"33", 0.12.0→0.19.0) — orchestrator writes the `ui/CLAUDE.md` row.

## Wiring / entry point (Step 7.5)
**none new** — the regenerated contract layer is consumed by **already-wired**
code: the `gateway-client` boundary parsers (`parseProjectionPage` / capabilities),
`StatusPill`/`descriptors` (status render + completeness drift-pin), and
`safety/model.ts`. The slice changes the contract surface those consumers read;
no new production entry point. The **13 new value-sets are intentionally
exposed-ahead-of-consumer** (the intent seam / Brain drawer consume them in slices
2/8.2) — same pattern as the 0.12.0 daemon-1.5 enum additions; flag at Step 7.5 as
expected, not a wiring miss. `ServerFrame` (if defined) is likewise contract-ahead
(the demux wires with the real `UdsGatewayPort`).

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` — REGENERATED (`npm run gen:contracts`); never hand-edited.
- `ui/src/contracts/index.ts` — rename the two exports + `validators` keys to schema
  names; expose the 13 new value-sets (Q5: derive `validators` from `bundle.shape`,
  keep named exports only for consumed enums); keep `ProjectionNameEnum` bare-name handling.
- `ui/src/contracts/provisional.ts` — retarget the `Approval`/`ActionRequest`
  delegations + the `AuditOutcomeStatus.extract` source; confirm `ProjectionDelta`
  matches frozen; (Q2) §6.4-codes treatment; (Q3) optional `ServerFrame`.
- `ui/src/status/descriptors.test.ts` — machine-name↔validator-key handling for the
  rename (Q1 default: 2-entry alias map; the descriptor TABLE machine keys stay stable).
- `ui/src/safety/model.ts` + `ui/src/safety/model.test.ts` — `ActionRequest`→`ActionRequestStatus`.
- `ui/src/gateway-client/mock.test.ts` — tripwire `0.12.0`→`0.19.0`.

**Only if Q1 resolves to "rename machine identifiers" (NOT the default):**
`status/StatusPill.tsx`, `status/two-surface.test.tsx`, `status/StatusPill.test.tsx`,
`projections/items.ts`, `projections/items.test.ts`, `overlays/GatewayModal.tsx`,
`overlays/overlays.test.tsx` (the `data-machine`/`data-item-id` namespace) — ~7 extra files.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
The RED **pre-exists** (the 3 drift checks). The slice makes them + the
rename-broken existing tests GREEN. The only **net-new** test is conditional:

1. **(pre-existing) `generated.test.ts` ×3** — member-set / version / accept-canonical.
   - Asserts: generated layer == frozen 0.19.0 schema (33 value-sets, version match).
   - Why: §5.0 drift-caught consumer; Lesson §1.
2. **(pre-existing, will RED on rename) `descriptors.test.ts` / `safety/model.test.ts` /
   `two-surface.test.tsx`** — must be GREEN post-reconcile.
   - Asserts: completeness sweep + the §17 `.extract` + the two-surface R-5 still hold under the renamed enums.
   - Why: §5.1 status binding; §17 audit-integrity; Lesson §5.
3. **NEW — only if Q3 = "define ServerFrame":** `serverframe_variant_fields_match_frozen_schema`
   - Asserts: the `ServerFrame` provisional's two variant field-sets == the frozen
     schema's `ServerFrame.oneOf` variant property sets (`rpc_response`,
     `subscription_push`), tagged `spec(§6.4)`.
   - Why: §2.5-seam shared contract — a schema-snapshot test so a daemon frame-shape
     change fails loudly (template §2.5-seam rule).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** the contract layer's value-set **count** changes (20→33)
  + two enum **renames** + `CONTRACT_VERSION` 0.12.0→0.19.0. No new UI-authored
  invariant; the generated layer mirrors the daemon authority.
- **Orchestrator doc rows to write hot (Step 9 routing):** update the **`ui/CLAUDE.md`
  "Generated Zod contract layer" cross-doc row** — "13 enum value-sets (9 status
  machines + ActorType/IdKind/DesktopObjectKind)" → the 0.19.0 count/breakdown +
  `CONTRACT_VERSION 0.19.0`. The "Status→attention-rank table" row stays accurate
  under the Q1 default (machine keys unchanged). The **`ARCHITECTURE.md` Appendix-A /
  §5.0 row** (if it pins the value-set count) routes to the **integration checkout**
  per the lead's shared-root-doc rule — flag it; the orchestrator coordinates.
- **§2.5-seam model touched?** Yes — `ProjectionDelta`/`ServerFrame` cross the
  daemon↔ui §2.5 seam (§6.4). If `ServerFrame` is defined (Q3), the RED **must**
  include the schema-snapshot test above.

## Things to flag at Step 2.5
1. **Machine-name ↔ enum-name coupling on the rename.** The schema renamed the value-sets
   (`ActionRequest`→`ActionRequestStatus`, `Approval`→`ApprovalStatus`); the
   `validators` keys MUST follow (drift test pins keys to `$def` names). But the UI's
   **status-machine identifiers** (`StatusPill machine=`, descriptor TABLE keys,
   `data-machine`/`data-item-id` namespace, `ProjectionItem.machine`) are a separate,
   **UI-canonical render-policy** naming. Options: **(A-lite)** rename only the
   contract exports/`validators` keys; keep the UI machine identifiers stable
   (`"Approval"`/`"ActionRequest"`); add a 2-entry `{Approval:"ApprovalStatus",
   ActionRequest:"ActionRequestStatus"}` alias in the completeness lookup — ~7 files,
   `data-*` namespace untouched. **(B)** rename the machine identifiers everywhere to
   match the schema — ~14 files, churns the `data-item-id` namespace + the
   `overlays.test` selectors. My default vote: **A-lite** — the descriptor table is
   UI-canonical render policy (cross-doc row says so), the human-facing labels
   (`"Action"`/`"Approval"`) are already a separate map and don't change, the drift-pin
   stays intact, and it's the minimal-regret diff. (Escalate to me if you think the
   UI machine name should track the contract — it's UI-render-policy, settle-able
   between us, not a human escalation.)
2. **§6.4 codes — adopt = regen-only, or rewire `ConflictReason`?** `fencing_conflict`
   is now a frozen `IpcErrorCode` member, so the provisional
   `ConflictReason = z.enum(["fencing_conflict"])` *could* delegate via
   `IpcErrorCode.extract(["fencing_conflict"])`. Default vote: **regen-only — keep
   `ConflictReason` provisional, do NOT rewire it.** The §17 conflict schema is NOT
   frozen (it stays in the "provisional until §17 freeze" bucket); rewiring a safety
   surface belongs with the intent-seam slice (slice 2, security-reviewer present),
   not a mechanical regen. "Adopt the §6.4 codes" = the regen brings the updated
   `IpcErrorCode` member set; any error-code consumer gains them automatically.
3. **`ServerFrame` adoption depth.** Options: **(define)** add a schema-pinned
   provisional `ServerFrame` discriminated union (+ the snapshot test) now —
   drift-aware, readies slice 2; **(defer)** leave it to the intent-seam/transport
   slice that actually demuxes. Default vote: **define it** (cheap, the lead named it,
   a drift-tested type isn't speculative) but do **NOT** rewire the subscribe transport
   (still single-connection → `ProjectionDelta`; the daemon makes subscribe a dedicated
   terminal connection — no demux needed in MVP). Flip to **defer** if you judge an
   unused type out of the bounded scope.
4. **`ProjectionDelta.row` widening.** Frozen `row` is projection-agnostic; provisional
   types it `SessionRow.optional()`. The 6.2 "widen to projection-discriminated" box is
   gated on **non-Session subscriptions landing**, not on the contract bump. Default
   vote: **keep Session-specific (defer the widen)** — no non-Session subscription
   exists yet; widening now adds coupling for no consumer. The box stays unticked
   honestly.
5. **`validators` registry — derive from `bundle.shape` or hand-list all 33?** Currently
   hand-listed (20 entries) — which is exactly what forced this reconcile. Default vote:
   **derive `validators` from `bundle.shape`** (auto-covers all 33 + future additions →
   the drift-pin's "keys equal" check self-maintains; named exports stay only for
   *consumed* enums: `Session`, `ActorType`, `IdKind`, `ActionRequestStatus`, etc.).
   Future contract bumps then become a pure `npm run gen:contracts` with no `index.ts`
   edit unless a new enum gains a typed consumer. Flip to hand-list if you prefer the
   explicit surface.

## Dependencies + sequencing
- **Depends on:** the 0.19.0 freeze (landed — merge `cc2cc78`). Nothing else.
- **Blocks:** **slice 2** (the intent seam + real `GatewayModal` + permission card)
  — it consumes the `ActionRequestStatus`/`ApprovalStatus`/`ApprovalMode`/`ApprovalScope`/
  `PreviewClass`/`PolicyDecisionStatus`/`RequiredApproverKind` enums + `ServerFrame`
  this slice adopts. Also unblocks promoting the **ui CI job to blocking** (the §5.0
  drift sentinel clears with this regen — carry-forward residual (i)).

## Estimated commit count
**1.** One logical unit (contract resume). **Not safety-critical** — no intent/mutation
path is wired here (INV-SEC-1 stays daemon-side; this is the read/contract layer), so
no `security-reviewer` and no own-commit safety rule. Bundles cleanly. (If Q1 resolves
to B, it's still one commit — just a wider mechanical find-replace.)

## Lessons-logged candidates anticipated
- **Convention candidate** — "On a contract bump, `validators` derives from the
  generated bundle (never hand-listed); named exports exist only for consumed enums;
  enum **renames** retarget delegations + `.extract` sources, never re-declare"
  (extends Lessons §1/§2).
- **Architecture-doc note candidate** — the value-set count is now 33 at 0.19.0
  (`ui/CLAUDE.md` Generated-contract row); the UI status-machine identifier is a
  UI-render-policy name decoupled from the frozen enum `$def` name (the A-lite
  decision, if taken).
- **Future TODO — next-brief working set** — `ServerFrame` demux wiring + the
  `ProjectionName`→`ProjectionNameEnum` bare-name retirement + `ProjectionDelta.row`
  projection-discrimination all stay flagged for the intent-seam / non-Session-subscription slices.

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" (5 design questions).
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd contract_regen_0190_and_bounded_provisional_reconcile`.**
4. Step 0 (Restate) — confirm against the Feature line.
5. Step 1 (Identify files) — confirm against "Files expected to touch".
6. **Step 2.5** — answer the 5 design questions (or take defaults) and send the test-design write-up; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
7. Step 9 — surface the cross-doc invariant flag (Generated-contract row) + anything beyond the anticipated lessons.

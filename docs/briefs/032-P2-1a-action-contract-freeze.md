# /tdd brief — action_contract_freeze

## Feature
Freeze the **§6.2 Gateway core data model** in the `shared/` authority crate — the typed `ActionRequest` / `ActionPlan` / `Approval` / `ActionResult` family + their enums + the new platform IDs — so the action contract is stable and the ui + edges tracks can fan out against it. **No daemon behavior** lands here (pipeline, durable rows, executors, events → 2.1b); this is the pure cross-language contract freeze (§5.0 Option-A: Rust authority → schema artifact → Zod/Pydantic).

> **This is the §2.5 forced-serial bottleneck's freeze step.** "Freeze the action contract early, then fan out." Get the field sets right the first time — a re-freeze thrashes every downstream consumer.

## Use case + traceability
- **Task ID:** P2.1a (split from plan task 2.1 — the contract-freeze half; 2.1b = the daemon pipeline)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.2` (Gateway core data model — PRIMARY), `§6.1` (the wire ack types `ActionAck`/`PlanAck`), `§5.1` (Approval(10)/ActionRequest(15) machines — **already frozen** in `shared/src/status.rs`; this slice references them), `§5.2` (the new platform IDs), `§5.0` (the freeze mechanism), `§15` (reject-unknown / fail-closed parse boundary).
- **BINDING source vs. origin:** the binding field sets are **`ARCHITECTURE.md` Appendix A (rows ActionRequest / ActionPlan / Approval / ActionResult / ResourceRef·EvidenceRef·PolicyDecision) + `docs/planning/DATA_MODEL.md` sec 2.9 (the `action_requests`/`approvals` DDL)**. **`docs/domains/ACTION_GATEWAY.md` sec 9.1–9.9 is ORIGIN/RATIONALE only** (a planning doc with its own section-numbering — NOT `ARCHITECTURE.md` anchors) — its `ActionRequest` draft is *richer* than the reconciled binding contract (it carries `policyContext`, `preconditions`, `dryRunRequired`, `confirmationPreference`, `source`, `intent`, and session/team/workflow context fields that Appendix A **reconciled OUT** — the action's context lives on the event envelope's typed columns; policy/preconditions → 2.2/2.4; expiry → `Approval`). **Freeze the Appendix-A set, not the AG draft.** (See Step-2.5 Q1.)
- **Related context:** brief `031` (2.0-SEC, the prior slice); `shared/src/{status,ids,actor,events,event_envelope,objects,schema}.rs` + `shared/tests/contract.rs` are the patterns to mirror; CONTRACT_VERSION is currently **0.14.0**.

## Acceptance criteria (what "done" means)
- [ ] A new `shared/src/actions.rs` defines the §6.2 models as `serde` + `JsonSchema` types with `#[serde(rename_all = "snake_case")]` + `#[serde(deny_unknown_fields)]` (reject-unknown, §15/§5.0 — mirrors `events.rs`): `ActionRequest`, `ActionPlan`, `ActionPlanStep`, `ActionDependency`, `ActionPreview`, `Approval`, `ActionResult`, `ResourceRef`, `EvidenceRef`, `PolicyDecision`.
- [ ] The NEW enums are frozen, snake_case wire values, with an `ALL` const (mirror the `status_machine!`/`IdKind` pattern): `RequesterType`(6), `RiskLevel`(0–4), `ApprovalScope`(3), `ApprovalMode`(4), `PolicyDecisionStatus`(6), `ActionResultStatus`(4), `ResourceType`(see Q7), `EvidenceType`(11), `EvidenceConfidence`(4).
- [ ] The model structs reference the **already-frozen** `ActionRequestStatus`(15) + `ApprovalStatus`(10) enums (`shared/src/status.rs`) — they are NOT re-defined here.
- [ ] The NEW platform IDs are minted prefixed-ULID newtypes **following the `shared/src/objects.rs` desktop-object precedent** (RULED Option A — see Q2): `ApprovalId`(`appr_`) + `ActionPlanId`(`aplan_`), keyed off a NEW `GatewayObjectKind` enum (sibling to `DesktopObjectKind`) via a `gateway_minted_id!` macro mirroring `desktop_minted_id!` (`objects.rs:58-122`) — so the frozen 22-member `IdKind` enum + `test_all_22_ids_present_with_prefixes` stay **untouched** (these are "platform-minted, non-cross-product (Gateway objects)", exactly as the 4 desktop objects are non-cross-product). `ActionRequestId`(`act_`) already exists in `IdKind`. Verify `appr_`/`aplan_` collide with none of the 16 `IdKind` + 4 `DesktopObjectKind` prefixes.
- [ ] The `RequesterType`→`ActorType` alias map (R-2) is implemented + tested: `agent_session→session_adapter`, `workflow_pack→workflow_runtime`, `system_policy→automation_policy`; `user`/`project_brain`/`remote_client` map straight through.
- [ ] A `Timestamp` newtype (`#[serde(transparent)]` over `String`, schemars `format: "date-time"`, RFC3339-validating `parse`) is introduced and used for every timestamp field in the new models (Carry-forward fold; see Q8 re: retrofitting the envelope).
- [ ] **§2.5-seam schema-snapshot tests** (mandatory — these models cross a §2.5 dependency edge): one test per model asserting its **field-name set == a checked-in snapshot**, each tagged `spec(§6.2)` (see RED outline). A field added/removed/renamed fails the snapshot — this is the freeze guard.
- [ ] `CONTRACT_VERSION` bumped (0.14.0 → see Q9) + the schema artifact regenerated + the 3-way (Rust/Zod/Pydantic) verify extended to cover the new types + green.
- [ ] `ContractBundle` (`shared/src/schema.rs`) extended so the new types are in the emitted schema (so `test_schema_artifact_matches_rust` covers them).
- [ ] `/preflight` clean (fmt + clippy `-D warnings` + check + test); cross-doc invariants flagged at Step 9 (orchestrator writes the Appendix-A reconcile + `daemon/CLAUDE.md` rows).

## Wiring / entry point (Step 7.5)
**This is a contract-freeze slice — its "entry point" is the published contract surface, not a runtime call path.** The new types are `pub`-exported from `shared/src/lib.rs` (a new `pub mod actions;`) and enter the §5.0 propagation pipeline: `emit_schema` bin → the committed `contracts/schema/*.json` artifact → the generated Zod/Pydantic → the 3-way verify. **Production consumers land in later slices** (named here so the freeze isn't orphaned): the daemon Gateway pipeline (2.1b) constructs/persists these types; the ui regenerates its Zod against the bumped schema at its track resume (Carry-forward `ui ↔ daemon` reconcile); the edges track's executors bind the frozen `ActionRequest`/`ResourceRef`. **Reachability for this slice = "exported from `shared` + present in the regenerated schema artifact + 3-way-verified"** — there is no daemon entry point until 2.1b (state this explicitly at Step 7.5; it is the correct answer for a contract-only slice, mirroring how the 0.5 freeze + the 1.5 IPC types landed).

## Files expected to touch
**New:**
- `shared/src/actions.rs` — the 10 §6.2 model structs + 9 new enums + the `RequesterType→ActorType` map.
- (the `Timestamp` newtype: `shared/src/actions.rs` or a small `shared/src/time.rs` — Q8.)

**Modified:**
- `shared/src/lib.rs` — `pub mod actions;` (+ `pub mod time;` if split) + the `CONTRACT_VERSION` bump.
- the new `ApprovalId`/`ActionPlanId` newtypes + `GatewayObjectKind` + `gateway_minted_id!` — **mirror `shared/src/objects.rs`** (the `desktop_minted_id!`/`DesktopObjectKind` pattern). Place in `objects.rs` alongside the desktop tier, a new `shared/src/gateway_ids.rs`, or `actions.rs` — implementer's call (Q2 latitude). `shared/src/ids.rs` itself stays UNTOUCHED (the frozen 22 + `IdKind`).
- `shared/src/schema.rs` — extend `ContractBundle` with the new types.
- `shared/tests/contract.rs` — the schema-snapshot tests + the new-enum wire-value tests + the `RequesterType` map test + the new-id newtype tests.
- `shared/tests/envelope.rs` — only if Q8 retrofits the envelope's `occurred_at`/`recorded_at`/`seq`.
- the committed schema artifact (`contracts/schema/*.json`) + the 3-way verify fixtures (regenerate via the `emit_schema` bin + the established `gen-contracts`/verify flow).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2) — `shared/tests/contract.rs` (+ a unit section in `actions.rs`)
1. **`test_action_model_field_names_snapshot`** (the §2.5-seam guard) — for each of the 10 models, assert the serialized field-name set == a checked-in expected set.
   - Asserts: `serde_json::to_value(&Model::sample()).as_object().keys()` (or a schemars-introspection of the model's `properties`) == the frozen field list.
   - Why: §2.5-seam mandate (brief-template "schema-snapshot test"); tag `spec(§6.2)`. This is THE freeze guard — adding/removing/renaming a field fails it.
2. **`test_requester_type_values`** — `RequesterType::ALL` serializes to exactly the 6 snake_case wire strings.
   - Asserts: round-trip + `deny_unknown` rejects an unknown value. Why: §6.2 / R-2 / §15 reject-unknown.
3. **`test_requester_type_maps_to_actor_type`** — the 3 aliases + 3 straight-throughs resolve to the right `ActorType`.
   - Asserts: `agent_session→SessionAdapter`, `workflow_pack→WorkflowRuntime`, `system_policy→AutomationPolicy`, `user→User`, `project_brain→ProjectBrain`, `remote_client→RemoteClient`. Why: Appendix-A R-2 binding mapping.
4. **`test_risk_level_wire_is_0_to_4`** — `RiskLevel` serializes to integers 0–4 (per Q3 ruling), `ALL` is 5 variants, ordered.
   - Asserts: serde value == 0..=4; an out-of-range integer is rejected. Why: §6.2 risk 0–4 (AG sec 7 risk levels).
5. **`test_approval_scope_and_mode_and_policy_status_and_result_status_values`** — the 4 closed enums serialize to their exact wire sets (ApprovalScope 3, ApprovalMode 4, PolicyDecisionStatus 6, ActionResultStatus 4).
   - Why: §6.2 / Appendix A. reject-unknown each.
6. **`test_resource_type_and_evidence_type_values`** — `ResourceType` (the AG sec 9.8 set — see Q7 re: 20-vs-21) + `EvidenceType`(11) + `EvidenceConfidence`(4) serialize exactly.
   - Why: §6.2 (ResourceRef/EvidenceRef; AG origin sec 9.8/9.9).
7. **`test_gateway_minted_id_newtypes`** — `ApprovalId`/`ActionPlanId` mint with the right prefix + parse/reject (fail-closed) per the `desktop_minted_id!` pattern; **`test_idkind_still_22`** asserts `IdKind::ALL.len() == 22` unchanged; **`test_gateway_prefixes_dont_collide`** asserts `appr_`/`aplan_` ∉ the 16 `IdKind` + 4 `DesktopObjectKind` prefixes.
   - Why: RULED Option A — `IdKind` frozen; the new ids are non-cross-product Gateway objects (the desktop-object precedent, ARCH sec 5.3 / `DesktopObjectKind`).
8. **`test_timestamp_newtype_format_and_rfc3339`** — `Timestamp` parses a valid RFC3339, rejects garbage, schemars emits `format: date-time`, serializes transparently (wire value == the bare string).
   - Why: Carry-forward (1.1 L1) + §5.0.
9. **`test_action_request_round_trips_with_required_and_optional_fields`** — a full `ActionRequest` (and one with all optionals `None`) round-trips; `deny_unknown_fields` rejects an extra key.
   - Why: §6.2 / §15 fail-closed parse boundary.
10. **`test_schema_artifact_matches_rust`** (existing — extend) + **`test_contract_version_bumped`** — the regenerated artifact covers the new types; CONTRACT_VERSION moved past 0.14.0.
    - Why: §5.0 propagation gate.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW — `ActionRequest`, `ActionPlan`, `ActionPlanStep`, `ActionDependency`, `ActionPreview`, `Approval`, `ActionResult`, `ResourceRef`, `EvidenceRef`, `PolicyDecision` + the 9 enums + `ApprovalId`/`ActionPlanId` + `Timestamp`.
- **Orchestrator doc rows to write hot (Step 9):** reconcile the `ARCHITECTURE.md` Appendix-A rows (ActionRequest/ActionPlan/Approval/ActionResult/ResourceRef·EvidenceRef·PolicyDecision) to the **as-frozen** field sets (the rows are currently prose summaries — make them match the code) + add the §5.2/Appendix-A note for the two non-cross-product platform IDs (Option A) + the `daemon/CLAUDE.md` cross-doc rows + the CONTRACT_VERSION row. **Escalate-if-safety:** none of these are §15 invariants (pure contract types) — orchestrator-writes, no human escalation. **The AG-vs-Appendix-A reconciliation (Q1) — if the implementer finds a reconciled-out AG field that IS load-bearing for the wire contract, that's a Step-9 Finding** (cross-doc), not a silent add.
- **§2.5-seam (shared-contract) model touched?** **YES** — these models are explicitly on the `ARCHITECTURE.md §2.5` seam list ("ActionRequest/ActionPlan/Approval/ActionResult (§6.2)"). The schema-snapshot test (RED #1) is **mandatory** and authored in this cycle.

## Things to flag at Step 2.5
1. **Q1 — AG-draft vs. Appendix-A reconciliation (THE load-bearing one).** The AG sec 9.1 `ActionRequest` is richer than Appendix A. My default vote: **freeze the leaner Appendix-A set** (action_request_id, project_id?, action_type, requester_type, requester_id, resource_refs[], inputs, risk_level, idempotency_key?, fencing_token?, status, preview?, created_at) — the dropped AG fields are intentional (context → envelope columns; policyContext/preconditions → 2.2/2.4; expiry → Approval). **If you judge any dropped field load-bearing for the wire/ui contract, flag it as a Step-9 Finding** before GREEN — don't silently re-add or silently drop.
2. **Q2 — new-ID treatment — RULED OPTION A (lead, 2026-06-11; logged).** `ApprovalId`(`appr_`)+`ActionPlanId`(`aplan_`) are platform-minted prefixed-ULID newtypes OUTSIDE the frozen 22-member `IdKind` — `IdKind` + `test_all_22_ids_present_with_prefixes` stay untouched. **Precedent (decisive):** the architecture ALREADY has seam-crossing prefixed-ULID newtypes outside `IdKind` — the 4 desktop objects (`dev_`/`lr_`/`eprj_`/`rc_`) live in `DesktopObjectKind` + `desktop_minted_id!` (`objects.rs:12-122`), NOT `IdKind`, because `IdKind`'s boundary is the PBI cross-product 22 specifically (not "everything crossing daemon→UI"). approval_id/plan_id are the same case → same pattern. **Implementation latitude (your call):** standalone newtypes vs. a small `GatewayObjectKind` grouping enum (sibling to `DesktopObjectKind`) + a `gateway_minted_id!` macro — **my lean: the `GatewayObjectKind` sibling**, because it mirrors `objects.rs` 1:1 and self-documents the "non-cross-product Gateway objects" category (which is exactly how the ARCH-sec-5.3 desktop tier coexists with the §5.2 cross-product set). Either way: typed newtype + prefix validation (uniform with the desktop tier), documented in §5.2/Appendix-A as "platform-minted, non-cross-product (Gateway objects)". No longer gated — proceed.
3. **Q3 — `RiskLevel` shape.** Enum `Level0..Level4` (serde as integer 0–4) vs. a validated `u8` newtype. My default vote: **a 5-variant enum** serialized as integer — closed set, type-safe, `Ord` for the risk-range comparisons (`≥1`, `==4`), reject-out-of-range for free.
4. **Q4 — `ActionPreview` per-class previews.** The 6 typed sub-previews (command/diff/api/session/workflow/rollback, AG sec 9.4) are **2.3's deliverable** (preview/dry-run). My default vote: **freeze only the `ActionPreview` envelope now** (action_request_id, generated_at, risk_level, risk_reasons[], summary, changed_resources[], cannot_preview_reason?); add the 6 typed per-class structs in 2.3. Don't freeze placeholders you'll re-shape.
5. **Q5 — under-defined sub-structs (`ActionDependency`, `RollbackPlan`, `ApprovalConstraint`).** AG references them but the core-data-model excerpt (AG sec 9) doesn't fully define them. My default vote: **read the rest of `ACTION_GATEWAY.md` for their shapes; freeze a minimal typed shape now if defined, else flag the gap** (don't invent fields). `ActionDependency` minimally is a step-ordering edge (`{step_id, depends_on_step_ids[]}` or `{from_step, to_step}`); `RollbackPlan`/`ApprovalConstraint` may be `Option`-deferred to 2.4/2.2 with a typed placeholder if AG doesn't pin them — flag the choice.
6. **Q6 — `ActionResult` / `Approval` field reconciliation.** AG sec 9.6 `ActionResult` adds started_at/finished_at/executor/output_summary/rollback_action_request_id; AG sec 9.5 `Approval` adds message/approved_at/denied_at. Appendix A is leaner (decided_by/decided_at). My default vote: **freeze the Appendix-A set**; treat the AG extras as 2.1b/2.3 execution-metadata (flag if load-bearing now). `required_approver` shape: an enum `RequiredApprover { CurrentUser, ProjectOwner, Actor(RequesterType?) }` vs. `String` — vote a small enum.
7. **Q7 — `ResourceType` count: AG sec 9.8 enumerates 20, but §6.2/Appendix-A says "21 types."** My default vote: **freeze the AG sec 9.8 20-value set verbatim + flag the 20-vs-21 discrepancy as a Step-9 cross-doc note** (either AG missed one — e.g. `terminal`/`commit`/`pr_check` — or "21" is approximate). Don't invent the 21st; surface it for reconciliation.
8. **Q8 — `Timestamp` newtype scope (Carry-forward 2.1).** Apply to the new models only, vs. also retrofit the envelope's `occurred_at`/`recorded_at` + add `seq` `minimum: 1` (the other two 2.1 carry-forwards). My default vote: **introduce `Timestamp` + apply to new models AND retrofit the envelope** — `#[serde(transparent)]` means the wire value is unchanged (still a bare RFC3339 string), so it's a schema-`format`-only, NON-breaking refinement; that's the whole point of the carry-forward. Add `seq minimum:1` in the same touch. **If the envelope retrofit risks a 3-way-verify churn you'd rather isolate, flag it** — splitting it to its own follow is acceptable.
9. **Q9 — CONTRACT_VERSION bump size — CONFIRMED 0.14.0 → 0.15.0** (lead-endorsed: minor/additive — new Gateway surface, no breaking change to frozen contracts; consistent with the per-phase additive-bump convention, events.rs accretion). Confirm no frozen type changed shape (only additions + the transparent Timestamp/seq refinement).

## Dependencies + sequencing
- **Depends on:** Phase 1 ✅ (the frozen `shared/` status machines + IDs + envelope + the §5.0 schema/3-way mechanism). Q2 ID ruling = **RULED Option A** (no longer gating — proceed).
- **Blocks:** **2.1b** (the daemon Gateway pipeline builds against these types) · the **ui track resume** (regen Zod against the bumped schema; the Gateway-modal/intent-seam consume `ActionPlan`/`ActionRequest`/`Approval`) · the **edges track** (executors bind `ActionRequest`/`ResourceRef`). This slice IS the §2.5 freeze that unblocks fan-out.
- **2.1b will add an `action_plans` metadata table + a nullable `plan_id` FK on `action_requests`** (the plan = grouping-over-action_requests model the lead agreed at Q1; single action = `plan_id` NULL, per ARCH sec 11.5 "plan 1..N" + AG sec 9.3). That table is a **DATA_MODEL sec 2.9 ADDITION** → flag at **2.1b** Step-9 as a cross-doc note (documented DDL add vs. an `/arch-finalize` reconcile is the 2.1b Step-9 routing call). Not in 2.1a's scope (2.1a is `shared/` types only).

## Estimated commit count
**2 commits (a layered slice, driven layer→layer; NOT safety-critical-code — pure contract types, so bundleable within one brief):**
- **L1 — the models + enums + new IDs + the `RequesterType` map + `Timestamp` + the schema-snapshot/wire-value tests** (RED #1–#9). One coherent contract-definition unit.
- **L2 — `ContractBundle` extension + schema-artifact regen + 3-way verify + CONTRACT_VERSION bump** (RED #10).

Bundling rationale: all in `shared/`, one logical unit (the freeze), no §15 invariant code path (these are type definitions; the INV-SEC-1 *enforcement* is 2.1b's pipeline). Split L1/L2 only because the regen/verify is a distinct mechanical step. **security-reviewer NOT required for 2.1a** (no mutation path, no invariant code — pure contract); it runs on **every 2.1b slice** (the pipeline IS the INV-SEC-1 chokepoint). _(Orchestrator note: confirm this with the reviewer policy at Step 8 — `security-reviewer: invariant`; 2.1a touches no invariant code, so it's `code-quality-reviewer` only.)_

## Lessons-logged candidates anticipated
- **Convention candidate** — "the binding contract is `ARCHITECTURE.md` Appendix A + the DATA_MODEL DDL; the `docs/domains/*.md` drafts are origin/rationale and may be richer — freeze the reconciled set, flag drops as Findings." (Likely recurs at every domain-model freeze: the harness-types freeze, the Brain-mapping freeze.)
- **Architecture-doc note candidate** — the Appendix-A ActionRequest/Approval/etc. rows reconciled to the as-frozen field sets; the 20-vs-21 ResourceType count; the two non-cross-product platform IDs note in §5.2.
- **Future TODO — operational** — the §6.3 `ActionType` catalog enum (the ~21 action-type names + per-type contracts) freezes in **2.2** (action_type is `String` here); the `ActionPreview` per-class structs freeze in **2.3**.

## How to invoke
1. **Read this brief end-to-end** — especially Q1 (AG-vs-Appendix-A reconciliation, the load-bearing one). Q2 (ID treatment) is **already RULED Option A** — follow the `objects.rs` precedent; nothing gated.
2. **Run `/tdd action_contract_freeze`.**
4. **Step 0 (Restate)** — confirm the restatement matches the Feature line (contract freeze only; no daemon behavior).
5. **Step 2.5** — send the test-design write-up + answers to Q1–Q9 (take defaults or push back). Wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
6. **Step 9** — surface the cross-doc reconciles (Appendix-A rows, the 20-vs-21 ResourceType, the non-cross-product ID note) + any AG-reconciliation Finding.

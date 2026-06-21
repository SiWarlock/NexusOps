# /tdd brief — gateway_modal_actionplan_render

## Feature
Extend the Action Gateway approval surface to render a bundled **`ActionPlan` (1..N steps)** as per-step rows + plan-level approval controls (approve-all-eligible, approve-step, deny-with-reason), driven by the **FROZEN §6.2 `ActionPlan`/`ActionPlanStep` contract** and exposed-ahead of a live plan-submitter. The single-action `GatewayModal` is the N=1 degenerate case and stays unchanged. **Zero Brain dependency** — this is the general §11.5 multi-step Gateway Review Modal; the Brain "Run via Gateway" submit path is a SEPARATE later slice (A), built guarded-disabled.

## Use case + traceability
- **Task ID:** P8.2 (the `gateway-modal/ extended for ActionPlan` half of 8.2; Phase 8 un-deferred 2026-06-21, user-approved)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.5` (Action Gateway & Brain UI — the Gateway Review Modal accepts an ActionPlan 1..N + the Screen-16 controls), `§6.2` (the frozen `ActionPlan`/`ActionPlanStep`/`Approval` models)
- **Related context:**
  - `ui/src/overlays/GatewayModal.tsx` — the existing single-action modal (the N=1 case; **its 10 cat-1 pins must stay green** — LESSON [[17]]). `ResultNotice` is already exported; `safety/model.ts describeRejection` routes §6.4 codes.
  - Frozen source: `shared/src/actions.rs:357` (`ActionPlan{plan_id,title,steps,dependencies,overall_risk,approval_mode}`), `:342` (`ActionPlanStep{step_id,label,action_request,required,can_skip,rollback_action_type?,status}`), `:333` (`ActionDependency{step_id,depends_on_step_ids}`), `:371` (`Approval{approval_id,action_request_id?,plan_id?,status,…}`). Generated TS in `contracts/generated.ts`; `ApprovalMode` = `approve_all|step_by_step|mixed|blocked`.
  - Daemon: `submit_action_plan`/`PlanAck` + the `proj_approval_queue` plan-grouping landed @2.1c (`c681e54`/`b9e00a1`/`bb9b4b0`); approve-cascade excludes critical/4 **daemon-side** (catalog-authoritative, 2.1c L3). **Per-step `policy_decision` is NULL today** (daemon 8.1 follow-on) → render honest-absent.
  - LESSONS [[16]] (pure submitter), [[17]] (pure renderer — never UI-derived risk/preview; daemon-status-never-done), [[33]] (trim audited reason), [[11]] (distinct fail-closed surfaces), forbidden #2/#6.

## Acceptance criteria (what "done" means)
- [ ] `PlanModal` renders an `ActionPlan`'s **N steps as N rows**, each showing: step index, `label`, `action_request.action_type`, target (`action_request.resource_refs` ids), step `status`, and `required`/`can_skip` flags.
- [ ] Plan header renders `title`, **`overall_risk` (the daemon's plan-level value, never UI-derived)**, `approval_mode`, `plan_id`, and a plan-level `Approval` status pill.
- [ ] **Per-step risk is the daemon-provided value, NEVER UI-derived** — a step whose daemon risk is absent renders an honest "—"/pending, never a fabricated number (LESSON [[17]] / forbidden #2). Pin: a fixture step with no daemon risk → no invented digit.
- [ ] **Per-step policy/preview honest-absent when NULL** (per-step `policy_decision` is daemon-NULL today) — render "pending"/absent, never a synthesized consequence (forbidden #2).
- [ ] **Approve all eligible** control present → submits the **plan-level** approve via the seam (`approve(approval_id)` no step_id). The **UI never computes which steps are critical/excluded** — eligibility is daemon-authoritative (2.1c L3). Pin: the component derives no per-step risk-class itself.
- [ ] **Approve step** (per row) → submits `approve(approval_id, step_id)`.
- [ ] **Deny** → plan-level `deny(approval_id, reason)` with the reason **trimmed** (whitespace-only → explicit default; LESSON [[33]]).
- [ ] **Save as policy (`policy_grant`)** checkbox present but **DISABLED** (own cat-1; mirror the existing always-allow disabled control).
- [ ] All mutation controls gated by **`canSubmitIntent && port.mutationsEnabled`** (forbidden #6) — **the SAME gates as single-action; NO new go-live flag, NO new capability** (the approve/deny seam is the already-live, user-signed-off L2-C path; `approve(approval_id, step_id?)` already supports the step_id arg).
- [ ] **No-optimistic-done:** post-submit shows the daemon-reported `ActionAck.status` verbatim, never "executed"/"done" (LESSON [[17]] / Q3); reuse `ResultNotice`.
- [ ] **§6.4 reject codes routed verbatim** to their distinct §11.5 cards (`fencing_conflict` never re-approvable #6) — reuse `describeRejection`.
- [ ] **GUARDRAIL (MANDATORY — lead-ruled 2026-06-21): the N-step extension does NOT regress the live single-step L2-C approve path.** The existing `GatewayModal` approve/deny behavior stays **byte-identical** — the N-step render is **PURELY ADDITIVE**. Pin explicitly: an existing single-action approve still calls `approve(approval_id)` (no step_id) and renders identically; the 10 cat-1 modal pins stay green; `security-reviewer` confirms **no behavior change to the signed-off L2-C capability**. An N=1 plan renders as one row without special-casing.
- [ ] Production entry point reachable (see Wiring); all unit tests in `ui/src/overlays/PlanModal.test.tsx` pass; `/preflight` clean.
- [ ] **security-reviewer pass** (MANDATORY — invariant-touching: the INV-SEC-1 human-approval card on a multi-step path).

## Wiring / entry point (Step 7.5)
**`ui/src/shell/Shell.tsx` gateway overlay** (`overlay?.kind === "gateway"`, currently line ~768) **branches on the selected approval's `plan_id`:** a plan-bearing approval → `PlanModal` (rendered from the assembled `ActionPlan`); a single-action approval (`plan_id` null) → the existing `GatewayModal`. Reachable-by-construction the moment a plan-bearing approval is selected from the Human Input Queue.

**Honest data-availability note (exposed-ahead):** there is **no live plan-submitter in production yet** (the Brain is 8.1-gated; nothing else submits plans), so no `ActionPlan` reaches the queue today. B renders against the **frozen `ActionPlan` model from a Mock/dev fixture** (the intent-seam-shadow precedent); the **live plan-data assembly** (grouping `proj_approval_queue` rows by `plan_id` into the model, or a future `get_action_plan` RPC if the daemon serves the full model) is a **deferred follow-on — wiring of the live feed lands in the plan-feed slice** when a plan-submitter exists. The Shell branch + the component are wired now; the production feed is the documented gap (PR-workspace-placeholder pattern).

## Files expected to touch
**New:**
- `ui/src/overlays/PlanModal.tsx` — the N-step plan approval modal (composes the shared single-action primitives).
- `ui/src/overlays/PlanModal.test.tsx` — the RED pins.

**Modified:**
- `ui/src/shell/Shell.tsx` — the gateway overlay branches on `plan_id` → `PlanModal` vs `GatewayModal` (+ an `enrichPlan`/assembly helper if needed).
- `ui/src/shell/display-meta.ts` — a **daemon-shaped Mock `ActionPlan` sample** for tests/dev (mirror the existing `GatewayModal {Approval,PolicyDecision}` SAMPLE at ~line 145).
- Possibly `ui/src/overlays/GatewayModal.tsx` — **only** to EXPORT (not change) any shared render helper `PlanModal` reuses (`ResultNotice` is already exported; keep single-action behavior byte-identical).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2) — `ui/src/overlays/PlanModal.test.tsx`
1. **`renders_n_steps_as_rows`** — a 3-step plan → 3 step rows with label/action_type/target. Asserts: row count == steps.length; each row shows its `action_request.action_type`. Why: §11.5 per-step rows.
2. **`header_renders_daemon_overall_risk_never_derived`** — Asserts: header shows `overall_risk` from the plan; a plan with a different per-step mix does NOT recompute it. Why: §11.5 / LESSON [[17]] (daemon-authoritative risk).
3. **`per_step_risk_absent_renders_honest_dash`** — a step with no daemon risk → "—"/pending, no fabricated digit. Why: forbidden #2 / LESSON [[17]].
4. **`per_step_policy_absent_is_honest_not_synthesized`** — per-step policy NULL → "pending"/absent surface, not invented consequences. Why: forbidden #2.
5. **`approve_all_submits_plan_level_no_step_id`** — clicking Approve-all → `approve(approval_id)` with NO step_id; the component computes no eligibility. Why: 2.1c daemon-authoritative critical-exclusion.
6. **`approve_step_submits_step_id`** — per-row approve → `approve(approval_id, step_id)`. Why: §6.1 `approve(approval_id, step_id?)`.
7. **`deny_trims_reason`** — whitespace-only reason → the explicit default is sent, never blank. Why: LESSON [[33]].
8. **`policy_grant_checkbox_disabled`** — the save-as-policy control is present + disabled. Why: own cat-1 (mirror always-allow).
9. **`controls_disabled_when_not_submittable`** — `canSubmitIntent` false OR `mutationsEnabled` false → all approve/deny controls disabled (no enabled-button-that-throws). Why: forbidden #6 / honest-degrade (same gate as single-action; no new go-live).
10. **`post_submit_shows_daemon_status_never_done`** — after approve, render the daemon `ActionAck.status` verbatim, never "executed". Why: LESSON [[17]] / Q3.
11. **`reject_code_routed_verbatim`** — a `fencing_conflict` reject → the hard-conflict card with NO re-approve affordance (#6); a `precondition_stale` → re-approvable. Why: §6.4 verbatim / forbidden #6 (reuse `describeRejection`).
12. **`n_equals_1_plan_renders_one_row`** — a 1-step plan renders cleanly (no special-case break). Why: single-action = the N=1 degenerate.
13. **`single_action_l2c_approve_path_unchanged` (GUARDRAIL — MANDATORY, lead-ruled)** — after the `PlanModal` addition, the existing `GatewayModal` single-action approve STILL submits `approve(approval_id)` with NO step_id and renders byte-identically; the existing 10 cat-1 modal pins stay green (cite + assert no diff). Why: lead-ruled **no-regression to the signed-off live L2-C capability** — the N-step render is additive, never a behavior change to the existing approve path. `security-reviewer` verifies this explicitly.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** — consumes the FROZEN `ActionPlan`/`ActionPlanStep`/`Approval` (§6.2, frozen @2.1a/0.15). **No contract bump, no regen** expected.
- **Cross-track seam (shared-contract) model touched?** The slice **consumes** seam models but does not change them. If the implementer adds a **provisional Zod shadow** for `ActionPlan`/`ActionPlanStep` (if not already generated), it is **field-set drift-pinned to `shared/src/actions.rs`** (the [[2]]/[[24]] frozen-shadow discipline) — author that drift-pin test in this cycle. Verify first whether `generated.ts` already exposes these (object `$defs`) before shadowing.
- **Orchestrator doc rows to write hot (Step 9):** likely an update to the **`GatewayPort mutation surface + intent seam` cross-doc row** in `ui/CLAUDE.md` (extend it with the PlanModal N-step render) + a new LESSON candidate (see below). Implementer FLAGS; orchestrator writes hot.

## Things to flag at Step 2.5
1. **Plan data source for the render.** (a) Render against the frozen `ActionPlan` model from a Mock/dev fixture (exposed-ahead; live feed deferred) — OR (b) assemble the plan now from `proj_approval_queue` rows grouped by `plan_id`. **My default vote: (a)** — there is no live plan-submitter, and `proj_approval_queue` (per-approval rows) does NOT carry the full model's `title`/`approval_mode`/`dependencies`/per-step `label`. Verify whether the daemon serves the `ActionPlan` model via any RPC; if not, (a) + a documented live-feed follow-on is the honest scope. Confirm at design time.
2. **Compose vs extend.** A **sibling `PlanModal`** reusing GatewayModal's exported primitives (`ResultNotice`, `describeRejection`, the preview/policy render helpers) — OR widen `GatewayModal` to take a plan. **My default vote: sibling `PlanModal` + shared helpers** — keeps the single-action 10 cat-1 pins byte-identical (extending the 343-line cat-1 modal risks regressing them); extract a shared helper only where it's a pure lift.
3. **Per-step risk source.** `overall_risk` (plan, daemon) for the header + per-step from the step's `action_request.risk_level` (daemon-set) **labeled as the daemon's value** — OR honest-absent until a per-step `preview_action`. **My default vote: render the daemon-provided per-step value (never UI-derived, LESSON [[17]]); defer per-step live preview** (N preview calls) to a follow-on — render honest-absent where the daemon value isn't present.
4. **Screen-16 control scope in B.** Core set (approve-all / approve-step / deny / disabled policy_grant) — OR also edit-before-approve / remove-step / require-manual-execution. **My default vote: CORE in B**; edit/remove/require-manual-execution = an explicit B2 follow-on (keeps B a clean slice; those add re-preview + per-step state machinery).
5. **Trust-boundary scoping of plan-approve — ✅ CONFIRMED (lead-ruled 2026-06-21): ride L2-C, NO new gate.** The plan approve/deny submit reuses the existing live, user-signed-off L2-C seam (`approve(approval_id, step_id?)`) with the SAME `canSubmitIntent && mutationsEnabled` gates — **NO new gate, NO empty-gate.** **Rationale (bake into the slice framing):** the approve/deny path **IS the human control that ENFORCES INV-SEC-1 #10** ("Brain proposes, never executes") — it is NOT a mutation to gate; the capability that stays gated is the **SUBMITTER** (Brain→Gateway "Run via Gateway", guarded-disabled in slice A). An empty-gate here would gate the wrong thing, add no safety (no plan executes without the approval anyway), and risk **regressing** the live single-step approve. → see the **MANDATORY GUARDRAIL** (acceptance bullet + RED test #13): the N-step extension must be proven additive — zero behavior change to the existing live single-step L2-C approve path, verified by tests + `security-reviewer`.

## Dependencies + sequencing
- **Depends on:** the intent seam (043/044, landed) + L2-C go-live (landed) + the frozen §6.2 `ActionPlan` (2.1a, landed) + daemon `submit_action_plan`/proj_approval_queue plan-grouping (2.1c, landed).
- **Blocks:** **A** (the Brain drawer "Run via Gateway" submits `plan.steps` → this modal renders the resulting plan-approval); the per-step `policy_decision` daemon follow-on (8.1) consumes this card; the live plan-data-feed follow-on.

## Estimated commit count
**1–2.** One focused component (`PlanModal` + its pins) + the Shell branch wiring. It's a **cat-1 / invariant-touching** surface → it gets its **OWN commit** (no bundling); if the Shell-branch wiring is sizable it may be a 2nd layer commit (the orchestrator drives layer→layer — LESSON [[7]]). **security-reviewer runs before the commit.**

## Lessons-logged candidates anticipated
- **Convention candidate** — "The N-step plan-approval modal is a PURE renderer of the daemon's `ActionPlan`/per-step values (never UI-derived per-step risk/eligibility — the daemon's critical-exclusion is catalog-authoritative), reusing the single-action seam + reject-routing; the plan-approve submit rides the existing L2-C gate, no new go-live." (extends LESSON [[17]])
- **Architecture-doc note candidate** — §11.5 as-built: the plan modal renders honest-absent per-step `policy_decision` (daemon-NULL today) pending the 8.1 daemon follow-on.
- **Future TODO — phase** — the live plan-data feed (assemble from `proj_approval_queue` by `plan_id`, or a `get_action_plan` RPC) → a real task line under 8.1/8.2 when a plan-submitter lands.

## How to invoke
1. Read this brief end-to-end — especially the 5 Step-2.5 questions (the plan-data-source + the trust-boundary scoping need answers before GREEN).
2. Run `/tdd gateway_modal_actionplan_render`.
3. Step 0 (Restate) — confirm: extend the Gateway approval surface to an N-step `ActionPlan` render, frozen-contract, exposed-ahead, single-action path unchanged.
4. Step 1 (Identify files) — confirm against Files expected to touch.
5. Step 2.5 — ping back with answers (or take defaults). **security-reviewer is MANDATORY at Step 7→8** (invariant-touching).
6. Step 9 — surface anything beyond the anticipated lessons-logged candidates.

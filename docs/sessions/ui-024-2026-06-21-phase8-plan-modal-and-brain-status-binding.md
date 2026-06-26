# ui-024 — Phase-8 UI arc: N-step PlanModal (B) + Brain status header binding (A1)

- **Date:** 2026-06-21
- **Phase:** 8 (Project Brain seam & drawer — un-deferred 2026-06-21, user-approved) · track `ui`
- **Predecessor:** [ui-023](ui-023-2026-06-21-real-head-sha-pin.md)
- **Successor:** [ui-025](ui-025-2026-06-25-pr-mutations-cat1-go-live.md)
- **Commits:** `15fad1f` (ui-073, B) · `9a7f2ef` (ui-074, A1) — both LOCAL on `track/ui`, push HELD.

## Why this session existed

Phase 8 moved DEFERRED → active (the user is building the `brain/` sibling in parallel). Two
buildable-now, NON-gated shell-binding slices were dispatched while the daemon's auth-bootstrap /
live-validation work proceeds elsewhere. The PR-mutations go-live flip stays HELD (that arc is
already built guarded-disabled). This session shipped the first two Phase-8-UI slices:

- **B (ui-073)** — the general §11.5 multi-step Gateway Review Modal (an N-step `ActionPlan` render),
  exposed-ahead of a live plan-submitter. cat-1 / INV-SEC-1 approval card.
- **A1 (ui-074)** — the Brain drawer/page header bound to a `ProjectBrain` §5.1 status via a FakeBrain
  seam + honest-degraded §13.1 states. NON-cat-1 (read/display + a UI provider).

## What was built

### ui-073 (B) — N-step ActionPlan Gateway modal

**Files created:**
- `ui/src/overlays/PlanModal.tsx` — the N-step plan approval modal: per-step rows + plan-level
  controls (approve-all-eligible / approve-step / deny / disabled policy_grant). A PURE renderer of
  the frozen §6.2 `ActionPlan` (header `overall_risk` = daemon aggregate; per-step risk = daemon's
  `action_request.risk_level`, honest-absent "—"; per-step policy honest-absent). Reuses
  `ResultNotice` + `describeRejection` (no-optimistic-done, §6.4 verbatim). Per-step controls lock
  after a plan-level result (no 2nd mutation from a post-decision surface).
- `ui/src/overlays/GatewayOverlay.tsx` — the Shell gateway-overlay dispatcher: branches on
  `approval.plan_id` → `PlanModal` vs the unchanged single-action `GatewayModal`.
- `ui/src/overlays/PlanModal.test.tsx` (14 pins) · `ui/src/overlays/GatewayOverlay.test.tsx` (2 pins).

**Files modified:**
- `ui/src/intent/submit-intent.ts` — `IntentSeam.approve` threads an OPTIONAL `step_id` (the §6.1
  `approve(approval_id, step_id?)` wire); additive/backward-compat — single-action stays a length-1
  `port.approve(id)` call (conditional, so existing pins stay green).
- `ui/src/intent/submit-intent.test.ts` — the step_id pin + the 3 new shadows in the §2.5-seam
  drift-pin + a `.strict()`/approval_mode-delegation pin.
- `ui/src/contracts/intent-contracts.ts` — 3 NEW provisional shadows `ActionPlan` / `ActionPlanStep`
  / `ActionDependency` (`.strict()`, enum fields delegated), field-set drift-pinned to
  `shared/src/actions.rs`.
- `ui/src/shell/display-meta.ts` — `samplePlan` + `enrichPlan` (the daemon-shaped exposed-ahead fixture).
- `ui/src/shell/Shell.tsx` — the gateway overlay renders `<GatewayOverlay>`.

### ui-074 (A1) — Brain status header binding

**Files created:**
- `ui/src/views/brain/brain-status.ts` — the FakeBrain model + provider: `BrainStatusValue`
  `{status, grounded_at, absent}`, `deriveBrainHeader(value, _now?)` (absent→not_configured;
  descriptor-bound label/visualKind; §13.1 degraded flag), `BRAIN_DEGRADED_BY_STATUS` (a
  completeness `Record` over the 10 frozen states, drift-pinned to `ProjectBrain.options`),
  `BrainStatusProvider`/`useBrainStatus` (LESSON 13 pattern, mirrors `active-project.ts`).
- `ui/src/views/brain/BrainStatusHeader.tsx` — the shared status surface (descriptor-bound
  `StatusPill` + grounded-at + a DISTINCT additive `brain-degraded` surface when degraded).
- `ui/src/views/brain/brain-status.test.ts` (4) · `ui/src/views/brain/BrainStatusHeader.test.tsx` (5).

**Files modified:**
- `ui/src/views/brain/BrainPage.tsx` — replaced the static "display fixture" Badge with
  `<BrainStatusHeader>`; dropped the now-unused `Badge` import.
- `ui/src/shell/Shell.tsx` — Shell-root `<BrainStatusProvider value={fakeBrainStatus}>`.

## Decisions made

- **B plan-data-source (Q1) = (a) Mock/dev fixture** vs the frozen `ActionPlan` (no live plan-submitter
  exists; `proj_approval_queue` lacks the full model) → `samplePlan`/`enrichPlan`; the live plan-data
  feed is a deferred follow-on (PR-workspace-placeholder precedent).
- **B trust-boundary (Q5, lead-ruled) = ride L2-C, NO new gate.** The approve/deny path IS the human
  control that ENFORCES INV-SEC-1 #10 (Brain proposes, never executes) — not a mutation to gate; the
  gated capability is the *submitter* (Brain "Run via Gateway", guarded-disabled). The seam `step_id`
  thread is purely additive (the mandatory #13 no-regression guardrail proves the single-action path
  is byte-identical).
- **B per-step risk (Q3) = render the daemon's `action_request.risk_level`** labeled as the daemon's
  per-step value (a daemon-formed plan = catalog-authoritative, distinct from LESSON 19's UI-formed
  per-hunk hint); per-step live `preview_action` deferred.
- **A1 deriveBrainHeader signature = `(BrainStatusValue, _now?)`** — folds `absent`→not_configured into
  one resolution; `_now` reserved-unused (DISPLAY-only, NEVER derives `stale` — forbidden #4).
- **A1 `transcript_ingestion_off` = NON-degraded** (the 10th enum value the brief's test #2 didn't
  list) — a benign config state (descriptor rank-0/idle), not a fault. The completeness drift-pin
  forces all 10 to classify.
- **A1 BrainDrawer.tsx NOT modified** (orchestrator-approved) — the drawer hosts `<BrainPage drawer/>`,
  whose header renders `BrainStatusHeader`, so the status shows in the drawer without a 2nd binding
  (avoids a duplicated degraded surface + keeps every production change test-covered).

## Decisions explicitly NOT made (deferred)

- The live daemon plan-data feed (group `proj_approval_queue` by `plan_id`, or a `get_action_plan`
  RPC) — wired the day a plan-submitter exists. (8.1/8.2)
- Per-step live `preview_action` enrichment (N calls); Screen-16 extended controls
  (edit/remove-step/require-manual-execution = B2); per-step `rollback_action_type` render (CORE-scope
  deferred); N=0 empty-plan empty-state.
- The live daemon `ProjectBrain` status source (a projection / project-row field) that replaces the
  FakeBrain provider (the single swap-point); the rich Brain content panes (thread/evidence/plan) +
  EvidenceChip real-freshness; relative-time grounded-at display (the reserved `_now`).

## TDD compliance

- **ui-073: clean.** All 16 pins written RED-first, Step-2.5 reviewed (APPROVED after the #13 guardrail
  ADD). The code-quality [high] post-result-lock fix was TDD'd (RED test `per_step_controls_locked_after_plan_result`
  confirmed failing before the `locked` prop landed). security-reviewer CLEAR (8 invariants, 0 findings).
- **ui-074: clean, one minor note.** All 8 brief pins written RED-first, Step-2.5 reviewed (APPROVED).
  Minor: the `grounded_at: null` → "grounded —" render branch existed in the header impl from GREEN but
  its dedicated assertion (`grounded_at_null_renders_honest_dash`) was added at Step 8 after the
  code-quality reviewer flagged the coverage gap (the broader `content_still_renders` test exercised
  the null value but didn't pin the dash). Non-safety; coverage closed in-slice. code-quality-reviewer
  (NON-cat-1, no security review per policy).

## Cross-doc invariant audit

**Clean — no frozen-contract model field add/remove/rename this session.** Both slices CONSUME frozen
contracts: ui-073 consumes the frozen §6.2 `ActionPlan`/`ActionPlanStep`/`ActionDependency` (3 NEW
UI-local provisional shadows, drift-pinned to `shared/src/actions.rs` — a consumer addition, not a
contract change; NO bump/regen); ui-074 consumes the frozen `ProjectBrain` §5.1 enum + descriptor (the
FakeBrain value shape is a UI-local provisional). Multi-track memory check: every doc touch was flagged
at Step 9 and the orchestrator confirmed (it is writing `ui/CLAUDE.md` + `ARCHITECTURE.md` notes +
`ui/LESSONS.md` hot — visible uncommitted in the working tree, riding its `/orchestrate-end`).

## Reachability

- **ui-073 PlanModal** — reachable from `Shell` gateway overlay via `Shell → GatewayOverlay → PlanModal`
  (branch on `overlay.approval.plan_id`), pinned by the `GatewayOverlay` dispatcher test. Exposed-ahead:
  no live plan-submitter in production yet (Brain 8.1-gated) → the branch + modal are wired now; the
  production data feed is the documented deferred follow-on (NOT a wiring gap).
- **ui-074 BrainStatusHeader** — reachable from the Shell-root `BrainStatusProvider` via (1) the
  content-view `contentView==="brain"` → `<BrainPage/>` → `BrainStatusHeader`, and (2) the TopBar Brain
  trigger → `BrainDrawer` → `<BrainPage drawer/>` → `BrainStatusHeader`.
- No tested-but-unwired gaps.

## Open follow-ups (Step-9 categorized — orchestrator routes hot)

- **Convention candidates (LESSON):** (B) the N-step plan-approval modal is a PURE renderer of the
  daemon's ActionPlan/per-step values (never UI-derived per-step risk/eligibility), riding the existing
  L2-C gate (no new go-live); a post-decision surface is read-only — plus the daemon-formed-vs-UI-formed
  risk distinction (extends LESSON 17/19). (A1) a Brain shell-binding is a FakeBrain provider (LESSON 13)
  exposing the frozen ProjectBrain §5.1 status as the single swap-point; honest-degraded §13.1 is a
  DISTINCT additive surface that never blocks; the degraded set is a completeness-drift-pinned Record
  (LESSON 11).
- **Architecture-doc notes:** §11.5 as-built — PlanModal renders honest-absent per-step `policy_decision`
  (daemon-NULL today) pending the 8.1 follow-on; §11.5/§13.1 as-built — the Brain header binds a
  FakeBrain seam pending the daemon 8.1 ProjectBrain status source.
- **Cross-doc-doc-rows (orchestrator territory):** extend the `GatewayPort mutation surface + intent
  seam + consumer` row (PlanModal N-step render + the 3 ActionPlan shadows + the seam `step_id` thread);
  add a Brain status-header binding / FakeBrain seam note; retire the stale `plan:707` "Brain TopBar
  inert" note.
- **Future TODO (8.1/8.2):** live plan-data feed · per-step preview · Screen-16 extended controls ·
  rollback_action_type render · N=0 empty-state · live daemon ProjectBrain status source · rich Brain
  content panes + EvidenceChip real-freshness · relative-time grounded-at display.
- **Visual gate (LESSON 10/12):** both PlanModal + BrainStatusHeader are NEW rendered surfaces — green ≠
  looks right. BrainStatusHeader's FakeBrain default feeds it live in the dev shell (a visual gate IS
  feasible by flipping `fakeBrainStatus`); PlanModal has no live data (exposed-ahead, manual operator
  step). FLAGGED for lead/visual sign-off.

# /tdd brief — usage_dashboard

## Feature
The **Usage dashboard** (§11.4/§11.7/§9.1): per-session token/cost usage with **accuracy labels** (exact/estimated/unavailable, shown in **every** variant — §11.7), **Codex context-% rendered as `"unknown"`** (never a number/0% when `supportsContextMetadata=false` — **forbidden #4**, load-bearing), and an **Agent-SDK credit-pool meter** (normal / near-exhaustion / hard-stop, distinct from token spend) with the threshold on a **non-color channel** (§11.6 / never-color-alone). Reads a fixture-backed **provisional** Usage projection. Mounts as an interim content-view; relocates to a Settings tab (§11.2) at 6.4c. Second daemon-independent 6.4 slice (Decision-C reorder).

## Use case + traceability
- **Task ID:** P6.4b (6.4 decomposition: 6.4a a11y ✅ → **6.4b Usage** → 6.4c Settings display → 6.4d Survival/recovery display).
- **Architecture sections:** `ARCHITECTURE.md §11.4` (Usage surfaces; credit-pool meter near-exhaustion+hard-stop; Codex context=unknown + `metric_quality` on telemetry), `§11.7` (UsageMeter renders the accuracy label in **all** variants + shows "unknown" not 0%/empty when unavailable/NULL), `§9.1` (`supportsContextMetadata=false` for Codex; `metric_quality`), `§4.2` (renders from projections). Forbidden **#4** (Codex %=unknown) + **#5** (never color alone).
- **Related context:** the provisional-shape pattern (Lesson §2 — usage shapes are NOT in the frozen contract; hand-author banner-marked provisional types, enum fields delegated where frozen); the `StatusPill`/descriptor pattern + the kit `UsageMeter` (§11.1 inventory — confirm its props at Step 1; if closed-props, wrap per Lesson §6); the view-switch seam (6.3b) + the a11y foundation (Lesson §9 — focus ring + reachability apply to any new controls). Deterministic core = the usage view-model (accuracy label / Codex-unknown / credit-pool state — pure); the dashboard render is render-tested.

## Acceptance criteria
- [ ] A **provisional** `UsageRow` shape (`ui/src/contracts/provisional.ts`, banner-marked, Lesson §2) + a usage fixture (`ui/src/projections/fixtures/proj_usage.ts`): per-subject `{subject_id, harness, tokens, cost, metric_quality, context_pct }` + credit-pool data; **`metric_quality`** is a provisional enum (`exact|estimated|unavailable`, hand-declared with banner — not in the frozen contract; reconcile at freeze).
- [ ] A pure usage view-model (`ui/src/views/usage/model.ts`): maps rows → VMs applying **(a)** the accuracy label from `metric_quality`, **(b)** the **Codex-context rule** — `context_pct` → the literal `"unknown"` when the harness is Codex / `supportsContextMetadata=false` / null (NEVER a number or `0%`), a percentage otherwise, and **(c)** the **credit-pool state** (`normal | near_exhaustion | hard_stop`) from used/limit thresholds.
- [ ] `<UsageDashboard/>` renders the usage rows with the **accuracy label present in every variant** (§11.7 — the label is never dropped), Codex context as `"unknown"`, `"unknown"` (not `0%`/empty) for null/unavailable usage, and a **credit-pool meter** whose threshold shows on a **non-color channel** (glyph/label/marker — §11.6/never-color-alone). Reads only through the gateway boundary; new controls inherit the Lesson §9 focus ring + are keyboard-reachable.
- [ ] Mounts as a **4th content-view** ("Usage") via the existing view-switch (interim — relocates to a Settings tab at 6.4c; tracked). `/preflight` clean.
- [ ] **Reachable from** `Shell → view-switch (Usage) → UsageDashboard → gateway-client`.

## Wiring / entry point (Step 7.5)
`Shell` view-switch → `<UsageDashboard/>` → the usage view-model over the gateway-boundary Usage projection (fixture). Extends the view-switch seam. Confirm at Step 7.5 the Usage option mounts + reads through the boundary; note the Settings-tab relocation as a 6.4c follow-up (not a false wire — it IS mounted now).

## Files expected to touch
**New:** `ui/src/views/usage/model.ts`, `ui/src/views/usage/UsageDashboard.tsx`, `ui/src/views/usage/{model.test.ts, UsageDashboard.test.tsx}`, `ui/src/projections/fixtures/proj_usage.ts`.
**Modified:** `ui/src/contracts/provisional.ts` (provisional `UsageRow` + `metric_quality` enum + the `Usage` page in the registry), `ui/src/gateway-client/boundary.ts` (+ the Usage page schema, if get_projection is to serve it), `ui/src/shell/Shell.tsx` (4th view-switch option + fetch the Usage projection + mount), `ui/src/shell/Shell.test.tsx`. Flag anything beyond at Step 2.5.

## RED test outline (Step 2)
**`views/usage/model.test.ts`:**
1. **`codex_context_is_unknown_not_a_number`** — a Codex row (`supportsContextMetadata=false`/null `context_pct`) → the literal `"unknown"`, never a number/`0%`. **[load-bearing — forbidden #4]**
2. **`claude_context_renders_percentage`** — a Claude row with a real `context_pct` → the percentage (the unknown rule is Codex/null-scoped, not blanket).
3. **`accuracy_label_from_metric_quality`** — `exact|estimated|unavailable` → the right label; an `unavailable`/null usage → `"unknown"` (not `0`/empty). **[§11.7]**
4. **`credit_pool_state_from_thresholds`** — used/limit → `normal` / `near_exhaustion` / `hard_stop` at the thresholds; pure.

**`views/usage/UsageDashboard.test.tsx` (jsdom):**
5. **`renders_rows_with_accuracy_label_in_all_variants`** — each usage row shows its accuracy label (the label is present, not dropped — §11.7). **[load-bearing]**
6. **`codex_row_shows_unknown_context`** — a Codex row renders `"unknown"` for context (forbidden #4). **[load-bearing]**
7. **`credit_pool_threshold_on_non_color_channel`** — the credit-pool meter exposes its state via a glyph/label/marker (a non-color channel), not color alone (§11.6). **[load-bearing — forbidden #5]**
8. **`renders_only_projection_rows`** — rendered set === fixture set (no invented usage — forbidden #2); empty → explicit empty state.

**`shell/Shell.test.tsx` (extend):**
9. **`view_switch_mounts_usage_dashboard`** — selecting **Usage** mounts `<UsageDashboard/>` reachable from the Shell; switch-not-stack. **[wiring — Step 7.5]**

## Cross-doc invariant impact
- **Model field changes:** **none frozen.** `UsageRow` + `metric_quality` are **provisional UI-local** (Lesson §2, banner-marked) — fold into the existing Carry-forward **provisional→generated reconcile** when the daemon freezes the Usage/telemetry schema. **Orchestrator rows:** none. _(I will add `UsageRow`/`metric_quality` to the provisional-reconcile spread at Step-9 routing.)_

## Things to flag at Step 2.5
1. **Mount location.** Default vote: interim **4th content-view** (view-switch) now; relocate to a Settings tab (§11.2) at 6.4c (tracked). Confirm vs deferring the mount to 6.4c (would leave the component tested-but-unmounted — avoid).
2. **Usage granularity.** Default vote: per-session/subject usage rows + a per-Claude-profile (or global) credit-pool meter; fixture provides both. Confirm the shape.
3. **Credit-pool thresholds.** Default vote: provisional `near_exhaustion ≤ ~15%`, `hard_stop` at exhausted/0 — **flag if a §-anchor pins exact thresholds** (else provisional, reconcile when the daemon defines them). Confirm.
4. **`metric_quality` provisional enum.** Default vote: hand-declared provisional (`exact|estimated|unavailable`, banner, §2); reconcile at freeze. Confirm.

## Dependencies + sequencing
- **Depends on:** the provisional-shape pattern (§2) + the view-switch seam (6.3b) + the a11y foundation (6.4a, Lesson §9). No daemon dependency (Decision-C daemon-independent).
- **Blocks:** 6.4c Settings (the Usage dashboard relocates into a Settings tab); the §25-demo usage visibility.
- **Carry-forward consumed:** none net-new; **adds** `UsageRow`/`metric_quality` to the provisional→generated reconcile spread (Step-9).

## Estimated commit count
**1** — a cohesive Usage-dashboard slice (provisional shape + fixture + pure view-model + the view + the view-switch mount). Reuses §2/§9 + the seam; no safety invariant (read/render only) → **security-reviewer NOT required**; **code-quality every-slice**. (If the provisional-shape + fixture + view feel separable, splitting to 2 is fine — flag at Step 2.5; default 1.)

## Lessons-logged candidates anticipated
- **Convention candidate** — the Codex-context-`"unknown"` rule + the accuracy-label-in-all-variants are render-policy single-sourced in the usage view-model (views never fabricate a % / drop the label). Candidate if it recurs.
- **Future TODO — provisional reconcile** — `UsageRow` + `metric_quality` → generated when the daemon freezes the telemetry schema (added to the spread).
- **Architecture-doc note candidate** — credit-pool exact thresholds if §11.4 doesn't pin them (may surface a §-note).

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd usage_dashboard`.
1. Read this brief; Q1 (mount location) + Q3 (credit-pool thresholds) are the ones to confirm at Step 2.5.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN. (forbidden #4 is the load-bearing pin.)
3. Step 7.5 — name `Shell → view-switch (Usage) → UsageDashboard → gateway-client`.
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me.

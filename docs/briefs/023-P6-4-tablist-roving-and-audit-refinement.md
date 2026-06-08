# /tdd brief — settings_tablist_roving_and_audit_refinement

## Feature
Add WAI-ARIA **arrow-key roving tabindex** to the Settings tablist (exactly one tab in the tab order; Arrow/Home/End move focus + activate) AND teach the **§9 reachability audit** to be roving-aware — a `role="tab"` at `tabIndex=-1` that is a roving member is *reachable* (not a violation) — plus **extend the audit to cover `role="tabpanel"`** (panels are focusable-or-hidden but currently unaudited). The two are one atomic slice: the roving change makes inactive tabs `tabIndex=-1`, which would break the current `auditFocusable` (`tabIndex >= 0`) unless the audit is taught in the same change.

## Use case + traceability
- **Task ID:** P6.4 (tablist arrow-key roving + §9 audit refinement; origin 2026-06-08 P6.4c a11y polish)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.6` (accessibility invariants — keyboard-operable composite widgets), `§11.2` (Settings tablist surface). WAI-ARIA APG **Tabs pattern** (roving tabindex, automatic activation).
- **Related context:** Polish round (ui-006) slice 2, after slice 1 (TopBar history-nav, `022`). The Settings tablist (`ui/src/views/settings/Settings.tsx:52-65`) currently ships **all tabs as plain `<button>`s** → all `tabIndex=0` (conformant + audit-green, per the in-code note "arrow-key roving is a later a11y enhancement"). The **§9 reachability audit** lives as the `auditFocusable` helper inside `ui/src/a11y/reachability.test.tsx:22-32` (asserts every interactive control has `tabIndex >= 0`). The tabpanels (`Settings.tsx:71-85`) are `tabIndex=0` when visible / `hidden` when not, but the audit's selector doesn't include `[role="tabpanel"]` — unaudited. Lesson §9 is the a11y-foundation lesson this extends.

## Acceptance criteria (what "done" means)
- [ ] A pure roving helper `nextTabIndex(currentIndex, count, key)` (in `ui/src/views/settings/roving.ts`) maps a key to the next focus index: `ArrowRight` → `(i+1) % count` (wrap), `ArrowLeft` → `(i-1+count) % count` (wrap), `Home` → `0`, `End` → `count-1`, any other key → `currentIndex` unchanged (a sentinel the caller treats as "no roving move"). Pure + deterministic.
- [ ] The Settings tablist applies roving tabindex: the **selected** tab is `tabIndex=0`, every other tab `tabIndex=-1` (exactly one tabstop — the roving invariant).
- [ ] Arrow/Home/End on a focused tab moves focus to the computed tab AND activates it (**automatic activation** — panels are instant; matches the existing click=select model). Click selection is unchanged.
- [ ] The §9 audit (`auditFocusable`, extracted to a testable `ui/src/a11y/reachability.ts`) is **roving-aware**: a `role="tab"` at `tabIndex=-1` is reachable **iff** it is a member of a `role="tablist"` that has **exactly one** `tabIndex=0` tab. A genuinely-unreachable control (a non-tab `tabIndex=-1` actionable, or a tablist with zero/multiple tabstops) still fails.
- [ ] The §9 audit **covers `role="tabpanel"`**: a visible (`:not([hidden])`) tabpanel must be focusable (`tabIndex >= 0`); hidden panels are excluded.
- [ ] The existing whole-Shell reachability sweep (`every_interactive_control_is_keyboard_focusable`) stays green with the roving tablist in place.
- [ ] All unit tests in `ui/src/views/settings/roving.test.ts` + `ui/src/a11y/reachability.test.tsx` (new classification units) pass.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`main.tsx → <Shell/> → TopBar (onOpenSettings → navigate("settings")) → <Settings/> → the `role="tablist"` onKeyDown roving handler`. The roving is reached via the live Settings tablist (exercised through the Shell, not unit-only); the audit refinement is exercised by the existing `reachability.test.tsx` Settings sweep + the new classification units.

## Files expected to touch
**New:**
- `ui/src/views/settings/roving.ts` — the pure `nextTabIndex` helper (write it generically — the ProjectSwitcher dropdown widget, polish slice 5, will reuse roving).
- `ui/src/views/settings/roving.test.ts` — `nextTabIndex` unit tests.
- `ui/src/a11y/reachability.ts` — `auditFocusable` extracted from the test file as a testable module (the roving-aware + tabpanel-aware classifier).
- `ui/src/a11y/reachability.classify.test.tsx` — units pinning the refined audit (roving tab passes; broken tab fails; tabpanel coverage). _(Or fold into `reachability.test.tsx` — implementer's call at Step 2.5.)_

**Modified:**
- `ui/src/views/settings/Settings.tsx` — roving `tabIndex={tab.selected ? 0 : -1}`; tab button refs; `onKeyDown` on the tablist computing `nextTabIndex` → focus + activate.
- `ui/src/a11y/reachability.test.tsx` — import `auditFocusable` from the new module (no behavior change to the whole-Shell sweep).
- `ui/src/views/settings/Settings.test.tsx` — roving behavior tests (arrow moves focus + selects; Home/End; wrap; exactly one tabstop).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
`ui/src/views/settings/roving.test.ts` (pure helper):
1. **`roving_arrow_right_wraps`** — Asserts: `nextTabIndex(count-1, count, "ArrowRight") === 0`. Why: horizontal roving wraps (APG Tabs).
2. **`roving_arrow_left_wraps`** — Asserts: `nextTabIndex(0, count, "ArrowLeft") === count-1`. Why: wrap the other way.
3. **`roving_home_end`** — Asserts: `Home → 0`, `End → count-1`. Why: APG Home/End jump.
4. **`roving_other_key_is_unchanged`** — Asserts: a non-roving key returns `currentIndex` (caller no-ops). Why: only roving keys move focus.

`ui/src/views/settings/Settings.test.tsx` (behavior):
5. **`tablist_has_exactly_one_tabstop`** — Asserts: exactly one tab has `tabIndex=0`, the rest `-1`, and it's the selected one. Why: the roving invariant.
6. **`arrow_moves_focus_and_activates`** — Asserts: focus a tab, ArrowRight → focus + `aria-selected` move to the next tab (+ its panel shows). Why: automatic activation.
7. **`home_end_jump`** — Asserts: End focuses+selects the last tab; Home the first. Why: AC.

`ui/src/a11y/reachability.classify.test.tsx` (the refined audit — test-first the classification):
8. **`audit_passes_roving_tab_at_-1`** — Asserts: `auditFocusable` passes a tablist with one `tabIndex=0` tab + the rest `tabIndex=-1` (`role="tab"`). Why: roving members are arrow-reachable, not violations.
9. **`audit_fails_nontab_unreachable`** — Asserts: `auditFocusable` still fails a non-tab actionable at `tabIndex=-1` (e.g. a `<button>` with no roving context) AND a tablist with zero or multiple `tabIndex=0` tabs. Why: don't weaken the gate.
10. **`audit_covers_visible_tabpanel`** — Asserts: a visible `role="tabpanel"` without `tabIndex>=0` fails; a `hidden` one is excluded; the `tabIndex=0` visible panel passes. Why: panels are now audited.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. A11y behavior + test-infra refinement; no contract/`shared/` touch.
- **Orchestrator doc rows to write hot (Step 9 routing):** none (the roving + roving-aware-audit pattern likely **extends Lesson §9** — orchestrator banks; flag at Step 9).

## Things to flag at Step 2.5
1. **Activation model — automatic or manual?** Options: (a) automatic (arrow moves focus AND selects/shows the panel); (b) manual (arrow moves focus only; Enter/Space selects). My default vote: **(a) automatic** — APG recommends it when panels display without latency (ours are instant pending-stubs + the live Usage dashboard), and it matches the existing click=select model.
2. **Keys — Left/Right + Home/End with wrap?** My default vote: **yes** — `ArrowLeft`/`ArrowRight` (horizontal tablist) with wraparound + `Home`/`End`. No Up/Down (that's for vertical tablists). 
3. **Extract `auditFocusable` to a module vs refine inline?** My default vote: **extract to `ui/src/a11y/reachability.ts`** so the roving + tabpanel classification is itself test-first (the project's "the test IS the spec" discipline); `reachability.test.tsx` then imports it (no behavior change to the existing whole-Shell sweep).
4. **Roving reachability rule precision.** My default vote: a `role="tab"` at `tabIndex=-1` is reachable **iff** its `role="tablist"` ancestor has **exactly one** `tabIndex=0` tab — this both whitelists roving AND pins the one-tabstop invariant (zero or multiple tabstops = a real violation). Flag if you'd express it differently.
5. **Make the roving helper reusable for slice 5 (ProjectSwitcher dropdown)?** My default vote: **write `nextTabIndex` generically now** (it's already widget-agnostic) so the dropdown-popover widget's radiogroup/listbox roving (polish slice 5) reuses it rather than re-deriving. No premature abstraction beyond keeping it pure + parameterized.

## Dependencies + sequencing
- **Depends on:** the Settings tablist (6.4c `765923f`, landed) + the §9 reachability audit (6.4a `f70757e`, landed). Slice 1 (`022`) is independent (orthogonal surface) — no ordering constraint, but it lands first.
- **Blocks:** soft — polish slice 5 (ProjectSwitcher dropdown widget) reuses the `nextTabIndex` roving helper. Not a hard block (slice 5 could inline its own), but sequencing slice 2 before 5 lets 5 reuse it.

## Estimated commit count
**1 (atomic).** The roving change and the §9 audit-teach are **mutually dependent for green** — making inactive tabs `tabIndex=-1` breaks the current audit unless the audit is taught in the same change, so they land together. One cohesive a11y slice, same area, no safety invariant. Internally layered (pure `nextTabIndex` + roving tests → Settings wiring → audit extract+teach+tabpanel) but one Step-10 commit. (If the implementer finds a clean green intermediate — e.g. extract `auditFocusable` with no behavior change first — a 2-commit split is acceptable; enumerate at Step 2.5.)

## Lessons-logged candidates anticipated
- **Convention candidate** — WAI-ARIA **roving tabindex** for composite widgets (exactly one tabstop; Arrow/Home/End move + automatically activate); the §9 reachability audit is **roving-aware** (a `tabIndex=-1` roving member is reachable) + covers `role="tabpanel"`. Likely **extends Lesson §9** (a11y foundation) — orchestrator decides at Step 9.
- **Architecture-doc note candidate** — if §11.6 should pin "composite widgets use roving tabindex + automatic activation," flag it.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5."
2. **Run `/tdd settings_tablist_roving_and_audit_refinement`** (already oriented this round — no `/session-start`).
3. **Step 0 (Restate)** → confirm against the Feature line.
4. **Step 1 (Identify files)** → confirm against "Files expected to touch."
5. **Step 2.5** → tight test-design write-up + answers to the 5 design questions; wait for `APPROVED.` / `TWEAK:` / `ADD:`.
6. **Step 9** → categorized flags + ship-ask.

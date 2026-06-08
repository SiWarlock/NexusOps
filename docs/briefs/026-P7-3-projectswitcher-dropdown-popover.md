# /tdd brief — projectswitcher_dropdown_popover

## Feature
Upgrade the ProjectSwitcher from a flat row of `aria-pressed` buttons to a **dropdown-popover**: a trigger button (active project + caret) that opens a **WAI-ARIA listbox** popover of projects with **roving tabindex** (reusing `nextTabIndex`), full keyboard support (Arrow/Home/End/Enter/Escape), click-outside-to-close, and focus-return to the trigger on close. The active-project selection model/context is unchanged — only the switcher's presentation changes.

## Use case + traceability
- **Task ID:** P7.3 (the deferred ProjectSwitcher dropdown-popover widget — completes the active-project selection UI; origin P7.3 Q3)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (TopBar project switcher), `§11.6` (a11y — composite widget = roving tabindex + automatic activation; keyboard-operable; never color alone), WAI-ARIA APG **Listbox** + button-trigger popover.
- **Related context:** Polish round (ui-006) slice 5 (last). The current ProjectSwitcher (`ui/src/shell/ProjectSwitcher.tsx`) is a functional flat single-select (`<button aria-pressed>` per project, ✓ Active glyph+label, counts) reading the `ActiveProjectContext` via `useActiveProject()` (`active-project.ts` — **unchanged by this slice**). The deferred widget (Lesson §13 / P7.3 note) = trigger + caret + popover + WAI-ARIA listbox + roving. **Reuses slice 2's roving primitive** `nextTabIndex` (currently `ui/src/views/settings/roving.ts`) — see Step-2.5 Q2 (re-home to a shared `a11y/roving.ts`). The **slice-2 roving-aware §9 audit** already supports a roving listbox (a `tabIndex=-1` option in a one-tabstop list is reachable) — so the whole-Shell sweep stays green. Selection still flows `setActiveProject(id)` → graph re-roots + Sessions filter (no behavior change to the downstream scope).

## Acceptance criteria (what "done" means)
- [ ] A trigger `<button aria-haspopup="listbox" aria-expanded={open}>` shows the **active project name + a caret**; clicking it toggles the popover. At zero projects the trigger is disabled with a "No project" label (wire-or-disable — never a dead click, §11.6).
- [ ] The popover is a `role="listbox"` (labeled) of `role="option"` items, one per project, each with `aria-selected` on the active one + a ✓ glyph+label (never color alone, §11.6) + its counts as the accessible name (preserving the current self-contained naming).
- [ ] **Roving tabindex** in the listbox: exactly one option `tabIndex=0` (the active, or the first), the rest `-1`; Arrow/Home/End move focus via `nextTabIndex` (reused); the one-tabstop invariant holds (the slice-2 audit passes).
- [ ] **Keyboard:** Enter/Space on a focused option selects it (`setActiveProject`) + closes + returns focus to the trigger; **Escape** closes + returns focus to the trigger (no selection change); Arrow/Home/End move within the open listbox.
- [ ] **Open** focuses the active option (or the first if none); **click-outside** closes the popover (no selection change); selecting closes it.
- [ ] Clicking an option still calls `setActiveProject(project_id)` — selection behavior + downstream (graph re-root / Sessions filter) unchanged.
- [ ] `nextTabIndex` is re-homed to `ui/src/a11y/roving.ts` (shared a11y primitive) with slice-2's `Settings.tsx` import updated; the settings roving tests still pass (see Q2).
- [ ] All tests in `ui/src/shell/ProjectSwitcher.test.tsx` (updated to the dropdown structure) pass; the whole-Shell `reachability.test.tsx` sweep stays green.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`main.tsx → <Shell/> → <TopBar/> → <ProjectSwitcher/> (trigger → popover listbox → option select → setActiveProject)`. Confirm the popover renders + selects on the real Shell path (the existing Shell integration test for active-project still passes — selecting through the dropdown re-roots the graph + filters Sessions).

## Files expected to touch
**New:**
- `ui/src/a11y/roving.ts` — `nextTabIndex` (+ `isRovingKey`) **moved** from `views/settings/roving.ts` (shared a11y primitive; co-located with `reachability.ts`).

**Modified:**
- `ui/src/shell/ProjectSwitcher.tsx` — the trigger + popover listbox + roving + keyboard + click-outside + focus management (the bulk of the slice).
- `ui/src/shell/ProjectSwitcher.test.tsx` — rewritten to the dropdown structure (open/close, select, keyboard, roving, focus-return, never-color-alone).
- `ui/src/views/settings/roving.ts` → **deleted** (moved to `a11y/roving.ts`); `ui/src/views/settings/roving.test.ts` re-pointed to the new path; `ui/src/views/settings/Settings.tsx` import updated.
- _(Possibly)_ a small `popover` open/close hook if the implementer extracts one — flag at Step 2.5.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
`ui/src/shell/ProjectSwitcher.test.tsx`:
1. **`trigger_shows_active_project_and_toggles`** — Asserts: the trigger shows the active project name + `aria-expanded=false`; clicking opens the listbox (`aria-expanded=true`). Why: the popover trigger.
2. **`listbox_lists_projects_with_selected`** — Asserts: one `role="option"` per project; the active has `aria-selected=true` + the ✓ glyph+label. Why: listbox structure + never-color-alone.
3. **`option_click_selects_and_closes`** — Asserts: clicking an option calls `setActiveProject(id)`, closes the popover, returns focus to the trigger. Why: selection behavior preserved.
4. **`roving_one_tabstop_and_arrows_move`** — Asserts: exactly one option `tabIndex=0`; ArrowDown/Up (or Right/Left) move focus via `nextTabIndex`; Home/End jump. Why: roving (reuse) + APG.
5. **`enter_selects_escape_closes`** — Asserts: Enter/Space on a focused option selects+closes; Escape closes WITHOUT a selection change; both return focus to the trigger. Why: keyboard contract.
6. **`open_focuses_active_option`** — Asserts: opening focuses the active option (or the first if none active). Why: APG open behavior.
7. **`click_outside_closes`** — Asserts: a click outside the popover closes it with no selection change. Why: dismiss behavior.
8. **`zero_projects_trigger_disabled`** — Asserts: with no projects the trigger is `disabled` + a "No project" label (never a dead click). Why: wire-or-disable (§11.6).

`ui/src/a11y/roving.test.ts` (moved) + `reachability.test.tsx`:
9. **`roving_helper_unchanged_after_move`** — Asserts: `nextTabIndex` behaves identically from `a11y/roving.ts` (the moved tests pass). Why: a pure move, no behavior change.
10. **`shell_sweep_green_with_dropdown`** — Asserts: the whole-Shell §9 reachability sweep passes with the dropdown closed AND open (the roving listbox satisfies the one-tabstop rule). Why: no a11y regression.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. Presentation of an existing UI selection (§13 family); no contract/`shared/` touch, no mutation/intent.
- **Orchestrator doc rows to write hot (Step 9 routing):** none (likely a small note under Lesson §9/§13 — the dropdown/popover composite pattern + the roving re-home; orchestrator decides at Step 9).

## Things to flag at Step 2.5
1. **ARIA pattern — listbox or radiogroup?** My default vote: **listbox** (`button[aria-haspopup="listbox"]` trigger + `role="listbox"`/`role="option"`/`aria-selected`) — the conventional select-style dropdown; pairs naturally with roving tabindex. (Radiogroup is also valid but reads as a form control, not a switcher.)
2. **Re-home `nextTabIndex` to `a11y/roving.ts`, or import in place from `views/settings/`?** My default vote: **re-home to `ui/src/a11y/roving.ts`** — it's a shared a11y primitive (belongs with `reachability.ts`/`focus.css`); `shell/` importing from `views/settings/` is a code-smell. A small mechanical move + import update in slice-2's `Settings.tsx`; the settings roving tests move with it.
3. **Roving tabindex vs `aria-activedescendant` for the listbox?** My default vote: **roving tabindex** — reuses `nextTabIndex` (consistency with the tablist) and the slice-2 audit already understands it.
4. **Open/Escape focus management.** My default vote: open focuses the **active** option (first if none); Escape + select + click-outside all close; Escape/select **return focus to the trigger** (APG). Flag if you'd vary it.
5. **Trigger content — name + caret only, or also counts?** My default vote: **active project name + caret** on the trigger (compact); the **counts stay on the options** (where the current self-contained naming lives). The selected option keeps `aria-selected` + ✓ glyph + label.

## Dependencies + sequencing
- **Depends on:** the active-project model/context (`86727ec`, landed — unchanged here) + slice 2's `nextTabIndex` roving helper (`128714e`, landed — reused/re-homed) + slice 2's roving-aware §9 audit (so the roving listbox audits clean). Sequenced last for these reasons.
- **Blocks:** nothing — last slice of the polish round.

## Estimated commit count
**2 (Lesson §7 — driven layer→layer).** This is the round's largest slice; split for green intermediates + bisectability:
- **L1 — popover shell:** the trigger (`aria-haspopup`/`aria-expanded` + active-name + caret + zero-projects disabled), the `role="listbox"`/`option` structure, open/close state, **click-selects-and-closes**, the `nextTabIndex` re-home. Click-driven; green.
- **L2 — keyboard a11y:** roving tabindex + Arrow/Home/End/Enter/Space/Escape + open-focuses-active + focus-return-to-trigger + click-outside-to-close + the whole-Shell sweep green. Green.

I'll drive L1→L2 (one wake per layer at the commit boundary). If the implementer judges it lands cleanly as 1 commit, that's fine — confirm at Step 2.5.

## Lessons-logged candidates anticipated
- **Convention candidate** — the **dropdown/popover composite-widget** pattern: a `button[aria-haspopup]` trigger + a roving `role="listbox"` popover, open-focuses-active / Escape-&-select-return-focus-to-trigger / click-outside-closes, reusing the shared `nextTabIndex` (`a11y/roving.ts`); the §9 roving-aware audit covers it. Likely a small extension to **Lesson §9** (composite-widget a11y) — orchestrator decides at Step 9.
- **Architecture-doc note candidate** — §11.2 already names the project switcher; flag only if the dropdown interaction model needs an explicit note.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5."
2. **Run `/tdd projectswitcher_dropdown_popover`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** → confirm against the Feature line.
4. **Step 1 (Identify files)** → confirm against "Files expected to touch" (incl. the `nextTabIndex` re-home).
5. **Step 2.5** → tight test-design write-up + answers to the 5 design questions + your commit-count call; wait for `APPROVED.` / `TWEAK:` / `ADD:`.
6. **Multi-commit drive (Lesson §7):** if 2 commits, I wake you layer→layer at each commit boundary.
7. **Step 9** → categorized flags + ship-ask.

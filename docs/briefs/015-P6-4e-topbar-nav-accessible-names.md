# /tdd brief — topbar_nav_accessible_names

## Feature
Two TopBar-touching items (human-confirmed + tracked): **(1) the §11.2 nav reconcile** — wire the existing **TopBar "Settings" placeholder** to open the Settings view (`contentView="settings"`) and **drop "Settings" from the content-view-switch** (switch = content surfaces CC/Graph/Sessions; TopBar reaches Settings + later Brain); **(2) accessible names** (§11.7 carry-forward) on the TopBar's **icon-only / back-forward controls** (kit `Button` props are closed → aria-label routed onto the NexusOps wrapper, Lesson §6). Daemon-independent. Resolves the 6.4c nav Finding (human-confirmed 2026-06-08) + the §11.7 accessible-names carry-forward.

## Use case + traceability
- **Task ID:** P6.4e (TopBar slice; from the 6.4c nav Finding + the §11.7 accessible-names carry-forward).
- **Architecture sections:** `ARCHITECTURE.md §11.2` (nav-model note — view-switch = content surfaces; TopBar = Settings/Brain; **human-confirmed 2026-06-08**), `§11.7` (accessible names on icon-only/back-forward controls; kit closed-props → wrapper pattern), `§11.4`/`§11.6` (a11y). Lesson §6 (route `aria-*` onto wrappers), §9 (focus ring + reachability).
- **Related context:** `shell/TopBar.tsx` (the placeholder Settings `<button>` from 6.1a, no onClick; any back/forward + icon-only controls); `shell/Shell.tsx` (the `contentView` state — drop the "settings" option from the view-switch, add it to the TopBar trigger); the §9 reachability audit (Settings is now reached via the TopBar, not the view-switch — the audit's Settings-sweep updates). Deterministic core = the nav-trigger wiring + the accessible-name presence (render-tested).

## Acceptance criteria
- [ ] The **TopBar "Settings" button is wired** → sets `contentView="settings"` (opens the Settings view in the content pane); **"Settings" is REMOVED from the content-view-switch** (switch now offers CC / Graph / Sessions only).
- [ ] Settings remains reachable + functional (the 6.4c tablist + Usage tab) — now via TopBar → Settings.
- [ ] Every **icon-only / back-forward TopBar control has an accessible name** (`aria-label` on the NexusOps wrapper — kit `Button` props are closed, Lesson §6); no icon-only control is unlabeled.
- [ ] The **§9 reachability audit updates** so its Settings sweep reaches Settings via the **TopBar** trigger (not the removed view-switch option); audit stays green; the TopBar Settings + icon-only controls are keyboard-reachable + focus-ringed (Lesson §9).
- [ ] Renders only real state (forbidden #2); `/preflight` clean.
- [ ] **Reachable from** `Shell → TopBar (Settings) → Settings view`; the view-switch no longer carries Settings.

## Wiring / entry point (Step 7.5)
`TopBar` Settings button → `Shell` `setContentView("settings")` → `<Settings/>`; the content-view-switch renders CC/Graph/Sessions only. Confirm at Step 7.5: TopBar Settings opens Settings + the view-switch dropped it + the reachability audit reaches Settings via the TopBar.

## Files expected to touch
**Modified:** `ui/src/shell/TopBar.tsx` (wire Settings onClick; aria-labels on icon-only/back-forward controls), `ui/src/shell/Shell.tsx` (TopBar Settings → contentView; drop "settings" from the view-switch options; pass the handler/state to TopBar), `ui/src/shell/{TopBar.test.tsx, Shell.test.tsx}`, `ui/src/a11y/reachability.test.tsx` (reach Settings via the TopBar). Flag anything beyond at Step 2.5.

## RED test outline (Step 2)
**`shell/Shell.test.tsx` (+ TopBar.test.tsx):**
1. **`topbar_settings_opens_settings_view`** — clicking the TopBar Settings button sets `contentView="settings"` → `<Settings/>` mounts. **[load-bearing — nav reconcile]**
2. **`view_switch_no_longer_offers_settings`** — the content-view-switch offers exactly CC / Graph / Sessions (no "Settings"). **[load-bearing — drop the duplicate]**
3. **`settings_still_reachable_and_functional`** — Settings (tablist + Usage tab) renders when opened via the TopBar.
4. **`icon_only_topbar_controls_have_accessible_names`** — every icon-only / back-forward TopBar control exposes an `aria-label` (non-empty accessible name). **[load-bearing — §11.7]**

**`a11y/reachability.test.tsx` (update):**
5. **`reachability_reaches_settings_via_topbar`** — the multi-view audit reaches the Settings surface via the TopBar trigger (not the removed view-switch option); all its controls keyboard-focusable; audit non-vacuous. **[§9 net]**

## Cross-doc invariant impact
- **Model field changes:** **none.** Nav wiring + a11y labels; no shape/contract change. The §11.2 nav-model arch-note is **already written** (this slice executes it). **Orchestrator rows:** none.

## Things to flag at Step 2.5
1. **Nav rewire (human-confirmed — execute).** Default: TopBar Settings → `contentView="settings"`; view-switch = CC/Graph/Sessions. (Confirmed 2026-06-08 — flag only if the wiring surfaces a snag.)
2. **Accessible-name set.** Default vote: label every icon-only / back-forward TopBar control; name the exact controls at Step 2.5 (what's actually icon-only in `TopBar.tsx`).
3. **Brain TopBar button.** Default vote: **deferred** — the Brain drawer (§11.5) isn't built; this slice is the Settings rewire + accessible-names only (a Brain TopBar trigger lands with the Brain drawer, Phase 8). Confirm.

## Dependencies + sequencing
- **Depends on:** 6.4c Settings (`765923f`) + the §11.2 nav-model arch-note (already written) + Lesson §6/§9.
- **Resolves:** the 6.4c nav Finding (the duplicate Settings) + the §11.7 accessible-names carry-forward (the shell-controls portion).
- **Note:** unstyled until the 6.5 theme pass.

## Estimated commit count
**1** — a cohesive TopBar slice (nav rewire + accessible-names; both touch `TopBar.tsx`/`Shell.tsx`). No safety invariant → **security-reviewer NOT required**; **code-quality every-slice**.

## Lessons-logged candidates anticipated
- **Convention candidate** — none new expected (executes the §11.2 nav-model + reuses Lesson §6 wrapper-aria pattern).
- **Future TODO** — the Brain TopBar trigger lands with the Brain drawer (Phase 8); the remaining §11.7 kit-closed-props HTMLAttributes-passthrough resolution stays a 6.4/kit-contract item.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd topbar_nav_accessible_names`.
1. Read this brief; Q2 (accessible-name set) + Q3 (Brain deferred) are the ones to confirm; Q1 is human-confirmed.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN.
3. Step 7.5 — name `Shell → TopBar (Settings) → Settings view`; confirm the audit reaches Settings via the TopBar.
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me.

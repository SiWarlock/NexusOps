# /tdd brief — settings_surface

## Feature
The **Settings tabbed surface** (§11.2: "Usage Dashboard → Settings tab"): a 4th content-view "Settings" (**replacing** the interim "Usage" in the view-switch) with tabs — **Integrations health · Security & policy · Notifications · Usage · Execution Profiles** — where the **Usage tab mounts the existing `<UsageDashboard/>`** (the 6.4b relocation), the daemon-coupled panels render **honest "pending" empty-states** (no fabricated data — forbidden #2), and the **Execution Profiles tab stays 0.5b-gated** (rendered "pending", enum NOT hard-bound). Tab controls are keyboard-reachable + ARIA-correct (Lesson §9). **Intent-submitting controls (notification toggles, policy edits, profile changes) are PARKED** (intent seam) — this slice is the tabbed shell + display + relocation only. Daemon-independent (Decision-C track).

## Use case + traceability
- **Task ID:** P6.4c (6.4 decomposition: 6.4a ✅ → 6.4b ✅ → **6.4c Settings + Usage relocation** → 6.4d Survival/recovery display → accessible-names/checking-banner → 6.5 theme pass).
- **Architecture sections:** `ARCHITECTURE.md §11.2` (Settings tab folds Usage; Integrations/Security/Notifications/Execution-Profiles), `§11.4` (Settings: Integrations health, Execution Profiles, Security & policy, Notifications), `§4.2` (renders from projections — no invented state). a11y per Lesson §9.
- **Related context:** 6.4b `<UsageDashboard/>` (relocates from the interim content-view into the Usage tab); the 6.3b `contentView` switch (Settings becomes the 4th option, replacing "Usage"); Lesson §9 (tab controls inherit the focus ring + must be keyboard-reachable; the reachability audit sweeps the new view); ExecutionProfile held 0.5b. Deterministic core = the tab state/selection + the honest-stub rendering (pure-ish); the relocation reachability is render-tested.

## Acceptance criteria
- [ ] A **Settings** content-view (`ui/src/views/settings/Settings.tsx`) is a **tablist** (`role="tablist"` + `role="tab"`/`aria-selected` + `role="tabpanel"`) with tabs Integrations / Security & policy / Notifications / **Usage** / **Execution Profiles**; tab controls are `<button>`s (keyboard-reachable, focus ring — Lesson §9).
- [ ] The **Usage tab mounts `<UsageDashboard/>`** (relocation); the view-switch's interim "Usage" option is **removed** and replaced by "Settings" (Usage is now reached via Settings → Usage).
- [ ] Integrations / Security & policy / Notifications tabs render **honest empty-state stubs** (e.g. "Integrations health — pending integrations (Phase 7)") — **no fabricated data**, no fake toggles; intent controls explicitly deferred.
- [ ] The **Execution Profiles tab is 0.5b-gated** — renders a "pending the ExecutionProfile enum (0.5b)" state; does NOT import/hard-bind a non-frozen enum.
- [ ] Selecting a tab shows its panel + sets `aria-selected`/`aria-controls` correctly (one selected at a time; a sensible default tab). Renders only real state (forbidden #2). `/preflight` clean; the §9 reachability audit sweeps the Settings view + its tabs.
- [ ] **Reachable from** `Shell → view-switch (Settings) → Settings tabs → {UsageDashboard | stubs}`.

## Wiring / entry point (Step 7.5)
`Shell` view-switch → `<Settings/>` (4th view, replacing interim "Usage") → tablist → the Usage tab mounts `<UsageDashboard/>` over the gateway boundary; stub tabs render pending-states. Confirm the view-switch swap (Usage→Settings) + Usage reachable via Settings.

## Files expected to touch
**New:** `ui/src/views/settings/Settings.tsx`, `ui/src/views/settings/{tabs.ts (tab model/state), Settings.test.tsx, tabs.test.ts}`.
**Modified:** `ui/src/shell/Shell.tsx` (view-switch: "Usage"→"Settings"; mount `<Settings/>`; the Usage projection fetch stays — now consumed inside Settings→Usage), `ui/src/shell/Shell.test.tsx`, `ui/src/a11y/reachability.test.tsx` (sweep the Settings view + tabs — §9 net). Flag anything beyond at Step 2.5.

## RED test outline (Step 2)
**`views/settings/tabs.test.ts`:**
1. **`default_tab_selected`** — the tab model exposes a sensible default selected tab; exactly one selected.
2. **`select_tab_switches_active`** — selecting a tab updates the active tab (pure state); others deselect.
3. **`execution_profiles_tab_is_gated`** — the Execution Profiles tab is marked gated/pending (no frozen-enum binding).

**`views/settings/Settings.test.tsx` (jsdom):**
4. **`renders_tablist_with_aria`** — `role="tablist"` + a `role="tab"` per tab with `aria-selected`; the selected tab's `role="tabpanel"` shows. **[a11y]**
5. **`usage_tab_mounts_usage_dashboard`** — the Usage tab renders `<UsageDashboard/>` (relocation). **[load-bearing — relocation]**
6. **`pending_tabs_show_honest_empty_state`** — Integrations/Security/Notifications render an explicit "pending" state, **not** fabricated data/toggles (forbidden #2).
7. **`execution_profiles_tab_pending_not_bound`** — the Execution Profiles tab renders "pending 0.5b", no enum import.

**`shell/Shell.test.tsx` (extend):**
8. **`view_switch_mounts_settings_not_interim_usage`** — the view-switch offers **Settings** (not a top-level "Usage"); selecting it mounts `<Settings/>`; Usage is reachable via Settings→Usage. **[wiring — Step 7.5]**

## Cross-doc invariant impact
- **Model field changes:** **none.** Reuses the 6.4b Usage shapes (already provisional + in the reconcile spread); Settings is UI structure. **Orchestrator rows:** none.

## Things to flag at Step 2.5
1. **View-switch swap.** Default vote: replace the interim top-level "Usage" with "Settings"; Usage moves into Settings→Usage (the §11.2 home). Confirm vs keeping both.
2. **Pending stubs.** Default vote: honest empty-states for Integrations (Phase 7) / Security & policy (Phase 2) / Notifications (notifier) — no fake data/toggles; intent controls parked. Confirm.
3. **Execution Profiles tab.** Default vote: a gated "pending 0.5b" tab, no enum binding (don't hard-bind — lead directive). Confirm.
4. **Tab a11y pattern.** Default vote: full ARIA tabs (`role=tablist/tab/tabpanel` + `aria-selected`), tab `<button>`s keyboard-reachable (Lesson §9); arrow-key roving optional (flag if deferring to a later a11y pass). Confirm the pattern.

## Dependencies + sequencing
- **Depends on:** 6.4b `<UsageDashboard/>` (`db9b89b`), the view-switch seam (6.3b), Lesson §9 (a11y). No daemon dependency.
- **Blocks:** the §11.2 Settings home; the parked intent controls (notification toggles / policy / profile changes) land here when the intent seam lands (daemon-1.5); the ExecutionProfile tab fills at 0.5b.
- **Note:** the Settings panels will be **unstyled** until the 6.5 theme pass (accepted — human-sequenced).

## Estimated commit count
**1** — a cohesive slice (tab model + Settings view + Usage relocation + the view-switch swap + honest stubs). No safety invariant (display/tabs; intent controls parked) → **security-reviewer NOT required**; **code-quality every-slice**.

## Lessons-logged candidates anticipated
- **Convention candidate** — honest "pending [Phase X]" empty-states for daemon-coupled surfaces built ahead of their data (no fabricated data/controls — forbidden #2); the tab a11y pattern. Candidate if it recurs (6.4d may reuse).
- **Future TODO** — the parked Settings intent controls (notification toggles / `save-as-policy` / profile changes) land with the daemon-1.5 intent seam; the ExecutionProfile tab fills at 0.5b (both already tracked).

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd settings_surface`.
1. Read this brief; Q1 (view-switch swap) + Q4 (tab a11y pattern) are the ones to confirm at Step 2.5.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN.
3. Step 7.5 — name `Shell → view-switch (Settings) → tabs → UsageDashboard/stubs`.
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me.

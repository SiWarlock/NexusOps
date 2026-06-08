# visual brief — shell_chrome_grid (6.5b)

> **NON-`/tdd` visual slice** (same flow as 6.5a): read → send me the planned CSS + the grid-mapping design-Q answers (visual Step-2.5) → I review → apply → `/preflight` (the existing Shell + a11y/reachability tests must stay green through the structural refactor) → Vite dev server rendered check + screenshot → report. Acceptance is VISUAL.

## Feature
The **6.5b shell chrome + layout grid** — the 2nd theme layer. Refactor `Shell.tsx` from its current **flexbox column** to the prototype's **CSS Grid** cockpit layout, and add `ui/src/theme/shell.css` painting the shell regions (top bar / sidebar / main / activity dock / status bar / banner stack / safety-host) with kit **semantic** token surfaces, borders, and elevation. Lifts the inline structural `style={{}}` out of `Shell/TopBar/Sidebar/StatusBar` into token-referencing classes. After this, the cockpit has its real chrome + layout; only the in-view panels (6.5c) remain bare.

## Use case + traceability
- **Task ID:** P6.5b (6.5 decomposition — `MVP_TASKS.md §6.5`). Depends on 6.5a (base layer, landed).
- **Architecture sections:** `ARCHITECTURE.md §11.1` (semantic-token layer only — re-hue promise), `§11` (the cockpit layout), `§11.4` (the shell regions: top bar / sidebar / main / drawer / dock / status bar). Target = the prototype `NexusOps-ui-kit/ui_kits/control-plane/index.html` `<style>`: `.shell { display:grid; grid-template-columns: var(--shell-sidebar-w) 1fr; grid-template-rows: var(--shell-topbar-h) 1fr auto; grid-template-areas: "top top" "side main" "dock dock"; }` + `.main { grid-area:main; overflow:hidden; min-width:0; }`.
- **Related context:** the current `Shell.tsx` render (flexbox: TopBar → banner stack [DegradedBanner/RecoveryBanner/AuditIntegrityAlert/`.safety-host`] → flex-row{Sidebar, main, DrawerStack} → ActivityDock → StatusBar). 6.5a's `global.css` set the base canvas (dark/Geist). Lesson §3 (semantic tokens only) · §10 (visual gate) · §9 (don't break the reachability audit — the structural refactor must preserve every control's keyboard-reachability + the `data-testid="safety-host"` + the region landmarks).

## The mapping problem (the load-bearing design decision)
The prototype's grid is a minimal **2-col × 3-row** (top / side+main / dock). The app has **more regions** than the prototype's 3 areas — a **conditional banner stack**, a **safety-host**, a **right DrawerStack** (empty/Phase-8 placeholder), and a **status bar**. 6.5b must map the app's real regions onto an ADAPTED grid (the prototype is the visual target, not a literal 3-area constraint). **My default mapping** (the brief's proposal — confirm/adjust at Step-2.5):
```css
.shell {
  display: grid;
  grid-template-columns: var(--shell-sidebar-w) 1fr;            /* sidebar | main */
  grid-template-rows: var(--shell-topbar-h) auto 1fr auto auto; /* top | banner | content | dock | status */
  grid-template-areas:
    "top    top"
    "banner banner"
    "side   main"
    "dock   dock"
    "status status";
  height: 100vh; min-height: 0;
}
```
- `top` = `<TopBar>` · `banner` = the banner stack (auto → 0 height when nothing renders) · `side` = `<Sidebar>` · `main` = content · `dock` = `<ActivityDock>` · `status` = `<StatusBar>`.
- **DrawerStack:** empty placeholder until the Brain drawer (Phase 8) — render it as an **overlay/absent** (NOT a permanent 3rd grid column) so the 2-col cockpit matches the prototype; flag if you'd rather reserve a column.

## Acceptance criteria
- [ ] **`ui/src/theme/shell.css`** (NEW): the `.shell` grid (above) + region chrome — `.topbar` (token surface + bottom border + `--shell-topbar-h`), `.sidebar` (`--surface-panel` + right border + `--shell-sidebar-w`), `.main` (`grid-area:main; overflow:hidden; min-width:0`), `.activity-dock` (token surface + top border), `.status-bar` (token surface + top border + ~26px), `.safety-host` (no chrome when empty), `.content-switch` (a toolbar styled with token spacing/controls). **Semantic tokens only** (§11.1).
- [ ] **`Shell.tsx` refactored** flexbox→grid: the `.shell` wrapper loses its inline `style`; `TopBar`/banner-stack/`Sidebar`/`main`/`ActivityDock`/`StatusBar` become **direct grid children** mapped to areas (remove the wrapping `<div style={{display:flex…}}>`); `main` keeps its `aria-label="Main surface"`. The render ORDER + the `data-testid="safety-host"` + the content-switch + every view are preserved.
- [ ] **Inline structural styles lifted** out of `Shell.tsx` (the 2 inline `style={{}}`), `TopBar.tsx` (the flex-row styles), `Sidebar.tsx` (`width`/list-reset/flex), `StatusBar.tsx` (flex) → token-referencing classes in `shell.css`.
- [ ] **The existing Shell + a11y/reachability tests stay green** — the structural refactor preserves reachability (every control keyboard-reachable, §9), the region landmarks, and the view-switch behavior. `/preflight` clean (suite 133+).
- [ ] **Rendered check + screenshot:** the cockpit now shows the dark grid layout — top bar (44px) + sidebar (264px, panel surface) + main + dock + status bar, with token borders/surfaces distinguishing the regions (vs 6.5a's bare flexbox). *(Full prototype-match gate is at 6.5c.)*

## Files expected to touch
**New:** `ui/src/theme/shell.css`.
**Modified:** `ui/src/main.tsx` (import `./theme/shell.css` after `global.css`), `ui/src/shell/Shell.tsx` (grid refactor), `ui/src/shell/TopBar.tsx` / `Sidebar.tsx` / `StatusBar.tsx` (inline → classes). Possibly `ActivityDock.tsx`. Flag anything beyond at Step 2.5.

## Design questions (the visual Step-2.5)
1. **Grid mapping (8 regions → areas).** Default: the 5-row grid above (top/banner/side+main/dock/status). My default vote: **that mapping** — banners get their own auto-height row (push content down, full-width, matching current behavior); status bar a thin bottom row. Confirm/adjust.
2. **DrawerStack.** Default: **overlay/absent** (not a permanent grid column) until the Brain drawer (Phase 8) — keeps the 2-col cockpit matching the prototype. My default vote: **absent/overlay now**. Confirm vs reserving a `drawer` column.
3. **`--shell-sidebar-w` / `--shell-topbar-h` token source.** Verify: are these **kit tokens** or prototype-local `<style>` vars? Default: if kit tokens exist, use them; else define **app-level layout vars** in `shell.css` (referencing the kit spacing/control scale — 264px sidebar / 44px top bar). My default vote: **use kit tokens if present, else app-level layout vars in shell.css** (still semantic, not primitives). Confirm what you find.
4. **Banner stack — own grid row vs inside main.** Default: **own `banner` row** (full-width, above side+main — matches the current above-content placement + the §17 "prominent/seen" requirement for the audit-integrity alert). My default vote: **own row**. Confirm.

## Cross-doc invariant impact
- **None.** Pure visual/structural-CSS layer; no contract/model/enum. The Shell refactor preserves the component contract (props/testids/landmarks unchanged).

## Estimated commit count
**1** — the shell chrome + grid (one cohesive layout layer). Non-safety, non-/tdd. *(If the grid refactor + the chrome classes feel separable, a 2-commit split [grid refactor → region chrome] is fine — flag at Step 2.5; default 1.)*

## Lessons-logged candidates anticipated
- **Convention candidate** — folds into the 6.5 theme-pass lesson at the pass close (the theme layers APPLY semantic tokens via `ui/src/theme/*.css`; the structural shell refactor preserves reachability + landmarks; the visual gate is the acceptance — incl. the 6.5a `*/`-in-CSS-comment §10 reinforcement).

## How to invoke
> Non-`/tdd`. Do NOT run `/tdd` or `/session-start`.
1. Read this brief + the prototype `<style>` (the grid) + the current `Shell.tsx` render.
2. **Send me the planned `shell.css` + the grid mapping + your 4 design-Q answers** (esp. Q1 mapping + Q3 token source) — the visual Step-2.5. Wait for APPROVED./TWEAK:.
3. Apply → `/preflight` (Shell + a11y/reachability tests green through the refactor) → Vite dev server rendered check + screenshot.
4. Report (files, suite count, screenshot, any flags) → I give the commit message → land + `TaskUpdate` completed + wake me for 6.5c (the final layer + the full visual gate).

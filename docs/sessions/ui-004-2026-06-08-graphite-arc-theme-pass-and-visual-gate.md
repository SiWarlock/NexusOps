# ui-004 — Graphite Arc theme pass (6.5a/b/c) + the full visual gate

- **Date:** 2026-06-08
- **Phase:** Phase 6 (UI track, `track/ui`) — the **6.5 visual layer**; closes Phase-6 logic + visual (modulo parked 6.3d/e).
- **Predecessor session:** [ui-003](ui-003-2026-06-08-safety-state-display-and-checking-banner.md)
- **Successor session:** _(pending — most remaining ui work is daemon-gated: parked 6.3d/e + the daemon-1.5 integration; Phase 7 PR-Review/Tasks is the next daemon-independent-ish UI surface)_
- **Round commits:** `c439379` (6.5a) · `b5618af` (6.5b) · `3b6135d` (6.5c). Suite **131 → 133 green**; tsc + oxlint clean throughout.

## Why this session existed

Close the **6.5 Graphite Arc theme/visual layer** — the unstyled-layer Finding (rounds 1–2 shipped functionally-correct but completely unstyled; the kit defines tokens but the app never applied them — `ui/LESSONS.md §10`). Three decomposed layers (base → shell-chrome → view-panels) ending with the **full automated visual gate** (dev server vs the prototype) as the acceptance for the whole pass.

## What was built

All three layers APPLY the kit's already-imported **semantic** tokens via `ui/src/theme/*.css` (semantic only — the §11.1 re-hue promise, never primitives). Surfaces grounded against the prototype's actual rendering (`ui_kits/control-plane/index.html` + `kit-shell.jsx`/`kit-views.jsx`).

### 6.5a — base/global theme layer (`c439379`)
- **NEW** `ui/src/theme/global.css` — `html,body` reset (dark `--surface-window` + `--text-primary` + Geist `--font-sans` + 13px `--fs-body` + `--lh-normal` + `overflow:hidden`), `#root` 100vh, webkit scrollbar chrome (`--border-strong` thumb / `--surface-canvas` border), and the global `.sr-only` visually-hidden utility.
- **NEW** `ui/src/theme/global.test.tsx` — deterministic wiring-guard (theme imported after the kit tokens; the TopBar accessible-name spans use `.sr-only`).
- **MOD** `ui/src/main.tsx` (import `global.css` after the kit `styles.css`, before `focus.css`); `ui/src/shell/TopBar.tsx` (`SR_ONLY` inline constant → the global `.sr-only` class; removed the now-unused `CSSProperties` import — behavior-preserving, Lesson §6).

### 6.5b — shell chrome + cockpit grid (`b5618af`)
- **NEW** `ui/src/theme/shell.css` — the `.shell` CSS Grid (2-col `--shell-sidebar-w | 1fr` × 5-row `top/banner/side+main/dock/status`) + region chrome (topbar/sidebar/dock/status all `--surface-panel` + `--border-default`; content-switch toolbar `--surface-input`/`--surface-active`; `.drawer-stack` an `:empty`-collapsing overlay).
- **MOD** `ui/src/main.tsx` (import `shell.css`); `ui/src/shell/Shell.tsx` (flexbox→grid: `.banner-stack` wrap, flattened the flex-row wrapper, `main` classed, DrawerStack→overlay child); `TopBar.tsx`/`Sidebar.tsx`/`StatusBar.tsx` (inline structural styles → token classes; fixed the sidebar width drift 240 → kit `--shell-sidebar-w` 264).

### 6.5c — view panels + severity surfaces + per-panel scroll (`3b6135d`)
- **NEW** `ui/src/theme/components.css` — themes the content views (CommandCenter `--surface-canvas` panel + section dividers + attention rows; Sessions/Usage/Graph dense tables with `--surface-sunken` headers + `--border-subtle` rows; ProjectGraph canvas `--graph-canvas` + `--surface-card` node chips; Settings tablist/panels) and the banner/safety surfaces (DegradedBanner `--warning-*`/update `--danger-*`/checking subtle; RecoveryBanner; AuditIntegrityAlert critical `--critical-*`/warning `--warning-*`; HardConflictCard `--danger-*`) — **severity color ADDITIVE to the existing glyph+label** (never-color-alone preserved). Per-panel scroll (`.main` overflow:hidden + view roots scroll). ProjectSwitcher compacted to a CSS horizontal row.
- **MOD** `ui/src/main.tsx` (import `components.css`); `ui/src/theme/shell.css` (`.main` overflow auto→hidden); `ui/src/views/command/CommandCenter.tsx` + `ui/src/views/graph/ProjectGraph.tsx` (inline-style lifts → classes).

## Decisions made

- **Theme via `ui/src/theme/{global,shell,components}.css`** — three layered app stylesheets that APPLY the kit semantic tokens (the kit ships tokens-only; applying them is the app's job). Cascade: kit `styles.css` → global → shell → components → focus.
- **GROUND surfaces against the prototype's actual rendering, don't guess** — the prototype's region/panel chrome lives in `kit-shell.jsx`/`kit-views.jsx` (not the index.html `<style>`); sampling the source caught the **6.5b `--surface-canvas`→`--surface-panel` divergence** (topbar/dock/status). Corrected at the source rather than at the gate.
- **The visual gate is the acceptance** (not green tests) — it caught the **6.5a `*/`-in-CSS-comment bug** (a comment token-glob `--n-*/--ink-*` prematurely closed the comment and dropped the entire `html,body` reset; tsc/oxlint/jsdom/wiring-guard all passed because none parse+apply CSS) and the 6.5b surface divergence.
- **Structural refactor (flexbox→CSS-Grid) preserves reachability** — the Shell + a11y/reachability tests stayed green through the DOM restructure (landmarks/testids/roles/view-switch preserved).
- **Severity colors ADDITIVE to glyph+label** — theming the never-color-alone surfaces (banners/safety/Usage) adds background/border color that REINFORCES the existing glyph + label + severity channels, never replacing them (§11.6 / forbidden #4/#5).
- **ProjectSwitcher: CSS-compact-row, defer the prototype's single-select dropdown** — a faithful dropdown needs an active-project model (a BEHAVIORAL feature, out of a theme pass) and a presentational-only dropdown would be a dishonest "fake selector"; the CSS-row fits the 44px bar with every name/count visible (tests green) → the real dropdown defers to Phase-7 project selection (orchestrator-routed).
- **DrawerStack = `:empty`-collapsing overlay** (not a reserved grid column) until the Brain drawer (Phase 8).

## Decisions explicitly NOT made (deferred)

- **The prototype's full feature surface** — the gate confirmed the FOUNDATION matches; the divergences (single-select switcher dropdown, two-column CommandCenter + right CommandRail/HIQ, command palette, Brain drawer, Gateway modal, Task inbox, PR review, richer sidebar nav) are **unbuilt Phase-7/8 features**, not theme defects — deliberately out of the 6.5 theme-pass scope.
- **Per-view internal-scroll polish** beyond the per-panel scroll seam (e.g. sticky table headers within a scrolling panel) — left as later polish.
- **The banner/safety overlay placement** the prototype uses (HIQ/Gateway overlays) — kept our approved dedicated 5-row banner grid-row adaptation (the app has distinct DegradedBanner/RecoveryBanner/AuditIntegrityAlert + a StatusBar region the prototype folds differently); flagged to the lead at the gate.

## TDD compliance

**Clean — non-`/tdd` visual slices, correct coverage path.** A theme/visual layer is TDD-exempt (`ui/LESSONS.md §10`: jsdom/tsc/oxlint don't parse+apply CSS, so a failing-test-first can't pin appearance). Coverage was the **rendered visual gate** (the project's non-deterministic-coverage path) + a deterministic wiring-guard (6.5a: import-order + `.sr-only` className). No test was written-after-impl to fake red→green; the acceptance was explicitly the rendered comparison. No violation.

## Reachability

- The three theme stylesheets are imported in `ui/src/main.tsx` (the production entry: `main.tsx → <Shell/>`) — global CSS over the already-wired Shell + views. The visual gate rendered every view (Command Center, Project Graph, Sessions, Settings/Usage) + the driven banner/safety surfaces from the production Shell tree.
- The `.sr-only` utility is reachable via `TopBar` back/forward accessible-name spans (the SR_ONLY→.sr-only swap is behavior-preserving; the reachability audit stayed green).
- No new tested-but-unwired gaps — CSS theming + inline-style lifts don't add reachability surfaces; the components were already wired in prior slices.

## Open follow-ups (already routed hot to the orchestrator)

- **ProjectSwitcher single-select dropdown → Phase-7** (needs the active-project model + selection wiring; the 6.5c compact-row is the theme-pass placeholder). _(orchestrator-routed)_
- **The 6.5 theme-pass lesson** _(orchestrator banks — see the recap's proposed framing; likely a new `ui/LESSONS.md §12` referencing §10)._
- **Phase-7/8 feature surfaces** vs the prototype (right CommandRail/HIQ, command palette, Brain drawer, Gateway modal, Task inbox, PR review, richer sidebar nav) — tracked by their phases, surfaced in the gate divergence table.
- **Final aesthetic sign-off** — the gate is LEAD-ACCEPTED (the lead independently reviewed the screenshots + concurs); the final production-aesthetic sign-off is flagged for the user on return.

## How to use what was built

The cockpit now renders the Graphite Arc theme automatically (the three `ui/src/theme/*.css` are imported in `main.tsx`). To re-run the visual gate: `node_modules/.bin/vite` (dev server) + a gstack-browser screenshot per view; for the prototype side-by-side, serve `NexusOps-ui-kit/` over HTTP (`python3 -m http.server`) — `file://` blocks the prototype's relative kit scripts. The banner/safety surfaces are conditional (clean by default); drive them via the Shell `safety` prop + connection state (or a throwaway preview entry as used in the gate).

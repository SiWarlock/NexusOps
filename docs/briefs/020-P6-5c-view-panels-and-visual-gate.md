# visual brief — view_panels_and_visual_gate (6.5c)

> **NON-`/tdd` visual slice + the standing VISUAL GATE.** Same flow (visual Step-2.5 → review → apply → rendered check), but this layer ENDS with the **full automated gstack-browser gate** (dev server vs the prototype, all views) — the acceptance for the whole 6.5 pass. Ground surfaces against the prototype's actual rendering (as in 6.5b), don't guess.

## Feature
The **6.5c view/component panel layer** — the 3rd + final theme layer. `ui/src/theme/components.css` themes the in-view panels + the banner/safety surfaces (which 6.5a/b left bare), lifts the remaining view inline styles into token classes, and applies the **two 6.5b-flagged refinements**. Then the **full automated visual gate** (dev server vs `ui_kits/control-plane/index.html`, across all views) confirms the Graphite Arc match — the acceptance for the entire 6.5 pass + the standing visual-verification gate going forward (`ui/LESSONS.md §10`).

## Use case + traceability
- **Task ID:** P6.5c (6.5 decomposition — `MVP_TASKS.md §6.5`). Depends on 6.5a (base) + 6.5b (shell chrome + grid), both landed.
- **Architecture sections:** `ARCHITECTURE.md §11.1` (semantic tokens only — re-hue), `§11`/`§11.3`/`§11.4` (the content views + surfaces), `§11.6` (a11y — don't regress the reachability audit / never-color-alone). Target = the prototype's rendered views (its panel chrome lives in `kit-shell.jsx` + the kit component CSS, NOT the index.html `<style>` — sample the actual surfaces, as you found at 6.5b).
- **Related context:** 6.5a `global.css` (base) + 6.5b `shell.css` (chrome/grid) — `components.css` is the 3rd `ui/src/theme/` stylesheet, imported after `shell.css`. The views + surfaces already carry class hooks from prior slices (mostly unstyled): CommandCenter (`.command-center*`), SessionsTable (`.sessions*`), ProjectGraph (`.project-graph*`), Settings/Usage (tablist + meters), and the banners/safety surfaces (`.degraded-banner*`, RecoveryBanner, `.audit-integrity-alert` + `data-treatment`/`data-severity`, `.hard-conflict-card` + `data-conflict-reason`). Lesson §3 (semantic tokens) · §10 (the gate is acceptance) · §11 (never-color-alone surfaces stay glyph+label+severity — theme them without collapsing to color-only) · §9 (don't regress reachability).

## Acceptance criteria
- [ ] **`ui/src/theme/components.css`** (NEW) themes the in-view panels with kit **semantic** tokens (surfaces/borders/`--sp-*` spacing/type scale/`--r-*` radii/`--elev-*`), grounded against the prototype's actual rendering:
  - **Content views:** `.command-center` (groups + items + group-titles + empty state), `.sessions` (dense table — rows/headers/zebra-or-border/`resume-mode` indicator), `.project-graph` (canvas + toolbar + the list/table fallback), Settings (tablist tabs + panels) + the Usage dashboard (meters/accuracy labels/credit-pool — preserve never-color-alone, forbidden #4/#5).
  - **Banner/safety surfaces:** `.degraded-banner` (+ `--update` variant), the RecoveryBanner, `.audit-integrity-alert` (severity-themed but glyph+label preserved), `.hard-conflict-card` (the parked-disabled resolution affordance styled as disabled). Severity colors via the kit `--attention-*`/`--danger-*`/`--critical-*`/`--caution-*` semantic families (never color ALONE — keep the glyph+label).
- [ ] **Remaining view inline styles lifted** → token classes (CommandCenter's `style={{}}`, any other view inline structural styles).
- [ ] **Refinement A — ProjectSwitcher compaction** (6.5b flag): compact the ProjectSwitcher to fit the 44px top bar (it currently renders a taller multi-row list spilling below the bar — the fixed-height grid surfaced it). A compact control (dropdown/select-style) within `--shell-topbar-h`.
- [ ] **Refinement B — per-panel scroll + sticky toolbar** (6.5b note ②): `.main { overflow:hidden }` + the content panels own their scroll regions (matching the prototype's per-panel scroll), with the `.content-switch` toolbar staying put.
- [ ] **Semantic tokens only** (§11.1); **a11y not regressed** — the reachability audit + never-color-alone tests stay green; focus rings intact. `/preflight` clean (suite 133+).
- [ ] **FULL VISUAL GATE** (the acceptance): run the Vite dev server + the prototype side-by-side in the gstack browser; compare **every view** (Command Center, Project Graph, Sessions, Settings/Usage) + the banner/safety surfaces (drive fixtures to render a conflict + an audit-integrity alert + a degraded banner). Capture a screenshot per view + a notes table (match / divergence + reason). Confirm the dark-cockpit/Geist/grid/chrome/panels MATCH the Graphite Arc prototype. **Report the comparison to me** (I relay to the lead for the best-effort production-match judgment; final visual sign-off is flagged for the user on return).

## Files expected to touch
**New:** `ui/src/theme/components.css`.
**Modified:** `ui/src/main.tsx` (import `components.css` after `shell.css`); the view components for inline-style lifts + the ProjectSwitcher compaction (`ui/src/shell/ProjectSwitcher.tsx` or wherever it lives) + `.main` overflow change (`shell.css`). Flag the full file list at the visual Step-2.5.

## Design questions (the visual Step-2.5)
1. **Surface grounding.** Same as 6.5b: sample the prototype's ACTUAL panel/surface colors (from `kit-shell.jsx` + kit component CSS) and match — don't guess. My default vote: **ground every panel surface against the prototype**; report divergences found.
2. **ProjectSwitcher compaction shape.** Default: a compact dropdown/select within the 44px bar. My default vote: **a compact trigger** (current-project label + caret) opening the list on demand — confirm vs an always-visible compact row.
3. **Per-panel scroll.** Default: `.main{overflow:hidden}` + each view panel scrolls internally (prototype behavior). My default vote: **per-panel scroll**. Confirm.
4. **Commit count / gate placement.** Default: **1 commit** for components.css + the lifts + the 2 refinements, then the gate as the final verification step (the gate doesn't add a commit — it's the acceptance). If the panels + the refinements feel large, a 2-split (view-panels → banner/safety-surfaces) is fine. My default vote: **1 commit + the gate**. Confirm.

## Cross-doc invariant impact
- **None.** Pure visual layer; no contract/model/enum. Preserve the never-color-alone + reachability invariants (don't regress §11.6).

## Estimated commit count
**1** (+ the gate as the acceptance step) — the view/component panel layer. Non-safety, non-/tdd. 2-split available if large.

## Lessons-logged candidates anticipated
- **The 6.5 theme-pass lesson** (banked at the pass close, in this slice's `/session-end` → my `/orchestrate-end`): the theme layers APPLY the kit semantic tokens via `ui/src/theme/{global,shell,components}.css` (semantic only, §11.1); GROUND surfaces against the prototype's actual rendering (don't guess — caught real divergences at 6.5b); the **visual gate is the acceptance** (it caught the 6.5a `*/`-in-CSS-comment bug + the 6.5b surface divergence — things tsc/oxlint/jsdom structurally can't); structural refactors preserve reachability/landmarks. Likely **extends `ui/LESSONS.md §10`** or a new §12.

## How to invoke
> Non-`/tdd`. Do NOT run `/tdd` or `/session-start`.
1. Read this brief + the prototype's rendered views (sample its panel surfaces).
2. **Send me the planned `components.css` scope + the 2-refinement plan + design-Q answers** (the visual Step-2.5). Wait for APPROVED./TWEAK:.
3. Apply → `/preflight` (a11y/reachability + never-color-alone green) → **the full visual gate** (all views + surfaces, dev server vs prototype) + per-view screenshots + a match/divergence notes table.
4. Report the gate comparison → I relay to the lead for the production-match judgment → commit message → land + `TaskUpdate` completed. This **closes the 6.5 theme pass + Phase 6** (modulo the parked 6.3d/e).

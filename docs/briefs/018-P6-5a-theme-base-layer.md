# visual brief — theme_base_layer (6.5a)

> **NON-`/tdd` visual slice.** No RED/GREEN test-first (a visual/theme layer is TDD-exempt — `ui/LESSONS.md §10`). Flow: read this → send me the planned CSS approach + design-Q answers (the **visual Step-2.5 equivalent**) → I review → apply → lightweight wiring-guard + `/preflight` + a **rendered sanity check** → report (Step-9 equivalent) → I give the commit message. Acceptance is **VISUAL**, not "green tests."

## Feature
The **6.5a base/global theme layer** — the FIRST of the 3 Graphite Arc theme layers (6.5a base → 6.5b shell-chrome+grid → 6.5c view-panels → visual gate). A global app stylesheet that **APPLIES** the kit's already-imported semantic tokens at the document root: `html,body` reset (dark `--surface-window` bg + Geist `--font-sans` + `--text-primary` + `overflow:hidden`), base type defaults, scrollbar chrome, and a global `.sr-only` utility (replacing TopBar's inline `SR_ONLY`). Closes the first part of the unstyled-layer Finding (kit tokens defined but never applied — `ui/LESSONS.md §10`).

## Use case + traceability
- **Task ID:** P6.5a (6.5 decomposition — see `MVP_TASKS.md §6.5`). Phase-6's last item; the visual layer.
- **Architecture sections:** `ARCHITECTURE.md §11.1` (the design-system / re-hue promise — reference the **SEMANTIC** token layer only, never primitives), `§11.6` (a11y — the `.sr-only` utility serves accessible names), `§11` (Graphite Arc visual layer). Target = the assembled prototype `NexusOps-ui-kit/ui_kits/control-plane/index.html` (the embedded `<style>` shows the body reset + scrollbar chrome to match).
- **Related context:** the recon synthesis (in `MVP_TASKS.md §6.5`): the kit `styles.css` is **already imported** in `ui/src/main.tsx` (all tokens at `:root`) — the gap is APPLYING them. The app currently has NO `body{}` reset (browser-default white/serif), NO scrollbar styling. `ui/src/a11y/focus.css` is the only global app CSS today (focus ring). Lesson §3 (kit consumption: tokens-via-`styles.css`, semantic layer only) · Lesson §6 (the accessible-NAME = a visually-hidden child INSIDE a closed-prop control — the `SR_ONLY`/`.sr-only` span) · Lesson §10 (visual gate; green ≠ looks right).

## Acceptance criteria
- [ ] **`ui/src/theme/global.css`** (NEW) applying the kit semantic tokens at the root:
  - `html, body` — `margin:0; height:100%; background: var(--surface-window); color: var(--text-primary); font-family: var(--font-sans); overflow: hidden;` (match the prototype's reset).
  - **base type defaults** — root `font-size`/`line-height` from the kit type scale (`--fs-body`/the body line-height); `-webkit-font-smoothing` if the prototype uses it.
  - **scrollbar chrome** — match the prototype (`--border-strong`/`--border-default` thumb on a `--surface-canvas`/`--surface-sunken` track; `::-webkit-scrollbar*`).
  - **global `.sr-only`** utility (the visually-hidden pattern — `position:absolute; width:1px; height:1px; clip-path/clip; overflow:hidden;` exactly preserving the current `SR_ONLY` behavior).
- [ ] **`ui/src/main.tsx`** imports `./theme/global.css` **after** the kit `styles.css` (tokens defined first, then applied) — confirm cascade order vs `focus.css`.
- [ ] **`SR_ONLY` → `.sr-only`** — replace TopBar's inline `SR_ONLY` constant with the global `.sr-only` class on the same visually-hidden accessible-name spans (Lesson §6); **the existing a11y/reachability tests must stay green** (the accessible names still resolve — this is a behavior-preserving refactor).
- [ ] **References the SEMANTIC token layer only** — never a primitive (`--n-900`/`--line-1`/`--ink-1`); the re-hue promise (§11.1).
- [ ] **Lightweight wiring-guard** (the deterministic part): a test asserting (a) `main.tsx` imports the theme stylesheet, and (b) the accessible-name spans carry `className="sr-only"` (DOM-checkable in jsdom). Per Lesson §10 this catches gross absence but does NOT replace the rendered check.
- [ ] `/preflight` clean (oxlint + tsc + test:run — full suite stays green, 131+).
- [ ] **Rendered sanity check** (the visual part): run the Vite dev server, confirm the app now renders **dark** (`--surface-window`) with **Geist** + styled scrollbars (vs the current white/serif). Capture a screenshot. *(The FULL visual gate vs the prototype runs at 6.5c — 6.5a's contribution is the base canvas.)*

## Files expected to touch
**New:** `ui/src/theme/global.css` (+ a small wiring-guard test, e.g. `ui/src/theme/global.test.tsx` or extend an existing a11y test).
**Modified:** `ui/src/main.tsx` (import), `ui/src/shell/TopBar.tsx` (`SR_ONLY` → `.sr-only`; remove the inline constant).
If the `SR_ONLY` constant is used beyond TopBar, flag the full list at the approach-review.

## Design questions (the visual Step-2.5 — answer before applying)
1. **`.sr-only` swap now vs defer.** Default: **swap now** — 6.5a is the natural home for the global utility (the §6.4e/Lesson-§6 inline `SR_ONLY` was explicitly flagged to move to "a global sr-only utility at the 6.5 theme pass"). My default vote: **swap now**, behavior-preserving (a11y tests guard it). Confirm vs deferring to 6.5c.
2. **CSS file location/structure.** Default: `ui/src/theme/global.css` (a new `theme/` dir; 6.5b `shell.css` + 6.5c `components.css` join it). My default vote: **`ui/src/theme/`**. Confirm.
3. **Cascade order in `main.tsx`.** Default: kit `styles.css` (tokens) → `theme/global.css` (applies) → `a11y/focus.css` (focus ring). My default vote: **that order** (global theme before focus, so focus-ring tokens win on `:focus-visible`). Confirm there's no conflict.
4. **Scrollbar scope.** Default: global `::-webkit-scrollbar` (the app is desktop/Tauri = Chromium, so webkit scrollbars are fine; no Firefox concern). My default vote: **webkit scrollbars globally**, matching the prototype. Confirm.

## Dependencies + sequencing
- **Depends on:** the kit `styles.css` tokens (already imported); the prototype reference (confirmed present).
- **Blocks:** 6.5b (shell chrome + grid) consumes the base layer; 6.5c then the visual gate.
- **Note:** the app will look **partially** themed after 6.5a (dark canvas + font, but the shell is still flexbox + unstyled chrome until 6.5b) — that's expected; the full match is judged at the 6.5c visual gate.

## Cross-doc invariant impact
- **None.** Pure visual layer; no contract/model/enum touched. No orchestrator doc rows.

## Estimated commit count
**1** — the base/global theme layer (one cohesive stylesheet + the import + the `.sr-only` refactor). Non-safety, non-/tdd.

## Lessons-logged candidates anticipated
- **Convention candidate** — the theme layers APPLY the kit semantic tokens via app stylesheets in `ui/src/theme/` (reference semantic only, §11.1); the visual gate (not green tests) is the acceptance. Likely banks as one 6.5 lesson at the theme-pass close (or extends Lesson §10).

## How to invoke
> Session already oriented — do NOT `/session-start`. This is **non-`/tdd`**: do NOT run `/tdd`.
1. Read this brief + the prototype `<style>` block (the visual target) + the kit `surfaces.css`/`typography.css` semantic tokens.
2. **Send me the planned `global.css` approach** (the body reset + scrollbar + `.sr-only` CSS, the `main.tsx` import order, the `SR_ONLY`→`.sr-only` swap scope) + your answers to the 4 design questions — the **visual Step-2.5**. Wait for my review (APPROVED./TWEAK:).
3. Apply → wiring-guard + `/preflight` → run the Vite dev server + a rendered sanity check (dark + Geist + scrollbars) + screenshot.
4. Report (Step-9 equivalent: files, suite count, the screenshot/rendered result, any flags) → I give the commit message → land + `TaskUpdate` completed + wake me for 6.5b.

# /tdd brief — a11y_focus_motion

## Feature
The **§11.6 accessibility MUSTs that are daemon-independent**: a **global `:focus-visible` ring** on every interactive control (kit tokens exist — `--focus-ring`/`--ring-w`/`--ring-offset` — but are **unapplied** in `src` today) and **reduced-motion** support (a `useReducedMotion()` hook for JS-gated motion + verifying the kit's global `@media (prefers-reduced-motion: reduce)` guard is wired). The deterministic core is the **keyboard-reachability audit** (every interactive control is focusable — the substance the ring decorates) + the `useReducedMotion` hook. First reordered slice under **Decision C** (the daemon-independent Phase-6 a11y merge-gate). First sub-slice of 6.4.

> **`drag→non-drag` is NOT in this slice** — the named drag surfaces (task-chip overflow, Dispatch-dialog) are intent-coupled + don't exist yet; that MUST lands with them at daemon-1.5 (forbidden #5 still pins the rule). The Project Graph list/table fallback + keyboard surface MUSTs already shipped in 6.3b.

## Use case + traceability
- **Task ID:** P6.4a (Decision-C decomposition of 6.4: **6.4a a11y MUSTs** → 6.4b Usage → 6.4c Settings display → 6.4d Survival/recovery display → accessible-names + "checking" banner; intent-coupled controls + ExecutionProfile tab parked/gated).
- **Architecture sections:** `ARCHITECTURE.md §11.6` (**LOCKED — PRD §14.8 MUST; tested §14** — global `:focus-visible` ring on every interactive control; reduced-motion), `§11.1` (never-color-alone — motion is one channel; kit motion tokens), `§14` (frontend a11y tests are a merge-gate).
- **Related context:** the kit `tokens/motion.css` (already ships the `@media (prefers-reduced-motion)` guard + `--dur-*`/`--ease-*` + `cp-live-pulse`/`cp-attention-beacon` keyframes) + `tokens/space.css` (`--ring-w: 2px`, `--ring-offset: 1px`) + `tokens/surfaces.css` (`--focus-ring`); imported via the kit `styles.css` in `main.tsx` (Lesson §3). Interactive controls to cover (all of `src`): the content-view switch (CC/Graph/Sessions), the Graph|List toggle, the Sessions `<button>`-in-`<th>` sort headers, sidebar items, `DegradedBanner` Retry/Repair, `ConnectionIndicator`, TopBar/ProjectSwitcher controls. Deterministic core = the reachability audit + the hook (pure); the ring CSS is the styling applied on top.

## Acceptance criteria
- [ ] A **global focus-visible stylesheet** (`ui/src/a11y/focus.css`) applies the kit ring tokens (`outline: var(--ring-w) solid var(--focus-ring); outline-offset: var(--ring-offset)`) to **every interactive control** via a `:where(button, a[href], [role="button"], [tabindex]:not([tabindex="-1"]), input, select, summary):focus-visible` rule (global, not per-component), imported in `main.tsx`.
- [ ] **Every interactive control in the rendered shell is keyboard-focusable** (semantic element or proper role; never `tabindex="-1"` on an actionable control; no `div onClick` actionables) — the reachability the ring decorates. Pinned by an audit test over the rendered Shell.
- [ ] `useReducedMotion()` (`ui/src/a11y/useReducedMotion.ts`) reads `matchMedia('(prefers-reduced-motion: reduce)')`, returns the current value, subscribes to changes (cleanup on unmount), and **defaults to `false` (motion allowed)** when `matchMedia` is unavailable (SSR/jsdom-without-mock).
- [ ] The kit's global reduced-motion guard is **verified wired** — `motion.css` is in the `styles.css` import chain (Lesson §3), so all animation/transition is auto-suppressed under `prefers-reduced-motion` (no new app-level guard needed; pin that it's present).
- [ ] **No new motion added** in this slice (applying `cp-live-pulse`/`cp-attention-beacon` to surfaces is a separate enhancement); `/preflight` clean.
- [ ] **Reachable from** `main.tsx` (focus.css import) + the hook is reachable-by its first JS-motion consumer (tracked — like 6.1c's `canSubmitIntent`; the hook + guard exist, the animated consumers land later). Confirm at Step 7.5.

## Wiring / entry point (Step 7.5)
`main.tsx` imports `a11y/focus.css` (global ring, reachable on every render) + the kit `styles.css` carries the reduced-motion guard (`motion.css`). `useReducedMotion()` is a utility reachable-by future animated components (no animated consumer yet — track it, don't claim a false wire). Confirm at Step 7.5: focus.css is in the import graph + the reachability audit drives the real shell.

## Files expected to touch
**New:** `ui/src/a11y/focus.css`, `ui/src/a11y/useReducedMotion.ts`, `ui/src/a11y/useReducedMotion.test.ts`, `ui/src/a11y/reachability.test.tsx` (the keyboard-reachability audit over `<Shell/>`).
**Modified:** `ui/src/main.tsx` (import `a11y/focus.css`; confirm `styles.css`/`motion.css` already imported). If the audit finds a non-focusable actionable control, fix it in its source component (flag which at Step 2.5).
Flag anything beyond at Step 2.5.

## RED test outline (Step 2)

**`a11y/useReducedMotion.test.ts`:**
1. **`returns_true_when_reduce_preferred`** — with `matchMedia('(prefers-reduced-motion: reduce)')` mocked to `matches:true`, the hook returns `true`. **[load-bearing]**
2. **`returns_false_when_no_preference`** — `matches:false` → `false`.
3. **`defaults_false_when_matchmedia_unavailable`** — no `matchMedia` (jsdom default) → `false` (motion allowed), no throw.
4. **`updates_on_preference_change`** — a `change` event on the media query updates the returned value; listener removed on unmount (no leak).

**`a11y/reachability.test.tsx` (jsdom, renders `<Shell/>`):**
5. **`every_interactive_control_is_keyboard_focusable`** — query the rendered shell for actionable elements; each is a focusable semantic element/role and **none is `tabindex="-1"` or a non-focusable `div`/`span` with a click handler. **[load-bearing — §11.6]**
6. **`focus_stylesheet_is_imported`** — assert `a11y/focus.css` is wired into the app entry (import-graph/structural check) so the global `:focus-visible` ring is present (jsdom can't compute `:focus-visible`; pin the wiring, not the pixel).
7. **`reduced_motion_guard_present`** — assert the kit reduced-motion guard is in the loaded styles (the `@media (prefers-reduced-motion: reduce)` rule from `motion.css` is in the import chain) — the merge-gate that suppresses motion.

## Cross-doc invariant impact
- **Model field changes:** **none.** a11y is UI render policy; uses **existing kit tokens** (`--focus-ring`/`--ring-w`/`--ring-offset`/`motion.css`) — no new token, no contract change. **Orchestrator rows:** none expected.
- **Note:** if the reachability audit forces a control from `div onClick` → `button`, that's a same-slice fix (improves the existing control), not a cross-doc change.

## Things to flag at Step 2.5
1. **Reachability audit approach.** Default vote: render `<Shell/>` (default CC view) + assert all actionable elements are keyboard-focusable, asserting representative controls (view-switch, Graph|List toggle, sort headers, sidebar items, banner buttons). Confirm vs per-control unit tests. _(If a control is currently non-focusable, fixing it is in-scope — name which at Step 2.5.)_
2. **Reduced-motion = hook + guard-verify, NOT adding motion.** Default vote: ship `useReducedMotion()` + pin the kit guard is wired; do **not** apply `cp-live-pulse`/`cp-attention-beacon` to surfaces (that's a later enhancement — the guard auto-suppresses it when it lands). Confirm.
3. **Global focus-ring mechanism.** Default vote: one global `a11y/focus.css` `:where(...):focus-visible` rule using kit tokens, imported in `main.tsx` — NOT per-component focus styles. Confirm the global-CSS approach.
4. **`useReducedMotion` matchMedia default.** Default vote: `false` (motion allowed) when `matchMedia` is unavailable — reduced-motion is an explicit OS opt-in; absence ≠ reduce. Confirm.

## Dependencies + sequencing
- **Depends on:** the kit token integration (Lesson §3, 6.1b) + the shell + 6.3 views (the controls being made reachable/ring-bearing). No daemon dependency (Decision C — daemon-independent).
- **Blocks:** the Phase-6 a11y merge-gate (acceptance criteria 6: "Accessibility MUSTs pass — focus ring, …"); future animated components (consume `useReducedMotion`).
- **Decision-C context:** this is the first reordered daemon-independent slice; 6.3d/6.3e + the intent seam stay parked (Carry-forward).

## Estimated commit count
**1** — a cohesive a11y-foundation slice: the global focus-visible ring + the `useReducedMotion` hook + the reachability/guard audits. Small, same area (`ui/src/a11y/`), one logical unit, **no safety invariant** (a11y is not a §15 invariant) → **security-reviewer NOT required**; **code-quality every-slice**. (If the reachability audit surfaces multiple controls needing focusability fixes, that could grow — flag at Step 2.5; default is 1.)

## Lessons-logged candidates anticipated
- **Convention candidate** — the focus ring is applied **globally via one `a11y/focus.css` `:where(...):focus-visible` rule** using kit tokens (never per-component); reduced-motion is the **kit's global guard** + a `useReducedMotion()` hook for JS-gated motion; **every interactive control must be keyboard-reachable** (the ring decorates reachability). Candidate for a new `ui/LESSONS.md` entry (the a11y-foundation pattern 6.4b–d + future controls follow).
- **Future TODO — operational** — when motion lands (live-pulse / attention-beacon on the AttentionMarker, skeletons, overlay entrances), it's auto-suppressed by the kit guard; gate any **JS-driven** motion via `useReducedMotion`.
- **Architecture-doc note candidate** — none expected; this is exactly §11.6.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd a11y_focus_motion`.
1. Read this brief; Q1 (reachability audit approach) + Q2 (no-new-motion) are the ones to confirm at Step 2.5.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN.
3. Step 7.5 — name `main.tsx → a11y/focus.css` (global ring) + the reachability audit driving `<Shell/>` as the entry; track `useReducedMotion`'s future animated consumer (no false wire).
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me.

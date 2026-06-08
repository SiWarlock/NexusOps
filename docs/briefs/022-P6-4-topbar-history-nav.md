# /tdd brief — topbar_view_history_nav

## Feature
Wire the TopBar back/forward controls — currently **named-but-inert** (a real dead-affordance gap) — to a real content-view navigation history: a pure history model (back/forward/navigate with browser-style forward-truncation) that the Shell drives and the TopBar buttons operate, with each button **disabled when its direction is unavailable** so a named control is never a dead click.

## Use case + traceability
- **Task ID:** P6.4 (TopBar back/forward history nav — the §6.4 inert-control closure; origin 2026-06-08 P6.4e)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (TopBar nav model — back/forward + Settings reached here, not the view-switch; human-confirmed 2026-06-08), `§11` (shell as projection-driven client), `§11.6` (a11y MUSTs — a control is reachable + does what its name says).
- **Related context:** This is the highest-value item in the FINAL daemon-independent polish round (lead-directed, user away). The TopBar back/forward buttons today (`ui/src/shell/TopBar.tsx:31-38`) carry visually-hidden accessible names ("Back"/"Forward") from P6.4e but **no `onClick`** — history nav was never wired. The Shell holds the content view as a single `useState<"command"|"graph"|"sessions"|"settings">` (`Shell.tsx:95-97`); every nav is a `setContentView(...)`. This slice replaces that with a history model so back/forward become live. Sibling pattern: **active-project** (`active-project.ts`, Lesson §13) — UI selection/scope state over the frozen projection, a pure model + a thin React wrapper. View history is the same shape (pure UI state, NO daemon dep, NO provisional contract, NOT a Gateway intent → no `canSubmitIntent` gate).

## Acceptance criteria (what "done" means)
- [ ] A pure history model `viewHistoryReducer` (in `ui/src/shell/view-history.ts`) with browser-style semantics:
  - initial state `{ stack: ["command"], cursor: 0 }` → `current === "command"`, `canBack === false`, `canForward === false`
  - `navigate(view)` to a **different** view truncates any forward entries, pushes, advances the cursor → `canBack === true`, `canForward === false`
  - `navigate(view)` to the **current** view is a **no-op** (no duplicate history entry — collapse)
  - `back()` decrements the cursor (only when `canBack`); `forward()` increments (only when `canForward`); neither mutates the stack
  - after `back()`, `canForward === true`; a subsequent `navigate(newView)` **truncates** the stranded forward entry (classic browser forward-discard)
  - `back()` at the start and `forward()` at the end are no-ops (idempotent, never throw / never out-of-bounds)
- [ ] A `useViewHistory()` hook wrapping the reducer (mirrors `active-project.ts`'s pure-model-plus-thin-wrapper) exposing `{ current, canBack, canForward, navigate, back, forward }`.
- [ ] `Shell.tsx` drives content from `useViewHistory()` — `contentView = current`; every existing `setContentView(x)` call site (the 3 content-switch buttons + `onOpenSettings`) becomes `navigate(x)`; back/forward + can-flags pass to TopBar.
- [ ] `TopBar.tsx` back/forward buttons call `onBack`/`onForward` and are `disabled={!canBack}` / `disabled={!canForward}` (kit `Button` supports `disabled` natively — typed in `ButtonProps`).
- [ ] All unit tests in `ui/src/shell/view-history.test.ts` pass.
- [ ] Integration: `ui/src/shell/Shell.test.tsx` proves a full nav→back→forward round-trip re-renders the right content surface; `TopBar.test.tsx` proves the buttons are disabled at the history boundaries and call their handlers when enabled.
- [ ] `/preflight` clean (oxlint + tsc + vitest).

## Wiring / entry point (Step 7.5)
`main.tsx → <Shell/> → useViewHistory() → TopBar (onBack/onForward/canBack/canForward) + the content-switch (navigate)`. The production entry points being made live are the two TopBar `<Button>`s inside `<nav aria-label="History">`. Confirm `back()`/`forward()` are reached from those buttons (not only from tests), and that `navigate` flows from all real view-switch call sites.

## Files expected to touch
**New:**
- `ui/src/shell/view-history.ts` — the pure `viewHistoryReducer` + `ViewName` type + `useViewHistory()` hook.
- `ui/src/shell/view-history.test.ts` — reducer unit tests (the deterministic core).

**Modified:**
- `ui/src/shell/Shell.tsx` — swap the `contentView` `useState` for `useViewHistory()`; `navigate(...)` at the 4 call sites (3 content-switch buttons + `onOpenSettings`); pass `onBack`/`onForward`/`canBack`/`canForward` to `<TopBar>`.
- `ui/src/shell/TopBar.tsx` — add `onBack`/`onForward`/`canBack`/`canForward` props; wire the two `<Button>`s (`onClick` + `disabled`).
- `ui/src/shell/TopBar.test.tsx` — back/forward wiring + disabled-at-boundary tests.
- `ui/src/shell/Shell.test.tsx` — nav→back→forward integration through the real Shell.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `ui/src/shell/view-history.test.ts` (pure reducer — the test-first core):
1. **`history_initial_is_command_no_back_no_forward`** — Asserts: `current==="command"`, `!canBack`, `!canForward`. Why: §11.2 default content view + a fresh history has no neighbors.
2. **`navigate_to_new_view_enables_back`** — Asserts: after `navigate("graph")`, `current==="graph"`, `canBack`, `!canForward`. Why: forward nav pushes a back-step.
3. **`navigate_to_current_view_is_noop`** — Asserts: `navigate("command")` from `command` leaves `{stack,cursor}` unchanged. Why: collapse duplicate entries (Step-2.5 Q2).
4. **`back_then_forward_round_trips`** — Asserts: `navigate("graph")` → `back()` gives `current==="command"`+`canForward`; `forward()` returns `current==="graph"`. Why: back/forward move the cursor, never the stack.
5. **`navigate_after_back_truncates_forward`** — Asserts: `navigate("graph")` → `back()` → `navigate("sessions")` yields `current==="sessions"`, `!canForward`, and `forward()` is a no-op. Why: classic browser forward-discard.
6. **`back_at_start_and_forward_at_end_are_noops`** — Asserts: `back()` on initial state and `forward()` at the tip return the same state (no throw, no out-of-bounds cursor). Why: idempotent boundaries.
7. **`settings_participates_in_history`** — Asserts: `navigate("settings")` → `back()` returns to the prior content view. Why: Settings is a content view in the history even though it's *reached* via the TopBar, not the view-switch (Step-2.5 Q4).

Tests in `ui/src/shell/TopBar.test.tsx`:
8. **`topbar_back_forward_disabled_at_boundaries`** — Asserts: with `canBack=false`/`canForward=false` the two buttons are `disabled`; with both true they are enabled. Why: a named control is never a dead click (AC + §11.6).
9. **`topbar_back_forward_invoke_handlers`** — Asserts: clicking enabled Back/Forward calls `onBack`/`onForward`. Why: the wiring is real.

Tests in `ui/src/shell/Shell.test.tsx`:
10. **`shell_history_nav_round_trips_content`** — Asserts: navigate Command→Graph (content re-renders), TopBar Back returns the Command surface, TopBar Forward returns Graph. Why: end-to-end reachability through the real Shell.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. View history is pure UI state — no frozen/provisional contract, no `shared/` touch (Lesson §13 family).
- **Orchestrator doc rows to write hot (Step 9 routing):** none. (If the implementer judges the view-history-as-UI-state pattern worth banking as a convention, flag it at Step 9 — orchestrator decides whether it extends Lesson §13 or stands alone.)

## Things to flag at Step 2.5
1. **Unavailable back/forward — `disabled` or enabled-noop?** Options: (a) `disabled` when the direction is unavailable; (b) always enabled, no-op when unavailable. My default vote: **(a) disabled** — the kit `Button` supports `disabled` natively (typed `ButtonProps.disabled` → native `<button disabled>`); an enabled-but-noop button is still a dead affordance, which is exactly the gap this slice closes. Matches real browser chrome.
2. **Navigating to the current view — push a duplicate or collapse?** Options: (a) collapse (no push if `view===current`); (b) push every nav. My default vote: **(a) collapse** — clicking the already-active view-switch tab shouldn't create a back-step to itself; the content-switch buttons read as tabs, and a self-referential history entry is confusing UX.
3. **Model placement / shape.** My default vote: **a pure `viewHistoryReducer` is the deterministic test-first core; `useViewHistory()` wraps it with `useReducer`** — mirrors `active-project.ts` (pure model + thin React wrapper). Unit tests target the reducer directly. Flag if you'd prefer a different decomposition.
4. **Does Settings participate in the history?** My default vote: **yes** — all 4 content views are uniform history entries (`command`→`settings`→Back returns to `command`). The §11.2 nav model governs *how Settings is reached* (TopBar, not the view-switch), not whether it's a history entry. Flag if §11.2 should special-case it.
5. **Keyboard shortcuts (e.g. Cmd-[ / Cmd-])?** My default vote: **out of scope** — this slice wires the visible controls only; global history keybindings are a separate a11y/UX item if wanted later.

## Dependencies + sequencing
- **Depends on:** P6.4e TopBar nav rewire + accessible names (`823d16e`, landed) — the buttons + names already exist; this slice makes them live. The active-project context (`86727ec`, landed) is unaffected (view history and active-project are orthogonal UI state).
- **Blocks:** nothing hard. It's slice 1 of the polish round (sequenced highest-value-first); the remaining polish items (tablist roving + §9 audit, sessions-table filtering, sidebar resume-mode indicator, ProjectSwitcher dropdown widget) are independent and follow.

## Estimated commit count
**1.** One focused logical unit — "wire TopBar history nav." The internal RED→GREEN ordering is natural (reducer tests + reducer first, then the Shell/TopBar wiring tests + wiring), but it's one cohesive slice, same code area (`ui/src/shell/`), well under the bundle size cap, touches **no safety invariant** → one Step-10 commit. Not a multi-commit layer-driven slice (Lesson §7 doesn't apply).

## Lessons-logged candidates anticipated
- **Convention candidate** — view-navigation history is **UI state** (a pure reducer + thin hook) over the content-view selection, the same family as active-project (Lesson §13: UI selection/scope over a frozen projection — no daemon dep, no provisional contract, no intent gate); back/forward are **disabled-when-unavailable** so a named control is never a dead click. Likely extends §13 rather than a new lesson — orchestrator decides at Step 9.
- **Architecture-doc note candidate** — if the §11.2 history semantics (forward-truncation, Settings-as-history-entry, disabled-at-boundary) warrant a one-line pin in `ARCHITECTURE.md §11.2`, flag it.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5."
2. **Run `/tdd topbar_view_history_nav`** in the implementer session (already oriented this round — no `/session-start`).
3. **Step 0 (Restate)** — confirm against the Feature line.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — send the tight test-design write-up (one `Asserts: <invariant> (§anchor)` line per test) + answers to the 5 design questions; wait for `APPROVED.` / `TWEAK:` / `ADD:`.
6. **Step 9** — categorized flags + ship-ask; surface anything beyond the anticipated lessons candidates.

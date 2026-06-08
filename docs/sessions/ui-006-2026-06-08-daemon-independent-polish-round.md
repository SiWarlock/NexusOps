# ui-006 — Daemon-independent polish round (dead-affordance / a11y closures)

- **Date:** 2026-06-08
- **Phase:** Phase 6 COMPLETE — this is the FINAL daemon-independent polish round (post-Phase-6, ui track)
- **Track:** `track/ui`
- **Predecessor:** [ui-005 — active-project selection](ui-005-2026-06-08-active-project-selection.md)
- **Successor:** _(none yet — ui track PAUSES here at its parallel-track limit)_
- **Round commits:** `25ef3ed` · `128714e` · `9a7945f` · `64ffa42` · `0af44a5` + `9f2c91f` (slice 5 = 2 commits)
- **Tests:** 142 → **193 green** (+51 across the round); tsc + oxlint clean throughout.

---

## Why this session existed

Phase 6 was complete (logic + Graphite Arc theme + visual gate) and the active-project selection (P7.3 fwd) had landed. Everything else on the ui track is **daemon/integration-gated** (6.3d/e + the mutation/intent seam; Phase 7/8 PR-Review/Task-Inbox/Brain/Gateway). The lead directed a final **daemon-independent** round to close the remaining dead-affordance / a11y gaps before pausing the track: five production-grade closures, all fixture-buildable against the frozen `shared/` 0.5.0 + `MockGatewayPort` + fixtures + `NexusOps-ui-kit`.

## What was built — 5 slices

### Slice 1 — TopBar back/forward history nav (`25ef3ed`, brief 022)
Wired the named-but-INERT TopBar back/forward controls to a real content-view history.
- **New:** `ui/src/shell/view-history.ts` — pure `viewHistoryReducer` + selectors (`currentView`/`canGoBack`/`canGoForward`) + `useViewHistory()` hook. Browser semantics: navigate pushes + truncates-forward, collapse-on-same (no-op preserves the forward entry), back/forward move only the cursor, idempotent boundaries. `view-history.test.ts` (8 reducer units).
- **Modified:** `Shell.tsx` (`useState<contentView>` → `useViewHistory()`; 4 nav call sites → `navigate(...)`), `TopBar.tsx` (required `onBack`/`onForward`/`canBack`/`canForward`; buttons wired `onClick` + `disabled={!can…}`), `TopBar.test.tsx` (+2), `Shell.test.tsx` (+1 round-trip), `theme/global.test.tsx` (TopBar render updated).

### Slice 2 — Settings tablist roving + roving-aware §9 audit (`128714e`, brief 023)
WAI-ARIA arrow-key roving on the Settings tablist + taught the §9 reachability audit to be roving-aware. Atomic (roving makes inactive tabs `tabIndex=-1`, which breaks the old audit unless taught simultaneously).
- **New:** `ui/src/views/settings/roving.ts` — pure `nextTabIndex` + `isRovingKey` (`roving.test.ts`, 4 units). `ui/src/a11y/reachability.ts` — `auditFocusable` extracted from the test file (vitest-free; throws a plain Error); roving-aware (a `tabIndex=-1` tab is reachable iff its tablist has exactly one tabstop) + `role="tabpanel"` coverage. `reachability.classify.test.tsx` (3 classifier units).
- **Modified:** `Settings.tsx` (roving tabIndex + tab refs + `onKeyDown` reading focus at event time → activate + focus; `preventDefault`), `Settings.test.tsx` (+3), `reachability.test.tsx` (imports `auditFocusable`).

### Slice 3 — Sessions-table status + free-text filtering (`9a7945f`, brief 024)
Status + free-text filtering on the (sortable) Sessions table.
- **Modified:** `views/sessions/model.ts` — `SessionsFilter` + `filterSessionRows` (status-exact AND ci name|project substring, fields **OR'd independently** — never a joined string, which would match across the field boundary; identity early-return) + `distinctStatuses` (deduped, alphabetical, from the **unfiltered** set). `model.test.ts` (+7), `SessionsTable.tsx` (filter state; pipeline build→filter→sort; labeled `<select>` + `<input type="search">`; filtered-empty + Clear distinct from truly-empty), `SessionsTable.test.tsx` (+4).

### Slice 4 — Sidebar resume-mode indicator (`64ffa42`, brief 025)
Session resume-mode (▶ Resumed / ⟳ Replayed) on its sidebar item via an **id-keyed side map**, WITHOUT widening the shared `ProjectionItem` (Lesson §8).
- **Modified:** `recovery/model.ts` — `resumeModesBySessionId(sessions): Record<string, ResumeMode>` (only sessions with a mode). `model.test.ts` (+3), `Sidebar.tsx` (optional `resumeModes` prop; `SidebarResumeIndicator` reusing `describeResumeMode`; glyph `aria-hidden` + `.sr-only` label; lookup guarded to `machine === "Session"`), `Sidebar.test.tsx` (+4), `Shell.tsx` (builds the map, passes it), `Shell.test.tsx` (+1 live-path).

### Slice 5 — ProjectSwitcher dropdown-popover (`0af44a5` L1 + `9f2c91f` L2, brief 026)
Flat `aria-pressed` row → WAI-ARIA dropdown-popover (2-commit layer-driven, Lesson §7).
- **New:** `ui/src/a11y/roving.ts` — `nextTabIndex`/`isRovingKey` **re-homed** from `views/settings/` (byte-identical at L1; the slice-2 roving tests moved to `a11y/roving.test.ts`), then **L2 added an `orientation` param** (default `"horizontal"` → Settings unchanged; listbox passes `"vertical"`).
- **Deleted:** `views/settings/roving.ts` + `roving.test.ts` (renamed into `a11y/`).
- **Modified:** `ProjectSwitcher.tsx` — trigger (`aria-haspopup=listbox`/`aria-expanded`, active name + caret, disabled "No project" at zero projects) + `role=listbox`/`option` popover + roving tabindex + keyboard (Arrow/Home/End/Enter/Space/Escape) + open-focuses-active (`useLayoutEffect` on the open transition) + focus-return-to-trigger + click-outside-close; handler reads the focused index from `document.activeElement` at event time (§9). `ProjectSwitcher.test.tsx` (rewritten: 4 L1 + 4 L2), `Settings.tsx` (import re-pointed), `a11y/reachability.ts` (swept set + `role="option"`; `isRovingMember` generalized to {tab→tablist, option→listbox}), `reachability.classify.test.tsx` (+2), `reachability.test.tsx` (+open-dropdown sweep), `Shell.test.tsx` (2 tests adapted to select via the dropdown).

## Decisions made

- **All UI state/nav/filter/selection is §13 family** — pure UI state over the frozen projections (or local view selection): no daemon dep, no provisional contract, no Gateway intent, no `canSubmitIntent` gate. Ships without the parked intent seam.
- **Wire-or-disable** (§11.6) — a named control is never a dead click: wire it (history nav, dropdown select) or `disable` it (back/forward at a boundary; the zero-projects trigger).
- **Shared a11y roving primitive** — `nextTabIndex` re-homed to `a11y/roving.ts` (co-located with `reachability.ts`), made **orientation-aware** (default horizontal keeps tablist callers byte-identical; the listbox passes vertical). Two real orientations justify the param (not speculative).
- **§9 audit generalized** — `auditFocusable` is roving-aware for **{tab-in-tablist, option-in-listbox}** (one-tabstop invariant enforced for both) + covers visible `role="tabpanel"`; throws a plain Error (vitest-free module).
- **Side map over item-widening** (Lesson §8) — session-specific display data (resume_mode) rides an id-keyed side-map prop, not a new field on the shared `ProjectionItem`.
- **Filter semantics** — compose **filter → sort** (attention order holds within the subset); `distinctStatuses` from the **unfiltered** set (stable options); OR text fields **independently** (a joined-string match would falsely match across the field boundary).
- **Listbox manual activation** — arrows move focus only; Enter/Space selects; Escape cancels; open focuses the active option; Enter/Escape return focus to the trigger; click-outside closes (focus follows the click, not forced back to the trigger — per brief Q4).

## Decisions explicitly NOT made (deferred)

- **History keyboard shortcuts** (Cmd-[ / Cmd-]) — out of scope; a separate a11y/UX item if wanted later.
- **`isRovingMember` nested-container hardening** — `closest()` + descendant query could over-count if composites were ever nested; no nested tablist/listbox exists today, so deferred (YAGNI; flagged as a Future TODO).
- **Within-table project facet** on the Sessions filter — redundant with the active-project switcher (the Shell pre-filters rows); status + text only. Re-add only if a real unscoped need surfaces.
- **The daemon-gated remainder** — 6.3d/e (Session Terminal + Code/Diff review) + the mutation/intent seam; Phase 7/8 (PR Review, Task Inbox, Brain drawer, Gateway modal); provisional→generated reconcile; ExecutionProfile 0.5b. All await daemon contracts.

## TDD compliance

**Clean — no violations.** Every slice ran strict RED → Step-2.5 design review → GREEN, with RED confirmed for the right reason each time (missing module / unwired prop / unhandled orientation / un-swept role). Slice 5 was 2-commit layer-driven (Lesson §7): each layer its own RED→GREEN→commit; L1 byte-identical roving move, L2 the orientation param + keyboard. Review-driven coverage tests were added where the code-quality reviewer found gaps (these pin already-correct behavior an external reviewer flagged — not back-filled implementation).

## Cross-doc invariant audit

**No drift.** The two models in the `ui/CLAUDE.md` cross-doc table (Generated Zod contract layer §5.0/§5.1; Status→attention-rank descriptor table §11.3) were untouched this round. All five slices are §8/§9/§13 family — pure UI state/nav/filter/a11y with **no `shared/` / contract touch and no model field add/remove/rename**. No `ARCHITECTURE.md` field-level edit was required. (The orchestrator's hot edits this round are Lesson-index + §11.2/§11.6 prose notes, not contract-model changes.)

## Reachability (Step 7.5 — confirmed)

- **view-history / TopBar nav** — `main.tsx → Shell → useViewHistory() (Shell.tsx) → TopBar onBack/onForward (disabled at boundary) + content-switch navigate`. Shell test #10 drives the real path.
- **Settings tablist roving** — `main.tsx → Shell → TopBar(onOpenSettings → navigate("settings")) → Settings tablist onKeyDown`. The whole-Shell §9 sweep audits the live roving tablist.
- **Sessions filtering** — `main.tsx → Shell → content-switch "Sessions" → SessionsTable filter controls → filterSessionRows → sortSessionRows`. The §9 sweep audits the live select/input.
- **Sidebar resume-mode** — `main.tsx → Shell → resumeModesBySessionId(data.sessions) → <Sidebar resumeModes={…}/>`. Shell test asserts the indicator on the live path.
- **ProjectSwitcher dropdown** — `main.tsx → Shell → TopBar → ProjectSwitcher (trigger → listbox → option select → setActiveProject)`. The §9 open-dropdown sweep genuinely audits the roving listbox; the Shell active-project test selects via the dropdown + re-roots the graph / filters Sessions.

**No tested-but-unwired gaps.** The slice-5 roving re-home updated the Settings import (tests green) — no wiring removed.

## Open follow-ups (Step-9 categorized — already routed hot by the orchestrator)

- **Convention candidates (banked by the orchestrator):** Lesson **§13** extended (view-history nav as UI state; wire-or-disable). Lesson **§9** extended (roving tabindex + roving-aware/`role="tabpanel"` audit → then the dropdown/popover composite + orientation-aware roving + audit generalization to {tab-in-tablist, option-in-listbox}). Lesson **§8** extended (id-keyed side map for session-specific display data). Sessions-filter pattern noted under §13 (compose filter→sort; `distinctStatuses` from the unfiltered set; OR fields independently; filtered-empty ≠ truly-empty).
- **Architecture-doc notes (orchestrator):** §11.2 view-history semantics; §11.6 composite-widget roving + automatic/manual activation.
- **Future TODOs (out of scope):** history keyboard shortcuts (Cmd-[/]); `isRovingMember` nested-container hardening (theoretical, no nesting today); within-table project facet (redundant).
- **Cross-doc invariant changes:** none.
- **ui-track PAUSE:** the track pauses here at its parallel-track limit — every remaining ui slice converges on daemon/integration contracts (6.3d/e + intent seam; Phase 7/8; provisional→generated reconcile; 0.5b). The final 6.5 Graphite Arc aesthetic sign-off remains flagged for the user.

## How to use what was built

- **Roving a composite widget:** import `nextTabIndex`/`isRovingKey` from `a11y/roving.ts`; pass `"vertical"` for a listbox/menu, omit for a horizontal tablist. The §9 `auditFocusable` (`a11y/reachability.ts`) recognizes `{tab-in-tablist, option-in-listbox}` roving members automatically — keep composites to exactly one tabstop.
- **Surfacing per-entity display data on a shared-shape widget:** build an id-keyed side map (cf. `resumeModesBySessionId`) and pass it as a prop; do not widen `ProjectionItem` (§8).

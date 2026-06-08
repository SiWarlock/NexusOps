# ui-005 — active-project selection + view re-rooting (P7.3 fwd)

- **Date:** 2026-06-08
- **Phase:** Phase 7 (forward) — a daemon-independent UI-shell completion pulled forward (lead-directed measured continue); a single-slice round.
- **Predecessor session:** [ui-004](ui-004-2026-06-08-graphite-arc-theme-pass-and-visual-gate.md)
- **Successor session:** _(pending — fresh team after the auto-cycle; most remaining ui work is daemon-gated: parked 6.3d/e + the daemon-1.5 integration)_
- **Round commit:** `86727ec`. Suite **133 → 142 green**; tsc + oxlint clean.

## Why this session existed

Complete the previously-**inert** ProjectSwitcher (6.5c gave it a selectable-row shape but no selection; the Shell hardcoded the graph to `projects[0]`) and resolve the two lingering 6.3b gaps (the graph project-source Q3 + the zero-projects guard). A `/tdd` deterministic slice — pure UI selection state, **daemon-independent** (no provisional contract, no daemon dep, not a Gateway intent).

## What was built

### Files created
- `ui/src/shell/active-project.ts` — the UI active-project model: `defaultActiveProject(projects) → first|null`, `resolveActiveProject(projects, rawId)` (stale-id guard → default), `filterByActiveProject(rows, activeId)` (null = unscoped; unassigned rows excluded under a non-null active), and `ActiveProjectContext`/`ActiveProjectProvider`/`useActiveProject` (mirrors `ReadOnlyProvider` — createElement, default context, hook).
- `ui/src/shell/active-project.test.tsx` — model + context tests.
- `ui/src/shell/ProjectSwitcher.test.tsx` — single-select + never-color-alone tests.

### Files modified
- `ui/src/shell/ProjectSwitcher.tsx` — single-select: each project is a `<button aria-pressed>` calling `setActiveProject(id)` (via `useActiveProject`); the active project carries a "✓ Active" glyph+label (never color alone, §11.6) + a self-contained `aria-label` (counts spelled out, worded to avoid a `getByRole` name-collision with the view-switch buttons; the visible count glyphs are aria-hidden).
- `ui/src/shell/Shell.tsx` — `rawActiveProjectId` state + `activeProjectId = resolveActiveProject(data.projects, rawActiveProjectId)`; wraps the WHOLE shell (incl. TopBar) in `ActiveProjectProvider` so the switcher reads the context without prop-drilling; the graph re-roots (`projectId={activeProjectId ?? ""}`) + Sessions filters (`filterByActiveProject(data.sessions, activeProjectId)`); Command Center stays global.
- `ui/src/views/graph/ProjectGraph.tsx` — a distinct no-projects guard (`graph-no-project`) for `!projectId` (no active / no projects), separate from the per-project `graph-empty`.
- `ui/src/views/graph/ProjectGraph.test.tsx` + `ui/src/shell/Shell.test.tsx` — re-root + zero-guard + the Shell integration test.

## Decisions made

- **Active-project = UI selection state over the frozen projects projection** — NOT a daemon mutation / provisional contract / Gateway intent (a read/scope selection → no `canSubmitIntent` gate). Hence security-reviewer correctly skipped.
- **Context provider (mirrors `ReadOnlyProvider`), switcher reads it via `useActiveProject()`** — avoids prop-drilling through TopBar; the provider wraps the whole shell incl. TopBar so the deep switcher sees it.
- **View-filtering scope:** Graph re-roots + Sessions filters; **Command Center stays GLOBAL** (cross-project "needs my attention" triage — scoping it would hide cross-project attention).
- **Default scope = first-or-null** (`defaultActiveProject`); **stale-id re-scopes to default** (`resolveActiveProject`) so a selected-then-removed project doesn't root views at a ghost id.
- **Single-select via `<button aria-pressed>`** — consistent with the existing content-view switch (the same segmented-control pattern); the WAI-ARIA radiogroup/listbox is deferred with the dropdown widget.
- **`filterByActiveProject(rows, null) → all`** (unscoped); under a non-null active, an unassigned (`project_id`-less) row is excluded (belongs to no project).

## Decisions explicitly NOT made (deferred)

- **The prototype's dropdown-popover ProjectSwitcher widget** (trigger + caret + popover list + WAI-ARIA radiogroup/listbox + roving tabindex) — a presentation polish, deferred (origin: this slice Q3).
- **Per-project scoping for more views** as they land (Phase 7+).
- **A "clear selection / reset to default" affordance through the context** — `setActiveProject` only accepts a string; the Shell-side `resolveActiveProject` handles the stale-id fallback. Bounded; no consumer caches the raw id.
- **The Shell provider-wrap re-indentation** — cosmetic (the `<div className="shell">` subtree isn't re-indented under the new `ActiveProjectProvider`); no prettier in the gate, the boundary is comment-documented, a formatter pass normalizes it.

## TDD compliance

**Clean, with one honest note.** The slice was strictly test-first: tests written at Step 2, reviewed at Step 2.5, RED confirmed for the right reason (missing `./active-project` module, missing `graph-no-project` guard, switcher not selectable / graph not re-rooting), then GREEN. **Minor deviation:** the code-quality-review robustness add `resolveActiveProject` had its test (`resolve_active_project_guards_stale_id`) written alongside the function (a back-fill to already-test-covered model code), not RED-first — a review-driven fix, not new feature behavior outside the test-first loop.

## Reachability

`main.tsx → <Shell/> → ActiveProjectProvider → TopBar → ProjectSwitcher (select → setActiveProject) → ActiveProjectContext → Shell activeProjectId → ProjectGraph (re-root) + SessionsTable (filter)` — exercised end-to-end by `shell_active_project_reroots_graph_and_filters_sessions` (select billing → graph re-roots + Sessions filters to billing's 2). No tested-but-unwired gaps.

## Open follow-ups (already routed hot to the orchestrator)

- **Future TODO:** the dropdown-popover ProjectSwitcher widget (deferred presentation polish); per-project scoping for more views.
- **Convention candidate** _(orchestrator banks)_: active-project is **UI selection state over a frozen projection** (NOT a daemon mutation / provisional contract); scoped views read it via context; Command Center stays global. Pairs with the read-only/safety-display family — bank if the UI-selection-state pattern recurs.
- **Cosmetic:** the Shell provider-wrap indentation (prettier-fixable; no formatter in the gate).

## How to use what was built

The TopBar ProjectSwitcher is now a functional single-select: clicking a project sets it active (the active one shows "✓ Active" + `aria-pressed`); the Project Graph re-roots at it and the Sessions view filters to it (Command Center stays cross-project). At zero projects the graph shows the `graph-no-project` guard. A selected project removed from the projection re-scopes to the default.

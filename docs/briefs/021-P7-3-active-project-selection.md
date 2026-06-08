# /tdd brief — active_project_selection

## Feature
The **active-project model + selection + view-filtering** — completes the currently-**inert** ProjectSwitcher (6.5c compacted it to a row, but it has NO selection; the Shell hardcodes the graph to `projects[0]`). Add a UI **active-project** state, make the ProjectSwitcher **single-select** (pick a project → it's active, marked never-color-alone), and **re-root the project-scoped views** at the active project (resolving the 6.3b graph project-source Q3 + the zero-projects guard). **DAEMON-INDEPENDENT** — pure UI state over the FROZEN projects projection (no provisional-contract risk, unlike the parked mutation seam). The prototype's full dropdown-popover **widget** styling (caret + popover) is **deferred** (a later polish) — this slice builds the BEHAVIOR + a functional selector.

## Use case + traceability
- **Task ID:** §7.3 active-project task (pulled FORWARD as the daemon-independent Phase-6-shell completion — lead-directed measured continue, 2026-06-08). Origin: 2026-06-08 P6.5c ProjectSwitcher deferral.
- **Architecture sections it implements:** `ARCHITECTURE.md §11` (the cockpit — the active project scopes the project-scoped views), `§11.2` (the TopBar project switcher), `§4.2` (views render from projections, filtered by UI selection state — selection is UI state, NOT a daemon mutation). No safety invariant (read/selection only; no intent).
- **Related context:** the Shell currently passes `projectId={data.projects[0]?.project_id ?? ""}` to `<ProjectGraph/>` (hardcoded first project — the 6.3b Q3 graph project-source gap + the untested zero-projects path). The ProjectSwitcher (`ui/src/shell/ProjectSwitcher.tsx`) takes `projects`/`counts`, renders `role="group"`, **no selection**. The read-only/active-project state mirrors the existing `ReadOnlyProvider` context pattern (`ui/src/connection/read-only.ts`). Lesson §8 (projection-item mappers) · §9 (new controls keyboard-reachable + reachability audit) · §11/§11.6 (never-color-alone for the active indicator). **This is a fixture/projection read — NO provisional contract, NO daemon dep.**

## Acceptance criteria
- [ ] A pure **active-project model** (`ui/src/shell/active-project.ts` or `ui/src/projects/`): `defaultActiveProject(projects) → project_id | null` (first project if any, else null); an `ActiveProjectContext`/provider (mirroring `ReadOnlyProvider`) exposing `{ activeProjectId, setActiveProject }`.
- [ ] **ProjectSwitcher single-select** — selecting a project calls `setActiveProject(id)`; the active project is marked with a **glyph+label indicator** (never color alone — §11.6); all project names/counts stay visible + keyboard-reachable (don't regress the existing `findByText(project.name)` tests or the §9 reachability audit).
- [ ] **Graph re-roots at the active project** — `<ProjectGraph projectId={activeProjectId ?? ""}/>` (replacing the hardcoded `projects[0]`), resolving the 6.3b Q3 graph project-source; **zero/no-active state** → the explicit no-projects guard (resolving the 6.3b zero-projects degraded state), not an empty-pid root.
- [ ] **View-filtering** for the project-scoped views per the Step-2.5 scoping decision (default: Graph re-roots + Sessions filters to the active project; Command Center stays GLOBAL cross-project triage).
- [ ] Renders only fixture/projection state + UI selection (no invented data — forbidden #2); selection is **UI state, not a Gateway intent** (no `canSubmitIntent` gate needed — it's a read/scope selection, not a mutation). `/preflight` clean.
- [ ] **Reachable from** `Shell → ProjectSwitcher (select) → ActiveProjectContext → ProjectGraph/Sessions (re-root/filter)`.

## Wiring / entry point (Step 7.5)
`Shell` holds `activeProjectId` (`useState`, default `defaultActiveProject(data.projects)`) + wraps content in `ActiveProjectProvider`; `<ProjectSwitcher onSelect={setActiveProject} activeProjectId={activeProjectId}/>`; `<ProjectGraph projectId={activeProjectId ?? ""}/>` + the filtered views read the context. Confirm: selecting in the switcher re-roots the graph + filters the scoped views; no-projects → the guard state.

## Files expected to touch
**New:** `ui/src/shell/active-project.ts` (model + context/provider) + `active-project.test.ts`; render-test updates.
**Modified:** `ui/src/shell/Shell.tsx` (activeProjectId state + provider + wiring), `ui/src/shell/ProjectSwitcher.tsx` (single-select + active indicator) + its test, `ui/src/views/graph/ProjectGraph.tsx` (project-source = activeProjectId + the zero-projects guard) + test, and the Sessions view if it filters (Step-2.5). Flag anything beyond at Step 2.5.

## RED test outline (Step 2)
**`shell/active-project.test.ts`:**
1. **`default_active_project_first_or_null`** — `defaultActiveProject([p1,p2])`→`p1.id`; `defaultActiveProject([])`→`null`. **[load-bearing]**
2. **`set_active_project_updates_state`** — `setActiveProject(p2.id)` → context `activeProjectId === p2.id`.
3. **`filter_by_active_project`** (if Sessions/views filter) — given items + an active id, returns the active project's subset; null active → the no-active treatment.

**`shell/ProjectSwitcher.test.tsx`:**
4. **`selecting_a_project_sets_active`** — clicking project p2 calls `onSelect(p2.id)`. **[load-bearing — completes the inert control]**
5. **`active_project_marked_never_color_alone`** — the active project shows a glyph+label indicator (not color alone — §11.6); names/counts still visible.

**`views/graph/ProjectGraph.test.tsx` (extend):**
6. **`graph_roots_at_active_project`** — `projectId=p2.id` → the graph roots at p2 (not always projects[0]); resolves 6.3b Q3.
7. **`graph_zero_or_no_active_shows_guard`** — no projects / null active → the explicit no-projects guard state (not an empty-pid root). **[resolves the 6.3b zero-projects gap]**

## Cross-doc invariant impact
- **Model field changes:** **none.** Active-project is **UI selection state** (not a contract/projection field; no provisional shape — it's a selection over the FROZEN projects projection). **Orchestrator rows:** none.

## Things to flag at Step 2.5
1. **View-filtering scope — which views filter by the active project?** Default: **Graph re-roots** (clearly project-scoped, resolves Q3) **+ Sessions filters** to the active project; **Command Center stays GLOBAL** (cross-project "needs my attention" triage — scoping it would hide cross-project attention). My default vote: **Graph + Sessions filter; CC stays global.** Confirm (esp. CC global-vs-scoped).
2. **Active-project default.** Default: **first project** (`projects[0]`) when any exist, else `null`. My default vote: **first-or-null** — a sensible initial scope; null only at zero-projects. Confirm vs null-by-default (force explicit selection).
3. **Selector shape (widget deferral).** Default: the existing compact ProjectSwitcher row becomes **single-select** (click → active, glyph+label indicator); the prototype's full **dropdown-popover widget** (trigger + caret + popover list) is **DEFERRED** (a later polish — it's a presentation refinement, not behavior). My default vote: **selectable-row now, dropdown-widget later** — builds the behavior + completes the dead control without the widget scope. Confirm.
4. **State location.** Default: Shell `useState(activeProjectId)` + an `ActiveProjectProvider` context (mirroring `ReadOnlyProvider`) so the views read it without prop-drilling. My default vote: **context provider.** Confirm.

## Dependencies + sequencing
- **Depends on:** the frozen projects projection (exists) + the ProjectSwitcher (6.5c) + ProjectGraph (6.3b). **No daemon dependency** (UI selection over a frozen projection).
- **Blocks:** resolves the 6.3b graph project-source (Q3) + the zero-projects guard; the prototype's dropdown-widget polish (later); future per-project scoping.
- **Note:** the dropdown-popover widget styling is deferred (Step-2.5 Q3).

## Estimated commit count
**1–2** — a cohesive behavioral slice (active-project model + selection + graph re-root + filtering). No safety invariant → **security-reviewer NOT required**; **code-quality every-slice**. If the model+selection vs the view-filtering split cleanly, 2 layer-commits (model+switcher-selection → graph/view re-root) is fine — flag at Step 2.5; default 1.

## Lessons-logged candidates anticipated
- **Convention candidate** — active-project is **UI selection state over a frozen projection** (NOT a daemon mutation / NOT a provisional contract); the project-scoped views read it via context, the Command Center stays global. Candidate if the UI-selection-state pattern recurs (filters, scopes).
- **Future TODO** — the prototype's dropdown-popover ProjectSwitcher widget (deferred presentation polish); per-project scoping for more views as they land.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd active_project_selection`.
1. Read this brief; Q1 (filtering scope) + Q3 (widget deferral) are the ones to confirm at Step 2.5.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN.
3. Step 7.5 — name `Shell → ProjectSwitcher (select) → ActiveProjectContext → Graph/Sessions (re-root/filter)`.
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me. **Context: you're at ~61% — if this slice pushes you to ≥70% (WARN), say so at Step 9; the lead authorized a seal + auto-cycle after it.**

# /tdd brief — sessions_table_filtering

## Feature
Add **status + free-text filtering** to the Sessions table (sortable today, not filterable) — a pure `filterSessionRows` model composed *before* the existing sort, plus a status `<select>` and a search `<input>` in the table header, with a filtered-empty state distinct from the truly-empty state.

## Use case + traceability
- **Task ID:** P6.3 (sessions-table filtering — non-blocking polish; deferred at 6.3c Step-2.5 Q3)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (Sessions dense list), `§5.2` (attention ordering — preserved under filtering), `§11.6` (a11y — labeled, keyboard-operable filter controls).
- **Related context:** Polish round (ui-006) slice 3. The Sessions table (`ui/src/views/sessions/SessionsTable.tsx`) builds rows via `buildSessionRows` + sorts via `sortSessionRows` (`model.ts`) — both pure. **Project scoping is already handled globally** by the active-project switcher (round 5, `86727ec`): the Shell passes `filterByActiveProject(data.sessions, activeProjectId)` into the table, so the rows the table receives are *already* project-scoped. That makes a *within-table* project facet largely redundant (see Step-2.5 Q1). This is the **§13 family** (UI selection/scope/**filter** state over the frozen projection — Lesson §13's title already includes "filters") — daemon-independent, no contract/mutation/intent.

## Acceptance criteria (what "done" means)
- [ ] A pure `filterSessionRows(rows, filter)` (in `ui/src/views/sessions/model.ts`) where `filter = { text: string; status: string | null }`, AND-composing the active facets:
  - `status === null` → no status constraint; else keep rows whose `status === filter.status`.
  - `text === ""` → no text constraint; else keep rows whose **name OR project name** contains `text` (case-insensitive substring).
  - both empty → returns the input rows unchanged (identity).
- [ ] A pure `distinctStatuses(rows)` returning the statuses present in the **unfiltered** rows (stable options that don't collapse as the filter narrows), in a deterministic order.
- [ ] The table composes **filter → sort** (filter first, then the existing `sortSessionRows`), so attention ordering (§5.2) holds within the filtered subset.
- [ ] A status `<select>` (labeled "Filter by status", options = "All" + `distinctStatuses`) and a search `<input type="search">` (labeled "Search sessions") in the table header; both keyboard-operable + labeled (§11.6).
- [ ] **Filtered-empty ≠ truly-empty:** when rows exist but the filter excludes all, render "No sessions match the filters" + a **Clear filters** control; when no rows arrive at all, keep the existing "No sessions." Both states keep the table semantics intact.
- [ ] All unit tests in `ui/src/views/sessions/model.test.ts` (filter units) pass.
- [ ] UI behavior tests in `ui/src/views/sessions/SessionsTable.test.tsx` pass.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`main.tsx → <Shell/> → content-switch "Sessions" → <SessionsTable/> (filter state + status<select> + search<input>) → filterSessionRows → sortSessionRows → render`. The filter is reached through the live Sessions view (the Shell already routes `filterByActiveProject` rows in); confirm the controls drive `filterSessionRows` on the real path, not unit-only.

## Files expected to touch
**New:** _(none — extends the existing sessions model + table)_

**Modified:**
- `ui/src/views/sessions/model.ts` — add `SessionsFilter`, `filterSessionRows`, `distinctStatuses` (cohesive with `buildSessionRows`/`sortSessionRows`).
- `ui/src/views/sessions/model.test.ts` — filter + distinct-statuses units.
- `ui/src/views/sessions/SessionsTable.tsx` — filter state; the status `<select>` + search `<input>`; compose filter→sort; filtered-empty + Clear.
- `ui/src/views/sessions/SessionsTable.test.tsx` — filter UI behavior + filtered-empty + clear.

If implementation needs files beyond this list (e.g. a separate `filter.ts`), **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
`ui/src/views/sessions/model.test.ts` (pure):
1. **`filter_empty_is_identity`** — Asserts: `{text:"",status:null}` returns all input rows. Why: no facet = no constraint.
2. **`filter_by_status_exact`** — Asserts: `status` keeps only rows with that status. Why: status facet.
3. **`filter_by_text_matches_name_and_project_ci`** — Asserts: a substring matches on name AND on project name, case-insensitively. Why: text facet scope (Q3).
4. **`filter_ands_status_and_text`** — Asserts: both facets active → only rows matching both. Why: AND composition.
5. **`distinct_statuses_from_unfiltered_rows`** — Asserts: returns the present statuses (deduped, deterministic order) from the full row set. Why: stable filter options (Q2).
6. **`filter_then_sort_preserves_attention_order`** — Asserts: filtering then `sortSessionRows` keeps §5.2 attention order within the subset. Why: compose order.

`ui/src/views/sessions/SessionsTable.test.tsx`:
7. **`status_select_filters_rows`** — Asserts: choosing a status in the `<select>` narrows the visible rows. Why: control wiring.
8. **`search_input_filters_rows`** — Asserts: typing in the search input narrows by name/project substring. Why: control wiring.
9. **`filtered_empty_shows_distinct_message_and_clear`** — Asserts: a filter that excludes all rows (with rows present) shows "No sessions match the filters" + a Clear control; Clear restores all rows. Why: filtered-empty ≠ truly-empty (AC).
10. **`truly_empty_unchanged`** — Asserts: zero input rows still shows "No sessions." Why: don't regress the existing empty state.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. Pure UI filter state over the frozen Session projection (§13 family); no contract/`shared/` touch.
- **Orchestrator doc rows to write hot (Step 9 routing):** none (likely **extends Lesson §13** — orchestrator banks; flag at Step 9).

## Things to flag at Step 2.5
1. **Facets — status + text only, or also a project facet?** My default vote: **status + text only**; **defer the project facet** — project scoping is already owned by the active-project switcher (the Shell pre-filters rows by active project), so a within-table project control is redundant when scoped and only meaningfully active when unscoped (rare — `defaultActiveProject` picks the first project). Re-add only if a real unscoped need surfaces. (This is facet-selection, not a capability cut — project filtering already exists via the switcher.) Flag to the orchestrator if you or the lead want the project facet in-slice.
2. **Status options — derived from present rows, or all frozen session statuses?** My default vote: **derive `distinctStatuses` from the (unfiltered) present rows** — no dead options that match nothing; stable w.r.t. the current filter (computed off the full set passed in).
3. **Text match scope — name only, or name + project name?** My default vote: **name + project name** (both are visible columns) — broadest useful match; case-insensitive substring.
4. **Filtered-empty vs truly-empty messaging.** My default vote: **distinguish** — "No sessions match the filters" + Clear-filters when a filter is active and excludes all; the existing "No sessions." when zero rows arrive. A filtered table that looks identical to an empty projection is a confusing dead-end.
5. **Control types — native `<select>` (status) + `<input type="search">` (text)?** My default vote: **yes** — native, keyboard-accessible, labeled; a combobox/popover/multi-select is over-scope for this polish slice.

## Dependencies + sequencing
- **Depends on:** 6.3c Sessions table (`23fbda3`, landed) + the active-project Shell pre-filter (`86727ec`, landed — establishes that the table receives already-project-scoped rows). Independent of slices 1–2.
- **Blocks:** nothing.

## Estimated commit count
**1.** One cohesive slice — the pure filter model + the table's filter controls live in the same area (`ui/src/views/sessions/`), well under the size cap, no safety invariant. Internal RED→GREEN ordering is natural (filter units + model first, then the controls), one Step-10 commit.

## Lessons-logged candidates anticipated
- **Convention candidate** — table filtering is **pure UI filter state** over the frozen projection (compose **filter → sort**; `distinctStatuses` from the unfiltered set; filtered-empty distinct from truly-empty). Same **§13 family** (its title already covers "filters") — orchestrator decides at Step 9 whether to note it under §13.
- **Architecture-doc note candidate** — if §11.2 should note the Sessions filter facets (status + text; project owned by the switcher), flag it.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5" (Q1 facet-selection is the load-bearing one).
2. **Run `/tdd sessions_table_filtering`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** → confirm against the Feature line.
4. **Step 1 (Identify files)** → confirm against "Files expected to touch."
5. **Step 2.5** → tight test-design write-up + answers to the 5 design questions; wait for `APPROVED.` / `TWEAK:` / `ADD:`.
6. **Step 9** → categorized flags + ship-ask.

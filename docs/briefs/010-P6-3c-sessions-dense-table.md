# /tdd brief — sessions_table

## Feature
The **Sessions list — a dense, sortable table** (§11.2: "Sessions List/Board as a dense table, **not only** Command-Center groups"): every session as a row with Name / Status / Attention / Project, **attention-sorted by default** (§5.2) and **sortable by any column header** (asc/desc, `aria-sort`). Mounts as a third content view in the shell's view-switch (Command Center | Project Graph | **Sessions**), extending the 6.3b `contentView` seam. The deterministic core is the pure **table model** (`sessions × projects → enriched rows`, attention via the descriptor table) + the **sort comparator** (per-column, stable). Third sub-slice of Phase 6.3.

## Use case + traceability
- **Task ID:** P6.3c (decomposition of 6.3: 6.3a Command Center ✅ → 6.3b Graph ✅ → **6.3c Sessions** → 6.3d Terminal → 6.3e Code/Diff)
- **Architecture sections:** `ARCHITECTURE.md §11.2` (Sessions List/Board = dense table, net-new beyond the CC groups), `§11.3`/`§5.2` (attention-first ordering; status binding), `§11.7` (SessionRow adds model + team/role — **deferred**, fields not yet in the projection), `§4.2` (renders from projections; the project-name join is a pure derivation — forbidden #2).
- **Related context:** L1 `toSessionItems` (`ui/src/projections/items.ts` — the `{id,label,machine,status}` base shape, single source — Lesson §8); 6.2a `describeStatus`/`compareByAttention`; 6.2b `StatusPill`/`AttentionMarker`; 6.3b `contentView` switch in `Shell.tsx` (the mount seam) + the graph's project-name-join pattern; fixtures (`proj_session` — 5 sessions across 2 projects incl. terminal `completed`; `proj_project_activity` — names). Deterministic core = the table model + sort (pure); the table render + header-sort wiring + mount are render-tested.

## Acceptance criteria
- [ ] `buildSessionRows(sessions, projects)` enriches each session into a row `{id, machine:"Session", status, label, attentionRank, projectName}` — base shape via `toSessionItems` (no inline re-map, §8), `attentionRank` via `describeStatus` (single source, never re-derived), `projectName` resolved by joining `project_id → ProjectActivity.name` (pure §4.2 derivation; unknown/absent project → a visible fallback, not a crash).
- [ ] `sortSessionRows(rows, key, dir)` is a **pure, stable** sort over keys `attention | name | status | project`; **default = `attention` desc** (`compareByAttention`, §5.2 attention-first), with a stable secondary tiebreak.
- [ ] `<SessionsTable/>` renders a **dense semantic `<table>`** — columns Name / Status (`StatusPill`) / Attention (`AttentionMarker`) / Project — attention-sorted by default; **column headers are sortable** (click → key+dir, `aria-sort` reflects the active column/direction); renders only the projection's sessions (no invented rows — forbidden #2); `data-item-id` namespaced `Session:<id>` (§8); an empty session set renders an explicit empty state.
- [ ] The Shell view-switch gains a **Sessions** option; selecting it mounts `<SessionsTable/>` (switch-not-stack — CC/Graph unmount), reading **only through the gateway boundary**. Command Center stays the default.
- [ ] **Reachable from** `Shell → view-switch (Sessions) → SessionsTable → gateway-client`. `/preflight` clean (oxlint + typecheck + test:run + vite build).

## Wiring / entry point (Step 7.5)
`Shell` view-switch → `<SessionsTable/>` → `buildSessionRows`/`sortSessionRows` over the gateway-boundary `Session` + `ProjectActivity` projections. Extends the 6.3b `contentView` seam (its designed-for purpose). Confirm at Step 7.5 the Sessions option mounts the table + the table reads through the boundary.

## Files expected to touch
**New:** `ui/src/views/sessions/model.ts` (`buildSessionRows` + `sortSessionRows` + `SessionRowVM`/`SessionsSortKey` types), `ui/src/views/sessions/SessionsTable.tsx`, `ui/src/views/sessions/{model.test.ts, SessionsTable.test.tsx}`.
**Modified:** `ui/src/shell/Shell.tsx` (add the Sessions view to the `contentView` switch + mount), `ui/src/shell/Shell.test.tsx` (Sessions switch mounts the table).
Flag anything beyond at Step 2.5.

## RED test outline (Step 2)

**`views/sessions/model.test.ts`:**
1. **`builds_rows_with_project_name_and_attention`** — `buildSessionRows` enriches each session: `attentionRank` via `describeStatus`, `projectName` via the `project_id→name` join; base shape from `toSessionItems`. **[load-bearing — §4.2 derivation]**
2. **`unknown_project_id_falls_back_visibly`** — a session whose `project_id` has no `ProjectActivity` match (or is absent) gets a visible fallback projectName (e.g. the raw id / "—"), not a crash/blank.
3. **`default_sort_is_attention_desc`** — `sortSessionRows(rows, "attention", "desc")` (the default) orders by `compareByAttention` (needs-attention first), stable tiebreak. **[load-bearing — §5.2]**
4. **`sorts_by_name_and_toggles_direction`** — `sortSessionRows` by `name` asc/desc reverses order; pure + stable (no mutation of input).
5. **`includes_terminal_sessions`** — terminal sessions (`completed`/`failed`/etc.) appear in the rows (full list, not active-only), sorted to the bottom by attention. [Q4]

**`views/sessions/SessionsTable.test.tsx` (jsdom):**
6. **`renders_a_row_per_session_attention_sorted`** — one `<tr>` per session, default attention order; rendered id set (`data-item-id`) === projection session set (no invented rows — forbidden #2). **[load-bearing]**
7. **`column_header_click_sorts_and_sets_aria_sort`** — clicking a column header re-sorts by that key and sets `aria-sort` on the active header (toggles asc/desc). **[load-bearing — sortable + a11y]**
8. **`status_rendered_via_pill_not_color_alone`** — each row's status renders via `StatusPill` (glyph+label, never-color-alone §11) + attention via `AttentionMarker`.
9. **`empty_sessions_shows_explicit_empty_state`** — an empty session set renders an explicit empty state (not an empty/absent table body).

**`shell/Shell.test.tsx` (extend):**
10. **`view_switch_mounts_sessions_table`** — selecting **Sessions** mounts `<SessionsTable/>` reachable from the Shell; CC stays default; switch-not-stack. **[wiring — Step 7.5]**

## Cross-doc invariant impact
- **Model field changes:** **none.** The table model is **UI render policy** (reuses `ProjectionItem` + the descriptor table); the project-name join is a pure §4.2 derivation, not a new projection/ID. **Orchestrator rows:** none expected.
- **Deferred (flag, do NOT build):** §11.7 **`model` + team/role columns** on the table — `SessionRow` doesn't carry those fields yet (provisional). Render the columns it has now; the model/team columns fold into the existing Carry-forward **provisional→generated reconcile** when `SessionRow` gains them. Same posture as 6.3b's deferred node ownership.

## Things to flag at Step 2.5
1. **Board view.** Default vote: **defer the board** (status/kanban-grouped columns) — the **Command Center already provides the grouped/triage view**; 6.3c delivers the **net-new dense sortable table** ("not only Command-Center groups," §11.2). Confirm vs a List|Board toggle (a board would largely duplicate the CC grouping). _(Not an escalation — the dense table is the acceptance-relevant net-new surface; if a reviewer/demo shows the board is load-bearing, surface it at Step 9.)_
2. **Columns.** Default vote: Name / Status (`StatusPill`) / Attention (`AttentionMarker`) / Project (resolved name). `model`/team/role columns deferred (§11.7, SessionRow lacks them). Confirm.
3. **Sort.** Default vote: keys `attention | name | status | project`; default `attention` desc (§5.2); header-click toggles asc/desc with `aria-sort`. **Filtering deferred** (later polish). Confirm the key set + default.
4. **Terminal sessions.** Default vote: **include ALL sessions** incl. terminal (`completed`/`failed`/`archived`/`killed`) — it's a full list, not the active-only count; terminal rows sort to the bottom via low attention. Confirm vs filtering them out.

## Dependencies + sequencing
- **Depends on:** 6.2a (`b32c3c0`) + 6.2b (`e2cebbc`) + 6.3a (`144b6b6`) + 6.3b (`c420cd7`+`885cc0d`, the `contentView` seam + `items.ts` mapper).
- **Blocks:** 6.3d/6.3e (reuse the `contentView` seam); a session row → Session Terminal (6.3d) / inspector navigation (later); the §25 demo Sessions surface.
- **Carry-forwards consumed:** none net-new (reuses §8 mapper + the contentView seam). The §11.7 model/team columns + filtering are forward-flagged (provisional reconcile / later polish), not consumed here.

## Estimated commit count
**1** — a cohesive, purely-additive slice: the sort/table model + `SessionsTable` view + a one-option addition to the existing `contentView` switch. Small, same area, reuses the L1 mapper + the descriptor table + the 6.3b mount seam; no refactor prerequisite (unlike 6.3b), so **no layer-driving** — single RED→GREEN→commit. No safety invariant (read/render only) → **security-reviewer NOT required**; **code-quality every-slice**. (If the sort model + view feel separable in practice, splitting to 2 commits is fine — flag at Step 2.5; default is 1.)

## Lessons-logged candidates anticipated
- **Convention candidate** — the `contentView` switch now hosts three views (CC/Graph/Sessions) via one seam; reaffirms the mount pattern (likely no new lesson — §8 + the seam already cover it). If 6.3c surfaces a cleaner view-registration shape, that's a candidate.
- **Future TODO — provisional reconcile** — `model`/team/role columns when `SessionRow` gains them (already in the Carry-forward provisional→generated spread); session-table **filtering** as a later polish.
- **Architecture-doc note candidate** — none expected; the dense sortable table is exactly §11.2.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd sessions_table`.
1. Read this brief; Q1 (board defer) + Q4 (terminal sessions) are the ones to confirm at Step 2.5.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN.
3. Step 7.5 — name `Shell → view-switch (Sessions) → SessionsTable → gateway-client` as the entry point.
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me.

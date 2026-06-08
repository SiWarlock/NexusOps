# /tdd brief — project_graph

## Feature
The **Project Graph** view + its **accessibility list/table fallback** (§11.6 OBS-6): a per-project graph of the project's entities (the project root + its sessions + its pull requests) with a **Graph | List toggle**, where the **List/table fallback is functionally equivalent** (same nodes + edges, each carrying type + status + attention) and is the designated keyboard surface. The deterministic core is the pure **graph model** (`projections → {nodes, edges}`, scoped to one project, attention via the 6.2a descriptor table) and the **fallback equivalence** (every node a row; every edge represented). Mounts as a switchable content view in the shell alongside the Command Center. Second sub-slice of Phase 6.3.

This slice also folds in two `MVP_TASKS.md` "Next ui brief (6.3b) working set" carry-forwards: the **shared `SessionItem` extraction** (kills the Shell `sidebarItems`/`commandItems` duplication) and the **`data-item-id` locator convention** (settled across 6.3 views before the §25 demo).

## Use case + traceability
- **Task ID:** P6.3b (decomposition of 6.3: 6.3a Command Center ✅ → **6.3b Graph + list/table fallback** → 6.3c Sessions → 6.3d Terminal → 6.3e Code/Diff)
- **Architecture sections:** `ARCHITECTURE.md §11.6` (**load-bearing** — Project Graph list/table fallback: functionally equivalent same-nodes+edges, Graph|List toggle, keyboard-reachable; node names include type+status+attention), `§11.2` (Project Home/Graph in the surface map), `§11.7` (GraphNode node types — MVP subset only; full set deferred), `§11.3`/`§5.2` (attention/status binding — node attention via the descriptor table), `§4.2` (renders from projections; edges are a pure derivation, never invented state — forbidden #2).
- **Related context:** 6.2a model (`ui/src/status/` — `describeStatus`, `compareByAttention`, `AttentionRank`); 6.2b wrappers (`StatusPill`/`AttentionMarker`); 6.3a `groupForCommandCenter` + `CommandItem` (the item shape this slice extracts a shared mapper for); 6.1b Shell (`sidebarItems`/`commandItems` map `data.sessions` → items — the duplication to dedup; the content pane to add a view switch to); fixtures (`proj_session`, `proj_project_activity`, `proj_pull_request` — `project_fixture_1` populated, `project_fixture_3` activity-free for the empty case). Deterministic core = the graph model + items mappers (pure logic); the view (toggle/table/empty-state) is render-tested.

## Acceptance criteria
- [ ] **(Layer 1)** A shared item mapper (`toSessionItems`/`toPrItems`/`toApprovalItems`) produces the `{id,label,machine,status}` shape; the Shell's `sidebarItems` + `commandItems` and the graph's session/PR nodes all route through it (no inline re-map). Behavior-preserving — existing Shell + Command Center tests stay green.
- [ ] **(Layer 1)** The **`data-item-id` locator convention** is `<namespace>:<id>` (namespace = the status-machine name for status items, the node-type for graph nodes) — collision-safe. 6.3a's bare `data-item-id={item.id}` is retrofit to `${machine}:${id}`.
- [ ] **(Layer 2)** `buildProjectGraph({projectId, projects, sessions, pullRequests})` returns `{projectId, nodes, edges}`: a **project-root node** + a node per session + per PR **scoped to that project** (`project_id === projectId`), each node carrying `type` (`project`/`session`/`pull_request`) + `label` + `status` (none for root) + `attentionRank` (via `describeStatus`, never re-derived); and a `contains` **edge** from the root to each child (edge count === child count).
- [ ] **(Layer 2)** `<ProjectGraph/>` renders a **Graph | List toggle**; the **list/table fallback** renders **exactly one row per node** (functional equivalence) with type/status (`StatusPill`)/attention (`AttentionMarker`), and **represents every edge** (each child row names its parent; root = "—"). Each node's accessible name/cells include **type + status + attention** (§11.6). The table is keyboard-reachable.
- [ ] **(Layer 2)** Renders **only** the model's nodes (no invented nodes — forbidden #2); an **empty project** (root-only) renders an explicit empty state.
- [ ] **(Layer 2 — wiring)** The Shell exposes a content-view switch (Command Center | Project Graph); the Graph view mounts `<ProjectGraph/>` for the first project, reads **only through the gateway boundary**, and Command Center stays the default. `/preflight` clean (oxlint + typecheck + test:run + vite build).

## Wiring / entry point (Step 7.5)
`Shell` content pane → content-view switch → `<ProjectGraph project=… />` → `buildProjectGraph` over the gateway-boundary projections (`ProjectActivity`/`Session`/`PullRequest`). Names the real reachable entry: the graph is reachable from a production control (the view switch), not test-only. Confirm at Step 7.5 the switch mounts the graph + the list toggle is reachable from it. (The visual-graph node keyboard-operability + global focus ring are the 6.4 a11y pass — here the **List/table is the designated keyboard surface** per the §11.6 OR-clause.)

## Files expected to touch
**Layer 1 (refactor/prep):**
- **New:** `ui/src/projections/items.ts` (`SessionItem` shape + `toSessionItems`/`toPrItems`/`toApprovalItems`), `ui/src/projections/items.test.ts`.
- **Modified:** `ui/src/shell/Shell.tsx` (route `sidebarItems`/`commandItems` through the mappers), `ui/src/views/command/CommandCenter.tsx` (locator `item.id` → `${machine}:${id}`), `ui/src/views/command/CommandCenter.test.tsx` (locator assertion).

**Layer 2 (feature):**
- **New:** `ui/src/views/graph/model.ts` (pure `buildProjectGraph`), `ui/src/views/graph/ProjectGraph.tsx` (toggle + visual graph + table fallback), `ui/src/views/graph/{model.test.ts, ProjectGraph.test.tsx}`.
- **Modified:** `ui/src/shell/Shell.tsx` (content-view state + switch control + mount `<ProjectGraph/>`), `ui/src/shell/Shell.test.tsx` (view-switch mounts the graph).

Flag anything beyond at Step 2.5.

## RED test outline (Step 2)

**Layer 1 — `projections/items.test.ts`:**
1. **`to_session_items_maps_rows`** — `toSessionItems(rows)` → `{id:session_id, label:title??session_id, machine:"Session", status}` per row; label falls back to id when `title` absent.
2. **`to_pr_items_maps_rows`** — `toPrItems` → `{id:pr_number, label:title??`PR #${n}`, machine:"PullRequest", status}`.
3. **`to_approval_items_maps_rows`** — `toApprovalItems` → `{id:approval_id, label:title??approval_id, machine:"Approval", status}`.

**Layer 1 — `views/command/CommandCenter.test.tsx` (extend):**
4. **`item_locator_is_machine_namespaced`** — a rendered item's `data-item-id` === `${machine}:${id}` (was bare). **[load-bearing — locator convention]**

(The existing Shell + CommandCenter render tests are the **regression guard** — the layer-1 refactor must keep them green; the chrome/triage output is unchanged.)

**Layer 2 — `views/graph/model.test.ts`:**
5. **`builds_project_root_plus_session_and_pr_nodes`** — `buildProjectGraph` for `project_fixture_1` → nodes = [project root, its sessions, its PRs], each with `type`/`label`/`status`/`attentionRank`. **[load-bearing]**
6. **`scopes_nodes_to_the_given_project`** — only `project_fixture_1`'s sessions/PRs appear; `project_fixture_2`'s are excluded.
7. **`builds_contains_edges_root_to_each_child`** — one `contains` edge root→each child; `edges.length` === child-node count.
8. **`node_attention_from_descriptor_table`** — a `waiting_on_permission` session → `attentionRank` 4 via `describeStatus` (single source, never re-derived); project root → rank 0.
9. **`empty_project_has_root_only_no_edges`** — `project_fixture_3` (activity-free) → nodes = [root], edges = [].

**Layer 2 — `views/graph/ProjectGraph.test.tsx` (jsdom):**
10. **`defaults_to_graph_and_toggles_to_list`** — Graph view by default; activating List switches to the table (Graph|List toggle, §11.6).
11. **`list_fallback_has_a_row_per_node`** — the table renders exactly one row per graph node (every node appears). **[load-bearing — OBS-6 equivalence]**
12. **`list_fallback_represents_every_edge`** — every edge represented (each child row names its parent; root = "—"). **[load-bearing — OBS-6 equivalence]**
13. **`node_accessible_name_includes_type_status_attention`** — each node/row exposes type + status + attention. **[load-bearing — §11.6]**
14. **`renders_only_projection_nodes`** — rendered node set === model node set (no invented nodes — forbidden #2). **[load-bearing]**
15. **`empty_graph_shows_explicit_empty_state`** — root-only project renders an explicit empty state (not absent).

**Layer 2 — `shell/Shell.test.tsx` (extend):**
16. **`view_switch_mounts_project_graph`** — switching the content view to Graph mounts `<ProjectGraph/>` for the first project, reachable from the Shell. **[wiring — Step 7.5]**

## Cross-doc invariant impact
- **Model field changes:** **none.** The graph model is **UI render policy** (like the 6.2a attention table) — not a frozen cross-language contract; no `shared/` field changes, no new ID. Edges are a **pure derivation** from the projections' `project_id` linkage (§4.2), not a new projection. **Orchestrator rows:** none expected.
- **Deferred (flag, do NOT build):** §11.7's **full GraphNode node-type set** (team_lead/orchestrator/Task/Workflow-command) and **ownership** (team/role) in the node — no projection carries them yet (Task = Phase 7; team/role = §11.7 carry-forward / 6.4). MVP node types = project/session/pull_request; ownership in the node a11y name is N/A until `SessionRow` gains team/role. If you find yourself wanting them, that's a Step-9 follow-up, not scope creep into this slice.

## Things to flag at Step 2.5
1. **Node + edge set.** Default vote: nodes = **project root + its sessions + its PRs**; edges = project→child `contains`. Approvals excluded (queue items, not graph entities); tasks excluded (Phase 7 projection). PRs attach to the **project root** (no session→PR linkage in the data yet). Confirm.
2. **List fallback form.** Default vote: a **semantic `<table>`** — one row per node; columns Type / Name / Status / Attention / Contained-by — where the Contained-by (parent) column represents the edge set. Confirm vs a flat `<ul>` list.
3. **Graph's project source.** Default vote: the **first project** (`data.projects[0]`); a real selected-project state is **deferred** (wires when the ProjectSwitcher gains selection — a follow-up, likely 6.3c/Phase 7). Confirm.
4. **Default view + keyboard surface.** Default vote: **Graph** shown first, List one toggle away; the **List/table is the designated keyboard surface** (§11.6 OR-clause). Per-node graph keyboard-operability + the global `:focus-visible` ring are the **6.4** a11y pass, NOT this slice. Confirm.
5. **Locator convention.** Default vote: `data-item-id = `${machine}:${id}`` for status items (retrofit 6.3a's bare id) and `${type}:${id}` for graph nodes (the machine-less project root) — both `namespace:id`, collision-safe. Confirm.

## Dependencies + sequencing
- **Depends on:** 6.2a (`b32c3c0`) + 6.2b (`e2cebbc`) + 6.3a (`144b6b6`) + 6.1b shell (`39a87c6`).
- **Blocks:** the rest of 6.3 (Sessions/Terminal/Code-Diff reuse the items mapper + the locator convention + the content-view switch seam); the §25 demo Project-Home surface; the 6.4 a11y pass (focus ring / drag→non-drag / node keyboard-operability build on this fallback).
- **Carry-forwards consumed:** shared `SessionItem` extraction; the 6.3 `data-item-id` locator convention. The 6.3 code-quality nits (`compareByAttention`→`AttentionRank` narrow; `useMemo` sidebar/command items) do **NOT** trigger here (graph ranks are real descriptor ranks, not synthetic; data is still fixtures) — they stay carry-forward at `last-consumer-slice: 6.3`.

## Estimated commit count
**2** — multi-commit slice; per Lesson §7 the layers are enumerated and the orchestrator drives layer→layer (the implementer idles after the layer-1 commit; expect one wake onto layer 2):
- **Layer 1 — items extraction + locator convention** (`projections/items.ts` + Shell/CommandCenter retrofit). Behavior-preserving refactor with the locator change pinned. Small (~40-60 lines), no new screen.
- **Layer 2 — graph model + view + Shell wiring** (`views/graph/` + content-view switch). The feature.

No safety invariant touched (read/render only — no intent submission, no mutation) → **security-reviewer NOT required**; **code-quality every-slice** (per `CLAUDE.md` Step-8 policy). The two layers are bundled into one slice/brief (same code area, shared context, one logical unit: "the graph view and the dedup it consumes"); they split into two commits only for clean per-layer RED→GREEN + bisectability.

## Lessons-logged candidates anticipated
- **Convention candidate** — graph nodes derive `status`/`attentionRank` from the descriptor table (single source); views never re-derive attention (extends `ui/LESSONS.md §5`/§6). Likely a small extension note, not a new lesson.
- **Convention candidate** — projection→item mappers (`toSessionItems`/…) are the single source for the `{id,label,machine,status}` item shape (chrome/views don't re-map inline); and `data-item-id` is namespaced `<namespace>:<id>` across 6.3 views (collision-safe locator). Candidate for a new `ui/LESSONS.md` entry if it recurs in 6.3c–e.
- **Future TODO — operational** — the §-perf "Graph render < 500 ms" budget (`ARCHITECTURE.md` perf table): memo `buildProjectGraph` + virtualize the table when real subscriptions + larger graphs land (still fixtures now — not yet).
- **Architecture-doc note candidate** — none expected; the toggle + equivalent table is exactly §11.6.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd project_graph`.
1. Read this brief end-to-end; Q1 (node/edge set) + Q4 (default view / keyboard surface) are the ones to confirm at Step 2.5.
2. **Layer 1 first:** Step 2.5 test-design write-up for the items mapper + locator (`Asserts:` lines) → wait for the magic-words reply → GREEN → Step-9 → commit. Then **idle for the layer-2 wake** (the orchestrator drives layer→layer — Lesson §7).
3. **Layer 2:** the graph model + view + Shell wiring (its own Step 2.5 if the design shifted from this brief, else proceed) → Step 7.5 name `Shell → view-switch → ProjectGraph → gateway-client` as the entry point → Step-9 commit-message-first.

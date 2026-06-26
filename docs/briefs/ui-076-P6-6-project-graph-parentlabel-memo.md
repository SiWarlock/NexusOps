# /tdd brief — project_graph_parent_label_memo

## Feature
Memoize `ProjectGraph`'s `parentLabelOf` — replace the O(n²) per-row double-`.find()` (a scan of
`edges` + a scan of `nodes` for every table row) with an O(n) precomputed `childId → parentLabel`
map (O(1) per row). Behavior-preserving; guarded by the ui-066 6.6 graph-render bench. The
comment-flagged TODO at `ProjectGraph.tsx:164` ("when real subscriptions … land, memoize") — that
trigger is now MET (the live `UdsGatewayPort` subscriptions render real graph data, not just fixtures).

## Use case + traceability
- **Task ID:** P6.6 (the §18 graph-render perf-hardening the ui-066 bench guards; `IMPLEMENTATION_PLAN.md` 6.6 + the Cross-track carry-forward "ui perf memoization · `ProjectGraph` `parentLabelOf` O(n²)").
- **Architecture sections it implements:** `ARCHITECTURE.md §18` (the graph-render perf budget the ui-066 bench tracks), `§11.6` (the graph list/table fallback — the Contained-by column where `parentLabelOf` renders).
- **Related context:** NON-cat-1, pure ui perf — no daemon dep, no contract/model change. Guarded by the ui-066 graph-render bench (`*.bench.ts` / `*.bench.guard.ts`, §18, < 150 ms). [[31]] (the ui bench is the §18 analogue).

## Acceptance criteria (what "done" means)
- [ ] A single O(n) precomputed lookup (`childId → parent node label`) replaces the two per-row `.find()` scans; the Contained-by column reads it O(1) per row.
- [ ] **Behavior-preserving:** for every node, the memoized label === the current `parentLabelOf(node)` result — same labels, same `"—"` (em-dash) for a root node (no incoming edge) AND for an edge whose `from` node is absent from `nodes`.
- [ ] The ui-066 graph-render bench guard stays green (the memo must not regress; it should improve at scale).
- [ ] `/preflight` clean (oxlint + typecheck + test:run).

## Wiring / entry point (Step 7.5)
`src/views/graph/ProjectGraph.tsx` — the graph table view (the §11.6 list/table fallback), reachable from the **Graph** content-view via the Shell. The Contained-by column (`<td className="graph-table__parent">`) reads the new map instead of calling `parentLabelOf` per row. No new entry point — same render path, fewer scans.

## Files expected to touch
**Modified:**
- `src/views/graph/ProjectGraph.tsx` — build the `childId → parentLabel` map once (O(n)); the column reads it.
- `src/views/graph/ProjectGraph.test.tsx` — the behavior-preserving + edge-case pins.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `src/views/graph/ProjectGraph.test.tsx`:

1. **`parent_label_matches_for_all_nodes`** — a multi-node/multi-edge graph: every node's rendered Contained-by label === the label the prior `parentLabelOf` logic yields. Asserts: behavior-preserving. Why: §18 perf-hardening must not change output.
2. **`parent_label_root_node_renders_dash`** — a node with no incoming edge → `"—"`. Asserts: root case preserved. Why: §11.6 table fallback correctness.
3. **`parent_label_missing_parent_renders_dash`** — an edge whose `from` id is absent from `nodes` → `"—"`. Asserts: the defensive fallback survives the memo. Why: preserve the current `?? "—"` guard.

## Cross-doc invariant impact
- **Model field changes:** none — no contract/projection/model change.
- **Orchestrator doc rows to write hot:** none (pure ui perf; not a cross-doc invariant). Possible minor LESSON candidate (memoize O(n²) per-row graph lookups into an O(n) map once real subscriptions land) — orchestrator decides at Step 9.
- **Shared-contract seam model touched?** No.

## Things to flag at Step 2.5
1. **Memo mechanism.** A within-render `Map<childId, label>` built once (O(n)) vs a `useMemo` keyed on `graph`. My default vote: **a within-render `Map`** — the real win is eliminating the per-row O(n²) double-scan; `graph` is a fresh object per subscription nudge so `useMemo` on its identity rarely caches across renders anyway. Add `useMemo` only if profiling shows the within-render build is itself hot (it isn't — O(n) once).
2. **Shell `sidebarItems`/`commandItems` memo — in or out?** The carry-forward names them, but I couldn't confirm a real per-render recompute in `Shell.tsx`. My default vote: **OUT (scope to the confirmed ProjectGraph hotspot)** — if you find a genuine Shell recompute, flag it as a separate follow-on rather than widening this slice.
3. **`"—"` sentinel.** Preserve the exact current em-dash for root/missing. Default vote: **byte-identical** (no UX change).

## Dependencies + sequencing
- **Depends on:** ui-066 (✅ the 6.6 graph-render bench that guards this); the live subscriptions (✅ landed) that make it timely.
- **Blocks:** nothing — a standalone perf-hardening.

## Estimated commit count
**1.** A focused, behavior-preserving perf-memo — one concern, one commit.

## Lessons-logged candidates anticipated
- **Convention candidate (minor)** — memoize O(n²) per-row graph/table lookups into an O(n) precomputed map once real (subscribed) data replaces fixtures; behavior-preserving, bench-guarded (§18). Orchestrator decides whether it rises to a logged lesson.

## How to invoke
1. **Read this brief end-to-end** + the `parentLabelOf` site (`ProjectGraph.tsx:163-171`).
2. **Run `/tdd project_graph_parent_label_memo`.**
3. **Step 2.5** — ping me with the test-design write-up + the 3 design-Q answers/defaults.
4. **Step 9** — surface anything beyond the anticipated lesson candidate (NON-cat-1 → no security-reviewer required unless something surfaces).

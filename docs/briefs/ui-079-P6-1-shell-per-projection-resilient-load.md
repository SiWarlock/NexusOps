# /tdd brief — shell_per_projection_resilient_load

## Feature
Make the Shell's initial cockpit load **per-projection-resilient**: replace the single
`Promise.all([7 get_projection + get_capabilities])` (`Shell.tsx:200-210`) — where ANY one
projection rejecting tanks the ENTIRE cockpit (blank screen) — with per-result resilience so a
single failed projection degrades **only its tile**, and the rest of the cockpit renders. A REAL,
**version-independent** robustness bug (a stale/skewed/boundary-rejecting projection must not blank
the whole cockpit). Cross-track directive (user, 2026-06-25).

## Use case + traceability
- **Task ID:** P6.11 (Cockpit-load per-projection resilience; `IMPLEMENTATION_PLAN.md` section 6.11) — cockpit-load robustness.
- **Architecture sections it implements:** `ARCHITECTURE.md §11.4` (READ-ONLY/degraded display), `§11.7` (honest degrade — never stale-as-live; a failed projection is NOT silently "empty"), `§11.2` (the cockpit tiles), `§6.1` (the projection reads).
- **Related context:** the boundary Zod-parse (`BoundaryValidationError`, [[22]]) or a daemon read-fault can reject ONE `get_projection`; today `Promise.all` propagates that rejection → the `try` catch leaves `data` null → a blank cockpit. The fix lets the other 6 projections + caps render. Version-independent (do regardless of the 0.46 sync). One-stream-degraded connection semantics ([[25]]/[[29]]) are unchanged — this is the INITIAL load, not the live subscribe path.

## Acceptance criteria (what "done" means)
- [ ] The 7-projection initial load is **per-projection-resilient** (`Promise.allSettled` or per-call try/catch) — a single `get_projection` rejecting does NOT prevent the other projections (or `get_capabilities`) from loading + rendering.
- [ ] A failed projection degrades **honestly** (§11.7) — NOT silently shown as genuinely-empty: its tile shows an honest "unavailable" treatment (distinct from real-empty) OR, if degrade-to-empty is chosen for MVP, it's logged (`console.error`) + flagged (Step-2.5 decides the §11.7 treatment).
- [ ] `deriveProjectSwitcherCounts` handles a missing/failed projection gracefully (its count derives from `[]`, not a crash) — `ProjectActivity`/`PullRequest`/`Session`/`ApprovalQueue` are its inputs.
- [ ] `get_capabilities` failure is handled fail-safe (version `unknown` → read-only, [[4]]) WITHOUT blanking the projections (caps is independent of the projection tiles).
- [ ] **Happy path unchanged:** all 7 succeed → the cockpit renders exactly as today (no regression).
- [ ] No fabricated/optimistic data for a failed projection (honest absence, [[11]]/§11.7).
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`src/shell/Shell.tsx` the initial-load `useEffect` (`:196-229`) — the `Promise.all` → per-result resilience; `setData(...)` populates from the surviving results (a failed projection → `[]`/degraded for its slice). Reachable on every cockpit mount (the production entry). No new entry point.

## Files expected to touch
**Modified:**
- `src/shell/Shell.tsx` — the resilient load + the per-slice degrade wiring.
- `src/shell/Shell.test.tsx` (or `Shell.subscribe.test.tsx`) — the resilience pins.
- possibly `src/shell/<data-model>.ts` (if a per-projection degrade flag is added to `ShellData`).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`one_projection_failure_renders_the_rest`** — mock ONE `get_projection` (e.g. `PullRequest`) to reject → the other 6 + caps still load + the cockpit renders (NOT blank). Why: §11.4/§6.1 — the core resilience.
2. **`failed_projection_degrades_honestly_not_fabricated`** — the failed projection's slice is honest-absent/degraded (empty rows + the §11.7 treatment Step-2.5 picks), never invented data. Why: §11.7/[[11]].
3. **`counts_survive_a_failed_counts_input_projection`** — a failed `ProjectActivity` (or `PullRequest`) → `deriveProjectSwitcherCounts` still computes (that input → 0/absent), no crash. Why: §11.2 switcher robustness.
4. **`capabilities_failure_is_fail_safe_read_only`** — `get_capabilities` rejecting → version `unknown` (read-only, `canSubmitIntent` false) WITHOUT blanking the projection tiles. Why: [[4]]/§11.4 fail-safe.
5. **`all_succeed_unchanged`** — all 7 + caps succeed → the cockpit data === today's shape (regression guard). Why: behavior-preserving.

## Cross-doc invariant impact
- **Model field changes:** none, UNLESS a per-projection degrade flag is added to `ShellData` (a UI-local view-model type, not a frozen contract — no cross-doc row). Flag at Step 9 if `ShellData` gains a field.
- **Orchestrator doc rows to write hot:** none expected (UI-local robustness; no frozen-contract change). A possible convention candidate (initial multi-projection load = per-projection-resilient + honest per-tile degrade, never all-or-nothing) — orchestrator decides at Step 9.
- **Shared-contract seam model touched?** No.

## Things to flag at Step 2.5
1. **Mechanism.** `Promise.allSettled` (one array, map each result) vs per-call `.catch(()=>fallback)` wrappers. My default vote: **`Promise.allSettled`** — one clean structure; each result independently `status==="fulfilled"?rows:degraded`.
2. **The §11.7 degrade treatment for a failed tile.** (a) degrade-to-empty (`rows:[]`) + `console.error` [MVP-simplest, but empty≈failed risks stale-as-live]; (b) a per-projection degrade flag on `ShellData` → the tile renders an honest "unavailable" state [§11.7-correct, more wiring]. My default vote: **(b) the honest per-tile flag** if the tile components can show a degrade state cheaply; else (a) + a flagged §11.7 follow-on. Confirm what the tiles support.
3. **caps failure scope.** Treat `get_capabilities` failure as fail-safe read-only (version unknown) while still rendering projections — vs lumping caps with the projections. My default vote: **caps failure → read-only, projections independent** (a caps fault shouldn't blank data tiles, and a data-tile fault shouldn't force read-only).

## Dependencies + sequencing
- **Depends on:** the `track/ui ← main` 0.46 sync + regen (lands FIRST, orch — so this slice builds against 0.46). The bug itself is version-independent.
- **Blocks:** the `track/ui → main` merge (this robustness fix should ride the merge per the cross-track directive).

## Estimated commit count
**1.** A focused cockpit-load resilience fix (one `useEffect`).

## Lessons-logged candidates anticipated
- **Convention candidate** — a multi-projection initial load is per-projection-resilient (`allSettled`), never all-or-nothing (`Promise.all`); a failed projection degrades its tile honestly (§11.7, not silent-empty), caps-fault → fail-safe read-only independent of the data tiles. Complements the live-subscribe per-stream worst-of aggregation ([[29]]) for the INITIAL load.

## How to invoke
1. **Read this brief** + the load `useEffect` (`Shell.tsx:196-229`).
2. **Run `/tdd shell_per_projection_resilient_load`.**
3. **Step 2.5** — ping me with the test-design write-up + the 3 design-Q answers (esp. the §11.7 degrade treatment).
4. **Step 9** — flag a `ShellData` field add if any; surface anything beyond the convention candidate.

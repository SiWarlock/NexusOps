# bench hand-off — graph_render_benchmark (§18; OWN-cadence, NOT a /tdd RED/GREEN slice)

> **This is a benchmark hand-off, not a `/tdd` brief.** A perf bench is timing-based — there is NO RED/GREEN cycle (timing assertions are flaky in a per-slice loop). The flow is: write the bench → measure the as-built render → calibrate a CI guard (tighter than the SLO, with margin) → wire it to the OWN cadence (nightly + `/phase-exit`), NEVER `vitest run`/per-slice. The deliverable is the bench file + a calibrated guard + the cadence wiring.

## Feature
The **§18 Project-Graph render benchmark** (task 6.6) — bench the `ProjectGraph` view render against the §18 budget **< 500 ms** for a typical-size project graph. The FIRST ui bench (establishes the ui bench pattern, mirroring the daemon's `daemon/benches/` + `daemon/LESSONS.md` §22 — daemon-side). Fixture-driven, runnable now (the view exists).

## Use case + traceability
- **Task ID:** P6.6 (the §18 graph-render benchmark)
- **Architecture sections it implements:** `ARCHITECTURE.md §18` (the perf budget — graph render < 500 ms)
- **Related context:** `ui/src/views/graph/ProjectGraph.tsx` (the render target — 14.6K) + `model.ts` (the graph-derivation model); the daemon bench precedent (`daemon/LESSONS.md` §22 — drive the REAL production entry under a defined load model, realistic-gated + saturating-reported; calibrate the CI guard on the as-built measurement + margin, tighter than the SLO; run at the bench's OWN cadence; match the throughput/timing unit to the guard's basis; retire proxy numbers); the existing `.github/workflows/nightly.yml` (where the daemon bench runs — the ui bench wires alongside it); `ui/vitest.config.ts` (vitest's native `bench()` runs via `vitest bench`).

## Deliverable (what "done" means)
- [ ] **The bench file** — `ui/src/views/graph/graph.bench.ts` (NEW; vitest `bench()`): render the **real `ProjectGraph` production component** (NOT a proxy — `daemon/LESSONS.md` §22) with a **typical-size fixture**, measuring render time. Drive the as-built render path (the same `ProjectGraph` the Shell mounts), fed from a realistic graph fixture.
- [ ] **The load model** — a **typical** project graph (realistic-gated) + report the saturating case. State the node/edge count explicitly (the "typical" definition — see bench-design #1). Source the fixture from the existing graph fixtures / projection fixtures where possible (don't invent a parallel shape).
- [ ] **Measure + calibrate the CI guard** — run the bench, record the as-built render time, set a CI guard **tighter than the 500 ms SLO, with margin** (e.g. guard at the as-built p-something + headroom, well under 500 ms) so a regression trips the guard before it breaches the SLO. State the measured number + the chosen guard in the bench/PR.
- [ ] **Wire the OWN cadence** — the bench runs at **`nightly.yml` + `/phase-exit`**, NEVER in `vitest run` / the per-slice suite (timing is flaky there). Add the `vitest bench` invocation to `nightly.yml` (mirror the daemon bench job); confirm `pnpm test:run` does NOT pick up `*.bench.ts`.
- [ ] `tsc --noEmit` + `oxlint` clean on the bench file; the regular suite still green + unaffected (the bench is excluded from it).
- [ ] Cross-doc: none (a bench is not a contract surface). Flag at Step 9 only if the bench surfaces a §18 re-baseline need (that's a load-bearing escalation — `daemon/LESSONS.md` §22: a §18-budget re-baseline is USER-ruled, not agent-set).

## Wiring / entry point (Step 7.5)
A bench has no production-runtime entry point — its entry is the **CI cadence**: the `vitest bench` invocation added to `.github/workflows/nightly.yml` (+ run at `/phase-exit` for the §18 graph row), NEVER `pnpm test:run`/the per-slice suite. The bench itself drives the **real `ProjectGraph` production component** (the as-built render the Shell mounts — not a proxy; `daemon/LESSONS.md` §22). `/wired` target: the `nightly.yml` bench job invokes `graph.bench.ts`, which renders the production `ProjectGraph`.

## Bench design (the checkpoints — surface to me before finalizing, like a lighter Step-2.5)
1. **The "typical" load model** (node/edge count). My default: a realistic mid-size project graph (e.g. ~50–100 nodes with proportional edges — match what a real `proj_project_graph` projection yields for a typical repo). Saturating case reported separately (e.g. ~500 nodes) — realistic-gated for the guard, saturating-reported for visibility. Confirm the count + the fixture source.
2. **The guard threshold + margin.** My default: calibrate on the as-built measurement + margin (the daemon §22 precedent — tighter than the 500 ms SLO so a regression trips early). State the measured render time; pick a guard with clear headroom under 500 ms. If the as-built is ALREADY near/over 500 ms, STOP + flag it as a Finding (a real §18 budget risk — not a guard-calibration choice; `daemon/LESSONS.md` §22 re-baseline is USER-ruled).
3. **The render harness.** My default: render `ProjectGraph` via the same testing-library/JSDOM path the component tests use (the as-built production component), timing the render. Confirm vitest `bench()` measures the render meaningfully in JSDOM (if JSDOM render timing is too noisy to be useful, flag it — a bench that can't measure the real cost is worse than none; `daemon/LESSONS.md` §22 "drive the REAL production entry").

## Files expected to touch
**New:** `ui/src/views/graph/graph.bench.ts` (the vitest bench).
**Modified:** `.github/workflows/nightly.yml` (add the `vitest bench` invocation — mirror the daemon bench job) · possibly `ui/vitest.config.ts` (a bench config / ensure `*.bench.ts` is excluded from `test:run`).

If implementation needs files beyond this list, flag before finalizing.

## NOT a RED/GREEN cycle — the flow
1. Write `graph.bench.ts` (the bench harness + the fixture).
2. Run `vitest bench` → record the as-built render time.
3. Calibrate the guard (as-built + margin, < 500 ms with headroom).
4. Wire the nightly + phase-exit cadence; confirm `test:run` excludes it.
5. Step 9 → me: the measured number + the chosen guard + the cadence wiring (+ flag if the as-built is near the SLO = a §18 re-baseline escalation).

## Dependencies + sequencing
- **Depends on:** none (the Phase-6 graph view exists).
- **Blocks:** the 6.6 phase-exit row (the §18 graph budget is verified by this bench at `/phase-exit`). 6.7 diff-open benchmark follows (gated on the 6.3e Code/Diff surface; sequence after 6.6 if clean).

## Estimated commit count
**1** — the bench file + the cadence wiring (one cohesive bench deliverable). NOT a RED/GREEN slice; no Step-2.5 RED design (the bench-design checkpoints above replace it).

## Lessons-logged candidates anticipated
- **Convention candidate** — the FIRST ui bench establishes the ui-side bench pattern (drive the real production component; realistic-gated + saturating-reported; calibrate the guard tighter than the SLO; own-cadence not per-slice; exclude `*.bench.ts` from `test:run`). Likely a ui `LESSONS` entry banking the ui bench discipline (the daemon's bench-discipline analogue for the Tauri/vitest side).
- **Escalation (conditional)** — if the as-built render is near/over the 500 ms SLO, that's a §18 re-baseline → USER-ruled (`daemon/LESSONS.md` §22), surfaced as a Finding, not a guard tweak.

## How to invoke
1. Read this hand-off — note it is NOT a `/tdd` RED/GREEN cycle (bench-design checkpoints replace Step 2.5).
2. Write `graph.bench.ts` + run `vitest bench` to measure.
3. Surface the bench-design calls (load model / guard / harness) + the measured number to me before finalizing the guard + the cadence wiring.
4. Step 9 → the measured number + the calibrated guard + the cadence; flag a §18 re-baseline if the as-built is near the SLO.

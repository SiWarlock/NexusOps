# ui-020 — §18 Project-Graph render benchmark (the first ui bench) (ui-066)

- **Date:** 2026-06-17
- **Phase:** Phase 6 — **P6.6** (the §18 graph-render benchmark, `ARCHITECTURE.md §18` perf budget < 500 ms)
- **Predecessor:** [ui-019](ui-019-2026-06-17-whole-cockpit-live-pr-workspace-oneof-const.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`

## Why this session existed

A single-slice bench round (lead-directed, away-mode "keep building"): stand up the **first ui benchmark** — the §18 Project-Graph render budget (< 500 ms) — establishing the ui-side bench pattern (the daemon `LESSONS.md` §22 analogue for the Tauri/vitest side). It was sealed as a clean-boundary CYCLE (a fresh implementer pair after).

## What was built (1 commit — `28c4cf8`)

### Files created

- `ui/src/views/graph/graph-bench-load.ts` — the shared load model + render harness: fixtures (100-node typical guard basis + 500-node saturating, built from the real `SessionRow`/`PullRequestRow`/`ProjectActivityRow` shapes with statuses cycled across the frozen enums so the descriptor/attention render path is exercised, not a degenerate fixture) + `renderGraph` (the `bench()` mount+unmount unit) + `GUARD_MS = 150` + `medianRenderMs` (median-of-15 render timing after 3 warmups). Carries neither `bench()` nor `test()` → import-safe in both vitest modes.
- `ui/src/views/graph/graph.bench.ts` — the `vitest bench()` ×2 (typical + saturating) for `vitest bench` reporting (`pnpm bench`). Carries the **verbatim JSDOM honest-proxy caveat**.
- `ui/src/views/graph/graph.bench.guard.ts` — the §18 CI guard `test()` (`expect(medianRenderMs(typical) < 150)`); logs the as-built median for nightly visibility.
- `ui/vitest.bench.config.ts` — a dedicated config (`test.include: ["src/**/*.bench.guard.ts"]`, `environment: "jsdom"`) so `pnpm bench:guard` (`vitest run --config …`) runs ONLY the guard test() — never the `bench()` file (which throws under `vitest run`).

### Files modified

- `ui/package.json` — added `bench` (`vitest bench --run`) + `bench:guard` (`vitest run --config vitest.bench.config.ts`) scripts.
- `.github/workflows/nightly.yml` — a `ui-graph-bench` job (mirrors the daemon `perf` job — non-gating, allowed-to-fail-loudly; `workflow_dispatch` for the `/phase-exit` §18 row): `pnpm bench` (report) + `pnpm bench:guard` (assert).

## Decisions made

- **As-built measured, NO §18 Finding.** Typical 100-node render mean **33.9 ms** (p99 ~40 ms); saturating 500-node mean **145 ms** (p99 ~176 ms) — both comfortably under the 500 ms SLO. No re-baseline needed (the re-baseline is USER-ruled, daemon §22; the guardrail did not trip).
- **Guard = typical render median < 150 ms** — calibrated on the as-built (~34 ms) + margin: ~3.75× over the as-built p99 (absorbs shared-runner slowness + noise), ~3.3× under the SLO (a real ~4× regression trips the guard before the SLO breaches). Saturating reported-only (not guarded — above a typical real graph; a 500-node case on a slow runner could approach the SLO).
- **3-file split (a real vitest constraint).** `bench()` and `test()` CANNOT share a file — `bench()` throws "only available in benchmark mode" under `vitest run` (the guard's test mode). So the bench reporting (`graph.bench.ts`, run by `vitest bench`) and the guard `test()` (`graph.bench.guard.ts`, run by `vitest run --config`, which includes ONLY `*.bench.guard.ts`) are split, sharing the load module. Same mechanism-A intent (self-contained, locally runnable via `pnpm bench:guard`).
- **Own cadence, never per-slice.** `pnpm test:run` excludes `*.bench.ts`/`*.bench.guard.ts` (the default `*.test.*` include — 389 unchanged). The bench runs only at nightly + `/phase-exit` (timing is flaky in a per-slice loop — daemon §22).
- **JSDOM harness (honest proxy).** Documented verbatim in the bench files: measures the React render + DOM-construction + model-derivation cost (the regression-sensitive part) — a meaningful PROXY for the §18 budget; does NOT measure browser layout/paint. A real-browser bench (tauri-driver/playwright) is the heavier follow-up if paint cost ever matters.
- **No `continue-on-error` on the nightly job** — intentional (mirrors the daemon `perf` posture): the two are separate parallel jobs (a ui-bench breach can't mask the daemon perf result), and a scheduled nightly is non-blocking (no PR gate) → a breach is exactly the "red nightly = Finding-to-review" signal.

## Decisions explicitly NOT made / deferred

- **The 6.7 diff-open bench ui-half = DEFERRED (D-8).** The full diff-open bench is cross-track — the dominant cost is the daemon `diff_read.rs` git2-read half; the ui hunk-render half pairs with it. A cross-track pairing, not an in-lane ui slice.
- **The `parentLabelOf` O(n²)** in `ProjectGraph` (a scan per row for the Contained-by column) — a known model inefficiency, **NOT a current issue** (typical render 34 ms, well under SLO). The bench now **guards against it worsening** (a 4× regression trips the 150 ms guard). No premature fix.
- **The bench `SessionRow` fixture omits the ui-062 recovery fields** (`resume_mode`/etc.) — by design: `ProjectGraph` does NOT render them (`resume_mode` is a Sidebar field), so they carry zero graph-render cost; the fixture is faithful to the measured path.
- **A real-browser (paint) bench** — the heavier tauri-driver/playwright follow-up, only if browser layout/paint cost ever becomes the §18 concern (JSDOM is the right call for the render+derivation regression target now).
- **Standing deferred HITL (from ui-019, unchanged):** the ui-064 visual gate (lead manual sign-off), the `/preflight` prettier no-op, the PR-mutations go-live (future cat-1), the ui→main merge (D-3).

## TDD compliance

**N/A — NOT a RED/GREEN slice (an own-cadence performance bench).** Timing assertions are flaky in a per-slice TDD loop (daemon §22), so the flow was: write the bench → measure the as-built → calibrate the guard tighter than the SLO → wire the own cadence. The bench-design checkpoints (load model / guard / harness) replaced Step 2.5 and were orchestrator-approved before finalizing. No production deterministic code changed (the regular suite is unaffected — 389/389).

## Reachability

- The bench has **no production-runtime entry** — its entry is the **CI cadence**: the `ui-graph-bench` nightly job (+ `workflow_dispatch` for `/phase-exit`) invokes `pnpm bench` / `pnpm bench:guard`, which drive the **real production `ProjectGraph`** component (not a proxy; daemon §22). Verified: `pnpm bench:guard` runs the guard test + passes; `pnpm bench` reports both load sizes; `pnpm test:run` excludes both bench files.

## Open follow-ups (Step-9 categorized — routed hot; orchestrator-owned at `/orchestrate-end`)

- **[Convention candidate]** the first ui bench establishes the ui bench-discipline pattern (the daemon §22 analogue): drive the real production component; realistic-gated + saturating-reported; calibrate the guard tighter than the SLO with margin; own-cadence not per-slice; exclude `*.bench.ts`/`*.bench.guard.ts` from `test:run`; the JSDOM honest-proxy caveat; the `bench()`/`test()` 3-file split. → a ui `LESSONS` entry (orchestrator banks).
- **[Future TODO — cross-track]** the 6.7 diff-open bench (D-8) — the daemon `diff_read.rs` half + the ui hunk-render half.
- **[Carry-forward]** the `parentLabelOf` O(n²) (guarded against worsening; no premature fix); a real-browser paint bench (heavier follow-up).
- **Cross-doc invariant change:** NONE this session (a bench is not a contract surface; no shadow/contract field add/remove/rename).

## How to use what was built

- Report: `cd ui && pnpm bench` (the `vitest bench` hz/mean/p99 for typical + saturating).
- Guard (the §18 assert): `cd ui && pnpm bench:guard` (fails if the typical render median ≥ 150 ms).
- CI: the `ui-graph-bench` nightly job runs both; `workflow_dispatch` it before `/phase-exit` for the §18 graph row.

## Quality gate

**389/389 green** (regular suite, bench excluded) · `tsc --noEmit` clean · `oxlint` clean · `pnpm bench:guard` passes (typical median ~34 ms < 150 ms guard).

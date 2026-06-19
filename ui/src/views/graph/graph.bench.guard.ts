// @vitest-environment jsdom
//
// §18 graph-render CI GUARD (ui-066) — the calibrated threshold assert, companion to graph.bench.ts.
// A vitest `test()` (NOT `bench()` — the two can't share a file: `bench()` throws under `vitest run`)
// so it runs via `pnpm bench:guard` (`vitest run --config vitest.bench.config.ts`, which includes ONLY
// `*.bench.guard.ts`). The default `vitest.config.ts` (`*.test.*` include) EXCLUDES it from the per-slice
// `pnpm test:run` — timing assertions are flaky in a per-slice loop (the daemon LESSONS §22 own-cadence
// discipline). Runs at nightly + /phase-exit; a breach is a non-gating Finding-to-review.
import { describe, test, expect } from "vitest";
import { GUARD_MS, medianRenderMs, typicalPrs, typicalSessions } from "./graph-bench-load";

describe("ProjectGraph render §18 guard", () => {
  test("graph_render_typical_under_guard", () => {
    const median = medianRenderMs(typicalSessions, typicalPrs);
    // Report the as-built number for the nightly/phase-exit log (visibility) — the guard is the assert.
    console.log(
      `[graph.bench] §18 typical 100-node render median = ${median.toFixed(1)} ms ` +
        `(guard ${GUARD_MS} ms · SLO 500 ms)`,
    );
    expect(median).toBeLessThan(GUARD_MS);
  });
});

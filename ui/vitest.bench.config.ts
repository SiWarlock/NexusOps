import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config";

// Dedicated config for the OWN-cadence §18 bench GUARD (ui-066 — the first ui bench). It runs the guard
// `test()` files (`*.bench.guard.ts`) AS TESTS so the calibrated §18 threshold assert executes at nightly
// + /phase-exit via `pnpm bench:guard` (`vitest run --config vitest.bench.config.ts`).
//
// It includes ONLY `*.bench.guard.ts` — deliberately NOT `*.bench.ts`, because a `*.bench.ts` file holds
// `bench()` calls that THROW under `vitest run` (test mode). The `bench()` reporting (`graph.bench.ts`)
// runs separately via `pnpm bench` (`vitest bench`, which finds `*.bench.ts` through its own
// `benchmark.include`). The default `vitest.config.ts` `*.test.*` include EXCLUDES both from the
// per-slice `pnpm test:run` (timing is flaky in a per-slice loop — the daemon LESSONS §22 discipline).
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      // jsdom: the guard renders the real ProjectGraph (needs a DOM). Set explicitly here so it does
      // not depend on the per-file `@vitest-environment jsdom` docblock winning the merge order.
      environment: "jsdom",
      include: ["src/**/*.bench.guard.ts"],
      globals: false,
    },
  }),
);

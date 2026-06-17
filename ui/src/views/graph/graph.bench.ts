// @vitest-environment jsdom
//
// §18 Project-Graph render benchmark (P6.6) — the FIRST ui bench (mirrors the daemon's
// `daemon/benches/` discipline, daemon LESSONS §22): drive the REAL production `ProjectGraph` component
// (NOT a proxy) under a defined load model. Reporting via `vitest bench` (`pnpm bench`); the calibrated
// §18 CI guard is the companion `graph.bench.guard.ts` (`pnpm bench:guard`). OWN cadence only (nightly +
// /phase-exit) — `*.bench.ts` is excluded from `pnpm test:run` (the default `*.test.*` include).
//
// HARNESS CAVEAT (honest proxy — the daemon LESSONS §22 "drive the REAL production entry" discipline,
// applied honestly): this bench measures the React render + DOM-construction + model-derivation cost
// (the regression-sensitive part) — a meaningful PROXY for the §18 render budget. It does NOT measure
// browser layout/paint (JSDOM has no layout engine). A real-browser bench (tauri-driver / playwright) is
// a heavier follow-up if paint cost ever becomes the concern. JSDOM is the right call here: the
// testing-library precedent + the existing vitest infra, and the as-built render (~34 ms typical) × a
// generous paint multiplier still sits well under the 500 ms SLO.
import { bench, describe } from "vitest";
import {
  renderGraph,
  saturatingPrs,
  saturatingSessions,
  typicalPrs,
  typicalSessions,
} from "./graph-bench-load";

describe("ProjectGraph render (§18 — graph render < 500 ms)", () => {
  bench("project_graph_render_typical_100_nodes", () => {
    renderGraph(typicalSessions, typicalPrs);
  });

  bench("project_graph_render_saturating_500_nodes", () => {
    renderGraph(saturatingSessions, saturatingPrs);
  });
});

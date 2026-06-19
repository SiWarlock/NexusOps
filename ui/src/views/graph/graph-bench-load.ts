// Shared load model + render harness for the §18 graph-render bench (ui-066).
//
// Split out so BOTH the `bench()` reporting (graph.bench.ts, run by `vitest bench`) AND the guard
// `test()` (graph.bench.guard.ts, run by `vitest run --config vitest.bench.config.ts`) consume one
// fixture/harness without either importing the other — `bench()` throws under `vitest run` (test mode),
// so the bench file and the guard file MUST stay separate; this module carries neither `bench()` nor
// `test()`, so it is import-safe in both modes. No runner collects it directly (the filename matches
// neither the `*.test.*`, `*.bench.ts`, nor `*.bench.guard.ts` include).
//
// Load model (realistic-gated + saturating-reported):
//   typical    = 1 project + 50 sessions + 49 PRs   = 100 nodes / 99 edges  (the guard basis)
//   saturating = 1 project + 250 sessions + 249 PRs = 500 nodes / 499 edges (visibility only)
// `buildProjectGraph` makes one node per project-scoped session + PR + the root, with a root→child edge
// each — so node count is the load knob; these counts bracket a typical→heavy real `proj_project_graph`.
import { render, cleanup } from "@testing-library/react";
import { createElement } from "react";
import { ProjectGraph } from "./ProjectGraph";
import type {
  ProjectActivityRow,
  PullRequestRow,
  SessionRow,
} from "../../contracts/index";

export const PROJECT_ID = "project_bench";
export const projects: ProjectActivityRow[] = [
  { project_id: PROJECT_ID, name: "bench-service" },
];

// Cycle through real frozen-enum statuses so the descriptor/attention path (the real render cost) is
// exercised, not a single repeated status (a degenerate fixture would under-measure).
const SESSION_STATUSES: SessionRow["status"][] = [
  "active",
  "waiting_on_permission",
  "changes_ready",
  "running_tests",
  "idle",
  "completed",
];
const PR_STATUSES: PullRequestRow["status"][] = [
  "open",
  "needs_review",
  "approved",
  "mergeable",
  "merged",
  "checks_failing",
];

function buildSessions(n: number): SessionRow[] {
  return Array.from({ length: n }, (_, i) => ({
    session_id: `session_bench_${i}`,
    status: SESSION_STATUSES[i % SESSION_STATUSES.length]!,
    display_name: `Bench session ${i}`,
    project_id: PROJECT_ID,
  }));
}

function buildPrs(n: number): PullRequestRow[] {
  return Array.from({ length: n }, (_, i) => ({
    pr_id: `repo_bench#${i}`,
    project_id: PROJECT_ID,
    repo_id: "repo_bench",
    pr_number: i,
    title: `Bench PR ${i}`,
    status: PR_STATUSES[i % PR_STATUSES.length]!,
    head_branch: `feat/bench-${i}`,
    base_branch: "main",
    pr_checked_at: null,
    mergeable: i % 2 === 0,
    checks_summary: "3/3 checks passing",
  }));
}

export const typicalSessions = buildSessions(50);
export const typicalPrs = buildPrs(49);
export const saturatingSessions = buildSessions(250);
export const saturatingPrs = buildPrs(249);

/** One mount+unmount of the production ProjectGraph (the as-built render the Shell mounts) — the
 *  `bench()` measured unit. cleanup() tears down between iterations so containers don't accumulate. */
export function renderGraph(
  sessions: SessionRow[],
  pullRequests: PullRequestRow[],
): void {
  render(
    createElement(ProjectGraph, {
      projectId: PROJECT_ID,
      projects,
      sessions,
      pullRequests,
    }),
  );
  cleanup();
}

// The §18 CI guard (own-cadence: nightly + /phase-exit; NEVER `pnpm test:run`). Calibrated tighter than
// the 500 ms SLO with margin (daemon LESSONS §22): the as-built TYPICAL render is ~34 ms (p99 ~40 ms) on
// dev hardware, so a guard at 150 ms sits ~3.75× over the p99 (absorbs shared-runner slowness + noise —
// a breach is a non-gating Finding-to-review, the nightly posture) and ~3.3× under the SLO (a real ~4×
// regression — e.g. the O(n²) `parentLabelOf` worsening — trips the guard BEFORE the SLO breaches).
export const GUARD_MS = 150;
const GUARD_WARMUP = 3;
const GUARD_SAMPLES = 15;

/** Median of GUARD_SAMPLES render timings (the mount + its post-render effect flush timed; teardown NOT
 *  timed), after GUARD_WARMUP warm renders — robust to the per-iteration noise that makes a single timing
 *  flaky. The effect flush (FocusableNode's deps-less useEffect) is consistent across iterations, so it
 *  adds no variance — it is part of the render cost the §18 budget measures. */
export function medianRenderMs(
  sessions: SessionRow[],
  pullRequests: PullRequestRow[],
): number {
  const once = (): number => {
    const t0 = performance.now();
    render(
      createElement(ProjectGraph, {
        projectId: PROJECT_ID,
        projects,
        sessions,
        pullRequests,
      }),
    );
    const elapsed = performance.now() - t0;
    cleanup(); // teardown — NOT timed
    return elapsed;
  };
  for (let i = 0; i < GUARD_WARMUP; i++) once();
  const times = Array.from({ length: GUARD_SAMPLES }, once).toSorted((a, b) => a - b);
  return times[times.length >> 1]!;
}

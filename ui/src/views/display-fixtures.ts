// PROVISIONAL DISPLAY FIXTURES for the daemon-gated view surfaces (the kit-data
// demo content, adapted to this repo's fixture world). Each block backs ONE
// view whose real data needs a daemon contract that hasn't landed:
//   diff/worktrees   → worktree/diff contract (Phase 7 PR review)
//   editor           → worktree file-access contract
//   team             → AgentTeam projection
//   packs            → WorkflowInstance/pack-registry projection
//   plan             → ImplementationPlan projection (workflow-pack parser)
//   brain            → the Project Brain sidecar (Phase 8)
// The views render the prototype's visual treatment over these and FLAG the
// gate in their headers/notes — display-only, never presented as live state
// (forbidden #2 honored by labeling, mutation affordances disabled §11.6).
import type { TermLineKind } from "./cockpit";
import type { RiskLevel } from "../shell/display-meta";

// ── Code / Diff Review ────────────────────────────────────────────────────────
export interface DiffLineFx {
  type: "add" | "del" | "ctx";
  text: string;
  ln?: number;
}
export interface DiffFileFx {
  file: string;
  header: string;
  comments: number;
  lines: DiffLineFx[];
}

export const diffFixture: DiffFileFx[] = [
  {
    file: "src/gateway/review.ts",
    header: "@@ -10,4 +10,6 @@",
    comments: 2,
    lines: [
      { type: "ctx", ln: 10, text: "export async function review(plan: ActionPlan) {" },
      { type: "del", text: "  return execute(plan)" },
      { type: "add", ln: 11, text: "  const dry = await dryRun(plan)" },
      { type: "add", ln: 12, text: '  if (dry.risk === "critical") return gateway.requireTyped(dry)' },
      { type: "add", ln: 13, text: "  return gateway.confirm(dry)" },
      { type: "ctx", ln: 14, text: "}" },
    ],
  },
  {
    file: "src/gateway/risk.ts",
    header: "@@ -3,2 +3,5 @@",
    comments: 0,
    lines: [
      { type: "ctx", ln: 3, text: "export type Risk =" },
      { type: "add", ln: 4, text: "  | 'readonly' | 'low' | 'medium'" },
      { type: "add", ln: 5, text: "  | 'high' | 'critical'" },
    ],
  },
];

export interface WorktreeFx {
  id: string;
  path: string;
  branch: string;
  base: string;
  status: "dirty" | "active" | "conflict";
  dirty: number;
  task?: { id: string; tone: "linear" | "github" | "accent" };
  commit: string;
  pr?: string;
  risk: RiskLevel;
}

export const worktreesFixture: WorktreeFx[] = [
  {
    id: "wt1",
    path: "~/wt/auth-refactor",
    branch: "agent/auth-refactor",
    base: "main",
    status: "dirty",
    dirty: 7,
    task: { id: "ENG-310", tone: "linear" },
    commit: "4f18a70",
    pr: "#101",
    risk: "medium",
  },
  {
    id: "wt2",
    path: "~/wt/rate-limit",
    branch: "agent/rate-limit",
    base: "main",
    status: "dirty",
    dirty: 3,
    task: { id: "ENG-284", tone: "linear" },
    commit: "a91c2d1",
    risk: "low",
  },
  {
    id: "wt3",
    path: "~/wt/flaky-it",
    branch: "fix/flaky-integration",
    base: "main",
    status: "conflict",
    dirty: 4,
    task: { id: "#214", tone: "github" },
    commit: "e22a9c4",
    pr: "#201",
    risk: "high",
  },
];

/** PR display extras the PullRequest projection doesn't carry yet (branch /
 *  diff stats / age — daemon enrichment flagged). Keyed by pr_number. */
export const prDisplayFixture: Record<
  string,
  { branch: string; adds: number; dels: number; files: number; comments: number; age: string }
> = {
  "101": { branch: "agent/auth-refactor", adds: 412, dels: 98, files: 6, comments: 2, age: "2h" },
  "102": { branch: "chore/deps", adds: 64, dels: 12, files: 3, comments: 0, age: "14m" },
  "201": { branch: "fix/flaky-integration", adds: 12, dels: 4, files: 1, comments: 1, age: "6m" },
};

// ── Editor ───────────────────────────────────────────────────────────────────
export interface TreeNodeFx {
  type: "dir" | "file";
  name: string;
  open?: boolean;
  depth: number;
  git?: "M" | "A" | "D";
  active?: boolean;
  agent?: boolean;
}

export const editorTreeFixture: TreeNodeFx[] = [
  { type: "dir", name: "src", open: true, depth: 0 },
  { type: "dir", name: "gateway", open: true, depth: 1, git: "M" },
  { type: "file", name: "review.ts", depth: 2, git: "M", active: true, agent: true },
  { type: "file", name: "risk.ts", depth: 2, git: "M" },
  { type: "file", name: "confirm.ts", depth: 2 },
  { type: "dir", name: "parser", open: false, depth: 1 },
  { type: "dir", name: "brain", open: false, depth: 1 },
  { type: "file", name: "index.ts", depth: 1 },
  { type: "dir", name: "tests", open: false, depth: 0, git: "M" },
  { type: "file", name: "package.json", depth: 0 },
];

export interface EditorFileFx {
  path: string;
  dirty: boolean;
  agent: boolean;
  lines: string[];
  marks: Record<number, "add" | "del" | "ctx">;
  agentEdit: { line: number; who: string; note: string } | null;
  problems: { sev: "warn" | "fail"; text: string; at: string }[];
}

export const editorFilesFixture: Record<string, EditorFileFx> = {
  "review.ts": {
    path: "src/gateway",
    dirty: true,
    agent: true,
    lines: [
      "import { ActionPlan } from './plan'",
      "import { dryRun } from './sandbox'",
      "import { gateway } from './confirm'",
      "",
      "// Risk-gated execution — every action passes the Gateway",
      "export async function review(plan: ActionPlan) {",
      "  const dry = await dryRun(plan)",
      "  if (dry.risk === 'critical') {",
      "    return gateway.requireTyped(dry)",
      "  }",
      "  return gateway.confirm(dry)",
      "}",
      "",
      "export function classify(plan: ActionPlan): Risk {",
      "  return plan.writes ? 'high' : 'readonly'",
      "}",
    ],
    marks: { 6: "add", 7: "add", 8: "add", 9: "add" },
    agentEdit: { line: 8, who: "Claude · ENG-310", note: "inserting critical-risk gate" },
    problems: [
      { sev: "warn", text: "'Risk' type imported but used before its declaration", at: "review.ts:14" },
    ],
  },
  "risk.ts": {
    path: "src/gateway",
    dirty: true,
    agent: false,
    lines: [
      "// Risk taxonomy for the Action Gateway",
      "export type Risk =",
      "  | 'readonly'",
      "  | 'low'",
      "  | 'medium'",
      "  | 'high'",
      "  | 'critical'",
      "",
      "export const ORDER: Risk[] = [",
      "  'readonly', 'low', 'medium', 'high', 'critical'",
      "]",
    ],
    marks: { 5: "add", 6: "add" },
    agentEdit: null,
    problems: [],
  },
  "gateway.test.ts": {
    path: "tests",
    dirty: false,
    agent: false,
    lines: [
      "import { review } from '../src/gateway/review'",
      "import { classify } from '../src/gateway/risk'",
      "",
      "test('critical actions require typed confirm', async () => {",
      "  const plan = { writes: true, risk: 'critical' }",
      "  expect(classify(plan)).toBe('high')",
      "  // snapshot below is stale — regenerate with -u",
      "  expect(await review(plan)).toMatchSnapshot()",
      "})",
    ],
    marks: { 6: "del" },
    agentEdit: null,
    problems: [{ sev: "fail", text: "snapshot mismatch — Risk enum changed", at: "gateway.test.ts:8" }],
  },
  "confirm.ts": {
    path: "src/gateway",
    dirty: false,
    agent: false,
    lines: [
      "import { Risk } from './risk'",
      "",
      "export const gateway = {",
      "  confirm: (dry: DryRun) => emit('approval.request', dry),",
      "  requireTyped: (dry: DryRun) => emit('approval.typed', dry),",
      "}",
    ],
    marks: {},
    agentEdit: null,
    problems: [],
  },
};

// ── Agent Team ───────────────────────────────────────────────────────────────
export interface TeamWorkerFx {
  id: string;
  role: string;
  harness: "claude-code" | "codex-cli" | "codex-cloud" | "shell";
  status: "running" | "waiting-perm" | "completed";
  task: string;
  ctx: number;
  wt: string;
  lines: { k: TermLineKind; t: string }[];
}

export const teamFixture = {
  name: "Phase 2 · Observability Graph",
  pack: "cc-crew",
  branch: "team/phase-2-graph",
  lead: {
    role: "Orchestrator",
    harness: "claude-code" as const,
    profile: "Claude Team Work",
    status: "active" as const,
    task: "decomposing into 3 workstreams",
    ctx: 66,
    lines: [
      { k: "cmd", t: "/team-start backend --profile Claude Team Work" },
      { k: "out", t: "decomposing into 3 workstreams" },
      { k: "out", t: "delegated → renderer, adapters, layout" },
      { k: "live", t: "awaiting worker results" },
    ] as { k: TermLineKind; t: string }[],
  },
  workers: [
    {
      id: "w1",
      role: "Graph renderer",
      harness: "claude-code",
      status: "running",
      task: "editing GraphCanvas.tsx",
      ctx: 44,
      wt: "~/wt/phase-2/w1",
      lines: [
        { k: "cmd", t: "edit GraphCanvas.tsx" },
        { k: "dim", t: "› writing component tree" },
        { k: "live", t: "editing GraphCanvas.tsx" },
      ],
    },
    {
      id: "w2",
      role: "Status adapters",
      harness: "codex-cli",
      status: "waiting-perm",
      task: "wants to install d3-force",
      ctx: 21,
      wt: "~/wt/phase-2/w2",
      lines: [
        { k: "cmd", t: "npm i d3-force" },
        { k: "warn", t: "permission required — install dependency" },
      ],
    },
    {
      id: "w3",
      role: "Layout solver",
      harness: "claude-code",
      status: "completed",
      task: "opened PR #86",
      ctx: 35,
      wt: "~/wt/phase-2/w3",
      lines: [
        { k: "cmd", t: "git push" },
        { k: "dim", t: "› opened PR #86" },
        { k: "out", t: "opened PR #86" },
      ],
    },
  ] as TeamWorkerFx[],
};

// ── Workflow Packs ───────────────────────────────────────────────────────────
export interface PackCommandFx {
  name: string;
  type: "recipe" | "slash";
  needsInstance: boolean;
  creates?: string;
}
export interface PackFx {
  id: string;
  name: string;
  provider: "bundled" | "user" | "third_party";
  version: string;
  instance: "active" | "ready" | "needs_personalization" | "upgrade_available";
  project: string;
  desc: string;
  commands: PackCommandFx[];
  roles: string[];
  parser: string | null;
  recipes: number;
}

export const packsFixture: PackFx[] = [
  {
    id: "cc-crew",
    name: "cc-crew",
    provider: "bundled",
    version: "1.4.0",
    instance: "active",
    project: "auth-service",
    desc: "Multi-agent backend crew — orchestrator delegates to implementer + reviewer workers.",
    commands: [
      { name: "/team-start", type: "recipe", needsInstance: true, creates: "agent team" },
      { name: "/plan-sync", type: "slash", needsInstance: true },
      { name: "/review-pass", type: "slash", needsInstance: false },
    ],
    roles: ["Orchestrator", "Implementer", "Reviewer"],
    parser: "PHASE_PLAN.md",
    recipes: 2,
  },
  {
    id: "docs-refresh",
    name: "docs-refresh",
    provider: "user",
    version: "0.9.0",
    instance: "ready",
    project: "auth-service",
    desc: "Keeps architecture docs synced to code; runs on a schedule or on drift.",
    commands: [{ name: "/docs-drift", type: "slash", needsInstance: false }],
    roles: [],
    parser: null,
    recipes: 1,
  },
  {
    id: "weekly-commit",
    name: "weekly-commit",
    provider: "third_party",
    version: "2.1.0",
    instance: "upgrade_available",
    project: "docs-site",
    desc: "Scheduled commit + summary automation. A newer pack version is available.",
    commands: [{ name: "/weekly", type: "slash", needsInstance: false }],
    roles: [],
    parser: null,
    recipes: 1,
  },
  {
    id: "billing-scaffold",
    name: "billing-scaffold",
    provider: "user",
    version: "0.3.0",
    instance: "needs_personalization",
    project: "billing",
    desc: "Template scaffold. Must be personalized against this project's architecture before its commands can run.",
    commands: [{ name: "/bs-init", type: "recipe", needsInstance: true, creates: "session" }],
    roles: ["Mapper"],
    parser: "CODE_AREAS.yaml",
    recipes: 1,
  },
];

// ── Implementation plan ──────────────────────────────────────────────────────
export interface PlanTaskFx {
  id: string;
  title: string;
  status: "completed" | "in-progress" | "ready" | "todo" | "backlog";
  ac: number;
  files: number;
  anchors: string[];
  task?: { id: string; tone: "linear" | "github" | "accent" };
  session?: boolean;
  team?: boolean;
}
export interface PlanPhaseFx {
  id: string;
  name: string;
  status: "completed" | "active" | "backlog";
  tracks: { id: string; name: string; tasks: PlanTaskFx[] }[];
}

export const planFixture = {
  name: "auth-service — Implementation Plan",
  source: "PHASE_PLAN.md",
  parser: "cc-crew",
  phases: [
    {
      id: "p1",
      name: "Phase 1 · Foundations",
      status: "completed",
      tracks: [
        {
          id: "t1a",
          name: "Track A · Runtime & profiles",
          tasks: [
            { id: "1.1", title: "Local runtime detection", status: "completed", ac: 4, files: 8, anchors: ["ARCHITECTURE.md#runtime"] },
            { id: "1.2", title: "Execution profile manager", status: "completed", ac: 5, files: 11, anchors: ["ARCHITECTURE.md#profiles"] },
          ],
        },
      ],
    },
    {
      id: "p2",
      name: "Phase 2 · Observability Graph",
      status: "active",
      tracks: [
        {
          id: "t2a",
          name: "Track A · Sessions & graph",
          tasks: [
            { id: "2.1", title: "GitHub OAuth callback", status: "in-progress", ac: 3, files: 4, anchors: ["ARCHITECTURE.md#auth"], task: { id: "ENG-310", tone: "linear" }, session: true },
            { id: "2.3", title: "Project observability graph", status: "ready", ac: 4, files: 0, anchors: ["ARCHITECTURE.md#graph"], task: { id: "Phase 2.3", tone: "accent" } },
          ],
        },
        {
          id: "t2b",
          name: "Track B · Status adapters",
          tasks: [
            { id: "2.4", title: "Harness status adapters", status: "in-progress", ac: 6, files: 7, anchors: ["ARCHITECTURE.md#adapters"], team: true },
          ],
        },
      ],
    },
    {
      id: "p3",
      name: "Phase 3 · Action Gateway",
      status: "backlog",
      tracks: [
        {
          id: "t3a",
          name: "Track A · Approvals",
          tasks: [
            { id: "3.1", title: "Action Gateway approval cards", status: "backlog", ac: 5, files: 0, anchors: ["ARCHITECTURE.md#gateway"], task: { id: "Phase 3.1", tone: "accent" } },
            { id: "3.4", title: "Token usage meter accuracy", status: "todo", ac: 2, files: 0, anchors: [], task: { id: "ENG-240", tone: "linear" } },
          ],
        },
      ],
    },
  ] as PlanPhaseFx[],
};

// ── Project Brain ────────────────────────────────────────────────────────────
export interface BrainEvidenceFx {
  kind: "commit" | "pr" | "anchor" | "plantask" | "memory" | "decision";
  label: string;
  sub?: string;
  freshness?: "fresh" | "stale";
}
export interface BrainMsgFx {
  from: "user" | "brain";
  text: string;
  evidence?: BrainEvidenceFx[];
  plan?: { title: string; steps: { risk: RiskLevel; text: string }[] };
}

export const brainThreadFixture: BrainMsgFx[] = [
  { from: "user", text: "Why are PR #101 checks failing?" },
  {
    from: "brain",
    text: "The **unit** check fails because the new `Risk` enum added `'critical'`, but the snapshot in `review.test.ts` wasn't regenerated. Nothing else in the suite is affected.",
    evidence: [
      { kind: "commit", label: "4f18a70", sub: "add critical risk gate" },
      { kind: "pr", label: "#101", sub: "checks failing" },
      { kind: "anchor", label: "review.test.ts#snapshot", freshness: "stale" },
    ],
    plan: {
      title: "Fix snapshot drift",
      steps: [
        { risk: "readonly", text: "Open review.test.ts at failing snapshot" },
        { risk: "low", text: "Regenerate snapshot — jest -u" },
        { risk: "medium", text: "Commit to fix/snapshots and re-run checks" },
      ],
    },
  },
];

export const brainMemoryFixture = [
  { kind: "decision", label: "Gateway-first execution", sub: "all actions risk-rated", t: "2d ago" },
  { kind: "decision", label: "OAuth via callback gate", sub: "ENG-310", t: "4h ago" },
  { kind: "memory", label: "Worktree-per-session", sub: "isolation invariant", t: "1w ago" },
  { kind: "anchor", label: "ARCHITECTURE.md#gateway", sub: "grounded @ 4f18a70", t: "fresh" },
] as { kind: "decision" | "memory" | "anchor"; label: string; sub: string; t: string }[];

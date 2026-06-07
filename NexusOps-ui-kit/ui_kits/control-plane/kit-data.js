/* Sample data for the Control Plane UI kit. Realistic, per the spec. */
window.KIT = {
  projects: [
    { id: 'cp',  name: 'AI Engineering Control Plane', repo: 'org/control-plane', active: 3, waiting: 1, prs: 2, workflow: 'active', brain: 'ready' },
    { id: 'pb',  name: 'Project Brain',        repo: 'org/project-brain', active: 1, waiting: 0, prs: 1, workflow: 'drift', brain: 'indexing' },
    { id: 'cc',  name: 'cc-crew Scaffold Demo', repo: 'org/cc-crew-demo', active: 0, waiting: 0, prs: 0, workflow: 'needs-personalization', brain: 'stale' },
    { id: 'rg',  name: 'RepoGraph Parser',     repo: 'org/repograph', active: 1, waiting: 1, prs: 0, workflow: 'none', brain: 'ready' },
    { id: 'wc',  name: 'Weekly Commit Automation', repo: 'org/weekly-commit', active: 0, waiting: 0, prs: 1, workflow: 'active', brain: 'ready' },
  ],
  sessions: [
    { id: 's1', proj: 'cp', status: 'waiting-human', title: 'ENG-221 · GitHub OAuth callback', harness: 'claude-code', provider: 'claude', profile: 'Claude Max Main',
      task: { id: 'ENG-221', tone: 'linear' }, branch: 'agent/eng-221-oauth', worktree: '~/wt/eng-221', pr: '#84',
      context: { value: 186, max: 200 }, current: '$ npm test — awaiting permission', activity: '2m ago' },
    { id: 's2', proj: 'cp', status: 'running', title: 'GH-184 · parser memory leak', harness: 'codex-cli', provider: 'codex', profile: 'Codex CLI Main',
      task: { id: '#184', tone: 'github' }, branch: 'fix/gh-184-leak', worktree: '~/wt/gh-184',
      context: { value: 96, max: 200 }, current: 'editing src/parser/stream.ts', activity: 'just now' },
    { id: 's3', proj: 'cp', status: 'active', title: 'Phase 2 · Observability Graph', team: true, harness: 'claude-code', provider: 'claude', profile: 'Claude Team Work',
      task: { id: 'P2.3', tone: 'accent' }, branch: 'team/phase-2-graph', worktree: '~/wt/phase-2',
      context: { value: 132, max: 200 }, current: 'orchestrator delegating 3 workers', activity: '40s ago' },
    { id: 's4', proj: 'rg', status: 'failed', title: 'PR checks fix — snapshot drift', harness: 'codex-cli', provider: 'codex', profile: 'Codex Cloud GitHub',
      task: { id: '#71', tone: 'github' }, branch: 'fix/snapshots', worktree: '~/wt/snap',
      context: { value: 58, max: 200 }, current: 'check "unit" failed — exit 1', activity: '6m ago' },
    { id: 's5', proj: 'cp', status: 'completed', title: 'Docs drift refresh', harness: 'claude-code', provider: 'claude', profile: 'Claude Team Work',
      branch: 'chore/docs-drift', pr: '#82', context: { value: 40, max: 200 }, current: 'summarized to Project Brain', activity: '14m ago' },
  ],
  profiles: [
    { name: 'Claude Max Main', provider: 'claude', health: 'active' },
    { name: 'Claude Max Secondary', provider: 'claude', health: 'available' },
    { name: 'Claude Team Work', provider: 'claude', health: 'rate-limited' },
    { name: 'Codex CLI Main', provider: 'codex', health: 'active' },
    { name: 'Codex Cloud GitHub', provider: 'codex', health: 'auth-expired' },
  ],
  prs: [
    { id: '#84', proj: 'cp', title: 'Add workflow command registry', branch: 'agent/eng-221-oauth', base: 'main', lane: 'open', status: 'pr-open', checks: 'failing', author: 'Codex · PR fix', adds: 412, dels: 98, files: 6, comments: 2, age: '2h' },
    { id: '#82', proj: 'cp', title: 'Docs drift refresh', branch: 'chore/docs-drift', base: 'main', lane: 'ready', status: 'pr-ready', checks: 'passing', author: 'Claude · Docs drift', adds: 64, dels: 12, files: 3, comments: 0, age: '14m' },
    { id: '#80', proj: 'cp', title: 'Risk taxonomy + gateway scaffolding', branch: 'feat/risk-enum', base: 'main', lane: 'merged', status: 'pr-open', checks: 'passing', author: 'Claude · ENG-210', adds: 220, dels: 40, files: 9, comments: 5, age: '1d' },
    { id: '#71', proj: 'rg', title: 'Snapshot regeneration', branch: 'fix/snapshots', base: 'main', lane: 'open', status: 'pr-open', checks: 'failing', author: 'Codex · #71', adds: 12, dels: 4, files: 1, comments: 1, age: '6m' },
  ],
  worktrees: [
    { id: 'wt1', proj: 'cp', path: '~/wt/eng-221', branch: 'agent/eng-221-oauth', base: 'main', status: 'dirty', dirty: 7, session: 'ENG-221 · OAuth', task: { id: 'ENG-221', tone: 'linear' }, commit: '4f18a70', pr: '#84', checks: 'failing', risk: 'medium' },
    { id: 'wt2', proj: 'cp', path: '~/wt/gh-184', branch: 'fix/gh-184-leak', base: 'main', status: 'dirty', dirty: 3, session: 'GH-184 · leak', task: { id: '#184', tone: 'github' }, commit: 'a91c2d1', pr: null, checks: null, risk: 'low' },
    { id: 'wt3', proj: 'cp', path: '~/wt/phase-2', branch: 'team/phase-2-graph', base: 'main', status: 'active', dirty: 12, session: 'Phase 2 team', task: { id: 'P2.3', tone: 'accent' }, commit: '7b3f1e0', pr: null, checks: null, risk: 'low' },
    { id: 'wt4', proj: 'rg', path: '~/wt/snap', branch: 'fix/snapshots', base: 'main', status: 'conflict', dirty: 4, session: '#71 · snapshot fix', task: { id: '#71', tone: 'github' }, commit: 'e22a9c4', pr: '#71', checks: 'failing', risk: 'high' },
  ],
  // ---- Usage / cost ----
  usage: {
    today: { tokensIn: 1284000, tokensOut: 412000, spend: 34, spendLimit: 50, sessions: 5, context: 512, contextMax: 1000 },
    // last 14 days of spend ($)
    spend14: [12, 18, 9, 22, 31, 14, 6, 19, 27, 24, 38, 41, 29, 34],
    byProfile: [
      { name: 'Claude Max Main', provider: 'claude', spend: 14.2, tokens: 720000, pct: 42 },
      { name: 'Claude Team Work', provider: 'claude', spend: 9.6, tokens: 480000, pct: 28 },
      { name: 'Codex CLI Main', provider: 'codex', spend: 6.8, tokens: 360000, pct: 20 },
      { name: 'Claude Max Secondary', provider: 'claude', spend: 3.4, tokens: 136000, pct: 10 },
    ],
    topContext: [
      { session: 'ENG-221 · OAuth', value: 186, max: 200 },
      { session: 'Phase 2 · Graph', value: 132, max: 200 },
      { session: 'GH-184 · leak', value: 96, max: 200 },
    ],
  },
  events: [
    { t: '14:22:08', kind: 'approval', actor: 'Claude ENG-221', text: 'requested permission to run npm test', risk: 'medium' },
    { t: '14:21:54', kind: 'git', actor: 'Codex GH-184', text: 'committed 3 files to fix/gh-184-leak' },
    { t: '14:20:11', kind: 'brain', actor: 'Project Brain', text: 'proposed action plan · 4 steps · create worktree + /team-start' },
    { t: '14:18:02', kind: 'pr', actor: 'Codex PR-fix', text: 'opened PR #84 — checks running' },
    { t: '14:15:40', kind: 'workflow', actor: 'cc-crew', text: '/team-start backend launched 3 workers' },
    { t: '14:12:19', kind: 'session', actor: 'Claude Docs', text: 'session completed · summarized to Project Brain' },
  ],
  diff: [
    { file: 'src/gateway/review.ts', header: '@@ -10,4 +10,6 @@', comments: 2, lines: [
      { type: 'ctx', ln: 10, text: 'export async function review(plan: ActionPlan) {' },
      { type: 'del', text: '  return execute(plan)' },
      { type: 'add', ln: 11, text: '  const dry = await dryRun(plan)' },
      { type: 'add', ln: 12, text: '  if (dry.risk === "critical") return gateway.requireTyped(dry)' },
      { type: 'add', ln: 13, text: '  return gateway.confirm(dry)' },
      { type: 'ctx', ln: 14, text: '}' },
    ]},
    { file: 'src/gateway/risk.ts', header: '@@ -3,2 +3,5 @@', comments: 0, lines: [
      { type: 'ctx', ln: 3, text: 'export type Risk =' },
      { type: 'add', ln: 4, text: "  | 'readonly' | 'low' | 'medium'" },
      { type: 'add', ln: 5, text: "  | 'high' | 'critical'" },
    ]},
  ],

  // ---- Editor / IDE ----
  tree: [
    { type: 'dir', name: 'src', open: true, depth: 0 },
    { type: 'dir', name: 'gateway', open: true, depth: 1, git: 'M' },
    { type: 'file', name: 'review.ts', depth: 2, git: 'M', active: true, agent: true },
    { type: 'file', name: 'risk.ts', depth: 2, git: 'M' },
    { type: 'file', name: 'confirm.ts', depth: 2 },
    { type: 'dir', name: 'parser', open: false, depth: 1 },
    { type: 'dir', name: 'brain', open: false, depth: 1 },
    { type: 'file', name: 'index.ts', depth: 1 },
    { type: 'dir', name: 'tests', open: false, depth: 0, git: 'M' },
    { type: 'file', name: 'package.json', depth: 0 },
  ],
  tabs: [
    { name: 'review.ts', path: 'src/gateway', dirty: true, active: true, agent: true },
    { name: 'risk.ts', path: 'src/gateway', dirty: true },
    { name: 'gateway.test.ts', path: 'tests' },
  ],
  // each line: { n, t (text), c (class: kw/str/com/type/fn/num/punct mix handled in-component), mark }
  code: [
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
  // gutter marks by line index (0-based)
  codeMarks: { 6: 'add', 7: 'add', 8: 'add', 9: 'add', 14: 'ctx' },
  agentEdit: { line: 8, who: 'Claude · ENG-221', note: 'inserting critical-risk gate' },
  problems: [
    { sev: 'warn', text: "'Risk' type imported but used before its declaration", at: 'review.ts:14' },
  ],
  // per-file content for the editor (keyed by filename)
  files: {
    'review.ts': {
      path: 'src/gateway', dirty: true, agent: true,
      lines: [
        "import { ActionPlan } from './plan'", "import { dryRun } from './sandbox'", "import { gateway } from './confirm'", "",
        "// Risk-gated execution — every action passes the Gateway", "export async function review(plan: ActionPlan) {",
        "  const dry = await dryRun(plan)", "  if (dry.risk === 'critical') {", "    return gateway.requireTyped(dry)", "  }",
        "  return gateway.confirm(dry)", "}", "", "export function classify(plan: ActionPlan): Risk {", "  return plan.writes ? 'high' : 'readonly'", "}",
      ],
      marks: { 6: 'add', 7: 'add', 8: 'add', 9: 'add' },
      agentEdit: { line: 8, who: 'Claude · ENG-221', note: 'inserting critical-risk gate' },
      problems: [{ sev: 'warn', text: "'Risk' type imported but used before its declaration", at: 'review.ts:14' }],
    },
    'risk.ts': {
      path: 'src/gateway', dirty: true, agent: false,
      lines: [
        "// Risk taxonomy for the Action Gateway", "export type Risk =", "  | 'readonly'", "  | 'low'", "  | 'medium'",
        "  | 'high'", "  | 'critical'", "", "export const ORDER: Risk[] = [", "  'readonly', 'low', 'medium', 'high', 'critical'", "]",
      ],
      marks: { 5: 'add', 6: 'add' },
      agentEdit: null,
      problems: [],
    },
    'gateway.test.ts': {
      path: 'tests', dirty: false, agent: false,
      lines: [
        "import { review } from '../src/gateway/review'", "import { classify } from '../src/gateway/risk'", "",
        "test('critical actions require typed confirm', async () => {", "  const plan = { writes: true, risk: 'critical' }",
        "  expect(classify(plan)).toBe('high')", "  // snapshot below is stale — regenerate with -u", "  expect(await review(plan)).toMatchSnapshot()", "})",
      ],
      marks: { 6: 'del' },
      agentEdit: null,
      problems: [{ sev: 'fail', text: 'snapshot mismatch — Risk enum changed', at: 'gateway.test.ts:8' }],
    },
    'confirm.ts': {
      path: 'src/gateway', dirty: false, agent: false,
      lines: [
        "import { Risk } from './risk'", "", "export const gateway = {", "  confirm: (dry: DryRun) => emit('approval.request', dry),",
        "  requireTyped: (dry: DryRun) => emit('approval.typed', dry),", "}",
      ],
      marks: {},
      agentEdit: null,
      problems: [],
    },
  },

  // ---- Agent Team ----
  team: {
    name: 'Phase 2 · Observability Graph',
    pack: 'cc-crew',
    lead: { role: 'Orchestrator', harness: 'claude-code', profile: 'Claude Team Work', status: 'active', task: 'decomposing into 3 workstreams', ctx: 132 },
    workers: [
      { id: 'w1', role: 'Graph renderer', harness: 'claude-code', status: 'running', task: 'editing GraphCanvas.tsx', ctx: 88, wt: '~/wt/phase-2/w1' },
      { id: 'w2', role: 'Status adapters', harness: 'codex-cli', status: 'waiting-perm', task: 'wants to install d3-force', ctx: 41, wt: '~/wt/phase-2/w2' },
      { id: 'w3', role: 'Layout solver', harness: 'claude-code', status: 'completed', task: 'opened PR #86', ctx: 70, wt: '~/wt/phase-2/w3' },
    ],
  },

  // ---- Action Gateway queue items ----
  gateway: {
    q1: { id: 'q1', risk: 'medium', who: 'Claude · ENG-221 · Claude Max Main', short: 'Permission to run npm test', title: 'Run npm test', cmd: 'npm test', wt: '~/wt/eng-221',
      desc: 'Sandboxed in worktree. No network. Reversible.', conseq: [['terminal', 'Execute test runner (jest) in worktree'], ['folder', 'Read access to src/** and test fixtures'], ['shield-off', 'No writes outside worktree · no push']] },
    q2: { id: 'q2', risk: 'high', who: 'Codex · GH-184 · Codex Cloud', short: 'Approve force-push to fix/gh-184-leak', title: 'Force-push fix/gh-184-leak', cmd: 'git push --force-with-lease', wt: '~/wt/gh-184',
      desc: 'Rewrites remote history on a shared branch.', conseq: [['git-branch', 'Overwrite remote fix/gh-184-leak'], ['users-round', 'Affects anyone tracking the branch'], ['rotate-ccw', 'Reversible only via reflog']] },
    edit: { id: 'edit', risk: 'medium', who: 'Claude · ENG-221 · Claude Max Main', short: 'Apply agent edit to review.ts', title: 'Apply 4-line edit to review.ts', cmd: 'write src/gateway/review.ts', wt: '~/wt/eng-221',
      desc: 'Inserts a critical-risk gate, then runs the suite.', conseq: [['file-code', 'Modify src/gateway/review.ts (+4 −1)'], ['terminal', 'Run jest after applying'], ['shield-off', 'No push']] },
    perm: { id: 'q1', risk: 'medium', who: 'Claude · ENG-221 · Claude Max Main', short: 'Permission to run npm test', title: 'Run npm test', cmd: 'npm test', wt: '~/wt/eng-221',
      desc: 'Sandboxed in worktree. No network. Reversible.', conseq: [['terminal', 'Execute test runner (jest) in worktree'], ['folder', 'Read access to src/** and test fixtures'], ['shield-off', 'No writes outside worktree · no push']] },
  },

  // ---- Project Brain (co-pilot chat) ----
  brainThread: [
    { from: 'user', text: 'Why are PR #84 checks failing?' },
    { from: 'brain', text: "The **unit** check fails because the new `Risk` enum added `'critical'`, but the snapshot in `review.test.ts` wasn't regenerated. Nothing else in the suite is affected.",
      evidence: [
        { kind: 'commit', label: '4f18a70', sub: 'add critical risk gate' },
        { kind: 'pr', label: '#84', sub: 'checks failing' },
        { kind: 'anchor', label: 'review.test.ts#snapshot', freshness: 'stale' },
      ],
      plan: { title: 'Fix snapshot drift', steps: [
        { risk: 'readonly', text: 'Open review.test.ts at failing snapshot' },
        { risk: 'low', text: 'Regenerate snapshot — jest -u' },
        { risk: 'medium', text: 'Commit to fix/snapshots and re-run checks' },
      ]},
    },
  ],
  // canned co-pilot replies keyed by loose intent
  brainReplies: [
    { match: /next|backend|start|task/i, text: "Next up is **PlanTask `phase-2-backend-auth`**. I can spin it up: new worktree, a Claude Code session on Claude Max Main, linked to the plan task, with a generated prompt.",
      evidence: [ { kind: 'plantask', label: 'phase-2-backend-auth', sub: 'ready' }, { kind: 'anchor', label: 'ARCHITECTURE.md#auth' }, { kind: 'memory', label: 'decision: OAuth via gateway' } ],
      plan: { title: 'Start backend auth task', steps: [
        { risk: 'low', text: 'Create worktree agent/p2-backend-auth' },
        { risk: 'low', text: 'Start Claude Code session · Claude Max Main' },
        { risk: 'readonly', text: 'Link session to PlanTask phase-2-backend-auth' },
        { risk: 'medium', text: 'Send generated task prompt + open terminal' },
      ]},
    },
    { match: /test|fail|check|fix/i, text: "The failing check is snapshot drift in `review.test.ts`. Regenerating the snapshot and re-running checks should clear it. Want me to stage that as a plan?",
      evidence: [ { kind: 'pr', label: '#84', sub: 'checks failing' }, { kind: 'commit', label: '4f18a70', sub: 'add critical risk gate' } ] },
    { match: /.*/, text: "Here's what I can ground that in across the project — code, docs, commits, sessions, PRs and plan tasks. Ask me to *Plan* an action and I'll draft it for the Action Gateway.",
      evidence: [ { kind: 'memory', label: '142 indexed objects' }, { kind: 'decision', label: 'gateway-first execution' } ] },
  ],
  brainMemory: [
    { kind: 'decision', label: 'Gateway-first execution', sub: 'all actions risk-rated', t: '2d ago' },
    { kind: 'decision', label: 'OAuth via callback gate', sub: 'ENG-221', t: '4h ago' },
    { kind: 'memory', label: 'Worktree-per-session', sub: 'isolation invariant', t: '1w ago' },
    { kind: 'anchor', label: 'ARCHITECTURE.md#gateway', sub: 'grounded @ 4f18a70', t: 'fresh' },
  ],

  // ---- Audit / Event timeline ----
  audit: [
    { t: '14:22:08', kind: 'approval', proj: 'cp', actor: 'You', target: 'Claude · ENG-221', text: 'approved — run npm test', risk: 'medium', result: 'approved' },
    { t: '14:22:01', kind: 'approval', proj: 'cp', actor: 'Claude · ENG-221', text: 'requested permission to run npm test', risk: 'medium', result: 'pending' },
    { t: '14:21:54', kind: 'git', proj: 'cp', actor: 'Codex · GH-184', text: 'committed 3 files to fix/gh-184-leak', meta: '4f18a70' },
    { t: '14:21:30', kind: 'session', proj: 'cp', actor: 'Codex · GH-184', text: 'session resumed after rate-limit backoff' },
    { t: '14:20:11', kind: 'brain', proj: 'cp', actor: 'Project Brain', text: 'proposed action plan · 4 steps · create worktree + /team-start' },
    { t: '14:19:40', kind: 'session', proj: 'rg', actor: 'Codex · #71', text: 'check "unit" failed — snapshot drift', result: 'pending' },
    { t: '14:19:02', kind: 'gateway', proj: 'cp', actor: 'You', target: 'Codex · GH-184', text: 'denied — force-push to main', risk: 'critical', result: 'denied' },
    { t: '14:18:02', kind: 'pr', proj: 'cp', actor: 'Codex · PR-fix', text: 'opened PR #84 — checks running', meta: '#84' },
    { t: '14:16:30', kind: 'git', proj: 'rg', actor: 'Codex · #71', text: 'created worktree ~/wt/snap', meta: 'fix/snapshots' },
    { t: '14:15:40', kind: 'workflow', proj: 'cp', actor: 'cc-crew', text: '/team-start backend launched 3 workers' },
    { t: '14:14:21', kind: 'profile', proj: 'cp', actor: 'Runtime', text: 'Claude Team Work entered rate-limited state' },
    { t: '14:12:19', kind: 'session', proj: 'cp', actor: 'Claude · Docs', text: 'session completed · summarized to Project Brain' },
    { t: '14:09:50', kind: 'git', proj: 'cp', actor: 'Claude · ENG-221', text: 'created worktree ~/wt/eng-221', meta: 'agent/eng-221-oauth' },
  ],
  auditFilters: ['All', 'Approvals', 'Git', 'Sessions', 'Brain', 'Workflow'],

  // ---- Settings / Execution profiles ----
  profilesDetail: [
    { name: 'Claude Max Main', provider: 'claude', health: 'active', harness: 'claude-code', sessions: 2, usage: 62, limit: 100, resets: '—', note: 'Primary interactive profile' },
    { name: 'Claude Max Secondary', provider: 'claude', health: 'available', harness: 'claude-code', sessions: 0, usage: 8, limit: 100, resets: '—', note: 'Overflow / parallel work' },
    { name: 'Claude Team Work', provider: 'claude', health: 'rate-limited', harness: 'claude-code', sessions: 1, usage: 98, limit: 100, resets: 'in 24m', note: 'Shared team seat' },
    { name: 'Codex CLI Main', provider: 'codex', health: 'active', harness: 'codex-cli', sessions: 1, usage: 34, limit: 80, resets: '—', note: 'Local CLI harness' },
    { name: 'Codex Cloud GitHub', provider: 'codex', health: 'auth-expired', harness: 'codex-cloud', sessions: 0, usage: 0, limit: 80, resets: '—', note: 'Re-auth required' },
  ],

  // ---- Workflow Packs (pack ≠ instance) ----
  packs: [
    { id: 'cc-crew', name: 'cc-crew', provider: 'bundled', version: '1.4.0', instance: 'active', project: 'Control Plane',
      desc: 'Multi-agent backend crew — orchestrator delegates to implementer + reviewer workers.',
      commands: [
        { name: '/team-start', type: 'recipe', needsInstance: true, creates: 'agent team' },
        { name: '/plan-sync', type: 'slash', needsInstance: true },
        { name: '/review-pass', type: 'slash', needsInstance: false },
      ],
      roles: ['Orchestrator', 'Implementer', 'Reviewer'], parser: 'PHASE_PLAN.md', recipes: 2, drift: null },
    { id: 'docs-refresh', name: 'docs-refresh', provider: 'user', version: '0.9.0', instance: 'ready', project: 'Control Plane',
      desc: 'Keeps architecture docs synced to code; runs on a schedule or on drift.',
      commands: [{ name: '/docs-drift', type: 'slash', needsInstance: false }], roles: [], parser: null, recipes: 1, drift: null },
    { id: 'weekly-commit', name: 'weekly-commit', provider: 'third_party', version: '2.1.0', instance: 'upgrade_available', project: 'Weekly Commit Automation',
      desc: 'Scheduled commit + summary automation. A newer pack version is available.',
      commands: [{ name: '/weekly', type: 'slash', needsInstance: false }], roles: [], parser: null, recipes: 1, drift: 'upgrade' },
    { id: 'rg-scaffold', name: 'repograph-scaffold', provider: 'user', version: '0.3.0', instance: 'needs_personalization', project: 'RepoGraph Parser',
      desc: 'Template scaffold. Must be personalized against this project’s architecture before its commands can run.',
      commands: [{ name: '/rg-init', type: 'recipe', needsInstance: true, creates: 'session' }], roles: ['Mapper'], parser: 'CODE_AREAS.yaml', recipes: 1, drift: null },
  ],

  // ---- Task intake (GitHub issues · Linear tickets · plan tasks) ----
  tasks: [
    { source: 'linear', id: 'ENG-221', title: 'Add GitHub OAuth callback', priority: 'High', labels: ['auth', 'backend'], status: 'In progress', planTask: 'Phase 2.1', harness: 'claude-code', profile: 'Claude Max Main', branch: 'agent/eng-221-oauth', session: 's1' },
    { source: 'github', id: '#184', title: 'Fix parser memory leak', priority: 'Urgent', labels: ['bug', 'parser'], status: 'In progress', planTask: null, harness: 'codex-cli', profile: 'Codex CLI Main', branch: 'fix/gh-184-leak', session: 's2' },
    { source: 'plan', id: 'Phase 2.3', title: 'Project observability graph', priority: 'High', labels: ['phase-2'], status: 'Ready', planTask: null, harness: 'claude-code', profile: 'Claude Team Work' },
    { source: 'plan', id: 'Phase 3.1', title: 'Action Gateway approval cards', priority: 'Medium', labels: ['phase-3', 'gateway'], status: 'Backlog', planTask: null, harness: 'claude-code', profile: 'Claude Max Main' },
    { source: 'linear', id: 'ENG-240', title: 'Token usage meter accuracy', priority: 'Medium', labels: ['usage'], status: 'Todo', planTask: 'Phase 3.4', harness: 'claude-code', profile: 'Claude Max Secondary' },
    { source: 'github', id: '#190', title: 'Workflow pack registry schema', priority: 'Low', labels: ['workflow'], status: 'Todo', planTask: null, harness: 'codex-cli', profile: 'Codex CLI Main' },
    { source: 'linear', id: 'ENG-205', title: 'Audit trail export to NDJSON', priority: 'Low', labels: ['audit'], status: 'Done', planTask: null, harness: 'claude-code', profile: 'Claude Max Main' },
  ],

  // ---- Implementation plan (Phase → Track → PlanTask) ----
  plan: {
    name: 'Control Plane — Implementation Plan',
    source: 'PHASE_PLAN.md', parser: 'cc-crew',
    phases: [
      { id: 'p1', name: 'Phase 1 · Foundations', status: 'completed', tracks: [
        { id: 't1a', name: 'Track A · Runtime & profiles', tasks: [
          { id: '1.1', title: 'Local runtime detection', status: 'completed', ac: 4, files: 8, anchors: ['ARCHITECTURE.md#runtime'], task: null },
          { id: '1.2', title: 'Execution profile manager', status: 'completed', ac: 5, files: 11, anchors: ['ARCHITECTURE.md#profiles'], task: null },
        ]},
      ]},
      { id: 'p2', name: 'Phase 2 · Observability Graph', status: 'active', tracks: [
        { id: 't2a', name: 'Track A · Sessions & graph', tasks: [
          { id: '2.1', title: 'GitHub OAuth callback', status: 'in-progress', ac: 3, files: 4, anchors: ['ARCHITECTURE.md#auth'], task: { id: 'ENG-221', tone: 'linear' }, session: 's1' },
          { id: '2.3', title: 'Project observability graph', status: 'ready', ac: 4, files: 0, anchors: ['ARCHITECTURE.md#graph'], task: { id: 'Phase 2.3', tone: 'accent' } },
        ]},
        { id: 't2b', name: 'Track B · Status adapters', tasks: [
          { id: '2.4', title: 'Harness status adapters', status: 'in-progress', ac: 6, files: 7, anchors: ['ARCHITECTURE.md#adapters'], task: null, team: true },
        ]},
      ]},
      { id: 'p3', name: 'Phase 3 · Action Gateway', status: 'backlog', tracks: [
        { id: 't3a', name: 'Track A · Approvals', tasks: [
          { id: '3.1', title: 'Action Gateway approval cards', status: 'backlog', ac: 5, files: 0, anchors: ['ARCHITECTURE.md#gateway'], task: { id: 'Phase 3.1', tone: 'accent' } },
          { id: '3.4', title: 'Token usage meter accuracy', status: 'todo', ac: 2, files: 0, anchors: [], task: { id: 'ENG-240', tone: 'linear' } },
        ]},
      ]},
    ],
  },

  // ---- Human Input Queue (centralized triage) ----
  humanInput: [
    { id: 'q1', group: 'Permission requests', kind: 'gateway', risk: 'medium', actor: 'Claude · ENG-221', target: '~/wt/eng-221', reason: 'Run npm test in the worktree sandbox', age: '2m' },
    { id: 'q2', group: 'High-risk actions', kind: 'gateway', risk: 'high', actor: 'Codex · GH-184', target: 'fix/gh-184-leak', reason: 'Force-push rewrites shared remote history', age: '5m' },
    { id: 'dec-71', group: 'Failed checks · needs decision', kind: 'decision', risk: 'medium', actor: 'Codex · #71', target: 'RepoGraph Parser', reason: 'unit check failed — snapshot drift. Retry, regenerate, or skip?', age: '6m' },
    { id: 'pers-rg', group: 'Workflow personalization', kind: 'personalization', risk: 'low', actor: 'repograph-scaffold', target: 'RepoGraph Parser', reason: 'Template pack must be personalized before its commands can run', age: '1h' },
  ],

  // ---- Integrations (Settings) ----
  integrations: [
    { id: 'runtime', name: 'Local runtime', icon: 'cpu', connected: true, detail: 'Healthy · 3 sessions · worktree root ~/wt', action: 'Manage' },
    { id: 'github', name: 'GitHub', icon: 'github', connected: true, detail: 'org · 12 repos · mapped org/control-plane', scope: 'Issues · PRs · checks', action: 'Manage' },
    { id: 'linear', name: 'Linear', icon: 'square-kanban', connected: false, detail: 'Not connected — link a team to pull tickets into the Task Inbox', scope: null, action: 'Connect' },
    { id: 'brain', name: 'Project Brain store', icon: 'brain', connected: true, detail: 'Ready · 142 objects indexed · grounded @ 4f18a70', action: 'Manage' },
  ],
};

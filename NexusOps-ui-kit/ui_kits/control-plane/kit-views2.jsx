/* ============================================================
   Control Plane UI kit — graph, terminal & diff views + inspector.
   ============================================================ */
const _NS2 = window.ControlPlaneDesignSystem_a21911;
const { Button: GBtn, IconButton: GIconBtn, StatusPill: GPill,
        RiskBadge: GRisk, UsageMeter: GMeter, HarnessBadge: GHarness,
        ProfileBadge: GProfile, MetaChip: GMeta, GraphNode: GNode,
        EvidenceChip: GEvidence, DiffHunk: GDiff } = _NS2;
const GBadge = _NS2.Badge || (({ children, mono, tone, size, variant, style = {} }) => (
  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, height: 18, padding: '0 6px', borderRadius: 'var(--r-1)', background: 'var(--neutral-surface)', color: 'var(--text-secondary)', font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>
));
const { Ico: GIco, Eyebrow: GEye } = window.KitShell;
const KD2 = window.KIT;

/* ---------------- Project Graph ---------------- */
const NODES = [
  { id: 'proj', proj: 'cp', kind: 'project', title: 'Control Plane', subtitle: 'org/control-plane', status: 'active', x: 40, y: 150, owner: null,
    detail: { Repo: 'org/control-plane', Sessions: '3 active', PRs: '2 open', Workflow: 'cc-crew · active' }, open: { view: 'command', label: 'Open project' } },
  { id: 's1', proj: 'cp', kind: 'session', title: 'ENG-221 · OAuth', subtitle: 'claude-code · max-main', status: 'waiting-human', beacon: true, x: 250, y: 60, meta: ['93% ctx'],
    detail: { Harness: 'Claude Code', Profile: 'Claude Max Main', Branch: 'agent/eng-221-oauth', Context: '186k / 200k', Activity: '2m ago' }, open: { view: 'terminal', sel: 's1', label: 'Open terminal' } },
  { id: 's2', proj: 'cp', kind: 'session', title: 'GH-184 · leak', subtitle: 'codex-cli', status: 'running', x: 250, y: 230, meta: ['48% ctx'],
    detail: { Harness: 'Codex CLI', Profile: 'Codex CLI Main', Branch: 'fix/gh-184-leak', Context: '96k / 200k', Activity: 'just now' }, open: { view: 'terminal', sel: 's2', label: 'Open terminal' } },
  { id: 'team', proj: 'cp', kind: 'team', title: 'Phase 2 team', subtitle: '1 lead · 3 workers', status: 'active', x: 250, y: 380,
    detail: { Pack: 'cc-crew', Lead: 'Orchestrator', Workers: '3', Waiting: '1 · permission', Branch: 'team/phase-2-graph' }, open: { view: 'terminal', team: true, label: 'Open all team terminals' } },
  { id: 'wt1', proj: 'cp', kind: 'worktree', title: 'agent/eng-221', status: 'active', x: 480, y: 60, meta: ['+412 −98'],
    detail: { Path: '~/wt/eng-221', Branch: 'agent/eng-221-oauth', Diff: '+412 −98', Dirty: 'yes' }, open: { view: 'editor', label: 'Open in editor' } },
  { id: 'pr1', proj: 'cp', kind: 'pr', title: '#84 registry', subtitle: 'Codex', status: 'pr-open', x: 690, y: 60,
    detail: { Number: '#84', Author: 'Codex · PR-fix', Checks: 'failing', Diff: '+412 −98' }, open: { view: 'code', label: 'Open review' } },
  { id: 'wt2', proj: 'cp', kind: 'worktree', title: 'fix/gh-184', status: 'active', x: 480, y: 230, meta: ['+96 −12'],
    detail: { Path: '~/wt/gh-184', Branch: 'fix/gh-184-leak', Diff: '+96 −12', Dirty: 'yes' }, open: { view: 'editor', label: 'Open in editor' } },
  { id: 'brain', proj: 'cp', kind: 'brain', title: '3 evidence', subtitle: 'grounded @ 4f18a70', status: 'idle', x: 480, y: 380,
    detail: { Grounded: '@ 4f18a70', Evidence: '3 objects', Freshness: 'fresh' }, open: { view: 'brain', label: 'Open Project Brain' } },
  /* RepoGraph Parser project */
  { id: 'rgproj', proj: 'rg', kind: 'project', title: 'RepoGraph Parser', subtitle: 'org/repograph', status: 'active', x: 60, y: 150,
    detail: { Repo: 'org/repograph', Sessions: '1 active', PRs: '0 open', Workflow: 'none' }, open: { view: 'command', label: 'Open project' } },
  { id: 'rgs', proj: 'rg', kind: 'session', title: '#71 · snapshot fix', subtitle: 'codex-cloud', status: 'failed', x: 300, y: 150, meta: ['29% ctx'],
    detail: { Harness: 'Codex Cloud', Profile: 'Codex Cloud GitHub', Branch: 'fix/snapshots', Checks: 'failing', Activity: '6m ago' }, open: { view: 'terminal', sel: 's4', label: 'Open terminal' } },
  { id: 'rgwt', proj: 'rg', kind: 'worktree', title: 'fix/snapshots', status: 'active', x: 540, y: 150, meta: ['+12 −4'],
    detail: { Path: '~/wt/snap', Branch: 'fix/snapshots', Diff: '+12 −4', Dirty: 'yes' }, open: { view: 'editor', label: 'Open in editor' } },
];
const EDGES = [
  ['proj', 's1', 'active'], ['proj', 's2', 'active'], ['proj', 'team', 'active'],
  ['s1', 'wt1', 'active'], ['wt1', 'pr1', 'active'], ['s2', 'wt2', 'active'],
  ['team', 'brain', 'evidence'],
  ['rgproj', 'rgs', 'active'], ['rgs', 'rgwt', 'blocked'],
];

function ProjectGraph({ sel, setSel, onInspect, project = 'cp', projectName = 'Project' }) {
  const nodes = project === 'all' ? [] : NODES.filter(n => n.proj === project);
  const ids = new Set(nodes.map(n => n.id));
  const edges = EDGES.filter(([a, b]) => ids.has(a) && ids.has(b));
  const byId = Object.fromEntries(nodes.map(n => [n.id, n]));
  const isAll = project === 'all';
  const edgeColor = { active: 'var(--graph-edge-active)', evidence: 'var(--graph-edge-evidence)', blocked: 'var(--graph-edge-blocked)' };
  return (
    <div style={{ position: 'relative', height: '100%', background: 'var(--graph-canvas)', overflow: 'auto',
      backgroundImage: 'radial-gradient(var(--graph-grid) 1px, transparent 1px)', backgroundSize: '22px 22px' }}>
      <div style={{ position: 'sticky', top: 0, zIndex: 5, display: 'flex', alignItems: 'center', gap: 10, padding: '12px 16px', background: 'linear-gradient(var(--graph-canvas), transparent)' }}>
        <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>Project Graph</h1>
        <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)' }}>{projectName}</span>
        <GBadge tone="neutral" mono>{nodes.length} nodes</GBadge>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
          <GBtn variant="ghost" size="sm" icon={<GIco n="filter" />}>Filter</GBtn>
          <GBtn variant="secondary" size="sm" icon={<GIco n="maximize" />}>Fit</GBtn>
        </div>
      </div>
      {nodes.length === 0 ? (
        <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 10, color: 'var(--text-muted)' }}>
          <GIco n="workflow" s={{ width: 26, height: 26, color: 'var(--text-faint)' }} />
          <div style={{ font: 'var(--fw-medium) var(--fs-body) var(--font-sans)' }}>{isAll ? 'The graph is per-project' : `No observability graph for ${projectName} yet`}</div>
          <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-faint)' }}>{isAll ? 'Pick a single project from the switcher to see its graph.' : 'Start a session or run a workflow to populate it.'}</div>
        </div>
      ) : (
      <React.Fragment>
      <svg style={{ position: 'absolute', inset: 0, width: 920, height: 480, pointerEvents: 'none' }}>
        {edges.map(([a, b, rel], i) => {
          const na = byId[a], nb = byId[b];
          const x1 = na.x + 84, y1 = na.y + 26, x2 = nb.x, y2 = nb.y + 26;
          const mx = (x1 + x2) / 2;
          return <path key={i} d={`M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`} fill="none"
            stroke={edgeColor[rel] || 'var(--graph-edge)'} strokeWidth="1.5" strokeDasharray={rel === 'evidence' ? '4 3' : undefined} />;
        })}
      </svg>
      <div style={{ position: 'relative', width: 920, height: 480 }}>
        {nodes.map(n => (
          <div key={n.id} style={{ position: 'absolute', left: n.x, top: n.y }}>
            <GNode kind={n.kind} title={n.title} subtitle={n.subtitle} status={n.status}
              beacon={n.beacon} meta={n.meta} selected={sel === n.id} onClick={() => { setSel(n.id); onInspect && onInspect(n); }} />
          </div>
        ))}
      </div>
      </React.Fragment>
      )}
    </div>
  );
}

const NODE_KIND_ICON = { project: 'folder-git-2', session: 'bot', team: 'users-round', worktree: 'folder-git-2', pr: 'git-pull-request', brain: 'brain' };
window.NODE_KIND_ICON = NODE_KIND_ICON;

/* ---------------- Session Terminal ---------------- */
function SessionTerminal({ sel, openGateway, overrides = {}, team = false }) {
  if (team) return <TeamTerminal openGateway={openGateway} />;
  const base = KD2.sessions.find(x => x.id === sel) || KD2.sessions[0];
  const s = overrides[base.id] ? { ...base, status: overrides[base.id] } : base;
  const waiting = s.status === 'waiting-human';
  const lines = [
    { k: 'cmd', t: `/team-start backend --profile ${s.profile}` },
    { k: 'out', t: 'resolved worktree ' + (s.worktree || '~/wt/session') },
    { k: 'out', t: 'harness ' + s.harness + ' attached · context window 200k' },
    { k: 'cmd', t: 'npm test -- --runInBand' },
    { k: 'dim', t: '› PASS  src/gateway/risk.test.ts (12)' },
    { k: 'dim', t: '› PASS  src/gateway/review.test.ts (8)' },
    { k: waiting ? 'warn' : 'live', t: s.current },
  ];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--surface-canvas)' }}>
      {/* header */}
      <div style={{ padding: '11px 16px', borderBottom: '1px solid var(--border-default)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-faint)', marginBottom: 7 }}>
          <span>{s.task ? s.task.id : 'Session'}</span><GIco n="chevron-right" s={{ width: 12, height: 12 }} /><span style={{ color: 'var(--text-secondary)' }}>{s.branch}</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <GPill status={s.status} beacon={waiting} size="md" />
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>{s.title}</h1>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
            <GBtn variant="ghost" size="sm" icon={<GIco n="git-pull-request" />}>Open PR</GBtn>
            <GBtn variant="secondary" size="sm" icon={<GIco n="pause" />}>Pause</GBtn>
          </div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 9, flexWrap: 'wrap' }}>
          <GHarness harness={s.harness} /><GProfile name={s.profile} provider={s.provider} />
          {s.branch && <GMeta tone="branch">{s.branch}</GMeta>}
          {s.worktree && <GMeta tone="worktree">{s.worktree}</GMeta>}
          {s.pr && <GMeta tone="pr">{s.pr}</GMeta>}
        </div>
      </div>
      {/* terminal well */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '12px 16px', background: 'var(--surface-sunken)', boxShadow: 'var(--elev-inset)',
        fontFamily: 'var(--font-mono)', fontSize: 'var(--fs-body)' }}>
        {lines.map((l, i) => <TermLine key={i} l={l} />)}
        {waiting && <PermissionPrompt s={s} openGateway={openGateway} />}
      </div>
    </div>
  );
}

/* ---------------- Team terminals (all panes at once) ---------------- */
function TeamTerminal({ openGateway }) {
  const t = KD2.team;
  const agents = [
    { id: 'lead', role: t.lead.role, harness: t.lead.harness, status: t.lead.status, lead: true, lines: [
      { k: 'cmd', t: '/team-start backend --profile ' + t.lead.profile }, { k: 'out', t: 'decomposing into 3 workstreams' },
      { k: 'out', t: 'delegated → renderer, adapters, layout' }, { k: 'live', t: 'awaiting worker results' } ] },
    ...t.workers.map(w => ({ id: w.id, role: w.role, harness: w.harness, status: w.status, wt: w.wt, lines:
      w.status === 'running' ? [{ k: 'cmd', t: 'edit ' + w.task.replace('editing ', '') }, { k: 'dim', t: '› writing component tree' }, { k: 'live', t: w.task }]
      : w.status === 'waiting-perm' ? [{ k: 'cmd', t: 'npm i d3-force' }, { k: 'warn', t: 'permission required — install dependency' }]
      : [{ k: 'cmd', t: 'git push' }, { k: 'dim', t: '› opened PR #86' }, { k: 'out', t: w.task }] })),
  ];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--surface-canvas)', minHeight: 0 }}>
      <div style={{ padding: '11px 16px', borderBottom: '1px solid var(--border-default)', display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ color: 'var(--teal-ink)' }}><GIco n="users-round" /></span>
        <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>{t.name}</h1>
        <GMeta tone="branch">team/phase-2-graph</GMeta>
        <GBadge mono style={{ color: 'var(--text-muted)' }}>{agents.length} terminals</GBadge>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
          <GBtn variant="ghost" size="sm" icon={<GIco n="layout-grid" />}>Split</GBtn>
          <GBtn variant="secondary" size="sm" icon={<GIco n="git-merge" />}>Integrate</GBtn>
        </div>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: 'grid', gridTemplateColumns: '1fr 1fr', gridTemplateRows: '1fr 1fr', gap: 1, background: 'var(--border-default)' }}>
        {agents.map(a => <AgentTermPane key={a.id} a={a} openGateway={openGateway} />)}
      </div>
    </div>
  );
}

function AgentTermPane({ a, openGateway }) {
  const waiting = a.status === 'waiting-perm';
  return (
    <div style={{ display: 'flex', flexDirection: 'column', minHeight: 0, background: 'var(--surface-canvas)' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '7px 11px', background: 'var(--surface-panel)', borderBottom: '1px solid var(--border-subtle)' }}>
        <span style={{ color: a.lead ? 'var(--teal-ink)' : 'var(--text-faint)' }}><GIco n={a.lead ? 'workflow' : 'bot'} s={{ width: 14, height: 14 }} /></span>
        <span style={{ font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', color: 'var(--text-primary)' }}>{a.role}</span>
        <GPill status={a.status} size="xs" beacon={waiting} />
        <span style={{ marginLeft: 'auto' }}><GHarness harness={a.harness} showLabel={false} /></span>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '9px 11px', background: 'var(--surface-sunken)', boxShadow: 'var(--elev-inset)', fontFamily: 'var(--font-mono)', fontSize: 'var(--fs-meta)' }}>
        {a.lines.map((l, i) => <TermLine key={i} l={l} />)}
        {waiting && (
          <div style={{ marginTop: 9, border: '1px solid var(--attention-line)', background: 'var(--attention-surface)', borderRadius: 'var(--r-2)', padding: '8px 9px', fontFamily: 'var(--font-sans)' }}>
            <div style={{ font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)', color: 'var(--attention-ink)', marginBottom: 7 }}>Permission · install d3-force</div>
            <GBtn variant="attention" size="xs" icon={<GIco n="check" s={{ width: 12, height: 12 }} />} onClick={() => openGateway('q2')}>Grant</GBtn>
          </div>
        )}
      </div>
    </div>
  );
}

const TERM_COLORS = { cmd: 'var(--text-primary)', out: 'var(--text-secondary)', dim: 'var(--text-faint)', live: 'var(--live-ink)', warn: 'var(--attention-ink)' };
function TermLine({ l }) {
  const prefix = l.k === 'cmd' ? '$ ' : l.k === 'live' ? '▶ ' : l.k === 'warn' ? '◆ ' : '';
  const prefixColor = l.k === 'cmd' ? 'var(--accent-ink)' : 'inherit';
  return (
    <div style={{ color: TERM_COLORS[l.k], lineHeight: '22px', minHeight: 22, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
      {prefix && <span style={{ color: prefixColor, animation: l.k === 'live' ? 'cp-live-pulse 1.6s var(--ease-inout) infinite' : undefined }}>{prefix}</span>}
      {l.t}
    </div>
  );
}

function PermissionPrompt({ s, openGateway }) {
  return (
    <div style={{ marginTop: 12, border: '1px solid var(--attention-line)', background: 'var(--attention-surface)', borderRadius: 'var(--r-3)', padding: '12px 13px', fontFamily: 'var(--font-sans)' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <GRisk level="medium" /><span style={{ font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)', color: 'var(--attention-ink)' }}>Permission required</span>
      </div>
      <div style={{ font: 'var(--fs-body)/1.5 var(--font-sans)', color: 'var(--text-secondary)', marginBottom: 11 }}>
        Session wants to run <code style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-primary)', background: 'var(--surface-active)', padding: '1px 5px', borderRadius: 3 }}>npm test</code> — a sandboxed command in this worktree.
      </div>
      <div style={{ display: 'flex', gap: 7 }}>
        <GBtn variant="attention" size="sm" icon={<GIco n="check" />} onClick={() => openGateway('q1')} kbd="⏎">Approve once</GBtn>
        <GBtn variant="secondary" size="sm">Always allow tests</GBtn>
        <GBtn variant="ghost" size="sm">Deny</GBtn>
      </div>
    </div>
  );
}

/* ---------------- Code & Delivery (Review · Worktrees · PRs) ---------------- */
function DiffReview({ openBrain, project = 'cp', openGateway }) {
  const [tab, setTab] = React.useState('Review');
  React.useEffect(() => { const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24); return () => clearTimeout(t); }, [tab]);
  const tabs = ['Review', 'Worktrees', 'Pull requests'];
  const counts = {
    Worktrees: (KD2.worktrees || []).filter(w => project === 'all' || w.proj === project).length,
    'Pull requests': (KD2.prs || []).filter(p => project === 'all' || p.proj === project).length,
  };
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--surface-canvas)', minHeight: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 2, padding: '10px 16px 0', borderBottom: '1px solid var(--border-subtle)' }}>
        <span style={{ color: 'var(--text-secondary)', marginRight: 8, display: 'inline-flex' }}><GIco n="git-pull-request" /></span>
        {tabs.map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: '8px 12px', border: 'none', background: 'transparent', cursor: 'pointer', font: `${tab === t ? 'var(--fw-semibold)' : 'var(--fw-medium)'} var(--fs-label) var(--font-sans)`, color: tab === t ? 'var(--accent-ink)' : 'var(--text-muted)', boxShadow: tab === t ? 'inset 0 -2px 0 var(--accent-solid)' : 'none' }}>
            {t}{counts[t] != null ? <span style={{ marginLeft: 6, font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>{counts[t]}</span> : null}
          </button>
        ))}
      </div>
      <div style={{ flex: 1, minHeight: 0 }}>
        {tab === 'Review' && <ReviewTab openBrain={openBrain} />}
        {tab === 'Worktrees' && <WorktreesTab project={project} openGateway={openGateway} />}
        {tab === 'Pull requests' && <PRsTab project={project} openReview={() => setTab('Review')} openGateway={openGateway} />}
      </div>
    </div>
  );
}

function ReviewTab({ openBrain }) {
  const files = KD2.diff;
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '220px 1fr', height: '100%', background: 'var(--surface-canvas)' }}>
      <aside style={{ borderRight: '1px solid var(--border-default)', background: 'var(--surface-panel)', overflowY: 'auto', padding: '12px 8px' }}>
        <GEye style={{ padding: '0 8px 10px' }}>Changed files</GEye>
        {files.map((f, i) => (
          <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 7, padding: '6px 8px', borderRadius: 'var(--r-2)', background: i === 0 ? 'var(--surface-active)' : 'transparent', cursor: 'pointer', marginBottom: 2 }}>
            <GIco n="file-code" s={{ width: 13, height: 13, color: 'var(--text-faint)' }} />
            <span style={{ flex: 1, font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{f.file.split('/').pop()}</span>
            {f.comments > 0 && <GBadge tone="review" size="xs" mono>{f.comments}</GBadge>}
          </div>
        ))}
        <div style={{ marginTop: 'auto', padding: '10px 8px 0' }}>
          <GBadge tone="success" variant="dot">+476</GBadge> <GBadge tone="danger" variant="dot">−110</GBadge>
        </div>
      </aside>
      <div style={{ overflowY: 'auto', padding: '14px 16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 14 }}>
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>Review · PR #84</h1>
          <GPill status="failing" /><GMeta tone="pr">#84</GMeta>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
            <GBtn variant="secondary" size="sm" icon={<GIco n="brain" />} onClick={openBrain}>Ask Brain</GBtn>
            <GBtn variant="primary" size="sm" icon={<GIco n="check" />}>Approve PR</GBtn>
          </div>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
          {files.map((f, i) => (
            <GDiff key={i} file={f.file} header={f.header} lines={f.lines} comments={f.comments}
              onAsk={openBrain} onRequestFix={() => {}} />
          ))}
        </div>
      </div>
    </div>
  );
}

/* ---- Worktrees tab ---- */
const WT_STATUS = { dirty: { pill: 'dirty', label: 'Dirty' }, active: { pill: 'active', label: 'Clean' }, conflict: { pill: 'conflict', label: 'Conflict' } };
function WorktreesTab({ project, openGateway }) {
  const rows = (KD2.worktrees || []).filter(w => project === 'all' || w.proj === project);
  return (
    <div style={{ height: '100%', overflowY: 'auto', padding: '14px 16px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
        <GEye>Worktrees</GEye><GBadge mono style={{ color: 'var(--text-muted)' }}>{rows.length}</GBadge>
        <div style={{ marginLeft: 'auto' }}><GBtn variant="secondary" size="sm" icon={<GIco n="plus" />}>New worktree</GBtn></div>
      </div>
      {rows.length === 0 ? <div style={{ font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-faint)', padding: '16px 2px' }}>No worktrees for this project.</div> : (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {rows.map(w => {
          const st = WT_STATUS[w.status] || WT_STATUS.active;
          return (
            <div key={w.id} style={{ border: `1px solid ${w.status === 'conflict' ? 'var(--danger-line)' : 'var(--border-default)'}`, borderRadius: 'var(--r-3)', background: 'var(--surface-card)', padding: '11px 13px', display: 'flex', alignItems: 'center', gap: 12 }}>
              <span style={{ width: 30, height: 30, flex: 'none', borderRadius: 'var(--r-2)', background: 'var(--surface-active)', color: 'var(--teal-ink)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><GIco n="folder-git-2" s={{ width: 15, height: 15 }} /></span>
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
                  <code style={{ font: 'var(--fw-medium) var(--fs-label) var(--font-mono)', color: 'var(--text-primary)' }}>{w.path}</code>
                  <GPill status={st.pill} size="xs" label={st.label} />
                  {w.dirty > 0 && <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--caution-ink)' }}>{w.dirty} changed</span>}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 5, flexWrap: 'wrap' }}>
                  <GMeta tone="branch">{w.branch}</GMeta>
                  <span style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>← {w.base}</span>
                  {w.task && <GMeta tone={w.task.tone === 'github' ? 'github' : w.task.tone === 'linear' ? 'linear' : 'accent'} mono={false}>{w.task.id}</GMeta>}
                  <GMeta icon={<GIco n="git-commit-horizontal" s={{ width: 12, height: 12 }} />}>{w.commit}</GMeta>
                  {w.pr && <GMeta tone="pr">{w.pr}</GMeta>}
                  {w.checks && <GPill status={w.checks === 'passing' ? 'passing' : 'failing'} size="xs" />}
                </div>
              </div>
              <GRisk level={w.risk} />
              {w.status === 'conflict'
                ? <GBtn variant="danger" size="sm" icon={<GIco n="git-merge" s={{ width: 13, height: 13 }} />} onClick={() => openGateway && openGateway('q2')}>Resolve</GBtn>
                : <GIconBtn icon={<GIco n="chevron-right" s={{ width: 15, height: 15 }} />} size="sm" aria-label="Open" />}
            </div>
          );
        })}
      </div>)}
    </div>
  );
}

/* ---- Pull requests tab (lanes) ---- */
const LANES = [['open', 'Open', 'review'], ['ready', 'Ready to merge', 'success'], ['merged', 'Merged', 'review']];
function PRsTab({ project, openReview, openGateway }) {
  const prs = (KD2.prs || []).filter(p => project === 'all' || p.proj === project);
  return (
    <div style={{ height: '100%', overflowY: 'auto', padding: '14px 16px' }}>
      <div style={{ display: 'flex', gap: 12, alignItems: 'flex-start', minWidth: 0 }}>
        {LANES.map(([lane, label, tone]) => {
          const items = prs.filter(p => p.lane === lane);
          return (
            <div key={lane} style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 7, padding: '0 2px 9px' }}>
                <span style={{ width: 7, height: 7, borderRadius: 999, background: `var(--${tone}-solid)` }} />
                <span style={{ font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-muted)' }}>{label}</span>
                <span style={{ font: '10px var(--font-mono)', color: 'var(--text-faint)' }}>{items.length}</span>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {items.map(p => (
                  <div key={p.id} onClick={openReview} style={{ cursor: 'pointer', border: '1px solid var(--border-default)', borderRadius: 'var(--r-3)', background: 'var(--surface-card)', padding: '11px 12px', display: 'flex', flexDirection: 'column', gap: 8 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
                      <GMeta tone="pr">{p.id}</GMeta>
                      <GPill status={p.checks === 'passing' ? 'passing' : 'failing'} size="xs" />
                      <span style={{ marginLeft: 'auto', font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>{p.age}</span>
                    </div>
                    <div style={{ font: 'var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)', color: 'var(--text-primary)' }}>{p.title}</div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>
                      <span style={{ color: 'var(--diff-add-ink)' }}>+{p.adds}</span>
                      <span style={{ color: 'var(--diff-del-ink)' }}>−{p.dels}</span>
                      <span>· {p.files} files</span>
                      {p.comments > 0 && <span>· 🗩 {p.comments}</span>}
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <GMeta tone="branch">{p.branch}</GMeta>
                      {lane === 'ready' && <GBtn variant="primary" size="xs" icon={<GIco n="git-merge" s={{ width: 12, height: 12 }} />} onClick={(e) => { e.stopPropagation(); openGateway && openGateway('q2'); }} style={{ marginLeft: 'auto' }}>Merge</GBtn>}
                    </div>
                  </div>
                ))}
                {items.length === 0 && <div style={{ font: 'var(--fs-micro) var(--font-sans)', color: 'var(--text-faint)', padding: '6px 2px', border: '1px dashed var(--border-subtle)', borderRadius: 'var(--r-2)', textAlign: 'center' }}>None</div>}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

window.KitViews2 = { ProjectGraph, SessionTerminal, DiffReview };

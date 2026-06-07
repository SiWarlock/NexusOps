/* ============================================================
   Control Plane UI kit — main views.
   CommandCenter · ProjectGraph · SessionTerminal · DiffReview
   + SessionInspector. Composes DS components from the window NS.
   ============================================================ */
const _NS = window.ControlPlaneDesignSystem_a21911;
const { Button: VBtn, IconButton: VIconBtn, StatusPill: VPill,
        RiskBadge: VRisk, UsageMeter: VMeter, AttentionMarker: VMark,
        HarnessBadge: VHarness, ProfileBadge: VProfile, MetaChip: VMeta,
        SessionRow: VSessionRow, GraphNode: VNode, EvidenceChip: VEvidence,
        DiffHunk: VDiff } = _NS;
const VBadge = _NS.Badge || (({ children, mono, tone, style = {} }) => (
  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, height: 18, padding: '0 6px', borderRadius: 'var(--r-1)', background: 'var(--neutral-surface)', color: 'var(--text-secondary)', font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>
));
const { Ico: VIco, Eyebrow: VEye } = window.KitShell;
const KD = window.KIT;

const panel = { background: 'var(--surface-canvas)', overflow: 'hidden' };
const sectionPad = { padding: '14px 16px' };

/* ---------------- Command Center ---------------- */
function CommandCenter({ setSel, setView, openGateway, queue, overrides = {}, project = 'cp', onTaskDrop }) {
  const proj = KD.projects.find(p => p.id === project) || { id: 'all', name: 'All projects' };
  const projSessions = project === 'all' ? KD.sessions : KD.sessions.filter(s => s.proj === project);
  const sessions = projSessions.map(s => overrides[s.id] ? { ...s, status: overrides[s.id] } : s);
  const attention = sessions.filter(s => ['waiting-human', 'failed'].includes(s.status));
  const working = sessions.filter(s => ['running', 'active'].includes(s.status));
  const done = sessions.filter(s => ['completed', 'idle', 'archived'].includes(s.status));
  const open = (id) => { const s = projSessions.find(x => x.id === id); setSel(id); setView(s && s.team ? 'team' : 'terminal'); };
  const [dragOver, setDragOver] = React.useState(false);
  const onColDrop = (e) => { e.preventDefault(); setDragOver(false); const t = window.__dragTask; if (t && onTaskDrop) onTaskDrop(t, null); };

  return (
    <div style={{ ...panel, display: 'grid', gridTemplateColumns: '1fr 300px', height: '100%' }}>
      <div style={{ overflowY: 'auto', position: 'relative' }}
        onDragOver={(e) => { if (window.__dragTask) { e.preventDefault(); setDragOver(true); } }}
        onDragLeave={(e) => { if (e.currentTarget === e.target) setDragOver(false); }}
        onDrop={onColDrop}>
        {dragOver && (
          <div style={{ position: 'absolute', inset: 6, zIndex: 20, pointerEvents: 'none', border: '2px dashed var(--accent-line)', borderRadius: 'var(--r-3)', background: 'var(--accent-surface)', display: 'flex', alignItems: 'center', justifyContent: 'center', font: 'var(--fw-medium) var(--fs-body) var(--font-sans)', color: 'var(--accent-ink)' }}>
            Drop to dispatch a new session
          </div>
        )}
        <div style={{ ...sectionPad, display: 'flex', alignItems: 'center', gap: 10, borderBottom: '1px solid var(--border-subtle)' }}>
          <button onClick={() => setView('projects')} title="All projects" style={{ display: 'inline-flex', alignItems: 'center', gap: 6, padding: '3px 8px 3px 6px', borderRadius: 'var(--r-1)', border: '1px solid var(--border-default)', background: 'var(--surface-card)', cursor: 'pointer', font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)' }}>
            <VIco n="chevron-left" s={{ width: 13, height: 13 }} /> Projects
          </button>
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h2)/1.1 var(--font-sans)', letterSpacing: 'var(--tracking-tight)' }}>{proj.name}</h1>
          <VBadge tone="neutral" mono>{sessions.length} sessions</VBadge>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 7 }}>
            <VBtn variant="ghost" size="sm" icon={<VIco n="arrow-up-down" />}>Sort: attention</VBtn>
            <VBtn variant="primary" size="sm" icon={<VIco n="plus" />}>New session</VBtn>
          </div>
        </div>

        {/* Needs my attention */}
        <div style={sectionPad}>
          <VEye style={{ marginBottom: 10, color: attention.length ? 'var(--attention-ink)' : 'var(--text-faint)' }}>● Needs my attention</VEye>
          {attention.length === 0
            ? <div style={{ font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: 7, padding: '6px 0' }}><VIco n="check-check" s={{ width: 15, height: 15, color: 'var(--success-ink)' }} /> All clear. No session is waiting on you.</div>
            : <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
                {attention.map(s => (
                  <AttentionCard key={s.id} s={s} onOpen={() => open(s.id)} openGateway={openGateway} />
                ))}
              </div>}
        </div>

        {/* Working */}
        <div style={sectionPad}>
          <VEye style={{ marginBottom: 8 }}>▶ Working now</VEye>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {working.map(s => <SessionRowWrap key={s.id} s={s} onOpen={() => open(s.id)} onTaskDrop={onTaskDrop} />)}
          </div>
        </div>

        {/* Settled */}
        <div style={sectionPad}>
          <VEye style={{ marginBottom: 8 }}>Recently settled</VEye>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6, opacity: 0.85 }}>
            {done.map(s => <SessionRowWrap key={s.id} s={s} onOpen={() => open(s.id)} onTaskDrop={onTaskDrop} />)}
          </div>
        </div>
      </div>

      {/* Right rail */}
      <CommandRail openGateway={openGateway} queue={queue} />
    </div>
  );
}

function AttentionCard({ s, onOpen, openGateway }) {
  const waiting = s.status === 'waiting-human';
  return (
    <div style={{ position: 'relative', border: `1px solid ${waiting ? 'var(--attention-line)' : 'var(--danger-line)'}`,
      background: waiting ? 'var(--attention-surface)' : 'var(--danger-surface)', borderRadius: 'var(--r-3)', padding: '11px 12px 12px', overflow: 'hidden' }}>
      <span style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: 3, background: waiting ? 'var(--attention-solid)' : 'var(--danger-solid)' }} />
      <div style={{ display: 'flex', alignItems: 'center', gap: 7, marginBottom: 8 }}>
        <VPill status={s.status} beacon={waiting} />
        <span style={{ marginLeft: 'auto' }}><VHarness harness={s.harness} showLabel={false} /></span>
      </div>
      <div style={{ font: 'var(--fw-medium) var(--fs-body)/1.3 var(--font-sans)', marginBottom: 6 }}>{s.title}</div>
      <div style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)', marginBottom: 11, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{s.current}</div>
      <div style={{ display: 'flex', gap: 6 }}>
        {waiting
          ? <><VBtn variant="attention" size="sm" icon={<VIco n="check" />} onClick={() => openGateway('q1')}>Review &amp; approve</VBtn>
              <VBtn variant="ghost" size="sm" onClick={onOpen}>Open</VBtn></>
          : <><VBtn variant="secondary" size="sm" icon={<VIco n="refresh-cw" />}>Retry checks</VBtn>
              <VBtn variant="ghost" size="sm" onClick={onOpen}>Open log</VBtn></>}
      </div>
    </div>
  );
}

function SessionRowWrap({ s, onOpen, onTaskDrop }) {
  const [over, setOver] = React.useState(false);
  return (
    <div onDragOver={(e) => { if (window.__dragTask) { e.preventDefault(); e.stopPropagation(); setOver(true); } }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => { e.preventDefault(); e.stopPropagation(); setOver(false); const t = window.__dragTask; if (t && onTaskDrop) onTaskDrop(t, s); }}
      style={{ borderRadius: 'var(--r-2)', boxShadow: over ? 'inset 0 0 0 2px var(--accent-line)' : 'none' }}>
      <VSessionRow status={s.status} title={s.title} harness={s.harness} profile={s.profile}
        task={s.task} branch={s.branch} worktree={s.worktree} pr={s.pr}
        context={s.context} current={s.current} activity={s.activity}
        team={s.team} onClick={onOpen} />
    </div>
  );
}

function CommandRail({ openGateway, queue = [] }) {
  return (
    <aside style={{ borderLeft: '1px solid var(--border-default)', background: 'var(--surface-panel)', overflowY: 'auto', display: 'flex', flexDirection: 'column' }}>
      <div style={{ ...sectionPad, borderBottom: '1px solid var(--border-subtle)' }}>
        <VEye style={{ marginBottom: 10 }}>Human input queue {queue.length > 0 && <span style={{ color: 'var(--attention-ink)' }}>· {queue.length}</span>}</VEye>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {queue.length === 0
            ? <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-faint)', padding: '6px 2px', display: 'flex', alignItems: 'center', gap: 6 }}><VIco n="check-check" s={{ width: 14, height: 14, color: 'var(--success-ink)' }} /> Queue clear — nothing waiting on you.</div>
            : queue.map(q => <QueueItem key={q.id} risk={q.risk} who={q.who} text={q.short} onClick={() => openGateway(q.id)} />)}
        </div>
      </div>
      <div style={{ ...sectionPad, borderBottom: '1px solid var(--border-subtle)' }}>
        <VEye style={{ marginBottom: 10 }}>Capacity</VEye>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <VMeter label="Aggregate context" value={512} max={1000} valueText="512k / 1M" />
          <VMeter label="Spend today" value={34} max={50} valueText="$34 / $50" accuracy="estimated" />
          <VMeter label="Active runtime" value={3} max={6} valueText="3 / 6 agents" />
        </div>
      </div>
      <div style={{ ...sectionPad }}>
        <VEye style={{ marginBottom: 10 }}>Recent events</VEye>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {KD.events.slice(0, 6).map((e, i) => <EventLine key={i} e={e} />)}
        </div>
      </div>
    </aside>
  );
}

function QueueItem({ risk, who, text, onClick }) {
  return (
    <button onClick={onClick} style={{ textAlign: 'left', cursor: 'pointer', border: '1px solid var(--attention-line)', background: 'var(--attention-surface)', borderRadius: 'var(--r-2)', padding: '9px 10px', display: 'flex', flexDirection: 'column', gap: 7 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <VRisk level={risk} />
        <span style={{ marginLeft: 'auto', font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-muted)' }}>{who}</span>
      </div>
      <div style={{ font: 'var(--fw-medium) var(--fs-label)/1.35 var(--font-sans)', color: 'var(--text-primary)' }}>{text}</div>
    </button>
  );
}

const EVENT_ICON = { approval: 'shield-question', git: 'git-commit-horizontal', brain: 'brain', pr: 'git-pull-request', workflow: 'workflow', session: 'circle-check' };
function EventLine({ e }) {
  return (
    <div style={{ display: 'flex', gap: 8, padding: '5px 0', alignItems: 'flex-start' }}>
      <span style={{ marginTop: 1, color: e.kind === 'brain' ? 'var(--brain-ink)' : e.kind === 'approval' ? 'var(--attention-ink)' : 'var(--text-faint)' }}><VIco n={EVENT_ICON[e.kind] || 'dot'} s={{ width: 13, height: 13 }} /></span>
      <div style={{ minWidth: 0 }}>
        <div style={{ font: 'var(--fs-meta)/1.4 var(--font-sans)', color: 'var(--text-secondary)' }}>{e.text}</div>
        <div style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)', marginTop: 1 }}>{e.t} · {e.actor}</div>
      </div>
    </div>
  );
}

/* ---------------- Projects overview (top-layer) ---------------- */
const WF_TONE = { active: ['Active', 'var(--teal-ink)', 'var(--teal-surface)', 'var(--teal-line)'], drift: ['Drift', 'var(--warning-ink)', 'var(--warning-surface)', 'var(--warning-line)'], 'needs-personalization': ['Needs setup', 'var(--caution-ink)', 'var(--caution-surface)', 'var(--caution-line)'], none: ['No pack', 'var(--text-faint)', 'var(--neutral-surface)', 'var(--border-default)'] };
const BRAIN_TONE = { ready: ['Brain ready', 'var(--brain-ink)'], indexing: ['Indexing', 'var(--live-ink)'], stale: ['Brain stale', 'var(--warning-ink)'] };

function ProjectsOverview({ onSelectProject, active }) {
  return (
    <div style={{ height: '100%', overflowY: 'auto', background: 'var(--surface-canvas)' }}>
      <div style={{ padding: '16px', borderBottom: '1px solid var(--border-subtle)', display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ color: 'var(--text-secondary)' }}><VIco n="layout-grid" /></span>
        <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h2)/1 var(--font-sans)', letterSpacing: 'var(--tracking-tight)' }}>Projects</h1>
        <VBadge tone="neutral" mono>{KD.projects.length}</VBadge>
        <div style={{ marginLeft: 'auto' }}><VBtn variant="primary" size="sm" icon={<VIco n="plus" />}>Add project</VBtn></div>
      </div>
      <div style={{ padding: '16px', display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))', gap: 12 }}>
        {KD.projects.map(p => {
          const wf = WF_TONE[p.workflow] || WF_TONE.none;
          const br = BRAIN_TONE[p.brain] || BRAIN_TONE.ready;
          return (
            <button key={p.id} onClick={() => onSelectProject(p.id)} style={{ textAlign: 'left', cursor: 'pointer', border: `1px solid ${active === p.id ? 'var(--accent-line)' : 'var(--border-default)'}`, borderRadius: 'var(--r-3)', background: 'var(--surface-card)', padding: '14px 15px', display: 'flex', flexDirection: 'column', gap: 11, boxShadow: active === p.id ? 'inset 0 0 0 1px var(--accent-line)' : 'none' }}>
              <div style={{ display: 'flex', alignItems: 'flex-start', gap: 9 }}>
                <span style={{ width: 30, height: 30, flex: 'none', borderRadius: 'var(--r-2)', background: 'var(--surface-active)', color: 'var(--text-secondary)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><VIco n="folder-git-2" s={{ width: 16, height: 16 }} /></span>
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div style={{ font: 'var(--fw-semibold) var(--fs-body-lg)/1.25 var(--font-sans)', color: 'var(--text-primary)' }}>{p.name}</div>
                  <div style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)', marginTop: 2 }}>{p.repo}</div>
                </div>
                {p.waiting > 0 && <span title="waiting on human" style={{ display: 'inline-flex', alignItems: 'center', gap: 4, height: 18, padding: '0 6px', borderRadius: 999, background: 'var(--attention-solid)', color: 'var(--attention-on-solid)', font: 'var(--fw-semibold) var(--fs-micro) var(--font-mono)' }}>◆ {p.waiting}</span>}
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 14, font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>
                <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}><span style={{ width: 6, height: 6, borderRadius: 999, background: 'var(--live-solid)' }} />{p.active} active</span>
                <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}><VIco n="git-pull-request" s={{ width: 12, height: 12 }} />{p.prs} PR</span>
                <span style={{ marginLeft: 'auto', color: br[1] }}>{br[0]}</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5, height: 20, padding: '0 8px', borderRadius: 'var(--r-1)', background: wf[2], border: `1px solid ${wf[3]}`, color: wf[1], font: 'var(--fw-medium) var(--fs-micro) var(--font-sans)' }}><VIco n="package" s={{ width: 12, height: 12 }} /> {wf[0]}</span>
                <span style={{ marginLeft: 'auto', font: 'var(--fs-meta) var(--font-sans)', color: 'var(--accent-ink)', display: 'inline-flex', alignItems: 'center', gap: 3 }}>Open <VIco n="arrow-right" s={{ width: 13, height: 13 }} /></span>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}

window.KitViews = { CommandCenter, ProjectsOverview };

/* ============================================================
   Control Plane UI kit — interactive desktop shell.
   Composes the design-system components (window namespace) + KIT data.
   ============================================================ */
const NS = window.ControlPlaneDesignSystem_a21911;
const { Button, IconButton, StatusPill, RiskBadge, UsageMeter, AttentionMarker,
        HarnessBadge, ProfileBadge, MetaChip, SessionRow, EvidenceChip, GraphNode, DiffHunk } = NS;
const { useState, useEffect } = React;
const K = window.KIT;

const Ico = ({ n, s }) => <i data-lucide={n} style={{ width: 16, height: 16, ...s }}></i>;
const Eyebrow = ({ children, style }) => (
  <div style={{ font: 'var(--fw-semibold) var(--fs-micro)/1 var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-faint)', ...style }}>{children}</div>
);

/* ---------------- Top bar ---------------- */
function ProjectSwitcher({ proj, project, onSelect }) {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  }, [open]);
  const items = [{ id: 'all', name: 'All projects', repo: `${K.projects.length} projects`, active: 0, waiting: 0, all: true }, ...K.projects];
  const wfTone = { active: 'var(--teal-ink)', drift: 'var(--warning-ink)', 'needs-personalization': 'var(--caution-ink)', none: 'var(--text-faint)' };
  return (
    <div style={{ position: 'relative' }} onClick={(e) => e.stopPropagation()}>
      <button onClick={() => setOpen(o => !o)} title="Switch project" style={{ display: 'inline-flex', alignItems: 'center', gap: 7, height: 28, padding: '0 9px', borderRadius: 'var(--r-2)', border: `1px solid ${open ? 'var(--border-strong)' : 'var(--border-default)'}`, background: 'var(--surface-input)', cursor: 'pointer' }}>
        <Ico n={proj.all ? 'layout-grid' : 'folder-git-2'} s={{ width: 14, height: 14, color: 'var(--text-muted)' }} />
        <span style={{ font: 'var(--fw-medium) var(--fs-label)/1 var(--font-sans)', color: 'var(--text-primary)', maxWidth: 220, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{proj.name}</span>
        <Ico n="chevrons-up-down" s={{ width: 13, height: 13, color: 'var(--text-faint)' }} />
      </button>
      {open && (
        <div style={{ position: 'absolute', top: 32, left: 0, zIndex: 'var(--z-popover)', width: 320, background: 'var(--surface-card)', border: '1px solid var(--border-strong)', borderRadius: 'var(--r-3)', boxShadow: 'var(--elev-3)', overflow: 'hidden', padding: '5px', animation: 'cp-pop-in 0.14s var(--ease-out)' }}>
          <div style={{ font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-faint)', padding: '6px 8px 4px' }}>Switch project</div>
          {items.map(p => {
            const on = project === p.id;
            return (
              <button key={p.id} onClick={() => { onSelect(p.id); setOpen(false); }} style={{ display: 'flex', alignItems: 'center', gap: 9, width: '100%', padding: '7px 8px', borderRadius: 'var(--r-2)', border: 'none', cursor: 'pointer', textAlign: 'left', background: on ? 'var(--accent-surface)' : 'transparent' }}>
                <Ico n={p.all ? 'layout-grid' : 'folder-git-2'} s={{ width: 15, height: 15, color: on ? 'var(--accent-ink)' : 'var(--text-muted)' }} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', color: on ? 'var(--accent-ink)' : 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.name}</div>
                  <div style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>{p.repo}</div>
                </div>
                {!p.all && p.waiting > 0 && <span title="waiting" style={{ width: 7, height: 7, borderRadius: 999, background: 'var(--attention-solid)' }} />}
                {!p.all && p.active > 0 && <span style={{ font: '10px var(--font-mono)', color: 'var(--text-muted)' }}>{p.active}</span>}
                {!p.all && <span title={'workflow: ' + p.workflow} style={{ width: 6, height: 6, borderRadius: 2, background: wfTone[p.workflow] || 'var(--text-faint)' }} />}
                {on && <Ico n="check" s={{ width: 14, height: 14, color: 'var(--accent-ink)' }} />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

function TopBar({ view, setView, proj, project, onSelectProject, onBack, canBack, onForward, canForward, onBrain, onGateway, onPalette, onHumanInput, onTasks, waiting }) {
  proj = proj || { name: 'Project', repo: '', active: 0, prs: 0, brain: 'ready' };
  return (
    <header style={{ gridArea: 'top', display: 'flex', alignItems: 'center', gap: 10, padding: '0 12px', height: 'var(--shell-topbar-h)', background: 'var(--surface-panel)', borderBottom: '1px solid var(--border-default)' }}>
      <div style={{ display: 'flex', gap: 6, alignItems: 'center', paddingRight: 2 }}>
        <span style={{ display: 'flex', gap: 6 }}>
          {['#3b3b40', '#3b3b40', '#3b3b40'].map((c, i) => <span key={i} style={{ width: 11, height: 11, borderRadius: 999, background: c, border: '1px solid var(--border-strong)' }} />)}
        </span>
      </div>
      <div style={{ display: 'flex', gap: 1 }}>
        <button onClick={onBack} disabled={!canBack} title="Back" style={{ display: 'inline-flex', alignItems: 'center', justifyContent: 'center', width: 26, height: 26, borderRadius: 'var(--r-2)', border: 'none', background: 'transparent', cursor: canBack ? 'pointer' : 'default', color: canBack ? 'var(--text-secondary)' : 'var(--text-faint)', opacity: canBack ? 1 : 0.4 }}><Ico n="arrow-left" s={{ width: 16, height: 16 }} /></button>
        <button onClick={onForward} disabled={!canForward} title="Forward" style={{ display: 'inline-flex', alignItems: 'center', justifyContent: 'center', width: 26, height: 26, borderRadius: 'var(--r-2)', border: 'none', background: 'transparent', cursor: canForward ? 'pointer' : 'default', color: canForward ? 'var(--text-secondary)' : 'var(--text-faint)', opacity: canForward ? 1 : 0.4 }}><Ico n="arrow-right" s={{ width: 16, height: 16 }} /></button>
      </div>
      <span style={{ width: 1, height: 18, background: 'var(--border-default)' }} />
      <ProjectSwitcher proj={proj} project={project} onSelect={onSelectProject} />
      {proj.repo && <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)' }}>{proj.repo}</span>}
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 11, marginLeft: 2, font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>
        <span title="active sessions" style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}><span style={{ width: 6, height: 6, borderRadius: 999, background: 'var(--live-solid)' }} />{proj.active}</span>
        <span title="open PRs" style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}><Ico n="git-pull-request" s={{ width: 12, height: 12 }} />{proj.prs}</span>
        {waiting > 0 && <span title="waiting on you" style={{ display: 'inline-flex', alignItems: 'center', gap: 4, color: 'var(--attention-ink)' }}>◆ {waiting}</span>}
      </span>
      <button onClick={onPalette} style={{ marginLeft: 'auto', width: 320, display: 'flex', alignItems: 'center', gap: 8, height: 28, padding: '0 10px', borderRadius: 'var(--r-2)', background: 'var(--surface-input)', border: '1px solid var(--border-default)', color: 'var(--text-muted)', cursor: 'text', font: 'var(--fs-label) var(--font-sans)' }}>
        <Ico n="search" s={{ width: 14, height: 14 }} /> Search or run a command…
        <kbd style={{ marginLeft: 'auto', padding: '1px 5px', borderRadius: 3, background: 'var(--surface-active)', border: '1px solid var(--border-default)', font: '10px var(--font-mono)' }}>⌘K</kbd>
      </button>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span title="Local runtime healthy" style={{ display: 'inline-flex', alignItems: 'center', gap: 5, padding: '0 8px', height: 24, borderRadius: 999, border: '1px solid var(--success-line)', background: 'var(--success-surface)', font: '10px var(--font-mono)', color: 'var(--success-ink)' }}><span style={{ width: 6, height: 6, borderRadius: 999, background: 'var(--success-solid)' }} /> runtime</span>
        <IconButton label="Task Inbox" onClick={onTasks}><Ico n="inbox" /></IconButton>
        <IconButton label="Human input queue" badge={waiting} onClick={onHumanInput}><Ico n="bell" /></IconButton>
        <IconButton label="Action Gateway" onClick={onGateway}><Ico n="shield-check" /></IconButton>
        <Button variant="brain" size="sm" icon={<Ico n="brain" />} onClick={onBrain}>Brain</Button>
        <IconButton label="Settings" onClick={() => setView('settings')}><Ico n="settings" /></IconButton>
      </div>
    </header>
  );
}

/* ---------------- Sidebar ---------------- */
const VIEWS = [
  { id: 'command', label: 'Command Center', icon: 'layout-dashboard' },
  { id: 'graph', label: 'Project Graph', icon: 'workflow' },
  { id: 'plan', label: 'Plan', icon: 'list-checks' },
  { id: 'editor', label: 'Editor', icon: 'code-xml' },
  { id: 'terminal', label: 'Session Terminal', icon: 'terminal' },
  { id: 'code', label: 'Code / Diff Review', icon: 'file-diff' },
];
const PLATFORM_VIEWS = [
  { id: 'packs', label: 'Workflow Packs', icon: 'package' },
  { id: 'audit', label: 'Audit Trail', icon: 'scroll-text' },
];

function NavBtn({ v, view, setView }) {
  const on = view === v.id;
  const brain = v.id === 'brain';
  return (
    <button onClick={() => setView(v.id)} style={{ display: 'flex', alignItems: 'center', gap: 8, width: '100%', height: 28, padding: '0 8px', borderRadius: 'var(--r-2)', border: 'none', cursor: 'pointer', font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', textAlign: 'left',
      background: on ? (brain ? 'var(--brain-surface)' : 'var(--accent-surface)') : 'transparent', color: on ? (brain ? 'var(--brain-ink)' : 'var(--accent-ink)') : 'var(--text-secondary)' }}>
      <Ico n={v.icon} /> {v.label}
    </button>
  );
}

function Sidebar({ view, setView, sel, setSel, waiting = 0, onHumanInput, onTasks, overrides = {} }) {
  const openSession = (s) => { setSel(s.id); setView(s.team ? 'team' : 'terminal'); };
  return (
    <nav style={{ gridArea: 'side', display: 'flex', flexDirection: 'column', background: 'var(--surface-panel)', borderRight: '1px solid var(--border-default)', overflow: 'hidden' }}>
      <div style={{ flex: 1, overflowY: 'auto', padding: '10px 10px 10px' }}>
        <Eyebrow style={{ padding: '0 6px 6px' }}>Workspace</Eyebrow>
        <ProjectsNav view={view} setView={setView} sel={sel} openSession={openSession} overrides={overrides} />
        {VIEWS.map(v => <NavBtn key={v.id} v={v} view={view} setView={setView} />)}
        <button onClick={onHumanInput} style={{ display: 'flex', alignItems: 'center', gap: 8, width: '100%', height: 28, padding: '0 8px', borderRadius: 'var(--r-2)', border: 'none', cursor: 'pointer', font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', background: 'transparent', color: waiting > 0 ? 'var(--attention-ink)' : 'var(--text-faint)' }}>
          <Ico n="circle-alert" /> Human Input
          {waiting > 0 && <span style={{ marginLeft: 'auto', padding: '0 6px', height: 16, display: 'inline-flex', alignItems: 'center', borderRadius: 999, background: 'var(--attention-solid)', color: 'var(--attention-on-solid)', font: '10px var(--font-mono)' }}>{waiting}</span>}
        </button>
        <button onClick={onTasks} style={{ display: 'flex', alignItems: 'center', gap: 8, width: '100%', height: 28, padding: '0 8px', borderRadius: 'var(--r-2)', border: 'none', cursor: 'pointer', font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', background: 'transparent', color: 'var(--text-secondary)', textAlign: 'left' }}>
          <Ico n="inbox" /> Task Inbox
          <kbd style={{ marginLeft: 'auto', font: '9px var(--font-mono)', color: 'var(--text-faint)', border: '1px solid var(--border-default)', borderRadius: 3, padding: '0 4px' }}>⌘⇧P</kbd>
        </button>
        <Eyebrow style={{ padding: '12px 6px 6px' }}>Platform</Eyebrow>
        {PLATFORM_VIEWS.map(v => <NavBtn key={v.id} v={v} view={view} setView={setView} />)}
      </div>
    </nav>
  );
}

/* Projects: expandable tree (chevron) + click-into page (label). */
function ProjectsNav({ view, setView, sel, openSession, overrides }) {
  const [exp, setExp] = useState(true);
  const on = view === 'projects';
  return (
    <div style={{ marginBottom: 2 }}>
      <div style={{ display: 'flex', alignItems: 'center', height: 28, borderRadius: 'var(--r-2)', background: on ? 'var(--accent-surface)' : 'transparent' }}>
        <button onClick={() => setExp(e => !e)} title={exp ? 'Collapse' : 'Expand'} style={{ display: 'inline-flex', alignItems: 'center', justifyContent: 'center', width: 24, height: 28, border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--text-faint)', flex: 'none' }}><Ico n={exp ? 'chevron-down' : 'chevron-right'} s={{ width: 14, height: 14 }} /></button>
        <button onClick={() => setView('projects')} style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, height: 28, padding: '0 8px 0 2px', border: 'none', background: 'transparent', cursor: 'pointer', font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', textAlign: 'left', color: on ? 'var(--accent-ink)' : 'var(--text-secondary)' }}>
          <Ico n="layout-grid" /> Projects
          <span style={{ marginLeft: 'auto', font: '10px var(--font-mono)', color: 'var(--text-faint)' }}>{K.projects.length}</span>
        </button>
      </div>
      {exp && (
        <div style={{ margin: '2px 0 4px', paddingLeft: 4 }}>
          {K.projects.map(p => (
            <ProjectGroup key={p.id} p={p} open={p.id === 'cp'} sessions={K.sessions.filter(s => s.proj === p.id)} sel={sel} overrides={overrides} setSel={(id) => { const s = K.sessions.find(x => x.id === id); if (s) openSession(s); }} />
          ))}
        </div>
      )}
    </div>
  );
}

function ProjectGroup({ p, open, sessions, sel, setSel, overrides = {} }) {
  const [exp, setExp] = useState(open);
  const wfTone = { active: 'var(--teal-ink)', drift: 'var(--warning-ink)', 'needs-personalization': 'var(--caution-ink)', none: 'var(--text-faint)' }[p.workflow];
  return (
    <div style={{ marginBottom: 2 }}>
      <button onClick={() => setExp(!exp)} style={{ display: 'flex', alignItems: 'center', gap: 6, width: '100%', padding: '5px 6px', borderRadius: 'var(--r-2)', border: 'none', background: 'transparent', cursor: 'pointer', textAlign: 'left' }}>
        <Ico n={exp ? 'chevron-down' : 'chevron-right'} s={{ color: 'var(--text-faint)' }} />
        <span style={{ flex: 1, font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.name}</span>
        {p.waiting > 0 && <span title="waiting on human" style={{ width: 7, height: 7, borderRadius: 999, background: 'var(--attention-solid)' }} />}
        {p.active > 0 && <span style={{ font: '10px var(--font-mono)', color: 'var(--text-muted)' }}>{p.active}</span>}
        <span title={'workflow: ' + p.workflow} style={{ width: 6, height: 6, borderRadius: 2, background: wfTone }} />
      </button>
      {exp && sessions.map(s => (
        <button key={s.id} onClick={() => setSel(s.id)} style={{ display: 'flex', alignItems: 'center', gap: 7, width: '100%', padding: '4px 6px 4px 22px', borderRadius: 'var(--r-2)', border: 'none', cursor: 'pointer', textAlign: 'left',
          background: sel === s.id ? 'var(--surface-active)' : 'transparent' }}>
          <AttentionMarker level={{ 'waiting-human': 5, failed: 4, running: 2, active: 1 }[(overrides[s.id] || s.status)] || 0} variant="dot" />
          <span style={{ flex: 1, minWidth: 0, font: 'var(--fs-meta) var(--font-sans)', color: sel === s.id ? 'var(--text-primary)' : 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{s.title}</span>
          {s.team && <span title="Agent team — open team view" style={{ display: 'inline-flex', alignItems: 'center', gap: 3, flex: 'none', height: 15, padding: '0 5px', borderRadius: 'var(--r-1)', background: 'var(--teal-surface)', border: '1px solid var(--teal-line)', color: 'var(--teal-ink)', font: 'var(--fw-semibold) 9px/1 var(--font-sans)' }}><Ico n="users-round" s={{ width: 10, height: 10 }} /> Team</span>}
        </button>
      ))}
    </div>
  );
}

/* ---------------- Bottom event / activity dock (drawer-first) ---------------- */
const DOCK_ICON = { approval: 'shield-check', gateway: 'shield-x', git: 'git-commit-horizontal', session: 'circle-dot', brain: 'brain', pr: 'git-pull-request', workflow: 'workflow', profile: 'key-round' };
function EventDock({ project, projectName, open, onToggle, onOpenAudit }) {
  const events = (K.audit || []).filter(e => project === 'all' || e.proj === project);
  const latest = events[0];
  return (
    <div style={{ gridArea: 'dock', background: 'var(--surface-panel)', borderTop: '1px solid var(--border-default)', display: 'flex', flexDirection: 'column', height: open ? 'var(--shell-timeline-h)' : 'var(--shell-statusbar-h)', transition: 'height var(--dur-3) var(--ease-standard)', overflow: 'hidden' }}>
      {/* status bar (always visible) */}
      <button onClick={onToggle} style={{ display: 'flex', alignItems: 'center', gap: 10, height: 'var(--shell-statusbar-h)', flex: 'none', padding: '0 12px', border: 'none', background: 'transparent', cursor: 'pointer', textAlign: 'left', width: '100%' }}>
        <Ico n={open ? 'chevron-down' : 'chevron-up'} s={{ width: 14, height: 14, color: 'var(--text-faint)' }} />
        <span style={{ font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-muted)' }}>Activity</span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5, font: '10px var(--font-mono)', color: 'var(--success-ink)' }}><span style={{ width: 6, height: 6, borderRadius: 999, background: 'var(--success-solid)' }} /> runtime healthy</span>
        {latest && !open && (
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6, minWidth: 0, font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>
            <span style={{ color: 'var(--text-faint)' }}>·</span>
            <Ico n={DOCK_ICON[latest.kind] || 'dot'} s={{ width: 12, height: 12, color: latest.kind === 'brain' ? 'var(--brain-ink)' : 'var(--text-faint)' }} />
            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: 420, color: 'var(--text-secondary)' }}>{latest.text}</span>
            <span style={{ color: 'var(--text-faint)' }}>{latest.t}</span>
          </span>
        )}
        <span style={{ marginLeft: 'auto', font: '10px var(--font-mono)', color: 'var(--text-faint)' }}>{events.length} events</span>
      </button>
      {/* expanded timeline */}
      {open && (
        <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '0 12px 6px' }}>
            <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)' }}>{projectName}</span>
            <button onClick={onOpenAudit} style={{ marginLeft: 'auto', display: 'inline-flex', alignItems: 'center', gap: 5, padding: '3px 8px', borderRadius: 'var(--r-1)', border: '1px solid var(--border-default)', background: 'transparent', cursor: 'pointer', font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-secondary)' }}>
              <Ico n="scroll-text" s={{ width: 13, height: 13 }} /> Full audit
            </button>
          </div>
          <div style={{ flex: 1, overflowY: 'auto', padding: '0 12px 10px' }}>
            {events.length === 0
              ? <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-faint)', padding: '8px 2px' }}>No recent activity for this project.</div>
              : events.map((e, i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '5px 0', borderBottom: '1px solid var(--border-subtle)' }}>
                  <span style={{ color: e.kind === 'brain' ? 'var(--brain-ink)' : e.result === 'denied' ? 'var(--danger-ink)' : e.result === 'approved' ? 'var(--success-ink)' : 'var(--text-faint)' }}><Ico n={DOCK_ICON[e.kind] || 'dot'} s={{ width: 13, height: 13 }} /></span>
                  <span style={{ flex: 1, minWidth: 0, font: 'var(--fs-meta)/1.4 var(--font-sans)', color: 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.text}</span>
                  <span style={{ font: 'var(--fs-micro) var(--font-mono)', color: e.actor === 'You' ? 'var(--accent-ink)' : 'var(--text-faint)' }}>{e.actor}</span>
                  <span style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)', width: 56, textAlign: 'right' }}>{e.t}</span>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
}

window.KitShell = { TopBar, Sidebar, Eyebrow, Ico, EventDock };

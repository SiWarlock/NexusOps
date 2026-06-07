/* ============================================================
   Control Plane UI kit — Plan View (implementation plan).
   Phase → Track → PlanTask hierarchy with dispatch.
   ============================================================ */
const _NS8 = window.ControlPlaneDesignSystem_a21911;
const { Button: PBtn, IconButton: PIconBtn, StatusPill: PPill, MetaChip: PMeta } = _NS8;
const PBadge = _NS8.Badge || (({ children, mono, style = {} }) => <span style={{ font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>);
const { Ico: PIco, Eyebrow: PEye } = window.KitShell;
const KD8 = window.KIT;
const { useState: useS8 } = React;

const TASK_STATUS = {
  completed: { pill: 'completed', label: 'Done' }, 'in-progress': { pill: 'running', label: 'In progress' },
  ready: { pill: 'idle', label: 'Ready' }, todo: { pill: 'idle', label: 'Todo' }, backlog: { pill: 'archived', label: 'Backlog' },
};
const PHASE_STATUS = { completed: 'completed', active: 'running', backlog: 'archived' };

function PlanView({ onDispatch, openBrain }) {
  const plan = KD8.plan;
  const all = plan.phases.flatMap(ph => ph.tracks.flatMap(t => t.tasks));
  const done = all.filter(t => t.status === 'completed').length;
  const pct = Math.round((done / all.length) * 100);
  const [openPhases, setOpenPhases] = useS8({ p1: false, p2: true, p3: true });
  React.useEffect(() => { const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24); return () => clearTimeout(t); }, [openPhases]);
  return (
    <div style={{ height: '100%', overflowY: 'auto', background: 'var(--surface-canvas)' }}>
      <div style={{ position: 'sticky', top: 0, zIndex: 5, padding: '14px 16px', background: 'var(--surface-canvas)', borderBottom: '1px solid var(--border-subtle)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <span style={{ color: 'var(--text-secondary)' }}><PIco n="list-checks" /></span>
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>Implementation plan</h1>
          <PMeta icon={<PIco n="file-text" s={{ width: 12, height: 12 }} />}>{plan.source}</PMeta>
          <PBadge tone="teal" style={{ display: 'inline-flex', alignItems: 'center', height: 18, padding: '0 6px', borderRadius: 'var(--r-1)', background: 'var(--teal-surface)', color: 'var(--teal-ink)' }}>parser: {plan.parser}</PBadge>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
            <PBtn variant="ghost" size="sm" icon={<PIco n="brain" />} onClick={openBrain}>Ask Brain</PBtn>
          </div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 11 }}>
          <div style={{ flex: 1, maxWidth: 320, height: 6, borderRadius: 999, background: 'var(--cap-track)', overflow: 'hidden' }}><i style={{ display: 'block', height: '100%', width: pct + '%', background: 'var(--success-solid)' }} /></div>
          <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>{done}/{all.length} tasks · {pct}%</span>
        </div>
      </div>

      <div style={{ padding: '14px 16px', maxWidth: 820, display: 'flex', flexDirection: 'column', gap: 10 }}>
        {plan.phases.map(ph => {
          const open = openPhases[ph.id];
          const ptasks = ph.tracks.flatMap(t => t.tasks);
          const pdone = ptasks.filter(t => t.status === 'completed').length;
          return (
            <div key={ph.id} style={{ border: '1px solid var(--border-default)', borderRadius: 'var(--r-3)', overflow: 'hidden', background: 'var(--surface-card)' }}>
              <div onClick={() => setOpenPhases(s => ({ ...s, [ph.id]: !s[ph.id] }))} role="button" style={{ display: 'flex', alignItems: 'center', gap: 9, width: '100%', padding: '11px 13px', background: ph.status === 'active' ? 'var(--accent-surface)' : 'transparent', cursor: 'pointer', textAlign: 'left' }}>
                <PIco n={open ? 'chevron-down' : 'chevron-right'} s={{ width: 15, height: 15, color: 'var(--text-faint)' }} />
                <span style={{ font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)', color: ph.status === 'active' ? 'var(--accent-ink)' : 'var(--text-primary)' }}>{ph.name}</span>
                <PPill status={PHASE_STATUS[ph.status]} size="xs" />
                <span style={{ marginLeft: 'auto', font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>{pdone}/{ptasks.length}</span>
                {ph.status !== 'completed' && <PBtn variant="ghost" size="xs" icon={<PIco n="users-round" s={{ width: 12, height: 12 }} />} onClick={(e) => { e.stopPropagation(); onDispatch({ source: 'plan', id: ph.name.split(' · ')[0], title: ph.name, harness: 'claude-code', profile: 'Claude Team Work', branch: 'team/phase', planTask: ph.name }); }}>Start team</PBtn>}
              </div>
              {open && (
                <div style={{ padding: '4px 10px 10px' }}>
                  {ph.tracks.map(tr => (
                    <div key={tr.id} style={{ marginTop: 6 }}>
                      <PEye style={{ padding: '4px 6px 7px' }}>{tr.name}</PEye>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
                        {tr.tasks.map(task => <PlanTaskRow key={task.id} task={task} onDispatch={onDispatch} openBrain={openBrain} />)}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function PlanTaskRow({ task, onDispatch, openBrain }) {
  const st = TASK_STATUS[task.status] || TASK_STATUS.todo;
  const dispatchable = task.status !== 'completed';
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '9px 10px', borderRadius: 'var(--r-2)', border: '1px solid var(--border-subtle)', background: 'var(--surface-panel)' }}>
      <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)', width: 30, flex: 'none' }}>{task.id}</span>
      <PPill status={st.pill} size="xs" label={st.label} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ font: 'var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)', color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{task.title}</div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 3, font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>
          <span title="acceptance criteria"><PIco n="check-check" s={{ width: 11, height: 11 }} /> {task.ac} AC</span>
          {task.files > 0 && <span title="files">· {task.files} files</span>}
          {task.anchors && task.anchors.map(a => <span key={a} style={{ color: 'var(--brain-ink)' }}>· ⚓ {a.split('#')[1] || a}</span>)}
          {task.session && <span style={{ color: 'var(--live-ink)' }}>· ● in session</span>}
          {task.team && <span style={{ color: 'var(--teal-ink)' }}>· team</span>}
        </div>
      </div>
      {task.task && <PMeta tone={task.task.tone === 'linear' ? 'linear' : task.task.tone === 'github' ? 'github' : 'accent'} mono={false}>{task.task.id}</PMeta>}
      {dispatchable
        ? <PBtn variant="secondary" size="xs" icon={<PIco n="play" s={{ width: 12, height: 12 }} />} onClick={() => onDispatch({ source: 'plan', id: task.id, title: task.title, harness: 'claude-code', profile: 'Claude Max Main', branch: task.task ? null : 'agent/' + task.id, planTask: task.id })}>Start</PBtn>
        : <PIconBtn icon={<PIco n="git-pull-request" s={{ width: 14, height: 14 }} />} size="sm" aria-label="View PR" />}
    </div>
  );
}

window.KitPlan = { PlanView };

/* ============================================================
   Control Plane UI kit — Task Inbox drawer + Dispatch dialog.
   Drawer-first task intake: GitHub issues, Linear tickets, plan
   tasks. Drag a chip onto a session/the command center, or click
   to choose how to start (single session vs cc-crew /team-start).
   ============================================================ */
const _NS7 = window.ControlPlaneDesignSystem_a21911;
const { Button: TBtn, IconButton: TIconBtn, StatusPill: TPill,
        RiskBadge: TRisk, HarnessBadge: THarness, ProfileBadge: TProfile, MetaChip: TMeta } = _NS7;
const TBadge = _NS7.Badge || (({ children, mono, style = {} }) => <span style={{ font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>);
const { Ico: TIco, Eyebrow: TEye } = window.KitShell;
const KD7 = window.KIT;
const { useState: useS7 } = React;

const SOURCE = {
  linear: { icon: 'square-kanban', label: 'Linear', tone: 'var(--domain-linear)', surf: 'var(--domain-linear-surface)' },
  github: { icon: 'circle-dot', label: 'GitHub', tone: 'var(--slate-ink)', surf: 'var(--slate-surface)' },
  plan: { icon: 'list-checks', label: 'Plan', tone: 'var(--brain-ink)', surf: 'var(--brain-surface)' },
};
const PRIO = { Urgent: 'var(--danger-ink)', High: 'var(--attention-ink)', Medium: 'var(--caution-ink)', Low: 'var(--text-muted)' };
const TABS = ['All', 'Linear', 'GitHub', 'Plan tasks', 'Completed'];

/* ---------------- Task Inbox drawer ---------------- */
function TaskInbox({ open, onClose, onDispatch, onDropToast }) {
  const [tab, setTab] = useS7('All');
  if (!open) return null;
  const tasks = KD7.tasks.filter(t => {
    if (tab === 'All') return t.status !== 'Done';
    if (tab === 'Linear') return t.source === 'linear';
    if (tab === 'GitHub') return t.source === 'github';
    if (tab === 'Plan tasks') return t.source === 'plan';
    if (tab === 'Completed') return t.status === 'Done';
    return true;
  });
  const counts = { Linear: KD7.tasks.filter(t => t.source === 'linear').length, GitHub: KD7.tasks.filter(t => t.source === 'github').length };
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, zIndex: 'var(--z-drawer)', display: 'flex', justifyContent: 'flex-end', background: 'var(--scrim)', backdropFilter: 'blur(2px)' }}>
      <div onClick={e => e.stopPropagation()} style={{ width: 400, height: '100%', background: 'var(--surface-panel)', borderLeft: '1px solid var(--border-strong)', boxShadow: 'var(--elev-4)', display: 'flex', flexDirection: 'column', animation: 'cp-slide-in 0.24s var(--ease-out)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '12px 14px', borderBottom: '1px solid var(--border-default)' }}>
          <span style={{ color: 'var(--text-secondary)' }}><TIco n="inbox" /></span>
          <span style={{ font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)' }}>Task Inbox</span>
          <TBadge mono style={{ color: 'var(--text-muted)' }}>{KD7.tasks.filter(t => t.status !== 'Done').length} open</TBadge>
          <span style={{ marginLeft: 'auto', display: 'flex', gap: 4 }}>
            <TIconBtn icon={<TIco n="refresh-cw" s={{ width: 15, height: 15 }} />} size="sm" aria-label="Sync" />
            <TIconBtn icon={<TIco n="x" s={{ width: 15, height: 15 }} />} size="sm" aria-label="Close" onClick={onClose} />
          </span>
        </div>
        <div style={{ display: 'flex', gap: 4, padding: '8px 12px', borderBottom: '1px solid var(--border-subtle)', overflowX: 'auto' }}>
          {TABS.map(t => (
            <button key={t} onClick={() => setTab(t)} style={{ whiteSpace: 'nowrap', padding: '4px 9px', borderRadius: 999, cursor: 'pointer', border: `1px solid ${tab === t ? 'var(--accent-line)' : 'var(--border-default)'}`, background: tab === t ? 'var(--accent-surface)' : 'transparent', color: tab === t ? 'var(--accent-ink)' : 'var(--text-muted)', font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)' }}>
              {t}{counts[t] ? ' · ' + counts[t] : ''}
            </button>
          ))}
        </div>
        <div style={{ padding: '7px 10px 4px', font: 'var(--fs-micro) var(--font-sans)', color: 'var(--text-faint)', display: 'flex', alignItems: 'center', gap: 6 }}>
          <TIco n="grip-vertical" s={{ width: 12, height: 12 }} /> Drag a task onto a session, or click to dispatch
        </div>
        <div style={{ flex: 1, overflowY: 'auto', padding: '4px 10px 14px', display: 'flex', flexDirection: 'column', gap: 7 }}>
          {tasks.map(t => <TaskChip key={t.source + t.id} task={t} onDispatch={onDispatch} />)}
          {tasks.length === 0 && <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-faint)', padding: '12px 4px' }}>Nothing here.</div>}
        </div>
      </div>
    </div>
  );
}

function TaskChip({ task, onDispatch, compact }) {
  const s = SOURCE[task.source];
  const onDragStart = (e) => {
    e.dataTransfer.setData('text/plain', task.source + ':' + task.id);
    e.dataTransfer.effectAllowed = 'copy';
    window.__dragTask = task;
  };
  return (
    <div draggable onDragStart={onDragStart} onDragEnd={() => { window.__dragTask = null; }} onClick={() => onDispatch && onDispatch(task)}
      title="Drag onto a session or click to dispatch"
      style={{ cursor: 'grab', border: '1px solid var(--border-default)', borderRadius: 'var(--r-2)', background: 'var(--surface-card)', padding: '9px 10px', display: 'flex', flexDirection: 'column', gap: 7 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, height: 16, padding: '0 5px', borderRadius: 'var(--r-1)', background: s.surf, color: s.tone, font: 'var(--fw-medium) var(--fs-micro) var(--font-mono)' }}>
          <TIco n={s.icon} s={{ width: 11, height: 11 }} /> {task.id}
        </span>
        <span style={{ marginLeft: 'auto', display: 'inline-flex', alignItems: 'center', gap: 4, font: 'var(--fs-micro) var(--font-mono)', color: PRIO[task.priority] }}>
          <span style={{ width: 5, height: 5, borderRadius: 999, background: 'currentColor' }} />{task.priority}
        </span>
      </div>
      <div style={{ font: 'var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)', color: 'var(--text-primary)' }}>{task.title}</div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 5, flexWrap: 'wrap' }}>
        {task.labels.map(l => <span key={l} style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-muted)', background: 'var(--neutral-surface)', padding: '1px 5px', borderRadius: 'var(--r-1)' }}>{l}</span>)}
        {task.planTask && <span style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--brain-ink)' }}>↳ {task.planTask}</span>}
        {task.session && <span style={{ marginLeft: 'auto', font: 'var(--fs-micro) var(--font-mono)', color: 'var(--live-ink)' }}>● in session</span>}
      </div>
    </div>
  );
}

/* ---------------- Dispatch dialog ---------------- */
function DispatchDialog({ task, onClose, onConfirm }) {
  const [mode, setMode] = useS7('single');
  const [harness, setHarness] = useS7(task ? task.harness : 'claude-code');
  const [profile, setProfile] = useS7(task ? task.profile : 'Claude Max Main');
  if (!task) return null;
  const s = SOURCE[task.source];
  const profiles = KD7.profilesDetail.map(p => p.name);
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, zIndex: 'var(--z-modal)', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--scrim)', backdropFilter: 'blur(2px)' }}>
      <div onClick={e => e.stopPropagation()} style={{ width: 520, background: 'var(--surface-card)', border: '1px solid var(--border-strong)', borderRadius: 'var(--r-4)', boxShadow: 'var(--elev-4)', overflow: 'hidden', animation: 'cp-pop-in 0.18s var(--ease-out)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '13px 16px', borderBottom: '1px solid var(--border-default)' }}>
          <span style={{ color: 'var(--text-secondary)' }}><TIco n="rocket" /></span>
          <span style={{ font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)' }}>Dispatch task</span>
          <span style={{ marginLeft: 'auto', display: 'inline-flex', alignItems: 'center', gap: 5, height: 18, padding: '0 7px', borderRadius: 'var(--r-1)', background: s.surf, color: s.tone, font: 'var(--fw-medium) var(--fs-micro) var(--font-mono)' }}><TIco n={s.icon} s={{ width: 11, height: 11 }} /> {task.id}</span>
        </div>
        <div style={{ padding: '16px' }}>
          <div style={{ font: 'var(--fw-medium) var(--fs-body-lg)/1.35 var(--font-sans)', marginBottom: 4 }}>{task.title}</div>
          <div style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)', marginBottom: 14 }}>
            {task.planTask ? '↳ ' + task.planTask + ' · ' : ''}suggested {task.branch || 'new worktree'}
          </div>

          <TEye style={{ marginBottom: 8 }}>How to start</TEye>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 7, marginBottom: 14 }}>
            <ModeOpt active={mode === 'single'} onClick={() => setMode('single')} icon="bot" title="Single session" desc="One agent in a fresh worktree." />
            <ModeOpt active={mode === 'team'} onClick={() => setMode('team')} icon="users-round" title="Agent team — cc-crew /team-start" desc="Orchestrator decomposes and delegates to workers." tone="teal" />
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: mode === 'single' ? '1fr 1fr' : '1fr', gap: 10 }}>
            {mode === 'single' && (
              <Field label="Harness">
                <select value={harness} onChange={e => setHarness(e.target.value)} style={selStyle}>
                  <option value="claude-code">Claude Code</option>
                  <option value="codex-cli">Codex CLI</option>
                  <option value="codex-cloud">Codex Cloud</option>
                </select>
              </Field>
            )}
            <Field label="Execution profile">
              <select value={profile} onChange={e => setProfile(e.target.value)} style={selStyle}>
                {profiles.map(p => <option key={p}>{p}</option>)}
              </select>
            </Field>
          </div>

          <div style={{ marginTop: 14, background: 'var(--surface-sunken)', borderRadius: 'var(--r-2)', padding: '10px 12px', boxShadow: 'var(--elev-inset)' }}>
            <TEye style={{ marginBottom: 7 }}>Will run via Gateway</TEye>
            <code style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--accent-ink)' }}>
              {mode === 'team' ? `/team-start ${task.branch ? task.branch.split('/')[0] : 'task'} --profile "${profile}"` : `start ${harness} --profile "${profile}" --task ${task.id}`}
            </code>
          </div>
        </div>
        <div style={{ display: 'flex', gap: 8, padding: '12px 16px', borderTop: '1px solid var(--border-default)' }}>
          <TBtn variant="ghost" size="md" onClick={onClose} style={{ marginRight: 'auto' }}>Cancel</TBtn>
          <TBtn variant="secondary" size="md" icon={<TIco n="link" s={{ width: 14, height: 14 }} />}>Link to plan</TBtn>
          <TBtn variant="primary" size="md" icon={<TIco n={mode === 'team' ? 'users-round' : 'play'} s={{ width: 14, height: 14 }} />} onClick={() => onConfirm(task, { mode, harness, profile })}>
            {mode === 'team' ? 'Start agent team' : 'Dispatch session'}
          </TBtn>
        </div>
      </div>
    </div>
  );
}

const selStyle = { width: '100%', background: 'var(--surface-input)', color: 'var(--text-primary)', border: '1px solid var(--border-default)', borderRadius: 'var(--r-2)', font: 'var(--fs-label) var(--font-sans)', padding: '6px 8px', height: 'var(--ctl-md)' };
function Field({ label, children }) {
  return <div><div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)', marginBottom: 5 }}>{label}</div>{children}</div>;
}
function ModeOpt({ active, onClick, icon, title, desc, tone }) {
  const ac = tone === 'teal' ? 'var(--teal-line)' : 'var(--accent-line)';
  const sf = tone === 'teal' ? 'var(--teal-surface)' : 'var(--accent-surface)';
  const ink = tone === 'teal' ? 'var(--teal-ink)' : 'var(--accent-ink)';
  return (
    <button onClick={onClick} style={{ textAlign: 'left', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 11, padding: '10px 12px', borderRadius: 'var(--r-2)', border: `1px solid ${active ? ac : 'var(--border-default)'}`, background: active ? sf : 'var(--surface-card)' }}>
      <span style={{ width: 30, height: 30, flex: 'none', borderRadius: 'var(--r-2)', background: active ? sf : 'var(--surface-active)', color: active ? ink : 'var(--text-muted)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><TIco n={icon} s={{ width: 16, height: 16 }} /></span>
      <div style={{ flex: 1 }}>
        <div style={{ font: 'var(--fw-medium) var(--fs-body) var(--font-sans)', color: active ? ink : 'var(--text-primary)' }}>{title}</div>
        <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)', marginTop: 2 }}>{desc}</div>
      </div>
      <span style={{ width: 16, height: 16, flex: 'none', borderRadius: 999, border: `2px solid ${active ? ink : 'var(--border-strong)'}`, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>
        {active && <span style={{ width: 7, height: 7, borderRadius: 999, background: ink }} />}
      </span>
    </button>
  );
}

window.KitTasks = { TaskInbox, DispatchDialog, TaskChip, HumanInputQueue };

/* ---------------- Human Input Queue (Screen 15) ---------------- */
const HIQ_RISK_ICON = { gateway: 'shield-check', decision: 'git-pull-request', personalization: 'wand-sparkles' };
function HumanInputQueue({ open, onClose, onResolve, resolved = [] }) {
  if (!open) return null;
  const items = KD7.humanInput.filter(it => !resolved.includes(it.id));
  const groups = {};
  items.forEach(it => { (groups[it.group] = groups[it.group] || []).push(it); });
  const order = ['Permission requests', 'High-risk actions', 'Failed checks · needs decision', 'Workflow personalization'];
  return (
    <div onClick={onClose} style={{ position: 'fixed', inset: 0, zIndex: 'var(--z-drawer)', display: 'flex', justifyContent: 'flex-end', background: 'var(--scrim)', backdropFilter: 'blur(2px)' }}>
      <div onClick={e => e.stopPropagation()} style={{ width: 420, height: '100%', background: 'var(--surface-panel)', borderLeft: '1px solid var(--attention-line)', boxShadow: 'var(--elev-4)', display: 'flex', flexDirection: 'column', animation: 'cp-slide-in 0.24s var(--ease-out)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '12px 14px', borderBottom: '1px solid var(--border-default)', background: 'var(--attention-surface)' }}>
          <span style={{ color: 'var(--attention-ink)' }}><TIco n="circle-alert" /></span>
          <span style={{ font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)', color: 'var(--text-primary)' }}>Human input</span>
          <TBadge mono style={{ color: 'var(--attention-ink)' }}>{items.length} waiting</TBadge>
          <span style={{ marginLeft: 'auto' }}><TIconBtn icon={<TIco n="x" s={{ width: 15, height: 15 }} />} aria-label="Close" onClick={onClose} /></span>
        </div>
        <div style={{ flex: 1, overflowY: 'auto', padding: '12px' }}>
          {items.length === 0 && <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8, padding: '40px 16px', color: 'var(--text-muted)' }}><TIco n="check-check" s={{ width: 26, height: 26, color: 'var(--success-ink)' }} /><span style={{ font: 'var(--fs-label) var(--font-sans)' }}>Queue clear — nothing waiting on you.</span></div>}
          {order.filter(g => groups[g]).map(g => (
            <div key={g} style={{ marginBottom: 14 }}>
              <TEye style={{ marginBottom: 8 }}>{g}</TEye>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {groups[g].map(it => <HIQCard key={it.id} it={it} onResolve={onResolve} />)}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function HIQCard({ it, onResolve }) {
  const loud = it.risk === 'high' || it.risk === 'critical';
  return (
    <div style={{ border: `1px solid ${loud ? 'var(--danger-line)' : 'var(--border-default)'}`, background: loud ? 'var(--danger-surface)' : 'var(--surface-card)', borderRadius: 'var(--r-3)', padding: '11px 12px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 7, marginBottom: 8 }}>
        <span style={{ color: 'var(--text-muted)' }}><TIco n={HIQ_RISK_ICON[it.kind] || 'circle-alert'} s={{ width: 14, height: 14 }} /></span>
        <TRisk level={it.risk} />
        <span style={{ marginLeft: 'auto', font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>{it.age}</span>
      </div>
      <div style={{ font: 'var(--fw-medium) var(--fs-label)/1.4 var(--font-sans)', color: 'var(--text-primary)', marginBottom: 5 }}>{it.reason}</div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)', marginBottom: 11 }}>
        <span style={{ color: 'var(--accent-ink)' }}>{it.actor}</span><span>→</span><span>{it.target}</span>
      </div>
      <div style={{ display: 'flex', gap: 7 }}>
        <TBtn variant={loud ? 'danger' : 'attention'} size="sm" icon={<TIco n={it.kind === 'personalization' ? 'wand-sparkles' : 'check'} s={{ width: 13, height: 13 }} />} onClick={() => onResolve(it, 'open')}>
          {it.kind === 'gateway' ? 'Review' : it.kind === 'decision' ? 'Decide' : 'Personalize'}
        </TBtn>
        {it.kind === 'gateway' && <TBtn variant="ghost" size="sm" onClick={() => onResolve(it, 'deny')}>Deny</TBtn>}
        {it.kind !== 'gateway' && <TBtn variant="ghost" size="sm" onClick={() => onResolve(it, 'defer')}>Defer</TBtn>}
      </div>
    </div>
  );
}

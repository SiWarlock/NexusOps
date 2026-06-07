/* ============================================================
   Control Plane UI kit — Project Brain (chat co-pilot),
   Audit timeline, Settings / Execution profiles.
   ============================================================ */
const _NS5 = window.ControlPlaneDesignSystem_a21911;
const { Button: BBtn, IconButton: BIconBtn, StatusPill: BPill,
        RiskBadge: BRisk, UsageMeter: BMeter, HarnessBadge: BHarness,
        ProfileBadge: BProfile, MetaChip: BMeta, EvidenceChip: BEvidence } = _NS5;
const BBadge = _NS5.Badge || (({ children, mono, style = {} }) => <span style={{ font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>);
const { Ico: BIco, Eyebrow: BEye } = window.KitShell;
const KD5 = window.KIT;
const { useState: useS5, useRef: useR5, useEffect: useE5 } = React;

/* tiny markdown-ish: **bold** and `code` */
function rich(text) {
  const parts = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g; let last = 0, m;
  while ((m = re.exec(text))) {
    if (m.index > last) parts.push(text.slice(last, m.index));
    const tok = m[0];
    if (tok.startsWith('**')) parts.push(<strong key={m.index} style={{ color: 'var(--text-primary)', fontWeight: 'var(--fw-semibold)' }}>{tok.slice(2, -2)}</strong>);
    else parts.push(<code key={m.index} style={{ font: 'var(--fs-meta) var(--font-mono)', background: 'var(--surface-active)', padding: '1px 5px', borderRadius: 3, color: 'var(--brain-ink)' }}>{tok.slice(1, -1)}</code>);
    last = m.index + tok.length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return parts;
}

/* ---------------- Project Brain page (co-pilot) ---------------- */
const BRAIN_MODES = ['Ask', 'Plan', 'Review', 'Decisions', 'Memory'];
const SCOPES = ['Entire project', 'Current session', 'Current PR', 'Current plan task'];

function ProjectBrainPage({ openGateway, drawer = false }) {
  const [mode, setMode] = useS5('Ask');
  const [scope, setScope] = useS5('Entire project');
  const [thread, setThread] = useS5(KD5.brainThread);
  const [draft, setDraft] = useS5('');
  const [thinking, setThinking] = useS5(false);
  const scrollRef = useR5(null);

  useE5(() => { if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight; setTimeout(() => window.lucide && window.lucide.createIcons(), 20); }, [thread, thinking]);

  const send = (text) => {
    const q = (text != null ? text : draft).trim();
    if (!q) return;
    setThread(t => [...t, { from: 'user', text: q }]);
    setDraft(''); setThinking(true);
    setTimeout(() => {
      const reply = KD5.brainReplies.find(r => r.match.test(q)) || KD5.brainReplies[KD5.brainReplies.length - 1];
      setThinking(false);
      setThread(t => [...t, { from: 'brain', text: reply.text, evidence: reply.evidence, plan: reply.plan }]);
    }, 850);
  };

  return (
    <div style={{ display: 'grid', gridTemplateColumns: drawer ? '1fr' : '1fr 280px', height: '100%', background: 'var(--surface-canvas)', minHeight: 0 }}>
      {/* chat column */}
      <div style={{ display: 'grid', gridTemplateRows: 'auto 1fr auto', minHeight: 0, minWidth: 0 }}>
        {/* header: identity + modes + scope */}
        <div style={{ padding: '12px 16px 0', borderBottom: '1px solid var(--border-subtle)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 9, marginBottom: 12 }}>
            <span style={{ width: 26, height: 26, borderRadius: 'var(--r-2)', background: 'var(--brain-surface)', border: '1px solid var(--brain-line)', color: 'var(--brain-ink)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><BIco n="brain" s={{ width: 15, height: 15 }} /></span>
            <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>Project Brain</h1>
            <BBadge style={{ display: 'inline-flex', alignItems: 'center', gap: 5, height: 18, padding: '0 7px', borderRadius: 999, background: 'var(--brain-surface)', color: 'var(--brain-ink)', font: 'var(--fs-micro) var(--font-mono)' }}><span style={{ width: 5, height: 5, borderRadius: 999, background: 'var(--brain-solid)' }} /> grounded · 142 objects</BBadge>
            <div style={{ marginLeft: 'auto', display: 'flex', gap: 6, alignItems: 'center' }}>
              <BIco n="search" s={{ width: 14, height: 14, color: 'var(--text-faint)' }} />
              <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)' }}>scope:</span>
              <select value={scope} onChange={e => setScope(e.target.value)} style={{ background: 'var(--surface-input)', color: 'var(--text-secondary)', border: '1px solid var(--border-default)', borderRadius: 'var(--r-1)', font: 'var(--fs-meta) var(--font-sans)', padding: '2px 6px' }}>
                {SCOPES.map(s => <option key={s}>{s}</option>)}
              </select>
            </div>
          </div>
          <div style={{ display: 'flex', gap: 2 }}>
            {BRAIN_MODES.map(m => (
              <button key={m} onClick={() => setMode(m)} style={{ padding: '7px 11px', border: 'none', background: 'transparent', cursor: 'pointer',
                font: `${mode === m ? 'var(--fw-semibold)' : 'var(--fw-medium)'} var(--fs-label) var(--font-sans)`,
                color: mode === m ? 'var(--brain-ink)' : 'var(--text-muted)', boxShadow: mode === m ? 'inset 0 -2px 0 var(--brain-solid)' : 'none' }}>{m}</button>
            ))}
          </div>
        </div>

        {/* thread */}
        <div ref={scrollRef} style={{ overflowY: 'auto', padding: '16px', display: 'flex', flexDirection: 'column', gap: 16 }}>
          {thread.map((m, i) => m.from === 'user'
            ? <UserMsg key={i} text={m.text} />
            : <BrainMsg key={i} m={m} openGateway={openGateway} send={send} />)}
          {thinking && <ThinkingMsg />}
        </div>

        {/* composer */}
        <div style={{ padding: '12px 16px', borderTop: '1px solid var(--border-default)', background: 'var(--surface-panel)' }}>
          <div style={{ display: 'flex', gap: 7, marginBottom: 9, flexWrap: 'wrap' }}>
            {['Start the next backend task', 'Why are checks failing?', 'Summarize today'].map(s => (
              <button key={s} onClick={() => send(s)} style={{ padding: '4px 9px', borderRadius: 999, border: '1px solid var(--border-default)', background: 'var(--surface-card)', color: 'var(--text-secondary)', font: 'var(--fs-meta) var(--font-sans)', cursor: 'pointer' }}>{s}</button>
            ))}
          </div>
          <div style={{ display: 'flex', alignItems: 'flex-end', gap: 8, background: 'var(--surface-input)', border: '1px solid var(--border-strong)', borderRadius: 'var(--r-3)', padding: '8px 8px 8px 12px' }}>
            <textarea value={draft} onChange={e => setDraft(e.target.value)} rows={1} placeholder={`Ask Project Brain to ${mode === 'Plan' ? 'plan an action' : 'explain, plan, or recall…'}`}
              onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); } }}
              style={{ flex: 1, resize: 'none', background: 'transparent', border: 'none', outline: 'none', color: 'var(--text-primary)', font: 'var(--fs-body)/1.5 var(--font-sans)', maxHeight: 90 }} />
            <BBtn variant="primary" size="sm" icon={<BIco n="arrow-up" s={{ width: 14, height: 14 }} />} onClick={() => send()}>Ask</BBtn>
          </div>
        </div>
      </div>

      {/* right rail: memory & decisions */}
      {!drawer && (
      <aside style={{ borderLeft: '1px solid var(--border-default)', background: 'var(--surface-panel)', overflowY: 'auto', padding: '14px' }}>
        <BEye style={{ marginBottom: 10 }}>Memory &amp; decisions</BEye>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {KD5.brainMemory.map((m, i) => (
            <div key={i} style={{ border: '1px solid var(--border-subtle)', borderRadius: 'var(--r-2)', padding: '9px 10px', background: 'var(--surface-card)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                <span style={{ color: m.kind === 'decision' ? 'var(--brain-ink)' : 'var(--text-faint)' }}><BIco n={m.kind === 'decision' ? 'gavel' : m.kind === 'anchor' ? 'anchor' : 'database'} s={{ width: 13, height: 13 }} /></span>
                <span style={{ font: 'var(--fw-medium) var(--fs-label) var(--font-sans)', color: 'var(--text-primary)' }}>{m.label}</span>
                <span style={{ marginLeft: 'auto', font: 'var(--fs-micro) var(--font-mono)', color: m.t === 'fresh' ? 'var(--success-ink)' : 'var(--text-faint)' }}>{m.t}</span>
              </div>
              <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)' }}>{m.sub}</div>
            </div>
          ))}
        </div>
      </aside>
      )}
    </div>
  );
}

function UserMsg({ text }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
      <div style={{ maxWidth: '76%', background: 'var(--accent-surface)', border: '1px solid var(--accent-line)', borderRadius: 'var(--r-3) var(--r-3) var(--r-1) var(--r-3)', padding: '9px 12px', font: 'var(--fs-body)/1.5 var(--font-sans)', color: 'var(--text-primary)' }}>{text}</div>
    </div>
  );
}

function ThinkingMsg() {
  return (
    <div style={{ display: 'flex', gap: 10 }}>
      <BrainAvatar />
      <div style={{ display: 'inline-flex', alignItems: 'center', gap: 6, padding: '9px 12px', borderRadius: 'var(--r-3)', background: 'var(--surface-card)', border: '1px solid var(--border-subtle)' }}>
        {[0, 1, 2].map(i => <span key={i} style={{ width: 6, height: 6, borderRadius: 999, background: 'var(--brain-ink)', animation: `cp-live-pulse 1s ${i * 0.18}s var(--ease-inout) infinite` }} />)}
        <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)', marginLeft: 4 }}>retrieving evidence…</span>
      </div>
    </div>
  );
}

function BrainAvatar() {
  return <span style={{ width: 26, height: 26, flex: 'none', borderRadius: 'var(--r-2)', background: 'var(--brain-surface)', border: '1px solid var(--brain-line)', color: 'var(--brain-ink)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><BIco n="brain" s={{ width: 14, height: 14 }} /></span>;
}

function BrainMsg({ m, openGateway, send }) {
  return (
    <div style={{ display: 'flex', gap: 10 }}>
      <BrainAvatar />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ background: 'var(--surface-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--r-3) var(--r-3) var(--r-3) var(--r-1)', padding: '11px 13px' }}>
          <div style={{ font: 'var(--fs-body)/1.6 var(--font-sans)', color: 'var(--text-secondary)' }}>{rich(m.text)}</div>
          {m.evidence && (
            <div style={{ marginTop: 11 }}>
              <BEye style={{ marginBottom: 7 }}>Evidence</BEye>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                {m.evidence.map((e, i) => <BEvidence key={i} kind={e.kind} label={e.label} sub={e.sub} freshness={e.freshness} />)}
              </div>
            </div>
          )}
          {m.plan && (
            <div style={{ marginTop: 12, border: '1px solid var(--brain-line)', borderRadius: 'var(--r-2)', overflow: 'hidden', background: 'var(--brain-surface)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 7, padding: '8px 11px', borderBottom: '1px solid var(--brain-line)' }}>
                <BIco n="list-checks" s={{ width: 14, height: 14, color: 'var(--brain-ink)' }} />
                <span style={{ font: 'var(--fw-semibold) var(--fs-label) var(--font-sans)', color: 'var(--brain-ink)' }}>{m.plan.title}</span>
                <span style={{ marginLeft: 'auto', font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-muted)' }}>{m.plan.steps.length} steps</span>
              </div>
              {m.plan.steps.map((s, i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '8px 11px', borderBottom: i < m.plan.steps.length - 1 ? '1px solid var(--brain-line)' : 'none', background: 'var(--surface-card)' }}>
                  <span style={{ width: 17, height: 17, flex: 'none', borderRadius: 999, background: 'var(--surface-active)', color: 'var(--text-muted)', font: 'var(--fw-semibold) var(--fs-micro) var(--font-mono)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>{i + 1}</span>
                  <span style={{ flex: 1, font: 'var(--fs-label)/1.35 var(--font-sans)', color: 'var(--text-secondary)' }}>{s.text}</span>
                  <BRisk level={s.risk} />
                </div>
              ))}
              <div style={{ display: 'flex', gap: 7, padding: '10px 11px', background: 'var(--surface-card)' }}>
                <BBtn variant="primary" size="sm" icon={<BIco n="shield-check" s={{ width: 14, height: 14 }} />} onClick={() => openGateway('edit')}>Run via Gateway</BBtn>
                <BBtn variant="ghost" size="sm">Edit plan</BBtn>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/* ---------------- Audit timeline ---------------- */
const AUDIT_ICON = { approval: 'shield-check', gateway: 'shield-x', git: 'git-commit-horizontal', session: 'circle-dot', brain: 'brain', pr: 'git-pull-request', workflow: 'workflow', profile: 'key-round' };
const AUDIT_GROUP = { Approvals: ['approval', 'gateway'], Git: ['git', 'pr'], Sessions: ['session'], Brain: ['brain'], Workflow: ['workflow'] };

function AuditTimeline({ project = 'cp', projectName = 'Project', allProjects }) {
  const [filter, setFilter] = useS5('All');
  const [scope, setScope] = useS5('project');
  const rows = KD5.audit.filter(e => (scope === 'all' || project === 'all' || e.proj === project) && (filter === 'All' || (AUDIT_GROUP[filter] || []).includes(e.kind)));
  const projTotal = KD5.audit.filter(e => e.proj === project).length;
  return (
    <div style={{ height: '100%', overflowY: 'auto', background: 'var(--surface-canvas)' }}>
      <div style={{ position: 'sticky', top: 0, zIndex: 5, padding: '14px 16px 10px', background: 'var(--surface-canvas)', borderBottom: '1px solid var(--border-subtle)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
          <span style={{ color: 'var(--slate-ink)' }}><BIco n="scroll-text" /></span>
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>Audit trail</h1>
          <BBadge mono style={{ color: 'var(--text-muted)' }}>append-only · {rows.length} events</BBadge>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 6, alignItems: 'center' }}>
            <div style={{ display: 'flex', borderRadius: 'var(--r-1)', overflow: 'hidden', border: '1px solid var(--border-default)' }}>
              <button onClick={() => setScope('project')} style={{ padding: '4px 9px', border: 'none', cursor: 'pointer', font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)', background: scope === 'project' ? 'var(--accent-surface)' : 'transparent', color: scope === 'project' ? 'var(--accent-ink)' : 'var(--text-muted)' }}>{projectName.length > 18 ? projectName.slice(0, 18) + '…' : projectName}</button>
              <button onClick={() => setScope('all')} style={{ padding: '4px 9px', border: 'none', borderLeft: '1px solid var(--border-default)', cursor: 'pointer', font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)', background: scope === 'all' ? 'var(--accent-surface)' : 'transparent', color: scope === 'all' ? 'var(--accent-ink)' : 'var(--text-muted)' }}>All projects</button>
            </div>
            <BBtn variant="ghost" size="sm" icon={<BIco n="download" />}>Export</BBtn>
          </div>
        </div>
        <div style={{ display: 'flex', gap: 6 }}>
          {KD5.auditFilters.map(f => (
            <button key={f} onClick={() => setFilter(f)} style={{ padding: '4px 10px', borderRadius: 999, cursor: 'pointer',
              border: `1px solid ${filter === f ? 'var(--accent-line)' : 'var(--border-default)'}`, background: filter === f ? 'var(--accent-surface)' : 'transparent',
              color: filter === f ? 'var(--accent-ink)' : 'var(--text-muted)', font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)' }}>{f}</button>
          ))}
        </div>
      </div>
      <div style={{ padding: '8px 16px 24px', maxWidth: 760 }}>
        {rows.length === 0
          ? <div style={{ font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-muted)', padding: '20px 4px', display: 'flex', alignItems: 'center', gap: 8 }}><BIco n="scroll-text" s={{ width: 15, height: 15, color: 'var(--text-faint)' }} /> No audit events for this project yet.</div>
          : rows.map((e, i) => <AuditRow key={i} e={e} last={i === rows.length - 1} />)}
      </div>
    </div>
  );
}

function AuditRow({ e, last }) {
  const resultColor = { approved: 'var(--success-ink)', denied: 'var(--danger-ink)', pending: 'var(--attention-ink)' }[e.result];
  return (
    <div style={{ display: 'flex', gap: 12, position: 'relative' }}>
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', flex: 'none', width: 26 }}>
        <span style={{ width: 26, height: 26, borderRadius: 999, background: 'var(--surface-card)', border: '1px solid var(--border-default)', color: e.kind === 'brain' ? 'var(--brain-ink)' : (resultColor || 'var(--text-muted)'), display: 'inline-flex', alignItems: 'center', justifyContent: 'center', zIndex: 1 }}><BIco n={AUDIT_ICON[e.kind] || 'dot'} s={{ width: 13, height: 13 }} /></span>
        {!last && <span style={{ flex: 1, width: 1, background: 'var(--border-subtle)', minHeight: 14 }} />}
      </div>
      <div style={{ flex: 1, minWidth: 0, paddingBottom: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
          <span style={{ font: 'var(--fw-medium) var(--fs-body) var(--font-sans)', color: 'var(--text-primary)' }}>{e.text}</span>
          {e.risk && <BRisk level={e.risk} />}
          {e.result && e.result !== 'pending' && <BBadge style={{ color: resultColor }}>{e.result}</BBadge>}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 3, font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)' }}>
          <span>{e.t}</span><span>·</span><span style={{ color: e.actor === 'You' ? 'var(--accent-ink)' : 'var(--text-muted)' }}>{e.actor}</span>
          {e.target && <><span>→</span><span>{e.target}</span></>}
          {e.meta && <BMeta tone={e.kind === 'pr' ? 'pr' : 'branch'} style={{ marginLeft: 2 }}>{e.meta}</BMeta>}
        </div>
      </div>
    </div>
  );
}

/* ---------------- Settings / Execution profiles ---------------- */
const HEALTH = {
  active: { pill: 'completed', label: 'Active' }, available: { pill: 'idle', label: 'Available' },
  'rate-limited': { pill: 'stale', label: 'Rate-limited' }, 'auth-expired': { pill: 'failed', label: 'Auth expired' },
};
function SettingsProfiles() {
  const [sec, setSec] = useS5('Integrations');
  useE5(() => { const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24); return () => clearTimeout(t); }, [sec]);
  return (
    <div style={{ height: '100%', overflowY: 'auto', background: 'var(--surface-canvas)' }}>
      <div style={{ padding: '14px 16px 0', borderBottom: '1px solid var(--border-subtle)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
          <span style={{ color: 'var(--text-secondary)' }}><BIco n="settings" /></span>
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>Settings</h1>
        </div>
        <div style={{ display: 'flex', gap: 2 }}>
          {['Integrations', 'Execution profiles', 'Usage', 'Security & policy'].map(t => (
            <button key={t} onClick={() => setSec(t)} style={{ padding: '8px 12px', border: 'none', background: 'transparent', cursor: 'pointer', font: `${sec === t ? 'var(--fw-semibold)' : 'var(--fw-medium)'} var(--fs-label) var(--font-sans)`, color: sec === t ? 'var(--accent-ink)' : 'var(--text-muted)', boxShadow: sec === t ? 'inset 0 -2px 0 var(--accent-solid)' : 'none' }}>{t}</button>
          ))}
        </div>
      </div>

      {sec === 'Integrations' && (
        <div style={{ padding: '16px', maxWidth: 760, display: 'flex', flexDirection: 'column', gap: 10 }}>
          {KD5.integrations.map(it => (
            <div key={it.id} style={{ border: `1px solid ${it.connected ? 'var(--border-default)' : 'var(--caution-line)'}`, borderRadius: 'var(--r-3)', background: it.connected ? 'var(--surface-card)' : 'var(--caution-surface)', padding: '13px 14px', display: 'flex', alignItems: 'center', gap: 13 }}>
              <span style={{ width: 34, height: 34, flex: 'none', borderRadius: 'var(--r-2)', background: 'var(--surface-active)', color: it.connected ? 'var(--text-secondary)' : 'var(--caution-ink)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><BIco n={it.icon} s={{ width: 17, height: 17 }} /></span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)' }}>{it.name}</span>
                  <BPill status={it.connected ? 'completed' : 'idle'} size="xs" label={it.connected ? 'Connected' : 'Not connected'} />
                  {it.scope && <BBadge mono style={{ color: 'var(--text-faint)' }}>{it.scope}</BBadge>}
                </div>
                <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)', marginTop: 3 }}>{it.detail}</div>
              </div>
              {it.connected
                ? <BBtn variant="secondary" size="sm" icon={<BIco n="settings-2" s={{ width: 13, height: 13 }} />}>{it.action}</BBtn>
                : <BBtn variant="attention" size="sm" icon={<BIco n="plug" s={{ width: 13, height: 13 }} />}>{it.action}</BBtn>}
            </div>
          ))}
          <div style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '11px 13px', borderRadius: 'var(--r-3)', border: '1px dashed var(--border-default)', color: 'var(--text-muted)', font: 'var(--fs-meta)/1.5 var(--font-sans)' }}>
            <BIco n="git-compare" s={{ width: 15, height: 15, color: 'var(--text-faint)' }} />
            Linking <strong style={{ color: 'var(--text-secondary)' }}>GitHub ↔ Linear</strong> lets a Linear ticket and its GitHub issue/PR resolve to one task in the inbox. Connect Linear to enable bi-directional mapping.
          </div>
        </div>
      )}

      {sec === 'Execution profiles' && (
      <div style={{ padding: '16px', maxWidth: 800, display: 'flex', flexDirection: 'column', gap: 10 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <BBadge mono style={{ color: 'var(--text-muted)' }}>{KD5.profilesDetail.length} configured</BBadge>
          <div style={{ marginLeft: 'auto' }}><BBtn variant="primary" size="sm" icon={<BIco n="plus" />}>Add profile</BBtn></div>
        </div>
        {KD5.profilesDetail.map((p, i) => {
          const h = HEALTH[p.health] || HEALTH.available;
          const over = p.usage >= 95;
          return (
            <div key={i} style={{ border: `1px solid ${over ? 'var(--warning-line)' : 'var(--border-default)'}`, borderRadius: 'var(--r-3)', background: 'var(--surface-card)', padding: '13px 14px', display: 'grid', gridTemplateColumns: 'auto 1fr 220px auto', alignItems: 'center', gap: 14 }}>
              <span style={{ width: 34, height: 34, flex: 'none', borderRadius: 'var(--r-2)', background: p.provider === 'claude' ? 'var(--domain-claude-surface)' : 'var(--domain-codex-surface)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', font: 'var(--font-mono)', color: p.provider === 'claude' ? 'var(--domain-claude)' : 'var(--domain-codex)', fontSize: 16 }}>{p.provider === 'claude' ? '✻' : '⌁'}</span>
              <div style={{ minWidth: 0 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)' }}>{p.name}</span>
                  <BPill status={h.pill} size="xs" label={h.label} />
                </div>
                <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)', marginTop: 3 }}>{p.note} · {p.sessions} active session{p.sessions === 1 ? '' : 's'}</div>
              </div>
              <div>
                <BMeter label="Quota" value={p.usage} max={p.limit} valueText={`${p.usage} / ${p.limit}${p.resets !== '—' ? ' · resets ' + p.resets : ''}`} />
              </div>
              <div style={{ display: 'flex', gap: 6, justifyContent: 'flex-end' }}>
                {p.health === 'auth-expired'
                  ? <BBtn variant="attention" size="sm" icon={<BIco n="refresh-cw" s={{ width: 13, height: 13 }} />}>Re-auth</BBtn>
                  : <BIconBtn icon={<BIco n="settings-2" s={{ width: 15, height: 15 }} />} size="sm" aria-label="Configure" />}
              </div>
            </div>
          );
        })}
      </div>
      )}

      {sec === 'Usage' && <UsageSection />}

      {sec === 'Security & policy' && (
        <div style={{ padding: '16px', maxWidth: 760, display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div>
            <BEye style={{ marginBottom: 10 }}>Default approval policy</BEye>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 7 }}>
              {[['shield-check', 'Read-only & low risk', 'Auto-approved — no confirmation', 'readonly'], ['shield', 'Medium risk', 'Confirm each action (create worktree, send to agent, draft PR)', 'medium'], ['shield-alert', 'High risk', 'Explicit confirmation (git writes, run commands, ticket changes)', 'high'], ['shield-x', 'Critical', 'Typed confirmation · no automation (force-push, delete, secrets)', 'critical']].map(([ic, name, desc, lvl], i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 11, padding: '11px 12px', borderRadius: 'var(--r-2)', border: '1px solid var(--border-default)', background: 'var(--surface-card)' }}>
                  <span style={{ color: 'var(--risk-' + lvl + ')' }}><BIco n={ic} s={{ width: 16, height: 16 }} /></span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}><span style={{ font: 'var(--fw-medium) var(--fs-body) var(--font-sans)' }}>{name}</span><BRisk level={lvl} /></div>
                    <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)', marginTop: 2 }}>{desc}</div>
                  </div>
                  <BIconBtn icon={<BIco n="settings-2" s={{ width: 15, height: 15 }} />} size="sm" aria-label="Edit" />
                </div>
              ))}
            </div>
          </div>
          <div>
            <BEye style={{ marginBottom: 10 }}>Standing permissions (policy automation)</BEye>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 7 }}>
              {[['Auto-summarize completed sessions to Brain', true], ['Auto-link branch names to plan tasks', true], ['Auto-create draft PR when checks pass', false], ['Auto-refresh stale owned docs', false]].map(([label, on], i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '9px 12px', borderRadius: 'var(--r-2)', border: '1px solid var(--border-subtle)', background: 'var(--surface-card)' }}>
                  <span style={{ font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-secondary)', flex: 1 }}>{label}</span>
                  <span style={{ width: 32, height: 18, borderRadius: 999, background: on ? 'var(--accent-solid)' : 'var(--surface-active)', position: 'relative', flex: 'none' }}><span style={{ position: 'absolute', top: 2, left: on ? 16 : 2, width: 14, height: 14, borderRadius: 999, background: on ? 'var(--accent-on-solid)' : 'var(--text-faint)', transition: 'left var(--dur-2)' }} /></span>
                </div>
              ))}
            </div>
            <div style={{ font: 'var(--fs-micro)/1.5 var(--font-sans)', color: 'var(--text-faint)', marginTop: 8, display: 'flex', alignItems: 'center', gap: 6 }}><BIco n="info" s={{ width: 13, height: 13 }} /> Standing permissions are opt-in, narrow, revocable, and fully audited via the Action Gateway.</div>
          </div>
        </div>
      )}
    </div>
  );
}

function UsageSection() {
  const u = KD5.usage;
  const max = Math.max(...u.spend14);
  const provColor = { claude: 'var(--domain-claude)', codex: 'var(--domain-codex)' };
  const card = { border: '1px solid var(--border-default)', borderRadius: 'var(--r-3)', background: 'var(--surface-card)', padding: '13px 14px' };
  return (
    <div style={{ padding: '16px', maxWidth: 820, display: 'flex', flexDirection: 'column', gap: 14 }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 10 }}>
        <StatCard label="Spend today" value={`$${u.today.spend}`} sub={`/ $${u.today.spendLimit} budget`} meter={u.today.spend / u.today.spendLimit} tone="caution" />
        <StatCard label="Tokens today" value={`${((u.today.tokensIn + u.today.tokensOut) / 1e6).toFixed(2)}M`} sub={`${(u.today.tokensIn/1e3)|0}k in · ${(u.today.tokensOut/1e3)|0}k out`} />
        <StatCard label="Aggregate context" value={`${u.today.context}k`} sub={`/ ${u.today.contextMax/1000}M window`} meter={u.today.context / u.today.contextMax} tone="accent" />
        <StatCard label="Active sessions" value={u.today.sessions} sub="across all projects" />
      </div>
      <div style={card}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 12 }}>
          <BEye>Spend · last 14 days</BEye>
          <span style={{ marginLeft: 'auto', font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>${u.spend14.reduce((a, b) => a + b, 0)} total</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'flex-end', gap: 5, height: 96 }}>
          {u.spend14.map((v, i) => (
            <div key={i} title={`$${v}`} style={{ flex: 1, height: `${(v / max) * 100}%`, borderRadius: '3px 3px 0 0', background: i === u.spend14.length - 1 ? 'var(--accent-solid)' : 'var(--accent-line)' }} />
          ))}
        </div>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
        <div style={card}>
          <BEye style={{ marginBottom: 11 }}>Spend by profile</BEye>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 9 }}>
            {u.byProfile.map((p, i) => (
              <div key={i}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                  <span style={{ font: 'var(--font-mono)', color: provColor[p.provider], fontSize: 13 }}>{p.provider === 'claude' ? '✻' : '⌁'}</span>
                  <span style={{ font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-primary)' }}>{p.name}</span>
                  <span style={{ marginLeft: 'auto', font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-secondary)' }}>${p.spend.toFixed(2)}</span>
                </div>
                <div style={{ height: 6, borderRadius: 999, background: 'var(--cap-track)', overflow: 'hidden' }}><i style={{ display: 'block', height: '100%', width: p.pct + '%', background: provColor[p.provider] }} /></div>
              </div>
            ))}
          </div>
        </div>
        <div style={card}>
          <BEye style={{ marginBottom: 11 }}>Top context consumers</BEye>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {u.topContext.map((c, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <span style={{ flex: 1, font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{c.session}</span>
                <BMeter value={c.value} max={c.max} valueText={`${c.value}k`} style={{ width: 150, flex: 'none' }} />
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function StatCard({ label, value, sub, meter, tone }) {
  const fill = tone === 'caution' ? 'var(--cap-warn)' : tone === 'accent' ? 'var(--accent)' : 'var(--text-secondary)';
  return (
    <div style={{ border: '1px solid var(--border-default)', borderRadius: 'var(--r-3)', background: 'var(--surface-card)', padding: '12px 13px' }}>
      <div style={{ font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-muted)' }}>{label}</div>
      <div style={{ font: 'var(--fw-semibold) var(--fs-h2)/1 var(--font-sans)', color: 'var(--text-primary)', margin: '6px 0 4px', fontVariantNumeric: 'tabular-nums' }}>{value}</div>
      <div style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>{sub}</div>
      {meter != null && <div style={{ height: 4, borderRadius: 999, background: 'var(--cap-track)', overflow: 'hidden', marginTop: 8 }}><i style={{ display: 'block', height: '100%', width: Math.min(100, meter * 100) + '%', background: fill }} /></div>}
    </div>
  );
}

window.KitViews4 = { ProjectBrainPage, AuditTimeline, SettingsProfiles };

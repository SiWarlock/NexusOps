/* ============================================================
   Control Plane UI kit — Editor (IDE) + Agent Team views.
   ============================================================ */
const _NS4 = window.ControlPlaneDesignSystem_a21911;
const { Button: EBtn, IconButton: EIconBtn, StatusPill: EPill,
        RiskBadge: ERisk, UsageMeter: EMeter, HarnessBadge: EHarness,
        ProfileBadge: EProfile, MetaChip: EMeta } = _NS4;
const EBadge = _NS4.Badge || (({ children, mono, style = {} }) => <span style={{ font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>);
const { Ico: EIco, Eyebrow: EEye } = window.KitShell;
const KD4 = window.KIT;

/* ---- lightweight TS tokenizer for cosmetic highlighting ---- */
const KW = /\b(import|from|export|async|function|const|let|return|if|else|await|type|interface|new|class)\b/g;
const TYPE = /\b([A-Z][A-Za-z0-9]+)\b/g;
function hi(line) {
  if (line.trim().startsWith('//')) return [{ c: 'var(--text-faint)', t: line }];
  const out = []; let i = 0;
  // naive: split on strings first
  const parts = line.split(/('[^']*')/g);
  parts.forEach(p => {
    if (p.startsWith("'") && p.endsWith("'")) { out.push({ c: 'var(--success-ink)', t: p }); return; }
    let last = 0; let m;
    const seg = p;
    const tokens = [];
    const re = new RegExp(KW.source + '|' + TYPE.source, 'g');
    while ((m = re.exec(seg))) {
      if (m.index > last) tokens.push({ c: 'var(--text-secondary)', t: seg.slice(last, m.index) });
      const isKw = KW.test(m[0]); KW.lastIndex = 0;
      tokens.push({ c: isKw ? 'var(--accent-ink)' : 'var(--brain-ink)', t: m[0] });
      last = m.index + m[0].length;
    }
    if (last < seg.length) tokens.push({ c: 'var(--text-secondary)', t: seg.slice(last) });
    out.push(...tokens);
  });
  return out;
}

/* ---------------- Editor / IDE ---------------- */
function EditorView({ openGateway }) {
  const { tree, files } = KD4;
  const markBg = { add: 'var(--diff-add-bg)', del: 'var(--diff-del-bg)', ctx: 'transparent' };
  const markGutter = { add: 'var(--diff-add-gutter)', del: 'var(--diff-del-gutter)' };
  const [active, setActive] = React.useState('review.ts');
  const [openTabs, setOpenTabs] = React.useState(['review.ts', 'risk.ts', 'gateway.test.ts']);
  React.useEffect(() => { setTimeout(() => window.lucide && window.lucide.createIcons(), 20); }, [active, openTabs]);
  const file = files[active] || files['review.ts'];
  const agentEdit = file.agentEdit;
  const problems = file.problems || [];
  const openFile = (name) => { if (!files[name]) return; setActive(name); setOpenTabs(o => o.includes(name) ? o : [...o, name]); };
  const closeTab = (e, name) => { e.stopPropagation(); setOpenTabs(o => { const next = o.filter(n => n !== name); if (active === name && next.length) setActive(next[next.length - 1]); return next.length ? next : o; }); };
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '210px 1fr', gridTemplateRows: '1fr', height: '100%', background: 'var(--surface-canvas)', minHeight: 0 }}>
      {/* Explorer */}
      <aside style={{ borderRight: '1px solid var(--border-default)', background: 'var(--surface-panel)', overflowY: 'auto', padding: '10px 6px' }}>
        <EEye style={{ padding: '0 8px 8px', display: 'flex', alignItems: 'center', gap: 6 }}><EIco n="folder-git-2" s={{ width: 12, height: 12 }} /> control-plane</EEye>
        {tree.map((node, i) => <TreeRow key={i} node={node} activeFile={active} onOpen={openFile} />)}
      </aside>

      {/* Editor column */}
      <div style={{ display: 'grid', gridTemplateRows: '34px 1fr 150px', minWidth: 0, minHeight: 0 }}>
        {/* Tab bar */}
        <div style={{ display: 'flex', alignItems: 'stretch', background: 'var(--surface-sunken)', borderBottom: '1px solid var(--border-default)', overflowX: 'auto' }}>
          {openTabs.map((name) => {
            const f = files[name]; if (!f) return null; const on = name === active;
            return (
              <div key={name} onClick={() => setActive(name)} style={{ display: 'flex', alignItems: 'center', gap: 7, padding: '0 10px 0 12px', borderRight: '1px solid var(--border-subtle)', cursor: 'pointer',
                background: on ? 'var(--surface-canvas)' : 'transparent', color: on ? 'var(--text-primary)' : 'var(--text-muted)',
                boxShadow: on ? 'inset 0 2px 0 var(--accent-solid)' : 'none', font: 'var(--fs-meta) var(--font-mono)' }}>
                <EIco n="file-code" s={{ width: 12, height: 12, color: f.agent ? 'var(--accent-ink)' : 'var(--text-faint)' }} />
                {name}
                {f.dirty
                  ? <span style={{ width: 6, height: 6, borderRadius: 999, background: 'var(--caution-solid)' }} />
                  : <span onClick={(e) => closeTab(e, name)} style={{ display: 'inline-flex', borderRadius: 3 }}><EIco n="x" s={{ width: 11, height: 11, color: 'var(--text-faint)' }} /></span>}
              </div>
            );
          })}
          <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 4, padding: '0 8px' }}>
            {file.agent && <><EHarness harness="claude-code" showLabel={false} /><span style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--accent-ink)' }}>live edit</span></>}
          </div>
        </div>

        {/* Code area */}
        <div style={{ overflow: 'auto', background: 'var(--surface-canvas)', position: 'relative', fontFamily: 'var(--font-mono)', fontSize: '13px' }}>
          {file.lines.map((line, i) => {
            const mark = file.marks[i];
            const isAgent = agentEdit && agentEdit.line === i;
            return (
              <div key={i} style={{ display: 'flex', alignItems: 'stretch', background: mark ? markBg[mark] : (isAgent ? 'var(--accent-surface)' : 'transparent'), minHeight: 22, lineHeight: '22px', position: 'relative' }}>
                <span style={{ width: 44, flex: 'none', textAlign: 'right', padding: '0 10px 0 0', color: 'var(--text-faint)', userSelect: 'none',
                  boxShadow: mark && markGutter[mark] ? `inset -2px 0 0 ${markGutter[mark]}` : 'none' }}>{i + 1}</span>
                <span style={{ paddingLeft: 10, whiteSpace: 'pre', flex: 1, minWidth: 0 }}>
                  {hi(line).map((tk, j) => <span key={j} style={{ color: tk.c }}>{tk.t}</span>)}
                  {isAgent && <span style={{ display: 'inline-block', width: 2, height: 15, marginLeft: 1, background: 'var(--accent-solid)', verticalAlign: 'middle', animation: 'cp-live-pulse 1.1s steps(1) infinite' }} />}
                </span>
                {isAgent && (
                  <span style={{ position: 'absolute', right: 8, top: 1, display: 'inline-flex', alignItems: 'center', gap: 5, height: 18, padding: '0 7px', borderRadius: 'var(--r-1)', background: 'var(--accent-solid)', color: 'var(--accent-on-solid)', font: 'var(--fw-medium) var(--fs-micro)/1 var(--font-sans)' }}>
                    <span style={{ width: 5, height: 5, borderRadius: 999, background: 'currentColor', animation: 'cp-live-pulse 1.6s var(--ease-inout) infinite' }} /> {agentEdit.who}
                  </span>
                )}
              </div>
            );
          })}
        </div>

        {/* Bottom panel */}
        <div style={{ borderTop: '1px solid var(--border-default)', background: 'var(--surface-sunken)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '0 12px', height: 30, borderBottom: '1px solid var(--border-subtle)' }}>
            <span style={{ font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-primary)', borderBottom: '2px solid var(--accent-solid)', height: 30, display: 'flex', alignItems: 'center' }}>{agentEdit ? 'Agent activity' : 'Problems'}</span>
            <span style={{ font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-muted)' }}>Problems <EBadge mono style={{ color: problems.length ? 'var(--warning-ink)' : 'var(--text-faint)' }}>{problems.length}</EBadge></span>
            <span style={{ marginLeft: 'auto', font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)' }}>{file.path}/{active}</span>
            {agentEdit && <><ERisk level="medium" /><EBtn variant="attention" size="xs" icon={<EIco n="check" s={{ width: 13, height: 13 }} />} onClick={openGateway}>Approve edit</EBtn></>}
          </div>
          <div style={{ overflowY: 'auto', padding: '8px 12px', font: 'var(--fs-meta)/1.7 var(--font-mono)', color: 'var(--text-secondary)' }}>
            {agentEdit
              ? <>
                  <div><span style={{ color: 'var(--accent-ink)' }}>{agentEdit.who}</span> · {agentEdit.note}</div>
                  <div style={{ color: 'var(--text-faint)' }}>↳ proposing edits in {file.path}/{active}</div>
                  {problems[0] && <div style={{ color: 'var(--warning-ink)' }}>⚠ {problems[0].text} <span style={{ color: 'var(--text-faint)' }}>({problems[0].at})</span></div>}
                  <div style={{ color: 'var(--text-faint)' }}>awaiting approval to apply edit + run tests…</div>
                </>
              : problems.length
                ? problems.map((p, i) => <div key={i} style={{ color: p.sev === 'fail' ? 'var(--danger-ink)' : 'var(--warning-ink)' }}>{p.sev === 'fail' ? '✕' : '⚠'} {p.text} <span style={{ color: 'var(--text-faint)' }}>({p.at})</span></div>)
                : <div style={{ color: 'var(--text-faint)' }}>No problems in this file.</div>}
          </div>
        </div>
      </div>
    </div>
  );
}

function TreeRow({ node, activeFile, onOpen }) {
  const isFile = node.type === 'file';
  const active = isFile && node.name === activeFile;
  const gitColor = { M: 'var(--caution-ink)', A: 'var(--success-ink)', D: 'var(--danger-ink)' }[node.git];
  return (
    <div onClick={() => isFile && onOpen(node.name)} style={{ display: 'flex', alignItems: 'center', gap: 6, height: 24, paddingLeft: 8 + node.depth * 13, paddingRight: 8, borderRadius: 'var(--r-1)', cursor: 'pointer',
      background: active ? 'var(--surface-active)' : 'transparent' }}>
      <EIco n={isFile ? 'file-code' : (node.open ? 'chevron-down' : 'chevron-right')} s={{ width: 13, height: 13, color: 'var(--text-faint)', flex: 'none' }} />
      {!isFile && !node.open && <EIco n="folder" s={{ width: 13, height: 13, color: 'var(--text-faint)', marginLeft: -2 }} />}
      <span style={{ flex: 1, font: `${isFile ? 'var(--fw-regular)' : 'var(--fw-medium)'} var(--fs-meta) var(--font-mono)`, color: active ? 'var(--text-primary)' : 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{node.name}</span>
      {node.agent && <span title="agent editing" style={{ width: 6, height: 6, borderRadius: 999, background: 'var(--accent-solid)', animation: 'cp-live-pulse 1.6s var(--ease-inout) infinite' }} />}
      {node.git && <span style={{ font: 'var(--fs-micro) var(--font-mono)', color: gitColor, fontWeight: 'var(--fw-semibold)' }}>{node.git}</span>}
    </div>
  );
}

/* ---------------- Agent Team ---------------- */
function AgentTeamView({ openGateway, onOpenTerminals }) {
  const t = KD4.team;
  return (
    <div style={{ height: '100%', overflowY: 'auto', background: 'var(--surface-canvas)' }}>
      <div style={{ padding: '14px 16px', borderBottom: '1px solid var(--border-subtle)', display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ color: 'var(--teal-ink)' }}><EIco n="users-round" /></span>
        <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)' }}>{t.name}</h1>
        <EBadge tone="teal" style={{ display: 'inline-flex', alignItems: 'center', height: 18, padding: '0 6px', borderRadius: 'var(--r-1)', background: 'var(--teal-surface)', color: 'var(--teal-ink)' }}>{t.pack}</EBadge>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
          <EBtn variant="secondary" size="sm" icon={<EIco n="terminal" />} onClick={onOpenTerminals}>Open terminals</EBtn>
          <EBtn variant="ghost" size="sm" icon={<EIco n="git-merge" />}>Integrate</EBtn>
          <EBtn variant="ghost" size="sm" icon={<EIco n="pause" />}>Pause team</EBtn>
        </div>
      </div>

      <div style={{ padding: '16px', maxWidth: 760 }}>
        {/* Orchestrator */}
        <EEye style={{ marginBottom: 8 }}>Orchestrator</EEye>
        <div onClick={onOpenTerminals} title="Open terminals" style={{ cursor: 'pointer', border: '1px solid var(--teal-line)', background: 'var(--teal-surface)', borderRadius: 'var(--r-3)', padding: '12px 13px', marginBottom: 6, display: 'flex', alignItems: 'center', gap: 11 }}>
          <span style={{ width: 30, height: 30, flex: 'none', borderRadius: 'var(--r-2)', background: 'var(--teal-solid)', color: 'var(--teal-on-solid)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><EIco n="workflow" s={{ width: 16, height: 16 }} /></span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)' }}>{t.lead.role}</span>
              <EPill status={t.lead.status} size="xs" />
            </div>
            <div style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)', marginTop: 3 }}>{t.lead.task}</div>
          </div>
          <EHarness harness={t.lead.harness} />
          <EMeter variant="ring" size="sm" value={t.lead.ctx} max={200} label="ctx" />
          <EIco n="terminal" s={{ width: 15, height: 15, color: 'var(--teal-ink)' }} />
        </div>

        {/* connector */}
        <div style={{ height: 14, borderLeft: '1.5px solid var(--teal-line)', marginLeft: 24 }} />

        <EEye style={{ marginBottom: 8 }}>Workers · {t.workers.length}</EEye>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {t.workers.map(w => (
            <div key={w.id} onClick={onOpenTerminals} title="Open terminals" style={{ cursor: 'pointer', position: 'relative', border: '1px solid var(--border-default)', background: 'var(--surface-card)', borderRadius: 'var(--r-3)', padding: '11px 12px', display: 'flex', alignItems: 'center', gap: 11, marginLeft: 24 }}>
              <span style={{ position: 'absolute', left: -24, top: '50%', width: 24, height: 1, background: 'var(--teal-line)' }} />
              <span style={{ width: 28, height: 28, flex: 'none', borderRadius: 'var(--r-2)', background: 'var(--surface-active)', color: 'var(--text-secondary)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><EIco n="bot" s={{ width: 15, height: 15 }} /></span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ font: 'var(--fw-medium) var(--fs-body) var(--font-sans)' }}>{w.role}</span>
                  <EPill status={w.status} size="xs" beacon={w.status === 'waiting-perm'} />
                </div>
                <div style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)', marginTop: 3 }}>{w.task} · {w.wt}</div>
              </div>
              <EHarness harness={w.harness} showLabel={false} />
              <EMeter variant="ring" size="sm" value={w.ctx} max={200} label="ctx" />
              {w.status === 'waiting-perm'
                ? <EBtn variant="attention" size="xs" onClick={(e) => { e.stopPropagation(); openGateway(); }}>Grant</EBtn>
                : <EIconBtn icon={<EIco n="terminal" s={{ width: 15, height: 15 }} />} size="sm" aria-label="Open terminal" onClick={(e) => { e.stopPropagation(); onOpenTerminals(); }} />}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

window.KitViews3 = { EditorView, AgentTeamView };

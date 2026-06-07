/* ============================================================
   Control Plane UI kit — Workflow Packs view.
   Enforces the pack-vs-instance distinction: a pack being
   installed never implies its commands are ready to run.
   ============================================================ */
const _NS6 = window.ControlPlaneDesignSystem_a21911;
const { Button: WBtn, IconButton: WIconBtn, StatusPill: WPill,
        RiskBadge: WRisk, MetaChip: WMeta } = _NS6;
const WBadge = _NS6.Badge || (({ children, mono, style = {} }) => <span style={{ font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>);
const { Ico: WIco, Eyebrow: WEye } = window.KitShell;
const KD6 = window.KIT;
const { useState: useS6 } = React;

// instance status -> { pill, label, tone }
const INSTANCE = {
  active:               { pill: 'running',      label: 'Active' },
  ready:                { pill: 'completed',    label: 'Ready' },
  needs_personalization:{ pill: 'waiting-perm', label: 'Needs personalization' },
  personalizing:        { pill: 'running',      label: 'Personalizing' },
  drift_detected:       { pill: 'degraded',     label: 'Drift detected' },
  upgrade_available:    { pill: 'stale',        label: 'Upgrade available' },
  detected:             { pill: 'idle',         label: 'Detected' },
  archived:             { pill: 'archived',     label: 'Archived' },
};
const PROVIDER = { bundled: 'Bundled', user: 'You', third_party: '3rd-party' };

function WorkflowPacksView({ openGateway, setView }) {
  const [sel, setSel] = useS6('cc-crew');
  React.useEffect(() => { const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24); return () => clearTimeout(t); }, [sel]);
  const pack = KD6.packs.find(p => p.id === sel) || KD6.packs[0];
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '300px 1fr', height: '100%', background: 'var(--surface-canvas)', minHeight: 0 }}>
      {/* pack library */}
      <aside style={{ borderRight: '1px solid var(--border-default)', background: 'var(--surface-panel)', overflowY: 'auto' }}>
        <div style={{ padding: '14px 14px 8px', display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ color: 'var(--teal-ink)' }}><WIco n="package" /></span>
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-sub)/1 var(--font-sans)' }}>Workflow Packs</h1>
          <span style={{ marginLeft: 'auto' }}><WIconBtn icon={<WIco n="plus" s={{ width: 15, height: 15 }} />} size="sm" aria-label="Install pack" /></span>
        </div>
        <div style={{ padding: '4px 8px 14px' }}>
          {KD6.packs.map(p => {
            const inst = INSTANCE[p.instance] || INSTANCE.detected;
            const active = sel === p.id;
            return (
              <button key={p.id} onClick={() => setSel(p.id)} style={{ display: 'block', width: '100%', textAlign: 'left', cursor: 'pointer', border: 'none', borderRadius: 'var(--r-2)', padding: '9px 10px', marginBottom: 3,
                background: active ? 'var(--surface-active)' : 'transparent', boxShadow: active ? 'inset 0 0 0 1px var(--accent-line)' : 'none' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
                  <span style={{ font: 'var(--fw-medium) var(--fs-body) var(--font-mono)', color: 'var(--text-primary)' }}>{p.name}</span>
                  <span style={{ marginLeft: 'auto' }}><WPill status={inst.pill} size="xs" label={inst.label} beacon={p.instance === 'needs_personalization'} /></span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 6, font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)' }}>
                  <span>v{p.version}</span><span>·</span><span>{PROVIDER[p.provider]}</span><span>·</span><span>{p.project}</span>
                </div>
              </button>
            );
          })}
        </div>
      </aside>

      {/* pack detail */}
      <PackDetail pack={pack} openGateway={openGateway} setView={setView} />
    </div>
  );
}

function PackDetail({ pack, openGateway, setView }) {
  const inst = INSTANCE[pack.instance] || INSTANCE.detected;
  const notReady = pack.instance === 'needs_personalization';
  const upgrade = pack.instance === 'upgrade_available';
  return (
    <div style={{ overflowY: 'auto', minWidth: 0 }}>
      <div style={{ padding: '16px', borderBottom: '1px solid var(--border-subtle)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
          <h1 style={{ margin: 0, font: 'var(--fw-semibold) var(--fs-h2)/1 var(--font-mono)', letterSpacing: 'var(--tracking-tight)' }}>{pack.name}</h1>
          <WPill status={inst.pill} label={inst.label} beacon={notReady} />
          <WBadge mono style={{ color: 'var(--text-muted)' }}>v{pack.version}</WBadge>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
            {notReady && <WBtn variant="attention" size="sm" icon={<WIco n="wand-sparkles" s={{ width: 14, height: 14 }} />} onClick={() => openGateway('edit')}>Personalize</WBtn>}
            {upgrade && <WBtn variant="primary" size="sm" icon={<WIco n="arrow-up-circle" s={{ width: 14, height: 14 }} />} onClick={() => openGateway('edit')}>Upgrade</WBtn>}
            {!notReady && !upgrade && <WBtn variant="secondary" size="sm" icon={<WIco n="settings-2" s={{ width: 14, height: 14 }} />}>Configure</WBtn>}
          </div>
        </div>
        <p style={{ margin: 0, font: 'var(--fs-body)/1.5 var(--font-sans)', color: 'var(--text-secondary)', maxWidth: 620 }}>{pack.desc}</p>
      </div>

      {/* readiness banner — the pack≠instance guardrail */}
      {notReady && (
        <div style={{ margin: '14px 16px 0', display: 'flex', alignItems: 'flex-start', gap: 10, padding: '11px 13px', borderRadius: 'var(--r-3)', background: 'var(--caution-surface)', border: '1px solid var(--caution-line)' }}>
          <span style={{ color: 'var(--caution-ink)', marginTop: 1 }}><WIco n="triangle-alert" s={{ width: 16, height: 16 }} /></span>
          <div>
            <div style={{ font: 'var(--fw-semibold) var(--fs-label) var(--font-sans)', color: 'var(--caution-ink)' }}>Template pack — not ready to run</div>
            <div style={{ font: 'var(--fs-meta)/1.5 var(--font-sans)', color: 'var(--text-secondary)', marginTop: 3 }}>This pack is installed but has no personalized instance for {pack.project}. Commands stay locked until personalization completes (a Gateway-reviewed run).</div>
          </div>
        </div>
      )}
      {upgrade && (
        <div style={{ margin: '14px 16px 0', display: 'flex', alignItems: 'center', gap: 10, padding: '11px 13px', borderRadius: 'var(--r-3)', background: 'var(--warning-surface)', border: '1px solid var(--warning-line)' }}>
          <span style={{ color: 'var(--warning-ink)' }}><WIco n="arrow-up-circle" s={{ width: 16, height: 16 }} /></span>
          <div style={{ font: 'var(--fs-meta)/1.5 var(--font-sans)', color: 'var(--text-secondary)' }}><strong style={{ color: 'var(--warning-ink)' }}>v{pack.version} → newer available.</strong> Review the changelog before upgrading; owned files will be re-generated through the Gateway.</div>
        </div>
      )}

      <div style={{ padding: '16px', display: 'grid', gridTemplateColumns: '1fr 240px', gap: 16, alignItems: 'start' }}>
        {/* commands */}
        <div>
          <WEye style={{ marginBottom: 10 }}>Commands · {pack.commands.length}</WEye>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 7 }}>
            {pack.commands.map((c, i) => {
              const locked = c.needsInstance && notReady;
              return (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px', borderRadius: 'var(--r-2)', border: '1px solid var(--border-default)', background: 'var(--surface-card)', opacity: locked ? 0.6 : 1 }}>
                  <span style={{ color: locked ? 'var(--text-faint)' : 'var(--accent-ink)' }}><WIco n={c.type === 'recipe' ? 'workflow' : 'slash'} s={{ width: 15, height: 15 }} /></span>
                  <code style={{ font: 'var(--fw-medium) var(--fs-body) var(--font-mono)', color: 'var(--text-primary)' }}>{c.name}</code>
                  <div style={{ display: 'flex', gap: 5, marginLeft: 4 }}>
                    <WBadge style={{ display: 'inline-flex', alignItems: 'center', height: 16, padding: '0 6px', borderRadius: 'var(--r-1)', background: 'var(--neutral-surface)', color: 'var(--text-muted)' }}>{c.type.replace('_', ' ')}</WBadge>
                    {c.creates && <WBadge style={{ display: 'inline-flex', alignItems: 'center', height: 16, padding: '0 6px', borderRadius: 'var(--r-1)', background: 'var(--teal-surface)', color: 'var(--teal-ink)' }}>creates {c.creates}</WBadge>}
                  </div>
                  <span style={{ marginLeft: 'auto' }}>
                    {locked
                      ? <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5, font: 'var(--fs-meta) var(--font-sans)', color: 'var(--caution-ink)' }}><WIco n="lock" s={{ width: 12, height: 12 }} /> needs instance</span>
                      : <WBtn variant="ghost" size="xs" icon={<WIco n="play" s={{ width: 12, height: 12 }} />} onClick={() => c.creates === 'agent team' ? setView('team') : openGateway('q1')}>Run</WBtn>}
                  </span>
                </div>
              );
            })}
          </div>

          {pack.roles.length > 0 && (
            <div style={{ marginTop: 18 }}>
              <WEye style={{ marginBottom: 10 }}>Agent roles</WEye>
              <div style={{ display: 'flex', gap: 7, flexWrap: 'wrap' }}>
                {pack.roles.map((r, i) => (
                  <span key={i} style={{ display: 'inline-flex', alignItems: 'center', gap: 6, height: 26, padding: '0 10px', borderRadius: 'var(--r-2)', border: '1px solid var(--teal-line)', background: 'var(--teal-surface)', color: 'var(--teal-ink)', font: 'var(--fw-medium) var(--fs-label) var(--font-sans)' }}>
                    <WIco n={i === 0 ? 'workflow' : 'bot'} s={{ width: 13, height: 13 }} /> {r}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* capabilities sidebar */}
        <div style={{ border: '1px solid var(--border-subtle)', borderRadius: 'var(--r-3)', background: 'var(--surface-card)', padding: '13px 14px' }}>
          <WEye style={{ marginBottom: 11 }}>Capabilities</WEye>
          <CapRow icon="terminal" label="Commands" val={pack.commands.length} />
          <CapRow icon="users-round" label="Agent roles" val={pack.roles.length || '—'} />
          <CapRow icon="rocket" label="Launch recipes" val={pack.recipes} />
          <CapRow icon="file-text" label="Plan parser" val={pack.parser || '—'} mono={!!pack.parser} />
          <div style={{ height: 1, background: 'var(--border-subtle)', margin: '10px 0' }} />
          <CapRow icon="git-merge" label="Mutations" val="via Gateway" />
          <CapRow icon="badge-check" label="Provider" val={PROVIDER[pack.provider]} />
        </div>
      </div>
    </div>
  );
}

function CapRow({ icon, label, val, mono }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '5px 0' }}>
      <span style={{ color: 'var(--text-faint)' }}><WIco n={icon} s={{ width: 14, height: 14 }} /></span>
      <span style={{ font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-muted)' }}>{label}</span>
      <span style={{ marginLeft: 'auto', font: `var(--fw-medium) var(--fs-label) ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, color: 'var(--text-primary)' }}>{val}</span>
    </div>
  );
}

window.KitViews5 = { WorkflowPacksView };

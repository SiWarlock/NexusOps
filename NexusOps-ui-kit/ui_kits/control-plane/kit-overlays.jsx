/* ============================================================
   Control Plane UI kit — overlays.
   BrainDrawer · GatewayModal · CommandPalette
   ============================================================ */
const _NS3 = window.ControlPlaneDesignSystem_a21911;
const { Button: OBtn, IconButton: OIconBtn, StatusPill: OPill,
        RiskBadge: ORisk, EvidenceChip: OEvidence, MetaChip: OMeta } = _NS3;
const OBadge = _NS3.Badge || (({ children, mono, tone, variant, style = {} }) => (
  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, height: 18, padding: '0 6px', borderRadius: 'var(--r-1)', background: 'var(--neutral-surface)', color: 'var(--text-secondary)', font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`, ...style }}>{children}</span>
));
const { Ico: OIco, Eyebrow: OEye } = window.KitShell;

/* ---------------- Project Brain drawer (hosts the co-pilot) ---------------- */
function BrainDrawer({ open, onClose, openGateway, onExpand }) {
  if (!open) return null;
  const Copilot = window.KitViews4 && window.KitViews4.ProjectBrainPage;
  return (
    <Overlay open={open} onClose={onClose} align="right" width={480}>
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--surface-canvas)', borderLeft: '1px solid var(--brain-line)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '10px 12px', borderBottom: '1px solid var(--border-default)', background: 'var(--brain-surface)' }}>
          <span style={{ color: 'var(--brain-ink)' }}><OIco n="brain" /></span>
          <span style={{ font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)', color: 'var(--text-primary)' }}>Project Brain</span>
          <OBadge tone="brain" variant="dot" style={{ marginLeft: 4 }}>co-pilot</OBadge>
          <span style={{ marginLeft: 'auto', display: 'flex', gap: 4 }}>
            <OIconBtn icon={<OIco n="brain-circuit" s={{ width: 15, height: 15 }} />} aria-label="Memory & decisions" onClick={onExpand} />
            <OIconBtn icon={<OIco n="x" />} aria-label="Close" onClick={onClose} />
          </span>
        </div>
        <div style={{ flex: 1, minHeight: 0 }}>
          {Copilot ? <Copilot openGateway={(id) => { openGateway && openGateway(id || 'edit'); }} drawer /> : null}
        </div>
      </div>
    </Overlay>
  );
}

/* ---------------- Action Gateway modal ---------------- */
function GatewayModal({ open, item, onClose, onResolve }) {
  if (!open || !item) return null;
  const isHigh = item.risk === 'high' || item.risk === 'critical';
  return (
    <Overlay open={open} onClose={onClose} align="center" width={520}>
      <div style={{ background: 'var(--surface-card)', border: '1px solid var(--border-strong)', borderRadius: 'var(--r-4)', boxShadow: 'var(--elev-4)', overflow: 'hidden' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '13px 16px', borderBottom: '1px solid var(--border-default)' }}>
          <span style={{ color: 'var(--accent-ink)' }}><OIco n="shield-check" /></span>
          <span style={{ font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)' }}>Action Gateway</span>
          <ORisk level={item.risk} style={{ marginLeft: 4 }} />
          <span style={{ marginLeft: 'auto' }}><OIconBtn icon={<OIco n="x" />} aria-label="Close" onClick={onClose} /></span>
        </div>
        <div style={{ padding: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
            <OPill status="waiting-human" beacon />
            <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>{item.who}</span>
          </div>
          <div style={{ font: 'var(--fw-medium) var(--fs-body-lg)/1.4 var(--font-sans)', marginBottom: 6 }}>{item.title.split(item.cmd)[0]}<code style={{ font: 'var(--fs-body) var(--font-mono)', background: 'var(--surface-sunken)', padding: '2px 6px', borderRadius: 4, color: isHigh ? 'var(--attention-ink)' : 'var(--accent-ink)' }}>{item.cmd}</code></div>
          <div style={{ font: 'var(--fs-label)/1.5 var(--font-sans)', color: 'var(--text-muted)', marginBottom: 14 }}>
            Worktree <code style={{ font: 'var(--fs-meta) var(--font-mono)' }}>{item.wt}</code> · {item.desc}
          </div>
          <div style={{ background: 'var(--surface-sunken)', borderRadius: 'var(--r-2)', padding: '11px 12px', boxShadow: 'var(--elev-inset)', marginBottom: 4 }}>
            <OEye style={{ marginBottom: 8 }}>What will happen</OEye>
            {item.conseq.map((c, i) => <ConseqLine key={i} icon={c[0]} text={c[1]} />)}
          </div>
        </div>
        <div style={{ padding: '12px 16px', borderTop: '1px solid var(--border-default)', display: 'flex', alignItems: 'center', gap: 8 }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: 7, font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-muted)', marginRight: 'auto' }}>
            <input type="checkbox" style={{ accentColor: 'var(--accent-solid)' }} /> Always allow in this project
          </label>
          <OBtn variant="ghost" size="md" onClick={() => onResolve(item, 'Denied')}>Deny</OBtn>
          <OBtn variant={isHigh ? 'danger-solid' : 'attention'} size="md" icon={<OIco n="check" />} kbd="⏎" onClick={() => onResolve(item, 'Approved')}>{isHigh ? 'Approve force-push' : 'Approve once'}</OBtn>
        </div>
      </div>
    </Overlay>
  );
}

function ConseqLine({ icon, text }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '3px 0', font: 'var(--fs-label) var(--font-sans)', color: 'var(--text-secondary)' }}>
      <span style={{ color: 'var(--text-faint)' }}><OIco n={icon} s={{ width: 14, height: 14 }} /></span>{text}
    </div>
  );
}

/* ---------------- Command palette (⌘K) ---------------- */
const CMDS = [
  { icon: 'brain', label: 'Ask Project Brain', hint: 'co-pilot', kbd: 'B', action: 'overlay:brain' },
  { icon: 'inbox', label: 'Open Task Inbox', hint: 'GitHub · Linear', kbd: 'P', action: 'overlay:tasks' },
  { icon: 'play', label: 'Start new session', hint: 'session', kbd: 'S', action: 'view:terminal' },
  { icon: 'package', label: 'Open Workflow Packs', hint: '/team-start', kbd: 'W', action: 'view:packs' },
  { icon: 'list-checks', label: 'Open Implementation Plan', hint: 'phases', action: 'view:plan' },
  { icon: 'users-round', label: 'Open Agent Team', hint: 'Phase 2 · cc-crew', action: 'view:team' },
  { icon: 'code-xml', label: 'Open Editor', hint: 'IDE', action: 'view:editor' },
  { icon: 'git-pull-request', label: 'Review pull request #84', hint: 'PR', action: 'view:code' },
  { icon: 'shield-check', label: 'Open Action Gateway queue', hint: 'approvals', action: 'overlay:gateway' },
  { icon: 'scroll-text', label: 'Open Audit Trail', hint: 'events', action: 'view:audit' },
  { icon: 'settings', label: 'Execution profiles & settings', hint: 'config', action: 'view:settings' },
];
function CommandPalette({ open, onClose, onAction }) {
  return (
    <Overlay open={open} onClose={onClose} align="top" width={560}>
      <div style={{ background: 'var(--surface-card)', border: '1px solid var(--border-strong)', borderRadius: 'var(--r-4)', boxShadow: 'var(--elev-4)', overflow: 'hidden' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '12px 14px', borderBottom: '1px solid var(--border-default)' }}>
          <span style={{ color: 'var(--text-faint)' }}><OIco n="search" /></span>
          <input autoFocus placeholder="Search objects or run a command…" style={{ flex: 1, background: 'transparent', border: 'none', outline: 'none', color: 'var(--text-primary)', font: 'var(--fs-body-lg) var(--font-sans)' }} />
          <kbd style={{ font: '10px var(--font-mono)', color: 'var(--text-faint)', border: '1px solid var(--border-default)', borderRadius: 3, padding: '1px 5px' }}>esc</kbd>
        </div>
        <div style={{ padding: '6px', maxHeight: 320, overflowY: 'auto' }}>
          <div style={{ font: 'var(--fs-micro) var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-faint)', padding: '7px 9px 4px' }}>Commands</div>
          {CMDS.map((c, i) => (
            <div key={i} onClick={() => { onAction && onAction(c.action); onClose(); }} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 9px', borderRadius: 'var(--r-2)', cursor: 'pointer', background: i === 0 ? 'var(--accent-surface)' : 'transparent' }}>
              <span style={{ color: i === 0 ? 'var(--accent-ink)' : 'var(--text-muted)' }}><OIco n={c.icon} s={{ width: 15, height: 15 }} /></span>
              <span style={{ flex: 1, font: 'var(--fs-body) var(--font-sans)', color: 'var(--text-primary)' }}>{c.label}</span>
              <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-faint)' }}>{c.hint}</span>
              {c.kbd && <kbd style={{ font: '10px var(--font-mono)', color: 'var(--text-muted)', border: '1px solid var(--border-default)', borderRadius: 3, padding: '1px 5px' }}>{c.kbd}</kbd>}
            </div>
          ))}
        </div>
      </div>
    </Overlay>
  );
}

/* ---------------- Overlay shell ---------------- */
function Overlay({ open, onClose, align = 'center', width = 480, children }) {
  if (!open) return null;
  const pos = {
    center: { alignItems: 'center', justifyContent: 'center' },
    top: { alignItems: 'flex-start', justifyContent: 'center', paddingTop: '12vh' },
    right: { alignItems: 'stretch', justifyContent: 'flex-end' },
  }[align];
  return (
    <div onClick={onClose} style={{ position: 'absolute', inset: 0, zIndex: 'var(--z-modal)', display: 'flex', background: 'var(--scrim)', backdropFilter: 'blur(2px)', ...pos }}>
      <div onClick={e => e.stopPropagation()} style={{ width: align === 'right' ? width : undefined, maxWidth: align === 'right' ? undefined : width, width: align === 'right' ? width : '100%', margin: align === 'right' ? 0 : '0 16px',
        animation: align === 'right' ? 'cp-slide-in 0.24s var(--ease-out)' : 'cp-pop-in 0.18s var(--ease-out)' }}>
        {children}
      </div>
    </div>
  );
}

/* ---------------- Inspector drawer (graph node / object) ---------------- */
function InspectorDrawer({ node, onClose, onOpen }) {
  if (!node) return null;
  const kindIcon = (window.NODE_KIND_ICON || {})[node.kind] || 'box';
  const tint = node.kind === 'brain' ? 'var(--brain-ink)' : node.kind === 'team' ? 'var(--teal-ink)' : 'var(--text-secondary)';
  const surf = node.kind === 'brain' ? 'var(--brain-surface)' : node.kind === 'team' ? 'var(--teal-surface)' : 'var(--surface-active)';
  return (
    <Overlay open={!!node} onClose={onClose} align="right" width={340}>
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--surface-panel)', borderLeft: '1px solid var(--border-strong)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '12px 14px', borderBottom: '1px solid var(--border-default)' }}>
          <span style={{ width: 28, height: 28, flex: 'none', borderRadius: 'var(--r-2)', background: surf, color: tint, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}><OIco n={kindIcon} s={{ width: 15, height: 15 }} /></span>
          <div style={{ minWidth: 0, flex: 1 }}>
            <div style={{ font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{node.title}</div>
            <div style={{ font: 'var(--fs-micro) var(--font-mono)', color: 'var(--text-faint)', textTransform: 'uppercase', letterSpacing: 'var(--tracking-caps)' }}>Inspector · {node.kind}</div>
          </div>
          <OIconBtn icon={<OIco n="x" />} aria-label="Close" onClick={onClose} />
        </div>
        <div style={{ padding: '12px 14px', borderBottom: '1px solid var(--border-subtle)', display: 'flex', alignItems: 'center', gap: 8 }}>
          <OPill status={node.status} beacon={node.beacon} />
          {node.subtitle && <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-muted)' }}>{node.subtitle}</span>}
        </div>
        <div style={{ flex: 1, overflowY: 'auto', padding: '12px 14px' }}>
          <OEye style={{ marginBottom: 8 }}>Details</OEye>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {Object.entries(node.detail || {}).map(([k, v]) => (
              <div key={k} style={{ display: 'flex', alignItems: 'baseline', gap: 12, padding: '7px 0', borderBottom: '1px solid var(--border-subtle)' }}>
                <span style={{ flex: 'none', width: 84, font: 'var(--fs-meta) var(--font-sans)', color: 'var(--text-faint)' }}>{k}</span>
                <span style={{ font: 'var(--fs-meta) var(--font-mono)', color: 'var(--text-secondary)', textAlign: 'right', marginLeft: 'auto' }}>{v}</span>
              </div>
            ))}
          </div>
        </div>
        {node.open && (
          <div style={{ padding: '12px 14px', borderTop: '1px solid var(--border-default)', display: 'flex', gap: 8 }}>
            <OBtn variant="primary" size="md" full icon={<OIco n={node.open.team ? 'terminal' : 'arrow-right'} s={{ width: 14, height: 14 }} />} onClick={() => { onClose(); onOpen && onOpen(node.open); }}>{node.open.label}</OBtn>
          </div>
        )}
      </div>
    </Overlay>
  );
}

window.KitOverlays = { BrainDrawer, GatewayModal, CommandPalette, InspectorDrawer };

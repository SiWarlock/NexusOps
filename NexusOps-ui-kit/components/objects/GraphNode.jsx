// GraphNode — an operational node for the observability graph. NOT decorative:
// it shows node type (glyph + domain tint), status, ownership, and exposes a
// selected/attention state. Node chrome = domain; status ring = state.
import React from 'react';

const KIND = {
  project:  { glyph: '◰', tint: 'var(--text-secondary)', label: 'Project' },
  session:  { glyph: '▷', tint: 'var(--accent-ink)',     label: 'Session' },
  team:     { glyph: '⧉', tint: 'var(--teal-ink)',       label: 'Agent Team' },
  worker:   { glyph: '▸', tint: 'var(--teal-ink)',       label: 'Worker' },
  worktree: { glyph: '⌥', tint: 'var(--teal-ink)',       label: 'Worktree' },
  branch:   { glyph: '⎇', tint: 'var(--slate-ink)',      label: 'Branch' },
  pr:       { glyph: '⇡', tint: 'var(--review-ink)',     label: 'Pull Request' },
  issue:    { glyph: '#', tint: 'var(--slate-ink)',      label: 'GitHub Issue' },
  ticket:   { glyph: '◇', tint: 'var(--domain-linear)',  label: 'Linear Ticket' },
  plantask: { glyph: '◇', tint: 'var(--accent-ink)',     label: 'Plan Task' },
  approval: { glyph: '⊘', tint: 'var(--caution-ink)',    label: 'Approval' },
  human:    { glyph: '◆', tint: 'var(--attention-ink)',  label: 'Human input' },
  brain:    { glyph: '✻', tint: 'var(--brain-ink)',      label: 'Project Brain' },
};

// status -> ring color
const RING = {
  active: 'var(--accent-solid)', running: 'var(--live-solid)', 'waiting-human': 'var(--attention-solid)',
  'waiting-perm': 'var(--caution-solid)', failed: 'var(--danger-solid)', blocked: 'var(--danger-solid)',
  conflict: 'var(--danger-solid)', stale: 'var(--warning-solid)', degraded: 'var(--warning-solid)',
  completed: 'var(--success-solid)', idle: 'var(--border-strong)', 'pr-open': 'var(--review-solid)',
};

export function GraphNode({
  kind = 'session', title = 'Node', subtitle, status, owner, meta = [],
  selected = false, beacon = false, onClick, style = {},
}) {
  const k = KIND[kind] || KIND.session;
  const ring = status ? (RING[status] || 'var(--border-strong)') : 'var(--border-strong)';
  return (
    <div
      onClick={onClick}
      role="button"
      aria-pressed={selected}
      style={{
        position: 'relative', display: 'flex', flexDirection: 'column', gap: 5,
        width: 196, padding: '9px 11px', cursor: 'pointer',
        background: 'var(--graph-node-bg)', borderRadius: 'var(--r-3)',
        border: '1px solid ' + (selected ? 'var(--graph-selected)' : 'var(--graph-node-line)'),
        boxShadow: selected ? 'var(--graph-selected-glow)' : 'var(--elev-1)',
        outline: selected ? '1px solid var(--accent-line)' : 'none',
        transition: 'border-color var(--dur-1), box-shadow var(--dur-2)',
        ...style,
      }}
      onMouseEnter={(e) => { if (!selected) e.currentTarget.style.borderColor = 'var(--graph-node-line-strong)'; }}
      onMouseLeave={(e) => { if (!selected) e.currentTarget.style.borderColor = 'var(--graph-node-line)'; }}
    >
      {/* status ring dot, top-right */}
      {status && (
        <span aria-label={status} title={status} style={{
          position: 'absolute', top: 9, right: 9, width: 9, height: 9, borderRadius: '999px',
          background: ring, boxShadow: beacon ? 'var(--attention-glow)' : 'none',
          animation: (status === 'running' && 'cp-live-pulse 1.6s var(--ease-inout) infinite') || (beacon && 'cp-attention-beacon 2.2s var(--ease-out) infinite') || 'none',
        }} />
      )}
      <div style={{ display: 'flex', alignItems: 'center', gap: 7, paddingRight: 14 }}>
        <span aria-hidden style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: k.tint, lineHeight: 1, flex: 'none' }}>{k.glyph}</span>
        <span style={{ display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <span style={{ font: 'var(--fw-semibold) 9px/1 var(--font-sans)', letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-faint)' }}>{k.label}</span>
          <span style={{ font: 'var(--fw-medium) var(--fs-label)/1.25 var(--font-sans)', color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{title}</span>
        </span>
      </div>
      {subtitle && <div style={{ font: 'var(--fw-regular) var(--fs-meta)/1.3 var(--font-mono)', color: 'var(--text-muted)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{subtitle}</div>}
      {(owner || meta.length > 0) && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 5, flexWrap: 'wrap' }}>
          {owner && <span style={{ font: 'var(--fw-medium) 10px/1 var(--font-sans)', color: 'var(--text-faint)' }}>{owner}</span>}
          {meta.map((m, i) => (
            <span key={i} style={{ padding: '1px 5px', borderRadius: 'var(--r-1)', border: '1px solid var(--border-subtle)', background: 'var(--surface-input)', font: '10px/1.4 var(--font-mono)', color: 'var(--text-muted)' }}>{m}</span>
          ))}
        </div>
      )}
    </div>
  );
}

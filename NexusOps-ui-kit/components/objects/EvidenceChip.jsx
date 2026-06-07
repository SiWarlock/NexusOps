// EvidenceChip — a Project Brain evidence reference. Grounds every Brain
// answer/action in a real object the user can open. Violet identity.
import React from 'react';

const KIND = {
  file:     { glyph: '⌗', label: 'file' },
  anchor:   { glyph: '⚓', label: 'anchor' },
  plantask: { glyph: '◇', label: 'plan' },
  session:  { glyph: '▷', label: 'session' },
  commit:   { glyph: '⎇', label: 'commit' },
  pr:       { glyph: '⇡', label: 'PR' },
  decision: { glyph: '§', label: 'decision' },
  ticket:   { glyph: '#', label: 'ticket' },
  event:    { glyph: '◴', label: 'event' },
  memory:   { glyph: '✻', label: 'memory' },
};

export function EvidenceChip({ kind = 'file', label, sub, freshness, onClick, style = {}, ...rest }) {
  const k = KIND[kind] || KIND.file;
  const stale = freshness === 'stale';
  return (
    <button
      type="button"
      onClick={onClick}
      title={sub ? `${label} — ${sub}` : label}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 6, maxWidth: '100%',
        height: 'var(--ctl-sm)', padding: '0 8px 0 6px', borderRadius: 'var(--r-1)',
        border: '1px solid var(--brain-line)', background: 'var(--brain-surface)',
        color: 'var(--brain-ink)', cursor: 'pointer', textAlign: 'left',
        font: 'var(--fw-regular) var(--fs-meta)/1 var(--font-sans)', whiteSpace: 'nowrap',
        transition: 'background var(--dur-1) var(--ease-standard)',
        ...style,
      }}
      onMouseEnter={(e) => { e.currentTarget.style.background = 'var(--brain-surface-2)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.background = 'var(--brain-surface)'; }}
      {...rest}
    >
      <span aria-hidden style={{ fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1, color: 'var(--brain-bright)' }}>{k.glyph}</span>
      <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis' }}>{label}</span>
      {sub && <span style={{ color: 'var(--text-faint)', overflow: 'hidden', textOverflow: 'ellipsis' }}>{sub}</span>}
      {stale && <span aria-label="stale" title="stale" style={{ width: 6, height: 6, borderRadius: '999px', background: 'var(--warning-solid)', flex: 'none' }} />}
    </button>
  );
}

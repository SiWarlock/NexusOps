// HarnessBadge — identifies the coding harness behind a session.
// Near-neutral by design (no official brand color implied); a faint
// warm/cool tint plus a glyph differentiates Claude vs Codex.
import React from 'react';

const HARNESS = {
  'claude-code': { label: 'Claude Code', tint: 'var(--domain-claude)', surf: 'var(--domain-claude-surface)', glyph: '✻' },
  'codex-cli':   { label: 'Codex CLI',   tint: 'var(--domain-codex)',  surf: 'var(--domain-codex-surface)',  glyph: '⌁' },
  'codex-cloud': { label: 'Codex Cloud', tint: 'var(--domain-codex)',  surf: 'var(--domain-codex-surface)',  glyph: '☁' },
  shell:         { label: 'Custom Shell', tint: 'var(--harness-shell)', surf: 'var(--neutral-surface)',      glyph: '$' },
};

export function HarnessBadge({ harness = 'claude-code', label, showLabel = true, style = {}, ...rest }) {
  const h = HARNESS[harness] || HARNESS['claude-code'];
  return (
    <span
      title={h.label}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 5,
        height: 'var(--ctl-xs)', padding: showLabel ? '0 7px 0 6px' : '0 5px',
        borderRadius: 'var(--r-1)', border: '1px solid var(--border-default)',
        background: h.surf, color: 'var(--text-secondary)',
        font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-sans)', whiteSpace: 'nowrap',
        ...style,
      }}
      {...rest}
    >
      <span aria-hidden style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: h.tint, lineHeight: 1 }}>{h.glyph}</span>
      {showLabel && (label || h.label)}
    </span>
  );
}

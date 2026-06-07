// MetaChip — generic metadata chip (icon + monospace value). Used for
// branches, worktrees, models, paths, SHAs, ticket IDs, counts.
import React from 'react';

const TONES = {
  default:  { ink: 'var(--text-secondary)', line: 'var(--border-default)', surf: 'transparent' },
  branch:   { ink: 'var(--text-secondary)', line: 'var(--border-default)', surf: 'var(--surface-input)' },
  worktree: { ink: 'var(--teal-ink)',       line: 'var(--teal-line)',      surf: 'var(--teal-surface)' },
  pr:       { ink: 'var(--review-ink)',     line: 'var(--review-line)',    surf: 'var(--review-surface)' },
  linear:   { ink: 'var(--domain-linear)',  line: 'var(--domain-linear-surface)', surf: 'var(--domain-linear-surface)' },
  github:   { ink: 'var(--slate-ink)',      line: 'var(--border-default)', surf: 'var(--slate-surface)' },
  brain:    { ink: 'var(--brain-ink)',      line: 'var(--brain-line)',     surf: 'var(--brain-surface)' },
  accent:   { ink: 'var(--accent-ink)',     line: 'var(--accent-line)',    surf: 'var(--accent-surface)' },
};

export function MetaChip({ children, icon = null, tone = 'default', mono = true, title, style = {}, ...rest }) {
  const t = TONES[tone] || TONES.default;
  return (
    <span
      title={title}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 4, maxWidth: '100%',
        height: 'var(--ctl-xs)', padding: '0 6px', borderRadius: 'var(--r-1)',
        border: '1px solid ' + t.line, background: t.surf, color: t.ink,
        font: `var(--fw-${mono ? 'regular' : 'medium'}) var(--fs-meta)/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
        whiteSpace: 'nowrap', overflow: 'hidden', ...style,
      }}
      {...rest}
    >
      {icon && <span aria-hidden style={{ display: 'inline-flex', width: 12, height: 12, flex: 'none', opacity: 0.9 }}>{icon}</span>}
      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{children}</span>
    </span>
  );
}

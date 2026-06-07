// ProfileBadge — Execution Profile (account/runtime context) a session uses.
// Makes account routing explicit and auditable.
import React from 'react';

const HEALTH = {
  active:     { dot: 'var(--success-solid)', label: 'active' },
  available:  { dot: 'var(--slate-solid)',   label: 'available' },
  'rate-limited': { dot: 'var(--warning-solid)', label: 'rate limited' },
  'auth-expired': { dot: 'var(--danger-solid)',  label: 'auth expired' },
  disabled:   { dot: 'var(--ink-4)',         label: 'disabled' },
};

export function ProfileBadge({ name = 'Claude Max Main', provider = 'claude', health, size = 'sm', style = {}, ...rest }) {
  const h = health ? (HEALTH[health] || HEALTH.available) : null;
  const small = size === 'sm';
  return (
    <span
      title={h ? `${name} · ${h.label}` : name}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 6,
        height: small ? 'var(--ctl-xs)' : 'var(--ctl-sm)', padding: '0 8px',
        borderRadius: 'var(--r-1)', border: '1px solid var(--border-default)',
        background: 'var(--surface-input)', color: 'var(--text-secondary)',
        font: `var(--fw-medium) ${small ? 'var(--fs-meta)' : 'var(--fs-label)'}/1 var(--font-sans)`,
        whiteSpace: 'nowrap', ...style,
      }}
      {...rest}
    >
      <span aria-hidden style={{
        fontFamily: 'var(--font-mono)', fontSize: 10, color: 'var(--text-faint)',
        borderRight: '1px solid var(--border-subtle)', paddingRight: 6, lineHeight: 1.4, textTransform: 'uppercase',
      }}>{provider === 'codex' ? 'CDX' : 'CLD'}</span>
      <span style={{ color: 'var(--text-primary)' }}>{name}</span>
      {h && <span aria-hidden style={{ width: 6, height: 6, borderRadius: '999px', background: h.dot, flex: 'none' }} />}
    </span>
  );
}

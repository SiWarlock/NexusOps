// IconButton — square icon-only control for dense toolbars and rows.
import React from 'react';

const SIZES = { sm: 22, md: 26, lg: 30 };

export function IconButton({
  children,
  label,
  size = 'md',
  variant = 'ghost',
  active = false,
  disabled = false,
  badge = null,
  onClick,
  style = {},
  ...rest
}) {
  const dim = SIZES[size] || SIZES.md;
  const base = {
    ghost:   { background: active ? 'var(--surface-active)' : 'transparent', color: active ? 'var(--accent-ink)' : 'var(--text-secondary)', border: '1px solid ' + (active ? 'var(--accent-line)' : 'transparent') },
    solid:   { background: 'var(--surface-input)', color: 'var(--text-primary)', border: '1px solid var(--border-default)' },
    danger:  { background: 'transparent', color: 'var(--danger-ink)', border: '1px solid transparent' },
  }[variant] || {};

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      style={{
        position: 'relative',
        display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
        width: dim, height: dim, flex: 'none',
        borderRadius: 'var(--r-2)',
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.4 : 1,
        transition: 'background var(--dur-1) var(--ease-standard), color var(--dur-1), border-color var(--dur-1)',
        ...base, ...style,
      }}
      onMouseEnter={(e) => { if (!disabled && !active && variant === 'ghost') { e.currentTarget.style.background = 'var(--surface-hover)'; e.currentTarget.style.color = 'var(--text-primary)'; } }}
      onMouseLeave={(e) => { if (!active && variant === 'ghost') { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.color = 'var(--text-secondary)'; } }}
      {...rest}
    >
      <span style={{ display: 'inline-flex', width: dim <= 22 ? 14 : 16, height: dim <= 22 ? 14 : 16 }}>{children}</span>
      {badge != null && (
        <span style={{
          position: 'absolute', top: -4, right: -4, minWidth: 14, height: 14, padding: '0 3px',
          display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
          borderRadius: '999px', background: 'var(--attention-solid)', color: 'var(--attention-on-solid)',
          font: 'var(--fw-semibold) 9px/1 var(--font-mono)', border: '1.5px solid var(--surface-panel)',
        }}>{badge}</span>
      )}
    </button>
  );
}

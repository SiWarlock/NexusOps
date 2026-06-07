// Button — primary action control for the control plane.
// Self-contained: React only, styling via CSS custom properties.
import React from 'react';

const SIZES = {
  sm: { height: 'var(--ctl-sm)', padding: '0 10px', font: 'var(--fs-label)', radius: 'var(--r-2)', gap: '6px' },
  md: { height: 'var(--ctl-md)', padding: '0 12px', font: 'var(--fs-body)', radius: 'var(--r-2)', gap: '7px' },
  lg: { height: 'var(--ctl-lg)', padding: '0 16px', font: 'var(--fs-body-lg)', radius: 'var(--r-2)', gap: '8px' },
};

const VARIANTS = {
  primary:   { background: 'var(--accent-solid)', color: 'var(--accent-on-solid)', border: '1px solid transparent' },
  secondary: { background: 'var(--surface-input)', color: 'var(--text-primary)', border: '1px solid var(--border-default)' },
  ghost:     { background: 'transparent', color: 'var(--text-secondary)', border: '1px solid transparent' },
  outline:   { background: 'transparent', color: 'var(--text-primary)', border: '1px solid var(--border-strong)' },
  danger:    { background: 'var(--danger-solid)', color: 'var(--danger-on-solid)', border: '1px solid transparent' },
  brain:     { background: 'var(--brain-surface)', color: 'var(--brain-ink)', border: '1px solid var(--brain-line)' },
};

export function Button({
  children,
  variant = 'secondary',
  size = 'md',
  icon = null,
  iconRight = null,
  disabled = false,
  loading = false,
  full = false,
  kbd = null,
  onClick,
  style = {},
  ...rest
}) {
  const s = SIZES[size] || SIZES.md;
  const v = VARIANTS[variant] || VARIANTS.secondary;
  const isDisabled = disabled || loading;

  return (
    <button
      type="button"
      disabled={isDisabled}
      onClick={onClick}
      style={{
        display: full ? 'flex' : 'inline-flex',
        width: full ? '100%' : 'auto',
        alignItems: 'center',
        justifyContent: 'center',
        gap: s.gap,
        height: s.height,
        padding: s.padding,
        borderRadius: s.radius,
        font: `var(--fw-medium) ${s.font}/1 var(--font-sans)`,
        letterSpacing: '0.005em',
        cursor: isDisabled ? 'not-allowed' : 'pointer',
        opacity: isDisabled ? 0.45 : 1,
        whiteSpace: 'nowrap',
        userSelect: 'none',
        transition: 'background var(--dur-1) var(--ease-standard), border-color var(--dur-1), filter var(--dur-1), transform var(--dur-1)',
        ...v,
        ...style,
      }}
      onMouseDown={(e) => { if (!isDisabled) e.currentTarget.style.transform = 'scale(var(--press-scale))'; }}
      onMouseUp={(e) => { e.currentTarget.style.transform = 'scale(1)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.transform = 'scale(1)'; }}
      {...rest}
    >
      {loading
        ? <span aria-hidden style={{ width: 12, height: 12, borderRadius: '999px', border: '1.5px solid currentColor', borderTopColor: 'transparent', animation: 'cp-live-pulse 1s linear infinite', opacity: 0.8 }} />
        : icon}
      {children}
      {iconRight}
      {kbd && (
        <kbd style={{
          marginLeft: 4, padding: '1px 5px', borderRadius: 'var(--r-1)',
          background: 'oklch(1 0 0 / 0.10)', border: '1px solid oklch(1 0 0 / 0.12)',
          font: 'var(--fw-medium) 10px/1 var(--font-mono)', opacity: 0.85,
        }}>{kbd}</kbd>
      )}
    </button>
  );
}

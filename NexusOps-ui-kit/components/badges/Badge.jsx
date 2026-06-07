import React from 'react';

/**
 * Badge — small non-interactive label for counts, metadata, harness,
 * risk levels, and domain tags. Quieter than StatusPill (no glyph by default).
 */

const TONES = {
  neutral:   ['--text-secondary', '--neutral-surface', '--border-default'],
  accent:    ['--accent-ink', '--accent-surface', '--accent-line'],
  brain:     ['--brain-ink', '--brain-surface', '--brain-line'],
  teal:      ['--teal-ink', '--teal-surface', '--teal-line'],
  success:   ['--success-ink', '--success-surface', '--success-line'],
  attention: ['--attention-ink', '--attention-surface', '--attention-line'],
  caution:   ['--caution-ink', '--caution-surface', '--caution-line'],
  warning:   ['--warning-ink', '--warning-surface', '--warning-line'],
  danger:    ['--danger-ink', '--danger-surface', '--danger-line'],
  review:    ['--review-ink', '--review-surface', '--review-line'],
  slate:     ['--slate-ink', '--slate-surface', '--border-default'],
};

export function Badge({
  children,
  tone = 'neutral',
  variant = 'soft', // soft | solid | outline | dot
  mono = false,
  icon = null,
  size = 'sm',
  style = {},
  ...rest
}) {
  const [ink, surface, line] = TONES[tone] || TONES.neutral;
  const fs = { xs: '10px', sm: '11px', md: '12px' };
  const h = { xs: '16px', sm: '18px', md: 'var(--ctl-xs)' };

  let bg, fg, bd;
  if (variant === 'solid') {
    bg = `var(${ink.replace('-ink', '-solid')}, var(${ink}))`;
    fg = `var(${ink.replace('-ink', '-on-solid')}, var(--ink-on-solid))`;
    bd = 'transparent';
  } else if (variant === 'outline') {
    bg = 'transparent'; fg = `var(${ink})`; bd = `var(${line})`;
  } else {
    bg = `var(${surface})`; fg = `var(${ink})`; bd = 'transparent';
  }

  return (
    <span
      style={{
        display: 'inline-flex', alignItems: 'center', gap: '4px',
        height: h[size], padding: variant === 'dot' ? '0' : '0 6px',
        borderRadius: 'var(--r-1)', border: `1px solid ${variant === 'dot' ? 'transparent' : bd}`,
        background: variant === 'dot' ? 'transparent' : bg, color: fg,
        fontFamily: mono ? 'var(--font-mono)' : 'var(--font-sans)',
        fontSize: fs[size], fontWeight: 'var(--fw-medium)',
        letterSpacing: mono ? '0' : '0.01em', lineHeight: 1,
        whiteSpace: 'nowrap', flex: 'none', ...style,
      }}
      {...rest}
    >
      {variant === 'dot' && (
        <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: `var(${ink})`, flex: 'none' }} />
      )}
      {icon && <span style={{ display: 'inline-flex', flex: 'none' }}>{icon}</span>}
      {children}
    </span>
  );
}

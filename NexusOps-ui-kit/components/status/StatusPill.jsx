// StatusPill — the canonical status marker. Encodes state on FOUR
// channels: color family, glyph, text label, and (optional) motion.
import React from 'react';

// status -> { ink, surface, line, solid, onSolid, glyph, label, beacon }
export const STATUS = {
  active:        { ink: 'var(--accent-ink)',    surface: 'var(--accent-surface)',    line: 'var(--accent-line)',    solid: 'var(--accent-solid)',    onSolid: 'var(--accent-on-solid)',    glyph: '●', label: 'Active' },
  running:       { ink: 'var(--live-ink)',      surface: 'var(--live-surface)',      line: 'var(--live-line)',      solid: 'var(--live-solid)',      onSolid: 'var(--live-on-solid)',      glyph: '▶', label: 'Running', pulse: true },
  editing:       { ink: 'var(--accent-ink)',    surface: 'var(--accent-surface)',    line: 'var(--accent-line)',    solid: 'var(--accent-solid)',    onSolid: 'var(--accent-on-solid)',    glyph: '✎', label: 'Editing', pulse: true },
  testing:       { ink: 'var(--live-ink)',      surface: 'var(--live-surface)',      line: 'var(--live-line)',      solid: 'var(--live-solid)',      onSolid: 'var(--live-on-solid)',      glyph: '◑', label: 'Running tests', pulse: true },
  idle:          { ink: 'var(--slate-ink)',     surface: 'var(--slate-surface)',     line: 'var(--border-default)', solid: 'var(--slate-solid)',     onSolid: 'var(--slate-on-solid)',     glyph: '○', label: 'Idle' },
  'waiting-human': { ink: 'var(--attention-ink)', surface: 'var(--attention-surface)', line: 'var(--attention-line)', solid: 'var(--attention-solid)', onSolid: 'var(--attention-on-solid)', glyph: '◆', label: 'Waiting · human', beacon: true },
  'waiting-perm':  { ink: 'var(--caution-ink)',   surface: 'var(--caution-surface)',   line: 'var(--caution-line)',  solid: 'var(--caution-solid)',  onSolid: 'var(--caution-on-solid)',  glyph: '⊘', label: 'Waiting · permission' },
  approval:      { ink: 'var(--attention-ink)',  surface: 'var(--attention-surface)', line: 'var(--attention-line)', solid: 'var(--attention-solid)', onSolid: 'var(--attention-on-solid)', glyph: '◆', label: 'Approval required', beacon: true },
  failed:        { ink: 'var(--danger-ink)',     surface: 'var(--danger-surface)',    line: 'var(--danger-line)',    solid: 'var(--danger-solid)',    onSolid: 'var(--danger-on-solid)',    glyph: '✕', label: 'Failed' },
  blocked:       { ink: 'var(--danger-ink)',     surface: 'var(--danger-surface)',    line: 'var(--danger-line)',    solid: 'var(--danger-solid)',    onSolid: 'var(--danger-on-solid)',    glyph: '■', label: 'Blocked' },
  conflict:      { ink: 'var(--danger-ink)',     surface: 'var(--danger-surface)',    line: 'var(--danger-line)',    solid: 'var(--danger-solid)',    onSolid: 'var(--danger-on-solid)',    glyph: '⨯', label: 'Conflict' },
  stale:         { ink: 'var(--warning-ink)',    surface: 'var(--warning-surface)',   line: 'var(--warning-line)',   solid: 'var(--warning-solid)',   onSolid: 'var(--warning-on-solid)',   glyph: '◌', label: 'Stale' },
  degraded:      { ink: 'var(--warning-ink)',    surface: 'var(--warning-surface)',   line: 'var(--warning-line)',   solid: 'var(--warning-solid)',   onSolid: 'var(--warning-on-solid)',   glyph: '△', label: 'Degraded' },
  completed:     { ink: 'var(--success-ink)',    surface: 'var(--success-surface)',   line: 'var(--success-line)',   solid: 'var(--success-solid)',   onSolid: 'var(--success-on-solid)',   glyph: '✓', label: 'Completed' },
  approved:      { ink: 'var(--success-ink)',    surface: 'var(--success-surface)',   line: 'var(--success-line)',   solid: 'var(--success-solid)',   onSolid: 'var(--success-on-solid)',   glyph: '✓', label: 'Approved' },
  passing:       { ink: 'var(--success-ink)',    surface: 'var(--success-surface)',   line: 'var(--success-line)',   solid: 'var(--success-solid)',   onSolid: 'var(--success-on-solid)',   glyph: '✓', label: 'Checks passing' },
  archived:      { ink: 'var(--ink-4)',          surface: 'var(--slate-surface)',     line: 'var(--border-subtle)',  solid: 'var(--slate-solid)',     onSolid: 'var(--slate-on-solid)',     glyph: '—', label: 'Archived' },
  'pr-open':     { ink: 'var(--review-ink)',     surface: 'var(--review-surface)',    line: 'var(--review-line)',    solid: 'var(--review-solid)',    onSolid: 'var(--review-on-solid)',    glyph: '⇡', label: 'PR open' },
  merged:        { ink: 'var(--review-ink)',     surface: 'var(--review-surface)',    line: 'var(--review-line)',    solid: 'var(--review-solid)',    onSolid: 'var(--review-on-solid)',    glyph: '⇊', label: 'Merged' },
  critical:      { ink: 'var(--critical-ink)',   surface: 'var(--critical-surface)',  line: 'var(--critical-line)',  solid: 'var(--critical-solid)',  onSolid: 'var(--critical-on-solid)',  glyph: '!', label: 'Critical' },
};

const SIZES = {
  xs: { h: 'var(--ctl-xs)', pad: '0 6px', font: 'var(--fs-micro)', glyph: 8 },
  sm: { h: 'var(--ctl-xs)', pad: '0 7px', font: 'var(--fs-meta)', glyph: 9 },
  md: { h: 'var(--ctl-sm)', pad: '0 9px', font: 'var(--fs-label)', glyph: 10 },
};

export function StatusPill({ status = 'idle', label, emphasis = 'soft', size = 'sm', beacon, style = {}, ...rest }) {
  const s = STATUS[status] || STATUS.idle;
  const sz = SIZES[size] || SIZES.sm;
  const solid = emphasis === 'solid';
  const showBeacon = beacon != null ? beacon : s.beacon;

  return (
    <span
      role="status"
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 5,
        height: sz.h, padding: sz.pad, borderRadius: 'var(--r-1)',
        font: `var(--fw-medium) ${sz.font}/1 var(--font-sans)`,
        letterSpacing: 'var(--tracking-wide)', whiteSpace: 'nowrap',
        background: solid ? s.solid : s.surface,
        color: solid ? s.onSolid : s.ink,
        border: '1px solid ' + (solid ? 'transparent' : s.line),
        boxShadow: showBeacon && solid ? 'var(--attention-glow)' : 'none',
        ...style,
      }}
      {...rest}
    >
      <span aria-hidden style={{
        fontFamily: 'var(--font-mono)', fontSize: sz.glyph, lineHeight: 1,
        animation: (s.pulse || (showBeacon && !solid)) ? 'cp-live-pulse 1.6s var(--ease-inout) infinite' : 'none',
      }}>{s.glyph}</span>
      {label || s.label}
    </span>
  );
}

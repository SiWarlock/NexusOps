// RiskBadge — Action Gateway risk classification. Text label is mandatory;
// critical adds a hazard hatch so it reads in grayscale.
import React from 'react';

const RISK = {
  readonly: { ink: 'var(--risk-readonly)', line: 'var(--border-default)',   label: 'Read-only' },
  low:      { ink: 'var(--risk-low)',      line: 'var(--success-line)',     label: 'Low' },
  medium:   { ink: 'var(--risk-medium)',   line: 'var(--caution-line)',     label: 'Medium' },
  high:     { ink: 'var(--risk-high)',     line: 'oklch(0.70 0.18 38 / 0.45)', label: 'High' },
  critical: { ink: 'var(--risk-critical)', line: 'var(--critical-line)',    label: 'Critical' },
};

export function RiskBadge({ level = 'low', label, showDot = true, style = {}, ...rest }) {
  const r = RISK[level] || RISK.low;
  const isCritical = level === 'critical';
  return (
    <span
      className={isCritical ? 'cp-hazard' : undefined}
      title={`Risk: ${label || r.label}`}
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 5,
        height: 'var(--ctl-xs)', padding: '0 7px', borderRadius: 'var(--r-1)',
        font: 'var(--fw-semibold) var(--fs-micro)/1 var(--font-sans)',
        letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase',
        color: r.ink, border: '1px solid ' + r.line,
        background: isCritical ? undefined : 'transparent',
        ...style,
      }}
      {...rest}
    >
      {showDot && <span aria-hidden style={{ width: 6, height: 6, borderRadius: '2px', background: r.ink, flex: 'none' }} />}
      {label || r.label}
    </span>
  );
}

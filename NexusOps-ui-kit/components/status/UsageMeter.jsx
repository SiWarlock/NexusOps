// UsageMeter — context / token / cost capacity meter. Bar or ring.
// Fill color escalates by threshold (normal -> warn -> risk -> stop).
import React from 'react';

function stopColor(pct) {
  if (pct >= 0.92) return 'var(--cap-stop)';
  if (pct >= 0.80) return 'var(--cap-risk)';
  if (pct >= 0.65) return 'var(--cap-warn)';
  return 'var(--cap-normal)';
}

export function UsageMeter({
  value = 0,
  max = 100,
  variant = 'bar',
  label,
  valueText,
  accuracy,          // 'exact' | 'estimated' | 'unavailable'
  size = 'md',
  style = {},
}) {
  const pct = max > 0 ? Math.min(1, Math.max(0, value / max)) : 0;
  const color = accuracy === 'unavailable' ? 'var(--slate)' : stopColor(pct);
  const acc = accuracy && accuracy !== 'exact'
    ? <span style={{ fontSize: 9, color: 'var(--text-faint)', fontFamily: 'var(--font-mono)' }}>{accuracy === 'estimated' ? '≈' : 'n/a'}</span>
    : null;

  if (variant === 'ring') {
    const d = size === 'sm' ? 26 : 34;
    const sw = size === 'sm' ? 3 : 4;
    const r = (d - sw) / 2;
    const c = 2 * Math.PI * r;
    return (
      <span title={label} style={{ display: 'inline-flex', alignItems: 'center', gap: 7, ...style }}>
        <svg width={d} height={d} style={{ transform: 'rotate(-90deg)', flex: 'none' }}>
          <circle cx={d/2} cy={d/2} r={r} fill="none" stroke="var(--cap-track)" strokeWidth={sw} />
          <circle cx={d/2} cy={d/2} r={r} fill="none" stroke={color} strokeWidth={sw}
            strokeDasharray={c} strokeDashoffset={c * (1 - pct)} strokeLinecap="round" />
        </svg>
        {(valueText || label) && (
          <span style={{ display: 'inline-flex', flexDirection: 'column', lineHeight: 1.25 }}>
            <span style={{ font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-mono)', color }}>{valueText || `${Math.round(pct*100)}%`}</span>
            {label && <span style={{ fontSize: 10, color: 'var(--text-faint)' }}>{label}</span>}
          </span>
        )}
      </span>
    );
  }

  const h = size === 'sm' ? 4 : 6;
  return (
    <span style={{ display: 'inline-flex', flexDirection: 'column', gap: 3, minWidth: 96, ...style }}>
      {(label || valueText) && (
        <span style={{ display: 'flex', alignItems: 'center', gap: 5, font: 'var(--fw-regular) var(--fs-meta)/1 var(--font-sans)', color: 'var(--text-muted)' }}>
          {label}
          {(valueText || acc) && <span style={{ marginLeft: 'auto', display: 'inline-flex', gap: 4, alignItems: 'center', fontFamily: 'var(--font-mono)', color: pct >= 0.8 ? color : 'var(--text-secondary)' }}>{acc}{valueText}</span>}
        </span>
      )}
      <span style={{ display: 'block', height: h, borderRadius: '999px', background: 'var(--cap-track)', overflow: 'hidden' }}>
        <span style={{ display: 'block', height: '100%', width: `${pct*100}%`, background: color, borderRadius: '999px', transition: 'width var(--dur-3) var(--ease-out)' }} />
      </span>
    </span>
  );
}

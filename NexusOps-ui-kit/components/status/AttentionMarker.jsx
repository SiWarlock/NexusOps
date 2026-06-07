// AttentionMarker — the left rail / dot that encodes a row's attention level.
// Drives the visual weight of sidebar rows, queue items, and graph nodes.
import React from 'react';

const LEVELS = {
  5: { color: 'var(--attention-solid)', beacon: true,  label: 'Waiting on human' },
  4: { color: 'var(--danger-solid)',    beacon: false, label: 'Failed / blocked' },
  3: { color: 'var(--warning-solid)',   beacon: false, label: 'Degraded / capacity' },
  2: { color: 'var(--live-solid)',      pulse: true,   label: 'Running' },
  1: { color: 'var(--accent-solid)',    beacon: false, label: 'Active' },
  0: { color: 'var(--slate-solid)',     beacon: false, label: 'Idle' },
};

export function AttentionMarker({ level = 0, variant = 'rail', style = {}, ...rest }) {
  const l = LEVELS[level] || LEVELS[0];
  const animate = l.beacon ? 'cp-attention-beacon 2.2s var(--ease-out) infinite'
                 : l.pulse ? 'cp-live-pulse 1.6s var(--ease-inout) infinite' : 'none';

  if (variant === 'dot') {
    return <span role="img" aria-label={l.label} title={l.label}
      style={{ width: 8, height: 8, borderRadius: '999px', background: l.color, flex: 'none', animation: animate, ...style }} {...rest} />;
  }
  // rail: a vertical bar meant to sit at the leading edge of a row/card
  return <span aria-label={l.label} title={l.label}
    style={{ width: level >= 4 ? 3 : level >= 1 ? 2 : 0, alignSelf: 'stretch', borderRadius: '0 2px 2px 0',
      background: level === 0 ? 'transparent' : l.color, flex: 'none', animation: l.pulse ? animate : 'none', ...style }} {...rest} />;
}

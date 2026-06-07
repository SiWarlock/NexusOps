import * as React from 'react';

/**
 * Encodes a row/card/node's attention level (0–5) as a leading rail or a dot.
 * Higher levels are louder and animate (level 5 beacons, level 2 pulses),
 * which is how attention-first sorting becomes visible at a glance.
 */
export interface AttentionMarkerProps {
  /** 5 waiting-human · 4 failed/blocked · 3 degraded · 2 running · 1 active · 0 idle. */
  level?: 0 | 1 | 2 | 3 | 4 | 5;
  variant?: 'rail' | 'dot';
  style?: React.CSSProperties;
}

export function AttentionMarker(props: AttentionMarkerProps): JSX.Element;

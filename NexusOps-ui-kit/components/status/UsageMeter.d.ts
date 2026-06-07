import * as React from 'react';

/**
 * Capacity meter for context window, token, or cost usage. The fill color
 * escalates by threshold (normal → warn → risk → hard-stop), and an accuracy
 * marker (≈ / n/a) reflects that adapters report exact, estimated, or
 * unavailable usage. Available as a horizontal `bar` or compact `ring`.
 */
export interface UsageMeterProps {
  value?: number;
  max?: number;
  variant?: 'bar' | 'ring';
  /** Caption (e.g. "Context"). */
  label?: string;
  /** Explicit value text (e.g. "128k / 200k"); defaults to a percentage. */
  valueText?: string;
  accuracy?: 'exact' | 'estimated' | 'unavailable';
  size?: 'sm' | 'md';
  style?: React.CSSProperties;
}

export function UsageMeter(props: UsageMeterProps): JSX.Element;

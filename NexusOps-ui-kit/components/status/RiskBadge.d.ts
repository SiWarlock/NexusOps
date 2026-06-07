import * as React from 'react';

/**
 * Action Gateway risk classification badge. The text label is always present
 * (never color-only); `critical` additionally renders a hazard hatch so it is
 * legible in grayscale and for color-blind users.
 */
export interface RiskBadgeProps {
  level?: 'readonly' | 'low' | 'medium' | 'high' | 'critical';
  /** Override the label text. */
  label?: string;
  showDot?: boolean;
  style?: React.CSSProperties;
}

export function RiskBadge(props: RiskBadgeProps): JSX.Element;

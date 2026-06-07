import * as React from 'react';

/**
 * Square icon-only control for dense toolbars, terminal headers, sidebar
 * quick-actions, and table rows. Always pass `label` for the accessible name
 * and tooltip. Supports an optional notification `badge` (e.g. waiting count).
 */
export interface IconButtonProps {
  /** Icon node (e.g. a 16px Lucide SVG). */
  children: React.ReactNode;
  /** Required accessible label + tooltip text. */
  label: string;
  size?: 'sm' | 'md' | 'lg';
  variant?: 'ghost' | 'solid' | 'danger';
  /** Toggled/selected state (e.g. active view). */
  active?: boolean;
  disabled?: boolean;
  /** Small corner badge content (number or short string). */
  badge?: React.ReactNode;
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  style?: React.CSSProperties;
}

export function IconButton(props: IconButtonProps): JSX.Element;

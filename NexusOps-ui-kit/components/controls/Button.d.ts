import * as React from 'react';

/**
 * Primary action control. Compact, keyboard-friendly, built for dense
 * desktop toolbars and inspector footers. Styling is driven entirely by
 * design-system CSS custom properties.
 *
 * @startingPoint section="Controls" subtitle="Primary / secondary / danger / brain action button" viewport="360x140"
 */
export interface ButtonProps {
  children?: React.ReactNode;
  /** Visual weight / intent. */
  variant?: 'primary' | 'secondary' | 'ghost' | 'outline' | 'danger' | 'brain';
  size?: 'sm' | 'md' | 'lg';
  /** Leading icon node (e.g. a Lucide SVG). */
  icon?: React.ReactNode;
  /** Trailing icon node. */
  iconRight?: React.ReactNode;
  disabled?: boolean;
  /** Shows a spinner and blocks interaction. */
  loading?: boolean;
  /** Stretch to fill container width. */
  full?: boolean;
  /** Optional keyboard hint rendered as a trailing <kbd>. */
  kbd?: string;
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  style?: React.CSSProperties;
}

export function Button(props: ButtonProps): JSX.Element;

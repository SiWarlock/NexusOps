import React from 'react';

export type BadgeTone =
  | 'neutral' | 'accent' | 'brain' | 'teal' | 'success'
  | 'attention' | 'caution' | 'warning' | 'danger' | 'review' | 'slate';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Color family. Map to meaning: brain=Project Brain, teal=workflow, review=PR. */
  tone?: BadgeTone;
  /** `soft` tinted (default), `solid` filled, `outline`, `dot` (leading dot + text). */
  variant?: 'soft' | 'solid' | 'outline' | 'dot';
  /** Render in Geist Mono — for counts, SHAs, versions, token amounts. */
  mono?: boolean;
  icon?: React.ReactNode;
  size?: 'xs' | 'sm' | 'md';
}

/**
 * Badge — quiet metadata label (counts, harness, risk, domain tags).
 * Use StatusPill instead when communicating live object state.
 */
export function Badge(props: BadgeProps): React.ReactElement;

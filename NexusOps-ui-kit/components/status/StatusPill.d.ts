import * as React from 'react';

/** Canonical status keys understood by StatusPill. */
export type StatusKind =
  | 'active' | 'running' | 'editing' | 'testing' | 'idle'
  | 'waiting-human' | 'waiting-perm' | 'approval'
  | 'failed' | 'blocked' | 'conflict' | 'stale' | 'degraded'
  | 'completed' | 'approved' | 'passing' | 'archived'
  | 'pr-open' | 'merged' | 'critical';

/**
 * The canonical status marker for sessions, tasks, worktrees, PRs, approvals,
 * and graph nodes. Every status is encoded on four channels — color family,
 * glyph, text label, and optional motion — so it never relies on color alone.
 *
 * @startingPoint section="Status" subtitle="Four-channel status pill (color + glyph + label + motion)" viewport="420x120"
 */
export interface StatusPillProps {
  status?: StatusKind;
  /** Override the default label text for this status. */
  label?: string;
  /** `soft` chip (default) or `solid` for the single loudest marker in view. */
  emphasis?: 'soft' | 'solid';
  size?: 'xs' | 'sm' | 'md';
  /** Force the attention beacon on/off (defaults per-status). */
  beacon?: boolean;
  style?: React.CSSProperties;
}

export function StatusPill(props: StatusPillProps): JSX.Element;
/** The status descriptor map (color tokens + glyph + default label). */
export const STATUS: Record<StatusKind, {
  ink: string; surface: string; line: string; solid: string; onSolid: string;
  glyph: string; label: string; pulse?: boolean; beacon?: boolean;
}>;

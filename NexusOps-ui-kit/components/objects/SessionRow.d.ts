import * as React from 'react';
import type { StatusKind } from '../status/StatusPill';

/**
 * The Session is the atomic operational unit of the platform. SessionRow is its
 * dense, selectable representation — composing StatusPill, AttentionMarker,
 * HarnessBadge, ProfileBadge, MetaChip, and a context UsageMeter so every row
 * answers: what is it doing, who owns it, on which account, and against which
 * task / worktree / branch / PR. Sorts by attention level (waiting-human first).
 *
 * @startingPoint section="Sessions" subtitle="Atomic session row with full ownership chain" viewport="560x108"
 */
export interface SessionRowProps {
  title?: string;
  status?: StatusKind;
  harness?: 'claude-code' | 'codex-cli' | 'codex-cloud' | 'shell';
  profile?: string;
  provider?: 'claude' | 'codex';
  /** Linked task/ticket, e.g. { id: 'ENG-221', tone: 'linear' }. */
  task?: { id: string; tone?: 'linear' | 'github' | 'accent' };
  branch?: string;
  worktree?: string;
  pr?: string;
  /** Context-window usage, e.g. { value: 128, max: 200 }. */
  context?: { value: number; max: number; text?: string };
  /** Last-activity timestamp text. */
  activity?: string;
  /** Current command / activity line (mono). */
  current?: string;
  selected?: boolean;
  density?: 'comfortable' | 'compact';
  onClick?: (e: React.MouseEvent<HTMLDivElement>) => void;
  style?: React.CSSProperties;
}

export function SessionRow(props: SessionRowProps): JSX.Element;

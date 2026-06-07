import * as React from 'react';

/**
 * A Project Brain evidence reference. Every Brain answer and proposed action is
 * grounded in openable evidence — a file/line, architecture anchor, plan task,
 * session episode, commit, PR, decision, ticket, event, or memory source. Violet
 * identity ties it to Project Brain; a freshness dot flags stale grounding.
 */
export interface EvidenceChipProps {
  kind?: 'file' | 'anchor' | 'plantask' | 'session' | 'commit' | 'pr' | 'decision' | 'ticket' | 'event' | 'memory';
  /** Primary reference (e.g. "review.ts:42" or "ENG-221"). */
  label: string;
  /** Optional secondary context. */
  sub?: string;
  freshness?: 'fresh' | 'stale';
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  style?: React.CSSProperties;
}

export function EvidenceChip(props: EvidenceChipProps): JSX.Element;

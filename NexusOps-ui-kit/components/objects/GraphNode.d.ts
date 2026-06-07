import * as React from 'react';
import type { StatusKind } from '../status/StatusPill';

/**
 * An operational node for the Project Observability Graph — not decorative.
 * Node chrome (glyph + tint) encodes the object DOMAIN; an overlaid status ring
 * encodes STATE; selection uses the accent + glow. Always shows ownership so the
 * graph reads as a live operations map, not a diagram.
 *
 * @startingPoint section="Graph" subtitle="Operational graph node (type + status + ownership)" viewport="240x140"
 */
export interface GraphNodeProps {
  kind?: 'project' | 'session' | 'team' | 'worker' | 'worktree' | 'branch' | 'pr'
       | 'issue' | 'ticket' | 'plantask' | 'approval' | 'human' | 'brain';
  title?: string;
  subtitle?: string;
  status?: StatusKind;
  /** Owner label (e.g. session/team that owns this node). */
  owner?: string;
  /** Small mono metadata pills (e.g. ["64% ctx", "+7 files"]). */
  meta?: string[];
  selected?: boolean;
  beacon?: boolean;
  onClick?: (e: React.MouseEvent<HTMLDivElement>) => void;
  style?: React.CSSProperties;
}

export function GraphNode(props: GraphNodeProps): JSX.Element;

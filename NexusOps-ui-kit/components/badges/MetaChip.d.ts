import * as React from 'react';

/**
 * Generic metadata chip: an optional icon plus a (usually monospace) value.
 * The workhorse for object ownership — branches, worktree paths, model names,
 * SHAs, ticket IDs, file counts. `tone` tints it to the owning domain.
 */
export interface MetaChipProps {
  children: React.ReactNode;
  icon?: React.ReactNode;
  tone?: 'default' | 'branch' | 'worktree' | 'pr' | 'linear' | 'github' | 'brain' | 'accent';
  /** Monospace value (default true) vs sans label. */
  mono?: boolean;
  title?: string;
  style?: React.CSSProperties;
}

export function MetaChip(props: MetaChipProps): JSX.Element;

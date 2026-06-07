import * as React from 'react';

export interface DiffLine {
  type: 'add' | 'del' | 'ctx';
  text: string;
  /** Line number in the new file (optional). */
  ln?: number | string;
}

/**
 * A reviewable code diff hunk — the platform's code review is first-class, so a
 * hunk carries its own per-hunk action bar (accept / reject / ask why /
 * request fix). Diff treatment uses the git diff tokens; a left ribbon reflects
 * review status. "Ask why" routes to Project Brain; "Request fix" to the agent.
 */
export interface DiffHunkProps {
  file?: string;
  /** Unified hunk header, e.g. "@@ -1,4 +1,5 @@". */
  header?: string;
  lines?: DiffLine[];
  status?: 'accepted' | 'rejected' | 'conflict';
  /** Review comment count (rendered in the header). */
  comments?: number;
  /** Show the per-hunk action bar. */
  actions?: boolean;
  onAccept?: () => void;
  onReject?: () => void;
  onAsk?: () => void;
  onRequestFix?: () => void;
  style?: React.CSSProperties;
}

export function DiffHunk(props: DiffHunkProps): JSX.Element;

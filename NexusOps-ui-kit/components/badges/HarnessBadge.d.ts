import * as React from 'react';

/**
 * Identifies the coding harness behind a session (Claude Code, Codex CLI,
 * Codex Cloud, or a custom shell). Intentionally near-neutral so it never
 * implies an official vendor brand color — a faint warm/cool tint plus a
 * glyph carries the distinction.
 */
export interface HarnessBadgeProps {
  harness?: 'claude-code' | 'codex-cli' | 'codex-cloud' | 'shell';
  label?: string;
  /** Hide text to render a compact glyph-only badge. */
  showLabel?: boolean;
  style?: React.CSSProperties;
}

export function HarnessBadge(props: HarnessBadgeProps): JSX.Element;

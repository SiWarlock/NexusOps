import * as React from 'react';

/**
 * Execution Profile badge — names the local account/runtime context a session
 * runs under (e.g. "Claude Max Main", "Codex Cloud GitHub"). A first-class
 * concept: account routing must be explicit, visible, and auditable; the
 * platform never silently hops accounts. Optional health dot surfaces auth /
 * rate-limit state.
 */
export interface ProfileBadgeProps {
  name?: string;
  provider?: 'claude' | 'codex';
  health?: 'active' | 'available' | 'rate-limited' | 'auth-expired' | 'disabled';
  size?: 'sm' | 'md';
  style?: React.CSSProperties;
}

export function ProfileBadge(props: ProfileBadgeProps): JSX.Element;

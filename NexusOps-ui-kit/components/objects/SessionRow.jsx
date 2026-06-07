// SessionRow — the atomic operational unit as a dense, selectable row.
// Composes the status / badge / meter primitives. Used in the sidebar,
// the Sessions list, and the Command Center.
import React from 'react';
import { StatusPill } from '../status/StatusPill.jsx';
import { AttentionMarker } from '../status/AttentionMarker.jsx';
import { UsageMeter } from '../status/UsageMeter.jsx';
import { HarnessBadge } from '../badges/HarnessBadge.jsx';
import { ProfileBadge } from '../badges/ProfileBadge.jsx';
import { MetaChip } from '../badges/MetaChip.jsx';

const ATTN = { 'waiting-human': 5, approval: 5, failed: 4, blocked: 4, conflict: 4, degraded: 3, stale: 3, running: 2, editing: 2, testing: 2, active: 1, 'pr-open': 1, idle: 0, completed: 0, archived: 0 };

export function SessionRow({
  title = 'Untitled session',
  status = 'idle',
  harness = 'claude-code',
  profile = 'Claude Max Main',
  provider = 'claude',
  task,                 // { id, tone } e.g. { id:'ENG-221', tone:'linear' }
  branch,               // string
  worktree,             // string
  pr,                   // string e.g. '#84'
  context,              // { value, max, text }
  activity = '',        // last activity text
  current = '',         // current command / activity line
  selected = false,
  density = 'comfortable', // 'comfortable' | 'compact'
  onClick,
  style = {},
}) {
  const level = ATTN[status] ?? 0;
  const compact = density === 'compact';

  return (
    <div
      onClick={onClick}
      role="row"
      aria-selected={selected}
      style={{
        display: 'flex', alignItems: 'stretch', gap: 0,
        background: selected ? 'var(--surface-active)' : 'transparent',
        borderRadius: 'var(--r-2)', cursor: 'pointer', overflow: 'hidden',
        boxShadow: selected ? 'inset 0 0 0 1px var(--accent-line)' : 'inset 0 0 0 1px transparent',
        transition: 'background var(--dur-1) var(--ease-standard)',
        ...style,
      }}
      onMouseEnter={(e) => { if (!selected) e.currentTarget.style.background = 'var(--surface-hover)'; }}
      onMouseLeave={(e) => { if (!selected) e.currentTarget.style.background = 'transparent'; }}
    >
      <AttentionMarker level={level} />
      <div style={{ flex: 1, minWidth: 0, padding: compact ? '6px 10px' : '8px 11px', display: 'flex', flexDirection: 'column', gap: compact ? 4 : 6 }}>
        {/* line 1: status + title + context */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
          <StatusPill status={status} size="xs" />
          <span style={{ font: `var(--fw-medium) var(--fs-body)/1.2 var(--font-sans)`, color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', minWidth: 0, flex: 1 }}>{title}</span>
          {context && <UsageMeter variant="ring" size="sm" value={context.value} max={context.max} label="ctx" />}
        </div>
        {/* line 2: ownership chips */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, flexWrap: 'wrap' }}>
          <HarnessBadge harness={harness} />
          <ProfileBadge name={profile} provider={provider} />
          {task && <MetaChip tone={task.tone || 'linear'} mono={false}>{task.id}</MetaChip>}
          {branch && <MetaChip tone="branch">{branch}</MetaChip>}
          {worktree && <MetaChip tone="worktree">{worktree}</MetaChip>}
          {pr && <MetaChip tone="pr">{pr}</MetaChip>}
        </div>
        {/* line 3 (optional): current activity + last activity */}
        {(current || activity) && !compact && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, font: 'var(--fw-regular) var(--fs-meta)/1.3 var(--font-mono)', color: 'var(--text-muted)', minWidth: 0 }}>
            {current && <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', minWidth: 0 }}>{current}</span>}
            {activity && <span style={{ marginLeft: 'auto', flex: 'none', color: 'var(--text-faint)' }}>{activity}</span>}
          </div>
        )}
      </div>
    </div>
  );
}

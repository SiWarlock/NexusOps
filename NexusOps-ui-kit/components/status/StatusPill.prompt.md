The canonical state marker used everywhere an object has status: session rows, task cards, worktrees, PRs, approvals, graph nodes. Encodes state on four channels (color + glyph + label + motion) so it survives grayscale and color-blindness.

```jsx
<StatusPill status="waiting-human" />        {/* loud amber + beacon */}
<StatusPill status="running" />               {/* cyan + pulse */}
<StatusPill status="critical" emphasis="solid" />
<StatusPill status="merged" label="Merged · #84" size="md" />
```

Keys: `active, running, editing, testing, idle, waiting-human, waiting-perm, approval, failed, blocked, conflict, stale, degraded, completed, approved, passing, archived, pr-open, merged, critical`. Use `emphasis="solid"` for the single most urgent marker in a view; everything else stays `soft`. Drives attention-first sort order — `waiting-human` is the loudest by design.

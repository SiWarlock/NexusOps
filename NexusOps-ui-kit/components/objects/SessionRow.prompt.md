The Session is the atomic unit of the platform; SessionRow is its dense, selectable representation. It composes the status, badge, and meter primitives so a single row carries the full ownership chain.

```jsx
<SessionRow
  status="waiting-human"
  title="ENG-221 · GitHub OAuth callback"
  harness="claude-code" profile="Claude Max Main"
  task={{ id: 'ENG-221', tone: 'linear' }}
  branch="agent/eng-221-oauth" worktree="~/wt/eng-221" pr="#84"
  context={{ value: 186, max: 200 }}
  current="$ npm test — awaiting permission" activity="2m ago"
  selected
/>
```

Used in the left sidebar, the Sessions list/board, and the Command Center. Rows sort by attention level (waiting-human → failed → running → idle). Use `density="compact"` in the sidebar, `comfortable` in main lists.

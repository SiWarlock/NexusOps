A reviewable code diff hunk with its own per-hunk action bar — code review is a first-class surface, not an afterthought.

```jsx
<DiffHunk
  file="src/gateway/review.ts" header="@@ -10,3 +10,4 @@"
  lines={[
    { type:'ctx', ln:10, text:'function review(plan) {' },
    { type:'del',         text:'  return run(plan)' },
    { type:'add', ln:11,  text:'  const dry = await dryRun(plan)' },
    { type:'add', ln:12,  text:'  return gateway.confirm(dry)' },
  ]}
  comments={2}
  onAsk={askBrain} onRequestFix={requestAgentFix}
/>
```

`status` shows a review ribbon (`accepted | rejected | conflict`). "Ask why" routes to Project Brain; "Request fix" / "Add tests" route to the active agent. Set `actions={false}` for read-only PR previews.

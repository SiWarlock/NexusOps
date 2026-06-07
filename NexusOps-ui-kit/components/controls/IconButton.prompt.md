Square icon-only control for toolbars, terminal/session headers, and row hover-actions. Always give it a `label` (used as accessible name + tooltip).

```jsx
<IconButton label="Attach terminal"><Terminal/></IconButton>
<IconButton label="Human input queue" badge={3}><Inbox/></IconButton>
<IconButton label="Graph view" active><Workflow/></IconButton>
```

Variants `ghost | solid | danger`; sizes `sm | md | lg`. Use `active` for toggled view state and `badge` for a waiting/notification count.

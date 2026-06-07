Primary action control for toolbars, inspector footers, and modals — compact and keyboard-friendly.

```jsx
<Button variant="primary" size="md" kbd="⌘↵" onClick={dispatch}>Start session</Button>
<Button variant="danger" icon={<Trash2/>}>Delete worktree</Button>
<Button variant="brain">Ask Project Brain</Button>
```

Variants: `primary` (azure solid), `secondary` (default), `ghost`, `outline`, `danger`, `brain` (violet, for Project Brain affordances). Sizes `sm | md | lg`. Supports `icon`, `iconRight`, `loading`, `full`, and a trailing `kbd` hint. Reserve `primary` for the single most important action in a surface; reserve `danger` for destructive/high-risk actions that route through the Action Gateway.

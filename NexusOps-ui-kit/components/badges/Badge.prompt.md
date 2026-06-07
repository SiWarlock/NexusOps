**Badge** — quiet, non-interactive label for counts, metadata, harness names, risk levels, domain tags; use StatusPill for live state instead.

```jsx
<Badge tone="brain" variant="dot">Project Brain</Badge>
<Badge tone="neutral" mono>128k</Badge>
<Badge tone="teal">Workflow pack</Badge>
```

Tones map to meaning (brain, teal=workflow, review=PR, slate=idle). Variants: `soft`, `solid`, `outline`, `dot`. Set `mono` for numerics/SHAs.

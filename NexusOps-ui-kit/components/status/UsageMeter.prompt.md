Context / token / cost capacity meter. Fill escalates normal → warn → risk → stop by threshold, so high-context and high-cost states read instantly.

```jsx
<UsageMeter label="Context" value={128} max={200} valueText="128k / 200k" />
<UsageMeter variant="ring" value={92} max={100} label="ctx" />
<UsageMeter label="Tokens" value={41} max={50} accuracy="estimated" valueText="≈41k" />
```

Use the `ring` in dense session rows and graph nodes; the `bar` in inspectors and the usage dashboard. Set `accuracy` to reflect adapter fidelity.

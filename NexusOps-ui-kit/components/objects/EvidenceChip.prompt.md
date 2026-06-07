Grounds Project Brain output in real, openable objects. Always attach evidence to a Brain answer or proposed action — Brain suggests, evidence proves, Action Gateway confirms.

```jsx
<EvidenceChip kind="commit" label="4f18a70" sub="add eval gate" />
<EvidenceChip kind="pr" label="#84" sub="workflow registry" />
<EvidenceChip kind="anchor" label="ARCHITECTURE.md#gateway" freshness="stale" />
<EvidenceChip kind="session" label="Codex · PR checks fix" onClick={open} />
```

Kinds: `file, anchor, plantask, session, commit, pr, decision, ticket, event, memory`. Render them as a row under a Brain answer; the stale dot warns when grounding is out of date.

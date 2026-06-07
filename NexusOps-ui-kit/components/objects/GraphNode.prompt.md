An operational node for the observability graph. Node chrome encodes object type; the status ring encodes state; selection uses the accent glow. The graph is a live map, so every node shows ownership and status — never decoration.

```jsx
<GraphNode kind="session" title="ENG-221 OAuth" status="waiting-human" beacon
  subtitle="claude-code · max-main" owner="Project A" meta={['93% ctx']} selected />
<GraphNode kind="worktree" title="agent/eng-221" status="active" meta={['+7 −2']} />
<GraphNode kind="pr" title="#84 workflow registry" status="pr-open" owner="Codex" />
<GraphNode kind="brain" title="3 evidence" subtitle="grounded @ 4f18a70" />
```

Kinds cover the whole object model (project, session, team, worker, worktree, branch, pr, issue, ticket, plantask, approval, human, brain). Pair with edges colored by relationship (`--graph-edge*` tokens).

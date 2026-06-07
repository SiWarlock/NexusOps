Leading rail or dot that makes a row's attention level visible, so queues and sidebars sort loudest-first.

```jsx
<div style={{display:'flex'}}>
  <AttentionMarker level={5} />   {/* beacon — waiting on human */}
  <SessionRow ... />
</div>
<AttentionMarker level={2} variant="dot" /> {/* pulsing — running */}
```

Levels mirror the attention ladder: `5` waiting-human, `4` failed/blocked, `3` degraded/capacity, `2` running, `1` active, `0` idle. Place the `rail` at the leading edge of a row/card; use `dot` inline.

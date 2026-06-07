Risk classification for Action Gateway steps, approval cards, and command rows. Always paired with a text label; never color-only.

```jsx
<RiskBadge level="readonly" />
<RiskBadge level="high" />
<RiskBadge level="critical" />   {/* adds hazard hatch */}
```

Levels: `readonly | low | medium | high | critical`. Use it next to any action the platform will execute, so the human can gauge consequence before approving.

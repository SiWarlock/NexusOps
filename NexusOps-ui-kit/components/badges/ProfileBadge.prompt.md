Names the Execution Profile (account/runtime context) a session uses — central to making cost attribution and account routing explicit.

```jsx
<ProfileBadge name="Claude Max Main" provider="claude" health="active" />
<ProfileBadge name="Codex Cloud GitHub" provider="codex" health="auth-expired" />
```

Show it on every session row, terminal header, and worker card. The health dot flags `rate-limited` / `auth-expired` profiles so the user can re-authenticate before dispatching.

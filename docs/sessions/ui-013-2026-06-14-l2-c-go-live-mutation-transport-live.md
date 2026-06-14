# ui-013 — L2-C the go-live: the live mutation transport is ON (L2 COMPLETE)

- **Date:** 2026-06-14
- **Phase:** Phase 6 (ui-resume) — **P6.8 L2-C** (the third + final L2 sub-slice; the USER-signed-off go-live; cat-1)
- **Predecessor:** [ui-012](ui-012-2026-06-14-pre-l2-gates-and-l2-transport-disabled.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `ui-team-lead`

## Why this session existed

L2-A (the crate mutation RPCs) and L2-B (the Tauri+TS mutation bridge, built but guarded-disabled behind the single `mutationsEnabled` switch) landed in ui-012. The only L2 work left was the go-live flip — and it was held on **explicit user sign-off (L2-O3)**. The lead ran the sign-off (via `AskUserQuestion`); the user chose **GO-now**, deferring the hands-on real-daemon operator walkthrough to a post-commit follow-on. This single-slice round flips the production Shell to construct the gateway port mutations-enabled — lighting up the live mutation transport + the UI submit controls together. **L2 is COMPLETE: a real human can now approve/deny/submit a real, daemon-risk-classified mutation from the cockpit.**

## What was built (1 slice — 1 commit `e7751ec`)

| Slice | Commit | What | security-reviewer |
|---|---|---|---|
| 057 / L2-C — the go-live flip (cat-1, USER-signed-off) | `e7751ec` | `Shell.tsx`: `new UdsGatewayPort()` → `new UdsGatewayPort({ mutationsEnabled: true })`. The single switch lights up the transport (port methods `invoke` instead of throw-not-enabled) + the controls (`canSubmitIntent && mutationsEnabled`) together. | CLEAR |

### Files created
- `docs/sessions/ui-013-2026-06-14-l2-c-go-live-mutation-transport-live.md` (this doc).

### Files modified
- `ui/src/shell/Shell.tsx` — the one-line production-port flip (`{ mutationsEnabled: true }`) + the go-live doc-comment.
- `ui/src/shell/Shell.uds-swap.test.tsx` — 2 production-Shell integration pins via a shared `productionShellAtCodeView()` helper (the per-hunk submit control ENABLES when connected; a click fires the live `invoke("gateway_submit_action")` through the production port) + the `Channel` stub and a never-settling `gateway_subscribe` handler so the connection stays connected for the pins.

## Decisions made

- **Flip-only (no feature-flag/setting).** The user sign-off IS the rollout control; a runtime toggle is unneeded surface. (Step-2.5 flag 1, default.)
- **The deterministic surface is the production-Shell flip; the load-bearing acceptance is the deferred live verification.** The Mock defaults `mutationsEnabled` true (set at L2-B), so the existing GatewayModal/DiffReview component tests already exercise the mutations-enabled state (controls-enable / disconnected-disabled / standing-grant-disabled / no-optimistic-done) — those are cited, not re-pinned. The genuinely-new pins are the two production-`<Shell/>` (real `UdsGatewayPort`) tests.
- **The go-live core is the click→invoke pin (ADD, orchestrator).** Test 1 proves the control is *enabled*; the sibling `production_shell_click_reaches_live_mutation_transport` proves a real click *fires the live invoke* through the production port — the deterministic proxy for the deferred real-daemon execution.
- **The test keeps the connection connected via a never-settling `gateway_subscribe`** (mocked `Channel` + `new Promise(()=>{})`) — a faithful mimic of a healthy live stream that never lag-closes (security-reviewer confirmed it does not mask a degrade path; the `Shell.subscribe.test.tsx` precedent).

## Decisions explicitly NOT made (deferred)

- **The real-daemon live-verification operator walkthrough** (launch the cockpit vs a real daemon → submit/approve a real action → the daemon executes + records the audit event → the cockpit reflects the daemon-confirmed status). Per the user's GO-now — a documented **post-commit operator follow-on**, NOT a blocking step. (Step-2.5 flag 2.)
- **The `policy_grant` "always allow" standing-grant** — its own cat-1; stays hardcoded-disabled (NOT this slice).
- Parked: Q7-B/C caching (parked-for-user); `submit_action_plan`/per-step; the other-5-projection live deltas.

## TDD compliance

**Clean.** The 2 production-Shell pins were written FIRST (RED confirmed for the right reason: the production port `mutationsEnabled` false → Stage disabled → click no-op → the live invoke never fired), then the one-line flip turned them GREEN. The Step-8 review fix was a comment-only change. No TDD violations; no safety-critical skips.

## Reachability (Step 7.5)

**LIVE — this IS the go-live.** `Shell.tsx:124` constructs the production `UdsGatewayPort({mutationsEnabled:true})` → the GatewayModal/DiffReview submit handlers (now enabled when connected) → the port's live `invoke` → the Tauri mutation command → the daemon Action Gateway. The `production_shell_click_reaches_live_mutation_transport` pin proves a real click fires `invoke("gateway_submit_action")` through the production port. The `canSubmitIntent` gate (054 single authority) is the load-bearing pre-submit gate (fail-safe FALSE on any degraded/disconnected/version-incompatible state); the daemon Gateway is the INV-SEC-1 chokepoint. No tested-but-unwired gaps introduced. (The L2-A typed crate helpers remain exposed-ahead by design — the bridge uses the generic `connect_and_call`; noted in ui-012, unchanged here.)

## Open follow-ups (Step-9 categorized — already routed hot)

- **Architecture doc note (orchestrator writes in `/orchestrate-end`):** **L2 COMPLETE — the mutation transport is LIVE** (the go-live flip, user-signed-off; the daemon Gateway remains the INV-SEC-1 chokepoint). → the `ui/CLAUDE.md` "Live UdsGatewayPort transport client" row.
- **Convention candidate (LESSON):** the cat-1 go-live = a SINGLE flag flip (the L2-B `mutationsEnabled` gate both the transport + the controls honor) gated on explicit user sign-off + a real-daemon live-verification operator gate; the deterministic surface is one line, the load-bearing acceptance is the live verification. Completes the L2 lesson arc (26/27).
- **Cross-doc invariant change:** NONE in `shared/` (the flip consumes the L2-B `mutationsEnabled` field; no CONTRACT bump; no schema-snapshot). **Multi-track memory check: no frozen-model field changed; nothing un-flagged.**
- **Future TODO (carry-forward, orchestrator records at the round seal):** the **DEFERRED real-daemon live-verification operator walkthrough** (a documented post-commit operator follow-on). Plus the parked follow-ons above (the standing-grant cat-1; Q7-B/C caching; `submit_action_plan`/per-step; the other-5-projection live deltas).

## Cross-doc invariant audit

Clean. No frozen `shared/` model field changed (the flip consumes the L2-B `mutationsEnabled` UI-local field). No drift.

## How to use what was built

The cockpit's mutation path is now live in production. With a connected + version-compatible daemon, the GatewayModal approve/deny + the DiffReview per-hunk submit controls are enabled; a click submits a typed intent to the daemon's Action Gateway, which risk-classifies, approves (the human-approval card), executes, and records the audit event. The UI submits intents only; the daemon is the single mutator. The next operator step is the deferred real-daemon walkthrough to confirm the end-to-end execute+audit against a live daemon.

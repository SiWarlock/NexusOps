# ui-010 — the live `UdsGatewayPort` read transport COMPLETE (051 TS port + read-swap · 052 subscribe streaming + recovery)

- **Date:** 2026-06-14
- **Phase:** Phase 6 (ui-resume) — **P6.8 L1** (the live `UdsGatewayPort` read transport — "go-live", reads-only, NON-cat-1), slices 3a/3 + 4/4 → **L1 COMPLETE**
- **Predecessor:** [ui-009](ui-009-2026-06-14-6.3e-completion-and-L1-read-transport.md)
- **Successor:** [ui-011](ui-011-2026-06-14-L2prep-regen-0.31-and-approval-real-risk.md)
- **Track:** `track/ui` · implementer `ui-implementer` (fresh session, cycled at the 050 boundary) · orchestrator `ui-orchestrator` · lead `ui-team-lead`

## Why this session existed

ui-009 built the L1 read-transport FOUNDATION (049 the pure-Rust `gateway-uds` crate + 050 the Tauri host + read bridge). This session consumed that foundation end-to-end to make the cockpit **read real daemon data, live**: the TS client + the Shell read-swap (051), then the streaming `subscribe` consumer + the reconnect recovery (052). After this, **L1 (the live read transport) is COMPLETE** — the app reads real daemon data on load AND stays live as the daemon mutates. L2 (the mutation transport) stays cat-1 HELD on the daemon's 0.30.0 ②-mini.

## What was built (4 commits across 2 slices)

| Slice / layer | Commit | What |
|---|---|---|
| 051 (A + C-single-shot) | `1996730` | the TS `UdsGatewayPort` single-shot reads + the Shell read-swap → real daemon data on load |
| 052 layer B-Rust | `01a1eac` | `subscribe_stream` push-loop + the dedicated-connection adapter on the 049 crate |
| 052 layer B-Tauri+TS | `ab4f6bf` | the `gateway_subscribe` Channel bridge + `UdsGatewayPort.subscribe()` AsyncIterable |
| 052 layer C | `67bf237` | the Session delta-reducer + the reconnect-recovery supervisor + the Shell subscribe-effect → **L1 COMPLETE** |

**051 — the Step-2.5 SPLIT:** at 051's test design I flagged that Layer B (subscribe streaming) was 4 new sub-surfaces and recommended carving it to 052; the orchestrator ruled the split. So 051 = Layer A (the single-shot read client) + Layer C-single-shot (the Shell read-swap); 052 = the streaming B+C.

### Files created
- **051:** `ui/src/gateway-client/uds.ts` (the `UdsGatewayPort` — the live §6.1 read client) + `uds.test.ts`; `ui/src/shell/Shell.uds-swap.test.tsx` (the production-default read-swap + transport-fault degrade).
- **052:** `ui/src/gateway-client/delta-reducer.ts` (`applySessionDelta` — apply a `ProjectionDelta` to the Session read cache) + `delta-reducer.test.ts`; `ui/src/gateway-client/subscribe-recovery.ts` (`runSubscriptionSupervisor` — the reconnect machine) + `subscribe-recovery.test.ts`; `ui/src/shell/Shell.subscribe.test.tsx` (the live-delta re-render integration).

### Files modified
- **051:** `ui/src/gateway-client/boundary.ts` (+`parseDiff`/`parseCapabilities`); `ui/src/shell/Shell.tsx` (default gateway `MockGatewayPort`→`UdsGatewayPort` for reads); `ui/src/main.tsx` (comment).
- **052:** `ui/gateway-uds/src/lib.rs` (`subscribe_stream` + `connect_and_subscribe` + a behavior-preserving `call`→`demux_rpc_response` refactor) + `ui/gateway-uds/tests/integration.rs` (`#[ignore]` live subscribe probe); `ui/src-tauri/src/commands.rs` (`SubscriptionEvent` + the `gateway_subscribe` Channel command + `GatewayCommandError: Clone` + the HIGH null-swallow fix) + `ui/src-tauri/src/lib.rs` (register `gateway_subscribe`); `ui/src/gateway-client/uds.ts` (real `subscribe()` + `subscriptionIterable`); `ui/src/gateway-client/uds.test.ts` (+subscribe tests; dropped the now-wired `subscribe`-throws assertion); `ui/src/gateway-client/mock.ts` (subscribe = a live stream: a benign delta then stays open); `ui/src/projections/fixtures/proj_session.ts` (`sessionDeltaFixture` made a benign no-op).

## Decisions made
- **051 Q1 SPLIT (orchestrator-ruled):** 051 = A + C-single-shot; 052 = B (streaming) + the streaming recovery. Rationale: B is 4 new sub-surfaces, each separately security-reviewable; A+C-single-shot is the clean go-live win on its own.
- **The wire-vs-transport error classification (THE security pin, LESSON §16):** a daemon `GatewayCommandError{kind:"wire",code}` is thrown as PLAIN `{code}` (NOT an `Error`) so the consumer routes the §6.4 code verbatim via `!instanceof Error`; a transport/host fault is thrown as an `Error` (honest degrade, §11.7); a non-`wire`/non-`version_skew` fault → `markDisconnected` (fail-safe gate). `version_skew` is connection-neutral (the version axis owns it).
- **The connection state is the port's (fail-safe):** `UdsGatewayPort` starts `connecting`, flips to `connected` only on a confirmed daemon response (LESSON §4); a transport fault → `disconnected`.
- **The subscribe transport = a dedicated persistent connection** (handshake→subscribe RPC→ack→push-read loop, **NO read timeout** — idle subscriptions block on the next push; only the daemon's lag-close ends it; id 1, multiplex deferred). `ProjectionDelta` carries no `seq` → close-on-lag terminates the stream cleanly; recovery is reconnect→re-subscribe→re-`get_projection`, NEVER client gap-fill (daemon LESSON 12).
- **Recovery order = re-`get_projection` (snapshot) BEFORE re-subscribe** (a fresh baseline before deltas resume — safer than the literal re-subscribe→re-fetch). Consequence: a tiny re-subscribe gap window (a delta in the snapshot↔subscribe interval is missed until the next delta for that row) — an accepted MVP limitation (self-heals; the seq-less model can't detect gaps anyway).
- **Q3 (orchestrator-ratified): land the live mechanism end-to-end on the Session projection** (the most dynamic) + SPREAD the other 5 (they reuse the identical mechanism). Keeps the slice + the streaming-boundary security review tractable.
- **The Tauri Channel payload = a tagged `SubscriptionEvent {delta|closed|error}`** (distinct close-vs-error, §11.7; a ui-host-local marshaling type, NOT a shared contract). Confirmed the Tauri 2.x `ipc::Channel` API via Context7.
- **The MockGatewayPort now simulates a live stream** (yields a benign delta then stays open) so the live supervisor doesn't spin recovery in Mock-backed tests; the status-changing live re-render is exercised against a dedicated fake gateway (`Shell.subscribe.test.tsx`).

## Decisions explicitly NOT made (deferred)
- **The two-writer `connection`-state reconciliation** — the Step-8 Finding (below): NOT hastily refactored (entangled with the Mock's degraded-banner test contract). Recommend its own slice **before L2**.
- **A deterministic subscription teardown** (AbortController→channel-drop / generator `.return()`) — the idle-unmount Rust-thread linger is accepted MVP (matches the daemon-side linger); a clean teardown is a follow-on.
- **The other-5-projection live deltas** — the mechanism is proven on Session; the spread is a mechanical follow-on.
- **L2 mutation transport** — cat-1, HELD on the daemon's 0.30.0 ②-mini approval-enrichment; its own cat-1-checkpointed slice.
- **Multiplexed unique-correlation-ids** — deferred (daemon MVP is 1-subscription-per-connection, terminal post-ack).

## TDD compliance
**CLEAN across both slices, all layers.** Each layer: tests written first → RED confirmed (module/symbol missing or assertion mismatch) → GREEN → refactor. 051: Layer A (10 pins) + Layer C-single-shot (2). 052: B-Rust (6 over a fake stream) → B-Tauri (the `SubscriptionEvent` distinct-kind pin) + B-TS (5 over a fake `start`) → C (delta-reducer 5 + recovery 4 + the Shell live-render integration). The Step-8 whole-boundary review fixes were test-strengthening or behavior-preserving with a covering test added the same pass (the HIGH `expect` is infallible/behavior-preserving; the projection-match guard got a dedicated test in this `/session-end` audit; the lost-wakeup + invoke-arg pins are net-new tests). **No production logic back-filled without a test.** `security-reviewer` ran on both transport boundaries (CLEAR).

## Reachability
- **051 (Step 7.5):** REAL — `main.tsx <Shell/>` → `new UdsGatewayPort()` → the load effect's `get_projection`×6 + `get_capabilities`; `DiffReview`/`SessionTerminal` get `gateway={client}` → `get_diff` reachable.
- **052 (Step 7.5):** REAL — `main.tsx <Shell/>` → the subscribe-effect → `runSubscriptionSupervisor` → `client.subscribe("Session")` → `invoke("gateway_subscribe", channel)` → the registered command → `connect_and_subscribe` → `subscribe_stream` → the daemon; `applySessionDelta` consumes deltas; the recovery path is reachable from a stream close.
- **Mutation methods stay un-wired** (only the 3 reads + `gateway_subscribe` are registered; `submit_action`/`approve`/`deny`/`preview_action` throw not-wired; `subscribe_terminal` is P4) — pinned that the read client can never reach a mutation `invoke`. No tested-but-unwired gaps.

## Open follow-ups
- **🔴 FINDING (routed by the orchestrator → lead):** the **two-writer `connection`-state race** in `Shell.tsx` — `client.onConnectionChange(setConnection)` (raw, read-path) vs the supervisor's guarded `setConnection`. A successful read firing `markConnected` could momentarily mask a supervisor stream-degrade → `canSubmitIntent` true while the live stream is down (a defense-in-depth / forbidden #6 / LESSON §4 weakening). Reads-only today (the daemon Gateway is the real INV-SEC-1 guard) → not exploitable now, but the SAME `canSubmitIntent` surface **L2 (cat-1) makes load-bearing**. Recommend reconcile (a single connection-state authority) **before L2**.
- **Carry-forwards (cross-track / future-phase):** the IDLE-unmount subscription-thread linger (deterministic teardown follow-on) · the other-5-projection live-delta spread (Q3) · L2 mutation transport (cat-1, HELD on 0.30.0) · multiplexed unique-correlation-ids (deferred) · csp:null pre-ship hardening · the 0.29.0 survival-types regen (at-leisure) · placeholder Tauri icons.
- **Orchestrator-handled (round seal):** the L1-complete LESSON (the subscribe-transport pattern — extends §16/§20/§21) + the two `ui/CLAUDE.md` transport rows → **052 DONE / L1 COMPLETE** + the Carry-forward triage + the staged 049–052 routing + the briefs.

## How to use what was built
- **Run the cockpit (dev):** `pnpm dev` (Vite) renders the chrome; `pnpm tauri dev` runs the Tauri app — against a running `nexusopsd`, the Shell reads REAL projections on load AND live-updates the session list as the daemon mutates.
- **Visual gate (Q4) — honest:** no live daemon in this worktree, so the live Tauri-window gate (real deltas rendering) is the **manual cross-track operator step**. Verified here: `Shell.subscribe.test.tsx` (a streamed delta renders a new session live), the reducer + recovery unit tests, the production Vite build, + the `#[ignore]` live subscribe integration probe (`cargo test -p nexusops-gateway-uds --test integration -- --ignored` against a running daemon).

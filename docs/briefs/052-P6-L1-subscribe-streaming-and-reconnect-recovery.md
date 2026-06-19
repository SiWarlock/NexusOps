# /tdd brief — subscribe_streaming_and_reconnect_recovery

## Feature
**L1 read transport, slice 4 of 4 -> L1 COMPLETE.** The streaming `subscribe` consumer + the
reconnect recovery — the part carved OUT of 051 at its Step-2.5 split (051 Q1). Three layers:
**(B-Rust)** a NEW `subscribe_stream` on the 049 `nexusops-gateway-uds` crate — a **dedicated
persistent connection**: handshake → the `subscribe` RPC → read the ack → loop reading
`ServerFrame::SubscriptionPush(ProjectionDelta)` → yield each delta; **close-on-lag / EOF →
terminate the stream** (the daemon closes on lag — `ProjectionDelta` carries **no `seq`** so a gap
is client-undetectable; recovery is reconnect, NOT client gap-fill — daemon LESSON 12). **(B-Tauri+TS)**
a Tauri **`ipc::Channel`** command (`gateway_subscribe`) streaming the deltas to the frontend + the
TS `UdsGatewayPort.subscribe()` returning an `AsyncIterable<ProjectionDelta>` that **`parseDelta`-parses
each frame at the boundary**. **(C)** the **Shell subscribe-effect + delta-reducer** (live deltas
re-render the projection views) + the **reconnect → re-subscribe → re-`get_projection` recovery
machine** (the cross-state `reconnect()` the 051 stub deferred). **NON-cat-1** (reads only — the
mutation methods stay un-wired/HELD). After this, **L1 (the live read transport) is COMPLETE: the app
reads real daemon data live AND stays live as the daemon mutates.**

> **Big, layered, multi-commit slice** (the fresh impl drives layer→layer, LESSON ui-7): B-Rust
> (`subscribe_stream` over a fake stream) → B-Tauri+TS (the Channel command + the AsyncIterable) →
> C (the Shell delta-reducer + the recovery machine). 049/050/051 are single-shot one-shot-per-call;
> **this is the FIRST persistent connection** (a push-read loop, not a request/response).

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport — go-live; L1 read transport, slice 4 of 4 → **L1 COMPLETE**)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the `subscribe` method + `ProjectionDelta`), `§6.4` (the wire — the `SubscriptionPush` frame demux + close-on-lag), `§11.1`/`§11.7` (the degraded/reconnect recovery gate + honest degradation — never stale-as-live), `§5.0` (the frozen frames the boundary `parseDelta`-parses).
- **Reference:**
  - **049 crate** (`ui/gateway-uds/src/lib.rs`, `285fee6`) — EXTEND with `subscribe_stream`; reuse `handshake`/`read_frame`/`encode_frame`/the `ServerFrame` demux + the 8 MiB `MAX_FRAME_SIZE` bound. The single-shot `connect_and_call` is the adapter precedent (but the subscribe connection does NOT use the 30 s `DEFAULT_READ_TIMEOUT` — see AC).
  - **050 bridge** (`ui/src-tauri/src/commands.rs`, `3188ea9`) — the `#[tauri::command]` + `call_daemon`/`spawn_blocking` + `GatewayCommandError` precedent; the NARROW typed allowlist (LESSON 21).
  - **051** (`ui/src/gateway-client/uds.ts`, `1996730`) — the `subscribe()` method currently `throw`s "not wired (slice 052)" (line 207); `reconnect()` is the 051 no-op stub explicitly deferring "the full cross-state reconnect … to the 052 subscribe-recovery machine" (line 166).
  - **Frozen contract** (`shared/src/ipc.rs:285-315,377-384`) — `SubscribeParams{projection: ProjectionName, filter?}` · `ProjectionDelta{projection, kind: upsert|remove, row?, id?}` (NO `seq`) · `ServerFrame::SubscriptionPush(ProjectionDelta)`. The TS side: `ProjectionDelta` contract + **`parseDelta` already exists** (`boundary.ts:76`); `SubscribeParams`/`subscribe()` are already on `GatewayPort` (`types.ts:34,46`).
  - **Tauri 2.x `ipc::Channel<T>`** for the Rust→frontend delta stream — pull Context7 `/tauri-apps/tauri-docs` for the current API.
  - The daemon's subscribe-serve (live since 1.6d; **dedicated connection, terminal/single-writer post-ack, close-on-lag** — daemon LESSON 12). Wire-contract questions → the daemon orchestrator (reference `nexusopsd smoke dev-client`); but the contract is FROZEN @ 0.28.0 and present in-workspace (`shared/`), so confirm shapes there directly.

## Acceptance criteria (what "done" means)
**Layer B-Rust — `subscribe_stream` on the 049 crate:**
- [ ] A `subscribe_stream<S: Read + Write>(stream, projection, sink)` (or equivalent) that: forms `SubscribeParams{projection}` → writes the `subscribe` `RpcRequest` (id 1) → reads + id-correlates the ack `ServerFrame::RpcResponse` (a `WireError` ack → `Err(Wire(code))`; a non-RpcResponse/id-mismatch/dual-None ack → `Protocol`, fail-closed, exactly as `call`) → then **loops** `read_frame` → demux `ServerFrame::SubscriptionPush(delta)` → push each `delta` to the sink; a non-`SubscriptionPush` frame mid-stream → `Protocol`; **EOF / `read_frame` `Io` (the daemon closed on lag) → terminate the stream cleanly** (a normal close, NOT a hard error — the recovery contract). TDD'd over a fake stream (a scripted ack + N pushes + a close).
- [ ] The **8 MiB per-frame bound holds on every pushed frame** (`read_frame`/`decode_len` reject pre-alloc — the existing codec; pin a `SubscriptionPush` body > 8 MiB → `FrameTooLarge`, never allocated).
- [ ] The real-socket adapter (`connect_and_subscribe(projection, on_delta)` or similar, `#[ignore]` integration test) opens a **dedicated** `UnixStream` and **does NOT set the 30 s single-shot read timeout** (a healthy idle subscription blocks on the next push indefinitely; only the daemon's lag-close ends it). Document the deliberate no-timeout.
- [ ] **Correlation-id note (resolves carry-forward (4)):** the dedicated subscribe connection issues **exactly one** RPC (the subscribe, id 1) then only reads pushes → id 1 is correct; **multiplexed unique-correlation-ids stay deferred** (the daemon MVP is 1-subscription-per-connection, terminal post-ack — no multiplexing). Confirm at Step 2.5.

**Layer B-Tauri + TS — the Channel bridge + the AsyncIterable:**
- [ ] `gateway_subscribe(projection, channel: Channel<T>)` `#[tauri::command]` — spawns `connect_and_subscribe` off the async runtime (`spawn_blocking`, the 050 precedent), sends each delta over the Tauri **`Channel`**, and signals **stream-end** (close/lag) + an **error** distinctly (a tagged Channel payload — e.g. `{kind:"delta", delta}` / `{kind:"closed"}` / `{kind:"error", error: GatewayCommandError}`; see Step-2.5 Q1). **Typed-narrow** — added to the allowlist; **still NO mutation/`gateway_call` command** (LESSON 21 holds). On teardown/recovery the **old blocking thread + stream are dropped before a re-subscribe** (no leaked subscription threads).
- [ ] `UdsGatewayPort.subscribe(params)` returns an `AsyncIterable<ProjectionDelta>` that consumes the Channel, **`parseDelta`-parses each delta at `boundary.ts`** (parse-don't-trust — a malformed delta → `BoundaryValidationError`, never yielded), and **ends the iterable on the stream-close / surfaces an Error on the error signal** (honest degrade, never silent). It replaces the 051 `throw "not wired (slice 052)"`.

**Layer C — the Shell subscribe-effect + delta-reducer + recovery:**
- [ ] The Shell opens the `subscribe` stream for the projection(s) it renders (the dedicated-connection model; the exact subscribed set is the impl's call — flag if it balloons, Step-2.5 Q3) and a **delta-reducer** applies each `ProjectionDelta` to the live projection view (`upsert` → insert/replace the row by `id`; `remove` → drop by `id`) so the cockpit re-renders as the daemon mutates. The local store stays a **read cache** (forbidden #2 — never authoritative).
- [ ] **Reconnect recovery machine:** on a subscribe stream close (lag/disconnect), the connection goes **degraded** (`connected → disconnected`, `canSubmitIntent` → false per §11.1) and the recovery runs **`reconnect()` (→ `reconnecting`) → re-subscribe → re-`get_projection` (re-fetch the snapshot) → `connected`** (then resume the delta stream). The cross-state `reconnect()` (051 stub) becomes real (drives `connected/disconnected → reconnecting → connected`, respecting `canTransition`). **No stale-as-live (§11.7):** while degraded the UI shows the degraded surface; it never renders a post-close stale delta as live, and only shows fresh data after the re-`get_projection`.
- [ ] Whole suite green (312 + the net-new TS pins; the Rust `subscribe_stream` pins); `/preflight` clean (oxlint/tsc/vitest + cargo `-p nexusops-gateway-uds -p nexusops-ui`); **the visual gate** confirms live deltas render (a running `nexusopsd`, or a recorded/scripted delta sequence if no daemon — flag which, Step-2.5 Q4; LESSON 10/22 — the live-rendered gate is the manual cross-track operator step).
- [ ] **`security-reviewer` REQUIRED** (the streaming boundary): the **`parseDelta` of every pushed frame** (parse-don't-trust), the **8 MiB frame bound on the stream**, the **no-mutation-reach** (the subscribe path adds no mutation command), the **recovery never shows stale-as-live** (§11.7), and **no leaked subscription thread/stream** on teardown/recovery.
- [ ] Cross-doc flagged at Step 9 (the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport" row + the "Tauri host + read-command bridge" row → mark 052 done / L1 COMPLETE).

## Wiring / entry point (Step 7.5)
**REAL entry — L1 goes fully LIVE.** The Shell's subscribe-effect mounts `UdsGatewayPort.subscribe(...)`
→ the `gateway_subscribe` Channel command → `spawn_blocking(connect_and_subscribe)` → the 049
`subscribe_stream` → the daemon's `gateway.sock`. `/wired subscribe`: Shell subscribe-effect →
`UdsGatewayPort.subscribe` → `invoke(gateway_subscribe, channel)` → the Tauri bridge → the 049 crate
→ the daemon push stream → the delta-reducer re-renders. The recovery path (`reconnect` →
re-subscribe → re-`get_projection`) is reachable from a stream close. The **mutation methods stay
un-wired (L2 HELD)** — confirm no production path reaches them.

## Files expected to touch
**New:**
- `ui/src/gateway-client/` — possibly a small `subscribe-recovery.ts` (the reconnect→re-subscribe→re-fetch machine) + a `delta-reducer.ts` (apply a `ProjectionDelta` to a projection page) if cleaner than inlining in the Shell, each with tests.

**Modified:**
- `ui/gateway-uds/src/lib.rs` — `+subscribe_stream` + the `connect_and_subscribe` adapter + unit tests (the fake-stream scripted-push pins) + `tests/integration.rs` (`#[ignore]` live).
- `ui/src-tauri/src/commands.rs` — `+gateway_subscribe` Channel command (+ the Channel payload type) + the pure-fn marshal/map tests; `ui/src-tauri/src/lib.rs` — register it in `generate_handler!`.
- `ui/src/gateway-client/uds.ts` — `subscribe()` real impl (the AsyncIterable over the Channel) + the real cross-state `reconnect()`.
- `ui/src/shell/Shell.tsx` (+ `ui/src/connection/` if the recovery machine lives there) — the subscribe-effect + the delta-reducer + the recovery wiring.

If beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2)
**B-Rust (`ui/gateway-uds`, vs a fake stream):**
1. `subscribe_stream_yields_each_push_delta` — ack then 2 `SubscriptionPush` → both deltas reach the sink, in order. — Asserts: the push-read loop demuxes `SubscriptionPush(ProjectionDelta)` (§6.4).
2. `subscribe_stream_terminates_on_close` — ack then EOF/`Io` → the stream ends cleanly (no panic, no error surfaced as a fault). — Asserts: close-on-lag → terminate, no gap-fill (daemon LESSON 12; §11.7).
3. `subscribe_ack_wire_error_fails_closed` — the subscribe ack is a `WireError` → `Err(Wire(code))` verbatim. — Asserts: §6.4 ack rejection fail-closed.
4. `subscribe_push_over_8mib_rejected` — an oversized push frame → `FrameTooLarge` pre-alloc. — Asserts: the §6.4 8 MiB bound holds on the stream.
5. `subscribe_non_push_frame_mid_stream_is_protocol` — a `RpcResponse`/`TerminalOutput` after the ack → `Protocol`. — Asserts: frame-discipline on the push loop.

**B-Tauri + TS (`commands.rs` pure fns + `uds.test.ts` vs a fake Channel):**
6. `subscribe_channel_payload_maps_delta_close_error_distinctly` (Rust pure-fn) — each Channel payload variant is distinct + leak-free. — Asserts: §11.7 distinct close vs error.
7. `subscribe_iterable_parsedelta_each_frame` — a malformed delta → `BoundaryValidationError`, never yielded. — Asserts: parse-don't-trust (§5.0/§4.2 law 2).
8. `subscribe_iterable_ends_on_stream_close` / `surfaces_error_on_error_signal` — Asserts: honest degrade, never silent (§11.7).
9. `subscribe_mutation_methods_still_throw_not_wired` — adding subscribe does NOT wire a mutation. — Asserts: no-mutation-reach (INV-SEC-1 / cat-1 HELD).

**C (TS, Shell/recovery/reducer):**
10. `delta_reducer_upsert_and_remove_by_id` — upsert replaces/inserts by id; remove drops by id. — Asserts: live projection re-render (forbidden #2 read-cache).
11. `stream_close_triggers_reconnect_resubscribe_refetch` — close → `disconnected` (canSubmitIntent false) → `reconnecting` → re-subscribe + re-`get_projection` → `connected`; no stale-as-live. — Asserts: §11.1/§11.7 recovery.
12. `recovery_respects_legal_transitions` — the machine never makes an illegal `canTransition` hop. — Asserts: the connection state-machine invariant.

Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet → its test.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** — consumes the FROZEN `SubscribeParams`/`ProjectionDelta`/`ServerFrame::SubscriptionPush` (`shared/` @ 0.28.0). The Channel payload type is a ui-host-local marshaling type (not a shared contract).
- **Orchestrator doc rows (Step 9):** update the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport" row + the "Tauri host + read-command bridge" row → **052 DONE / L1 COMPLETE** (subscribe streaming + the Channel command + the delta-reducer + the recovery machine). **No `ARCHITECTURE.md` edit** (the daemon authored §6.1/§6.4 subscribe + close-on-lag).
- **New shared-contract model?** No.

## Things to flag at Step 2.5
1. **The Tauri Channel payload shape.** Default: a **tagged enum** `{kind:"delta", delta: Value} | {kind:"closed"} | {kind:"error", error: GatewayCommandError}` over one `Channel<T>` (clean stream-end + error signalling; reuses the 050 `GatewayCommandError`). Alt: separate the close/error onto the command's `Result` return + only deltas on the Channel. My vote: **the tagged enum** — one ordered stream, distinct close vs error (§11.7). Confirm the Tauri 2.x `Channel` API via Context7.
2. **Auto-reconnect vs manual Retry for the read stream.** Default: **automatic** reconnect on close with a **bounded backoff** (don't hammer the daemon), the degraded banner visible throughout, the existing §11.1 Retry affordance (`reconnect()`) as the manual fallback. Rationale: a READ subscription recovery is non-cat-1 (no mutation replayed — distinct from the Q7 mutation cache/retry, which stays parked) so auto-recovery is safe + expected. Flag if you'd rather ship manual-Retry-only for 052 and defer auto-backoff.
3. **The subscribed-projection set + the delta-reducer integration.** Default: subscribe to the projection(s) the Shell currently renders from `get_projection`, one dedicated connection each (the daemon's 1-sub-per-connection MVP); the reducer applies deltas to those pages. Flag if wiring every projection balloons the slice — it's acceptable to land the mechanism on ONE representative projection (e.g. `Session`) and spread the rest, as long as L1 "live data stays live" is demonstrably real on that path.
4. **The visual gate without a live daemon.** Default: run the gate against a running `nexusopsd` if available (the gated cross-track operator step); else drive a **scripted/recorded delta sequence** through the reducer + render and flag it explicitly (LESSON 10/22 — green tests ≠ live render; the live-rendered gate stays a manual operator step in this worktree).
5. **051 split is consumed — confirm no residual.** 052 IS the carved-out B+C from 051's Step-2.5 split; confirm nothing from 051 Layer A/C-single-shot is re-touched here beyond the `subscribe()`/`reconnect()` real impls.

## Dependencies + sequencing
- **Depends on:** 049 (`285fee6` — extend `subscribe_stream`) + 050 (`3188ea9` — the Tauri host + `spawn_blocking`/`GatewayCommandError` precedent) + 051 (`1996730` — the `UdsGatewayPort` + the `subscribe()`/`reconnect()` stubs + the boundary `parseDelta`). The daemon's subscribe-serve (live since 1.6d, frozen @ 0.28.0).
- **Blocks:** nothing in L1 — **this COMPLETES L1.** The **L2 mutation transport** (cat-1) stays HELD on the daemon's 0.30.0 ②-mini approval-enrichment — a separate cat-1-checkpointed slice (orchestrator escalates to the lead BEFORE authoring). After L1: the app reads real daemon data live AND stays live across daemon mutations.

## Estimated commit count
**2–4** (layered, the impl drives layer→layer per LESSON ui-7): B-Rust `subscribe_stream` · B-Tauri+TS the Channel command + the AsyncIterable · C the delta-reducer + the recovery machine. **NON-cat-1** (reads only — the mutation methods stay un-wired/HELD) but **`security-reviewer` REQUIRED** (the streaming transport boundary).

## Lessons-logged candidates anticipated
- **Convention candidate** — "the subscribe transport is a **dedicated persistent connection** (049 `subscribe_stream`: handshake → subscribe RPC → ack → a push-read loop, NO single-shot read timeout) → a Tauri `Channel` → a TS `AsyncIterable`, `parseDelta`-parsed per frame; it **terminates on close/lag** (`ProjectionDelta` has no `seq` → recovery is reconnect → re-subscribe → re-`get_projection`, NEVER client gap-fill — daemon LESSON 12); the recovery machine drives the real cross-state `reconnect()` and never shows stale-as-live (§11.7)." Surface at Step 9.
- **Architecture-doc note candidate** — **L1 read transport COMPLETE**: the app reads real daemon data live AND stays live across mutations; **L2 (mutations) is the HELD next vertical** (cat-1, on 0.30.0 ②-mini).
- **Future TODO** — L2 mutation transport (cat-1, HELD); multiplexed unique-correlation-ids (deferred — the daemon MVP is 1-sub-per-connection); the `csp:null` pre-ship hardening; the 0.29.0 survival-types regen (at-leisure); placeholder Tauri icons.

## How to invoke
1. **Read this brief end-to-end** — the 3-layer structure + the 5 Step-2.5 questions.
2. Pre-flight: `track/ui`; `cargo` + the Tauri toolchain (verified at 050).
3. **Run `/tdd subscribe_streaming_and_reconnect_recovery`.**
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 5 questions + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (the streaming boundary: `parseDelta` per frame + the 8 MiB bound + no-mutation-reach + no-stale-as-live + no leaked thread).
7. Step 9 — the cross-doc flags (the two `ui/CLAUDE.md` transport rows → L1 COMPLETE) + the L1-complete lesson candidate.

# /tdd brief — ts_uds_gateway_port_and_shell_read_swap

## Feature
**L1 read transport, slice 3 of 3 — L1-COMPLETE (the go-live read path).** Three layers:
**(A)** the TS **`UdsGatewayPort`** implementing the `GatewayPort` READ methods (`get_projection`/
`get_diff`/`get_capabilities`) by `invoke`-ing the 050 Tauri commands + **Zod-parsing at the
boundary** (`boundary.ts`); **(B)** the **subscribe STREAMING** — a Rust `subscribe_stream` on the
049 `gateway-uds` crate (a persistent connection: handshake → subscribe RPC → loop reading
`ServerFrame::SubscriptionPush` → yield `ProjectionDelta`; close-on-lag) + a Tauri **`Channel`**
streaming the deltas to the frontend + the TS `subscribe()` `AsyncIterable`; **(C)** the **Shell
read-swap** (`MockGatewayPort`→`UdsGatewayPort` for the reads → **real daemon data in the UI**) +
the **reconnect→re-subscribe→re-`get_projection` recovery** (the daemon closes the subscribe
connection on lag — LESSON daemon-12 — so recovery = reconnect + re-subscribe + re-fetch). **NON-cat-1**
(reads only). After this, **L1 (the live read transport) is COMPLETE.**

> **Big, layered, multi-commit slice** (the fresh impl drives layer→layer): A (TS single-shot,
> delivers real data on load) → B (the Rust+Tauri+TS streaming) → C (the swap + recovery). **The
> subscribe streaming (B) is NEW Rust** — 049/050 are single-shot one-shot-per-call; streaming is a
> persistent connection + the push demux. **If B proves too large to co-land cleanly, flag at
> Step-2.5 to split it to 052** (then 051 = A+C-single-shot, L1-reads-on-load complete; 052 = B+C-streaming).

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport — go-live; L1 read transport, slice 3 of 3 → **L1 COMPLETE**)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the `GatewayPort` read surface + `subscribe`), `§6.4` (the wire — the `SubscriptionPush` frame + close-on-lag), `§5.0` (the frozen types the boundary Zod-parses), `§11.1`/`§7.2` (the degraded/reconnect recovery).
- **Reference:** the **049 crate** (`ui/gateway-uds`, `285fee6` — extend with `subscribe_stream`); the **050 bridge** (`3188ea9` — the `#[tauri::command]` read commands + the `GatewayCommandError`); **Tauri 2.x `ipc::Channel`** for the Rust→frontend delta stream (pull Context7 `/tauri-apps/tauri-docs`); the daemon's subscribe-serve (`daemon/src/ipc/server.rs` — dedicated connection, **close-on-lag**, LESSON daemon-12); `ui/src/gateway-client/{types.ts,boundary.ts,mock.ts}` (the interface + the Zod boundary + the swap point `Shell.tsx:100-114,164-177`).

## Acceptance criteria (what "done" means)
**Layer A — the TS `UdsGatewayPort` (single-shot reads):**
- [ ] `ui/src/gateway-client/uds.ts` — a `UdsGatewayPort implements GatewayPort` whose `get_projection`/`get_diff`/`get_capabilities` `invoke` the 050 commands (`@tauri-apps/api/core`) + **`boundary.ts`-parse the returned `Value`** (parse-don't-trust; a malformed payload → `BoundaryValidationError`, never returned) + map a `GatewayCommandError` → the `GatewayPort` error model (the `Wire{code}` → the §6.4-code path the consumers route).
- [ ] The mutation methods (`submit_action`/`approve`/`deny`/`preview_action`) on `UdsGatewayPort` **throw a "not-wired (L2)" error** (cat-1 HELD — they are NOT invokable; no Tauri mutation command exists). Pin this (the read client never reaches a mutation).

**Layer B — the subscribe streaming (Rust + Tauri + TS):**
- [ ] `gateway-uds`: a `subscribe_stream(projection, sink)` — handshake → the `subscribe` RPC → read the ack → **loop reading `ServerFrame::SubscriptionPush(ProjectionDelta)`** → push each to the sink; **close-on-lag / EOF → terminate the stream** (the recovery is reconnect, NOT client gap-fill — daemon-12); the **8 MiB per-frame bound** holds (the codec). TDD'd over a fake stream (a scripted push sequence + a close).
- [ ] The Tauri side: `gateway_subscribe(projection, channel: Channel<…>)` — spawns the `subscribe_stream` (off the async runtime), sends each delta via the Tauri **`Channel`**, signals stream-end on close/lag. Typed-narrow (added to the allowlist; still no mutation command).
- [ ] `UdsGatewayPort.subscribe(params)` returns an `AsyncIterable<ProjectionDelta>` consuming the Channel, **`boundary.ts`-parsing each delta** (`parseDelta`); the iterable ends when the stream closes.

**Layer C — the Shell swap + recovery:**
- [ ] The Shell instantiates **`UdsGatewayPort`** (not `MockGatewayPort`) for the READ path (the Mock stays dev/test-only — keep it injectable for tests). The initial `get_projection` fetches (`Shell.tsx:164-177`) now show **real daemon data**.
- [ ] **Reconnect recovery:** on a subscribe stream close (lag/disconnect), the connection-state goes degraded + the recovery runs **reconnect → re-subscribe → re-`get_projection`** (re-fetch the projection snapshot, then resume the delta stream) — the existing `connection`/`reconnect` surface drives it (§11.1 degraded gate; `canSubmitIntent` stays false while degraded).
- [ ] Whole suite green (297 + the net-new TS pins; the Rust `subscribe_stream` pins); `/preflight` clean (tsc/oxlint/vitest + cargo `-p`); **the visual gate** confirms real data renders (a running daemon, or a recorded fixture if no daemon — flag).
- [ ] **`security-reviewer` REQUIRED** (the transport boundary): the **Zod-parse of every daemon frame** (reads + deltas — parse-don't-trust), the **frame size-bound** (the deltas ride the 8 MiB codec bound), the no-mutation-reach (the read client can't submit), the recovery never silently shows stale-as-live (§11.7).
- [ ] Cross-doc flagged at Step 9 (the `ui/CLAUDE.md` `UdsGatewayPort` row).

## Wiring / entry point (Step 7.5)
**REAL entry — L1 goes LIVE.** The Shell mounts `UdsGatewayPort` for reads → the initial `get_projection`
fetches + the `subscribe` delta stream hit the real daemon (via the 050 Tauri commands + the new
subscribe channel). `/wired`: Shell → `UdsGatewayPort.get_projection`/`subscribe` → `invoke` → the
Tauri bridge → the 049 crate → the daemon. The Mock stays dev/test-only (injectable). The **mutation
methods stay un-wired (L2 HELD)** — confirm no production path reaches them.

## Files expected to touch
**New:** `ui/src/gateway-client/uds.ts` (+ `uds.test.ts`) · `ui/src/gateway-uds/src/` subscribe_stream (extend the crate) + tests.
**Modified:** `ui/src-tauri/src/commands.rs` (+`gateway_subscribe` channel command) + `lib.rs` (register it) · `ui/src/shell/Shell.tsx` (the read-swap + the recovery wiring) · possibly `ui/src/connection/` (the reconnect→re-subscribe recovery) · `ui/src/gateway-client/types.ts` (if the error model needs a not-wired variant).

If beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2)
**A (TS, uds.test.ts vs a fake `invoke`):** 1. `get_projection_invokes_command_and_boundary_parses` (the command + `parseProjectionPage`; a malformed payload → `BoundaryValidationError`). 2. `command_error_maps_wire_code_to_gatewayport_error` (the `GatewayCommandError{kind:wire,code}` → the §6.4-code path). 3. `mutation_methods_throw_not_wired_L2` (the read client never submits).
**B (Rust subscribe_stream, vs a fake stream):** 4. `subscribe_stream_yields_each_push_delta`. 5. `subscribe_stream_terminates_on_close_or_lag` (EOF/lag → stream end, no gap-fill). 6. `subscribe_push_frame_over_8mib_rejected` (the codec bound holds on the stream). **B (TS, vs a fake Channel):** 7. `subscribe_iterable_boundary_parses_each_delta` (`parseDelta`; malformed → error). 8. `subscribe_iterable_ends_on_stream_close`.
**C (TS, Shell/recovery):** 9. `shell_reads_real_via_uds_port` (the Shell uses `UdsGatewayPort` for reads). 10. `stream_close_triggers_reconnect_resubscribe_refetch` (the recovery sequence; degraded state; no stale-as-live).
Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (consumes the frozen §6.4 frames). The `UdsGatewayPort` is the real `GatewayPort` impl; no new shared model.
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` `UdsGatewayPort` row (the live read transport — single-shot + subscribe streaming + the Shell swap + the recovery; L1 COMPLETE). No `ARCHITECTURE.md` edit.
- **New shared-contract model?** No.

## Things to flag at Step 2.5
1. **Split B to 052?** If the subscribe-streaming (Rust `subscribe_stream` + the Tauri Channel + the TS AsyncIterable) is too large to co-land cleanly with A+C, **flag to split it to 052** (051 = A + the single-shot Shell swap = real data on load; 052 = B + the streaming swap + recovery). Default: keep one layered 051 (L1-complete in one slice; you have fresh context), but the split is sanctioned if the surface is too big.
2. **The Tauri streaming mechanism — `Channel` vs `emit`/`listen`.** Default: **`ipc::Channel<T>`** (Tauri 2.x — a typed per-subscription channel passed to the command; cleaner lifecycle than global events). Confirm via Context7; flag if `emit`/`listen` reads better.
3. **The recovery trigger + the degraded surface.** Default: a subscribe stream close (lag/disconnect) → `connection` degraded → reconnect → re-subscribe → re-`get_projection` (re-fetch the snapshot, then resume deltas). Reuse the existing `connection`/`reconnect`/§11.1 surface; `canSubmitIntent` stays false while degraded. Pin "no stale-as-live" (§11.7).
4. **The visual gate without a live daemon.** Default: run the visual gate against a running `nexusopsd` (the gated smoke path) if available; else against a recorded/fixture delta — flag which (LESSON 10: real data must actually render).

## Dependencies + sequencing
- **Depends on:** 049 (`285fee6` — extend `subscribe_stream`) + 050 (`3188ea9` — the Tauri host + the read commands). The daemon's subscribe-serve (live since 1.6d).
- **Blocks:** nothing in L1 (this COMPLETES L1). The **L2 mutation transport** (cat-1) stays HELD on the daemon's 0.30.0 ②-mini — a separate cat-1-checkpointed slice. After L1: the app reads real daemon data live.

## Estimated commit count
**2–4** (layered: A the TS single-shot + the Shell single-shot swap · B the Rust+Tauri+TS streaming · C the recovery — the impl drives layer→layer, LESSON ui-7). **NON-cat-1** (reads only — the mutation methods stay un-wired/HELD) but **`security-reviewer` REQUIRED** (the transport boundary: the Zod-parse of every daemon frame + the size-bound + the no-mutation-reach + no-stale-as-live). If B splits to 052 (Q1), 051 is 1–2 commits.

## Lessons-logged candidates anticipated
- **Convention candidate** — possibly: "the TS `UdsGatewayPort` is a parse-don't-trust client — every daemon frame (reads + subscribe deltas) is Zod-`.parse()`d at the `boundary.ts` seam before it reaches view code; the subscribe stream is a persistent Rust connection (049 `subscribe_stream`) → a Tauri `Channel` → a TS `AsyncIterable`, terminating on close/lag (recovery = reconnect→re-subscribe→re-get_projection, never client gap-fill); the mutation methods stay un-wired (L2 HELD)." Surface at Step 9.
- **Architecture-doc note candidate** — L1 read transport is COMPLETE; the app reads real daemon data live; L2 (mutations) is the HELD next vertical.
- **Future TODO** — L2 mutation transport (cat-1, HELD on 0.30.0 ②-mini); the csp:null pre-ship; the 0.29.0 survival-types regen (at-leisure).

## How to invoke
1. **Read this brief end-to-end** — the layer structure + the 4 Step-2.5 questions (esp. Q1 the split option).
2. Pre-flight: `track/ui`; `cargo` + the Tauri toolchain (verified at 050).
3. **Run `/tdd ts_uds_gateway_port_and_shell_read_swap`.**
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 4 questions (esp. the split) + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (the boundary Zod-parse + size-bound + no-mutation-reach + no-stale-as-live).
7. Step 9 — the cross-doc flag + the L1-complete lesson candidate + (if Q1) the split outcome.

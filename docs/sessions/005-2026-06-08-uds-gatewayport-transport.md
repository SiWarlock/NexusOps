# Session 005 — Phase 1.5 UDS GatewayPort transport (framing · peer-auth · handshake · dispatch · subscribe)

| | |
|---|---|
| **Date** | 2026-06-08 |
| **Phase** | Phase 1 (daemon foundation) — task 1.5 |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | [004](004-2026-06-08-lease-locks-and-fencing.md) |
| **Successor** | [006](006-2026-06-08-cold-start-bootstrap-and-runtime.md) — Phase 1.6a (cold-start bootstrap) + 1.6b (daemon runtime) |
| **Commits** | **1.5:** `ae27f1d` (L1 framing+getpeereid) · `fb941e8` (L2 handshake+contract 0.9.0) · `c1c54e5` (L3 dispatch+read-methods+protocol_error 0.10.0) · `6690e41` (L4 subscribe+ServerFrame 0.11.0) — slice complete |
| **Base** | merged `main` `48d9931` (post `track/ui`→main; the ui `GatewayPort` contract was the consumer surface) |

## Why this session existed
Implement the daemon side of the **UDS `GatewayPort` transport** per brief `007-P1-5-uds-gatewayport-transport.md` — the real `UdsGatewayPort` the merged ui's `MockGatewayPort` swaps to. A 4-layer multi-commit slice over a Unix-domain socket: **4-byte big-endian length-prefixed JSON-RPC** framing (bounded), a **`getpeereid()` peer-auth** gate (**safety rule #7**), a `HelloFrame`→`HelloAck`|`VersionSkewError` version handshake, the **§6.1 read methods** (`get_projection`/`get_capabilities`/`subscribe`) over **read-only WAL**, and a **frame-type multiplexing** envelope. Must satisfy the contract the ui already pinned (`ui/src/gateway-client/types.ts`, `SUPPORTED_PROTOCOL_RANGE {1,1}`). First Tokio/`libc` surface in the daemon (the runtime spawn is 1.6).

## What was built

### L1 — length-prefix framing + getpeereid peer-auth (`ae27f1d`) ⚠️ safety rule #7
- **Files created:** `daemon/src/ipc/mod.rs` (module + `IpcError`), `transport.rs` (`encode_frame`/`decode_len`/`MAX_FRAME_SIZE`=8 MiB — oversized rejected from the prefix BEFORE alloc), `peer.rs` (`authorize_peer` + `peer_uid` via isolated unsafe `libc::getpeereid`, fail-closed), `server.rs` (`serve_connection` — auth gate FIRST), `daemon/tests/ipc.rs` (tests 1–4).
- **Files modified:** `daemon/Cargo.toml`+`Cargo.lock` (+`libc 0.2`), `daemon/src/lib.rs` (`pub mod ipc`).

### L2 — handshake + the `shared/` IPC contract (`fb941e8`)
- **Files created:** `shared/src/ipc.rs` — the IPC wire contract (HelloFrame/HelloAck/VersionSkewError/Capabilities/WireError + IpcErrorCode + ProjectionName + protocol consts).
- **Files modified:** `daemon/src/ipc/{transport,mod,server}.rs` (`read_frame`/`write_frame`; `IpcError`+VersionSkew/Protocol; the handshake), `shared/src/lib.rs` (**CONTRACT_VERSION 0.8.0→0.9.0** + `pub mod ipc`), `shared/src/schema.rs` (ContractBundle), `contracts/schema/*.json` (regenerated), `shared/tests/{envelope,contract}.rs` (version pin + the IPC wire-pins).

### L3 — JSON-RPC dispatch + read methods + `protocol_error` (`c1c54e5`)
- **Files created:** `daemon/src/ipc/methods.rs` — `dispatch` + `get_projection`/`get_capabilities` over **read-only WAL** (`open_read_only`) + the closed-enum `ProjectionName`→`proj_*` table map + `read_table_as_json`; the infra-vs-client error split.
- **Files modified:** `shared/src/ipc.rs` (+RpcRequest/RpcResponse/GetProjectionParams/ProjectionScope + the lead-ratified **`protocol_error`** code), `shared/src/lib.rs` (**0.9.0→0.10.0**), `server.rs` (the dispatch loop), `mod.rs` (`IpcError::Read`), `shared/src/schema.rs` + regen + `shared/tests/*` + `daemon/tests/ipc.rs` (tests 8–11).

### L4 — subscribe streaming + `ServerFrame` frame-type multiplexing (`6690e41`)
- **Files created:** `daemon/src/ipc/subscribe.rs` — `push_subscription<W: Write>` (frame-type-tagged `SubscriptionPush` frames from a delta source).
- **Files modified:** `shared/src/ipc.rs` (+`ServerFrame` internally-tagged envelope + `ProjectionDelta`/`DeltaKind`/`SubscribeParams`), `shared/src/lib.rs` (**0.10.0→0.11.0**), `server.rs` (wrap responses in `ServerFrame::RpcResponse`), `methods.rs` (`subscribe` recognized/acked), `mod.rs` (re-export), `shared/src/schema.rs` + regen + `shared/tests/envelope.rs` + `daemon/tests/ipc.rs` (tests 11–14).

**Totals:** 8 files created, ~8 modified; **15 ipc integration tests** + shared wire-pins; workspace **83 passed / 0 failed (12 suites)**.

## Decisions made
- **Q1 versioning — two axes.** `CONTRACT_VERSION` (the §5.0 schema/codegen artifact, 0.8.0→**0.11.0** across L2–L4) vs `protocol_version` (the §6.4 wire-handshake skew check, pinned =1). Don't conflate.
- **Q2 `SUPPORTED_PROTOCOL_RANGE` — daemon-authored `{1,1}`** in `shared/` (the ui's provisional pin confirmed).
- **Q3 projection-name enum — canonical `UsageLedger`** (TWEAK: architecture-canonical; 5/6 ui names already matched; the ui's `Usage` is provisional → reconciles via Carry-forward). Closed enum, reject-unknown, **PascalCase** wire (matches the ui's `get_projection("Session")` literals).
- **getpeereid, NOT SO_PEERCRED** (ADR-004); the unsafe FFI isolated + fail-closed (a getpeereid error never authorizes); `Ok(uid)` no-cast (compile-error on a wider-uid_t target, not truncation).
- **`protocol_error` (lead-ratified, away-authority — Option B).** The §6.4 6-code set had no bad-first-frame/handshake-required code; L2 shipped `unknown_method` as a placeholder + escalated; the lead ruled "add `protocol_error`, fold within the round." Executed in L3 (verified `MVP_TASKS.md` line 614 ahead of an explicit relay; orchestrator affirmed): added the code, swapped the bad-frame/malformed/bad-params emissions (a genuine unknown METHOD name stays `unknown_method`), re-pinned test 7, **0.9.0→0.10.0**.
- **Frame-type multiplexing — internally-tagged `ServerFrame`** (`frame_type` JSON discriminant; **codec UNCHANGED** — no binary type-byte retrofit). Terminal-Channel tag **reserved** (no variant — encoding is a Phase-3 throughput call). **0.10.0→0.11.0**.
- **subscribe scope — mechanism now, live source → 1.6** (the established "ship the unit + contract, wire the runtime source later" pattern). L4 ships `push_subscription` + the contract + the `subscribe` dispatch ack; the EventStore→broadcast→subscriber delta-source + the dispatch→push routing join the 1.6 accept-loop spawn.
- **Read path = read-only WAL** (`open_read_only`); the closed-enum table map → no SQL injection; the write-actor stays sole writer (Forbidden #3 / LESSON §3).

## Decisions explicitly NOT made (deferred)
- **scope filtering** — `get_projection` accepts `scope.project_id` but does NOT enforce it (documented + test-pinned non-enforcement); filtering + `ProjectionScope.project_id` String→`ProjectId` newtype → post-MVP.
- **Terminal-Channel frame encoding** (JSON-base64 vs binary fast-path) → Phase-3 (throughput data).
- **mutation methods** (`submit_*`/`approve`/`deny`/`preview`) → Phase 2.
- **subscribe multi-subscription correlation** — the ack carries no `subscription_id` handle → 1.6 design if multiple concurrent subscriptions per connection are needed.
- **richer reads** — ProjectGraph edges (node-only today), pagination (`page` provisional), a column-allowlist (vs `SELECT *`) → later.

## TDD compliance
**Clean for all four layers.** Each was strict RED → GREEN; RED confirmed for the right reason every layer (missing module / missing types / signature-arity / serde tag). No Step-2.5 re-send needed beyond the upfront design + the L4 design checkpoint (the frame-type/subscribe-scope decisions, confirmed before RED).
- **Coverage-strengthening tests added post-review (NOT implement-before-test):** the at-cap encode (L1), scope-non-enforcement (L3), `test_subscribe_method_recognized` (L4), `test_server_frame_roundtrips_from_wire` (L4) — each pins already-correct behavior surfaced by a code-quality finding; the features themselves were test-first.
- **Ratified contract correction:** the `protocol_error` fold re-pinned test 7 (a lead-ratified §6.4 change folded mid-round).
- **Cross-slice fixture edits:** the `CONTRACT_VERSION` pin (`envelope.rs`) updated 0.8.0→0.11.0 across the bumps; `client_session` updated to unwrap `ServerFrame` (L4 envelope retrofit). Legitimate (version bump + envelope wrap), not implementation-chasing.

## Reachability
All 1.5 surfaces are `pub` mechanisms reachable from `tests/ipc.rs`; **none has a production entry point yet** (honest "ship the mechanism, wire the runtime at 1.6", the 1.3/1.4 precedent):
- **`serve_connection`** (auth → handshake → dispatch loop), **`encode_frame`/`decode_len`/`read_frame`/`write_frame`**, **`authorize_peer`/`peer_uid`**, **`push_subscription`** → the production caller is the **1.6 bootstrap cold-start ordering** (§16 "bind UDS" → accept-loop spawn → `getpeereid` → `serve_connection` on a blocking task). The real out-of-process consumer is the **ui** (`MockGatewayPort`→`UdsGatewayPort`) post-1.6.
- **subscribe live delta source** (EventStore.append → broadcast → subscriber) → 1.6 (joins the accept-loop spawn).

No tested-but-silently-unreachable gaps — every deferral names its consumer phase.

## Open follow-ups
**Orchestrator-owned (routed hot; land at `/orchestrate-end`):**
- **Cross-doc (NEW cross-language contract — NOT drift):** Appendix A **`GatewayPort`** row (§6.1 read surface: get_projection/get_capabilities/subscribe live; mutation→P2) + the **§6.4 IPC-wire** row (framing/handshake/version/error-codes incl. `protocol_error` + the frame-type multiplexing, Terminal reserved); `daemon/CLAUDE.md` + `ui/CLAUDE.md` cross-doc rows for the IPC/`protocol_version` contract; the §5.0 artifact @ **0.11.0**; the §6.4 `protocol_error` ratification edit; resolve Carry-forward `ui↔daemon-1.5` part (a) — `SUPPORTED_PROTOCOL_RANGE {1,1}` confirmed daemon-authored (keep the spread for the ui-side `MockGatewayPort`→real swap).
- **LESSON §7 candidate** — the UDS transport (length-prefix bounded pre-alloc; getpeereid peer-auth = primary gate not SO_PEERCRED, socket perms defense-in-depth; handshake-first + reject-unknown; read-only-WAL read methods + closed-enum table map = no SQL injection); the **two version axes**; the **internally-tagged `ServerFrame` frame-type multiplexing**.

**Future TODO — the 1.6 IPC-runtime bundle:** `bind()` + accept-loop spawn (§16); the **live subscribe delta-source** (EventStore.append→broadcast→subscriber) + the dispatch→push routing (`try_clone` read/write split); the **platform cfg-guard** (getpeereid is macOS/BSD-only — gate the UDS stack to the macOS-MVP target); the accept-loop **concurrency cap** (bound concurrent 8 MiB buffers + live connections); **one read-only-WAL conn per session** (today per `get_projection`); a **subscribe `subscription_id`/correlation handle** if multi-subscription-per-connection.

**Future TODO — Phase 2:** the mutation method surface (`submit_*`/`approve`/`deny`/`preview`) extends this transport + the gateway pipeline.

**Future TODO — Phase 3:** the Terminal-Channel frame encoding.

**Future TODO — post-MVP:** `get_projection` scope filtering (the WHERE + `ProjectId` newtype); ProjectGraph edges; pagination; a column allowlist.

## How to use what was built
The 1.6 bootstrap, in cold-start order, calls `PidLock::acquire` (1.4) → migrates the DB → binds the UDS socket → spawns a Tokio accept-loop: per connection, read the peer uid via `peer_uid(fd)`, then `tokio::task::spawn_blocking(move || serve_connection(stream, peer_uid, daemon_uid, &db_path))`. `serve_connection` runs the rule-#7 auth gate, the §6.4 handshake, then the §6.1 read/serve loop. For subscribe, the runtime feeds `push_subscription` from the EventStore delta broadcast over a `try_clone`'d write half. The ui swaps `MockGatewayPort`→`UdsGatewayPort` and reads projections over the socket.

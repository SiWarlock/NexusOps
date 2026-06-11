# IPC — UDS GatewayPort transport (`daemon/src/ipc/`)

## Executive summary

This layer is the daemon's front door. The desktop UI cannot touch the daemon's database or internals directly — instead it connects to a Unix domain socket (a local, file-like network endpoint at `~/Library/Application Support/NexusOps/gateway.sock`) and speaks a small JSON-RPC dialect over it. Before answering anything, the daemon checks *who* is on the other end of the socket (same OS user as the daemon, or rejected), then performs a version handshake, and only then serves requests. Today the surface is deliberately **read-only**: clients can fetch projection tables, ask for capabilities, and subscribe to live change notifications. The mutation methods (`submit_action`, `approve`, `deny`, `preview_action`) that the architecture defines for this port are Phase-2 work and are absent on purpose — there is currently no way to change anything through this socket.

## Responsibilities

- **Accountable for:** the wire framing (4-byte length prefix + JSON), the anti-DoS frame-size bound, `getpeereid()` peer authentication (safety rule #7), the `HelloFrame` handshake + protocol-version skew rejection, dispatch of the three read methods (`get_projection` / `get_capabilities` / `subscribe`), and the live subscription push stream with its close-on-lag resync policy.
- **NOT responsible for:** mutations of any kind (no `submit_*`/`approve`/`deny` — Phase 2, `daemon/src/ipc/methods.rs:44-48` returns `unknown_method` for them); writing the database (reads use read-only WAL connections only, `daemon/src/ipc/methods.rs:102-104`); binding the socket or running the accept loop (that lives in the runtime layer, `daemon/src/runtime/listener.rs`); producing the deltas it pushes (the write-actor publishes those post-commit); the wire *types* themselves (authored in `shared/src/ipc.rs`).

## Key components

| Component | What it does | Where |
|-----------|--------------|-------|
| `MAX_FRAME_SIZE` (8 MiB) | Fixed anti-DoS cap on a single frame's JSON body | `daemon/src/ipc/transport.rs:16` |
| `encode_frame` / `write_frame` | 4-byte big-endian length prefix + JSON body; refuses oversized bodies symmetrically | `daemon/src/ipc/transport.rs:20-32`, `:65-69` |
| `decode_len` | Validates the declared length from the 4-byte prefix **before** any allocation | `daemon/src/ipc/transport.rs:37-48` |
| `read_frame` | Production read path — `decode_len` fires before the body buffer is sized | `daemon/src/ipc/transport.rs:54-61` |
| `authorize_peer` | Pure rule-#7 decision: peer uid must equal daemon uid | `daemon/src/ipc/peer.rs:16-25` |
| `peer_uid` | The isolated unsafe `getpeereid(2)` FFI; fail-closed on syscall error | `daemon/src/ipc/peer.rs:39-54` (syscall `:46`, fail-closed `:47-49`) |
| `current_euid` | The daemon's own euid (the authority peers are compared against) | `daemon/src/ipc/peer.rs:30-34` |
| `serve_connection` | Per-connection handler: auth FIRST → handshake → RPC loop → subscribe dedication | `daemon/src/ipc/server.rs:31-153` |
| `methods::dispatch` | Routes `get_capabilities` / `get_projection` / `subscribe`; everything else → `unknown_method` | `daemon/src/ipc/methods.rs:43-62` |
| `projection_table` | Closed `ProjectionName` → `proj_*` table map (compile-time constants, no SQL injection surface) | `daemon/src/ipc/methods.rs:25-38` |
| `get_projection` | `SELECT *` over a **read-only WAL** connection → JSON row array | `daemon/src/ipc/methods.rs:90-107` |
| `push_subscription` | Writes each `ProjectionDelta` as a `ServerFrame::SubscriptionPush` frame | `daemon/src/ipc/subscribe.rs:22-34` |
| `next_push_action` | Pure classifier: matching delta → Push; other projection → Skip; `Lagged`/`Closed` → Stop | `daemon/src/ipc/subscribe.rs:53-63` |
| `run_push_loop` | Drives the push thread; `shutdown(Both)` on any exit so the client sees EOF and resyncs | `daemon/src/ipc/subscribe.rs:70-89` |
| `IpcError` | Daemon-internal typed transport errors (distinct from the `shared/` wire error codes) | `daemon/src/ipc/mod.rs:37-65` |
| Wire types (`HelloFrame`, `ServerFrame`, …) | The cross-language §6.1/§6.4 contract | `shared/src/ipc.rs` (details below) |
| `bind` + `spawn_accept_loop` (adjacent, runtime layer) | Unlink-before-bind, 0600 perms, semaphore-capped accept loop that calls into this layer | `daemon/src/runtime/listener.rs:29-40`, `:48-117` |

Tests: 17 integration tests in `daemon/tests/ipc.rs` (layered L1 framing/auth → L4 subscribe, see the map at `daemon/tests/ipc.rs:8-12`) + 3 inline unit tests in `daemon/src/ipc/subscribe.rs:91-183`.

## Interfaces & contracts

**The GatewayPort contract vs. this transport.** ARCHITECTURE.md §6.1 defines `GatewayPort` as the method contract (8 methods) that all out-of-daemon callers reach **only** over the §6.4 UDS transport. Callers depend on the *contract*, not the transport: the UI codes against its own `GatewayPort` interface (`ui/src/gateway-client/types.ts:35-50`) and currently injects a `MockGatewayPort` (`ui/src/shell/Shell.tsx:114`); the planned `UdsGatewayPort` swaps in behind the same interface (`daemon/src/ipc/mod.rs:6-8`). A future remote client (iOS) would reuse the contract + typed intents over a different transport — the UDS framing is not the contract. **Note:** §6.1 describes a Rust trait implemented in the daemon; no Rust `trait GatewayPort` exists in the daemon yet — the read surface is served by free functions (`serve_connection` + `methods::dispatch`). The trait presumably lands with the Phase-2 Action Gateway. UNVERIFIED whether Phase 2 will literally introduce a trait or keep function dispatch.

**Handshake (must happen before any method):**
- Client sends `HelloFrame { protocol_version, client_kind, app_version }` (`shared/src/ipc.rs:30-36`).
- In-range version → `HelloAck { protocol_version, daemon_version, capabilities }` (`shared/src/ipc.rs:40-46`; written at `daemon/src/ipc/server.rs:74-84`).
- Out-of-range → `VersionSkewError { supported_min, supported_max, client_protocol_version }` + disconnect (`shared/src/ipc.rs:50-56`; `daemon/src/ipc/server.rs:54-72`).
- Supported range is `{min:1, max:1}`, daemon-authored (`shared/src/ipc.rs:16-26`), matching the UI's pinned `SUPPORTED_PROTOCOL_RANGE`.

**Two version axes — do not conflate** (`shared/src/ipc.rs:7-9`): `protocol_version` is the wire-handshake skew check; `CONTRACT_VERSION` is the §5.0 schema/codegen artifact (currently `0.14.0`; the IPC contract itself landed across bumps 0.9.0 → 0.11.0, `shared/src/lib.rs:19-32`). They move independently.

**Method surface (read-only, §6.1 subset):**

| Method | Input → output | Where |
|---|---|---|
| `get_capabilities` | → `Capabilities { protocol_version, contract_version }` | `daemon/src/ipc/methods.rs:78-85` |
| `get_projection` | `GetProjectionParams { name, scope? }` → JSON array of rows | `daemon/src/ipc/methods.rs:90-107` |
| `subscribe` | `SubscribeParams { projection, filter? }` → ack `{ "subscribed": <name> }`, then live `SubscriptionPush` frames | `daemon/src/ipc/methods.rs:68-76`, `daemon/src/ipc/server.rs:115-145` |
| anything else | → structured `unknown_method` error | `daemon/src/ipc/methods.rs:48` |

**Error contract** — 7 closed snake_case codes (`shared/src/ipc.rs:70-80`): `version_skew, frame_too_large, unknown_method, unauthorized_peer, policy_denied, precondition_stale, protocol_error`. Client errors (unknown method, bad params) come back as structured `WireError` responses and the connection continues; protocol violations (non-Hello first frame, malformed frame) get a `protocol_error` frame then disconnect; infrastructure read failures disconnect (`daemon/src/ipc/methods.rs:5-7`, `daemon/src/ipc/server.rs:43-52`, `:98-105`).

**Server→client frame mux:** every server frame after the ack is an internally-tagged `ServerFrame` — `frame_type: "rpc_response" | "subscription_push"` (`shared/src/ipc.rs:219-224`). The Terminal-Channel tag space is **reserved, no variant** — its encoding (JSON-base64 vs binary fast-path) is a Phase-3 decision with throughput data (`shared/src/ipc.rs:217-218`; ARCHITECTURE.md §6.4).

## Data & state

This layer owns **no persistent state**. Everything it serves is derived:

- **Projection reads** open a fresh read-only WAL connection per request (`crate::eventstore::open_read_only`, `daemon/src/ipc/methods.rs:102-104`) and `SELECT *` from one of ten compile-time-mapped `proj_*` tables (`daemon/src/ipc/methods.rs:25-38`). The single-writer invariant (forbidden #3) is preserved — this layer never opens a writable connection.
- **Wire types** live in `shared/src/ipc.rs` as the cross-language contract (consumed by generated Zod on the UI side): all structs are `deny_unknown_fields` (reject-unknown, §15); `ProjectionName` is a closed 10-variant enum with **PascalCase wire values — intentionally no `rename_all`** (`shared/src/ipc.rs:106-120`); `DeltaKind` is `upsert|remove` (`shared/src/ipc.rs:193-198`); `ProjectionDelta { projection, kind, row?, id? }` (`shared/src/ipc.rs:202-213`) — note it carries **no sequence number**, which is why a lagged subscriber can't detect a gap (see Gotchas).
- **Per-connection transient state:** the blocking `UnixStream`, and (for subscribe connections) one `broadcast::Receiver<ProjectionDelta>` + one detached push thread.
- SQLite values are mapped to JSON conservatively; a BLOB (which `proj_*` tables shouldn't contain) becomes a `"<N bytes>"` placeholder, never raw bytes (`daemon/src/ipc/methods.rs:133-142`).

## Dependencies

- **Depends on:** `shared/src/ipc.rs` (the wire contract — every frame type); `eventstore::open_read_only` (`daemon/src/ipc/methods.rs:103-104`) for projection reads; `tokio::sync::broadcast` receivers minted from the write-actor's post-commit delta sender (`daemon/src/ipc/server.rs:117`); `libc` for the two isolated unsafe syscalls (`daemon/src/ipc/peer.rs:33`, `:46`).
- **Used by:** the runtime accept-loop — `spawn_accept_loop` reads the peer uid via `peer_uid` and calls `serve_connection` on a blocking task (`daemon/src/runtime/listener.rs:100-111`), wired from `main.rs:80-91` with `current_euid()` as the daemon uid and a 64-connection semaphore cap (`daemon/src/main.rs:32`). The UI's gateway-client is the intended remote consumer (today still `MockGatewayPort`; see Gotchas).

## How it works (flow)

```
main.rs:82 bind(gateway.sock)            listener.rs:29-40  unlink-before-bind, 0600 perms
        │
        ▼
spawn_accept_loop                        listener.rs:48-117
  semaphore permit (cap 64, refuse at cap)      :73-76
  tokio stream → blocking std stream            :86-97
  peer_uid(fd) via getpeereid  ── fail ──► drop (fail-closed)   :100-106
        │
        ▼
serve_connection (blocking task)         server.rs:31-153
  1. authorize_peer FIRST  ── uid≠daemon ──► disconnect, zero frames read   :38-39
  2. first frame MUST be HelloFrame ── else protocol_error + disconnect    :42-52
  3. version out of range ──► VersionSkewError + disconnect                :54-72
  4. HelloAck { capabilities }                                             :74-84
  5. RPC loop: read_frame → RpcRequest → dispatch → ServerFrame::RpcResponse
     (EOF = clean close; malformed frame = protocol_error + disconnect)    :91-151
  6. subscribe accepted ──► connection becomes DEDICATED:                  :115-145
       rx minted BEFORE the ack (no missed delta)                          :117
       ack written by this thread, THEN push thread spawned                :118-128
       main thread blocks on one read (writes nothing more)               :134
       run_push_loop: Push matching / Skip other / Stop on Lagged|Closed
         → shutdown(Both) on any exit                  subscribe.rs:70-89
```

Step-by-step on the hot path: the auth gate runs before any frame is read, so a foreign uid gets zero bytes served (pinned by `daemon/tests/ipc.rs:115-140` — the test proves auth-before-read because the handler would otherwise hang on the absent HelloFrame). Frame reads bound the declared length from the prefix *before* allocating the body buffer (`daemon/src/ipc/transport.rs:57-58`), so a hostile 4-GB declared length never drives an allocation. After a successful `subscribe` ack the connection is structurally single-writer: the push thread (spawned `server.rs:128`) is the sole writer for the connection's remaining life, and any further client request causes the server to close the connection rather than write a racing second response (`server.rs:134-139`, pinned by `daemon/tests/ipc.rs:747-810`).

## Design decisions & rationale

- **4-byte BE length-prefix + JSON, one framing** — locked by ADR-004 (ARCHITECTURE.md §6.4; newline framing dropped). Pre-allocation rejection at `MAX_FRAME_SIZE` = 8 MiB is the anti-DoS pin; UI outbound frames are small and large responses are (eventually) paginated (`daemon/src/ipc/transport.rs:12-15`).
- **`getpeereid()` not `SO_PEERCRED`** — safety rule #7 / ARCHITECTURE.md §15 (line 362): `SO_PEERCRED` is Linux-only; this daemon is macOS-MVP. Socket 0600 perms are defense-in-depth, **not** the primary gate (`daemon/src/runtime/listener.rs:37-38`, `daemon/src/ipc/peer.rs:4-7`). The unsafe FFI is isolated to one function and fail-closed both at runtime (non-zero rc → no uid → never authorized, `peer.rs:47-49`) and at compile time (a wider-`uid_t` target would fail to build rather than truncate, `peer.rs:50-53`).
- **Handshake-first, two version axes** — `protocol_version` catches wire skew at connect time so an old UI gets a renderable "update required" (`VersionSkewError`); `CONTRACT_VERSION` rides `Capabilities` for schema-level agreement (§5.0). Recorded in §6.4's IMPLEMENTED-1.5 note (ARCHITECTURE.md:192-195).
- **`protocol_error` as a distinct code** — the LOCKED §6.4 error set had no code for "bad first frame / malformed frame / bad params" vs. a genuinely unknown method name; the gap was escalated and lead-ratified (Option B) rather than silently reusing `unknown_method` (`shared/src/ipc.rs:67-69`; ARCHITECTURE.md §6.4).
- **Read-only surface only** — INV-SEC-1: until the Phase-2 Action Gateway exists, exposing mutation methods would create an unaudited mutation path. Their absence from `dispatch` (`methods.rs:44-48`) is the enforcement.
- **Close-on-lag instead of silently continuing** — `ProjectionDelta` carries no seq, so a dropped delta is an *undetectable* gap; continuing would silently diverge the client. `Lagged → Stop → close` forces the client to reconnect + re-`get_projection` from a consistent snapshot (`daemon/src/ipc/subscribe.rs:43-47`; LESSONS §9/§12). The broadcast channel is non-blocking, so a slow subscriber can never back-pressure the write-actor.
- **Dedicated subscribe connection** — multiplexing RPC responses and pushes on one connection with two writers would interleave and corrupt the frame stream; MVP makes a subscribe connection terminal/push-only (1 subscription per connection; a `subscription_id` mux is explicitly deferred, `daemon/src/ipc/server.rs:106-114`; LESSONS §12).
- **Pure classifier for push policy** — `next_push_action` is I/O-free so the back-pressure policy is unit-testable without timing flakiness (`subscribe.rs:36-37`, tests `:108-134`).
- **Closed-enum table map** — `ProjectionName → proj_*` is a compile-time `match`, never client-supplied text interpolated into SQL (`methods.rs:21-24`).

## Gotchas & sharp edges

- **`scope.project_id` is accepted but NOT enforced.** `get_projection` returns the full table regardless of scope; this is documented in the contract (`shared/src/ipc.rs:160-165`) and deliberately **test-pinned** so future enforcement is a conscious change, not a silent surprise (`daemon/tests/ipc.rs:469-501`). Don't build client logic that assumes server-side scoping.
- **Drift vs §6.1's method table:** the architecture specifies `get_projection(name, scope, page) → ProjectionPage`; the implementation takes no `page` param and returns a bare JSON row array, not a page envelope (`shared/src/ipc.rs:160` marks pagination "provisional and omitted for MVP"; `methods.rs:112-131`). Large projections therefore come back whole — bounded only by `MAX_FRAME_SIZE`.
- **Two of the seven error codes are currently dead on the wire.** `policy_denied` and `precondition_stale` exist in the closed contract enum (`shared/src/ipc.rs:77-78`) but nothing in the daemon emits them — they belong to the Phase-2 mutation surface. `frame_too_large` and `unauthorized_peer` are likewise enforced by disconnection (typed `IpcError`s), not by structured wire frames, on the current paths.
- **The real UDS client doesn't exist yet.** The daemon side is live, but the UI still instantiates `MockGatewayPort` (`ui/src/shell/Shell.tsx:114`); `UdsGatewayPort` is named in comments only (`ui/src/main.tsx:27`). The end-to-end socket path is exercised by daemon integration tests, not by the shipping UI.
- **No Rust `GatewayPort` trait yet** — §6.1 calls it "a Rust trait implemented in the daemon process"; current code is `serve_connection` + function dispatch. Expected to materialize with Phase 2; flagged here as doc-vs-code drift until it does.
- **Terminal Channel is reserved tag space only.** §6.4 specifies terminal output/input/backpressure frames over this socket; `ServerFrame` deliberately has no variant (`shared/src/ipc.rs:217-218`) — encoding is a Phase-3 decision.
- **macOS-only peer auth.** The `getpeereid` stack has no Linux cfg-guard; portability is a reviewer-sanctioned deferred item with no Linux CI to verify against (`daemon/src/runtime/listener.rs:10-12`).
- **A 0-length body is codec-valid.** `decode_len` accepts 0; semantic rejection of an empty/non-JSON body is the handshake/parse layer's job (`transport.rs:39-40`).
- **Subscribe ordering matters and is load-bearing:** the broadcast receiver is minted *before* the ack is written (`server.rs:117`), so a delta published immediately after the ack is never missed; the push thread is spawned only on ack success. Closing uses `shutdown(Shutdown::Both)`, not just drop — the push thread's `try_clone`'d fd would otherwise keep the socket half-open (`server.rs:135-138`).
- **At the connection cap, new connections are silently dropped** (refused, not queued — `listener.rs:71-76`); a client sees an immediate close with no error frame.

## Connects to

- [01-shared-contracts.md](01-shared-contracts.md) — every frame type this layer reads/writes is authored in `shared/src/ipc.rs` (handoff: `daemon/src/ipc/server.rs:17-20` imports; CONTRACT_VERSION history `shared/src/lib.rs:19-32`).
- [02-event-store.md](02-event-store.md) — projection reads go through `eventstore::open_read_only` (`daemon/src/ipc/methods.rs:103-104`); the subscribe deltas originate from the write-actor's post-commit broadcast.
- [04-projections.md](04-projections.md) — `get_projection` reads the ten `proj_*` tables those projectors fold (`daemon/src/ipc/methods.rs:25-38`).
- [05-locks.md](05-locks.md) — the pidlock's single-instance guarantee is what makes unlink-before-bind safe (`daemon/src/runtime/listener.rs:3-4`).
- [07-daemon-runtime.md](07-daemon-runtime.md) — `bind` + `spawn_accept_loop` host this layer (`daemon/src/main.rs:80-91`, `daemon/src/runtime/listener.rs:48-117`); the runtime hands in the daemon uid, db path, delta sender, and shutdown watch.
- [08-ui.md](08-ui.md) — the consuming side of the contract: the UI's `GatewayPort` interface (`ui/src/gateway-client/types.ts:35`) and its pinned `SUPPORTED_PROTOCOL_RANGE {1,1}` / `Capabilities` shapes are what the handshake (`daemon/src/ipc/server.rs:74-84`) answers to.

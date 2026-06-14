# /tdd brief — uds_read_client_crate

## Feature
**L1 read transport, slice 1 of ~3 (the transport core).** A standalone Rust crate
`ui/gateway-uds/` — the **UDS read-client**: the §6.4 frame codec (4-byte-BE-len + JSON,
8 MiB bound), the handshake (`HelloFrame`→`HelloAck`, version-skew), and the single-shot
read RPC (`RpcRequest` → read `ServerFrame` → demux `RpcResponse` → result / `WireError`)
for the **read methods** `get_projection` / `get_diff` / `get_capabilities`. **Pure Rust,
fully TDD-able against a fake stream — NO Tauri, NO running daemon** (the real `UnixStream`
connect is a thin gated integration adapter). It seeds from the daemon's `nexusopsd smoke
dev-client` reference. **NON-cat-1** (reads only — no mutation, no `submit_action`/`approve`/`deny`).

> **Why a standalone crate (de-risks the greenfield):** `ui/src-tauri/` does NOT exist yet
> (Tauri is unbuilt). Isolating the deterministic transport core (codec/handshake/demux) in a
> plain Rust crate makes it TDD-able NOW without standing up the Tauri toolchain — which is
> **slice 050** (the Tauri host + the command bridge + the TS `UdsGatewayPort` + the Shell
> read-path swap). **051** = the streaming `subscribe` + the reconnect→re-subscribe→re-`get_projection`
> recovery. This slice builds + reviews the transport core in isolation (the 043 foundation-first pattern).

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport — go-live; L1 read transport, slice 1 of ~3; ownership ui-track-confirmed)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.4` (the IPC wire contract — the codec, the handshake, the `ServerFrame` frames, the error codes), `§6.1` (the `GatewayPort` read methods), `§5.0` (the frozen `shared/` contract types the client (de)serializes).
- **Widens phase scope because** it builds the ui-side client of the daemon's `§6.1`/`§6.4`/`§5.0`
  wire contract — the live-transport go-live the runway assessment identified
  (`docs/planning/ui-post-p4-runway-assessment.md`); the lead GO'd L1 (reads-only, non-cat-1).
- **Reference (SEED FROM THIS — do NOT reverse-engineer the codec):** the daemon's
  **`nexusopsd smoke dev-client`** (`daemon/src/smoke.rs`, the `call()` fn) — the canonical
  synchronous wire flow: `UnixStream::connect` → write `HelloFrame` → read `HelloAck` → write
  `RpcRequest{method,params,id}` → read `ServerFrame` → match `RpcResponse{result|error}`. The
  codec is `daemon/src/ipc/transport.rs` (`encode_frame`/`decode_len`, `MAX_FRAME_SIZE = 8 MiB`,
  4-byte-BE-len). **Route any wire/protocol question to the daemon orchestrator — don't guess.**
- **Related context:** `docs/planning/ui-post-p4-runway-assessment.md` (the L1/L2 phasing); the
  frozen frame types in `shared/src/ipc.rs` (`nexusops-shared` crate).

## The frozen wire contract this implements (verified — `shared/src/ipc.rs`)
- **Socket path:** `$HOME/Library/Application Support/NexusOps/gateway.sock` (`smoke.rs` `socket_path()`).
- **Codec:** 4-byte **big-endian** length prefix + JSON body; **`MAX_FRAME_SIZE = 8 MiB`** — the declared length is validated **before** allocating the body (`decode_len`); a body over the cap → a typed `FrameTooLarge` error (symmetry on the write side).
- **Handshake:** write `HelloFrame{ protocol_version: 1, client_kind, app_version }` → read `HelloAck{ protocol_version, daemon_version, capabilities: Capabilities{ protocol_version, contract_version } }`. A `VersionSkewError{ supported_min, supported_max, client_protocol_version }` (out-of-range) → a typed error (don't proceed). `PROTOCOL_VERSION = 1`, range `{1,1}`.
- **RPC:** `RpcRequest{ method: String, params: Value, id: u64 }` → read `ServerFrame`; **demux `ServerFrame::RpcResponse(RpcResponse{ id, result: Option<Value>, error: Option<WireError> })`** — match the `id` to the request; `result` Some → the value; `error` Some (`WireError{ code: IpcErrorCode }`) → a typed client error carrying the verbatim §6.4 code (10 codes incl. `not_found`). A non-`RpcResponse` frame (`SubscriptionPush`/`TerminalOutput`) on a single-shot read → a protocol error.
- **Read methods (the L1 subset):** `get_projection(name, scope?, page?)` · `get_diff(worktree_id, file)` · `get_capabilities()` (the handshake already returns `Capabilities`, but the method is in the surface). **NOT** `subscribe` (streaming — slice 051) and **NOT** the mutation methods (L2).
- **Crate dep:** `nexusops-shared` (the frozen frame TYPES — `HelloFrame`/`RpcRequest`/`ServerFrame`/`RpcResponse`/`WireError`/`Capabilities`/`SubscribeParams`/`GetDiffParams`; serde with `deny_unknown_fields` = the Rust-side parse-don't-trust). **NO `tauri` dep, NO `daemon` dep** (the ui must not pull the daemon crate).

## Acceptance criteria (what "done" means)
- [ ] A new crate `ui/gateway-uds/` (workspace member) with **no `tauri`/`daemon` dependency** — only `nexusops-shared` + `serde_json` (+ `thiserror` for the typed error, optional).
- [ ] **The codec** (`encode_frame`/`decode_frame` over a generic `io::Write`/`io::Read`): 4-byte-BE-len + body; the **8 MiB bound enforced before alloc** on read; round-trips; an oversized declared length → a typed `FrameTooLarge` (never allocates).
- [ ] **The handshake** (`handshake(stream)`): writes `HelloFrame{protocol_version:1,…}`, reads + validates `HelloAck`; a `VersionSkewError` / a non-`HelloAck` first frame → a typed error (fail-closed, never proceeds to RPC).
- [ ] **The single-shot call** (`call(stream, method, params, id) → Result<Value, ClientError>`): writes `RpcRequest`, reads one `ServerFrame`, demuxes `RpcResponse` by `id`; `result` → `Ok(value)`; `WireError` → `Err(ClientError::Wire(code))` (the verbatim §6.4 code); a non-`RpcResponse` frame or an id-mismatch → `Err(ClientError::Protocol)`.
- [ ] The deterministic logic (codec/handshake/demux/call) is exercised over a **fake bidirectional in-memory stream** (e.g. a `Cursor`/a paired buffer) — no real socket, no daemon, no Tauri.
- [ ] A thin `connect_and_call(...)` adapter over the real `UnixStream` + the socket path (the smoke.rs one-shot pattern: connect → handshake → call → drop) — exercised by a **`#[ignore]`/feature-gated integration test** (needs a running daemon; documented, not run in the default suite).
- [ ] Typed read helpers `get_projection`/`get_diff`/`get_capabilities` forming the params (the JSON the daemon expects — match `methods.rs`).
- [ ] `cargo test -p nexusops-gateway-uds` green; `cargo clippy -p nexusops-gateway-uds -- -D warnings` clean; the workspace builds (`cargo check`).
- [ ] **`security-reviewer` REQUIRED** (the transport boundary — untrusted daemon input, even reads-only): verify the 8 MiB frame bound (pre-alloc), the `deny_unknown_fields` serde parse, the `WireError`/`ServerFrame` demux reject-unknown, the typed-error fail-closed paths, the socket path resolution.
- [ ] Cross-doc flagged at Step 9 (a new `ui/CLAUDE.md` row: the UDS read-client crate / the live-transport L1 foundation).

## Wiring / entry point (Step 7.5)
**none in production yet — wiring lands in slice 050.** The `ui/gateway-uds` crate is the
transport core, **exposed-ahead-of-consumer** (the 043 foundation-first pattern): slice 050
stands up `ui/src-tauri/` (the Tauri host) + a `#[tauri::command]` read bridge that calls this
crate + the TS `UdsGatewayPort` (`ui/src/gateway-client/uds.ts`) that invokes the bridge + the
Shell read-path swap (`MockGatewayPort`→real). The `#[ignore]` integration test (against a real
daemon) is the only live exercise this slice. Flag at Step 7.5 as **expected, not a wiring miss**.

## Files expected to touch
**New:**
- `ui/gateway-uds/Cargo.toml` — the crate manifest (member; deps `nexusops-shared`, `serde_json`, `thiserror?`; NO tauri/daemon).
- `ui/gateway-uds/src/lib.rs` — the codec + handshake + single-shot call + the typed `ClientError` + the read helpers (split into `codec.rs`/`client.rs` if it reads cleaner — implementer's call).
- `ui/gateway-uds/tests/integration.rs` — the `#[ignore]`/feature-gated real-`UnixStream` round-trip (documented).

**Modified:**
- `Cargo.toml` (root) — add `ui/gateway-uds` to `[workspace] members`.

If implementation needs files beyond this list (e.g. a `codec.rs` split), **flag at Step 2.5**.

## RED test outline (Step 2)
Tests in `ui/gateway-uds/src/lib.rs` (`#[cfg(test)]`) over a fake stream:
1. **`frame_roundtrips`** — encode→decode preserves the body; a 4-byte-BE len. Why: §6.4 codec.
2. **`oversized_frame_rejected_before_alloc`** — a declared length > 8 MiB → `FrameTooLarge`, never allocates. Why: §6.4 anti-DoS bound (the security pin).
3. **`handshake_writes_hello_reads_ack`** — `handshake` writes a `HelloFrame{protocol_version:1}` + accepts a valid `HelloAck`. Why: §6.4 handshake-first.
4. **`handshake_version_skew_fails_closed`** — a `VersionSkewError` / a non-`HelloAck` first frame → a typed error, no RPC issued. Why: §6.4 version negotiation (fail-closed).
5. **`call_demuxes_rpc_response_result`** — a `ServerFrame::RpcResponse{id,result}` matching the request id → `Ok(value)`. Why: §6.1/§6.4 demux.
6. **`call_wire_error_returns_typed_code`** — a `RpcResponse{error: WireError{code}}` → `Err(ClientError::Wire(code))`, the verbatim §6.4 code. Why: §6.4 error semantics (no swallow).
7. **`call_non_rpcresponse_frame_is_protocol_error`** — a `SubscriptionPush`/`TerminalOutput` on a single-shot read, or an id-mismatch → `Err(ClientError::Protocol)`. Why: §6.4 frame discipline.
8. **`unknown_field_in_frame_rejected`** — a daemon frame with an extra field → a serde parse error (deny_unknown_fields). Why: §5.0 parse-don't-trust.

(The real-socket round-trip is the `#[ignore]` integration test, not a unit RED.)

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (the crate (de)serializes the FROZEN `shared/src/ipc.rs` types; no new contract). The codec is re-implemented from `transport.rs` (a thin, well-known 4-byte-BE+JSON — see Q1).
- **Orchestrator doc rows to write hot (Step 9):** a new `ui/CLAUDE.md` row — the `ui/gateway-uds` read-client crate (the live-transport L1 foundation; the codec/handshake/demux; deps `nexusops-shared` only). No `ARCHITECTURE.md` edit (daemon-authored §6.4, already frozen).
- **§2.5-seam model touched?** The crate is a CONSUMER of the frozen §6.4 frame contract — a serde-deserialize conformance (deny_unknown_fields) + the demux. No new shared model.

## Things to flag at Step 2.5
1. **The codec — re-implement vs share.** The codec lives in `daemon/src/ipc/transport.rs` (NOT `shared/`). **Default vote:** re-implement the trivial 4-byte-BE-len+JSON codec in `ui/gateway-uds` (seeded verbatim from `transport.rs`; depend on `nexusops-shared` for the frame TYPES only). Promoting the codec to `shared/` for single-sourcing is a daemon-track ask (cross-track) — flag it as a follow-on if you judge the duplication risky, but default to re-implement (the codec is ~15 lines + the types are already shared).
2. **The connection model.** **Default vote:** the smoke.rs **one-shot-per-call** pattern (connect → handshake → call → drop) for these single-shot reads — simplest, matches the reference, fine for the initial projection fetches. A **persistent** connection + the dedicated `subscribe` connection ride slice 051 (the streaming). Don't build connection pooling here.
3. **The typed `ClientError` shape.** **Default vote:** a `thiserror` enum `{ Connect(io), Frame(FrameError), Handshake(skew/protocol), Wire(IpcErrorCode), Protocol(msg) }` — the `Wire(IpcErrorCode)` carries the verbatim §6.4 code so the TS layer (050) maps it through `describeRejection`. Flag if you'd rather not pull `thiserror` (a hand-rolled enum is fine).
4. **`get_capabilities` redundancy.** The handshake already returns `Capabilities`. **Default vote:** keep a `get_capabilities()` helper anyway (the `GatewayPort` interface has it; cache the handshake's `Capabilities` or re-handshake) — the TS layer decides; expose both.

## Dependencies + sequencing
- **Depends on:** the boundary merge (0.28.0 `nexusops-shared` on `track/ui` — landed); the lead's L1 GO (reads-only). Nothing else.
- **Blocks:** slice **050** (the Tauri host + the command bridge + the TS `UdsGatewayPort` + the Shell read-path swap — consumes this crate) → **051** (the streaming `subscribe` + the reconnect recovery). The **L2 mutation transport** (cat-1) is HELD on the daemon's ②-mini approval-enrichment (CONTRACT 0.30.0) — a separate cat-1-checkpointed slice.

## Estimated commit count
**1–2.** One focused crate (codec + handshake + single-shot call + the read helpers). **NON-cat-1**
(reads only — no mutation path; INV-SEC-1 stays daemon-side) but **`security-reviewer` REQUIRED**
(the transport boundary is a new untrusted-daemon-input + the frame-bound surface). The implementer
MAY split the codec from the client if it reads cleaner; default 1 commit (cohesive).

## Lessons-logged candidates anticipated
- **Convention candidate** — likely: "the ui-side UDS transport core is a pure Rust crate (codec/handshake/demux over a generic stream — TDD'd against a fake stream, the real socket a thin gated adapter), depending on `nexusops-shared` for the frozen frame types (deny_unknown_fields) — never the daemon crate; the 8 MiB frame bound + the WireError demux are the boundary discipline." Surface at Step 9.
- **Architecture-doc note candidate** — the live-transport go-live begins; the read-client crate is the foundation (050 wires Tauri + TS; 051 the streaming).
- **Future TODO — next-brief working set** — 050 (Tauri host + bridge + TS `UdsGatewayPort` + Shell read-swap), 051 (subscribe streaming + reconnect recovery), the L2 mutation transport (cat-1, HELD on 0.30.0 ②-mini), the codec-to-`shared` promotion (if Q1 flags it).

## How to invoke
1. **Read this brief end-to-end** — especially the reference (`smoke.rs` `call()` — SEED from it) + the 4 Step-2.5 questions.
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree. Confirm `cargo` is available (this is a Rust crate). **You do NOT need the Tauri toolchain for this slice** (pure Rust).
3. **Run `/tdd uds_read_client_crate`.**
4. Step 0 (Restate) — confirm against the Feature line.
5. Step 1 (Identify files) — confirm against "Files expected to touch".
6. **Step 2.5** — answer the 4 design questions + send the test-design write-up (one `Asserts: <invariant> (§anchor)` line per test + the coverage map); wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
7. **Step 8** — `security-reviewer` REQUIRED (the transport boundary: the 8 MiB bound, deny_unknown_fields, the WireError demux, fail-closed paths).
8. Step 9 — surface the cross-doc flag + the transport-core lesson candidate + the codec-share decision (Q1).

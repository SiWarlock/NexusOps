# /tdd brief — uds_gatewayport_transport

## Feature
The daemon-side **UDS `GatewayPort` transport** — a Unix-domain-socket server speaking
**4-byte big-endian length-prefixed JSON-RPC**, with a `getpeereid()` peer-auth gate (reject
uid ≠ daemon-uid), a `HelloFrame`→`HelloAck`|`VersionSkewError` version handshake, and the
**read-only method surface** (`get_projection`, `subscribe`, `get_capabilities`). This is the
**real `UdsGatewayPort`** the ui's `MockGatewayPort` swaps to — it must satisfy the contract the
ui already pinned (`ui/src/gateway-client/types.ts`, `SUPPORTED_PROTOCOL_RANGE = {min:1,max:1}`).
Mutation methods (`submit_*`/`approve`/`deny`/`preview`) are Phase 2; the Terminal-Channel
frame-type multiplexing is a reserved seam.

## Use case + traceability
- **Task ID:** P1.5
- **Architecture sections it implements:** `§6.1` `[LOCKED — ADR-004]` (the `GatewayPort` JSON-RPC
  method surface — **read-only subset only:** `get_projection name,scope,page → ProjectionPage`,
  `subscribe {projection|events|terminal,filter} → stream handle`, `get_capabilities → Capabilities{protocol_version,…}`;
  `submit_*`/`approve`/`deny`/`preview` → Phase 2); `§6.4` `[LOCKED — ADR-004]` (the IPC wire
  contract — **4-byte big-endian length prefix + JSON body**, newline-framing dropped;
  `HelloFrame{protocol_version,client_kind,app_version}` → `HelloAck{protocol_version,daemon_version,capabilities}` |
  `VersionSkewError`; fixed `MAX_FRAME_SIZE`; error codes `version_skew, frame_too_large, unknown_method,
  unauthorized_peer, policy_denied, precondition_stale`; Terminal-Channel frame-type tag reserved); `§12` (the
  UDS accept-loop as a Tokio task).
- **Safety invariant:** root `CLAUDE.md` **Key safety rule #7 — "UDS peer-auth = `getpeereid()`
  (macOS), reject uid≠daemon-uid; socket perms are defense-in-depth (§15)."** ADR-004: `getpeereid()`
  **NOT `SO_PEERCRED`** (Linux-only); length-prefixed framing. ⚠️ security-reviewer required (`invariant`).
- **Consumer contract (the merged ui track — load-bearing):** the real transport must satisfy the
  `GatewayPort` interface the ui already built + pinned:
  - `ui/src/gateway-client/types.ts` — `get_projection(name,scope?,page?)`, `subscribe(params)`,
    `get_capabilities()` (the §6.1 read surface; mutation methods explicitly OUT, gated on 1.5).
  - `ui/src/connection/version.ts` — `SUPPORTED_PROTOCOL_RANGE = {min:1, max:1}` + `checkVersionCompat`
    (§6.4 HelloAck/VersionSkewError). **The daemon is the authoritative source of `protocol_version`**
    (§6.4) — confirm the range agrees at `{1,1}` for v1 (Q2).
  - `ui/src/shell/Shell.tsx` — the active ui path calls `get_projection("ProjectActivity"|"Session"|
    "PullRequest"|"ApprovalQueue"|"AuditTrail"|"Usage")` + `get_capabilities()` on load.
- **Related context:** reads the 1.2 `proj_*` tables over a **read-only WAL connection** (single-writer
  preserved — LESSON §3 / Forbidden #3; the write-actor stays sole writer). The UDS `bind()` + accept-loop
  *spawn* is **1.6-bootstrap-wired** (§16 cold-start: "…→ bind UDS"), like the 1.3 drainer / 1.4 reaper.
  Folds in the **Carry-forward `ui ↔ daemon-1.5 integration`** spread (`SUPPORTED_PROTOCOL_RANGE` reconcile
  + the `MockGatewayPort` → real swap).

## Acceptance criteria (what "done" means)
- [ ] **Framing:** a 4-byte big-endian length-prefix + JSON-body codec round-trips; a frame whose
      declared length exceeds **`MAX_FRAME_SIZE`** is rejected with `frame_too_large` (never allocated).
- [ ] **Peer-auth (the safety pin):** the server calls `getpeereid()` on accept and **rejects any peer
      whose uid ≠ the daemon-uid** with `unauthorized_peer` + disconnect (NOT `SO_PEERCRED`). Socket dir
      `0700` + socket `0600` are defense-in-depth (not the primary gate). (security-reviewer — rule #7.)
- [ ] **Handshake:** first frame must be `HelloFrame{protocol_version,client_kind,app_version}`; an
      in-range `protocol_version` → `HelloAck{protocol_version,daemon_version,capabilities}`; an
      out-of-range one → `VersionSkewError` + disconnect (no methods served pre-handshake). Daemon pins
      `protocol_version` per `SUPPORTED_PROTOCOL_RANGE` (Q2; reconciled to the ui's `{1,1}`).
- [ ] **Read methods:** `get_projection(name,scope,page)` returns the named projection's rows from the
      `proj_*` table (over a read-only WAL connection); `get_capabilities()` returns
      `Capabilities{protocol_version,…}`; an unknown method → `unknown_method`. The projection-name set is
      the canonical enum both sides share (Q3 — incl. the `Usage`↔`UsageLedger` reconcile); an unfed
      projection (body re-homed to a later phase) returns its empty table, not an error.
- [ ] **subscribe:** `subscribe({projection,filter})` returns a stream that pushes `ProjectionDelta`
      frames (frame-type-tagged) as new events fold the projection (Q4).
- [ ] **Reachability honest (Step 7.5):** the UDS server is a `pub` mechanism exercised by tests over a
      temp socket; the production `bind()`/accept-spawn is **1.6 bootstrap** — stated explicitly, not
      claimed live.
- [ ] **Single-writer preserved** — the IPC read path opens **read-only WAL** connections; no second
      writable `Connection` (Forbidden #3 / LESSON §3). All tests pass; `/preflight` clean; the
      `shared/contracts/ipc` schema regenerated + 3-way verified if `CONTRACT_VERSION` bumps (Q1); cross-doc
      rows updated atomic with the round (orchestrator writes).

## Wiring / entry point (Step 7.5)
- **The UDS server (`bind` + accept-loop)** — production caller is the **1.6 bootstrap cold-start
  ordering** (§16: "…create+migrate DB → register Device+LocalRunner → **bind UDS**"). For 1.5 the server
  is a `pub` mechanism; tests drive it over a temp socket with a test client (handshake + read). Say so —
  the same "pub primitive, consumer-wired later" honesty as the 1.3 drainer / 1.4 pidlock+reaper. The
  **real out-of-process consumer is the ui** (`MockGatewayPort` → `UdsGatewayPort`), which connects once
  1.6 binds the socket.

## Files expected to touch
**New:**
- `daemon/src/ipc/` — `mod.rs` + submodules as the impl sees fit (e.g. `transport.rs` framing/codec +
  `MAX_FRAME_SIZE`, `peer.rs` getpeereid, `handshake.rs`, `server.rs` accept-loop, `methods.rs` JSON-RPC
  dispatch + the read methods, `subscribe.rs`).
- `shared/src/ipc.rs` (or similar) — the **JSON-RPC method + error contract** (HelloFrame/HelloAck/
  VersionSkewError, the error-code enum, the projection-name enum, Capabilities) authored in Rust per §5.0
  → `schemars` → `shared/contracts/schema/`.
- `daemon/tests/ipc.rs` — integration tests + a test client (frame codec + handshake + read).

**Modified:**
- `shared/src/lib.rs` — `CONTRACT_VERSION` bump (0.8.0 → 0.9.0) **if Q1 says the IPC schema rides the §5.0
  SoT artifact**; `pub mod ipc`.
- `daemon/src/lib.rs` — `pub mod ipc`.
- `shared/contracts/schema/*.json` — regenerated; 3-way verify @ the new version (if bumped).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)

**Layer 1 — transport framing + peer-auth (`ipc/transport.rs`, `ipc/peer.rs`, `ipc/server.rs`):** ⚠️ safety-critical
1. **`test_frame_codec_roundtrip`** — Asserts: encode→decode of a JSON body preserves bytes; the 4-byte
   big-endian length prefix is honored. Why: §6.4 framing.
2. **`test_frame_too_large_rejected`** — Asserts: a frame whose declared length > `MAX_FRAME_SIZE` →
   `frame_too_large`, no oversized allocation. Why: §6.4 `MAX_FRAME_SIZE`.
3. **`test_wrong_uid_peer_rejected`** *(the peer-auth pin — rule #7)* — Asserts: a connection whose
   `getpeereid()` uid ≠ daemon-uid → `unauthorized_peer` + disconnect; same-uid accepted. Why: §15 / safety
   rule #7 (`getpeereid`, not `SO_PEERCRED`).

**Layer 2 — handshake + version negotiation + the `shared/contracts/ipc` schema (`ipc/handshake.rs`, `shared/src/ipc.rs`):**
4. **`test_handshake_hello_ack`** — Asserts: a `HelloFrame{protocol_version=1,…}` → `HelloAck{protocol_version,
   daemon_version,capabilities}`. Why: §6.4 handshake happy path.
5. **`test_version_skew_disconnects`** — Asserts: an out-of-`SUPPORTED_PROTOCOL_RANGE` `protocol_version` →
   `VersionSkewError` + disconnect; no method served. Why: §6.4 version_skew; matches the ui's
   `checkVersionCompat`/`SUPPORTED_PROTOCOL_RANGE {1,1}`.
6. **`test_method_before_handshake_rejected`** — Asserts: any method frame before a successful handshake →
   structured error + disconnect. Why: handshake-first invariant.

**Layer 3 — JSON-RPC dispatch + read methods (`ipc/methods.rs`):**
7. **`test_get_projection_returns_rows`** *(integration)* — Asserts: with the event store seeded (a
   `SessionStarted` folded by 1.2), `get_projection("Session",…)` over the read-only WAL connection returns
   the `proj_session` row(s). Why: §6.1 read surface; matches the ui `Shell` load path.
8. **`test_get_projection_unfed_is_empty_not_error`** — Asserts: a projection whose body is re-homed to a
   later phase (e.g. `PullRequest`→7.1) returns its empty table, not an error (the DDL exists from 1.2). Why:
   projection-name-set contract (Q3).
9. **`test_unknown_method_and_get_capabilities`** — Asserts: an unknown method → `unknown_method`;
   `get_capabilities()` → `Capabilities{protocol_version,…}`. Why: §6.1 dispatch + capabilities.

**Layer 4 — subscribe streaming + Terminal-Channel frame-type seam (`ipc/subscribe.rs`):**
10. **`test_subscribe_pushes_projection_delta`** *(integration)* — Asserts: a `subscribe({projection:"Session"})`
    stream pushes a `ProjectionDelta` frame when a new event folds `proj_session`. Why: §6.1 subscribe.
11. **`test_frame_type_tag_distinguishes_streams`** — Asserts: rpc-response vs subscription-push frames carry
    a distinguishing frame-type tag (the Terminal-Channel tag space is reserved, not yet served). Why: §6.4
    frame-type multiplexing seam.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW — the **GatewayPort method surface + IPC wire contract** (HelloFrame/HelloAck/
  VersionSkewError, the error-code enum, the projection-name enum, Capabilities, `protocol_version`). Unlike
  leases/outbox this **IS a cross-language contract surface** (the ui's `gateway-client` consumes it) → it
  rides the §5.0 SoT artifact and **likely bumps `CONTRACT_VERSION` (0.8.0 → 0.9.0)** + a 3-way verify.
  **Confirm at Step 2.5 (Q1).**
- **Orchestrator doc rows to write hot (Step 9):** Appendix A `GatewayPort` row (§6.1 — read subset
  implemented; mutation methods → P2) + the `§6.4` IPC-wire row; the `daemon/CLAUDE.md` + `ui/CLAUDE.md`
  cross-doc rows for the IPC/protocol_version contract; the projection-name enum + `protocol_version` in the
  §5.0 artifact; **resolve the Carry-forward `ui ↔ daemon-1.5 integration` spread (a)** (SUPPORTED_PROTOCOL_RANGE
  confirmed) — but **keep the spread** until the ui-side `MockGatewayPort`→real swap (b/c) lands; LESSON
  candidate (§7).

## Things to flag at Step 2.5
1. **IPC contract versioning — `protocol_version` vs `CONTRACT_VERSION` (the #1 question).** My default vote:
   **two axes.** The IPC method/error **schemas** are authored in `shared/` per §5.0 (so the ui's
   `gateway-client` gets generated validators) → **`CONTRACT_VERSION` bumps to 0.9.0** + 3-way verify. The
   **runtime handshake** compatibility uses a *separate* `protocol_version` (§6.4; pinned **=1**), matching
   the ui's `SUPPORTED_PROTOCOL_RANGE {min:1,max:1}`. (CONTRACT_VERSION = domain+IPC schema/codegen axis;
   protocol_version = wire-handshake skew axis.) Confirm — or argue for folding them.
2. **`SUPPORTED_PROTOCOL_RANGE` ownership + value.** The ui provisionally pinned `{min:1,max:1}`; §6.4 makes
   `protocol_version` **daemon-authored**. My default vote: the **daemon owns** the authoritative range,
   pins **`{min:1,max:1}` for v1** (agreeing with the ui's provisional pin → confirmed-not-provisional at
   this integration), authored in `shared/` so both sides read one constant. Confirm.
3. **Projection-name enum + `Usage`↔`UsageLedger` reconcile.** The ui calls `get_projection` with
   `ProjectActivity|Session|PullRequest|ApprovalQueue|AuditTrail|Usage`; the daemon tables are `proj_*`
   (incl. `proj_usage_ledger`). My default vote: author a **canonical closed projection-name enum in
   `shared/`** (both sides reject-unknown) and **reconcile the one mismatch** — pick `UsageLedger` (the §7
   registry name) or `Usage` (the ui's) as canonical and align the other side. An unfed projection (body
   re-homed to a later phase) returns its **empty table**, not an error. Confirm the canonical name.
4. **`subscribe` streaming scope + mechanism.** My default vote: implement `subscribe` as a frame-type-tagged
   push stream; the delta source hooks the event-commit (after `apply_all`, notify subscribers of the folded
   projection). The ui's *active* path today is `get_projection` (initial load), so L1–L3 unblock the ui
   integration and L4 completes the read surface. **subscribe stays in 1.5 scope** (it's in §6.1 + the task);
   if the streaming mechanism balloons, flag it — a scope cut **escalates**, never silent.
5. **Read path = read-only WAL (single-writer).** My default vote: the IPC server holds **read-only WAL**
   connection(s) for `get_projection`/`subscribe`; the write-actor stays the sole writer (Forbidden #3 /
   LESSON §3). The UDS accept-loop is a Tokio task; concurrent reads are read-only. Confirm.
6. **Wiring (Step 7.5).** My default vote: ship the server as a `pub` mechanism; the `bind()`/accept-spawn
   call site is **1.6 bootstrap** (§16 "bind UDS") — say so explicitly (the 1.3/1.4 deferral pattern), don't
   claim it live.

## Dependencies + sequencing
- **Depends on:** 1.1 event store (read-only WAL + `proj_*` via 1.2 — LANDED); the frozen `shared/` §5.0 SoT
  mechanism (the IPC schema extends it); the merged **ui `GatewayPort` contract** (`types.ts` +
  `version.ts`, `track/ui`→main `48d9931`).
- **Blocks:** **1.6** bootstrap (binds the UDS socket in cold-start); **Phase 2** (the mutation methods
  `submit_*`/`approve`/`deny`/`preview` extend this same transport + the gateway pipeline); the **ui ↔ daemon
  integration** (the ui swaps `MockGatewayPort` → the real `UdsGatewayPort`); the §25 demo (UI reads
  projections over UDS — demo step 7).
- **Interacts with:** the Carry-forward `ui ↔ daemon-1.5 integration` spread (this slice resolves part (a)
  SUPPORTED_PROTOCOL_RANGE; the ui-side swap (b/c) is cross-track-timed by the user — the spread stays until
  then).

## Estimated commit count
**4** (multi-commit slice; foundational IPC transport). L1 carries the **safety-critical** peer-auth pin
(rule #7) → its **own** commit + security-reviewer.
- **L1 — UDS server + length-prefix framing + `getpeereid()` peer-auth** (tests 1–3). ⚠️ safety-critical →
  **security-reviewer (`invariant`)**.
- **L2 — handshake + version negotiation + `shared/contracts/ipc` schema** (tests 4–6). Touches the §5.0
  contract artifact → 3-way verify if `CONTRACT_VERSION` bumps; security-reviewer optional (version-skew is a
  trust-boundary path).
- **L3 — JSON-RPC dispatch + `get_projection` + `get_capabilities`** (tests 7–9). code-quality.
- **L4 — `subscribe` streaming + Terminal-Channel frame-type seam** (tests 10–11). code-quality.

> ⚠️ **Orchestrator drives layer→layer** (banked lesson, 3 mechanisms): next-layer directive folded into each
> SHIP message; re-wake immediately on any post-commit "proceeding"; roll straight into the next layer's RED.

## Lessons-logged candidates anticipated
- **Convention candidate** — "The UDS GatewayPort transport: 4-byte big-endian length-prefix framing (bounded
  by `MAX_FRAME_SIZE`); `getpeereid()` peer-auth (reject uid≠daemon-uid; NOT `SO_PEERCRED`) as the primary
  trust gate, socket perms defense-in-depth; handshake-first (`HelloFrame`→`HelloAck`|`VersionSkewError`) on
  a daemon-authored `protocol_version`; read methods over read-only WAL (single-writer preserved)."
- **Convention candidate** — "Two version axes: `CONTRACT_VERSION` (the §5.0 domain+IPC schema/codegen
  artifact) vs `protocol_version` (the §6.4 wire-handshake skew check); don't conflate them."
- **Architecture-doc note** — the §6.1 read subset is live in 1.5; mutation methods (`submit_*`/`approve`/
  `deny`/`preview`) extend the same transport in Phase 2; the UDS `bind` call site is 1.6.
- **Future TODO — 1.6:** the UDS `bind()` + accept-loop spawn in the cold-start ordering. **Phase 2:** the
  mutation-method surface. **ui (cross-track):** swap `MockGatewayPort` → the real `UdsGatewayPort` + confirm
  `SUPPORTED_PROTOCOL_RANGE`.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5"; Q1 (versioning axes) + Q2
   (SUPPORTED_PROTOCOL_RANGE) + Q3 (projection-name enum) need answers before tests, and the ui consumer
   contract (`ui/src/gateway-client/types.ts` + `connection/version.ts`) is the surface to match.
2. **Run `/tdd uds_gatewayport_transport`** in the implementer session.
3. **Step 0 (Restate)** — confirm against the Feature line.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch" (note the `shared/` IPC schema).
5. **Step 2.5** — send the test-design write-up (one `Asserts:` line per test) + answers to Q1–Q6. ⚠️ L1
   touches safety rule #7 — a safety-design question escalates to the human **before** sign-off.
6. **Step 7.5** — state reachability honestly: the UDS server is a `pub` mechanism, `bind`/spawn → 1.6
   bootstrap; the ui is the real out-of-process consumer post-1.6.
7. **Step 9 (summarize)** — categorized flags per "Lessons-logged candidates anticipated."

# /tdd brief — gateway_uds_mutation_rpcs

## Feature
**L2-A (the FIRST L2 sub-slice — foundation-first, lead-ruled L2-O1=(B)): the crate mutation RPCs.**
Extend the pure-Rust `ui/gateway-uds` transport crate with the 4 §6.1 **mutation-intent** RPC helpers —
`submit_action(ActionRequest)→ActionAck` · `preview_action(action_request_id)→ActionPreview` ·
`approve(approval_id, step_id?)→ActionAck` · `deny(approval_id, reason)→ActionAck` — mirroring the
existing read helpers (`get_diff`/`get_capabilities`) and **reusing the exact `call` +
`demux_rpc_response` machinery** (so the verbatim §6.4 `WireError`→code path, the dual-None→`Protocol`
and id-mismatch→`Protocol` discipline are inherited identically). **Pure Rust, TDD'd vs the in-memory
`FakeStream`** (the 049 pattern). **Exposed-ahead-of-consumer:** NO Tauri command + NO TS caller yet
(those are L2-B/056) — the crate helpers exist + are security-reviewed in isolation before any UI path
reaches them. **The UI still never mutates** — these helpers SEND a typed RPC to the daemon's Action
Gateway (the single mutator); the daemon is the INV-SEC-1 chokepoint (L2-D1 pure pass-through). Deps
**`nexusops-shared` ONLY**, never the daemon (layering). **security-reviewer REQUIRED** (the mutation
transport surface; the L2 cat-1 checkpoint Part A).

## Use case + traceability
- **Task ID:** P6.8 L2-A (the live mutation transport, sub-slice 1 of 3; the lead-ruled L2-O1=(B) foundation-first sequence: **A crate RPCs** → B Tauri+TS wire [disabled] → C enable-live [USER-gated])
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the GatewayPort mutation-intent method surface; daemon `daemon/src/ipc/methods.rs`), `§6.4` (IPC framing + `IpcErrorCode` verbatim), `§5.0` (the frozen `shared/` contract the crate is a client of).
- **Reference:**
  - **The L2 cat-1 checkpoint** (`docs/planning/L2-live-mutation-transport-cat1-checkpoint.md`, lead-RULED 2026-06-14): **Part A ratified** — L2-D1 (pure pass-through, no UI execution path), L2-D6 (the §6.4 `WireError`→verbatim code path that feeds the distinct §11.5 rejection cards at L2-C), L2-O4 (idempotency/fencing pass-through — the UI passes the `ActionRequest`'s `idempotency_key`/`fencing_token` opaquely; the daemon owns dedup+fencing). L2-O2=(A) live `preview_action` with the submit (so the preview helper lands here in A with the others).
  - **The durable cat-1 Q1–Q7** (`docs/planning/intent-seam-cat1-safety-design.md`) — consume, never re-open. Q1 (pure submitter, INV-SEC-1).
  - **The crate it extends:** `ui/gateway-uds/src/lib.rs` — the read helpers `get_projection`/`get_diff`/`get_capabilities` (mirror them), the shared `call(stream, method, params, id)` + `demux_rpc_response` (REUSE — do NOT fork), the `FakeStream` test harness + the `rpc_ok`/`frame_json` helpers. The module header `//! NON-cat-1: reads only — no submit_action/approve/deny` updates this slice.
  - **The daemon method contract** (`daemon/src/ipc/methods.rs`): `submit_action` parses an `ActionRequest`→`gateway_result` (`ActionAck`-shaped); `approve` `{approval_id, step_id?}` (step_id accepted-but-RESERVED in 2.1c — pass it, the daemon ignores it) →`gateway_result`; `deny` `{approval_id, reason}`→`gateway_result`; `preview_action` `{action_request_id}`→`ActionPreview`. The frozen types: `ActionRequest` (`shared/src/actions.rs:309`), `ActionPreview` (`actions.rs:292`), `ActionAck` (`shared/src/ipc.rs:185`).
  - LESSON 20 (the crate is pure-Rust over a generic stream, deps `nexusops-shared` only, fail-closed boundary discipline), LESSON 16/17 (the §6.4 codes route verbatim → distinct §11.5 cards downstream).

## Acceptance criteria (what "done" means)
- [ ] **The 4 typed mutation helpers exist** in `ui/gateway-uds/src/lib.rs`, each forming the daemon-expected params + calling via the EXISTING `call(stream, method, params, id)` + typed-deserializing the result: `submit_action(stream, &ActionRequest, id) → ActionAck` (params = the serialized `ActionRequest`); `preview_action(stream, action_request_id, id) → ActionPreview` (`{action_request_id}`); `approve(stream, approval_id, step_id: Option<&str>, id) → ActionAck` (`{approval_id, step_id?}`); `deny(stream, approval_id, reason, id) → ActionAck` (`{approval_id, reason}`).
- [ ] **Verbatim §6.4 rejection (L2-D6).** A `WireError{code}` response on ANY mutation helper → `Err(ClientError::Wire(code))` — the code is returned **verbatim, never collapsed/remapped** (so L2-C can route `fencing_conflict`→hard-conflict / `precondition_stale`→re-approvable / `policy_denied`→deny / `internal_error`→fail-closed). This is inherited from `demux_rpc_response` — pin it per mutation method (at least `submit_action` + one of approve/deny).
- [ ] **Fail-closed frame discipline (inherited, re-pinned).** A non-`RpcResponse` frame / an id-mismatch / a dual-None response on a mutation call → `Err(ClientError::Protocol)`; a malformed/extra-field frame → `Err(ClientError::Serde)` (deny_unknown_fields). A structurally-valid response whose `result` is NOT the expected typed shape (e.g. a non-`ActionAck`) → `Err(ClientError::Serde)` (the typed-deserialize fail-closed path, mirroring `get_diff_malformed_result_is_serde_error`).
- [ ] **Pure pass-through (L2-D1).** Each helper SENDS the RPC + returns the daemon's typed result — there is NO execution / mutation / state-holding in the crate (it's a transport). The crate deps `nexusops-shared` ONLY (no daemon dep) — a layering pin (the existing crate-level constraint holds).
- [ ] **Idempotency/fencing pass-through (L2-O4).** `submit_action` serializes the `ActionRequest` **as-is** (its `idempotency_key`/`fencing_token`/`resource_refs` ride opaquely to the daemon) — the crate forms no tokens, reasons about no fencing locally.
- [ ] **The module header updates** — `//! NON-cat-1: reads only` → the crate now carries the mutation-intent RPCs (transport only; INV-SEC-1 stays daemon-side; no UI consumer yet — L2-B).
- [ ] **`security-reviewer` REQUIRED:** verbatim §6.4 codes (no collapse — a collapsed `fencing_conflict` breaks #6 downstream); no execution path (pure transport); the typed-deserialize fail-closed; layering (`nexusops-shared` only).
- [ ] **Whole crate suite green** (`cargo test -p nexusops-gateway-uds`); the ui workspace `cargo clippy`/`check` clean; cross-doc flagged at Step 9.

## Wiring / entry point (Step 7.5)
**none — the production caller lands in L2-B (056).** L2-A is the pure crate transport, TDD'd vs the
in-memory `FakeStream` (the 049 `get_diff`/`subscribe_stream` exposed-ahead pattern). The real-socket
path (a typed connect adapter, or `connect_and_call` reuse) + the Tauri mutation commands + the TS
`UdsGatewayPort` live wire are **L2-B**; the live `GatewayModal`/`DiffReview` submit enable is **L2-C
(USER-gated)**. `/wired`: the new helpers are reachable only from the crate's own tests until L2-B — by
design (no UI mutation path exists yet). State exactly this at Step 7.5.

## Files expected to touch
**Modified:**
- `ui/gateway-uds/src/lib.rs` — the 4 typed mutation helpers (mirror `get_diff`/`get_capabilities`; reuse `call`/`demux_rpc_response`) + the module-header update + the in-`mod tests` coverage (mirror the read-helper tests: params-formation + typed-deserialize + WireError-verbatim + malformed-Serde).

If a real-socket connect adapter for mutations (`connect_and_submit`-style) is wanted in A vs B, **flag at Step 2.5** (default: defer the connect adapter to L2-B with the Tauri wiring; A is the deterministic stream-based core).

## RED test outline (Step 2) — in `ui/gateway-uds/src/lib.rs` `mod tests`
1. `submit_action_forms_params_and_returns_ack` — a framed `RpcResponse{id, result: ActionAck-json}` → the helper writes a `submit_action` `RpcRequest` carrying the serialized `ActionRequest` + returns the typed `ActionAck`. — Asserts: params-formation + typed return (§6.1).
2. `preview_action_returns_typed_preview` — `{action_request_id}` params → a typed `ActionPreview`. — Asserts: §6.1 preview (L2-O2 live preview).
3. `approve_forms_params_and_returns_ack` — `{approval_id, step_id?}` → `ActionAck`; the request carries `approval_id` (+ `step_id` when `Some`). — Asserts: §6.1 approve.
4. `deny_forms_params_and_returns_ack` — `{approval_id, reason}` → `ActionAck`; the request carries both. — Asserts: §6.1 deny.
5. `mutation_wire_error_returns_verbatim_code` — a `WireError{fencing_conflict}` (and one more, e.g. `precondition_stale`) response on `submit_action`/`approve` → `Err(Wire(code))` verbatim, never collapsed. — Asserts: L2-D6 (§6.4 verbatim; the distinct-card precondition).
6. `mutation_malformed_result_is_serde_error` — a structurally-valid `RpcResponse` whose `result` is not an `ActionAck` → `Err(Serde)`, never a bad partial. — Asserts: typed fail-closed (§5.0).
7. `mutation_non_rpcresponse_or_id_mismatch_is_protocol` — a non-`RpcResponse` frame / an id-mismatch on a mutation call → `Err(Protocol)` (inherited; re-pin on a mutation path). — Asserts: frame discipline (§6.4).
Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/` (consumes the frozen `ActionRequest`/`ActionAck`/`ActionPreview` — no new/changed contract; no CONTRACT bump). **No shared-contract schema-snapshot** (no cross-area model touched).
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport client" row → note L2-A (the crate mutation RPCs landed; transport only, exposed-ahead) + likely a LESSON (the read-crate → mutation-crate extension; verbatim-code inheritance). No `ARCHITECTURE.md` edit.
- **Shared-contract (cross-area) model touched?** No (the crate consumes the daemon-frozen wire types; the daemon owns the contract).

## Things to flag at Step 2.5
1. **Connect adapter in A or B?** Default: **defer** the real-socket mutation connect adapter (`connect_and_submit`-style, or reuse the generic `connect_and_call`) to **L2-B** with the Tauri wiring — L2-A is the deterministic stream-based core (TDD'd vs `FakeStream`), matching how 049 shipped `get_diff` stream-based and 050 wired the host. Flag if you'd rather land a typed connect adapter here.
2. **`approve` step_id passthrough.** Default: accept `step_id: Option<&str>` and include it in the params only when `Some` (the daemon accepts-but-reserves it in 2.1c; an absent field is fine). Flag if you'd omit it entirely for L2 (single-action submit is the L2 core; per-step is a follow-on).
3. **One `WireError` test or per-method?** Default: pin the verbatim-code path on `submit_action` + one of approve/deny (the demux is shared, so one proves the mechanism; a second guards the wiring). Flag if you want all four.
4. **Reuse vs wrap `call`.** Default: the helpers call the EXISTING `call(stream, method, params, id)` directly (like `get_projection`) — do NOT add a mutation-specific demux (the §6.1 exactly-one-of + verbatim-code contract is identical). Flag if a mutation needs different framing (it must not).

## Dependencies + sequencing
- **Depends on:** 054 (`b3ffcb3`, sealed — the connection-state single authority; the pre-L2 gate) + the frozen `ActionRequest`/`ActionAck`/`ActionPreview` in `nexusops-shared` (present @ 0.31.0).
- **Blocks:** **L2-B (056)** — the Tauri mutation commands + the TS `UdsGatewayPort` live wire (consumer-disabled), which call these helpers; then **L2-C** — the USER-gated live-enable.

## Estimated commit count
**1** (the focused crate-transport extension — the 4 mutation helpers + tests, one concern). **security-reviewer REQUIRED** (the mutation transport surface; L2 cat-1 Part A). NOT bundled (a safety-surface slice gets its own commit + review).

## Lessons-logged candidates anticipated
- **Convention candidate** — the L2 mutation transport REUSES the L1 read crate's `call`/`demux_rpc_response` unchanged: the §6.4 verbatim-code + exactly-one-of contract is identical for reads and mutations, so a mutation helper is just params-formation + typed-deserialize over the same demux — and the verbatim §6.4 code (never collapsed) is what lets the downstream card layer (L2-C) keep `fencing_conflict` distinct from re-approvable (#6). Extends LESSON 20.
- **Architecture-doc note candidate** — the crate is no longer reads-only; it carries the mutation-intent RPCs (transport; INV-SEC-1 daemon-side), exposed-ahead of the L2-B consumer.

## How to invoke
1. **Read this brief end-to-end** — the 4 helpers mirror the read helpers + reuse `call`/`demux_rpc_response`; the 4 Step-2.5 flags.
2. Pre-flight: `track/ui` (054 `b3ffcb3` sealed; the crate at `ui/gateway-uds`). Same session — no `/session-start`.
3. **Run `/tdd gateway_uds_mutation_rpcs`**.
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 4 flags + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (verbatim §6.4 codes / no execution path / typed fail-closed / layering).
7. Step 9 — the cross-doc flag (the crate row note) + the round-seal lesson; the done-wake so I dispatch L2-B (056).

# /tdd brief — execution_profiles_read_transport (WAVE-1 W1-prof — UI half)

## Feature
The paired UI half of the daemon **W1-prof** (093) 0.48 contract: regen `generated.ts` to 0.48 (clears
the `generated_contract_version` drift RED) **+** wire the `get_execution_profiles` read-RPC transport (a
`GatewayPort.get_execution_profiles()` method + boundary parser + the Tauri command/allowlist), so the
cockpit can read the daemon's secret-free execution-profile list. The transport is **exposed-ahead** — its
consumer (the W1-C profile picker for `session.profile_change`) lands later, gated on W1-exec (094).

## Use case + traceability
- **Task ID:** **W1-prof** (the UI half — the unticked `- [ ] **W1-prof**` line under `### WAVE-1`; the
  plan line notes "CONTRACT → 0.48, pairs a ui regen"; daemon-orchestrator homes + seals the line).
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the IPC read-RPC surface), `§5.0`
  (contract SoT — the Rust→schema→generated-Zod regen propagation), `§2.8` (the execution-profile registry
  the RPC reads), `§15` (the secret-free result — the ProfileRow NEVER carries a keychain_ref; §15 #4).
- **Phase scope:** this brief **widens phase scope because** it is the UI half of a daemon contract slice
  (a regen + a read transport), not a daemon-phase slice — the `§`-references are cross-doc context.
- **Related context (from daemon-orchestrator, design-FROZEN — regen against the SEALED schema):**
  - **ProfileRow (final, 8 fields — NO keychain_ref/secret, §15 #4):** `{ execution_profile_id, provider,
    harness, model?, account_alias?, status: ExecutionProfile, is_default: bool, has_credential: bool }`
    + the result wrapper `GetExecutionProfilesResult { profiles: ProfileRow[] }`.
  - **PLACEMENT (load-bearing for this regen):** ProfileRow + the result live in **`shared/src/ipc.rs`** —
    a **read-RPC RESULT type (the `DiffResult`/`get_pr_diff` precedent), NOT a projection-row** (no
    `ProjectionName`, not `get_projection`-served). So the UI half = a gateway-client
    `get_execution_profiles()` method + a boundary parser + the Tauri command — **consumed DIRECTLY, NOT a
    projection shadow / no `parseProjectionPage`**. (Confirmed: daemon-orchestrator, this round.)
  - **`status: ExecutionProfile`** reuses the EXISTING generated flat enum (frozen 0.5b, already in
    `generated.ts`) → **NO new flat enum** → the value-set count stays **42**; the regen adds the 2 object
    `$defs` only.
  - **Current state:** `generated.ts` `CONTRACT_VERSION === "0.47.0"`; the frozen schema is `0.48.0` →
    `generated_contract_version_matches_frozen_schema` is RED. The regen (`ui/scripts/gen-contracts.mjs`)
    takes it 0.47→0.48 (LESSON 69 — a daemon contract bump always pairs a UI regen).
  - **Transport pattern:** mirror `get_pr_diff` (the §6.1 read-RPC 5-layer add): the `ui/gateway-uds` crate
    helper (`gateway-uds/src/lib.rs`, alongside `get_diff:229`/`get_pr_diff:246`; reuses
    `call`/`demux_rpc_response` → verbatim §6.4 `WireError`) → the Tauri command
    (`gateway_get_execution_profiles` in `src-tauri/src/commands.rs`, added to the `lib.rs`
    `generate_handler!` allowlist — NEVER a generic `gateway_call`, ui LESSON [[21]]) → the
    `GatewayPort.get_execution_profiles` method → `UdsGatewayPort.get_execution_profiles` (`invokeRead` +
    boundary-parse) → `MockGatewayPort.get_execution_profiles`.
  - **It is a READ, NOT a mutation** → NOT gated on `mutationsEnabled` (mirrors `get_diff`/`get_pr_diff`;
    no `enabledSessionKill`-style gate). The profiles list is read-only display data.

## Acceptance criteria (what "done" means)
- [ ] `ui/src/contracts/generated.ts` regen'd → `CONTRACT_VERSION === "0.48.0"`; the
      `generated_contract_version_matches_frozen_schema` drift test GREEN; `ProfileRow` +
      `GetExecutionProfilesResult` present as generated Zod schemas; **value-set count HELD at 42** (no new
      flat enum — `status` reuses `ExecutionProfile`).
- [ ] `GatewayPort.get_execution_profiles(): Promise<GetExecutionProfilesResult>` exists; `UdsGatewayPort`
      invokes `gateway_get_execution_profiles` + boundary-parses via a new `parseExecutionProfilesResult`;
      `MockGatewayPort` returns canned profiles (≥1 with `is_default:true`).
- [ ] It is a READ → NOT gated on `mutationsEnabled` (mirrors `get_diff`/`get_pr_diff`); a daemon
      `WireError` (e.g. `not_found`/`internal_error`) surfaces VERBATIM (the §6.4 code routing, ui LESSON 16).
- [ ] `parseExecutionProfilesResult` fail-closes (`BoundaryValidationError`) on a malformed payload
      (parse-don't-trust — a bad daemon payload never reaches view code).
- [ ] The `ui/gateway-uds` crate gains a `get_execution_profiles` helper (reuses `call`/`demux_rpc_response`,
      verbatim §6.4 codes) + the Tauri `gateway_get_execution_profiles` command on the `generate_handler!`
      allowlist (NO generic `gateway_call`).
- [ ] All unit tests pass; `/preflight` clean (the **full suite GREEN** — the drift RED cleared).

## Wiring / entry point (Step 7.5)
The **regen** (`generated.ts`) is consumed repo-wide (the contract layer + the drift test) → wired by
definition. The **`get_execution_profiles` read method is exposed-ahead** — its production caller (the W1-C
profile picker / `session.profile_change` control) lands in **W1-C** (gated on W1-exec/094); this slice
lands the read transport, **reachable via the `GatewayPort` interface** (`MockGatewayPort` for dev/tests,
`UdsGatewayPort` for prod). This is the established read-ahead pattern (`get_diff`/`get_pr_diff` were
adopted ahead of their views). The Step-7.5 reachability check confirms the method is invocable on both
ports; the consuming control is a named W1-C deferral.

## Files expected to touch (multi-commit — enumerate the layers)
**Commit 1 — regen + native transport (exposed-ahead):**
- `ui/src/contracts/generated.ts` — regen to 0.48 via `ui/scripts/gen-contracts.mjs` (NEVER hand-edited).
- `ui/src/contracts/index.ts` — re-export `ProfileRow`/`GetExecutionProfilesResult` (`= shape.X`,
  exposed-ahead → re-export, the ui-061 pattern) if the picker will import them.
- `ui/gateway-uds/src/lib.rs` — `get_execution_profiles` helper (+ crate tests).
- `ui/src-tauri/src/commands.rs` + `lib.rs` — `gateway_get_execution_profiles` command + allowlist entry.
**Commit 2 — TS port + boundary:**
- `ui/src/gateway-client/boundary.ts` — `parseExecutionProfilesResult` (mirror `parseDiff`).
- `ui/src/gateway-client/types.ts` — `GatewayPort.get_execution_profiles`.
- `ui/src/gateway-client/uds.ts` — `UdsGatewayPort.get_execution_profiles` (`invokeRead` + parse).
- `ui/src/gateway-client/mock.ts` — `MockGatewayPort.get_execution_profiles` (canned).
- the corresponding `*.test.ts`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`gateway_uds_get_execution_profiles_helper`** (crate) — Asserts: sends the `get_execution_profiles`
   RPC + demuxes the typed result; verbatim §6.4 `WireError` on rejection; Protocol on a malformed frame.
   Why: the §6.1 read-RPC + the `demux_rpc_response` reuse (ui LESSON 16).
2. **`parse_execution_profiles_result`** (`boundary.test.ts`) — Asserts: a valid payload → typed
   `GetExecutionProfilesResult`; a malformed payload → `BoundaryValidationError` (fail-closed). Why: §5.0
   parse-don't-trust at the boundary.
3. **`uds_get_execution_profiles_invokes_and_parses`** (`uds.test.ts`) — Asserts: invokes
   `gateway_get_execution_profiles` + boundary-parses; works WITHOUT `mutationsEnabled` (a read, unlike
   submit_action). Why: read-not-mutation (the `get_diff`/`get_pr_diff` precedent).
4. **`mock_get_execution_profiles_returns_canned`** (`mock.test.ts`) — Asserts: ≥1 profile, one
   `is_default:true`, one `has_credential:false` (exercises the W1-C picker's default-preselect +
   needs-credential). Why: a working dev/test port.
5. **`generated_contract_version_is_0_48`** (existing `generated.test.ts`) — Asserts: post-regen
   `CONTRACT_VERSION==="0.48.0"` + `ProfileRow`/`GetExecutionProfilesResult` present + value-sets still 42.
   Why: LESSON 69 (the regen clears the drift); §5.0.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `ProfileRow` + `GetExecutionProfilesResult` are NEW generated types (daemon-frozen
  in `shared/src/ipc.rs` as read-RPC result types — NOT projection-rows). The UI consumes the GENERATED Zod
  schemas (no hand-written shadow). The CONTRACT bump 0.47→0.48 is the daemon's; the UI mirrors it via regen.
- **Orchestrator doc rows to write hot (Step 9 routing):** the `ui/CLAUDE.md` "Generated Zod contract
  layer" row gets a 0.47→0.48 regen note (value-set 42 HELD; +2 object `$defs` ProfileRow/
  GetExecutionProfilesResult; consumed by the get_execution_profiles read transport) — I write it at
  `/orchestrate-end`.
- **§2.5-seam (shared-contract) model touched?** The schema-snapshot is DAEMON-side (W1-prof's
  `shared/tests/contract.rs`); the UI side has NO snapshot test for ipc.rs result types — the
  `generated.test.ts` drift + version pins are the UI's gate. No UI snapshot test needed.

## Things to flag at Step 2.5
1. **Regen-only vs regen + transport in ONE slice.** My default vote: **bundle (the W1-prof UI half)** —
   the regen + the read transport share the contract context, and the transport needs the regen'd
   `ProfileRow` type. One brief, 2 commits. (Splitting a 3-line regen into its own slice over-atomizes.)
2. **Mock canned data shape.** My default vote: **2 profiles** — one `is_default:true has_credential:true`,
   one `is_default:false has_credential:false` (exercises the picker's default-preselect + the "needs
   credential" affordance). Confirm count/shape.
3. **`index.ts` re-exports for ProfileRow/GetExecutionProfilesResult.** My default vote: **expose now**
   (exposed-ahead → re-export `= shape.X`, the ui-061 pattern) so the W1-C picker imports them cleanly.
4. **Include the W1-C picker in this slice?** My default vote: **NO** — the picker (`session.profile_change`
   control) is additionally gated on W1-exec (094); keep the transport exposed-ahead, the picker is W1-C.

## Dependencies + sequencing
- **Depends on:** the daemon **W1-prof 0.48 contract SEALED + committed** — regen against the SEALED schema,
  NOT the in-flight working tree (daemon-orchestrator pings the sealed commit hash; the ProfileRow shape is
  design-frozen but uncommitted until then). **DO NOT dispatch this slice before the seal ping.**
- **Blocks:** **W1-C** (the profile picker + `session.profile_change`/`send_message`/`pause`/`resume`
  controls), which also needs W1-exec (094, the daemon executor bodies).

## Estimated commit count
**2** (regen + native transport / TS port + boundary). CONTRACT-mirroring (the bump is the daemon's; the UI
regens + consumes). No safety invariant from the UI side (a read transport; INV-SEC-1 is daemon-side); a
read RPC needs no `security-reviewer` beyond the standard §15-no-leak check (the ProfileRow is secret-free
BY the daemon contract — `has_credential` bool, never a keychain_ref).

## Lessons-logged candidates anticipated
- **Convention candidate** — a daemon read-RPC RESULT type (`shared/src/ipc.rs`, the `get_pr_diff`
  precedent, NOT a projection-row) → a UI gateway-client read METHOD + boundary parser + Tauri command (NO
  generic `gateway_call`), consumed DIRECTLY (no `parseProjectionPage`/no projection shadow). The
  read-result-vs-projection-row distinction on the UI consumer side.
- **(already logged daemon-side)** LESSON 69 — the contract-bump-pairs-ui-regen; this slice is the pairing.

## How to invoke
1. **Wait for the daemon-orchestrator's W1-prof SEAL ping** (the sealed commit hash) — regen against the
   sealed schema, not the in-flight tree.
2. Read this brief end-to-end. Don't skip "Things to flag at Step 2.5".
3. Run `/tdd execution_profiles_read_transport`.
4. Step 1 → confirm the regen + the 5-layer transport file list.
5. Step 2.5 → ping back with answers to the 4 design Qs (or take defaults).
6. Step 9 → flag the W1-C consumer (gated on W1-exec/094).

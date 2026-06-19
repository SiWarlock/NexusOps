# /tdd brief — tauri_host_and_read_command_bridge

## Feature
**L1 read transport, slice 2 of ~3 (the Tauri host + the Rust read bridge).** Stand up the
**greenfield `ui/src-tauri/` Tauri 2.x host** (the UI now runs as a Tauri app wrapping the
existing Vite frontend) + a **Rust read-command bridge**: typed `#[tauri::command]` async fns
`gateway_get_projection` / `gateway_get_diff` / `gateway_get_capabilities` that call the **049
`nexusops-gateway-uds` crate**'s `connect_and_call` and return the daemon's raw JSON `Value`
(or a typed, serializable command error) to the frontend. The deterministic TDD core is the
**`ClientError` → frontend-error mapping** + the **param marshaling**; the Tauri standup is
infra + a gated smoke round-trip proves the bridge end-to-end. **NON-cat-1** (reads only).

> **⚠️ TOOLCHAIN PREREQUISITE — verify at Step 1.** `ui/src-tauri/` is greenfield (Tauri unbuilt,
> not a dep). This slice needs the **Tauri 2.x toolchain** (the `@tauri-apps/cli`, the `tauri`/
> `tauri-build` cargo crates, the macOS WKWebView — built-in). **If the toolchain can't be
> installed/built in this env, STOP and flag it as a blocker** (don't burn context fighting it) —
> the live transport then needs an infra decision. The 049 crate (the transport core) is already
> landed + TDD'd regardless.

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport — go-live; L1 read transport, slice 2 of ~3)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the `GatewayPort` read methods, exposed via the bridge), `§6.4` (the wire — via the 049 crate), `§5.0` (the frozen contract types). _(All three are now in Phase-6's Spec anchors — added at the 049 round.)_
- **Reference:** Tauri 2.x docs (pull via Context7 `/tauri-apps/tauri-docs`) — the Vite integration (`beforeDevCommand: "pnpm dev"`, `devUrl: "http://localhost:5173"`, `frontendDist: "../dist"`), `#[tauri::command]` + `tauri::generate_handler!` in `lib.rs run()`, `invoke` from `@tauri-apps/api/core`. The **049 crate** (`ui/gateway-uds`, `285fee6`) — `connect_and_call(method, params) → Result<Value, ClientError>`; **LESSON 20** (the transport-core boundary discipline).
- **Related context:** `docs/planning/ui-post-p4-runway-assessment.md` (the L1 phasing); brief `049-…` (the crate this consumes).

## Acceptance criteria (what "done" means)
**The Tauri host (infra):**
- [ ] `ui/src-tauri/` stood up (Cargo.toml [workspace member; deps `tauri` 2, `tauri-build` 2 build-dep, `nexusops-gateway-uds` (049), `serde`/`serde_json`], `tauri.conf.json` [the Vite-integration build block + the app/window/identifier], `build.rs`, `src/main.rs` + `src/lib.rs` with the `tauri::Builder` + `generate_handler!`). Root `Cargo.toml` adds the member; `ui/package.json` adds `@tauri-apps/cli` (dev) + `@tauri-apps/api` + a `"tauri"` script.
- [ ] The Tauri app builds + runs (`pnpm tauri dev` wraps the existing Vite UI); the **existing `pnpm dev` (Vite) still works** for the visual gate (the UI is unchanged). `cargo check --workspace` clean.
- [ ] The Tauri build artifacts (`ui/src-tauri/target/`, `ui/src-tauri/gen/`) are git-ignored (flag the `.gitignore` edit at Step 9 — orchestrator territory).

**The read-command bridge (the TDD core + the glue):**
- [ ] Typed `#[tauri::command]` async fns `gateway_get_projection(name, scope?)` / `gateway_get_diff(worktree_id, file)` / `gateway_get_capabilities()` — each forms the method params + calls the 049 crate's `connect_and_call` + returns `Result<serde_json::Value, GatewayCommandError>` (the **raw daemon `Value`** on success — the TS layer Zod-parses it at 051; a typed serializable error on failure). Registered via `generate_handler!` (reachable from the frontend via `invoke`).
- [ ] **The `ClientError` → `GatewayCommandError` mapping (the TDD'd unit):** `Wire(IpcErrorCode)` → `{ kind: "wire", code: <verbatim §6.4 code> }` (so the TS `describeRejection` routes it); `VersionSkew` → `{ kind: "version_skew", … }`; `FrameTooLarge`/`Io`/`Protocol`/`Serde` → their distinct kinds. **Serializable** (`#[derive(Serialize)]`), **leaks nothing sensitive** (reads-only; structural error only).
- [ ] **The param marshaling (TDD'd):** `gateway_get_projection`'s params == the JSON the daemon's `get_projection` expects (`GetProjectionParams`); `gateway_get_diff` == `GetDiffParams`. Match `daemon/src/ipc/methods.rs`.
- [ ] A **gated smoke round-trip** (`#[ignore]`/feature-gated, or a documented `pnpm tauri dev` manual step): `invoke("gateway_get_capabilities")` → the bridge → the 049 crate → a running daemon → the real `Capabilities`. Documented (needs a running daemon); not in the default suite.
- [ ] `cargo test -p` (the bridge unit tests) + `cargo clippy --workspace -- -D warnings` + `cargo fmt --check` clean; `tsc`/`oxlint`/`vitest` unaffected (no TS change yet).
- [ ] **`security-reviewer` REQUIRED** (the bridge boundary: the commands expose daemon reads to the frontend — the typed (narrow) command allowlist [no generic arbitrary-method command], the error mapping leaks nothing, the Tauri capability/permission config is least-privilege, the params).
- [ ] Cross-doc flagged at Step 9 (the `ui/CLAUDE.md` row: the Tauri host + the read-command bridge).

## Wiring / entry point (Step 7.5)
**The Tauri host IS the production app entry** (the UI now runs as a Tauri app); the read commands
are **registered in `generate_handler!`** (reachable from the frontend via `invoke`). The **TS
caller** (`ui/src/gateway-client/uds.ts` `UdsGatewayPort` + the Shell read-swap) is **slice 051** —
so the commands are **exposed-ahead-of-consumer** for one slice (the 043/049 pattern); the gated
smoke round-trip is 050's live exercise. Flag at Step 7.5 as expected.

## Files expected to touch
**New:** `ui/src-tauri/{Cargo.toml, tauri.conf.json, build.rs, src/main.rs, src/lib.rs, src/commands.rs}` (the bridge + its unit tests in `commands.rs` `#[cfg(test)]`); optionally `ui/src-tauri/tests/smoke.rs` (the `#[ignore]` round-trip).
**Modified:** `Cargo.toml` (root: +`ui/src-tauri` member) · `ui/package.json` (+`@tauri-apps/cli`/`@tauri-apps/api` + the `"tauri"` script) · `.gitignore` (+the Tauri artifacts — flag at Step 9).

If implementation needs files beyond this list, **flag at Step 2.5**.

## RED test outline (Step 2)
The TDD core is the bridge's pure helpers (the Tauri macro + the socket are infra/gated):
1. **`client_error_maps_wire_to_kind_wire_verbatim_code`** — `ClientError::Wire(NotFound)` → `{kind:"wire", code:"not_found"}`. Why: §6.4 verbatim code → the TS `describeRejection`.
2. **`client_error_maps_each_variant_to_distinct_kind`** — VersionSkew/FrameTooLarge/Io/Protocol/Serde → distinct kinds; no variant collapses; nothing sensitive in the message. Why: honest, distinct, leak-free.
3. **`get_projection_params_match_daemon`** — the marshaled params == `GetProjectionParams{name,scope?}` (the daemon's shape). Why: §6.1/§5.0 wire conformance.
4. **`get_diff_params_match_daemon`** — == `GetDiffParams{worktree_id,file}`. Why: §6.1.
5. **(integration, `#[ignore]`) `smoke_get_capabilities_roundtrips`** — `invoke` (or the bridge fn) → a real daemon → `Capabilities`. Documented; not default-run.

(The command fns call `connect_and_call` — to keep the helpers unit-testable, factor the param-marshaling + the error-mapping as **pure fns** the `#[tauri::command]` wrappers call; the connect is the 049 crate's already-tested adapter — see Q3.)

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (the bridge passes the frozen `shared` types through; no new contract). `GatewayCommandError` is a UI-host-internal serializable error shape (not a `shared` contract).
- **Orchestrator doc rows to write hot (Step 9):** a `ui/CLAUDE.md` row — the Tauri host + the read-command bridge (typed commands wrapping the 049 crate; the `ClientError`→`GatewayCommandError` mapping; the least-privilege Tauri config). No `ARCHITECTURE.md` edit.
- **New shared-contract model?** No — the bridge consumes the frozen §6.4 contract (via the 049 crate); no new shared model, no schema-snapshot obligation this slice.

## Things to flag at Step 2.5
1. **The command error shape.** **Default vote:** a typed serializable `GatewayCommandError` enum (`#[derive(Serialize)]`) `{kind, code?, message?}` — `Wire` carries the verbatim §6.4 `code` so the 051 TS layer maps it via `describeRejection`. A bare `String` loses the structured code. Flag if you'd rather the TS parse a string.
2. **Command granularity — typed-per-method vs one generic `gateway_call`.** **Default vote:** typed-per-method commands (`gateway_get_projection`/`get_diff`/`get_capabilities`) — a narrow, auditable allowlist; a generic `gateway_call(method, params)` would let the frontend invoke ARBITRARY daemon methods (a wider attack surface incl. the L2 mutations). **Typed-narrow is the security-preferred shape** (the security-reviewer will check this).
3. **Keeping the helpers unit-testable.** The `#[tauri::command]` fn + `connect_and_call` are infra/integration. **Default vote:** factor the **param-marshaling** + the **`ClientError`→`GatewayCommandError` mapping** as pure fns (TDD'd); the command wrapper is a thin `marshal → connect_and_call → map` glue. Don't try to unit-test the Tauri macro or the socket (gated smoke covers the round-trip).
4. **The dev/build + visual-gate model.** **Default vote:** keep `pnpm dev` (Vite) working for the visual gate (the UI is unchanged this slice); `pnpm tauri dev` runs the Tauri app. Confirm the Tauri toolchain installs (Step 1); if it can't, **STOP + flag the blocker** (don't fight it).

## Dependencies + sequencing
- **Depends on:** slice **049** (`ui/gateway-uds`, `285fee6` — the transport core this bridges); the Tauri 2.x toolchain (verify Step 1).
- **Blocks:** slice **051** (the TS `UdsGatewayPort` invoking these commands + Zod-parsing + the Shell read-path swap → the app shows REAL projection/diff data) → **052** (the streaming `subscribe` + reconnect recovery + unique correlation ids). The **L2 mutation transport** (cat-1) stays HELD on the daemon's 0.30.0 ②-mini.

## Estimated commit count
**1–2.** The Tauri standup (infra) + the read-command bridge (the error-mapping/param TDD core + the glue). The implementer MAY split the standup commit from the bridge commit. **NON-cat-1** (reads only — no mutation) but **`security-reviewer` REQUIRED** (the bridge exposes daemon reads to the frontend — the command allowlist + the error-mapping + the Tauri capability config are the boundary).

## Lessons-logged candidates anticipated
- **Convention candidate** — possibly: "the Tauri read-command bridge is a narrow typed allowlist (one `#[tauri::command]` per read method, never a generic `gateway_call`) — the frontend can invoke ONLY the enumerated reads; the `ClientError`→serializable-`{kind,code}` mapping carries the verbatim §6.4 code (the TS maps it), leaks nothing; the param-marshaling + error-mapping are pure-fn TDD'd, the connect is the 049 adapter." Surface at Step 9.
- **Architecture-doc note candidate** — the UI is now a Tauri app (the host stood up); the read transport bridges TS↔Rust↔daemon.
- **Future TODO — next-brief working set** — 051 (the TS `UdsGatewayPort` + Shell read-swap), 052 (subscribe streaming + recovery + correlation ids), the L2 mutation transport (cat-1, HELD), the `.gitignore` Tauri-artifacts edit.

## How to invoke
1. **Read this brief end-to-end** — the TOOLCHAIN PREREQUISITE (verify Step 1; STOP+flag if it can't build) + the 4 Step-2.5 questions.
2. Pre-flight: confirm `track/ui` in `NexusOps-ui`; confirm `cargo` + attempt the Tauri toolchain (`pnpm add -D @tauri-apps/cli`); **if Tauri won't install/build, flag the blocker before going further.**
3. **Run `/tdd tauri_host_and_read_command_bridge`.**
4. Step 0/1 — confirm against the Feature + Files lines.
5. **Step 2.5** — answer the 4 questions + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (the bridge boundary: typed-narrow allowlist, leak-free error mapping, least-privilege Tauri config).
7. Step 9 — the cross-doc flag + the `.gitignore` flag + the bridge-allowlist lesson candidate.

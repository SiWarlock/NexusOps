# /tdd brief — l2_mutation_bridge_disabled

## Feature
**L2-B (the SECOND L2 sub-slice — the Tauri mutation commands + the TS `UdsGatewayPort` live wire,
CONSUMER-DISABLED).** Mirror the L1 read bridge for the 4 §6.1 mutation methods: add the typed Tauri
commands `gateway_submit_action` / `gateway_preview_action` / `gateway_approve` / `gateway_deny`
(each marshals params → the EXISTING `call_daemon(method, params)` → `connect_and_call` → raw `Value`
/ leak-free `GatewayCommandError`; reuse `map_client_error` — verbatim §6.4 codes), register them in
`lib.rs run()`, and rewire the TS `UdsGatewayPort` mutation methods (`submit_action`/`preview_action`/
`approve`/`deny`) to `invoke` those commands + boundary-parse the typed result. **CRUCIALLY GATED:**
the live mutation path is **guarded behind an explicit `mutationsEnabled` flag (default `false`)** so
**NO production path can reach a live mutation** — until **L2-C** (the USER-gated go-live) flips it.
The `GatewayModal`/`DiffReview` submit controls **stay disabled** (defense-in-depth). This is the
lead-ruled L2-O1=(B) "B Tauri+TS wire [disabled]" step. **The UI still never mutates** — a submit (once
enabled at L2-C) SENDS a typed intent; the daemon Gateway is the INV-SEC-1 chokepoint. **`security-reviewer`
REQUIRED** (the L2 cat-1 Part A — the mutation bridge + the no-production-reach guard).

## Use case + traceability
- **Task ID:** P6.8 L2-B (the live mutation transport, sub-slice 2 of 3; A crate RPCs ✅ → **B Tauri+TS wire [disabled]** → C enable-live [USER-gated])
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the GatewayPort mutation method surface), `§6.4` (IPC framing + `IpcErrorCode` verbatim → the §11.5 cards), `§11.4` (the `canSubmitIntent` gate — the controls stay disabled), `§11.1` (read-only/degraded).
- **Reference:**
  - **The L2 cat-1 checkpoint** (`docs/planning/L2-live-mutation-transport-cat1-checkpoint.md`, lead-RULED): L2-O1=(B) foundation-first; **L2-D2** (the Tauri MUTATION allowlist — one typed command per method, **still NO generic `gateway_call`**); L2-D1 (pure pass-through); L2-D6 (verbatim §6.4 codes); L2-O2 (live `preview_action` rides the enable with the submit); **🔒 L2-O3** (the go-live enable is L2-C, USER-gated — L2-B must NOT enable).
  - **The L1 read bridge to mirror:** `ui/src-tauri/src/commands.rs` — `gateway_get_diff` (the command pattern), `call_daemon(method, params)` (REUSE — `spawn_blocking` `connect_and_call`), `map_client_error` + `GatewayCommandError` (REUSE — verbatim `Wire{code}`), the params-marshal pure fns (`get_diff_params` pattern). The registration site: `lib.rs run()`'s `invoke_handler![...]` (add the 4 mutation commands to the typed allowlist).
  - **The TS port to rewire:** `ui/src/gateway-client/uds.ts` — the mutation methods currently throw `NOT_WIRED_L2` (`submit_action`/`preview_action`/`approve`/`deny`, lines 259-270); rewire to `invoke` + boundary-parse, behind the `mutationsEnabled` guard. The boundary parsers (`ui/src/gateway-client/boundary.ts`) gain `parseAck`/`parsePreview` (or reuse the intent-contracts shadows) for the typed results.
  - **L2-A** (`49b61d5`) — the crate's typed mutation helpers (the tested contract spec; the production path is `connect_and_call` generic, the read precedent) + LESSON 26.
  - LESSON 21 (the NARROW typed allowlist — the registered set IS the allowlist), LESSON 22 (parse-don't-trust + wire-rejection-plain vs transport-fault-Error), LESSON 16/17 (the seam is a pure submitter; verbatim codes → distinct cards), forbidden #6 (no intent control without `canSubmitIntent`).

## Acceptance criteria (what "done" means)
- [ ] **The 4 typed Tauri mutation commands** exist in `commands.rs` + are registered in `lib.rs run()`: `gateway_submit_action(request: ActionRequest) → Value` (marshal the `ActionRequest`), `gateway_preview_action(action_request_id: String) → Value`, `gateway_approve(approval_id: String, step_id: Option<String>) → Value`, `gateway_deny(approval_id: String, reason: String) → Value`. Each calls `call_daemon("<method>", params)` (the verbatim §6.4 `Wire{code}` rides through `map_client_error`). **Still NO generic `gateway_call`** (L2-D2 — the registered set is the allowlist).
- [ ] **The TS `UdsGatewayPort` mutation methods invoke the commands** + boundary-parse the typed result (`submit_action`→`ActionAck`, `preview_action`→`ActionPreview`, `approve`/`deny`→`ActionAck`), with the **same wire-rejection (plain `{code}`) vs transport-fault (`Error`) classification** as the reads (LESSON 22) — a daemon `WireError` routes the verbatim §6.4 code to the consumer; a transport fault is an honest `Error`.
- [ ] **The `mutationsEnabled` guard — NO production reach (the cat-1 crux).** The TS mutation methods are guarded behind `private mutationsEnabled = false` (constructor default): when `false`, each mutation method **throws a distinct "L2 mutation submit not enabled" error and NEVER `invoke`s** (no production path reaches a live mutation). Pin: with the default port, every mutation method throws-not-enabled + never calls `invoke` (a spy proves no invoke). The invoke path is tested with the flag forced on in tests only.
- [ ] **The UI submit controls stay disabled (defense-in-depth).** The `GatewayModal` approve/deny + the `DiffReview` per-hunk submit controls remain disabled (the existing `canSubmitIntent`-gated + disabled-pinned 044 state) — L2-B enables NOTHING in the UI (the enable is L2-C). Pin: the controls stay disabled (the 044/048 disabled-pins hold).
- [ ] **Verbatim §6.4 codes (L2-D6) end-to-end.** A daemon `WireError{fencing_conflict}` on a (test-enabled) submit → the verbatim code reaches `describeRejection` → the never-auto-resolved hard-conflict card (#6), distinct from `precondition_stale`→re-approvable. Pin the bridge `map_client_error` (Rust, exists) + the TS classification on a mutation path.
- [ ] **`security-reviewer` REQUIRED:** the `mutationsEnabled` guard (no production reach until L2-C; default false; the only setter is L2-C); the typed-narrow allowlist (no `gateway_call`); verbatim §6.4 codes; parse-don't-trust the ack/preview; the controls stay disabled.
- [ ] Whole suite green (the ui TS suite + the `nexusops-ui` Rust crate); `/preflight` clean; cross-doc flagged at Step 9.

## Wiring / entry point (Step 7.5)
**Partial — the wire is BUILT + DISABLED.** The Tauri commands are registered (reachable from the TS
`invoke` allowlist); the TS `UdsGatewayPort` mutation methods contain the live `invoke` path BUT it is
**guarded off** (`mutationsEnabled=false`) — so `/wired` shows: the mutation methods are reachable from
the `GatewayModal`/`DiffReview` submit handlers, but those controls are DISABLED and the methods
throw-not-enabled before any `invoke`. **No production mutation reaches the daemon.** The enable (flip
`mutationsEnabled` + enable the controls) is **L2-C (USER-gated)**. State exactly this at Step 7.5.

## Files expected to touch
**Modified:**
- `ui/src-tauri/src/commands.rs` — the 4 mutation commands + their params-marshal pure fns (mirror `get_diff_params`) + the module-header update (no longer reads-only; the mutation allowlist, still no `gateway_call`) + tests.
- `ui/src-tauri/src/lib.rs` — register the 4 commands in `run()`'s `invoke_handler!`.
- `ui/src/gateway-client/uds.ts` — rewire the 4 mutation methods (invoke + boundary-parse, behind `mutationsEnabled`) + the guard field + tests.
- `ui/src/gateway-client/boundary.ts` — `parseAck`/`parsePreview` (or reuse the `intent-contracts.ts` shadows) for the typed results + tests.

If the GatewayModal/DiffReview need a touch to KEEP the controls disabled (they should already be), **flag at Step 2.5** — L2-B must enable nothing.

## RED test outline (Step 2)
**Rust (`commands.rs` `mod tests`):**
1. `submit_action_params_match_daemon` — the marshaled params == the daemon's `ActionRequest` (round-trip). — Asserts: §6.1 params (L2-D1/O4 opaque).
2. `approve_deny_preview_params_match_daemon` — `{approval_id,step_id?}` / `{approval_id,reason}` / `{action_request_id}`. — Asserts: §6.1.
3. `mutation_commands_reuse_verbatim_wire_code` — the existing `map_client_error` carries `Wire{code}` verbatim on the mutation path (re-assert; the map is shared). — Asserts: L2-D6.
**TS (`uds.test.ts`):**
4. `mutation_methods_throw_not_enabled_by_default_and_never_invoke` — with the default port (`mutationsEnabled=false`), each of submit/preview/approve/deny throws "not enabled" + the `invoke` spy is NEVER called. — Asserts: the no-production-reach guard (the cat-1 crux).
5. `mutation_methods_invoke_and_parse_when_enabled` — with the flag forced on (test-only), `submit_action` invokes `gateway_submit_action` + boundary-parses the `ActionAck`; same for preview/approve/deny. — Asserts: the live wire (test-gated).
6. `mutation_wire_rejection_is_plain_data_not_error` — a daemon `WireError{code}` on an enabled submit → the verbatim code as plain `{code}` (not an `Error`) so `describeRejection` routes it; a transport fault → an `Error` (LESSON 22). — Asserts: L2-D6 + the classification.
**TS (controls-disabled pin):**
7. `submit_controls_stay_disabled_at_l2b` — the `GatewayModal` approve/deny + `DiffReview` submit stay disabled (the 044/048 disabled-pins hold; L2-B enables nothing). — Asserts: 🔒 L2-O3 (enable is L2-C).
Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/` (consumes the frozen `ActionRequest`/`ActionAck`/`ActionPreview`; no CONTRACT bump; no schema-snapshot). The `mutationsEnabled` flag + the `GatewayCommandError` reuse are UI-local.
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport client" row → note L2-B (the mutation bridge + TS wire, guarded-disabled) + likely a LESSON (the mutation bridge mirrors the read allowlist; the `mutationsEnabled` no-reach guard as the user-gate switch). The "Tauri host + read-command bridge" row → note it now carries the mutation allowlist (still no `gateway_call`). No `ARCHITECTURE.md` edit.
- **Shared-contract (cross-area) model touched?** No.

## Things to flag at Step 2.5
1. **The "[disabled]" mechanism (the load-bearing cat-1 design).** My default vote: an explicit **`mutationsEnabled` flag on `UdsGatewayPort` (default `false`)** gating all 4 TS mutation methods (preview included — L2-O2 couples it to the submit go-live) — when false they throw-not-enabled + never `invoke`; L2-C flips it to `true` (the single user-gated switch) + enables the UI controls. **Alternative:** keep the TS methods throwing `NOT_WIRED_L2` and land only the Rust commands at B (the TS wire + enable both at C). Default vote: **the `mutationsEnabled` guard** — it builds + security-reviews the full wire at B (the lead's "Tauri+TS wire") while keeping the go-live a single auditable flag flip at C. Flag if you prefer the alternative.
2. **`preview_action` gated or live at B?** Default: **gated with the others** (L2-O2=(A) — the live preview rides the submit go-live, so the human approves against a real daemon preview at the L2-C moment; preview is read-like/non-mutating but coupling it keeps one switch). Flag if you'd land live preview at B (it's safe — non-mutating).
3. **Ack/preview parsers — new or reuse.** Default: reuse the `intent-contracts.ts` `ActionAck`/`ActionPreview` shadows via `boundary.ts` `parseAck`/`parsePreview` (the 044 provisional shadows). Flag if a new parser is cleaner.
4. **Where the `mutationsEnabled` flag lives.** Default: a `UdsGatewayPort` private field (constructor default false), so the Mock is unaffected (it has its own mutation methods). Flag if it belongs on a shared gate.

## Dependencies + sequencing
- **Depends on:** L2-A (`49b61d5`, sealed — the crate transport; though the bridge reuses the generic `connect_and_call`, L2-A pins the param/return contract) + the L1 read bridge (`commands.rs`/`uds.ts`).
- **Blocks:** **L2-C** — the USER-gated live-enable (flip `mutationsEnabled` + enable the `GatewayModal`/`DiffReview` controls). I author L2-C's brief but **do NOT dispatch** until the lead runs the user sign-off (L2-O3).

## Estimated commit count
**1** (the mutation bridge + the guarded TS wire — one cat-1 unit; the lead scoped B as "Tauri+TS wire"). **security-reviewer REQUIRED** (L2 cat-1 Part A — the no-production-reach guard is the crux). NOT bundled with anything.

## Lessons-logged candidates anticipated
- **Convention candidate** — the mutation bridge mirrors the read allowlist (one typed command per method, no `gateway_call`, `map_client_error` verbatim) — LESSON 21 extended to mutations; the **`mutationsEnabled` no-reach guard** is the clean single-switch the user-gated go-live (L2-C) flips, so the full wire is built + reviewed under lead authority but provably unreachable until the user signs off. Extends LESSON 21/22/26.
- **Architecture-doc note candidate** — the Tauri bridge now carries the mutation allowlist (still no `gateway_call`); the TS port has the live mutation wire, guarded off until L2-C.

## How to invoke
1. **Read this brief end-to-end** — the mutation bridge mirrors the read bridge; the `mutationsEnabled` guard is the cat-1 crux; the 4 Step-2.5 flags.
2. Pre-flight: `track/ui` (L2-A `49b61d5` sealed). Same session — no `/session-start`.
3. **Run `/tdd l2_mutation_bridge_disabled`**.
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 4 flags (esp. #1 the "[disabled]" mechanism) + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (the no-production-reach guard / typed allowlist / verbatim §6.4 / parse-don't-trust / controls-stay-disabled).
7. Step 9 — the cross-doc flags + the lesson; the done-wake. **L2-C (the USER-gated enable) is authored-but-HELD** — I escalate the user sign-off to the lead before dispatching it.

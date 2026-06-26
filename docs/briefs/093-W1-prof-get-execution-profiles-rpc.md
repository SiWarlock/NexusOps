# /tdd brief — get_execution_profiles_read_rpc

## Feature
A `get_execution_profiles` IPC **read** RPC that serves a typed, **secret-free** `ProfileRow` list from the canonical `execution_profiles` registry (§2.8) — the daemon read surface the cockpit's session-launch **profile picker** (and `session.profile_change`) needs. The registry exists (P5.3a/5.3b) but is currently unreadable from the UI (no `ProjectionName`, no read method).

## Use case + traceability
- **Task ID:** W1-prof
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (GatewayPort read method + wire contract), `§2.8` (the `execution_profiles` durable registry), `§15` (#4 keychain-ref POINTER-only — **never serve the secret**), `§5.0`/`§2.5` (contract SoT + the seam — this slice bumps CONTRACT and freezes a new row).
- **Related context:** the read-RPC precedents `get_diff` (LESSON 33, 0.28.0) + `get_pr_diff` (LESSON 59, 0.40.0) — a request-on-demand read returning a typed result, NOT a `get_projection` (the registry is NOT a projection: not in `REBUILD_TABLES`, no `ProjectionName` — LESSON 62). The typed-fail-closed serve precedent = `ApprovalQueueRow`/`SessionRow` (LESSON 37). The registry read seam exists: `daemon/src/profiles/mod.rs::profile_exists` (read-only WAL `SELECT … FROM execution_profiles`), `ProfileSpec{provider,harness,model?,account_alias?,keychain_ref?,status}`. Keychain posture: LESSON 49/64/65 (the `keychain_ref` is a POINTER, never the secret).

## Acceptance criteria (what "done" means)
- [ ] `get_execution_profiles` (no-param, or an optional `{workspace_id?}` filter — Step-2.5 Q5) returns a `{profiles: Vec<ProfileRow>}` result over a **read-only WAL** connection (no mutation, no write-actor — the `profile_exists`/`get_diff` precedent).
- [ ] `ProfileRow` is a NEW frozen `shared/` contract type served **typed/fail-closed** (the LESSON-37 precedent): `{execution_profile_id, provider, harness, model?, account_alias?, status: ExecutionProfile, is_default: bool, has_credential: bool}`.
- [ ] **§15 #4 — the secret POINTER is NEVER served.** `keychain_ref` (and any secret) is **absent** from `ProfileRow`; the credential state is exposed ONLY as the derived `has_credential: bool` (= `keychain_ref.is_some()`). Pinned adversarially (seed a profile with a `keychain_ref` → assert it does not appear in the serialized row).
- [ ] The seeded default profile is returned and identifiable (`is_default` true for the `SqliteProfileLookup::default_id`).
- [ ] Empty registry → `{profiles: []}` (not an error).
- [ ] A corrupt / un-decodable registry row → **fail-closed** (the whole RPC errors `internal_error`, never a silent partial list — the LESSON-37 typed-serve precedent).
- [ ] CONTRACT bump → **0.48.0** (new method + new `ProfileRow`/result type); §2.5-seam schema-snapshot for `ProfileRow` + the 3-way verify updated.
- [ ] **Paired UI regen flagged** (LESSON 69) — the contract bump reds the UI generated-version drift test. The UI half (gateway-client `get_execution_profiles` method + the Tauri allowlist + `generated.ts` regen + the picker consuming `ProfileRow`) is a **separate ui-track slice** coordinated with ui-orchestrator; this brief is the daemon half only.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`daemon/src/ipc/methods.rs` — add `"get_execution_profiles" => get_execution_profiles(&req.params, db_path)?` to the method-name dispatch match (alongside `"get_diff"`/`"get_pr_diff"`, lines ~80-111). The new `get_execution_profiles` fn reads the `execution_profiles` table over read-only WAL → `Vec<ProfileRow>` → JSON result. Reachable from the live UDS dispatch (the `get_diff` `/wired` precedent — pin reachability from the real dispatch, not just a unit). The UI consumer (profile picker) is the paired ui slice.

## Files expected to touch
**New:**
- `daemon/tests/get_execution_profiles.rs` — the RPC behavior tests (or extend `daemon/tests/execution_profiles.rs`).

**Modified:**
- `daemon/src/ipc/methods.rs` — the dispatch arm + the `get_execution_profiles` reader (the `read_*_typed` family).
- `shared/src/ipc.rs` — `ProfileRow` + the `GetExecutionProfilesResult` (the `DiffResult`/`GetDiffParams` placement precedent — a read-RPC result type, NOT a projection row; Step-2.5 Q1) + `CONTRACT_VERSION` → 0.48.0.
- `shared/tests/contract.rs` — the `ProfileRow` schema-snapshot (@0.48.0) + 3-way verify.
- `shared/contracts/schema/` + the verify fixtures — regenerated.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/get_execution_profiles.rs` (+ `shared/tests/contract.rs`):

1. **`get_execution_profiles_returns_seeded_default`** — after cold-start seed, the RPC returns ≥1 `ProfileRow` incl. the default (`is_default == true`).
   - Asserts: the default profile is present + flagged. Why: §2.8 cold-start seed (LESSON 62).
2. **`profile_row_never_serves_keychain_ref_or_secret`** — seed a profile with `keychain_ref = Some(…)`; the serialized row has NO `keychain_ref`/secret field; `has_credential == true`.
   - Asserts: §15 #4 POINTER-never-served; `has_credential` is the only credential signal. Why: §15 #4 (LESSON 49/64/65).
3. **`get_execution_profiles_empty_registry_returns_empty_list`** — no profiles → `{profiles: []}`, not an error.
   - Asserts: empty is a valid result. Why: read-RPC totality.
4. **`get_execution_profiles_corrupt_row_fails_closed`** — an un-decodable row → `internal_error`, never a silent partial.
   - Asserts: fail-closed typed serve. Why: §6.1 read surface (no loose-JSON / no silent drop on a typed surface, the LESSON-37 precedent).
5. **`get_execution_profiles_reachable_from_dispatch`** — the method routes through the real `ipc` dispatch (the `get_diff` `/wired` precedent).
   - Asserts: Step-7.5 reachability. Why: §6.1 production-path.
6. **`profile_row_contract_snapshot`** (`shared/tests/contract.rs`) — `ProfileRow` field-name set == the checked-in snapshot @0.48.0, tagged `spec(§6.1)`; 3-way verify green.
   - Asserts: §2.5-seam freeze. Why: §5.0 / LESSON 15.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW `ProfileRow` + `GetExecutionProfilesResult` shared types; `CONTRACT_VERSION` → 0.48.0.
- **Orchestrator doc rows to write hot (Step-9 routing):** the `daemon/CLAUDE.md` "IPC GatewayPort wire contract" row (+ `get_execution_profiles`/`ProfileRow` @0.48.0) + the `ARCHITECTURE.md §6.1` Appendix-A mirror + a note on the `execution_profiles` registry row (now UI-readable via the RPC). The orchestrator writes these; the implementer does NOT touch them.
- **§2.5-seam (shared-contract) model touched?** YES — `ProfileRow` crosses the §6.1 seam → the schema-snapshot test (test 6) is **required** in this same `/tdd` cycle.
- **Paired UI regen (LESSON 69):** flag for ui-orchestrator — a 0.48.0 ui regen slice + the gateway-client method + the Tauri allowlist + the picker.

## Things to flag at Step 2.5
1. **`ProfileRow`/result placement** — `shared/src/ipc.rs` (the `DiffResult`/`get_pr_diff` read-RPC-result precedent) vs `shared/src/projections.rs` (the typed-row home). My default vote: **`shared/src/ipc.rs`** — it's a read-RPC result, not a projection row (no `ProjectionName`, not `get_projection`-served).
2. **`keychain_ref` exposure** — omit entirely + serve a derived `has_credential: bool`, vs serve the pointer string. My default vote: **omit + `has_credential: bool`** — §15 #4: a boolean is not a pointer; the picker needs "needs credential?" not the ref. (This is the §15-load-bearing call; if there is ANY doubt about exposing even `has_credential`, raise it — but a derived bool is safe.)
3. **`is_default` identification** — add `is_default: bool` to `ProfileRow` (derived from `SqliteProfileLookup::default_id`) so the picker pre-selects. My default vote: **yes** — the picker needs it; it's derivable, not a new stored field.
4. **Corrupt-row disposition** — fail-closed whole-RPC vs skip-and-continue. My default vote: **fail-closed whole-RPC** (the LESSON-37 typed-serve precedent — a mis-typed registry row is an integrity signal, not a row to silently drop).
5. **Result envelope** — `{profiles: [...]}` struct vs a bare `[ProfileRow]` array. My default vote: **`{profiles: [...]}` struct** — the `DiffResult` precedent + avoids the bare-array boundary gotcha that bit the projection pages (the UI consumes this via the gateway-client directly, not `parseProjectionPage`).
6. **Param shape** — no-param vs `{workspace_id?}` filter. My default vote: **no-param** (MVP is single-workspace; add the filter when multi-workspace lands).

## Dependencies + sequencing
- **Depends on:** the `execution_profiles` registry + `ProfileLookup` (P5.3a/5.3b — landed).
- **Blocks:** the ui **W1-C** profile picker + the `session.profile_change` UI control; the paired 0.48.0 ui regen.

## Estimated commit count
**1.** The daemon RPC + `ProfileRow` contract + the §2.5-seam snapshot are one cohesive, contract-bumping unit. **§15-adjacent** (the keychain-ref exclusion is a security pin) → the implementer runs `security-reviewer` (the `invariant` policy) at Step 8; the keychain-ref-never-served assertion (test 2) is the load-bearing pin. NOT a separable safety-pin commit (the whole slice is the read surface). The paired UI regen is a SEPARATE ui-track slice.

## Lessons-logged candidates anticipated
- **Convention candidate** — "a §2.8-registry read RPC serves a typed **secret-free** row — `keychain_ref`/secret NEVER served (§15 #4), credential state as a derived `has_credential: bool`; the `get_diff` read-RPC + the LESSON-37 typed-fail-closed precedents combined."
- **Architecture-doc note candidate** — `get_execution_profiles` joins the §6.1 read surface (+ `ProfileRow` @0.48.0); the `execution_profiles` registry is now UI-readable.

## How to invoke
1. Read this brief end-to-end (esp. the Step-2.5 §15 #4 keychain-ref call).
2. Run `/tdd get_execution_profiles_read_rpc`.
3. Step 2.5 — ping back with answers (or take defaults). The §15 #4 keychain-ref-never-served pin is non-negotiable; the rest are defaults.
4. Step 9 — surface the cross-doc rows + the paired-ui-regen flag.

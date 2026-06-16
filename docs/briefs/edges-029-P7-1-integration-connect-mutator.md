# /tdd brief — integration_connect_mutator (Wave-C, the new mutator)

## Feature
The `integration.connect` Gateway mutator (P7.1, Wave-C; USER-ruled modeling A) — a generic `ExecutorKind::Integration` executor that **registers** an integration connection and emits `IntegrationConnectionRegistered` (keychain_ref = a §15 #4 pointer-ONLY). Adds the catalog action_type + the ExecutorKind → **bumps CONTRACT 0.32 → 0.33** (edges-branch-local; the daemon ratifies the action_type + assigns the final version at the edges→main merge — like MIGRATION_10/11).

## Use case + traceability
- **Task ID:** P7.1 (Wave-C — the integration-connection mutator)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (the ActionTypeCatalog — a new entry) · `§15` (the §15 #4 keychain-ref-pointer-only invariant + #2 single-mutator/no-bypass) · `§7.2` (the connection registry feed). **Widens phase scope because:** this slice adds a catalog action_type + `ExecutorKind` (a `shared/` change) — the CONTRACT bump is the USER-ruled (build-now A) Wave-C surface; flagged PROMINENTLY for the edges→main reconciliation ledger.
- **Related context:**
  - **USER ruling (via lead):** build Wave-C now, **modeling A** — generic `integration.connect` (risk-2) + a new `ExecutorKind::Integration` (a keychain-write/registration op, NOT a network sync — so it does NOT belong in the Github/Linear network executors). CONTRACT 0.32→0.33 local; daemon ratifies at merge.
  - **The frozen event:** `IntegrationConnectionRegistered{connection_id, provider, keychain_ref, account?}` (`shared/src/events.rs:490`, frozen 0.26). `connection_id` = an edges-private bare-String id (NOT a frozen-22 IdKind — the `terminal_id`/`out_` precedent). `keychain_ref` = a NON-SECRET pointer.
  - **The mutator precedent is edges-019** (`daemon/src/project/executor.rs`, `ProjectExecutor`) — `validate()` + `execute()` returning `ExecutionOutcome::Succeeded{ emitted_events: vec![EmittedEvent::Namespaced{ event_type, payload_json }] }` (the Q1=B generic bridge; serialize-fault → `Failed` fail-closed). Mirror it. The arg-injection guard discipline is the edges-020/021 `reject_dash_operands` pattern (LESSON 31 in the lead's framing) — applied here to the connect operands.

### ⚠️ The §15 + LESSON-20-FORCED design: registration-only, the token NEVER flows through the action
A risk-2 action is **approval-gated**, so its `execute()` runs off the **durable, §15-REDACTED `action_requests` row** (LESSON 20 §7.2-split: "approve runs off the durable redacted row"). A secret placed in `inputs_json` is **redaction-masked before persist** (the LESSON 16 dual-gate) → **GONE by execute-time**. Therefore the connect mutator **CANNOT carry the token** and retrieve it to store it. It is **registration-only**: the action inputs carry `{provider, keychain_ref (a pre-existing pointer), account?}` — **NO token** — and the executor records the connection. **The token→keychain WRITE is a SEPARATE, DEFERRED mechanism** (a non-deterministic secret-I/O surface, HITL — analogous to live agents/PTY/GitHub; the `keyring` crate is NOT yet a daemon dep and no keychain-write path exists). This keeps the slice deterministic + test-first and makes §15 #4 hold **by construction** (the executor never receives a token). **Flagged as a Finding/scope note** — Wave-C builds the registration half; the credential-storage half is a documented carry-forward.

## Acceptance criteria (what "done" means)
- [ ] `ExecutorKind::Integration` added (`shared/src/catalog.rs`, the `catalog_enum!`); `"integration.connect"` added to `MVP_ACTION_TYPES`; `catalog::lookup("integration.connect")` returns `entry(R::Level2, P::Api, I::FromInputs, X::Integration, requires_resource_refs=false, params_schema_present=true)` with the Step-2.5-confirmed `standing_grant_eligible`.
- [ ] **CONTRACT 0.32 → 0.33:** `CONTRACT_VERSION` bumped (`shared/src/lib.rs:136`); the catalog + ExecutorKind schema regenerated; the catalog snapshot (`shared/tests/contract.rs`) + the §5.0 3-way verify updated to the new entry count (LESSON 15 — emit the ExecutorKind as a flat enum; verify the count delta).
- [ ] `IdGen::new_connection_id() → "conn_<ULID>"` added (the `new_terminal_id`/`new_outbox_id` bare-String precedent; injectable → deterministic in tests).
- [ ] An `IntegrationExecutor` (`ExecutorKind::Integration`): `validate()` accepts `{provider, keychain_ref, account?}`; `execute()` mints `connection_id`, emits `EmittedEvent::Namespaced{ IntegrationConnectionRegistered::EVENT_TYPE, payload_json }` with `{connection_id, provider, keychain_ref, account?}`; a serialize-fault → `ExecutionOutcome::Failed` (fail-closed, no malformed event). It holds **NO** `WriteHandle`/eventstore/SQL (emits ONLY via `emitted_events` — the edges executor boundary).
- [ ] **§15 #4 — keychain_ref pointer-ONLY (load-bearing pin, OWN commit):** the executor never receives/handles a token (registration-only); the emitted event carries ONLY the `keychain_ref` pointer; a test asserts no secret-shaped field reaches the event. The arg guard **rejects a `keychain_ref` that looks like a secret** (defense-in-depth pointer-shape validation — e.g. a known secret prefix / high-entropy run → reject, the LESSON 13 detector reused read-only).
- [ ] **Arg guard (LESSON 31):** `provider` is the closed `Provider` enum (reject-unknown, §5.0/§15); `account` is validated (no injection into the connection identity); fail-closed on a malformed operand.
- [ ] **INV-SEC-1 no-bypass:** `integration.connect` is reachable ONLY through the catalog-gated pipeline (risk-2 → approval-gated; the unknown-type fail-closed; the `requires_resource_refs`/Adjudication guards run before dispatch). The `IntegrationExecutor` is registered on the **live** production `CatalogExecutor` in `main.rs` (`/wired integration.connect` shows: submit → policy(risk-2) → approval → execute → emit → audit).
- [ ] All unit tests in `daemon/tests/integration_connect.rs` pass; the catalog snapshot/contract tests pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
`main.rs` production `CatalogExecutor` (the block registering Session/Project/Git/Github/Linear, ~main.rs:199-236) — add `.register(ExecutorKind::Integration, Box::new(IntegrationExecutor::new(idgen)))`. The action reaches it via the standard Gateway pipeline (`submit_action` → `CatalogPolicy` risk-2 → approval → `CatalogExecutor::execute` → `IntegrationExecutor` → in-txn append of the emitted event through the §15 gate). `/wired integration.connect` MUST show the live path; the executor is NOT reachable except via the pipeline.

## Files expected to touch
**New:**
- `daemon/src/integrations/connect.rs` (or extend `integrations/executor.rs`) — the `IntegrationExecutor`.
- `daemon/tests/integration_connect.rs` — the RED tests.

**Modified:**
- `shared/src/catalog.rs` — `ExecutorKind::Integration` + `MVP_ACTION_TYPES` += `"integration.connect"` + the `lookup` match arm.
- `shared/src/lib.rs` — `CONTRACT_VERSION` 0.32.0 → 0.33.0.
- `shared/tests/contract.rs` + `shared/contracts/` — catalog snapshot + 3-way verify regen.
- `daemon/src/idgen.rs` — `new_connection_id()` (trait + the real + the deterministic test impl).
- `daemon/src/integrations/mod.rs` — export the executor.
- `daemon/src/main.rs` — register `ExecutorKind::Integration`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/integration_connect.rs` (+ catalog assertions in the shared contract tests):

1. **`test_catalog_integration_connect_entry`** — `catalog::lookup("integration.connect")` = the expected entry (risk-2, `X::Integration`, `requires_resource_refs=false`, the confirmed `standing_grant_eligible`); `"integration.connect"` ∈ `MVP_ACTION_TYPES`.
   - Asserts: the catalog add. Why: §6.3 catalog-authoritative risk (LESSON 19).
2. **`test_executor_emits_registration`** — `execute()` with `{provider=github, keychain_ref="conn-gh-ref", account="me"}` → `Succeeded`, `emitted_events = [Namespaced{ IntegrationConnectionRegistered, {connection_id: "conn_…", provider, keychain_ref, account} }]`.
   - Asserts: the registration event shape; `connection_id` is `conn_`-prefixed (IdGen). Why: §7.2 connection feed; the edges-019 emit precedent.
3. **`test_keychain_ref_pointer_only_no_token`** (§15 #4 — OWN commit) — the emitted event carries ONLY the `keychain_ref` pointer; assert no secret-shaped field; AND a `keychain_ref` that looks like a secret (e.g. `ghp_…`/high-entropy) → **rejected** (`Failed`/validate-err).
   - Asserts: pointer-only; secret-shaped ref rejected. Why: §15 #4 (keychain_ref NEVER the token) + defense-in-depth (LESSON 13 detector, read-only).
4. **`test_arg_guard_rejects_unknown_provider_and_bad_account`** — an unknown `provider` → reject (closed `Provider` enum); a malformed `account` → fail-closed.
   - Asserts: reject-unknown + operand validation. Why: §15/§5.0 + the LESSON 31 arg-guard discipline.
5. **`test_invalid_inputs_fail_closed`** — missing `provider` or `keychain_ref` → `ExecutionOutcome::Failed`, NO event emitted.
   - Asserts: fail-closed, no malformed audit event. Why: §15 (the executor never emits a malformed event — the edges-019 serialize-fault precedent).
6. **`test_inv_sec_1_executor_holds_no_writehandle`** — a structural/import-grep test that `IntegrationExecutor` holds no `WriteHandle`/eventstore/SQL (emits only via `emitted_events`); + `/wired integration.connect` proves the live catalog-gated path.
   - Asserts: no-bypass by construction. Why: INV-SEC-1 #1/#2 (the merge's 6-criteria pattern); security-reviewer verifies.
7. **`test_catalog_snapshot_and_contract_at_0_33`** — the catalog snapshot + 3-way verify reflect the new entry + `ExecutorKind::Integration`; `CONTRACT_VERSION == "0.33.0"`.
   - Asserts: the contract surface is regenerated + pinned. Why: §5.0 (LESSON 15; the 4.0b-ui1 catalog-add-bumps-CONTRACT precedent).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **YES — a `shared/` contract change** (CONTRACT 0.32→0.33): `ExecutorKind` += `Integration`; `MVP_ACTION_TYPES` += `integration.connect`; the catalog `lookup`. `IntegrationConnectionRegistered` is already frozen (0.26 — NOT re-touched).
- **Shared-contract seam touched? YES** → the RED outline INCLUDES the catalog schema-snapshot + 3-way-verify update (test 7), authored this cycle.
- **Orchestrator doc rows to write hot (HELD-for-merge PLAN-DELTA — `docs/planning/edges-R5-wiring-plan.md`; edges does NOT edit the shared root docs in-worktree):**
  - **PROMINENT merge-ledger flag (USER-directed):** the catalog `integration.connect` add + the `ExecutorKind::Integration` add + the CONTRACT 0.32→0.33 bump — the daemon (catalog/CONTRACT owner) RATIFIES the action_type + assigns the final CONTRACT version at the edges→main merge (like the MIGRATION numbers). This is the single highest-visibility merge-reconciliation item.
  - **Cross-doc invariant row** (`daemon/CLAUDE.md` §6.3 ActionTypeCatalog + `ARCHITECTURE.md` Appendix A): the new `integration.connect` row + `ExecutorKind::Integration`.
  - **Arch-note (§15 #4):** the connect mutator is **registration-only** — the token→keychain write is a SEPARATE deferred mechanism; the action carries only the keychain_ref pointer (forced by §15 + LESSON 20).
  - **Completed-work tick:** P7.1 Wave-C mutator landed (the projector = edges-030; the credential-storage = deferred).

## Things to flag at Step 2.5
1. **`standing_grant_eligible` for `integration.connect`.** My default vote: **true** (grantable) — a connect is REVERSIBLE (disconnect) and risk-2; the eligibility axis is irreversibility/blast-radius (LESSON 32), and connect doesn't fit the non-grantable (destructive/risk-4) criteria. **Defer to security-reviewer** — if a credential registration warrants an always-per-action approval, set false (and the merge-ledger notes it).
2. **`preview_class` — `Api` vs `Command`.** My default vote: **`Api`** (the integration family) — flag if a connect (no network call in modeling A) reads better as `Command`.
3. **`idempotency_formula` — `FromInputs` vs `None`.** My default vote: **`FromInputs`** (re-connecting the same provider+account dedups — the `linear.create_issue` precedent). Flag if connections should always be distinct.
4. **The token→keychain write scope.** My default vote: **DEFER** (registration-only this slice; the credential-storage mechanism — the `keyring` crate + a non-Gateway secret-store path — is a follow-on HITL/live-integration slice; forced-out by §15 + LESSON 20 as detailed above). Object if the phase-exit needs the live credential write in-scope (it adds a new dep + macOS-keychain HITL right at the merge boundary).

## Dependencies + sequencing
- **Depends on:** the R8 merge (the live `CatalogExecutor` registry + the `EmittedEvent::Namespaced` bridge + `IntegrationConnectionRegistered` frozen 0.26) · edges-028 P5.1 (lands first; this is R9 slice-2).
- **Blocks:** edges-030 (the `integration_connections` projector + MIGRATION_11) · the deferred credential-storage mechanism · `/phase-exit 7`.
- **Sequencing:** R9 slice-2 (after edges-028). The projector (edges-030) is a SEPARATE non-safety slice (this mutator gets its OWN commit — safety pin + CONTRACT bump).

## Estimated commit count
**2 commits** (safety-pin isolation, the 3.3b/§15-pin-own-commit precedent):
1. The catalog add + `ExecutorKind::Integration` + CONTRACT 0.33 + the `IntegrationExecutor` registration core.
2. The **§15 #4 keychain_ref-pointer-only pin** (the no-token + secret-shaped-ref-reject guard) — its OWN commit (the load-bearing safety pin).

*(If the impl finds the §15 pin inseparable from the executor core — it may be, since "registration-only / no token" is structural — fold to 1 commit and say so, the LESSON 39 "one cohesive file → one commit when security-CLEAR-as-a-whole" carve-out.)*

**security-reviewer = REQUIRED** (a NEW mutator + §15 #4 + INV-SEC-1 — the `invariant` policy). `code-quality-reviewer` runs.

## Lessons-logged candidates anticipated
- **Convention candidate** — "A credential-registration mutator is registration-ONLY: the secret never flows through the action (a risk-≥1 action executes off the §15-redacted durable row → an `inputs_json` secret is masked by execute-time, LESSON 20); the action carries the keychain_ref POINTER, the token→keychain write is a separate non-Gateway mechanism." (edges lesson candidate — renumber to 44+ at the merge.)
- **Architecture-doc note candidate** — the §15 #4 registration-only data-flow (above) + the deferred credential-storage mechanism.
- **Future TODO — operational** — the credential-storage path (`keyring` crate + a non-Gateway secret-store IPC + the live macOS-keychain write) = a HITL/live-integration follow-on; + a fresh `cargo audit` when `keyring` lands.

## How to invoke
1. Read this brief end-to-end (don't skip Step 2.5 — answer Q1–Q4, esp. Q1 standing_grant + Q4 token-scope, or take defaults).
2. Run `/tdd integration_connect_mutator`.
3. Step 0 (Restate) → confirm against the Feature line + the registration-only §15 design.
4. Step 2.5 → ping back with Q1–Q4 + the per-acceptance-bullet coverage map (this is a safety slice — the §15 #4 + INV-SEC-1 asserts get reviewed closely).
5. Step 9 → categorized flags (the CONTRACT bump + the merge-ledger item + the deferred credential-storage).

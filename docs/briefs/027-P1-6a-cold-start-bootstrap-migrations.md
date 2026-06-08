# /tdd brief — cold_start_bootstrap

## Feature
The daemon's first-run/cold-start **orchestration** (`bootstrap.rs`): in §16-binding order, acquire the single-instance pidlock → create the app-support dir → open+migrate the event store (backup/rollback + DB-version floor already enforced by `EventStore::open`) → register the desktop-host `Device` + a fresh `LocalRunner` → return an initialized `DaemonContext` the runtime (1.6c) drives. Plus two typed-error cleanups the version-compat code wants. **Stale-socket reclaim, UDS `bind()`, the accept-loop, and all Tokio spawns are 1.6b; the §17 degradable replay is 1.6c — both out of scope here.**

## Use case + traceability
- **Task ID:** P1.6a (the cold-start/bootstrap/migrations third of the split 1.6)
- **Architecture sections it implements:** `ARCHITECTURE.md §16` (first-run bootstrap ordering, DB migrations backup/rollback, version-compat matrix), `§5.3` (LocalRunner MVP-live / Device desktop-host), `§7.1` (EventTypeRegistry accretion for the registration events), `§4.2`/INV-SEC-1 (single-mutator — see the Step-2.5 design call).
- **Related context:**
  - Predecessors already shipped most of the DB lifecycle — read these before writing tests:
    - `daemon/src/eventstore/migrations.rs` — `run` (forward-only, **already** backs up on `from≥1` + restores on failure), `refuses_db_newer_than_supported` (the §16 DB floor), `backup_db`/`restore_db`, `current_user_version`, `SUPPORTED_USER_VERSION=5`.
    - `daemon/src/eventstore/mod.rs` — `EventStore::open(path, idgen, clock, redactor)` **already** does pragmas → version-floor → `migrations::run` (backup/rollback) → `catch_up_replay` → `outbox::reset_in_flight`. Bootstrap *calls* this; it does not re-implement it. Also `user_version()` (line 420, returns `-1` sentinel — L1 fixes), the `.unwrap_or(-1)` at line 296.
    - `daemon/src/locks/pidlock.rs` — `PidLock::acquire(path) -> Result<PidLock, PidLockError>` (`AlreadyHeld` | `Io`); the held `PidLock` must stay alive for the daemon lifetime (Drop releases the OS lock).
  - Contract types (frozen `shared/`): `shared/src/objects.rs` `DesktopObjectKind` (`LocalRunner`/`Device` + `id_prefix()` `lr_`/`dev_`, `is_deferred()`); `shared/src/actor.rs` `ActorType` (`LocalRunner`, `System`); `shared/src/events.rs` `SessionStarted` — the **payload pattern to mirror** for the registration events (identity on the envelope typed fields where possible, type-specific attrs in the payload, `#[serde(deny_unknown_fields)]`, schemars-derived).
  - Carry-forward folded in: **1.1 L2** (`user_version()` → `Result`) + **1.1 L3** (typed restore-failure path + "rolled back to vN").
  - Session doc `005` "How to use what was built" describes the intended cold-start call order.

## Acceptance criteria (what "done" means)
**L1 — eventstore typed-error cleanups (daemon-internal; no contract bump):**
- [ ] `EventStore::user_version()` returns `Result<u32, EventStoreError>` (not `i64` with a `-1` sentinel); the line-296 `.unwrap_or(-1)` site is likewise made typed/honest (no silent sentinel on a real read error).
- [ ] `migrations::run` no longer **swallows** a restore failure: a failed `restore_db` after a failed migration surfaces a typed `EventStoreError::RestoreFailed { from, source }` (distinct from the generic `Migration`), carrying enough to render the §16 "update failed, rolled back to vN" message. A *successful* rollback still returns the original `Migration` error (the migration is what failed) but in a form the caller can map to "rolled back to vN".

**L2 — `bootstrap.rs` cold-start orchestration:**
- [ ] `cold_start(cfg: BootstrapConfig) -> Result<DaemonContext, BootstrapError>` performs, in this order: (1) `PidLock::acquire`; (2) create the app-support dir (idempotent — exists-ok); (3) `EventStore::open` (migrate + floor + replay happen inside); (4) register Device + LocalRunner (L3). Returns a `DaemonContext` holding the live `PidLock` + `EventStore` + the registered `DeviceId`/`LocalRunnerId`.
- [ ] A second `cold_start` against a held pidlock returns `BootstrapError::AlreadyRunning` (maps `PidLockError::AlreadyHeld`) — the single-instance guarantee (forbidden #3 / safety rule, §16).
- [ ] A DB newer than `SUPPORTED_USER_VERSION` makes `cold_start` refuse with a typed `BootstrapError` wrapping `DbNewerThanSupported` (downgraded-binary refuse-safe, §16) — daemon does NOT start.
- [ ] Re-running `cold_start` against an existing valid DB resumes (re-opens, replays, returns a context) — restart idempotency (§16 "daemon restart resumes against existing DB").
- [ ] `BootstrapConfig` injects the base dir + `Clock`/`IdGen`/`Redactor` (deterministic tests — §14 seams; no real `~/Library` access in tests).
- [ ] A composed `DaemonVersionInfo` (`app_version`, `protocol_range` = `SUPPORTED_PROTOCOL_RANGE`, `db_user_version`, `contract_version`) is available off the context for the handshake/diagnostics. **The enforcing floor is the DB check; the agent-CLI/SDK tuple + sidecar-MCP `initialize` rows of the §16 matrix are §9.1/§13.1 — out of scope, named as deferred.**

**L3 — Device + LocalRunner registration (contract-surface; CONTRACT_VERSION bump):**
- [ ] New registration event payload(s) in `shared/src/events.rs` mirroring `SessionStarted` (`deny_unknown_fields`, schemars), added to the EventTypeRegistry; `CONTRACT_VERSION` minor-bumped; schema regenerated; 3-way verify + envelope version pin updated.
- [ ] On cold-start the daemon appends the registration event(s) via the write-actor `append` path (redaction gate + projector fold both run). **Mechanism: USER-RULED Option B (2026-06-08) — system-event append, `actor_type=System`; NOT a Gateway Action.**
- [ ] **`workspace_id`:** registration events carry a reserved **system-workspace sentinel** — a well-known `ws_` constant for System-actor lifecycle events (USER-RULED; NOT a minted host-workspace, NOT a nullable-envelope change). Define it once in `shared/`, valid per `WorkspaceId::parse` (a fixed ULID body, e.g. the all-zero ULID `ws_00000000000000000000000000`) so the §15 fail-closed parse still holds.
- [ ] **Ids:** add `DeviceId`/`LocalRunnerId` `minted_id!` newtypes in `shared/src/ids.rs` off the frozen `DesktopObjectKind` prefixes (`dev_`/`lr_`), keyed via `DesktopObjectKind` — **NOT** new `IdKind` variants (the frozen-22 stays untouched) (USER-RULED).
- [ ] **object_refs:** `lr_`/`dev_` are NOT envelope typed columns → the projector sources the id from the registration **payload** to write `('device',id)`/`('local_runner',id)` rows (the payload is the identity home; rebuild-safe).
- [ ] **LocalRunner is minted fresh per daemon start** (§5.3 "minted per daemon start") — a new `lr_` id + a registration event every cold-start.
- [ ] **Device is the stable desktop host** — register-if-absent (idempotent across restarts): a restart does NOT mint a second `dev_` for the same host.
- [ ] Registration is reflected in a projection + `object_refs` (the read model the UI/Brain will consume) — reuse the 1.2 projector pattern; confirm which `proj_*` table holds the runner/device (likely a small addition or `object_refs`-only — flag at Step 2.5 if a new DDL is needed).

**Cross-cutting:**
- [ ] All unit + integration tests in `daemon/tests/bootstrap.rs` (+ `shared/tests/*` pins) pass.
- [ ] `/preflight` clean (fmt-check FIRST, then clippy `-D warnings`, check, test).
- [ ] Cross-doc invariant updated atomic with the registration-event contract change (orchestrator writes at Step 9 — see below).

## Wiring / entry point (Step 7.5)
`cold_start()` is the cold-start orchestrator. Its **production caller is the daemon binary `main.rs`**, which needs the Tokio runtime + accept-loop → **lands in 1.6b**. So in this slice `cold_start()` is reachable from `daemon/tests/bootstrap.rs` only; the honest "ship the mechanism, wire the runtime caller next slice" pattern (1.3/1.4/1.5 precedent). **Name this explicitly at Step 7.5** — `cold_start`'s production entry is the 1.6b `main.rs`/runtime; every other surface it calls (`PidLock::acquire`, `EventStore::open`, `append`) already has a real caller. Do NOT ship a stub `main.rs` that can't run the daemon (see Step-2.5 Q3).

## Files expected to touch
**New:**
- `daemon/src/bootstrap.rs` — `cold_start`, `BootstrapConfig`, `DaemonContext`, `BootstrapError`, `DaemonVersionInfo`.
- `daemon/tests/bootstrap.rs` — the integration tests.

**Modified:**
- `daemon/src/lib.rs` — `pub mod bootstrap;`.
- `daemon/src/eventstore/mod.rs` — `user_version() -> Result<u32,…>` (+ the line-296 sentinel); registration `append` call sites are via the existing API.
- `daemon/src/eventstore/migrations.rs` — typed `RestoreFailed` path in `run`.
- `daemon/src/eventstore/mod.rs` (errors) — `EventStoreError::RestoreFailed` variant (+ map in `BootstrapError`).
- `shared/src/events.rs` — registration payload(s); `shared/src/lib.rs` — `CONTRACT_VERSION` bump; `shared/src/schema.rs` + `contracts/schema/*.json` regen; `shared/tests/*` pins.
- `daemon/src/projections/*` — fold the registration event(s) into a read model + `object_refs` (mirror 1.2).

If implementation needs files beyond this list (e.g. a new `proj_*` DDL for the runner, or a path-resolution dep), **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
`daemon/tests/bootstrap.rs` (+ a couple in the eventstore tests for L1):

**L1**
1. **`test_user_version_returns_typed_result`** — Asserts: `user_version()` is `Ok(5)` on a migrated DB; a read error is `Err`, never `-1`. Why: 1.1 L2 carry-forward; no silent sentinel in version-compat code.
2. **`test_restore_failure_is_typed_not_swallowed`** — Asserts: when a migration fails AND restore can't run, the error is `EventStoreError::RestoreFailed{..}` (not generic `Migration`); a successful rollback maps to a "rolled back to vN"-renderable error. Why: 1.1 L3; §16 rollback UX.

**L2**
3. **`test_clean_first_run_creates_and_migrates`** — Asserts: cold_start on an empty base dir returns `Ok(DaemonContext)`; DB exists at `user_version==SUPPORTED`; app-support dir created. Why: §16 first-run ordering happy path.
4. **`test_second_instance_refused`** — Asserts: a second `cold_start` while the first's `PidLock` is held → `BootstrapError::AlreadyRunning`. Why: §16 single-instance / forbidden #3.
5. **`test_db_newer_than_binary_refuses`** — Asserts: a DB at `user_version > SUPPORTED` → cold_start refuses (typed `BootstrapError`), daemon doesn't start. Why: §16 downgraded-binary refuse-safe.
6. **`test_restart_resumes_existing_db`** — Asserts: cold_start, drop context (release pidlock), cold_start again → `Ok`, same DB, events intact. Why: §16 restart-resume.
7. **`test_app_support_dir_idempotent`** — Asserts: cold_start when the dir already exists succeeds (exists-ok, no error). Why: re-run robustness.

**L3**
8. **`test_localrunner_minted_per_start`** — Asserts: each cold_start appends a LocalRunner registration event with a fresh `lr_` id; two starts → two distinct runner ids. Why: §5.3 "minted per daemon start".
9. **`test_device_stable_across_restarts`** — Asserts: two cold_starts against the same host/base dir reuse one `dev_` id (register-if-absent). Why: §5.3 Device = stable desktop host.
10. **`test_registration_event_redacted_and_projected`** — Asserts: the registration event passes the §15 redaction gate (persists `redacted`) and lands in a projection + `object_refs`. Why: §15 + 1.2 projector contract.
11. **`shared`: `test_contract_version_bumped` + registration payload wire-pin** — Asserts: `CONTRACT_VERSION` bumped; payload round-trips snake_case; `deny_unknown_fields` rejects an extra field. Why: §5.0 contract discipline / LESSON §2.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW event-type(s) in the EventTypeRegistry (LocalRunner/Device registration payloads) → `CONTRACT_VERSION` minor bump. `DaemonContext`/`BootstrapError`/`DaemonVersionInfo` are daemon-internal (no `shared/` surface). The L1 typed errors are daemon-internal.
- **Orchestrator doc rows to write hot (Step 9 routing):** EventTypeRegistry row in `daemon/CLAUDE.md` cross-doc + `ARCHITECTURE.md` Appendix A / §7.1 (the new registration event types) + the §16 cross-doc note that bootstrap orchestration landed (ordering + version floor wired). The Device/LocalRunner **objects** are already frozen (§5.3) — this slice records that they are now *registered at bootstrap*, not a new object freeze.

> Implementer never edits `daemon/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, `daemon/LESSONS.md` — flag categorized at Step 9; orchestrator writes hot.

## Things to flag at Step 2.5
1. **[RESOLVED 2026-06-08 — USER-RULED Option B] Registration mechanism = system-event append via the write-actor (`actor_type=System`), NOT a Gateway Action.** Both sub-leans adopted: (i) reserved system-workspace sentinel for `workspace_id`; (ii) `DeviceId`/`LocalRunnerId` `minted_id!` newtypes off `DesktopObjectKind` (not new `IdKind` variants). Full record: `MVP_TASKS.md` Decisions-tabled. **L3 is UNBLOCKED — build per the L3 acceptance criteria above; security-reviewer applies (INV-SEC-1-touching).**
2. **App-support path resolution.** macOS `~/Library/Application Support/NexusOps/`. Hand-roll via `$HOME` + `std`, or add a `directories`/`dirs` dep? Default vote: **hand-roll via `std::env`** (one path, MVP is macOS-only, avoid a dep) — `BootstrapConfig` injects the base dir so tests never touch the real path. Flag if you'd rather add the dep.
3. **Ship a `main.rs` binary entry here?** Default vote: **no — defer `main.rs` to 1.6b** (it needs the Tokio runtime + accept-loop to be a real entry; a stub that can't run the daemon adds a fake entry point). `cold_start()` is reachable from tests this slice; 1.6b wires the production caller. Flag if you disagree.
4. **Registration projection target.** Does the runner/device land in an existing `proj_*` table, a new small DDL, or `object_refs`-only? Default vote: **`object_refs` + (if a read model is needed) the smallest addition** — don't invent a projector the UI doesn't consume yet; mirror 1.2's "DDL present, body where a consumer exists." Flag if a new DDL is warranted.
5. **`DeviceRegistered` a separate event from `LocalRunnerRegistered`?** Default vote: **two distinct payloads** (different identity + lifecycle: Device stable/idempotent, LocalRunner per-start) — cleaner than one polymorphic event. Flag if you'd unify.

## Dependencies + sequencing
- **Depends on:** 1.1 (event store + migrations + backup/rollback + floor), 1.2 (projections + object_refs), 1.4 (PidLock) — all LANDED.
- **Independent of:** 1.6c (the §17 degradable replay refines `open()`'s read-path internals; bootstrap is agnostic to which replay strategy `open()` uses).
- **Blocks:** 1.6b (runtime-task spawns consume 1.6a-L2's `DaemonContext`: stale-socket reclaim + `bind()` + accept-loop + the outbox/reaper/subscribe spawns + `main.rs`).

## Estimated commit count
**3** (layer→layer multi-commit — drive each layer's RED→GREEN→commit, do NOT idle between layers):
- **L1** — eventstore typed-error cleanups (`user_version()→Result`, typed `RestoreFailed`). Daemon-internal, no contract bump.
- **L2** — `bootstrap.rs` cold-start orchestration + `DaemonVersionInfo`. Daemon-internal.
- **L3** — Device + LocalRunner registration events (CONTRACT_VERSION bump + projector fold). **UNBLOCKED 2026-06-08 (USER-RULED Option B). Sequence: after 1.6b lands, return for 1.6a-L3 to close task #11.**

Not bundled into fewer because L3 carries a cross-doc/contract change (atomic doc-edit pairing wants its own commit) and the INV-SEC-1 gate sits only on L3 — L1+L2 land while Q1 is parked for the user. Not split further because L1+L2 are small, same-area, and share the bootstrap context. **No single layer is a §15 safety-critical pin** (the §17 replay — the safety-critical part of 1.6 — is the separately-atomized 1.6c).

## Lessons-logged candidates anticipated
- **Convention candidate** — "Daemon self-registration at cold-start is a System-actor system event, not a Gateway Action; the Gateway governs proposer intents, not the daemon's own lifecycle substrate" (pending the lead's Q1 ruling).
- **Convention candidate** — "LocalRunner minted-per-start vs Device register-if-absent: bootstrap idempotency differs by object identity model (§5.3)."
- **Architecture-doc note candidate** — §16 cold-start ordering is realized; the version-compat *matrix* enforcing-floor is the DB user_version check; the agent-CLI/SDK + sidecar-MCP rows land at §9.1/§13.1.
- **Future TODO — operational** — the full §16 version-compat matrix (agent-CLI/SDK tuple, sidecar `initialize` check) accretes in Phase 3/8.

## How to invoke
1. Read this brief end-to-end (don't skip Step-2.5 — **Q1 is an INV-SEC-1 gate on L3**).
2. Run `/tdd cold_start_bootstrap`.
3. Step 0 (Restate) — confirm against the Feature line.
4. Step 1 — confirm the file list.
5. Step 2.5 — L1+L2 ✅ landed (`48b7a2c`/`f1de088`). **L3 UNBLOCKED (USER-RULED Option B) — send the L3 test-design write-up when you take it up.**
6. Sequence: 1.6a L1→L2 ✅ → 1.6b (in flight) → **return for 1.6a-L3** (registration; closes task #11) → 1.6c (§17 replay) → 1.7.
7. Step 9 — categorized flags + ship-ask; the CONTRACT_VERSION bump + EventTypeRegistry row are the orchestrator's hot-write.

# /tdd brief — integration_connections_projector (Wave-C, the projector)

## Feature
An event-fed projector that folds `IntegrationConnectionRegistered` (the edges-029 `integration.connect` emitter) into a `proj_integration_connection` read model (MIGRATION_11 lays the table), closing the Wave-C connection vertical (mutator → event → projection). **CONTRACT-neutral** (no `shared/` surface — the event is frozen 0.26; no `ProjectionName` variant).

## Use case + traceability
- **Task ID:** P7.1 (Wave-C — the connection-registry projector)
- **Architecture sections it implements:** `ARCHITECTURE.md §7.2` (read models / projections).
- **Related context:**
  - **The mechanism precedent is edges-028 / edges-022** (`projections/project_registry.rs` / `worktree.rs`) — a `Projector` impl, registered in `projectors()`, in-band fold, in `REBUILD_TABLES`, rebuild-equivalent. Like edges-028 (and UNLIKE edges-022) there is **NO LESSON-17 sibling-read** — `IntegrationConnectionRegistered` is self-contained.
  - **Key difference from edges-028:** the identity `connection_id` is on the **PAYLOAD** (the mutator minted it), NOT the envelope → the projector keys by `payload.connection_id` (edges-028 keyed by `env.project_id`).
  - **The frozen event:** `IntegrationConnectionRegistered{connection_id, provider, keychain_ref, account?}` (`shared/src/events.rs:490`, frozen 0.26).
  - **Cross-doc resolution (same as edges-028):** DATA_MODEL 2.8 classifies `integration_connections` as a *durable registry* (canonical, NOT rebuildable, with `workspace_id`/`scopes_json`/`expires_at`). For MVP this is an **event-fed projection** (`proj_integration_connection`, rebuildable, the proj_ convention) carrying the fields the event supplies; the durable-registry fuller model is DEFERRED. Named `proj_integration_connection` (proj_ = rebuildable projection) to avoid colliding with the DATA_MODEL 2.8 bare `integration_connections` durable name.
  - **Migration:** **MIGRATION_11** (the next contiguous slot after edges-028's MIGRATION_10).

## Acceptance criteria (what "done" means)
- [ ] **MIGRATION_11** (`schema::MIGRATION_11_INTEGRATION_CONNECTIONS`) creates `proj_integration_connection`; the `MIGRATIONS` array appends it; `SUPPORTED_USER_VERSION` 10 → 11. A fresh DB opens at v11; a v10 DB migrates to v11.
- [ ] An `IntegrationConnectionProjector` folds `IntegrationConnectionRegistered` (and ONLY that `event_type`) → a `proj_integration_connection` row keyed by `payload.connection_id`, carrying `provider` (the `Provider` wire value via `wire_value`), `keychain_ref`, `account`, `status='connected'` (the resting state for a register), `updated_at_seq = env.seq`.
- [ ] Registered in `projectors()` **and** the table in `REBUILD_TABLES` → **rebuild-equivalence** (incremental fold == full rebuild, byte-identical; LESSON 4 / 17).
- [ ] **Healthy SKIP** when the payload won't bind / `connection_id` is empty → `Decode`-degrade + skip (no row); the reason **never echoes payload bytes** (no-payload-echo). *(No envelope-identity skip needed — the identity is on the payload.)*
- [ ] **keychain_ref stays a pointer:** the projector writes `keychain_ref` through from the already-redacted committed event (it is a pointer by the edges-029 mutator's keychain_ref-pointer-only construction) and MUST NOT log it. No new secret surface.
- [ ] Re-register of the same `connection_id` UPSERTs (`ON CONFLICT(connection_id) DO UPDATE`) — advances `updated_at_seq`.
- [ ] All unit tests in `daemon/tests/integration_connections_proj.rs` pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
`projections::projectors()` (`mod.rs:78`) — add `Box::new(integration_connections::IntegrationConnectionProjector)`. Invoked by `apply_all` (in-band) + `rebuild`/`replay`. The feeding event `IntegrationConnectionRegistered` is emitted by the edges-029 `integration.connect` executor on the live `CatalogExecutor` → reachable end-to-end once both land (`/wired IntegrationConnectionRegistered`).

**IPC read = DEFERRED (CONTRACT-neutral, same as edges-028):** serving `proj_integration_connection` over IPC needs a `ProjectionName::IntegrationConnection` variant (a `shared/` change → CONTRACT bump). NOT added here. Forward-laid + fold/rebuild-reachable; the read RPC is a consumer-gated follow-on. At `/phase-exit 7` declare the IPC-unreachability as intentional gating, not drift.

## Files expected to touch
**New:**
- `daemon/src/projections/integration_connections.rs` — the `IntegrationConnectionProjector`.
- `daemon/tests/integration_connections_proj.rs` — the RED tests.

**Modified:**
- `daemon/src/eventstore/schema.rs` — `MIGRATION_11_INTEGRATION_CONNECTIONS` const + add the table to `REBUILD_TABLES`.
- `daemon/src/eventstore/migrations.rs` — `SUPPORTED_USER_VERSION` 10 → 11 + append `M::up(...)`.
- `daemon/src/projections/mod.rs` — `mod integration_connections;` + register in `projectors()`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/integration_connections_proj.rs`:

1. **`test_registered_folds_proj_integration_connection`** — append an `IntegrationConnectionRegistered{connection_id="conn_X", provider=github, keychain_ref="ref", account="me"}` → a row keyed `conn_X` with `provider="github"` (wire), `keychain_ref="ref"`, `account="me"`, `status="connected"`, `updated_at_seq=seq`.
   - Asserts: the fold; key from the payload. Why: §7.2 read-model fold; the edges-028 pattern.
2. **`test_provider_bound_via_wire_value`** — `provider=linear` → stored wire value `"linear"` (via `wire_value`, the layer-correct serde producer — the edges-022 precedent).
   - Asserts: the closed-enum wire binding. Why: LESSON 2 (wire value is the contract).
3. **`test_account_none_folds_null`** — `account=None` → the column is NULL (no row rejected).
   - Asserts: optional handling. Why: optional-as-null.
4. **`test_unbindable_payload_degrades`** — a payload that won't bind → degrade + skip; the reason has NO payload bytes.
   - Asserts: `Decode`-degrade (contained), generic reason. Why: no-payload-echo + reject-unknown norm.
5. **`test_reregister_upserts_and_advances_seq`** — a 2nd event for `conn_X` → REPLACED (DO UPDATE), higher `updated_at_seq`.
   - Asserts: 1 row, advanced seq. Why: idempotent re-fold.
6. **`test_rebuild_equivalence`** — incremental fold == `rebuild()`, byte-identical; the table is in `REBUILD_TABLES`.
   - Asserts: rebuild-equivalence. Why: LESSON 4 / 17.
7. **`test_migration_11_applies`** — a fresh store opens at `user_version=11`; `proj_integration_connection` exists; `SUPPORTED_USER_VERSION==11`.
   - Asserts: migration wired + applied. Why: LESSON 8.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **NONE** (CONTRACT-neutral — `IntegrationConnectionRegistered` frozen 0.26; no new `ProjectionName`/shared model). *(The CONTRACT 0.33 bump is edges-029's, not this slice.)*
- **Shared-contract seam touched?** No → no schema-snapshot test.
- **Orchestrator doc rows to write hot (HELD-for-merge PLAN-DELTA, `docs/planning/edges-R5-wiring-plan.md`):**
  - **Arch-note:** the MVP `integration_connections` is an event-fed projection (`proj_integration_connection`); the DATA_MODEL 2.8 durable-registry model (workspace_id/scopes_json/expires_at + a disconnect/refresh lifecycle) is DEFERRED.
  - **MIGRATION_11** registered (`proj_integration_connection`); `SUPPORTED_USER_VERSION` 10→11.
  - **Completed-work tick:** P7.1 Wave-C connection vertical CLOSED (mutator edges-029 + projector edges-030; the IPC read + credential-storage deferred).

## Things to flag at Step 2.5
1. **`status` column + value.** My default vote: store `status='connected'` (the DATA_MODEL 2.8 default; mutable-from-event-type for a future disconnect/expire event, LESSON 17). Flag if you'd omit `status` until a lifecycle event exists.
2. **Deferred DATA_MODEL 2.8 fields (`workspace_id`/`scopes_json`/`connected_at`/`expires_at`).** My default vote: **omit for MVP** (the event doesn't carry them; `workspace_id` could source from the envelope if present — confirm). The durable-registry fuller shape is deferred.
3. **IPC read surface.** My default vote: **DEFER** (no `ProjectionName::IntegrationConnection` — would bump CONTRACT; same as edges-028).
4. **Table name `proj_integration_connection` (proj_) vs bare `integration_connections`.** My default vote: **`proj_integration_connection`** (proj_ = rebuildable projection; the bare name is the deferred durable registry) — consistent with edges-028's `proj_project`/`proj_repository`.

## Dependencies + sequencing
- **Depends on:** edges-029 (the `integration.connect` mutator emits `IntegrationConnectionRegistered`) · edges-028 (MIGRATION_10 — this is MIGRATION_11, contiguous).
- **Blocks:** the deferred IPC read RPC · `/phase-exit 7`.
- **Sequencing:** R9 slice-3 (after edges-029). Non-safety projector → SKIP security-reviewer.

## Estimated commit count
**1.** Projector + MIGRATION_11 + tests = one logical unit. **No safety invariant touched** (read-model projector, no mutator/secret) → **security-reviewer = SKIP** per the `invariant`-only policy (the edges-022/028 precedent; the `keychain_ref` fold writes an already-pointer value — confirm at Step 2.5 it's not logged). `code-quality-reviewer` runs.

## Lessons-logged candidates anticipated
- **Convention candidate** — folds into the edges-028 event-fed-registry-projection convention (key by payload identity when the event carries it; proj_ naming; defer the durable model).
- **Future TODO — operational** — the IPC read RPC + the DATA_MODEL 2.8 durable-registry fuller model (workspace_id/scopes/expires + disconnect/refresh lifecycle) are consumer-gated follow-ons.

## How to invoke
1. Read end-to-end (answer Step 2.5 Q1–Q4 or take defaults).
2. Run `/tdd integration_connections_projector`.
3. Step 0 (Restate) → confirm against the Feature line.
4. Step 2.5 → ping back with answers + the coverage map.
5. Step 9 → categorized flags.

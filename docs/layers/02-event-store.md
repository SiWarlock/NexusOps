# Event store & audit spine (`daemon/src/eventstore/`)

## Executive summary

The event store is NexusOps' permanent memory: a single append-only SQLite log where every fact the daemon records is written once and never changed. It is the foundation of the audit guarantee — if something happened, there is an immutable, ordered event for it; if the event can't be written, the action fails rather than happening silently. Exactly one writable connection exists (the daemon's write-actor owns it); everything else reads through read-only connections. The same module owns the schema migrations (with automatic backup-and-restore so an upgrade can never destroy the log), a quarantine mechanism so one corrupt row can't stop the daemon from starting, and the transactional outbox — the only doorway through which events reach the outside world (files, and later GitHub/Linear/the Brain).

## Responsibilities

- **Owns the one writable DB connection** and the canonical append path: `seq` assignment, `event_id`/`recorded_at` minting, the §15 redaction gate, projection fold, and outbox rows — all in one transaction (`daemon/src/eventstore/mod.rs:139-145`, `:197-296`).
- **Owns schema evolution**: forward-only `user_version` migrations 1–6 with pre-migration backup + restore-on-failure and a "refuse newer DB" floor (`daemon/src/eventstore/migrations.rs:15-126`).
- **Owns degradable reads + the quarantine table** (§17): corrupt/unredacted rows are diverted and recorded, never crash `open()` (`daemon/src/eventstore/mod.rs:636-739`, `daemon/src/eventstore/schema.rs:323-330`).
- **Owns the transactional outbox**: delivery-intent rows written in the event-commit txn; an at-least-once drainer with backoff/dead-letter (`daemon/src/eventstore/outbox.rs`).
- **NOT** the redaction logic itself — it only *calls* the `Redactor` trait; the detectors live in `redaction.rs` (see [03-redaction.md](03-redaction.md)).
- **NOT** the projector bodies — it invokes `projections::apply_all`/`catch_up_replay`/`rebuild` but the fold logic lives in `daemon/src/projections/` (see [04-projections.md](04-projections.md)).
- **NOT** a policy/approval layer — it records whatever a caller appends. INV-SEC-1 enforcement is the Phase-2 Action Gateway's job (not yet built); today's callers are bootstrap/system events and tests.
- **NOT** a `shared/` contract surface for `outbox`/`leases`/`quarantine` — those tables are daemon-internal (`daemon/src/eventstore/schema.rs:274`, `:298-299`, `:321-322`).

## Key components

| Component | What it does | Where |
|-----------|--------------|-------|
| `EventStore` | Holds the single writable `Connection` + injected `IdGen`/`Clock`/`Redactor` seams | `daemon/src/eventstore/mod.rs:140-145` |
| `EventStore::open` | Pragmas → version floor → migrations → projection catch-up → outbox in-flight reset | `daemon/src/eventstore/mod.rs:151-174` |
| `AppendIntent` | What a caller supplies; the store assigns `event_id`/`seq`/`recorded_at` | `daemon/src/eventstore/mod.rs:91-113` |
| `EventStore::append` | The one-txn append: redaction gate → seq → INSERT → projections → outbox → commit | `daemon/src/eventstore/mod.rs:197-296` |
| `divert_quarantined` | §15 write-side divert: replaces an unredactable event with a content-free `SensitiveOutputRedacted` | `daemon/src/eventstore/mod.rs:305-347` |
| `EventStoreError` | Typed fail-closed errors incl. `RedactionRequired`, `RestoreFailed`, `DbNewerThanSupported` | `daemon/src/eventstore/mod.rs:55-87` |
| `DegradableEvent` | Per-row read result: `Ok` / `Degraded` (unknown version) / `Quarantined` (corrupt) | `daemon/src/eventstore/mod.rs:116-128` |
| `classify_degradable_row` | The §17 per-row classifier shared by all degradable reads; reasons are content-free | `daemon/src/eventstore/mod.rs:678-739` |
| `read_events_after_degradable` | The replay reader (`seq > offset`) that feeds catch-up/rebuild | `daemon/src/eventstore/mod.rs:636-653` |
| `emit_quarantine_audit_events` | Appends one loud `AuditIntegrityViolation` per un-emitted quarantine row, idempotently | `daemon/src/eventstore/mod.rs:416-479` |
| `apply_pragmas` | WAL + `synchronous=NORMAL` + FK on + 5s busy timeout (ADR-003) | `daemon/src/eventstore/schema.rs:13-20` |
| `MIGRATION_1_EVENTS` … `MIGRATION_6_QUARANTINE` | The six DDL migrations (events / redaction cols / projections / outbox / leases / quarantine) | `daemon/src/eventstore/schema.rs:24-330` |
| `migrations::run` | Backup-before-raise, migrate to latest, restore-on-failure | `daemon/src/eventstore/migrations.rs:68-90` |
| `backup_db` / `restore_db` | WAL-checkpointed copy to `nexusops.db.bak-<from>`; restore removes stale `-wal`/`-shm` | `daemon/src/eventstore/migrations.rs:35-56` |
| `outbox::write_for_event` | One `pending` row per subscribed destination, inside the append txn | `daemon/src/eventstore/outbox.rs:61-85` |
| `outbox::drain_once` | One drain pass: claim `in_flight` → deliver → delivered/failed(backoff)/dead | `daemon/src/eventstore/outbox.rs:187-282` |
| `JsonlMirror` | The only built destination: appends redacted payloads to a JSONL file | `daemon/src/eventstore/outbox.rs:114-143` |
| `open_read_only` | The read path for everyone else: `SQLITE_OPEN_READ_ONLY` + `query_only=ON` | `daemon/src/eventstore/mod.rs:593-599` |

## Interfaces & contracts

**Write surface (the spine):**

- `EventStore::open(path, idgen, clock, redactor) -> Result<EventStore, EventStoreError>` — the only constructor of a writable store (`mod.rs:151`). All three seams are injected so a fixed input replays a byte-identical log (pinned by `daemon/tests/eventstore.rs:266-307`).
- `append(AppendIntent) -> Result<EventId, EventStoreError>` (`mod.rs:197`). Caller provides type/payload/identity fields; store assigns `event_id`, gapless monotonic `seq`, `recorded_at`. Errors: `RedactionRequired` (§15 gate, `mod.rs:224-226`), `DuplicateIdempotencyKey` (`mod.rs:293`), `Write` (any constraint/IO failure — nothing persists, `daemon/tests/eventstore.rs:133-149`).
- `emit_quarantine_audit_events() -> Result<usize, _>` (`mod.rs:416`) — called by the *caller* after `open` (production: `bootstrap::cold_start`), not by replay itself.

**Read surface:**

- `read_all()` — strict, errors on a malformed row (`mod.rs:351-364`).
- `read_all_degradable()` — never aborts; returns `Ok`/`Degraded`/`Quarantined` per row (`mod.rs:369-384`).
- `first_event_of_type(t)` — the register-if-absent read used by bootstrap's `DeviceRegistered` reuse (`mod.rs:389-406`).
- `open_read_only(path)` — the contract for every non-writer (UI reads over IPC, tests): a reader physically cannot mutate the log (`mod.rs:593-599`; enforced by test `daemon/tests/eventstore.rs:116-128`).

**Outbox surface:** `drain_once(clock, dest) -> DrainSummary` (`mod.rs:180-186` → `outbox.rs:187`); destinations implement `Destination { name, deliver(payload) -> DeliveryOutcome }` (`outbox.rs:103-109`) and must be idempotent — delivery is at-least-once.

**Migration/version surface:** `user_version() -> Result<u32, _>` (typed, never a `-1` sentinel, `mod.rs:557-564`); `refuses_db_newer_than_supported(path)` (`mod.rs:578-581`); `backup_db`/`restore_db` re-exported for tests + bootstrap (`mod.rs:584-589`).

**Lease surface:** `acquire_lease`/`renew_lease`/`release_lease`/`validate_lease_held`/`reap_leases` (`mod.rs:492-553`) — thin wrappers driving `locks/` over the same single connection so no second writable connection ever exists. Semantics documented in [05-locks.md](05-locks.md).

**What it expects from callers:** valid JSON in `payload_json` (the `CHECK(json_valid(...))` at `schema.rs:48` makes a violation a typed write error), envelope enums from `shared/` (serialized fail-closed via `enum_wire`, `mod.rs:812-819`), and that *only* the write-actor holds the `EventStore`.

## Data & state

State lives in one SQLite file: **`~/Library/Application Support/NexusOps/nexusops.db`** — the path is `ARCHITECTURE.md:200` (§7); in code it's `production_base_dir()` (`daemon/src/main.rs:108-112`) joined with `DB_FILENAME = "nexusops.db"` (`daemon/src/bootstrap.rs:38`, `:146`). WAL mode, `synchronous=NORMAL`, FK on, 5s busy timeout (`schema.rs:13-20`; pinned by `daemon/tests/eventstore.rs:312-337`).

**`events` (migration 1, `schema.rs:24-61`)** — the §7.1 envelope: identity (`event_id` PK, `seq`), typing (`event_type`, `event_version`), two timestamps (`occurred_at` caller-supplied, `recorded_at` store-assigned), routing/identity (`workspace_id`, `project_id`, `actor_*`, `source_*`, `correlation_id`, `causation_id`, `session_id`, `agent_team_id`, …), `idempotency_key`, `sensitivity`, `visibility` (default `'project'`), `payload_json` with a `json_valid` CHECK, hash columns, `schema_version`, `app_version`. Six indexes incl. `ux_events_seq` (unique, `schema.rs:54`) and the **partial unique** `ux_events_idempotency ON events(idempotency_key) WHERE idempotency_key IS NOT NULL` (`schema.rs:59`) — the dedup mechanism; the UNIQUE violation is mapped to `DuplicateIdempotencyKey` by extended-code + column-name match (`mod.rs:821-830`).

**Migration 2 (`schema.rs:68-71`)** adds `redaction_status` + `redaction_engine_version`. The backfill DEFAULT is deliberately `'unredacted'` — honest provenance for pre-gate rows, never falsely `'redacted'`.

**Migration 3 (`schema.rs:80-267`)** — `object_refs`, `projection_offsets`, and the 10 MVP projection read models (11 physical `proj_*` tables). Owned by [04-projections.md](04-projections.md).

**Migration 4 — `outbox` (`schema.rs:275-288`)**: `outbox_id` (`out_` ULID, daemon-internal, *not* one of the 22 frozen contract IDs), `destination`, `event_id` FK, `payload_json`, `status ∈ pending|in_flight|delivered|failed|dead`, `retry_count`, `next_attempt_at` (NULL = due now, drives `ix_outbox_due`), `last_error`, `created_at`.

**Migration 5 — `leases` (`schema.rs:300-312`)**: see [05-locks.md](05-locks.md).

**Migration 6 — `quarantine` (`schema.rs:323-330`)**: `seq` PK, structural `reason`, `detected_at`, `audit_emitted` flag (dedups the `AuditIntegrityViolation` emission across restarts and rebuilds).

`SUPPORTED_USER_VERSION = 6` (`migrations.rs:15`); a DB with a higher `user_version` is refused (`migrations.rs:116-126`). Pre-migration backups are versioned siblings: `nexusops.db.bak-<from>` (`migrations.rs:58-62`).

## Dependencies

- **Depends on:** `nexusops_shared` (the §7.1 `EventEnvelope`, enums, prefixed-ULID IDs, event-type structs — [01-shared-contracts.md](01-shared-contracts.md)); `crate::clock`/`crate::idgen` (injected determinism seams, §14); `redaction::Redactor` (the §15 gate input — [03-redaction.md](03-redaction.md)); `crate::projections` (called for in-band fold/catch-up/rebuild — [04-projections.md](04-projections.md)); `crate::locks` (driven over the same connection — [05-locks.md](05-locks.md)); `rusqlite` + `rusqlite_migration`.
- **Used by:** `bootstrap::cold_start` (opens the store, then calls `emit_quarantine_audit_events` — [07-daemon-runtime.md](07-daemon-runtime.md)); the runtime `WriteActor` (sole long-lived owner of the `EventStore`; the drainer/reaper interval loops call `drain_once`/`reap_leases` through its handle — `daemon/src/main.rs:60-78`); the IPC read methods, which use `open_read_only` only ([06-ipc.md](06-ipc.md)); the Phase-2 Action Gateway will be the production caller of the lease/append surface (not yet built).

## How it works (flow)

The load-bearing path is **append** — one transaction, four facts that commit or roll back together:

```
AppendIntent
   │
   ▼
1. enum_wire(...)  fail-closed enum serialization        mod.rs:200-208
2. redactor.redact(payload)                              mod.rs:212
     ├─ quarantine signal → divert_quarantined (SOR)     mod.rs:218-222, 305-347
     ├─ status != Redacted → Err(RedactionRequired)      mod.rs:224-226
     └─ redacted payload continues
3. mint event_id + recorded_at                           mod.rs:229-230
4. BEGIN IMMEDIATE                                        mod.rs:234-237
     seq = SELECT COALESCE(MAX(seq),0)+1                  mod.rs:238-242
     INSERT INTO events (redacted payload)                mod.rs:243-274
     read row back → fold ALL projections (same txn)      mod.rs:283-284
     write outbox rows (same txn, redacted payload)       mod.rs:288
   COMMIT                                                 mod.rs:289
```

- **Atomic seq:** the `MAX(seq)+1` read and the INSERT share one `BEGIN IMMEDIATE` transaction, so ordering is atomic at the DB level, not merely by Rust's `&mut` (`mod.rs:232-242`; gapless monotonic order pinned by `daemon/tests/eventstore.rs:71-111`).
- **One-txn invariant:** event + projection fold + outbox rows commit together; a redaction-gate abort persists *nothing* — no event, no projection rows, no outbox row (`mod.rs:281-295`; pinned by `daemon/tests/outbox.rs:118-174`).
- **Fail-closed audit write:** any INSERT failure (e.g. the `json_valid` CHECK) returns a typed error with zero rows persisted (`daemon/tests/eventstore.rs:133-149`) — the substrate for safety rule #5.

**Open / recovery path** (`mod.rs:151-174`): pragmas → §16 refuse-newer floor → `migrations::run` (backup `.bak-<from>` if `from ∈ 1..6` holds data, restore + classify on failure — `migrations.rs:68-113`; the classifier `on_migration_failure` distinguishes a clean rollback from a `RestoreFailed`, unit-pinned at `migrations.rs:136-168`) → projection catch-up replay → `outbox::reset_in_flight` (crash recovery: stuck `in_flight` rows become `pending` again, `outbox.rs:165-172`, called at `mod.rs:167`).

**Degradable replay** (§17 Option C): the replay reader classifies each row (`mod.rs:678-739`). Unknown `event_version > 1` → `Degraded` (forward-compat, not an integrity violation, `mod.rs:706-713`); corrupt or `redaction_status='unredacted'` → `Quarantined` (`mod.rs:714-737`). The projections layer records quarantines (`ON CONFLICT(seq) DO NOTHING`), marks the offset degraded, skips the row, and **the daemon starts** — pinned end-to-end by `daemon/tests/replay.rs:141` (corrupt row → open recovers, raw spine untouched), `:197` (unredacted row never folded), `:226` (healthy logs replay unchanged). After open, `emit_quarantine_audit_events` (`mod.rs:416-479`) appends one `AuditIntegrityViolation` per `audit_emitted=0` row through the normal append path (gate + fold run on it), with `audit-integrity-{seq}` idempotency keys — exactly one event per seq across restarts *and* rebuilds (`replay.rs:310`, `:347`, `:391`).

**Outbox drain** (`outbox.rs:187-282`): select up to `DRAIN_BATCH_LIMIT = 128` due rows (`:152`, `:199-204`) → commit the `in_flight` claim *before* delivering (`:228-232`, so a mid-delivery crash leaves a resettable row) → classify: `Delivered` → `delivered`; `Retryable` → `failed` with exponential backoff `30·2^n` capped at 3600s (`:156-159`) until `retry_count > MAX_RETRIES = 5` (6 total attempts, `:147`, pinned by `daemon/tests/outbox.rs:380-407`) → `dead`; `Terminal` → `dead` immediately (`:271-278`). At-least-once across crashes with idempotent consumers is pinned by `outbox.rs` test 9 (`daemon/tests/outbox.rs:412-456`).

## Design decisions & rationale

- **Event log as the spine, projections derived** (ARCHITECTURE §7/§7.2): raw `events` are irreplaceable; everything else is rebuildable. `rebuild_projections` truncates only the const `REBUILD_TABLES` list (`daemon/src/projections/schema.rs:12-26`) — `events`, `outbox`, `leases`, and `quarantine` are deliberately absent, so a rebuild can never mutate history or resurrect delivery intents (`outbox.rs:14-18`; pinned at `daemon/tests/outbox.rs:140-147`).
- **Single writer, atomic seq** (§15 / forbidden-pattern #3 / LESSONS §3): one writable connection, everyone else `open_read_only`; chosen over per-caller connections because concurrent SQLite writers would break canonical ordering.
- **In-band projection fold + transactional outbox** (§7, LESSONS §4/§5): folding inside the commit means a reader never sees an event whose read models lag, and an outbox row exists iff its event does ("recorded-iff-intended"). The alternative — async fan-out — would reintroduce dual-write races.
- **Redaction gate *at* INSERT** (§15, safety rule #3): the gate is inside `append`, so there is no code path that persists `redaction_status='unredacted'`. Quarantine-divert (write-side) trades losing a whole event against ever persisting an unboundable secret; mask-in-place is preferred when boundable (LESSONS §13).
- **Forward-only migrations with backup/restore** (§16): no down-migrations — the log is too valuable; instead a WAL-checkpointed `.bak-<from>` copy before any raise of a data-bearing DB, restored (and the failure *typed*, never swallowed) if the migration fails (`migrations.rs:68-113`).
- **Degrade, don't abort** (§17, user-ruled Option C, LESSONS §11): a corrupt row is recorded loudly (quarantine row + `AuditIntegrityViolation` event) but never blocks startup; the rejected alternatives were abort-on-corruption (one bad row bricks the daemon) and silent-skip (an invisible audit gap).
- **Outbox is the only external path** (§12/§17): an INV-SEC-1 analogue for side-effects — the drainer only delivers rows read from `outbox` (`daemon/tests/outbox.rs:274-286`), and every payload derives from the already-redacted event with filter-only transforms (`outbox.rs:44-55`), so the outbox is a §15 *sync* sink by construction.

## Gotchas & sharp edges

- **`seq` is `MAX(seq)+1`, not AUTOINCREMENT** — correctness depends on the `BEGIN IMMEDIATE` write lock *and* the single-writer discipline. A second writable connection would be a real hazard; don't open one (forbidden #3).
- **The `in_flight` claim is autocommit, not part of a claim+deliver transaction** (`outbox.rs:228-232`) — safe only under the one-drainer-per-destination model; a concurrent drainer would need an atomic claim (noted at `outbox.rs:184-186`).
- **Backoff due-ness is a lexical string compare** on RFC3339 (`next_attempt_at <= ?2`, `outbox.rs:202`) — correct only for same-offset (`Z`) timestamps; the injected `Clock` guarantees this (LESSONS §5).
- **Quarantine reasons must stay content-free** (§15): `classify_degradable_row` deliberately discards serde errors (which would quote the offending value) in favor of structural strings (`mod.rs:673-677`, `:734-737`; pinned by `replay.rs:291-307`). Preserve this when adding classifications.
- **`UNKNOWN_SEQ = -1`** is a named marker used *only* when the `seq` column itself is unreadable (`mod.rs:47-51`); a readable seq is always preserved verbatim.
- **The divert recursion guard**: a quarantine signal on a `SensitiveOutputRedacted` event itself is `RedactionRequired`, never a divert-of-a-divert (`mod.rs:218-221`); divert dedup keys are namespaced `divert-{k}` to avoid colliding with the original key (`mod.rs:336-340`).
- **Migration-2 backfill default is `'unredacted'`** (`schema.rs:68-71`) — *new* rows are always `'redacted'` via the gate, but a legacy row reads back as quarantined on replay (the replay-side §15 defense, `mod.rs:723-730`).
- **`fullfsync` is OFF** (`schema.rs:12`, §18 caveat) — `synchronous=NORMAL` under WAL accepts a small durability window on macOS power loss.
- **Unpopulated envelope columns**: `append` never writes `causation_id`, `action_request_id`, `approval_id`, `workflow_run_id`, `payload_hash`, `previous_event_hash`, or `app_version` (INSERT column list at `mod.rs:244-249`; `AppendIntent` doesn't carry them) — they're NULL despite existing in the DDL. The `schema.rs:52` comment says `app_version` is "populated at daemon bootstrap (Phase 1.6)", which has **not** happened — minor code-vs-comment drift. The hash-chain columns presumably await the Gateway phases (UNVERIFIED — no implementation or task anchor found in this layer).
- **Architecture-vs-code gap**: ARCHITECTURE §7 (`ARCHITECTURE.md:200`) lists `artifacts`, `tasks`, registry tables, `action_requests`/`approvals`, and `harness_session_map` — none exist in migrations 1–6 yet (later-phase tables, not yet built). Likewise §12's real outbox destinations: `destinations_for` returns only `jsonl_mirror` (`outbox.rs:36-38`); brain_mcp/github/linear/notifier adapters are explicitly re-homed to their producing phases.
- **`fts_events` exists from migration 1** (`schema.rs:60`) and is truncated on rebuild, but whether any projector populates it is owned by [04-projections.md](04-projections.md).

## Connects to

- **[01-shared-contracts.md](01-shared-contracts.md)** — the `EventEnvelope`, enums, prefixed IDs, and per-type payload structs (`SensitiveOutputRedacted`, `AuditIntegrityViolation`) this store persists; handoff at `mod.rs:21-29` imports.
- **[03-redaction.md](03-redaction.md)** — the `Redactor` trait called at the §15 gate (`mod.rs:212`) and the quarantine-divert signal it can return (`mod.rs:218-222`).
- **[04-projections.md](04-projections.md)** — `apply_all` inside the append txn (`mod.rs:284`), `catch_up_replay` at open (`mod.rs:164`), `rebuild` (`mod.rs:191-193`), all fed by `read_events_after_degradable` (`mod.rs:636`).
- **[05-locks.md](05-locks.md)** — the lease/fencing API the store fronts over its single connection (`mod.rs:481-553`); migration 5 DDL lives here (`schema.rs:300-312`).
- **[06-ipc.md](06-ipc.md)** — IPC read methods consume the DB via `open_read_only` (`mod.rs:593-599`), never the writer.
- **[07-daemon-runtime.md](07-daemon-runtime.md)** — `cold_start` opens the store and emits quarantine audits; the `WriteActor` owns it; drainer/reaper loops call `drain_once`/`reap_leases` (`daemon/src/main.rs:60-78`, `daemon/src/bootstrap.rs:146`).

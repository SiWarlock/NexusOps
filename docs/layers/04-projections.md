# Projection engine & read models (`daemon/src/projections/`)

## Executive summary

The projection engine turns the daemon's append-only event log into the tables the UI actually reads. Every fact in NexusOps is an immutable event; nobody wants to re-scan thousands of events to answer "what sessions are active right now?" — so this layer "folds" each event into small, queryable read models (`proj_session`, `proj_audit_trail`, a project graph, per-project counters) the moment the event is committed. The fold happens **inside the same database transaction** that writes the event, so a reader can never observe an event whose read models haven't caught up. The read models are disposable: they can be rebuilt from scratch by replaying the log, and a broken projector degrades quietly instead of taking the daemon down. The raw `events` table is never touched by anything in this layer — projection corruption can never corrupt the audit spine.

## Responsibilities

- **Accountable for:** deriving every `proj_*` read model + `object_refs` from the event stream; advancing each projector's cursor (`projection_offsets.last_seq`) in the same transaction as its rows; degraded-skip containment of projector logic failures; startup catch-up replay and full rebuild; recording quarantined rows during replay (`daemon/src/projections/mod.rs:1-12`, `:200-265`).
- **NOT accountable for:** writing or mutating raw `events` (explicitly forbidden — `mod.rs:8-9`, `object_refs.rs (module doc)`); redaction (runs **before** the fold, in the event store — `daemon/src/eventstore/mod.rs:277-284`); serving reads over IPC (that's `ipc/methods.rs`); emitting the `AuditIntegrityViolation` event for quarantined rows (the caller of `open()` does that — this layer only records the quarantine row); owning the migration DDL (lives in `eventstore/schema.rs` to keep the layer-dependency direction clean — `daemon/src/projections/schema.rs:3-5`).

## Key components

| Component | What it does | Where |
|-----------|--------------|-------|
| `Projector` trait | one read-model fold: `apply(tx, env)` inside the caller's txn; idempotent under replay | `daemon/src/projections/mod.rs:51-59` |
| `ProjectionError` | typed failure: `Db` (fail append closed), `Decode` (degrade+skip), `OffsetAnomaly` (fail closed) | `daemon/src/projections/mod.rs:31-45` |
| `projectors()` registry | the 5 registered projectors in apply order — `object_refs` deliberately first | `daemon/src/projections/mod.rs:66-74` |
| `apply_all` | the §7 fan-out: fold one event into every registered read model in the caller's txn | `daemon/src/projections/mod.rs:80-85` |
| `apply_one` | per-projector SAVEPOINT bracket: rows + offset advance succeed or roll back together | `daemon/src/projections/mod.rs:94-130` |
| `advance_offset` / `mark_degraded` / `expect_one_row` | same-txn cursor advance; sticky `degraded` flag; ≠1-row offset write fails closed | `daemon/src/projections/mod.rs:145-181` |
| `wire_value` | render a frozen contract enum to its canonical snake_case wire string, fail-closed | `daemon/src/projections/mod.rs:185-192` |
| `catch_up_replay` / `rebuild` / `replay` | startup catch-up (strict `seq > last_seq`) and full rebuild (truncate + replay-all) | `daemon/src/projections/mod.rs:200-250` |
| `record_quarantine` | record a corrupt/unredacted replay row (`ON CONFLICT(seq) DO NOTHING`) | `daemon/src/projections/mod.rs:257-265` |
| `REBUILD_TABLES` | the 13 derived tables a full rebuild truncates (`events` deliberately absent) | `daemon/src/projections/schema.rs:12-26` |
| `ObjectRefsProjector` | derives event→object edges from typed envelope/payload fields (never caller-supplied) | `daemon/src/projections/object_refs.rs:18-67` |
| `SessionProjector` | upserts `proj_session` from `SessionStarted`; status binds the frozen §5.1 Session enum | `daemon/src/projections/session.rs:15-57` |
| `GraphProjector` | folds `proj_graph_node`/`proj_graph_edge` from this event's `object_refs` rows (same txn) | `daemon/src/projections/graph.rs:13-56` |
| `AuditProjector` | one redaction-safe headline row per event + FTS5 index of the headline (never raw payload) | `daemon/src/projections/audit.rs:19-74` |
| `ActivityProjector` | per-project counters; increment-only until Phase 3; **not** idempotent under re-fold | `daemon/src/projections/activity.rs:19-42` |
| `MIGRATION_3_PROJECTIONS` | DDL for `object_refs`, `projection_offsets`, and all 10 read models (11 physical `proj_*` tables) | `daemon/src/eventstore/schema.rs:80-267` |

## Interfaces & contracts

This layer is `pub(crate)`/crate-internal — the event store is its only production caller. Surface:

- **`apply_all(tx, env) -> Result<(), ProjectionError>`** (`mod.rs:80-85`) — input: the open event-commit `Transaction` + the just-persisted (already-redacted) `EventEnvelope`, read back from the row so append and rebuild fold byte-identical envelopes (`eventstore/mod.rs:281-284`). Output: rows in every registered read model + advanced offsets, or `Db`/`OffsetAnomaly` error that aborts the whole append.
- **`catch_up_replay(conn)`** (`mod.rs:200-202`) — called once per `EventStore::open` (`eventstore/mod.rs:164`); folds each projector's pending tail (`seq > last_seq`); no-op on a current log.
- **`rebuild(conn)`** (`mod.rs:208-210`) — exposed as `EventStore::rebuild_projections` (`eventstore/mod.rs:191-193`); truncate + replay-all; raw `events` untouched.
- **Expects from the event store:** the replay read path `read_events_after_degradable` (`eventstore/mod.rs:636`), which classifies each row as `Ok` / `Degraded` (unknown `event_version`) / `Quarantined` (corrupt or unredacted) via `DegradableEvent` (`eventstore/mod.rs:116-128`); and `MAX_SUPPORTED_EVENT_VERSION = 1` (`eventstore/mod.rs:44`) for the in-band unfoldable-version check (`mod.rs:101-104`).
- **Contract binding:** status columns are plain `TEXT` in DDL, but projectors fail-closed-bind them to the frozen §5.1 enums before write via `wire_value` / typed payload deserialization (`session.rs:33-35`, `mod.rs:185-192`) — an unknown wire value is a `Decode`, never stored raw (§15 reject-unknown).

## Data & state

All state lives in the daemon's single SQLite DB, created by `MIGRATION_3_PROJECTIONS` (`daemon/src/eventstore/schema.rs:80-267`):

| Table | Notes |
|---|---|
| `object_refs` | normalized event→object edges; PK `(event_id, object_type, object_id)`; FK to `events` (`eventstore/schema.rs:82-88`) |
| `projection_offsets` | per-projector cursor: `last_event_id, last_seq, last_processed_at, state ∈ healthy\|rebuilding\|degraded` (`eventstore/schema.rs:91-98`) |
| `proj_project_activity` | per-project counters (`eventstore/schema.rs:101-112`) — fed |
| `proj_session` | derived session state; `status` = §5.1 Session(17) (`eventstore/schema.rs:115-139`) — fed |
| `proj_approval_queue` | §5.1 Approval(10) (`eventstore/schema.rs:142-158`) — DDL only, body → 2.1 |
| `proj_worktree` | two-axis Worktree (`eventstore/schema.rs:161-179`) — DDL only, body → 5.2 |
| `proj_plan_progress` | §5.1 Task (`eventstore/schema.rs:182-194`) — DDL only, body → 9.2 |
| `proj_pull_request` | GitHub-synced cache (`eventstore/schema.rs:198-209`) — DDL only, body → 7.1 |
| `proj_graph_node` + `proj_graph_edge` | ProjectGraph = 2 physical tables (`eventstore/schema.rs:212-227`) — fed |
| `proj_agent_team` | §5.1 AgentTeam (`eventstore/schema.rs:230-236`) — DDL only, body → 9.3 |
| `proj_audit_trail` | rendered redaction-safe timeline (`eventstore/schema.rs:239-250`) — fed |
| `proj_usage_ledger` | tokens/cost rollups (`eventstore/schema.rs:253-266`) — DDL only, body → 3.1 |

Also written by this layer during replay: `quarantine` (seq PK, structural reason, `detected_at`, `audit_emitted`; MIGRATION_6, `eventstore/schema.rs:314-324`; insert at `projections/mod.rs:257-265`). The FTS index `fts_events(event_id UNINDEXED, body)` is a MIGRATION_1 scaffold (`eventstore/schema.rs:60`) populated by the audit projector.

10 read models have DDLs (11 physical `proj_*` tables) but only **4 projector bodies exist** (Session, Graph, Audit+FTS, Activity — plus `object_refs`). The other 6 were deliberately re-homed to the phases that produce their feeding events: ApprovalQueue→2.1, UsageLedger→3.1, Worktree→5.2, PullRequest→7.1, PlanProgress→9.2, AgentTeam→9.3 (`projections/mod.rs:61-65`; human-approved scope at `IMPLEMENTATION_PLAN.md:206`). All DDLs landed early so the UI track could fixture every schema.

## Dependencies

- **Depends on:** `nexusops-shared` for `EventEnvelope` + typed event payloads (`SessionStarted`, `DeviceRegistered`, `LocalRunnerRegistered`, the §5.1 status enums) — the contract shapes the fold binds against (`session.rs:10-11`, `object_refs.rs:13-14`); `eventstore` for `DegradableEvent`, `MAX_SUPPORTED_EVENT_VERSION`, and the degradable replay reader (`mod.rs:26`, `:101`, `:234`). Note the deliberate inversion-avoidance: the migration DDL lives in `eventstore/schema.rs` so the migration registry never imports `projections/` (`projections/schema.rs:3-5`).
- **Used by:** `EventStore::append` — in-band fan-out at `eventstore/mod.rs:284`; `EventStore::open` — catch-up at `eventstore/mod.rs:164`; `EventStore::rebuild_projections` at `eventstore/mod.rs:191-193`. Downstream (read-only, not a code dependency into this module): the IPC `get_projection` method reads the `proj_*` tables over a read-only WAL connection via a closed enum→table map (`daemon/src/ipc/methods.rs:25-36`, `:101`).

## How it works (flow)

The in-band path (one event append):

```
EventStore::append
  redact payload (§15 gate) ──► INSERT INTO events            eventstore/mod.rs:243-274
  read the row back as an EventEnvelope                       eventstore/mod.rs:283
  apply_all(tx, env)                                          eventstore/mod.rs:284
    for each projector (object_refs → session → graph
                        → audit_trail → project_activity):    projections/mod.rs:66-74, 80-85
      ensure_offset_row (lazy seed, last_seq=0)               projections/mod.rs:134-141
      event_version > 1? → mark_degraded + skip               projections/mod.rs:101-104
      SAVEPOINT "<name>"                                      projections/mod.rs:111
        p.apply(tx, env)        ─ Ok  → advance_offset, RELEASE   :113-117
                                ─ Decode → ROLLBACK TO, degrade+skip :118-122
                                ─ Db/OffsetAnomaly → ROLLBACK TO, propagate (append aborts) :125-128
  outbox rows join the same txn                               eventstore/mod.rs:288
  COMMIT                                                      eventstore/mod.rs:289
```

Key step details:

1. **Order matters:** `ObjectRefsProjector` runs first (`mod.rs:66-74`) because `GraphProjector` SELECTs the `object_refs` rows written earlier in the same txn (`graph.rs:25-32`) and folds them into nodes + a `project --owns--> session` edge (`graph.rs:34-53`).
2. **Refs are derived, never caller-supplied:** `derive_refs` reconstructs edges from the envelope's typed identity columns (or, for `dev_`/`lr_` registration events, from the typed payload) so the append path and a rebuild produce identical rows (`object_refs.rs:43-67`).
3. **Offset never ahead of rows:** the SAVEPOINT brackets *both* the projector's rows and its `last_seq` advance, so a failure rolls them back together (`mod.rs:87-93`, `:111-122`; pinned by `test_offset_never_ahead_of_rows`, `daemon/tests/projections.rs:290`).
4. **Recovery:** on every `open`, `catch_up_replay` folds each projector's pending tail with strict `seq > last_seq` (`mod.rs:200-202`, `:230-247`) — idempotent on a current log (`tests/projections.rs:623`). `rebuild` first truncates the 13 `REBUILD_TABLES` + `projection_offsets` (`mod.rs:215-221`, `schema.rs:12-26`), then replays everything; result is byte-equivalent to the incremental fold (`tests/projections.rs:516`) and raw `events` are untouched (`tests/projections.rs:541`).
5. **Degradable replay (§17 Option C):** the replay tail is read via `read_events_after_degradable` — a `Degraded` (unknown-version) row marks the projector degraded and continues; a `Quarantined` (corrupt/unredacted) row additionally records a `quarantine` row (`mod.rs:236-245`, `:257-265`). `open()` never aborts on a single bad row.

## Design decisions & rationale

- **In-band (synchronous) fold, not an async worker** — ARCHITECTURE.md §7 (`ARCHITECTURE.md:202`): "a single event may update multiple projections within the one event-commit transaction." Trade-off: every append pays the fold cost, but the UI can never read an event whose projections lag, and there is no separate worker/queue to crash or race. The single-writer model (forbidden-pattern #3, `daemon/CLAUDE.md`) makes this cheap.
- **Per-projector SAVEPOINT containment** — a projector *logic* bug (bad payload, unknown enum) must not block the audit spine, but a *DB* failure must fail the append closed (§15/§17 fail-closed on audit-write). The `Decode`-vs-`Db` split in `ProjectionError` encodes exactly that policy (`mod.rs:28-45`, `:118-128`).
- **`degraded` is sticky** — once degraded, a projector keeps folding later events but stays flagged until a rebuild heals it (`mod.rs:143-158`), so missing rows are visible rather than silently partial (§7.2).
- **Offsets, not truncate-and-replay-always** — `projection_offsets.last_seq` advanced same-txn (DATA_MODEL §2.4) gives crash-healing for free: an un-advanced offset simply re-folds its tail on next open (`mod.rs:197-199`).
- **`OffsetAnomaly` fail-closed** — a silently missing offset row would strand `last_seq` at 0 and make the non-idempotent `ActivityProjector` counter double-count on every reopen; ≠1-row offset writes abort instead (`mod.rs:168-181`; structurally unreachable backstop, tested at `tests/projections.rs:290`).
- **Bodies re-homed, DDLs kept** — building 6 projectors before their feeding events exist would mean testing against invented contracts; keeping all DDLs lets the UI fixture every schema now (`mod.rs:61-65`, `IMPLEMENTATION_PLAN.md:206`).
- **Audit rows are rendered headlines, not payloads** — indexing the redaction-safe headline (never `payload_json`) into FTS keeps search from becoming a secret-searchable dump (`audit.rs:1-10`, §9/§4.5).
- Full prose for the engine decisions is banked as LESSONS §4 (`daemon/LESSONS.md`).

## Gotchas & sharp edges

- **`ActivityProjector` is NOT idempotent** — it increments a counter, unlike every other (upsert/INSERT-OR-IGNORE) projector. The strict `seq > last_seq` replay guard is the *only* thing preventing double-counting on restart; weaken it and counters silently inflate (`activity.rs:6-11`; pinned by `tests/projections.rs:476` + `:623`).
- **Registry order is load-bearing** — `GraphProjector` reads rows `ObjectRefsProjector` wrote in the same txn. Reordering the `projectors()` vec breaks the graph silently (`mod.rs:61-68`, `graph.rs:1-5`).
- **A `SessionStarted` missing `session_id`/`project_id` is a healthy no-op, not a degrade** (`session.rs:26-30`) — the row simply doesn't project. Don't expect every event to leave a `proj_session` trace.
- **Decode reasons must never echo payload bytes** — the error text could land in logs; projectors use generic messages (`mod.rs:35-36`, `session.rs:32-34`).
- **FTS drift (arch-noted):** DATA_MODEL §2.11 specifies a contentless `events_fts content='proj_audit_trail'`; the code reuses the standalone MIGRATION_1 `fts_events(event_id, body)` scaffold with a DELETE-then-INSERT idempotency dance (FTS5 has no PK) — a deliberate Q3 deviation, flagged in-code (`audit.rs:8-10`, `:49-57`).
- **Schema-shape drift (arch-noted):** `proj_pull_request` + `proj_agent_team` shapes were authored at 1.2 with no §2.3 sketch to follow — reconciliation into DATA_MODEL §2.3 is a flagged follow-up (`eventstore/schema.rs:78-79`, `:196-197`, `:229`).
- **`stale` is recomputed, not replayed** — §7.2's field-level note (`ARCHITECTURE.md:226`): `proj_session.status` is event-derived/replayable, but `stale` is daemon-time-derived and recomputed on rebuild. **Not yet visible in code** — the DDL has no `stale` column (`eventstore/schema.rs:115-138`) and only `SessionStarted` folds today; this lands with the Phase-3 session-lifecycle events. Architecture contract, not current behavior.
- **`record_quarantine` uses `ON CONFLICT(seq) DO NOTHING`, never REPLACE** — re-detection across rebuild/catch-up must not reset `audit_emitted`, or the `AuditIntegrityViolation` event would emit more than once per seq (`mod.rs:252-265`).
- **`detected_at` is a DB-side timestamp** — daemon-internal, explicitly not replay-deterministic (`mod.rs:256`, `:260`).
- **15 tests** pin this layer (`daemon/tests/projections.rs`): migration shape (:119, :156), identity persistence (:202), in-txn fold (:236), ref derivation (:262), offset-never-ahead (:290), redaction-gate ordering (:329), FTS population (:354), single-event fan-out (:376), counter increment (:411), catch-up (:476), rebuild-equivalence (:516), raw-events-untouched (:541), unknown-version degrade (:567), catch-up no-op (:623).

## Connects to

- **[02-event-store.md](02-event-store.md)** — the sole production caller: `apply_all` runs inside `EventStore::append`'s commit txn (`eventstore/mod.rs:284`), after the redaction gate and before the outbox write; `catch_up_replay` runs in `EventStore::open` (`eventstore/mod.rs:164`); the degradable replay reader + `quarantine` table live there.
- **[03-redaction.md](03-redaction.md)** — projectors only ever see already-redacted payloads; the §15 gate runs strictly before the fold (`eventstore/mod.rs:277-279`).
- **[01-shared-contracts.md](01-shared-contracts.md)** — `EventEnvelope`, typed payloads, and the frozen §5.1 status enums that `wire_value`/serde bind status columns to (`session.rs:33-35`).
- **[06-ipc.md](06-ipc.md)** — the read side: `get_projection` maps the closed `ProjectionName` enum to these tables over read-only WAL (`daemon/src/ipc/methods.rs:25-36`); `subscription_push` deltas notify the UI after commit.
- **[07-daemon-runtime.md](07-daemon-runtime.md)** — the write-actor that owns the single writable connection these folds execute on; cold-start ordering (pidlock → open → catch-up).
- **[08-ui.md](08-ui.md)** — the ultimate consumer: the Tauri UI renders these read models and never reads `events` directly.

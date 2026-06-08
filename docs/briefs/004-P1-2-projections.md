# /tdd brief — projection_engine

## Feature
The projection engine — the read-model side of the event-sourced core. Folds the
append-only `events` log into derived `proj_*` read tables **in-band, inside the
event-commit transaction**, advancing `projection_offsets` in the same txn;
populates the normalized `object_refs` edges; and recovers via startup catch-up
replay, full rebuild, and degraded-skip handling. The UI (Phase 6) reads these
projections; nothing treats a projection as truth when a live source exists (§7.2).

## Use case + traceability
- **Task ID:** P1.2
- **Architecture sections it implements:** `ARCHITECTURE.md §7` (data & state model —
  "a single event may update multiple projections **within the one event-commit
  transaction**"), `§7.2` (source-of-truth matrix + re-read invariant — `proj_*` are
  rebuildable, tracked by `projection_offsets`), `§6.4` (IPC wire contract — the
  read surface consumes projections; reserved for 1.5), `§5.1` (status enums the
  projection status columns mirror).
- **Data-model anchors:** `DATA_MODEL.md §2.2` (object_refs), `§2.3` (the 10 `proj_*`
  tables), `§2.4` (projection_offsets — "advance `last_seq` in the **same transaction**
  as the rows it writes"), `§7.2` (rebuild & crash recovery — startup replay / full
  rebuild / degraded-skip, "projection corruption must not corrupt raw events").
- **Demo trace:** PRD §25 demo **step 7** — a single `SessionStarted` fans out to
  `proj_session` **and** `proj_project_graph` in one txn (the load-bearing
  integration test).
- **Related context:** consumes the 1.1 event store (`df753aa`/`b998f20`/`61089ea`,
  session doc `002`). The writer (`daemon/src/eventstore/mod.rs::append`) is extended
  in-band; **the §15 redaction gate + atomic `seq` logic are NOT touched** — projection
  apply runs *after* the events INSERT (on the already-redacted payload), before commit.

## Acceptance criteria (what "done" means)
- [ ] A new migration (user_version **3**) creates `object_refs`, `projection_offsets`,
      and all **10** `proj_*` tables (DATA_MODEL §2.2/§2.3/§2.4) — forward-only, over an
      existing 1.1-era db (which already holds `events`), backed-up-before-migrate per §16.
- [ ] `EventStore::append` applies projectors **in the same `BEGIN IMMEDIATE` txn** as
      the events INSERT: writes `object_refs` rows from the event, folds the registered
      projectors, advances each touched projector's `projection_offsets.last_seq` — all
      atomic. A reader never sees an event whose projections haven't applied.
- [ ] **Crash-safety:** `projection_offsets.last_seq` is never ahead of the rows it
      represents (offset advance + row writes commit together or roll back together).
- [ ] **Startup catch-up:** `EventStore::open` (post-migration) replays
      `events WHERE seq > last_seq` per projector, advancing offsets in-txn — so a db with
      pre-existing events (migration 3 over a 1.1 log) folds them, and a crash mid-write
      heals.
- [ ] **Full rebuild:** a `rebuild_projections` path truncates `proj_*` + `object_refs`,
      sets `last_seq=0`, replays all events. Raw `events` are untouched (§7.2).
- [ ] **Rebuild-equivalence:** incremental fold (append-by-append) produces a
      byte-identical projection state to a full replay of the same log.
- [ ] **Degraded handling:** a projector error or an unknown `event_version` sets that
      projector's `projection_offsets.state='degraded'`, **skips** the offending event,
      continues — never crashes, never corrupts raw `events`.
- [ ] Projection **status columns bind to the frozen `shared/` §5.1 enums** (e.g.
      `proj_session.status` ∈ `Session` (17), not a bare string) — fail-closed on an
      unknown wire value (§15 reject-unknown posture).
- [ ] The `SessionStarted` integration test (demo step 7) passes: one append → a
      `proj_session` row **and** `proj_graph_node`/`proj_graph_edge` rows **and**
      `object_refs` rows, in one txn.
- [ ] `AuditTrail` projector populates the rendered audit row **and** the FTS index per
      event (resolves the 1.1 "FTS5 scaffolding → populated in 1.2" note).
- [ ] All unit tests in `daemon/src/projections/` pass; `/preflight` clean
      (`cargo fmt --check && clippy -D warnings && check && test`).
- [ ] Cross-doc invariant rows updated atomic with the round (orchestrator writes — see below).

## Wiring / entry point (Step 7.5)
**Two production entry points — both real, not test-only:**
1. **`EventStore::append`** (`daemon/src/eventstore/mod.rs`) — every appended event flows
   through `projections::apply_all(&tx, &envelope)` in-txn. This is the steady-state path;
   when Phase 2/3 emit `SessionStarted` etc., projections fan out automatically. (The
   *emission* of those events is future; the *fold* path is wired now and exercised by
   directly-appended events — deterministic, test-first.)
2. **`EventStore::open`** — after migrations, runs the startup catch-up replay so
   projections are eventually-consistent with the log on every daemon start.

`/wired projection_engine` must show `append → apply_all` and `open → catch_up_replay`
as reachable from production, not just tests.

## Files expected to touch
**New:**
- `daemon/src/projections/mod.rs` — the engine: `Projector` trait
  (`name()`, `apply(&Transaction, &EventEnvelope) -> Result<()>`), the registry
  (`apply_all`, `catch_up_replay`, `rebuild`), offset advancement, degraded-skip.
- `daemon/src/projections/object_refs.rs` — normalized `event→object` edge writes
  (the ProjectGraph backbone; written in the append txn).
- `daemon/src/projections/session.rs` — `proj_session` projector (binds `Session` enum).
- `daemon/src/projections/graph.rs` — `proj_graph_node`/`proj_graph_edge` projector
  (folds from `object_refs`).
- `daemon/src/projections/audit.rs` — `proj_audit_trail` + FTS population.
- `daemon/src/projections/activity.rs` — `proj_project_activity` session-counter rollups.
- `daemon/src/projections/schema.rs` — migration-3 DDL constants (all 10 `proj_*` +
  `object_refs` + `projection_offsets`).
- Test modules colocated per `/tdd` convention.

**Modified:**
- `daemon/src/eventstore/mod.rs` — `append` threads `tx` into `projections::apply_all`
  before commit; `open` calls `catch_up_replay` post-migration; extend `AppendIntent`
  with the optional identity/edge fields projectors need (`project_id`, `session_id`,
  `agent_team_id`, `visibility`, `object_refs` — the 1.1 carry-forward) + populate them
  in the INSERT (columns already exist from migration 1).
- `daemon/src/eventstore/migrations.rs` — register migration 3; bump
  `SUPPORTED_USER_VERSION` 2 → **3**.
- `daemon/src/lib.rs` — `pub mod projections;`.
- `shared/src/` (event_envelope or a new `events/` module) — the **minimal concrete
  event-type payload(s)** the projectors fold (at least `SessionStarted`), defined
  contract-first per §5.0 (schemars → schema → Zod/Pydantic). Bump `CONTRACT_VERSION`
  if a contract type is added. **Flag at Step 2.5** before committing a shared/ contract change.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)

**Layer 1 — schema + contracts (`projections/schema.rs`, `eventstore` migration):**
1. **`test_migration_3_creates_projection_tables`** — Asserts: after `open` on a fresh db,
   `object_refs`, `projection_offsets`, and all 10 `proj_*` tables exist; `user_version==3`.
   Why: DATA_MODEL §2.2/§2.3/§2.4.
2. **`test_migration_3_over_existing_events_backs_up`** — Asserts: migrating a db that
   already holds 1.1-era `events` (from==2) writes `.bak-2` first (§16), tables added,
   events intact. Why: §16 forward-only + backup-before-migrate.
3. **`test_append_intent_persists_identity_fields`** — Asserts: an `AppendIntent` carrying
   `project_id`/`session_id`/`visibility`/`object_refs` round-trips to the `events` row +
   `object_refs` rows. Why: 1.1 carry-forward (envelope optional fields).

**Layer 2 — in-band apply (`projections/{mod,session,graph,audit,activity,object_refs}.rs`, writer):**
4. **`test_append_folds_session_projection_in_txn`** — Asserts: appending a `SessionStarted`
   writes one `proj_session` row (status ∈ `Session` enum) **and** advances
   `projection_offsets['session'].last_seq` to that event's `seq`, atomically. Why: §7 in-txn fan-out.
5. **`test_append_writes_object_refs`** — Asserts: an event with `object_refs[]` writes the
   normalized rows in the same txn. Why: DATA_MODEL §2.2.
6. **`test_offset_never_ahead_of_rows`** — Asserts: a forced projector failure rolls back
   the offset advance **and** the row writes together (no offset ahead of applied rows). Why:
   §2.4 / §7.2 crash-safety ("offset never ahead").
7. **`test_redaction_gate_still_fail_closed`** — Asserts: the §15 fail-closed test (1.1 test 4)
   still passes with projection apply added — projectors see only the redacted payload. Why:
   regression guard on the safety invariant (root CLAUDE.md §15 #3).
8. **`test_audit_projection_populates_fts`** — Asserts: appending an event yields a
   `proj_audit_trail` row and an FTS hit on its headline. Why: DATA_MODEL §2.11 + 1.1 FTS note.
9. **`test_session_started_fans_out`** *(integration — demo step 7)* — Asserts: one
   `SessionStarted` append produces `proj_session` **and** `proj_graph_node`/`edge` **and**
   `object_refs`, all in one txn. Why: PRD §25 step 7 / §14 fan-out coverage.

**Layer 3 — recovery (`projections/mod.rs`):**
10. **`test_startup_catch_up_replays_pending`** — Asserts: events with `seq > last_seq` are
    folded on `open` (e.g. migration-3 over a log of pre-existing events). Why: §7.2 startup replay.
11. **`test_rebuild_equivalence`** — Asserts: full rebuild (truncate + replay-all) == the
    incremental-fold state for the same log. Why: §7.2 ("fully rebuildable").
12. **`test_rebuild_preserves_raw_events`** — Asserts: rebuild truncates `proj_*` but leaves
    `events` byte-identical. Why: §7.2 ("projection corruption must not corrupt raw events").
13. **`test_unknown_event_version_degrades_skips`** — Asserts: an event with
    `event_version > MAX_SUPPORTED` (or a projector error) sets `state='degraded'`, skips that
    event, continues; raw events intact. Why: §7.2 degraded handling / §17 resilience.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `AppendIntent` extended (optional identity/edge fields) — internal
  daemon type, not a frozen contract, but note it. **New shared contract type(s):** the
  concrete event-type payload(s) (`SessionStarted` …) IF placed in `shared/` per §5.0 →
  `CONTRACT_VERSION` bump + 3-way verify (flag the version at Step 9).
- **Orchestrator doc rows to write hot (Step 9 routing):**
  - `daemon/CLAUDE.md` cross-doc table + `ARCHITECTURE.md` Appendix A — add rows: **the 10
    `proj_*` projections + `object_refs` + `projection_offsets`** (now implemented, binding to
    §5.1 enums); note **projection status columns mirror the frozen §5.1 status machines**.
  - DATA_MODEL §2.2/§2.3/§2.4 `[PROPOSED]`/`[LOCKED]` markers → reconcile to **implemented**.
  - If the FTS5 table is reshaped (see Step-2.5 Q3), record the §2.11 deviation.

> **Implementer never edits `daemon/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, or
> `daemon/LESSONS.md`** — flag categorized at Step 9; orchestrator writes hot the same round.

## Things to flag at Step 2.5
1. **In-band (in the event-commit txn) vs async worker projections.** My default vote:
   **in-band / in-txn** — `ARCHITECTURE.md §7` ("within the one event-commit transaction") +
   DATA_MODEL §2.4 ("same transaction as the rows it writes") + the demo-step-7 one-txn fan-out
   make this the **locked** architecture, not an open choice. The "workers" wording in the
   MVP_TASKS line is loose. If you see a reason in-band can't hold (e.g. a projector that must
   do I/O), that's a Step-9 cross-doc flag, not a unilateral switch to async.
2. **Projector coverage scope (LOAD-BEARING — separately escalated to the human).** Most of the
   10 projections are fed by events that **later phases emit** (ApprovalQueue←Phase 2;
   Worktree←Phase 5 git2; PullRequest←Phase 7; PlanProgress/AgentTeam←Phase 9). My default vote:
   **build the engine + all 10 table DDLs + `object_refs` + `projection_offsets` now, plus the
   projectors feedable in Phase 1 (`Session`, `ProjectGraph`/`object_refs`, `AuditTrail`+FTS,
   `ProjectActivity` counters); re-home the later-phase projector BODIES to their producing
   phases as tasks** (each lands with its event-type schema + tests). Rationale: writing a
   projector against an event payload that doesn't exist yet is speculative; sequencing each
   projector with its event contract is more correct (production-grade-foundation). **Do not
   start GREEN on this until the orchestrator confirms the scope** — I'm escalating it to the
   human in parallel; take the default unless I send a `TWEAK`.
3. **FTS5 shape.** 1.1 shipped a standalone `fts_events(event_id UNINDEXED, body)` scaffold;
   DATA_MODEL §2.11 specifies a contentless `events_fts ... content='proj_audit_trail'`. My
   default vote: **populate the existing `fts_events` scaffold from the AuditTrail projector**
   (lowest churn; no migration reshape) and record the §2.11 deviation as an arch-note — unless
   you find the contentless-table join is needed now. Either way, one consistent choice + a flag.
4. **`AppendIntent` extension shape.** Add the optional fields as plain `Option<…>` newtype
   fields (`project_id: Option<ProjectId>`, `object_refs: Vec<ObjectRef>`, …). My default vote:
   **yes, typed `Option`/`Vec` of the frozen newtypes** (not bare strings) — matches the
   envelope + fail-closed parse posture.
5. **Where concrete event-type payloads live.** My default vote: **`shared/` per §5.0** (Rust
   authority → schema → Zod/Pydantic), starting with only the payload(s) 1.2 folds
   (`SessionStarted`). The full EventTypeRegistry (§7.1 Appendix A) accretes per phase — do
   **not** define all event types now. If you'd rather keep the first payload in `daemon/model`
   to avoid a contract bump this slice, flag it — but §5.0 says consumer-facing event shapes are
   contract surface.
6. **`proj_worktree` two-axis precedence.** `shared/status.rs` notes the git+overlay precedence
   fn is "Phase-1 projection logic." My default vote: **defer with the Worktree projector to
   Phase 5** (it needs live git2 reads that don't exist until then) — create the table now, leave
   the projector re-homed. Confirm under Q2.

## Dependencies + sequencing
- **Depends on:** 1.1 event store (LANDED — `append`/`AppendIntent`/`EventEnvelope`/migrations/
  read paths). Frozen `shared/` §5.1 enums + IDs (0.5, LANDED).
- **Blocks:** 1.3 outbox (event+projection+outbox in one txn — extends this txn); 1.5 UDS
  read surface (`get_projection`/`subscribe` serve these tables); Phase 2 Gateway (writes
  through this path); Phase 6 UI (reads projections via fixtures then live).
- **Consumes carry-forward:** the 1.2-tagged items — `AppendIntent` grows optional envelope
  fields; `object_refs` normalized table (deferred from 1.1, lands here); `user_version()` `-1`
  sentinel → consider returning `Result<u32>` while in this code (low-pri; flag at Step 9 if you
  touch it).

## Estimated commit count
**3** (a multi-commit slice, mirroring 1.1's cadence — NOT bundleable into one; no single safety
pin, but the writer-txn extension + the schema + the recovery are three distinct logical units
with independent test surfaces):
- **L1 — schema + contracts:** migration 3 (all 10 `proj_*` + `object_refs` + `projection_offsets`),
  `SUPPORTED_USER_VERSION`→3, `Projector` trait + registry skeleton, `AppendIntent` extension.
  Tests 1–3.
- **L2 — in-band apply:** wire `apply_all` into the append txn; the feedable projectors
  (Session, ProjectGraph/object_refs, AuditTrail+FTS, ProjectActivity); the §15 regression guard.
  Tests 4–9 (incl. the demo-step-7 integration test).
- **L3 — recovery:** startup catch-up replay, full rebuild, rebuild-equivalence, degraded-skip.
  Tests 10–13.

> ⚠️ **Orchestrator drives layer→layer.** The implementer idles after each layer commit
> (1.1 wake-gap ×2; auto-memory `drive-multicommit-slices`). "Proceeding to L2/L3" is the cue to
> send the next-layer wake — never a status line to idle on.

## Lessons-logged candidates anticipated
- **Convention candidate** — "Projections apply in-band, in the event-commit txn; offsets advance
  in the same txn (never ahead of rows). Recovery = catch-up replay + full rebuild; raw events are
  never mutated by a projector."
- **Convention candidate** — "Projection status columns bind to the frozen §5.1 `shared/` enums,
  not bare strings — fail-closed on unknown wire values."
- **Architecture-doc note candidate** — projector coverage is phase-sequenced: a projector lands
  with the phase that emits its feeding events (record the re-homing in Appendix A / the phase tasks).
- **Future TODO — operational** — rebuild cost at large `events` counts (§18 budgets); a background
  rebuild vs blocking-on-open decision when the log is big (defer; flag if it bites).

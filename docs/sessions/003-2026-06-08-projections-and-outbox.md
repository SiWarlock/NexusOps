# Session 003 — Phase 1.2 projection engine + Phase 1.3 transactional outbox

| | |
|---|---|
| **Date** | 2026-06-08 |
| **Phase** | Phase 1 (daemon foundation) — tasks 1.2 + 1.3 |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | [002](002-2026-06-07-event-store.md) |
| **Successor** | [004](004-2026-06-08-lease-locks-and-fencing.md) — Phase 1.4 lease locks + fencing + pidlock + reaper |
| **Commits** | **1.2:** `c8c6c72` (L1) · `a1dc482` (L2) · `e66c659` (L3) — slice complete · **1.3:** `343cc09` (L1) · `707845a` (L2) — slice complete |

## Why this session existed
Two sequenced daemon-core slices extending the 1.1 event store, per briefs `004-P1-2-projections.md` + `005-P1-3-outbox.md`:
- **1.2 — projection engine:** the read-model side of the event-sourced core. Fold the append-only `events` log into derived `proj_*` read tables **in-band, inside the event-commit txn**, with offsets, startup catch-up replay, full rebuild, degraded-skip.
- **1.3 — transactional outbox:** reliable side-effects. Outbox rows written in the **same** event-commit txn (recorded-iff-intended); an async drainer delivers at-least-once with backoff + retryable/terminal classification + bounded dead-letter; the **§15 sync sink**.

## What was built

### 1.2 — projection engine (3 commits)
- **Files created:** `daemon/src/projections/{mod,schema,object_refs,session,graph,audit,activity}.rs` — the `Projector` trait + registry + `apply_all` (savepoint-isolated, offset advance, degraded-skip) + `catch_up_replay`/`rebuild`; the 4 Phase-1-feedable projector bodies; `REBUILD_TABLES`. `shared/src/events.rs` — `SessionStarted` (first §7.1 EventTypeRegistry payload). `daemon/tests/projections.rs` — 15 integration tests.
- **Files modified:** `eventstore/schema.rs` (MIGRATION_3_PROJECTIONS — `object_refs` + `projection_offsets` + 11 physical `proj_*`), `migrations.rs` (M3; SUPPORTED_USER_VERSION 2→3), `eventstore/mod.rs` (AppendIntent +4 identity fields; INSERT 18→22 cols; `apply_all`/`read_envelope_by_seq`/`read_events_after`/`catch_up_replay` wired into append+open; `rebuild_projections`; `MAX_SUPPORTED_EVENT_VERSION` pub(crate)), `lib.rs` (pub mod projections), `shared/src/{lib.rs (CONTRACT_VERSION 0.7.0→0.8.0),schema.rs}` + regenerated `contracts/schema/*.json`.

### 1.3 — transactional outbox (2 commits)
- **Files created:** `daemon/src/eventstore/outbox.rs` — `destinations_for`/`build_payload`/`write_for_event` (in-txn write); `Destination` trait + `DeliveryOutcome` + `JsonlMirror` + `drain_once` + `backoff_secs` + `reset_in_flight` + `DrainSummary`. `daemon/tests/outbox.rs` — 9 integration tests + the StepClock/FakeDestination seams.
- **Files modified:** `eventstore/schema.rs` (MIGRATION_4_OUTBOX), `migrations.rs` (M4; SUPPORTED_USER_VERSION 3→4), `eventstore/mod.rs` (write_for_event into append; reset_in_flight + drain_once; re-exports), `idgen.rs` (`new_outbox_id`, separate FixedIdGen counter), `clock.rs` (`now_plus_secs` default + Z-suffix-UTC contract), `Cargo.toml` (`time` "parsing").

## Decisions made
- **object_refs DERIVED, not caller-supplied (1.2 Q4 deviation, orchestrator-confirmed more correct):** refs are derived from the event's typed identity fields + type, so they're rebuildable-from-events (§2.2). A free-form `AppendIntent.object_refs` would not survive a rebuild (test 11). 
- **Projector coverage = Option A / sequence (1.2 Q2, human-ratified):** engine + all 10 table DDLs + object_refs + offsets now; 4 feedable projector BODIES (Session/ProjectGraph/AuditTrail+FTS/ProjectActivity); 6 re-homed to producing phases.
- **`SessionStarted` in `shared/` per §5.0 (1.2 Q5):** event payloads are consumer-facing contract surface → CONTRACT_VERSION 0.8.0 + 3-way verify.
- **Outbox is daemon-internal (1.3):** `out_` id is NOT one of the 22 frozen IDs; `destination`/`status` are not `shared/` contracts → no CONTRACT_VERSION bump.
- **Transactional atomicity demonstrated via the redaction-gate abort (1.3 [A], orchestrator-confirmed):** a 1.2 projector Decode *degrades* (doesn't abort the txn); the gate-abort (0 events ⇒ 0 outbox) is the deterministic all-or-nothing path.
- **Crash recovery = reset `in_flight`→`pending` on open (1.3 Q3):** simplest at-least-once; idempotency is the destination's job.
- **Z-suffix-UTC `Clock` contract:** the outbox due-comparison is lexical; verified `time`'s RFC3339 emits `Z` (now_rfc3339 + now_plus_secs format-consistent).

## Decisions explicitly NOT made (deferred)
- **§17 degradable replay** (the corrupt-row-on-`open` Finding) — strict reconstruction in `catch_up_replay` aborts daemon start on a corrupt event row; the skip-vs-refuse design + audit-integrity is a deliberate 1.6 task, not expanded mid-slice.
- **Drainer Tokio spawn + bounded drain pass (LIMIT/batch)** — the runtime spawn lives in 1.6 bootstrap; `drain_once` ships as the deterministic unit only.
- **6 projector bodies + 4 outbox adapters** — re-homed to their producing phases (each lands with its event/client + tests).

## TDD compliance
**Clean.** Every layer of both slices was RED→Step-2.5→GREEN (RED confirmed for the right reason before each GREEN — compile errors for missing API, runtime failures for unwired behavior). Reviewer fan-out (security + code-quality) at L2/L3 of 1.2 and L1/L2 of 1.3; all valid findings fixed in-slice. The cq-HIGH "stuck-replay double-count" (1.2 L3) was traced + disproven + pinned by a strengthened test 13.

## Reachability (confirmed, not re-traced)
- **1.2:** `EventStore::append → apply_all → each projector` (live); `EventStore::open → catch_up_replay` (live, every start); `rebuild_projections` (pub API; CLI wiring → 1.6).
- **1.3:** `EventStore::append → write_for_event` (live); `EventStore::open → reset_in_flight` (live, every start); `drain_once` (pub EventStore method; Tokio drainer spawn → 1.6).
- No tested-but-silently-unwired gaps. The two pub entries whose runtime caller is 1.6 (`rebuild_projections` CLI, `drain_once` spawn) are honest belongs-to-1.6 deferrals.

## Open follow-ups (orchestrator hot-routed; referenced, not duplicated)
The orchestrator routed all of these into `MVP_TASKS.md` + `LESSONS §4/§5` + the `daemon/CLAUDE.md` cross-doc table during the session. For future-you:
- **§17 corrupt-row-on-`open` Finding → 1.6** — degradable `catch_up_replay` (quarantine-skip + degrade, not abort `open`); bundle the legacy-`unredacted`-row quarantine; skip-vs-refuse design + audit-integrity event.
- **6 re-homed projectors:** ApprovalQueue→P2, UsageLedger→P3, Worktree→P5, PullRequest→P7, PlanProgress+AgentTeam→P9.
- **4 re-homed outbox adapters:** brain_mcp→P8 (8.1), github+linear→P7 (7.1), notifier→P10 (10.2).
- **1.6:** drainer Tokio spawn + bounded drain pass (LIMIT/batch); `rebuild_projections` CLI wiring.
- **Minor (deferred):** dedicated EventStoreError variant for internal routing errors (vs Reconstruct); `status_of`/`tables()` test-helper hardening; the SELECT-prepare `map_err(Write)` pre-existing pattern.

## Preflight
Green at commit time (no changes since `707845a`): **56 workspace tests** (31 from 1.1 → 46 after 1.2 → 56 after 1.3) + 3-way contract verify @ 0.8.0; `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + `cargo check --all-targets` clean. Security-reviewer PASS on every §15/§17/§7.2 invariant across both slices (1.2 L3: one HIGH routed as the §17 Finding → 1.6; 1.3: 0 critical/high).

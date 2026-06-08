# /tdd brief — degradable_catch_up_replay

## Feature
Make the startup replay path (`catch_up_replay` / `replay`, called inside `EventStore::open`) **degrade instead of crash** on a corrupt or unredacted event row — the §17 read-path contract, **USER-RULED Option C** (2026-06-08): on a corrupt event row → **quarantine (preserve) the raw row + emit a loud audit-integrity event + mark the offset degraded + continue (daemon starts)**; never *silently* drop; raw spine untouched. The deferred 1.2 L3 Finding. **Safety-critical, atomized** (the reason 1.6 was split 3-way).

## Use case + traceability
- **Task ID:** P1.6c (the §17 degradable-replay third of the split 1.6)
- **Architecture sections it implements:** `ARCHITECTURE.md §17` (the "Duplicate / unknown-type / corrupt-payload / clock-skew events" row — *corrupt `payload_json` → quarantine + audit-integrity event*; *unknown version → store raw + degraded marker, don't crash* — and the "Projection update fails / corruption" row — *mark offset degraded, skip bad event, never corrupt raw events*); `§15` (redaction — the replay-time unredacted defense-in-depth); `§7.1` (EventTypeRegistry — the new audit-integrity event type).
- **USER RULING:** Option C (recorded in `MVP_TASKS.md` Decisions-tabled, 2026-06-08). The semantics are ruled; this brief implements them.
- **Related context — most primitives ALREADY EXIST (read before writing tests):**
  - `daemon/src/eventstore/mod.rs` — **`DegradableEvent`** enum (`Ok`/`Degraded{seq,reason}`/`Quarantined{seq,reason}`) + `is_degraded()`/`is_quarantined()` (line 109); **`read_all_degradable()`** (line 299) — the resilient read that quarantines a corrupt/unreadable row + degrades an unknown `event_version` instead of crashing; `read_all()` (line 282, strict); `MAX_SUPPORTED_EVENT_VERSION` (line 42); `row_to_envelope`/`row_to_envelope_offset`. **The `-1` unreadable-`seq` marker** (line 313-321) is the cq-medium re-homed here → give it a named const / `Option<i64>` so it's an explicit "unknown seq," never an implicit sentinel.
  - `daemon/src/projections/mod.rs` — **`replay(conn, full_rebuild)`** (line 210) is the shared body of `catch_up_replay` (line 198) + `rebuild` (line 206); it reads each projector's pending tail via `read_events_after(&tx, from)` (**strict** — this is what aborts `open` on a corrupt row) and folds via `apply_one`. `projection_offsets.state` is the degraded marker (LESSON §4).
  - LESSON §4 (in-band fold + offsets + recovery), §8 (cold-start — `open` calls this replay). `EventStore::open` (mod.rs:130) calls `catch_up_replay` at line 143; bootstrap (1.6a) calls `open`.

## Acceptance criteria (what "done" means)
**L1 — degradable replay + unredacted-skip + sentinel cleanup (read-path resilience; no contract bump):**
- [ ] `replay` reads each projector's pending tail via a **degradable** read (a `read_events_after_degradable` analogue of `read_all_degradable`, scoped to `seq > from`) instead of the strict `read_events_after`. A `DegradableEvent::Ok` folds normally; a `Quarantined`/`Degraded` row is **skipped (not folded)**, the projector's `projection_offsets.state` is marked `degraded`, and replay **continues** — `open` no longer aborts. (Raw `events` untouched — §17/§7.2.)
- [ ] A **quarantine record** preserves + marks each quarantined row (a `quarantine` table or equivalent: `seq`, `reason`, `detected_at`) — so the corrupt row is preserved for forensics (NOT dropped) and a later replay skips a known-quarantined row **without re-processing/re-emitting** (idempotent across restarts). Design shape = Step-2.5 Q1.
- [ ] **Replay-time `redaction_status='unredacted'` quarantine-skip** (§15 defense-in-depth) — a row that somehow carries `redaction_status='unredacted'` (non-producible via the 1.1 write gate today, so this is replay-side defense) is **quarantine-skipped**, never folded into a projection (a projection must never surface an unredacted payload). `reason` must NOT echo row content (§15 — mirror `read_all_degradable`'s "reason must not echo sensitive content").
- [ ] The `-1` unreadable-`seq` marker → a named const (e.g. `UNKNOWN_SEQ`) or `Option<i64>` (the 1.6a-L1 cq-medium, re-homed). No implicit `-1` flowing into version/seq logic.

**L2 — audit-integrity event (Option C's "loud record"; CONTRACT_VERSION bump; security-reviewer):**
- [ ] A NEW EventTypeRegistry payload (e.g. `AuditIntegrityViolation` / `EventQuarantined`) in `shared/src/events.rs` (mirror `SessionStarted`: `deny_unknown_fields`, schemars), carrying the quarantine reason + the affected `seq` (NOT the corrupt payload). `CONTRACT_VERSION` minor-bump; schema regenerated; 3-way verify + envelope version pin updated.
- [ ] On a **newly**-quarantined row, the daemon appends the audit-integrity event (`actor_type=System`, via the write-actor `append` path — redaction gate + projector fold both run), **idempotent** via the quarantine record (no duplicate event on the next restart's replay). Emission timing (during-open vs after-open by the caller) = Step-2.5 Q2.
- [ ] The audit-integrity event lands in a projection the UI/§17 safety-surface reads (AuditTrail at minimum) — the gap is **loud + consumer-visible**, the core of why Option C beats a silent skip.

**Cross-cutting:**
- [ ] **The invariant pin (the load-bearing test):** a corrupt event row at `open` → **daemon starts** (does NOT abort) AND the gap is **recorded** (quarantine record + audit-integrity event), never silently dropped AND the raw `events` row is untouched. This single behavior is the §17/Option-C contract.
- [ ] Tests in `daemon/tests/` (+ `shared/tests/*` pins) pass; `/preflight` clean.
- [ ] **security-reviewer applies** (§15/§17 audit-integrity invariant) — this is the atomized safety slice.

## Wiring / entry point (Step 7.5)
`replay`/`catch_up_replay` is already called from `EventStore::open` (mod.rs:143), which bootstrap (1.6a) calls — so the degradable path is reachable in production the moment `open` uses it. The audit-integrity emission's caller is `open`/bootstrap/runtime (per Q2). **Name at Step 7.5** that the degradable replay is on the real `open` path (not just tests) and the audit-integrity event reaches a projection.

## Files expected to touch
**New:**
- `shared/src/events.rs` — the audit-integrity payload (+ registry); `shared/src/lib.rs` `CONTRACT_VERSION` bump; `shared/src/schema.rs` + `contracts/schema/*.json` regen; `shared/tests/*` pins.
- (possibly) a `quarantine` table DDL — a new migration (`MIGRATION_6_QUARANTINE`?) if Q1 chooses a table. `daemon/src/eventstore/schema.rs` + `migrations.rs` (`SUPPORTED_USER_VERSION` 5→6).

**Modified:**
- `daemon/src/projections/mod.rs` — `replay` uses the degradable read + skip-quarantined + mark-degraded + continue; the unredacted-skip.
- `daemon/src/eventstore/mod.rs` — a `read_events_after_degradable`; the `-1`→named-const/`Option` cleanup; the audit-integrity append helper (per Q2); the quarantine-record read/write.
- `daemon/tests/*` — the degradable-replay + audit-integrity integration tests.

If a `quarantine` table / migration is needed (Q1), that's a new `user_version` → confirm at Step 2.5.

## RED test outline (Step 2)
1. **`test_corrupt_row_open_recovers_not_aborts`** *(the load-bearing pin)* — Asserts: a DB with one corrupt `payload_json` row → `open` returns `Ok` (daemon starts); the row is quarantine-recorded; the raw `events` row is unchanged. Why: §17/Option-C — degrades-not-crashes, never corrupts the spine.
2. **`test_quarantine_is_not_silent`** — Asserts: a quarantined row produces a quarantine record AND (L2) an audit-integrity event in a projection — the gap is recorded, never silently dropped. Why: Option C vs the rejected Option A.
3. **`test_quarantine_idempotent_across_restart`** — Asserts: a second `open` over the same corrupt DB does NOT emit a duplicate audit-integrity event (the quarantine record dedups). Why: restart idempotency / no event-spam.
4. **`test_unknown_event_version_degrades_continues`** — Asserts: an `event_version > MAX_SUPPORTED_EVENT_VERSION` row → degraded marker, replay continues, daemon starts. Why: §17 "unknown version → store raw + degraded, don't crash."
5. **`test_unredacted_row_quarantine_skipped_on_replay`** — Asserts: a row with `redaction_status='unredacted'` is quarantine-skipped, never folded into a projection; the reason echoes no row content. Why: §15 replay-side defense-in-depth.
6. **`test_healthy_log_replays_unchanged`** — Asserts: a clean log replays byte-identically to the strict path (no regression; rebuild-equivalence preserved). Why: the degradable path must not change healthy behavior.
7. **`test_unknown_seq_marker_is_explicit`** — Asserts: an unreadable-`seq` quarantine uses the named `UNKNOWN_SEQ` const / `Option`, never an implicit `-1` in version/seq logic. Why: 1.6a-L1 cq-medium.
8. **`shared`: audit-integrity payload wire-pin + `CONTRACT_VERSION` bump** — Asserts: round-trips snake_case; `deny_unknown_fields`; version bumped. Why: §5.0 / LESSON §2.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW audit-integrity event type → EventTypeRegistry accretion → `CONTRACT_VERSION` minor bump. (If Q1 = a `quarantine` table, that's a daemon-internal DDL + `user_version` bump — not a `shared/` contract, outbox/leases-analogous.)
- **Orchestrator doc rows to write hot:** the new event type → `daemon/CLAUDE.md` EventTypeRegistry row + `ARCHITECTURE.md` Appendix A / §7.1; resolve the §17 Decisions-tabled entry's implementation pointer (Option C realized); the §17 "corrupt-payload → quarantine + audit-integrity event" row marked `[IMPLEMENTED 1.6c]`.

## Things to flag at Step 2.5
1. **Quarantine record shape.** A dedicated `quarantine` table (`seq, reason, detected_at`) vs a `quarantined`/`state` column on `events` (but the spine is append-only + must stay untouched — a separate table is cleaner) vs reuse `projection_offsets.state='degraded'` only. Default vote: **a dedicated `quarantine` table** (preserve the raw row untouched; the table is the forensic + dedup record; daemon-internal, no `shared/` surface — leases/outbox precedent). Confirm the migration.
2. **Audit-integrity emission timing.** (a) during replay (write-during-`open`) vs **(b) collect quarantine findings during replay → emit after `open` completes, from the caller (bootstrap/runtime), idempotent via the quarantine record.** Default vote: **(b)** — replay stays a pure read (no write-during-read), restart idempotency is handled by the quarantine record (emit only for newly-quarantined seqs), and the event flows through the normal write-actor `append`. Confirm.
3. **`rebuild` (full) — same degradable treatment?** `replay` is shared by `catch_up_replay` + `rebuild`. Default vote: **yes, both degrade** (a full rebuild over a corrupt log must also not crash) — same path, one change. Flag if rebuild should stay strict (it shouldn't — same §17 logic).
4. **One audit-integrity event per quarantined row, or one summary per replay?** Default vote: **one per newly-quarantined row** (precise forensics + per-seq dedup) — but cap/batch if a mass-corruption replay could emit thousands (flag if a batch summary is safer at scale).

## Dependencies + sequencing
- **Depends on:** 1.1/1.2 (`DegradableEvent`, `read_all_degradable`, `replay`, `projection_offsets.state` — all LANDED); 1.6a-L2 (`open` is the production caller via bootstrap — LANDED). Independent of 1.6a-L3 + 1.6b.
- **Blocks:** Phase-1 acceptance (the §17 read-path contract). Pairs with 1.7 (the other Phase-1-acceptance blocker).
- **Sequence:** after 1.6b + 1.6a-L3 (per the post-ruling queue) — though it has no hard dependency on either; it can land any time after 1.6a-L2.

## Estimated commit count
**2** (layer→layer; safety-critical slice — security-reviewer on both):
- **L1** degradable replay + quarantine record + unredacted-skip + sentinel cleanup (read-path resilience; daemon-internal). **Not silent** — L1 records the quarantine + degrades the offset, so it is Option-C-compliant in substance even before L2.
- **L2** the audit-integrity event type + emission (the event-stream "loud record"; CONTRACT_VERSION bump).

**L1+L2 together deliver Option C** — land both before claiming §17 compliance (L1 alone preserves+records but doesn't yet emit the consumer-facing audit-integrity event). This is THE atomized safety slice (per "never bundle safety-critical"); nothing else rides here.

## Lessons-logged candidates anticipated
- **Convention candidate** — "Degradable replay (Option C): the read path quarantines (preserves + records) a corrupt/unredacted row, marks the offset degraded, emits a loud audit-integrity event (idempotent via the quarantine record), and continues — never silently drops, never aborts `open`, never corrupts the raw spine."
- **Architecture-doc note candidate** — §17 corrupt-payload + projection-corruption rows realized; the quarantine record + audit-integrity event are the "loud" mechanism.
- **Future TODO — operational** — mass-corruption batch/cap on audit-integrity emission; a UI surface for quarantined rows (the §17 safety-state card already scaffolded ui-side).

## How to invoke
1. Read end-to-end. The semantics are USER-RULED (Option C) — the Step-2.5 questions are HOW (quarantine record shape, emission timing), not WHETHER.
2. `/tdd degradable_catch_up_replay` → Step 0 restate → Step 2.5 write-up (Q1/Q2 are the design calls).
3. Drive L1→L2 (RED→GREEN→commit each; no idle between). security-reviewer on both.
4. Step 9 — the CONTRACT_VERSION bump + EventTypeRegistry row are the orchestrator's hot-write; flag the quarantine-table migration if added.

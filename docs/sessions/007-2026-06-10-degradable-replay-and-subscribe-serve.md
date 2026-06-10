# Session 007 — Phase 1.6c (§17 degradable replay) + 1.6d (subscribe-SERVE push)

| | |
|---|---|
| **Date** | 2026-06-10 |
| **Phase** | Phase 1 (daemon foundation) — tasks 1.6c + 1.6d → **Phase 1.6 CLOSED** |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | [006](006-2026-06-08-cold-start-bootstrap-and-runtime.md) |
| **Successor** | _(TBD — next: 1.7 redactor entropy fallback, the last Phase-1-acceptance blocker)_ |
| **Commits** | **1.6c:** `aeedc3f` (L1 degradable replay + quarantine table + unredacted-skip + UNKNOWN_SEQ) · `4df4078` (L2 AuditIntegrityViolation event + emission; CONTRACT 0.12.0→0.13.0). **1.6d:** `93f70a5` (subscribe-SERVE push; no contract bump). _(Also this session, pre-merge: 1.6a-L3 registration `980e1a0` — doc'd via the round Log, not here.)_ |
| **Base** | `46ed874` (cross-track ui↔main merge; daemon/shared empty-delta across it) |
| **Contract** | `CONTRACT_VERSION` **0.12.0→0.13.0** (1.6c L2 adds the `AuditIntegrityViolation` event payload); 1.6d is daemon-internal (no bump) |

## Why this session existed

Phase 1.6 had two safety/runtime gaps left after the cold-start + runtime work (006):
- **1.6c (§17, atomized safety slice):** startup replay (`catch_up_replay`/`rebuild`, inside `EventStore::open`) used the **strict** reader, so a single corrupt event row aborted `open()` → the daemon wouldn't start (no in-product recovery). §17 requires the read path to **quarantine + degrade + continue**, with a loud audit-integrity record (USER-RULED **Option C**).
- **1.6d:** the live broadcast SOURCE landed in 1.6b (publish-after-commit), but no production caller streamed deltas to a socket connection — the ui's `subscribe()` path was not live (the carved-out 1.6b L4 piece / brief 028 test 11).

(1.6a-L3 Device/LocalRunner registration also landed this session before the cross-track merge; per the lead's cycle ruling its narrative was folded into the round Log + `980e1a0`'s commit message + LESSON §10, so it has no dedicated session doc.)

## What was built

### 1.6c — degradable catch-up replay + audit-integrity event (Option C)

**Files created:**
- `daemon/tests/replay.rs` — the degradable-replay test suite (corrupt-row-recovers [load-bearing]; unknown-version-degrades; unredacted-quarantine-skip; healthy-replays-unchanged; re-detection idempotency; content-free reason; + L2 quarantine-is-not-silent + idempotent-across-restart via rebuild).

**Files modified:**
- `daemon/src/eventstore/schema.rs` — `MIGRATION_6_QUARANTINE` (the daemon-internal `quarantine` table: `seq PK, reason, detected_at, audit_emitted`).
- `daemon/src/eventstore/migrations.rs` — `SUPPORTED_USER_VERSION` 5→6 + M6 registered.
- `daemon/src/eventstore/mod.rs` — `UNKNOWN_SEQ` const (replaces the implicit `-1`); a shared `classify_degradable_row` (extracted from `read_all_degradable`, + a new `unredacted → Quarantined` arm, §15); `read_events_after_degradable` (replay-scoped); removed the now-dead strict `read_events_after`; `emit_quarantine_audit_events` (L2 — appends the AIV event per un-emitted quarantine row, idempotent); inline `UNKNOWN_SEQ` unit pin.
- `daemon/src/projections/mod.rs` — `replay()` reads degradably + matches `Ok`/`Degraded`/`Quarantined`; `record_quarantine` (`ON CONFLICT(seq) DO NOTHING`).
- `daemon/src/projections/audit.rs` — headline for the AIV event type.
- `daemon/src/bootstrap.rs` — `cold_start` step 6 calls `emit_quarantine_audit_events` (the caller, after open).
- `shared/src/events.rs` — `AuditIntegrityViolation { seq, reason }` payload + `EVENT_TYPE` const.
- `shared/src/schema.rs` / `shared/src/lib.rs` / `shared/contracts/schema/*.json` — schema bundle + `CONTRACT_VERSION` 0.13.0 + regen.
- `shared/tests/contract.rs` / `shared/tests/envelope.rs` — AIV wire-pin + version-pin 0.13.0.
- `daemon/tests/locks.rs` — relaxed M5's `user_version == 5` to `>= 5` (the M6 bump; the test's own prior note anticipated this).

### 1.6d — socket subscribe-SERVE push

**Files modified:**
- `daemon/src/ipc/subscribe.rs` — `PushAction` + the pure `next_push_action` classifier (`Ok(matching)→Push`, `Ok(other)→Skip`, `Lagged→Stop`, `Closed→Stop`) + `run_push_loop` (drives the push; `shutdown(Both)` on any exit) + inline unit tests.
- `daemon/src/ipc/server.rs` — `serve_connection` specialized generic `<S>` → `UnixStream`; + a `deltas: broadcast::Sender<ProjectionDelta>` param; **dedicated-subscribe** handling: mint receiver → write ack (main thread) → spawn the single push thread **iff the ack succeeded** → read-only-until-EOF → `shutdown(Both)` → return.
- `daemon/src/ipc/mod.rs` — re-export `run_push_loop` (pub(crate)).
- `daemon/src/runtime/listener.rs` — `spawn_accept_loop` + a `deltas` param threaded to `serve_connection`.
- `daemon/src/runtime/writer.rs` — `WriteHandle::delta_sender()` (clone of the post-commit broadcast sender).
- `daemon/src/main.rs` — passes `handle.delta_sender()` into the accept-loop.
- `daemon/tests/ipc.rs` — test 11 (push-delivered) + test 11b (dedicated-connection) + `no_deltas()` helper + ~9 call-site edits.
- `daemon/tests/runtime.rs` — `no_deltas()` helper + 4 accept-loop call-site edits.

## Decisions made

- **§17 Degraded ≠ Quarantined (blessed at Step-2.5).** Unknown `event_version` → **Degraded** (mark the offset degraded, continue; a newer binary folds it — not an integrity violation, no quarantine record, no AIV event). Corrupt (reconstruction-fail) **or** unredacted row → **Quarantined** (record + a loud AIV event). Splits §17's "unknown version → degrade quietly" from "corrupt payload → quarantine + audit-integrity event."
- **Quarantine record = a dedicated daemon-internal `quarantine` table** (Q1), `ON CONFLICT(seq) DO NOTHING` so re-detection (catch-up re-read / full rebuild) never resets `audit_emitted` → exactly-once AIV per seq. NOT in `REBUILD_TABLES` (forensic + dedup state survives a rebuild). Daemon-internal (outbox/leases precedent) — no `shared/` surface.
- **AIV emission from the caller after `open` (Q2)** — replay stays append-free; `cold_start` calls `emit_quarantine_audit_events`. Idempotent across restart AND rebuild via `audit_emitted` + an `audit-integrity-{seq}` idempotency_key (crash-safe). Content-free `reason` (the serde error is discarded — §15).
- **1.6d subscribe is structurally DEDICATED (single-writer).** Driven by a Step-8 review HIGH (see TDD/Reachability): post-ack the main loop is read-only-until-EOF, the push thread is the sole writer, exactly one per connection (bounded by the accept-loop cap). `Lagged → close` (LESSON §9 resync trigger; `ProjectionDelta` has no seq, so a gapped stream would silently diverge the client). Receiver minted **before** the ack (no missed delta); spawn gated on the ack succeeding (no parse-drift).

## Decisions explicitly NOT made (deferred)

- **`subscription_id` / >1 subscription per connection + the multiplexing it implies** — deferred (no wire field, no contract bump). MVP is one dedicated subscribe connection.
- **JoinSet-shutdown + the no-delta-disconnect push-thread linger** — a bounded leak (re-review-confirmed not a safety hole; exits on the next delta or broadcast-close at shutdown) → folded into a future **runtime-shutdown-hardening** slice.
- **Mass-corruption batch/cap on AIV emission** (Q4) — one AIV per quarantined seq for now; a batch summary if a mass-corruption replay could emit thousands → Carry-forward.
- **Event-type string-literal consolidation** — the new AIV type uses `AuditIntegrityViolation::EVENT_TYPE` (no new bare literal); the existing `SessionStarted`/`DeviceRegistered`/`LocalRunnerRegistered` literals consolidate in the next non-safety event-touching slice (Phase 2).

## TDD compliance

- **1.6c (L1 + L2): clean test-first.** RED confirmed for the right reason at each layer (L1: no quarantine table / strict-open aborts; L2: missing `AuditIntegrityViolation` + `emit_quarantine_audit_events`) → GREEN.
- **1.6d primary: clean test-first.** test 11 + the `next_push_action`/`run_push_loop` unit tests were RED (missing serve signature / symbols) → GREEN.
- **1.6d fix-sequencing note (transparent, not a feature violation):** the Step-8 security + code-quality review found a HIGH — concurrent socket writes (the main RPC-response loop racing the push thread) could interleave + corrupt the wire stream. The remediation (structurally-dedicated single-writer subscribe) was applied, then pinned by `test_subscribe_connection_is_dedicated` (test 11b) **after** the fix code — i.e. a review→fix→pin loop, not the primary feature's RED→GREEN. The fix was security-**re-reviewed clean** (prior HIGH resolved, 0 new critical/high).

## Reachability

- **1.6c degradable replay:** `main.rs::run()` → `cold_start` → `EventStore::open` → `catch_up_replay` → `replay` (degradable) → `read_events_after_degradable` + `record_quarantine`; also `rebuild_projections` → `replay`. **AIV emission:** `main.rs` → `cold_start` (step 6) → `emit_quarantine_audit_events`; the AIV event folds into `proj_audit_trail` (consumer-visible).
- **1.6d subscribe push:** `main.rs` (`delta_sender`) → `spawn_accept_loop` → `serve_connection` → (on subscribe) `run_push_loop` → `push_subscription`. Closes the ui live-subscribe path (`MockGatewayPort` → real for subscriptions).
- No tested-but-unwired gaps.

## Open follow-ups

- **Cross-doc (orchestrator hot-write, in flight this round):** `CONTRACT_VERSION` 0.13.0 + the `AuditIntegrityViolation` EventTypeRegistry row → §7.1/Appendix-A + `daemon/CLAUDE.md`; the `quarantine` table daemon-internal cross-doc row; §17 corrupt-payload + projection-corruption rows → `[IMPLEMENTED 1.6c]`; §6.1/§6.4 subscribe-SERVE → LIVE; §12/§16 live-subscribe path closed; the §17 Decisions-tabled Option-C entry → realized.
- **ui-guidance note (orchestrator → §6.4 + ui↔daemon Carry-forward):** a subscribe connection is DEDICATED — the ui opens a dedicated connection for subscribe (no multiplexing RPC + a subscription on one connection).
- **LESSON candidates (orchestrator-authored):** §11 (degradable replay Option C — quarantine + audit-integrity, Degraded≠Quarantined, never corrupt the spine); §12 (subscribe-SERVE — close-on-lag resync + dedicated-connection single-writer + the interleave HIGH).
- **Deferred slices/items** (see "NOT made"): subscription_id/multiplexing; runtime-shutdown-hardening (JoinSet + push-thread linger); mass-corruption AIV cap; event-type-literal consolidation (Phase 2).
- **Defer-grade review lows (no action):** the unreadable-`seq` arm is defense-in-depth (not SQL-constructible); the degradable SELECT's duplicate leading cols (pre-existing); the benign double-`shutdown(Both)` on a subscribe close (ENOTCONN swallowed).

## How to use what was built

A daemon started on a DB with a corrupt/unredacted event row now **starts** (replay quarantines the row, preserves the raw spine, marks the offset degraded, emits a loud `AuditIntegrityViolation` into the audit trail) instead of aborting `open()`. A socket client that handshakes + `subscribe`s now **receives live `ServerFrame::subscription_push` frames** for matching projection deltas over a dedicated connection; if it lags, the connection closes and the client re-establishes from a fresh `get_projection` snapshot.

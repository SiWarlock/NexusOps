# /tdd brief — transactional_outbox

## Feature
The transactional outbox — reliable, exactly-recorded / at-least-once-delivered
side-effects. Outbox rows are written **in the same event-commit transaction** as
the event + its projections (a fact is never recorded without its delivery intents,
never delivered without being recorded), and an async drainer delivers them with
backoff + retryable/terminal classification + a dead-letter terminal state. The
**outbox is the only path** an event reaches an external destination.

## Use case + traceability
- **Task ID:** P1.3
- **Architecture sections it implements:** `ARCHITECTURE.md §7` (event/projection/**outbox**
  one-txn flow — diagram D2), `§12` (outbox drainers as Tokio tasks; classify retryable vs
  terminal), `§17` (integration-failure contract: transient 429/5xx → backoff; terminal
  401/403 → dead after a bounded budget; offline → queue + drain on reconnect), `§15`
  (**the sync sink** — "same redactor gates persist+embed+sync"; per-destination payload
  is redacted).
- **Data-model anchor:** `DATA_MODEL.md §2.5` (`outbox` DDL: `out_` ULID; `destination`;
  `status` pending|in_flight|delivered|failed|dead; `retry_count`; `next_attempt_at`;
  `last_error`).
- **Related context:** **extends the 1.2 append txn** (`EventStore::append → apply_all`,
  `e66c659`). The outbox write joins that `BEGIN IMMEDIATE` txn after the projections.
  §12 Brain-outbox-payload + §17 integration-failure contract are the destination semantics.

## Acceptance criteria (what "done" means)
- [ ] Migration **4** (user_version 3→4; `SUPPORTED_USER_VERSION`→4) creates `outbox`
      per DATA_MODEL §2.5 (incl. `ix_outbox_due`).
- [ ] `EventStore::append` writes outbox rows **in the same txn** as the event + projections
      (transactional-outbox): for each destination subscribed to that `event_type`, one
      `pending` row. Atomic — event, projections, offsets, **and** outbox rows commit or roll
      back together.
- [ ] **§15 sync-sink gate (load-bearing):** every outbox payload is derived from the
      **already-redacted** event payload (never re-fetched raw) and per-destination filtering
      only *removes* fields (e.g. Brain = envelope minus restricted/secret, §12) — it can
      never re-introduce unredacted content. A secret in the source event appears in **no**
      outbox row. (security-reviewer required — `invariant` policy.)
- [ ] **`drain_once(clock, destination)`** (the deterministic unit the Tokio loop calls):
      selects due rows (`status IN (pending,failed) AND next_attempt_at <= now`, by
      `next_attempt_at`), attempts delivery, and transitions: success → `delivered`;
      **retryable** (429 Retry-After / 5xx / transport) → `failed` + `retry_count++` +
      `next_attempt_at = backoff(retry_count)`; **terminal** (401/403/4xx) → `dead`; and
      `failed` → `dead` once the bounded retry budget is exhausted.
- [ ] **At-least-once across crash:** a daemon crash after `in_flight` (before delivery
      confirmation) leaves the row re-deliverable on restart → redelivered, and an idempotent
      destination does **not** double-apply.
- [ ] **Outbox-is-the-only-path:** external delivery never bypasses the outbox (the
      destination invariant — analogous to INV-SEC-1 for mutations).
- [ ] All unit tests in `daemon/src/eventstore/outbox.rs` (or `daemon/src/projections/`) pass;
      `/preflight` clean; cross-doc rows updated atomic with the round (orchestrator writes).

## Wiring / entry point (Step 7.5)
- **`EventStore::append`** — outbox rows written in-txn (production write path; reachable from
  every append, same as 1.2's projections).
- **The drainer Tokio task** — calls `drain_once` on an interval; **must be spawned from the
  daemon runtime** (where the Tokio runtime lives). NOTE: the daemon binary's runtime
  bootstrap is Phase 1.6 — so for 1.3 the drainer task's *spawn* may be a `pub` entry the 1.6
  bootstrap wires. Confirm at Step 7.5 whether `drain_once` is reachable from a production
  spawn point now or is a 1.6-wired entry (if the latter, say so explicitly — like
  `rebuild_projections`'s CLI wiring → 1.6 — don't leave it falsely "wired").

## Files expected to touch
**New:**
- `daemon/src/eventstore/outbox.rs` — the `outbox` write (in-txn), the `Destination` trait,
  `drain_once`, backoff, retryable/terminal classification, the status machine, the
  event_type→destinations subscription map.
- `daemon/src/eventstore/outbox.rs` tests + a `FakeDestination` (scripted success/retryable/
  terminal responses) for deterministic drainer tests.

**Modified:**
- `daemon/src/eventstore/mod.rs` — `append` writes outbox rows in the txn after `apply_all`;
  migration 4 registered; `SUPPORTED_USER_VERSION`→4; a `pub` drainer entry if 1.6 wires it.
- `daemon/src/eventstore/schema.rs` — `MIGRATION_4_OUTBOX` DDL constant (co-located with
  M1–M3, the established placement).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)

**Layer 1 — outbox table + in-txn write (`schema.rs`, `outbox.rs`, writer):**
1. **`test_migration_4_creates_outbox`** — Asserts: `open` migrates 3→4 + creates `outbox` +
   `ix_outbox_due`; events/projections intact; backup-before-migrate (§16). Why: §2.5/§16.
2. **`test_append_writes_outbox_rows_in_txn`** — Asserts: appending an event whose type has N
   subscribed destinations writes N `pending` outbox rows in the same txn (forced projector
   failure also rolls back the outbox rows — all-or-nothing). Why: §7 transactional-outbox.
3. **`test_outbox_payload_has_no_secret`** *(§15 — load-bearing)* — Asserts: a source event
   carrying a secret (already redacted by the 1.1 gate) produces outbox rows whose payloads
   contain no secret; per-destination filtering only removes fields. Why: §15 sync-sink gate.
4. **`test_outbox_is_only_external_path`** — Asserts: there is no delivery path that doesn't
   read from `outbox` (structural — e.g. the `Destination` trait is only invoked by the
   drainer over outbox rows). Why: destination invariant (INV-SEC-1 analogue).

**Layer 2 — drainer + destinations (`outbox.rs`):**
5. **`test_drain_delivers_once_marks_delivered`** — Asserts: a `pending` row + a FakeDestination
   that succeeds → one delivery, row `delivered`. Why: §12 happy path.
6. **`test_retryable_backs_off`** — Asserts: a 429/5xx → `failed`, `retry_count++`,
   `next_attempt_at = backoff` (fake clock); re-drains after the delay. Why: §17 transient.
7. **`test_terminal_goes_dead`** — Asserts: a 401/403 → `dead` immediately (no retry). Why:
   §17 terminal.
8. **`test_retry_budget_exhausts_to_dead`** — Asserts: repeated retryable failures → `dead`
   once the bounded budget is hit. Why: §17 "dead only after bounded retry budget."
9. **`test_crash_redelivers_no_double_apply`** *(integration)* — Asserts: a row left `in_flight`
   by a simulated crash is re-attempted on restart → redelivered; an idempotent destination
   dedups (no double-apply). Why: at-least-once + §17 crash row.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** the `outbox` destination enum + status enum — **daemon-internal**
  (the UI/Brain don't read the outbox; it's the daemon's delivery mechanism), so likely **not**
  a frozen `shared/` contract surface (no CONTRACT_VERSION bump). **Confirm at Step 2.5.**
- **Orchestrator doc rows to write hot (Step 9):** `outbox` implemented (DATA_MODEL §2.5
  `[LOCKED]` → implemented); the transactional-outbox-in-one-txn convention (extends the 1.2
  LESSONS entry); the §15 sync-sink-gate note; **the re-homed destination adapters** (see Q1).

## Things to flag at Step 2.5
1. **Destination coverage scope (follows the human-approved 1.2 Option-A precedent).** The real
   drainer adapters need later-phase integrations: **brain_mcp→Phase 8** (8.1), **github +
   linear→Phase 7** (7.1), **notifier→Phase 10** (10.2). My default vote: **build the outbox
   engine + the drainer framework + `jsonl_mirror` as the one Phase-1-feedable real destination
   + a `FakeDestination` for the failure-mode tests; re-home the 4 external adapters to their
   phases** (each lands with its real client + auth + tests). This is the same sequencing the
   human ratified for 1.2's projectors — I'll hot-route the 4 re-homed adapters as phase tasks
   at Step 9 (nothing dropped). Flag if you'd rather include any adapter now.
2. **`drain_once` determinism seam.** My default vote: inject `Clock` (backoff/next_attempt_at)
   + the `Destination` trait (FakeDestination scripts success/retryable/terminal) so the
   drainer logic is test-first; the Tokio loop is a thin interval caller over `drain_once`.
3. **Crash-recovery reset.** On restart, how do `in_flight` rows become re-deliverable? My
   default vote: **treat `in_flight` as re-attemptable** (reset to `pending` on startup, or
   the drainer re-selects `in_flight` past a visibility timeout) — at-least-once; idempotency
   is the destination's responsibility. Confirm which mechanism.
4. **Drainer spawn / reachability (Step 7.5).** Is `drain_once` reachable from a production
   spawn now, or is the spawn a 1.6-bootstrap-wired entry? My default vote: **expose a `pub`
   drainer entry; the Tokio spawn is 1.6-wired** (like `rebuild_projections`'s CLI) — say so
   explicitly rather than claiming it's live.
5. **Subscription map home.** event_type→destinations routing — daemon-internal config. My
   default vote: **a daemon-internal map** (not a `shared/` contract), starting minimal (the
   event types 1.2 defined → jsonl_mirror), accreting per phase.

## Dependencies + sequencing
- **Depends on:** 1.2 (the append txn + `apply_all` it extends — LANDED `e66c659`); 1.1 event
  store; frozen `shared/` enums.
- **Blocks:** Phase 7 (GitHub/Linear syncers drain the outbox), Phase 8 (Brain notification
  adapter drains), Phase 10 (notifier drains), the §25 demo (Create-PR flow: gateway → octocrab
  → **outbox** → events).
- **Interacts with the pending §17 Finding (1.6):** the degradable-replay fix and the outbox
  both touch the append/recovery path but on independent surfaces — no conflict.

## Estimated commit count
**2** (multi-commit slice; the L1 in-txn write touches the safety-critical append txn + the §15
sync sink, so it's a focused unit; the L2 drainer is a distinct concern with its own test
surface). NOT one commit, NOT bundled with anything else.
- **L1 — outbox table + in-txn write + §15 sync-sink gate** (tests 1–4). ⚠️ §15-touching →
  security-reviewer (`invariant` policy).
- **L2 — drainer (classify/backoff/dead-letter) + crash-redelivery + jsonl_mirror + FakeDestination**
  (tests 5–9).

> ⚠️ **Orchestrator drives layer→layer** (banked lesson, 3 mechanisms): the next-layer directive
> is folded into each SHIP message; I re-wake immediately on any post-commit "proceeding"; roll
> straight into the next layer's RED after committing — no standalone status, no idle gap.

## Lessons-logged candidates anticipated
- **Convention candidate** — "External side-effects go through the transactional outbox: written
  in the event-commit txn (recorded-iff-intended), delivered at-least-once by a drainer with
  backoff + retryable/terminal classification + a dead-letter terminal; idempotent consumers."
- **Convention candidate / safety note** — "The outbox is the §15 *sync* sink: every outbox
  payload derives from the already-redacted event and per-destination filtering only removes
  fields — never re-fetch raw."
- **Architecture-doc note** — destination adapters are phase-sequenced (drainer framework here;
  brain_mcp/github/linear/notifier re-homed to P8/P7/P10).

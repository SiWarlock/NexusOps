# /tdd brief — lease_locks_and_fencing

## Feature
Cross-restart resource locks: a persistent SQLite **lease** per `(resource_id, lease_kind)`
with a **monotonic fencing token** that only ever increments and survives daemon restart,
owner-guarded `renew`/`release`, an expired-lease **reaper** (`reap_once` deterministic unit),
and **single-instance** enforcement via a std advisory file lock (`pidlock`). This is the
load-bearing primitive the Phase-2 Action Gateway uses to reject a stale-token mutation
(safety rule #6 / §17 → `fencing_conflict`) and the daemon bootstrap (1.6) uses to refuse a
second instance.

## Use case + traceability
- **Task ID:** P1.4
- **Architecture sections it implements:** `ARCHITECTURE.md ADR-008` `[LOCKED]` (cross-restart
  locks = SQLite lease table: owner + **monotonic fencing token** + heartbeat; **`pidlock`
  single-instance**); `§7.2` Locks row (`leases` table **authoritative**; expired → **reclaim
  w/ new fencing token**); `§17` recovery row line 387 (**stale-token write → `ActionFailed(fencing_conflict)`
  + conflict event; hard-conflict card, never auto-resolved** — the *enforcement* is the Phase-2
  gateway; 1.4 provides the primitive that makes a stale token *detectable*); `§16`
  (backup-before-migrate); `§12` (the reaper as a Tokio task — *spawn* is 1.6).
- **Safety invariant:** root `CLAUDE.md` **Key safety rule #6 — "Fencing tokens mandatory. A
  stale-token mutation is rejected → hard-conflict card, never auto-resolved (§5.1/§17)."** 1.4
  builds the mechanism; the gateway enforces it in Phase 2. ⚠️ security-reviewer required (`invariant`).
- **Data-model anchor:** `ARCHITECTURE.md` Appendix A **`Lease`** row (canonical field set):
  `resource_id, owner_id, fencing_token(monotonic), acquired_at, heartbeat_at, expires_at, lease_kind`.
  *(The `MVP_TASKS.md` 1.4 line abbreviates this — omits `lease_kind` + `acquired_at`; bind tests
  to the Appendix A set. Orchestrator reconciles the 1.4 line at `/orchestrate-end`.)*
- **Related context:** reuses the **1.3 `Clock` contract** (`now_rfc3339`/`now_plus_secs`,
  Z-suffix UTC — lexical compare; LESSON §5) for TTL/expiry, and the **1.1 single writable
  `Connection`** (LESSON §3 — do **not** open a second writer). `locks/` is the persistence-core
  module already named in `daemon/CLAUDE.md`'s layer DAG (sibling of `eventstore`/`projections`).

## Acceptance criteria (what "done" means)
- [ ] **Migration 5** (user_version 4→5; `SUPPORTED_USER_VERSION`→5) creates `leases` per the
      Appendix A field set + `ix_leases_expiry` (on `expires_at`, for the reaper scan);
      backup-before-migrate (§16); events/projections/outbox rows intact across the migration.
- [ ] **`acquire(resource_id, lease_kind, owner_id, ttl_secs)`** acquires when the slot is free
      OR expired; mints a fencing token **strictly greater than any prior token ever issued for
      that `(resource_id, lease_kind)`** (a persisted high-water mark — see Q2); sets
      `owner_id`/`acquired_at`/`heartbeat_at`/`expires_at = now + ttl`. **Refuses** when a *live*
      (non-expired) lease is held by a different owner (returns a typed "held" error, no mutation).
- [ ] **`renew(lease)`** is owner-**and**-token-guarded: only the current holder (matching
      `owner_id` AND `fencing_token` AND `expires_at > now`) extends `expires_at` + bumps
      `heartbeat_at`; **the fencing token is unchanged** (renew ≠ reclaim). A stale/wrong-token/
      expired renew is rejected (typed error), state unchanged.
- [ ] **`release(lease)`** is owner-guarded: frees the slot (`owner_id`/`acquired_at`/`expires_at`
      → NULL) but **preserves the fencing high-water mark** so the next acquire still increments.
- [ ] **Fencing detection (the safety pin):** after a lease is reclaimed by a new acquirer, the
      previous holder's token is **no longer current** — a validation primitive
      (`is_current_token(resource_id, lease_kind, token)` or equivalent) returns `false`. This is
      exactly what the Phase-2 gateway calls to reject a stale-token mutation (safety rule #6).
- [ ] **`reap_once(clock)`** frees every lease past `expires_at` (NULL the holder fields, **keep
      the token**) and returns the reclaimed set — the deterministic unit; the Tokio interval
      spawn is **1.6** (joins the outbox drainer spawn).
- [ ] **`pidlock` single-instance:** a second daemon instance is **refused** while the first holds
      the lock; **PID reuse does NOT yield a false single-instance** (the lock is an OS-held
      advisory file lock, auto-released on process death — *not* a `kill(pid,0)` liveness check;
      see Q3). Releasing/closing frees it.
- [ ] **Restart survival (integration):** fencing monotonicity is **persisted** — after a
      simulated restart (reopen the DB), a new `acquire` on a previously-held `(resource_id,
      lease_kind)` mints a **strictly-greater** token than the pre-restart holder, and the
      pre-restart token fails validation.
- [ ] **Single-writer preserved** — `locks/` writes through the **same single writable
      `Connection`**; no second writable connection is opened (Forbidden #3 / LESSON §3). All unit
      tests pass; `/preflight` clean; cross-doc rows updated atomic with the round (orchestrator writes).

## Wiring / entry point (Step 7.5)
1.4 is a **persistence-core primitive**; its production callers all live above it in the DAG, in
later phases. State this honestly — do **not** claim a live production caller where the consumer
is a later phase (the 1.3 `drain_once` / `rebuild_projections` precedent):
- **`locks::acquire`/`renew`/`release`/`is_current_token`** — production consumer is the
  **Phase-2 Action Gateway** (acquire a lease + re-read the live source after lock, validate the
  token at execute, emit `fencing_conflict` on a stale token — §7.2/§17). For 1.4 these are `pub`
  entries reachable from tests; **gateway-wired in Phase 2.** Say so at Step 7.5.
- **`pidlock` acquire** — production caller is the **1.6 bootstrap cold-start ordering** (§16:
  "pidlock → reclaim stale socket → …"). 1.4 ships the *mechanism*; the cold-start *call site* is
  1.6. Confirm the mechanism is testable now (it is) and the bootstrap wiring is the 1.6 deferral.
- **`reap_once`** — `pub` deterministic unit; **Tokio interval spawn → 1.6** (alongside the outbox
  drainer spawn, §12). Same explicit deferral as 1.3's drainer — not falsely "wired."

## Files expected to touch
**New:**
- `daemon/src/locks/mod.rs` (+ submodules as the impl sees fit, e.g. `lease.rs`/`pidlock.rs`) —
  the `Lease` model + `FencingToken` newtype + `acquire`/`renew`/`release`/`is_current_token`/
  `reap_once` over the single writable `Connection`; the `pidlock` advisory-file-lock guard.
- `daemon/tests/locks.rs` — integration tests (or co-located unit tests) + the advanceable-clock
  seam reused from the 1.3 outbox tests (StepClock/FixedClock) for deterministic expiry.

**Modified:**
- `daemon/src/eventstore/schema.rs` — `MIGRATION_5_LEASES` DDL constant (co-located with M1–M4, the
  established placement) + `ix_leases_expiry`.
- `daemon/src/eventstore/migrations.rs` — register M5; `SUPPORTED_USER_VERSION` 4→5.
- `daemon/src/lib.rs` — `pub mod locks;`.
- `daemon/src/eventstore/mod.rs` — **only if** the chosen plumbing for the shared single writable
  `Connection` requires it (see Q1) — e.g. exposing the connection to `locks/`. Flag the shape at Step 2.5.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)

> **Landed reconcile (1.4 complete, 2026-06-08):** the human-ruled Option-B addition
> (`test_expired_holder_rejected_even_if_unsuperseded`) made **L1 = 6 tests** (1–6), shifting the
> file numbering below: **L2 pidlock = tests 7–8**, **L3 reaper + restart = tests 9–11** (L3 gained
> `test_reap_at_exact_boundary` + reap→re-acquire-increments, closing the test-10 lazy-reclaim-vs-reap
> gap). Function names + coverage match this outline; only the layer→number map shifted. Slice:
> L1 `b3f7612` / L2 `442149a` / L3 `0347ac6`; **11 lease tests; workspace 67 green; security-reviewer
> PASS every layer.**

**Layer 1 — leases table + lease primitive + fencing (`schema.rs`, `migrations.rs`, `locks/`):** ⚠️ safety-critical
1. **`test_migration_5_creates_leases`** — Asserts: `open` migrates 4→5 + creates `leases` +
   `ix_leases_expiry`; events/projections/outbox intact; backup-before-migrate (§16). Why: §16/ADR-008.
2. **`test_acquire_renew_release_happy`** — Asserts: acquire on a free slot sets owner +
   `expires_at=now+ttl` + token=1; renew (same owner+token) extends `expires_at`, token unchanged;
   release frees the slot but the row's fencing token is preserved. Why: ADR-008 / §7.2 happy path.
3. **`test_acquire_refused_while_live_held`** — Asserts: a second `acquire` by a *different* owner
   on a non-expired lease returns the typed "held" error and does not mutate the row. Why: §7.2
   "authoritative" exclusivity.
4. **`test_stale_token_rejected_after_reclaim`** *(the fencing pin — safety rule #6)* — Asserts:
   holder A acquires (token=N); the lease expires (advance the fake clock past `expires_at`); B
   reclaims → token=N+1; A's token N is **no longer current** (`is_current_token`→false) and A's
   `renew` is rejected. Why: §17 line 387 / safety rule #6 — stale-token detectability.
5. **`test_fencing_token_strictly_monotonic`** — Asserts: repeated acquire/release/reclaim cycles
   on the same `(resource_id, lease_kind)` yield strictly-increasing tokens; release never lowers
   the high-water mark. Why: monotonic-fencing invariant (Q2).

**Layer 2 — pidlock single-instance (`locks/`):** ⚠️ protects single-writer
6. **`test_pidlock_refuses_second_instance`** — Asserts: holding the pidlock, a second acquire on
   the same lock path fails (typed error); after release/close, a fresh acquire succeeds. Why: ADR-008
   single-instance.
7. **`test_pidlock_pid_reuse_no_false_single_instance`** *(the PID-reuse pin)* — Asserts: a stale
   lock file containing a **reused/foreign PID** does NOT fool the mechanism (the OS advisory lock,
   not the file's PID contents, is the oracle — a foreign PID's presence in the file neither grants
   nor falsely blocks). Why: §17/ADR-008 — PID reuse must not yield a false single-instance.

**Layer 3 — reaper + restart survival (`locks/`):**
8. **`test_reap_once_frees_expired`** — Asserts: `reap_once` (fake clock past `expires_at`) frees
   expired leases (holder fields NULL, token kept) + returns the reclaimed set; non-expired leases
   untouched. Why: §12 reaper unit.
9. **`test_restart_preserves_fencing_monotonicity`** *(integration)* — Asserts: with a lease
   held+expired at token=N, reopen the DB (simulated restart) → B's acquire mints token=N+1 (the
   high-water mark persisted, not in-memory) and A's token=N fails validation. Why: ADR-008
   "cross-restart" + restart survival.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **`Lease`** (`resource_id, owner_id, fencing_token, acquired_at,
  heartbeat_at, expires_at, lease_kind`) — **daemon-internal** (no `proj_lease`, no `lease_` in the
  22 frozen IDs, **not** one of the 10 §5.1 status machines; the UI/Brain don't read the `leases`
  table). So **no `shared/` contract surface, no CONTRACT_VERSION bump** — analogous to the 1.3
  outbox. **Confirm at Step 2.5 (Q1/Q4).**
- **Orchestrator doc rows to write hot (Step 9):** confirm/refine the **Appendix A `Lease` row**
  (it already exists — add the monotonic-high-water-mark + persisted-across-restart + release-preserves-token
  semantics); add a **`daemon/CLAUDE.md` cross-doc table** `Lease` row (like the outbox row,
  daemon-internal); a **DATA_MODEL `leases` DDL** note; the **LESSON candidate** (lease/fencing
  convention, next anchor §6); **reconcile the `MVP_TASKS.md` 1.4 field list** to the Appendix A set;
  route the **reaper Tokio spawn → 1.6** (joins the outbox drainer item) + the **gateway wiring of
  acquire/validate → a Phase-2 note**.

## Things to flag at Step 2.5
1. **Single writable `Connection` plumbing (the #1 structural question).** How does `locks/` write
   through the **single write-actor** without opening a 2nd writable connection (Forbidden #3 /
   LESSON §3 — concurrent writers corrupt the WAL)? My default vote: `locks/` operates over the
   **same single writable `Connection`** the daemon owns (shared with `EventStore`) — passed in;
   the impl picks the lowest-friction shape (EventStore exposes the connection / a thin shared `Db`
   handle / lease methods co-located) **as long as there is exactly one writer**. Confirm the concrete
   shape here + re-confirm at Step 7.5. Also confirms the **daemon-internal** classification (no
   CONTRACT_VERSION bump).
2. **Fencing monotonicity mechanism + persistence.** My default vote: **one row per `(resource_id,
   lease_kind)`**; `fencing_token` is a high-water mark that only ever **increments in place**;
   `release` NULLs the holder fields but **keeps** the token; `acquire`/reclaim does `token + 1`
   (1 for a brand-new row). It survives restart because it's persisted in the row (no in-memory
   counter, no `MAX()` over deletable rows). Alternative (a dedicated counter table / a global
   counter) — I vote per-`(resource_id, lease_kind)` in-place. Confirm.
3. **pidlock mechanism — std advisory file lock vs PID-liveness.** Toolchain is **rustc 1.93** →
   `std::fs::File::try_lock`/`unlock` is stable (since 1.89). My default vote: **std advisory
   exclusive lock** (`File::try_lock`) on `…/Application Support/NexusOps/daemon.lock`; write the PID
   inside for **diagnostics only**; the OS holds the lock against the open fd → **auto-released on
   process death (even crash) → immune to PID reuse**. **No new dependency.** Reject a bare
   `kill(pid,0)` liveness oracle (false positives on PID reuse). Confirm + verify the
   release-on-process-death guarantee against the std docs at impl time.
4. **`lease_kind` / `owner_id` / `fencing_token` typing.** My default vote: `fencing_token` =
   a **`FencingToken(u64)` newtype** (load-bearing invariant → newtype, per the typing posture);
   `lease_kind` = `TEXT` with a single **seeded MVP kind constant** (e.g. `resource_mutation`) for
   tests — the closed enum defers to the Phase-2 gateway consumer that actually dictates kinds;
   `owner_id` = a minimal typed wrapper over `TEXT`, **not yet bound** to one of the 22 IDs (the
   gateway binds the concrete kind in Phase 2 — keep it the opaque wire-string for now). `resource_id`
   is an **opaque key** to the lease layer (the caller's worktree/repo/session id). Confirm.
5. **Reaper event emission + on-`open` behavior.** My default vote: `reap_once` frees + returns a
   summary, and emits **no new `shared/` event type** in 1.4 (lease-lifecycle event emission —
   `LeaseExpired`/the §17 conflict event — defers to the gateway/session consumer in Phase 2/4 where
   the event contract is defined; avoids a premature CONTRACT_VERSION bump). **No special on-`open`
   lease reset** (unlike the outbox's `reset_in_flight`) — expiry is **lazy + reaper-driven**; whether
   a restart should honor a still-*valid* (non-expired) lease is the **Phase-4 resume-or-replay**
   question, out of 1.4 scope. Confirm.

## Dependencies + sequencing
- **Depends on:** 1.1 event store (the migration runner + the **single writable `Connection`** +
  the `Clock`/`IdGen` seams — LANDED); the **1.3 `Clock` contract** (`now_rfc3339`/`now_plus_secs`
  Z-suffix UTC — reuse; LANDED `707843a`/LESSON §5); frozen `shared/` (resource IDs are opaque keys).
- **Blocks:** **Phase 2** — the Action Gateway **acquires a lease + re-reads the live source after
  lock + validates the fencing token before every mutation** (§7.2 re-read-after-lock invariant;
  §17 `fencing_conflict` → hard-conflict card); **1.6** bootstrap (pidlock cold-start ordering +
  the reaper Tokio spawn); the **§25 demo** (every gateway mutation is lease-guarded).
- **Interacts with:** **1.6** (pidlock is the mechanism here; 1.6 wires the §16 cold-start *ordering*
  + the reaper spawn — no conflict, complementary); **Phase 4** (resume-or-replay decides whether a
  restart honors still-valid leases — explicitly out of 1.4 scope, see Q5).

## Estimated commit count
**3** (multi-commit slice). L1 is a **safety-critical** pin (fencing tokens = safety rule #6 / §17)
→ its **own** commit + security-reviewer; the others are distinct surfaces.
- **L1 — `leases` table (migration 5) + lease primitive (acquire/renew/release) + monotonic fencing +
  token validation** (tests 1–5). ⚠️ safety-critical → **security-reviewer (`invariant` policy)**.
- **L2 — `pidlock` single-instance (std advisory file lock)** (tests 6–7). Protects the single-writer
  invariant (data integrity) → **security-reviewer** too; distinct surface (file lock, not SQLite).
- **L3 — lease reaper (`reap_once`) + restart survival** (tests 8–9). Extends the lease primitive →
  code-quality (`every-slice`); security-reviewer optional.

> ⚠️ **Orchestrator drives layer→layer** (banked lesson, 3 mechanisms): the next-layer directive is
> folded into each SHIP message; I re-wake immediately on any post-commit "proceeding"; roll straight
> into the next layer's RED after committing — no standalone status, no idle gap.

## Lessons-logged candidates anticipated
- **Convention candidate** — "Cross-restart resource locks = a SQLite lease row per `(resource_id,
  lease_kind)` carrying a **monotonic fencing high-water mark** that only increments + **persists
  across restart**; `release` preserves the token; `acquire`/reclaim mints `token+1`; a paused
  holder's stale token fails validation (safety rule #6 / §17 — the gateway rejects a stale-token
  mutation → `fencing_conflict`, hard-conflict card, never auto-resolved)."
- **Convention candidate / safety note** — "Single-instance via a **std advisory file lock** (held
  by the OS fd, auto-released on process death) — never a bare `kill(pid,0)` liveness check (PID
  reuse → false positives). PID written into the lock file is diagnostic only."
- **Architecture-doc note** — `Lease` is **daemon-internal** (no `shared/` contract / no
  CONTRACT_VERSION bump); production consumers are the **Phase-2 gateway** (acquire/validate before
  mutate) + the **1.6 bootstrap** (pidlock cold-start + reaper spawn).
- **Future TODO — 1.6:** the reaper's Tokio interval spawn (joins the outbox drainer spawn);
  **Phase 2:** the gateway wires acquire/validate → `fencing_conflict` `ActionFailed` + conflict
  event + hard-conflict card.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5"; Q1 (single-writer
   plumbing) + Q2 (fencing persistence) + Q3 (pidlock mechanism) need answers before tests.
2. **Run `/tdd lease_locks_and_fencing`** in the implementer session.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — send the test-design write-up (one `Asserts: <invariant> (§anchor)` line per
   test) + answers to Q1–Q5. ⚠️ This slice touches safety rule #6 — a safety-design question
   escalates to the human **before** sign-off.
6. **Step 7.5** — state the reachability honestly: acquire/validate → Phase-2 gateway, pidlock →
   1.6 bootstrap, reaper spawn → 1.6 (pub primitives, consumer-wired later — not false wiring).
7. **Step 9 (summarize)** — categorized flags per "Lessons-logged candidates anticipated."

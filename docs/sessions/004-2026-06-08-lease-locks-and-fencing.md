# Session 004 — Phase 1.4 lease locks + monotonic fencing + pidlock + reaper

| | |
|---|---|
| **Date** | 2026-06-08 |
| **Phase** | Phase 1 (daemon foundation) — task 1.4 |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | [003](003-2026-06-08-projections-and-outbox.md) |
| **Successor** | [005](005-2026-06-08-uds-gatewayport-transport.md) — Phase 1.5 UDS GatewayPort transport |
| **Commits** | **1.4:** `b3f7612` (L1 leases+fencing+validate_held) · `442149a` (L2 pidlock) · `0347ac6` (L3 reaper+restart) — slice complete |

## Why this session existed
One daemon-core slice per brief `006-P1-4-lease-locks-and-fencing.md`: the **cross-restart resource-lock primitive** behind **Key safety rule #6 (fencing tokens mandatory)**. A persistent SQLite `leases` row per `(resource_id, lease_kind)` carrying a **monotonic fencing token** that only increments and survives restart; owner-guarded `acquire`/`renew`/`release`; the **authority oracle** the Phase-2 gateway calls to reject a stale-token mutation (`fencing_conflict`, §17); an expired-lease **reaper**; and **single-instance** enforcement via a std advisory file lock. A 3-layer multi-commit slice (L1 safety-critical, L2 protects single-writer, L3 reaper).

## What was built

### L1 — leases table + lease primitive + monotonic fencing + authority oracle (`b3f7612`)
- **Files created:** `daemon/src/locks/lease.rs` — `ResourceId`/`LeaseKind`/`OwnerId`/`FencingToken(u64)` newtypes + `Lease` handle + `LeaseError{Held,NotHolder,TokenOverflow,Db}` + `acquire`/`renew`/`release`/`validate_held` (read-modify-write under `BEGIN IMMEDIATE`; owner+token+expiry-guarded). `daemon/src/locks/mod.rs` — module + re-exports (pub types, `pub(crate)` free fns). `daemon/tests/locks.rs` — tests 1–6 + the `StepClock` seam + read-only assertion helpers.
- **Files modified:** `eventstore/schema.rs` (`MIGRATION_5_LEASES` + `ix_leases_expiry`), `migrations.rs` (register M5; `SUPPORTED_USER_VERSION` 4→5), `eventstore/mod.rs` (`acquire_lease`/`renew_lease`/`release_lease`/`validate_lease_held` delegating methods over the single writable `Connection` + `locks` import), `lib.rs` (`pub mod locks`), `tests/outbox.rs` + `tests/projections.rs` (relaxed 3 stale `user_version == N` pins to `>= N`).

### L2 — pidlock single-instance (`442149a`)
- **Files created:** `daemon/src/locks/pidlock.rs` — `PidLock` guard + `PidLockError{AlreadyHeld,Io}`; `acquire` via std `File::try_lock` (stable 1.89; toolchain 1.93). OS holds the lock against the fd → auto-released on process death → immune to PID reuse; PID written diagnostic-only (guarded on `set_len` success).
- **Files modified:** `locks/mod.rs` (`mod pidlock` + re-export + module doc), `tests/locks.rs` (tests 7–8).

### L3 — reaper (`reap_once`) + restart-survival (`0347ac6`)
- **Files modified:** `locks/lease.rs` (`reap_once` — select-then-free in a `BEGIN IMMEDIATE` txn; NULLs holder fields, **keeps** the fencing token; returns the reclaimed set), `locks/mod.rs` (re-export `reap_once`), `eventstore/mod.rs` (`reap_leases` delegating method), `tests/locks.rs` (tests 9–11).

**Totals:** 4 files created, 6 modified; **11 integration tests** (`tests/locks.rs`); workspace **67 passed / 0 failed (11 suites)**.

## Decisions made
- **Q1 single-writer plumbing** — `locks/` is a persistence-core sibling of free fns over the single writable `Connection`; `EventStore` exposes thin delegating methods (the `drain_once`/`rebuild_projections` precedent; eventstore→projections sibling dep already existed). No second writable connection (Forbidden #3 / LESSON §3).
- **Q2 fencing persistence** — one row per `(resource_id, lease_kind)`; `fencing_token` is a high-water mark (new→1, reclaim→token+1); `release`/reaper NULL holder fields but **keep** the token; survives restart because persisted in the row.
- **Authority = a LIVE lease (Option B, human-ratified).** `validate_held` = owner-match ∧ `token == fencing_token` ∧ `expires_at > now`. An expired holder is rejected even if unsuperseded — closing the gap §17 line 387 left open. `validate_held` **subsumed** the originally-planned `is_current_token` (single oracle — avoids a weaker public check the gateway could misuse).
- **Q3 pidlock** — std advisory `File::try_lock` (no new dependency); `WouldBlock`→`AlreadyHeld`, IO error→typed `Io` (fail-closed: never "acquired"); no `kill(pid,0)` oracle.
- **Q4 typing** — `FencingToken(u64)` newtype (load-bearing → newtype; i64↔u64 fail-closed at the SQLite boundary); `LeaseKind`/`OwnerId`/`ResourceId` minimal `String` newtypes (the closed kind-enum + owner-id binding defer to the Phase-2 gateway).
- **Q5 events / on-open** — `reap_once` emits no new `shared/` event type; no special on-open lease reset (expiry is lazy + reaper-driven).
- **`leases` is daemon-internal** — no `shared/` contract surface, **no CONTRACT_VERSION bump** (stays 0.8.0), analogous to the 1.3 outbox.
- **Reaper boundary** — `reap_once` frees `expires_at <= now` (the exact complement of `live`'s `> now`), so a lease at exactly `now` is reaped and the two checks never overlap (pinned by test 11).

## Decisions explicitly NOT made (deferred)
- **Same-owner re-acquire / idempotent-acquire semantics** — current behavior returns `Held{owner:self}` on ANY live re-acquire; did NOT invent a test locking a guessed contract. → Phase-2 gateway decides.
- **renew-at-exact-expiry-boundary (`> now` strict) + TTL≤0 handling** — → Phase-2 gateway heartbeat loop.
- **Restart honoring a still-valid (non-expired) lease** — → Phase-4 resume-or-replay (out of 1.4 scope).
- **Tokio interval spawn for the reaper** — `reap_once` is the deterministic unit; the spawn (joins the outbox drainer spawn, §12) → 1.6 bootstrap.
- **Closed `LeaseKind` enum + `OwnerId` binding to a frozen `shared/` id** — → Phase-2 gateway.

## TDD compliance
**Clean for all three features.** Each layer was strict RED → Step-2.5 (full design reviewed once; human ADDed the Option-B `validate_held` + the expired-holder test) → GREEN. RED was confirmed for the right reason every layer (missing `locks` module / `PidLock` / `reap_leases` — compile-fail on missing impl, not test typos).
- **Note (not a violation):** **test 11** (`test_reap_boundary_then_reacquire_increments`) was added *after* L3 GREEN in response to a Step-8 code-quality finding — it strengthens coverage of `reap_once`'s `==now` boundary + reap→re-acquire-increment, pinning behavior that was already correct. The `reap_once` feature itself was test-first (tests 9–10). This is coverage-strengthening from review, not implement-before-test.
- **Test-fixture edits:** relaxed 3 stale `user_version == N` pins (outbox ×1, projections ×2) to `>= N` after M5 raised the version (the exact-version pin now lives in each migration's own test). Test edits to accommodate a legitimate version bump, not implementation-chasing.

## Reachability
All 1.4 surfaces are `pub` primitives reachable from `tests/locks.rs`; **none has a production entry point yet** — honest "pub primitive, consumer-wired later" (the 1.3 `drain_once` precedent), stated at each Step 7.5:
- **`acquire_lease`/`renew_lease`/`release_lease`/`validate_lease_held`** → consumer is the **Phase-2 Action Gateway** (acquire → re-read-after-lock → validate at execute → `fencing_conflict` on a stale token, §7.2/§17).
- **`PidLock::acquire`** → call site is the **1.6 bootstrap cold-start ordering** (§16: pidlock → reclaim stale socket → create app-support dir → migrate DB → bind UDS).
- **`reap_leases`** → the **Tokio interval spawn is 1.6** (joins the outbox drainer spawn, §12). Restart-survival is a persistence property the test verifies directly (no production wiring needed).

No tested-but-silently-unreachable gaps — every deferral names its consumer phase.

## Open follow-ups
**Orchestrator-owned (routed hot during the session; land at `/orchestrate-end`):**
- **Cross-doc (model add — NOT drift):** add the **Appendix A `Lease` row** (monotonic-high-water + persisted-across-restart + release-preserves-token + Option-B live-lease-authority) + a `daemon/CLAUDE.md` cross-doc `Lease` row (daemon-internal) + a DATA_MODEL `leases` DDL note; reconcile the `MVP_TASKS.md` 1.4 field list to the Appendix A set.
- **§17 line-387 clarification** — "stale = NOT a live lease (expired OR superseded)" (the Option-B ruling).
- **LESSON §6 candidate** — cross-restart lease = per-`(resource_id,lease_kind)` monotonic fencing high-water mark (persisted; release/reap keep it; acquire mints +1); authority = a live lease; single-instance via a std advisory file lock (OS-fd held, auto-released on death) — never `kill(pid,0)`.
- **Brief 006 test-number reconcile** — L1 gained the Option-B test, so pidlock = tests 7–8 and reaper = tests 9–11 (brief says 6–7 / 8–9).

**Future TODO — Phase-2 gateway:** wire acquire → re-read-after-lock → `validate_held` → `fencing_conflict ActionFailed` + conflict event + hard-conflict card; decide same-owner idempotent-acquire; handle renew-at-exact-expiry-boundary + TTL≤0.

**Future TODO — 1.6 bootstrap:** pidlock cold-start call site; reaper Tokio interval spawn (joins the outbox drainer spawn); optional `PidLockError::Io` coverage test (missing-parent-dir) + a diagnostic-PID-write assertion.

## How to use what was built
The Phase-2 gateway holds the `EventStore` (single connection owner) and, per mutation: `acquire_lease(resource, kind, owner, ttl, clock)` → do the work → `validate_lease_held(resource, kind, owner, token, clock)` at execute; a `false` is the `fencing_conflict` path. The 1.6 bootstrap calls `PidLock::acquire(lock_path)` first in the cold-start order and spawns a Tokio interval calling `reap_leases(clock)` alongside the outbox drainer.

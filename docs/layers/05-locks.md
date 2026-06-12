# Leases, fencing tokens & single-instance (`daemon/src/locks/`)

## Executive summary

This layer is the daemon's concurrency referee. NexusOps will run several AI agent sessions that may want to touch the same resource (a worktree, a repo) at the same time — and the daemon itself can crash and restart mid-work. The locks layer answers two questions safely: "who is allowed to mutate this resource right now?" (SQLite-backed **leases** with monotonic **fencing tokens** that survive restarts) and "is exactly one daemon running?" (a **PidLock** advisory file lock the OS releases automatically when the process dies). It is deliberately daemon-internal — the UI and Project Brain never see it — and it exists today as a primitive: the Phase-2 Action Gateway that will actually consult it before every mutation is not yet built.

## Responsibilities

- **Accountable for:** cross-restart mutual exclusion per `(resource_id, lease_kind)` slot, minting strictly-monotonic fencing tokens, answering the live-authority question (`validate_held`), reaping expired leases, and refusing a second daemon instance (`PidLock`).
- **NOT accountable for:** deciding *when* to check fencing (that is the Phase-2 Gateway's job — `daemon/src/eventstore/mod.rs:486-488`), heartbeating/renewing long-running holders (Gateway, task 2.4 — `IMPLEMENTATION_PLAN.md:296`), or any `shared/` contract surface — `leases` is explicitly not a cross-language contract, has no `lease_` ID among the frozen 22, and bumped no CONTRACT_VERSION (`daemon/src/locks/mod.rs:13-14`, `daemon/CLAUDE.md` Lease row).
- **NOT a writer of its own:** all lease writes go through the daemon's single writable `Connection`, driven by `EventStore` delegation — never a second connection (`daemon/src/locks/lease.rs:19-21`, `daemon/src/eventstore/mod.rs:483-485`).

## Key components

| Component | What it does | Where |
|-----------|--------------|-------|
| `MIGRATION_5_LEASES` | DDL: one row per `(resource_id, lease_kind)` PK; nullable holder fields; `fencing_token INTEGER NOT NULL DEFAULT 0`; `ix_leases_expiry` index | `daemon/src/eventstore/schema.rs:300-312` |
| `FencingToken` | Newtype over `u64`; i64 SQLite conversion fails closed on overflow (`TokenOverflow`) | `daemon/src/locks/lease.rs:55-69` |
| `ResourceId` / `LeaseKind` / `OwnerId` | Opaque `TEXT` newtypes (not closed enums, not bound to the frozen 22 IDs — Phase-2 Gateway binds concrete kinds) | `daemon/src/locks/lease.rs:29-50` |
| `Lease` | Granted handle returned by acquire/renew; the holder presents its `owner_id` + `fencing_token` back | `daemon/src/locks/lease.rs:73-82` |
| `acquire` | Mint token = high-water + 1 (1 for a new row) under `BEGIN IMMEDIATE`; refuse a live foreign holder with typed `Held` | `daemon/src/locks/lease.rs:124-182` |
| `renew` | Extend `expires_at`, bump `heartbeat_at`, **token unchanged**; owner+token+expiry guarded (renew ≠ reclaim) | `daemon/src/locks/lease.rs:187-224` |
| `release` | NULL the holder fields, **preserve** the fencing high-water mark | `daemon/src/locks/lease.rs:228-247` |
| `validate_held` | **The authority oracle**: owner-match AND token == high-water AND `expires_at > now` | `daemon/src/locks/lease.rs:254-270` |
| `reap_once` | Periodic sweep: free every lease with `expires_at <= now`, keep tokens, return the reclaimed set | `daemon/src/locks/lease.rs:282-328` |
| `PidLock` | Single-instance gate: std advisory file lock (`File::try_lock`), OS-fd-held, auto-released on death | `daemon/src/locks/pidlock.rs:33-69` |
| `EventStore` delegation | `acquire_lease` / `renew_lease` / `release_lease` / `validate_lease_held` / `reap_leases` over the single writable connection | `daemon/src/eventstore/mod.rs:492-553` |
| `spawn_reaper` | Tokio interval loop calling `reap_leases` through the write-actor `WriteHandle` | `daemon/src/runtime/drainer.rs:57-79` |
| Integration tests (11) | L1 leases/fencing/oracle · L2 pidlock · L3 reaper/restart | `daemon/tests/locks.rs:150-575` |

## Interfaces & contracts

**Visibility split (`daemon/src/locks/mod.rs:21-24`):** the free functions (`acquire`, `renew`, `release`, `validate_held`, `reap_once`) are `pub(crate)` — reachable *only* through the connection-owning `EventStore`. The types (`Lease`, `FencingToken`, `LeaseError`, `LeaseKind`, `OwnerId`, `ResourceId`) and `PidLock`/`PidLockError` are `pub` because they cross `EventStore`'s public method signatures.

The crate-public surface (all on `EventStore`, `daemon/src/eventstore/mod.rs:492-553`):

| Call | Inputs → Output |
|---|---|
| `acquire_lease` | `(resource_id, lease_kind, owner_id, ttl_secs, &dyn Clock)` → `Ok(Lease)` with a freshly-minted token, or `Err(Held{owner_id})` if a *live* lease is held by another (zero mutation on refusal) |
| `renew_lease` | `(&Lease, ttl_secs, clock)` → `Ok(Lease)` with extended expiry + same token, or `Err(NotHolder)` (wrong owner / stale token / already expired) |
| `release_lease` | `(&Lease)` → `Ok(())` freeing the slot, or `Err(NotHolder)` |
| `validate_lease_held` | `(resource_id, lease_kind, owner_id, token, clock)` → `Ok(bool)` — `false` for a free slot, expired lease, superseded token, or wrong owner |
| `reap_leases` | `(clock)` → `Ok(Vec<(ResourceId, LeaseKind)>)` — the reclaimed set |

`PidLock::acquire(path)` → `Ok(PidLock)` (hold it for the daemon's lifetime; `Drop` releases) | `Err(AlreadyHeld)` | `Err(Io)` — an IO error is **never** treated as acquired (`daemon/src/locks/pidlock.rs:16-27,42-57`).

**What it expects from callers:** an injectable `Clock` (RFC3339 Z-UTC strings — expiry comparison is *lexical*, `daemon/src/locks/lease.rs:145-147`), and — forward-looking — the Phase-2 Gateway to call `validate_held` after lock + before execute, mapping `false` → `fencing_conflict` (`daemon/src/eventstore/mod.rs:486-488`, `IMPLEMENTATION_PLAN.md:296`).

## Data & state

One table, daemon-internal, created by migration 5 (`daemon/src/eventstore/schema.rs:300-312`):

```sql
CREATE TABLE leases (
  resource_id   TEXT NOT NULL,
  lease_kind    TEXT NOT NULL,
  owner_id      TEXT,              -- NULL = slot free
  fencing_token INTEGER NOT NULL DEFAULT 0,  -- monotonic high-water mark; NEVER reset
  acquired_at   TEXT,
  heartbeat_at  TEXT,
  expires_at    TEXT,
  PRIMARY KEY (resource_id, lease_kind)
);
CREATE INDEX ix_leases_expiry ON leases(expires_at);
```

- **The row is the high-water mark.** `fencing_token` is persisted in the row — never an in-memory counter, never a `MAX()` over deletable rows — so monotonicity survives restart (`daemon/src/eventstore/schema.rs:290-293`; proven by `daemon/tests/locks.rs:506-539`).
- **Free slot = NULLed holder triple, token kept.** `release` and the reaper NULL `owner_id/acquired_at/heartbeat_at/expires_at` but keep `fencing_token` so the next acquire still increments — no token reuse (`daemon/src/locks/lease.rs:226-247,312-321`).
- The one seeded `LeaseKind` is `"resource_mutation"` (`daemon/src/locks/lease.rs:40-44`); the closed-enum freeze is deferred to the Phase-2 consumer.
- `PidLock` state lives in the OS: the advisory lock is held against the open fd inside the `PidLock` value (`daemon/src/locks/pidlock.rs:33-36`). The PID written into the file is **diagnostics only**, never read back as a liveness oracle (`daemon/src/locks/pidlock.rs:61-66`).

## Dependencies

- **Depends on:** `crate::clock::Clock` (injected; deterministic expiry in tests via `StepClock`, `daemon/tests/locks.rs:50-68`); rusqlite (`BEGIN IMMEDIATE` transactions); the event store's migration runner (MIGRATION_5 applies inside `EventStore::open`). `PidLock` depends only on std (`File::try_lock`, stable since Rust 1.89 — `daemon/src/locks/pidlock.rs:10-12`).
- **Used by:** `EventStore` (delegating methods over `self.conn`, `daemon/src/eventstore/mod.rs:492-553`); the runtime reaper loop (`spawn_reaper` → `WriteHandle::reap_leases` → write-actor thread, `daemon/src/runtime/drainer.rs:57-79`, `daemon/src/runtime/writer.rs:123,226`); the cold-start bootstrap (`PidLock::acquire` as the first refusing gate, `daemon/src/bootstrap.rs:139-140`).
- **Will be used by (NOT YET BUILT):** the Phase-2 Action Gateway as its fencing oracle (`IMPLEMENTATION_PLAN.md:294-298`).

## How it works (flow)

**Lease lifecycle (the fencing story):**

```
acquire(wt_1, resource_mutation, sess_a, ttl)        token=1   (live)
   │  ttl elapses, no renew
   ▼
[zombie window: sess_a expired, nobody reclaimed]    token=1   validate_held(sess_a)=false  ← Option B
   │
acquire(wt_1, ..., sess_b)  — reclaim                token=2   validate_held(sess_a)=false (superseded too)
   │
release(sess_b) — slot freed, token KEPT             token=2
   │
acquire(wt_1, ..., sess_c)                           token=3   (strictly monotonic forever)
```

1. **Acquire** opens a `BEGIN IMMEDIATE` transaction (`daemon/src/locks/lease.rs:135-137`), reads the slot (`read_slot`, :104-117), and branches: no row → token 1; row with a *live* holder (`owner.is_some() && expires_at > now`, lexically compared, :147) → return `Err(Held)` with the tx dropped (rollback — refusal mutates nothing, :149-153); free/expired row → mint `token + 1` (:154). It then upserts the full holder row and commits (:158-171).
2. **Renew** is a single guarded `UPDATE ... WHERE owner AND token AND expires_at > now` — zero rows matched means `NotHolder`, state unchanged; the token is never changed by renew (:200-218). The expiry guard binds `now` in its own parameter slot deliberately (:198-200).
3. **Validate** (`validate_held`, :254-270) is the read-only oracle: `true` iff the persisted row has this owner, this exact token, and `expires_at > now` strictly. A free slot, expired lease, superseded token, or wrong owner all yield `false` (:263-269). A `false` here is, by design, the Gateway's future `fencing_conflict` path (safety rule #6 — never auto-resolved).
4. **Reap** (`reap_once`, :282-328) scans `owner_id IS NOT NULL AND expires_at <= now` and frees each slot, keeping tokens, under one `BEGIN IMMEDIATE` (:287-322). The `<=` is the exact complement of liveness's strict `>`, so a lease at exactly `now` is reaped (:291-292; pinned by `daemon/tests/locks.rs:544-575`). The runtime wires this as a Tokio interval task through the write-actor (`daemon/src/runtime/drainer.rs:62-78`).
5. **PidLock** at cold-start: `create_dir_all` → `PidLock::acquire` → only then `EventStore::open` — the lock strictly precedes any DB write so a second instance can never reach a concurrent migration (`daemon/src/bootstrap.rs:128-148`). `acquire` takes a non-blocking exclusive `File::try_lock`; `WouldBlock` → `AlreadyHeld`, real IO error → fail-closed `Io` (`daemon/src/locks/pidlock.rs:53-57`); then writes its PID for diagnostics only (:63-66).

## Design decisions & rationale

- **SQLite lease table + monotonic fencing + pidlock = ADR-008** (`ARCHITECTURE.md:37`, `[LOCKED]`). The `leases` table is the authoritative lock source of truth; "expired → reclaim w/ new fencing token" (`ARCHITECTURE.md:215`, §7.2 state-SoT table).
- **Authority = a LIVE lease ("Option B", human-ratified 2026-06-08).** The original §17 text only covered the superseded-token case; the zombie-holder window (expired but unsuperseded) was a gap "§17 line 387 left open" (`daemon/src/locks/lease.rs:11-17`, `daemon/tests/locks.rs:364-365`). The ruling: "stale" = NOT a live lease — expired **OR** superseded — now recorded in the §17 fencing row (`ARCHITECTURE.md:397`) and pinned by `daemon/tests/locks.rs:332-378`. Trade-off: stricter (an expired-but-unsuperseded holder loses authority it arguably still "safely" had), in exchange for a closed window — the cost is that long actions must heartbeat (see Gotchas).
- **Persisted high-water mark, not in-memory or `MAX()`.** Survives restart by construction (`daemon/src/eventstore/schema.rs:290-293`); proven across a real close/reopen in `daemon/tests/locks.rs:506-539`.
- **OS advisory file lock, not `kill(pid, 0)`.** A PID-probe liveness check false-positives when the OS recycles a dead daemon's PID; the fd-held lock is immune and auto-releases on crash (`daemon/src/locks/pidlock.rs:1-10`; pinned by `daemon/tests/locks.rs:440-455`). Zero new dependencies (std `try_lock`).
- **Daemon-internal, no contract surface.** UI/Brain never read `leases`, so no `shared/` types, no `proj_lease`, no CONTRACT_VERSION bump — explicitly analogous to the 1.3 outbox (`daemon/src/locks/mod.rs:13-14`, `ARCHITECTURE.md:516` Appendix-A Lease row).
- **Single-writer discipline.** Free functions are `pub(crate)` and take the `Connection` that `EventStore` owns; nothing in `locks/` opens its own connection (Forbidden #3 / LESSON §3 — `daemon/src/locks/lease.rs:19-21`, `daemon/src/eventstore/mod.rs:483-485`).

## Gotchas & sharp edges

- **The zombie-holder window is closed on the *validation* side, not the holder side.** An expired holder fails `validate_held` even though its token is still the high-water mark (`daemon/tests/locks.rs:332-378`). Consequence: a legitimately long-running action **must renew before its own expiry or it self-fences** into a false `fencing_conflict`. That heartbeat loop is the Phase-2 Gateway's obligation, recorded as task 2.4 (`IMPLEMENTATION_PLAN.md:296,298`) — **not yet built**.
- **Same-owner re-acquire of a live lease returns `Held{owner: self}`.** 1.4 deliberately did not pin whether that should be idempotent/renew-like; the Gateway decides in 2.4 (`IMPLEMENTATION_PLAN.md:297`). Callers today must not assume re-acquire is a renew.
- **Strict expiry boundary.** Liveness is `expires_at > now` (strict) everywhere (`daemon/src/locks/lease.rs:147,204,265`); reap uses the complement `expires_at <= now` (:297), so a lease at exactly `now` is dead *and* reapable (`daemon/tests/locks.rs:555-563`). A future heartbeat loop must renew strictly *before* the boundary (`IMPLEMENTATION_PLAN.md:298`).
- **Timestamps are lexically-compared TEXT.** Correct only because the injected `Clock` emits RFC3339 Z-UTC (`daemon/src/locks/lease.rs:145-147,291-292`). A non-Z or offset-bearing timestamp would silently break expiry comparison.
- **`heartbeat_at` is currently write-only.** `renew` updates it, but `read_slot` selects only `(owner_id, fencing_token, expires_at)` (`daemon/src/locks/lease.rs:104-117`) — nothing reads `heartbeat_at` yet. Diagnostic until a Gateway/staleness consumer appears. UNVERIFIED whether Phase 2 will read it directly or rely solely on `expires_at`.
- **`FencingToken` fails closed on i64 overflow** (`TokenOverflow`, `daemon/src/locks/lease.rs:59-68`) — practically unreachable, but a load-bearing token is never silently wrapped.
- **`LeaseKind`/`OwnerId` are open strings**, not closed enums or frozen-ID newtypes (`daemon/src/locks/lease.rs:32-50`) — intentional deferral to the Phase-2 consumer; until then nothing stops a typo'd kind from creating a parallel slot.
- **The pidlock file's PID content is a red herring by design** — a stale foreign PID neither grants nor blocks acquisition; only the OS lock matters (`daemon/tests/locks.rs:440-455`). The PID write is even best-effort/non-fatal (`daemon/src/locks/pidlock.rs:61-66`).
- **Arch-vs-code drift: none found.** The §17 fencing row (`ARCHITECTURE.md:397`), Appendix-A Lease row (`ARCHITECTURE.md:516`), and code agree post-Option-B ruling; the doc was updated to record the ruling rather than the code drifting from the doc.

## Connects to

- **[02-event-store.md](02-event-store.md)** — `EventStore` owns the single writable connection and exposes the lease API as delegating methods (`daemon/src/eventstore/mod.rs:492-553`); MIGRATION_5 rides the same `user_version` migration chain (`daemon/src/eventstore/schema.rs:300-312`).
- **[07-daemon-runtime.md](07-daemon-runtime.md)** — cold-start orders `PidLock::acquire` strictly before `EventStore::open` (`daemon/src/bootstrap.rs:134-148`); the reaper runs as a runtime interval task through the write-actor (`daemon/src/runtime/drainer.rs:57-79`, `daemon/src/runtime/writer.rs:123,226`).
- **[01-shared-contracts.md](01-shared-contracts.md)** — negative space: `leases` is deliberately **not** a shared contract (no `lease_` ID, no CONTRACT_VERSION bump, `daemon/src/locks/mod.rs:13-14`).
- **[OVERVIEW.md](OVERVIEW.md)** — the Phase-2 Action Gateway (not yet built) is the intended production caller: `validate_held` after lock + before execute → `false` → `fencing_conflict`, never auto-resolved (safety rule #6; `daemon/src/eventstore/mod.rs:486-488`, `IMPLEMENTATION_PLAN.md:294-298`).

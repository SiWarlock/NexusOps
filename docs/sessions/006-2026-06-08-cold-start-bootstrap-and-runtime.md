# Session 006 — Phase 1.6a (cold-start bootstrap) + 1.6b (daemon runtime)

| | |
|---|---|
| **Date** | 2026-06-08 |
| **Phase** | Phase 1 (daemon foundation) — tasks 1.6a (L1+L2) + 1.6b |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | [005](005-2026-06-08-uds-gatewayport-transport.md) |
| **Successor** | _(TBD — next cycle resumes the queue: 1.6a-L3 registration · 1.6c §17 replay · 1.6d subscribe-serve · 1.7 redactor)_ |
| **Commits** | **1.6a:** `48b7a2c` (L1 typed user_version + non-swallowed RestoreFailed) · `f1de088` (L2 cold_start + DaemonContext). **1.6b:** `26898a8` (L1 write-actor + main.rs) · `9448035` (L2 drainer/reaper loops + bounded drain) · `8d06ed7` (L3 UDS bind + accept-loop + cap) · `f9c31b1` (L4 broadcast publish-after-commit) · `39f756e` (review fixes) |
| **Base** | `25fe66d` (Phase 1.5 round seal) |
| **Contract** | `CONTRACT_VERSION` unchanged at **0.11.0** (all of 1.6a-L1/L2 + 1.6b is daemon-internal — no `shared/` surface) |

## Why this session existed

Phase 1.6 (first-run bootstrap + the runtime that drives it) was the next daemon
slice after 1.5 landed the UDS transport mechanism. 1.6 is large, so the
orchestrator split it: **1.6a** (cold-start orchestration), **1.6b** (the async
runtime — the "wire the runtime" half of the 1.3/1.4/1.5 "ship the mechanism,
wire the runtime at 1.6" pattern), **1.6c** (§17 degradable replay), and a later
carve **1.6d** (subscribe-serve). This session delivered **1.6a-L1/L2 + all of
1.6b**. 1.6a-L3 (Device/LocalRunner registration) was held mid-session on an
INV-SEC-1 user ruling and re-deferred to the next cycle at round close-out.

## What was built

### Files created
- `daemon/src/bootstrap.rs` — `cold_start(BootstrapConfig) -> Result<DaemonContext, BootstrapError>`; the §16 cold-start orchestration (create_dir_all → PidLock → EventStore::open → version-info); `DaemonVersionInfo` (report-only); `DaemonContext::into_parts()` for the runtime.
- `daemon/src/main.rs` — the `#[tokio::main]` production bin entry: resolve the macOS app-support dir → cold_start → write-actor → drainer/reaper loops → UDS bind + accept-loop → block on SIGTERM/SIGINT → graceful shutdown.
- `daemon/src/runtime/mod.rs` · `writer.rs` · `drainer.rs` · `listener.rs` — the runtime topology: the single **write-actor** (dedicated thread + mpsc `WriteHandle`), the drainer/reaper interval loops, the UDS bind + accept-loop, and the broadcast delta source.
- `daemon/tests/bootstrap.rs` — 1.6a-L2 cold-start tests (6).
- `daemon/tests/runtime.rs` — 1.6b runtime tests (13).

### Files modified
- `daemon/src/eventstore/mod.rs` — `user_version() -> Result<u32,_>` (was `i64`/`-1` sentinel; out-of-u32 → `Migration` not `Reconstruct`); `EventStoreError::RestoreFailed { from, migration_error, source }`; `read_all_degradable` unreadable-seq quarantine; re-export `DRAIN_BATCH_LIMIT`.
- `daemon/src/eventstore/migrations.rs` — `on_migration_failure` classifier (clean rollback vs failed restore, never swallowed; preserves the original migration error); inline unit tests.
- `daemon/src/eventstore/outbox.rs` — `DRAIN_BATCH_LIMIT` (128) cap on the due-rows query (bounded drain; the 1.3 deferral).
- `daemon/src/ipc/peer.rs` · `mod.rs` — `current_euid()` (the daemon's own effective uid → `daemon_uid`).
- `daemon/src/lib.rs` — `pub mod bootstrap;` + `pub mod runtime;`.
- `daemon/Cargo.toml` + `Cargo.lock` — `tokio` (rt-multi-thread, macros, signal, sync, time, net) — the architecture-mandated async runtime (§12); the `nexusopsd` bin auto-discovered from `main.rs`.
- `daemon/tests/{eventstore,locks,outbox,projections}.rs` — adapt the `user_version()` callers to the typed `Result`.

## Decisions made
- **Write-actor = dedicated OS thread + mpsc (Q1, user-ratified).** rusqlite is blocking, so the one writable `EventStore` is owned by a dedicated thread (never a tokio worker); async callers hold a cloneable `WriteHandle` (append/drain_once/reap_leases/subscribe) + await oneshot replies. Reads stay `open_read_only`. The broadcast sender lives in the actor, publishing **after commit**. Concretization *within* the locked single-write-actor (forbidden #3 / LESSON §3), not a new arch call.
- **Cold-start ordering:** `create_dir_all` (idempotent prereq, no DB) → `PidLock::acquire` (first refusing gate) → `EventStore::open` (migrate/floor/replay) — the pidlock strictly precedes any DB write, so a 2nd instance can never reach a concurrent migration (security-reviewer confirmed).
- **Bounded drain = a const `DRAIN_BATCH_LIMIT` (128) in the query** (no signature change; the 1.3 tests insert < limit so they're unaffected).
- **Stale-socket reclaim = unlink-before-bind** (safe ∵ the pidlock guarantees single-instance).
- **Concurrency cap = a semaphore; at-cap REFUSES** (drops the connection); the permit releases on close (no leak).
- **Lag policy (Q3) = subscriber-detects-`Lagged` + resync** (no new wire frame → no CONTRACT bump).
- **L1 restore-failure tested via a pure classifier** (`on_migration_failure`) + inline unit, not a forced symmetric-copy disk fault (determinism-for-testability).

## Decisions explicitly NOT made (deferred)
- **1.6a-L3 (Device/LocalRunner registration)** — held mid-session on the INV-SEC-1 ruling, then **user-ruled Option B** (system-event append `actor_type=System`; `ws_0…0` system-workspace sentinel; `DeviceId`/`LocalRunnerId` `minted_id!` newtypes off `DesktopObjectKind`, NOT new `IdKind`). RED tests + a `desktop_minted_id!` design were drafted at L3 Step-2.5 and then reverted at close-out (no impl shipped). **Brief 027 L3 is updated; re-create + build next cycle (closes task #11).**
- **1.6b subscribe-SERVE push (brief 028 test 11)** — the socket push that streams `ProjectionDelta`s to a subscribed client. The broadcast SOURCE + `push_subscription` unit + `handle.subscribe()` are all ready; the remaining serve-layer wiring (thread the broadcast `Sender` into `serve_connection` + `spawn_accept_loop`; ~13 call-site edits + the post-ack streaming path) was carved to **task #13 / 1.6d** rather than rush a broad change to 1.5's safety-adjacent serve code under budget.
- **`subscription_id` on the subscribe ack** (Q2) — daemon-internal MVP is 1-subscription-per-connection; deferred → Carry-forward.

## TDD compliance
**Clean.** Every layer drove RED→GREEN→commit: L1 (eventstore typed errors) confirmed RED via the type-mismatch + missing-classifier compile errors; L2 (bootstrap) RED via the missing module; 1.6b L1–L4 RED via missing `runtime` symbols at each step; GREEN confirmed per layer; full suite green before each commit. The 1.6a-L3 RED tests were written then reverted (held) — no implementation shipped against them, so no violation. Step-2.5 reviews completed for both slices.

## Reachability
- **`cold_start`** — reachable from `main.rs` (the production bin) + `daemon/tests/bootstrap.rs`.
- **`user_version`/`RestoreFailed`/`on_migration_failure`** — on the `EventStore::open → migrations::run` path.
- **write-actor (append/drain_once/reap_leases)** — `main.rs` → `WriteActor::spawn` → the drainer/reaper loops + the accept-loop's serve.
- **`drain_once`/`reap_leases`** — production callers via the interval loops (closes the 1.3/1.4 deferral).
- **`serve_connection`/`bind`/`peer_uid`** — the accept-loop; a real client handshakes + reads a projection over a real socket (closes the 1.5 deferral).
- **broadcast delta SOURCE** — every committed `append` publishes (reachable).
- **GAP (tested-but-not-fully-wired):** the broadcast→socket subscribe **consumer** (push) — `handle.subscribe()` exists + the broadcast publishes, but no production caller streams it to a connection yet → **task #13 / 1.6d**.

## Open follow-ups
- **1.6a-L3** (registration) — brief 027 ready (Option B); re-create RED + build next cycle → closes task #11.
- **1.6c** (§17 degradable replay + audit-integrity event) — brief 029 ready (Option C); includes the re-homed `-1`-seq `DegradableEvent` typing cleanup.
- **1.6d** (task #13) — the socket subscribe-serve push.
- **1.7** — redactor entropy fallback (blocking Phase-1 acceptance).
- **Carry-forward:** `subscription_id` wire field (if >1 subscription/connection needed) · macOS-vs-other-unix cfg-guard over the getpeereid surface (reviewer-sanctioned; no Linux CI yet) · the `"SessionStarted"` string-literal duplication across `runtime/writer.rs` + the 1.2 projectors → a shared event-type-name const (Phase-1 cleanup) · await in-flight serve tasks on shutdown via a `JoinSet` (low-risk — serve tasks use their own read-only connections).
- **Reviewer outcomes:** security-reviewer PASS on both slices (forbidden #3 + rule #7 end-to-end); code-quality fix-in-slice items applied (MissedTickBehavior::Delay, WriteActor Drop, biased shutdown, reaper-survives test, RestoreFailed context).

## Resumption — next session's first slices: 1.6a-L3, then 1.6c

The round closed at the 1.6b boundary on the **lead's** ACTION-tier cycle ruling — NOT
any test/design issue. The orchestrator **blessed the 1.6a-L3 Step-2.5 design in
principle** so the next session recreates the (reverted) RED tests + builds GREEN
immediately. Build spec:

**1.6a-L3 (registration; closes task #11) — Option B, user-ruled. Brief 027 L3 updated.**
1. **`desktop_minted_id!` macro (APPROVED)** — a sibling to `minted_id!` in
   `shared/src/objects.rs`: transparent `String` newtype; `new()` / `as_str()` /
   `parse()` (validates prefix + ULID) / `Default` / `Display`; `KIND: DesktopObjectKind`;
   prefix from `DesktopObjectKind::id_prefix()`; reuse `ids::IdError`. Instantiate
   `DeviceId` = `Device`, `LocalRunnerId` = `LocalRunner`. **Frozen-22 guard:** assert
   `IdKind::from_prefix("dev_") == None` / `"lr_" == None` (the 22-ID set is NOT expanded).
2. **System-workspace sentinel — SHARED (APPROVED).** A reserved contract value: a
   `SYSTEM_WORKSPACE_ID` const + a `WorkspaceId::system()` accessor in `shared/`;
   value `ws_00000000000000000000000000` (all-zero ULID, valid `parse`). Bootstrap's
   System-actor registration events carry this as `workspace_id`.
3. **`EventStore::first_event_of_type(&str) -> Result<Option<EventEnvelope>>` (APPROVED,
   daemon-internal)** — bootstrap reads it for register-if-absent (reuse an existing
   `DeviceRegistered`'s id + workspace vs mint).
4. **3-way verify — Rust-side schema gate (`cargo run --bin emit_schema` + the byte-diff
   contract test) is the bar now**; flag the cross-language verify (`npx json-schema-to-zod`
   + `uvx datamodel-code-generator`) for where the tools exist — covered by the existing
   Carry-forward "wire §5.0 gates into CI" (1.5 hit the same).
5. **Mechanism:** registration is a **System-actor system event** (`actor_type=System`)
   appended via the write-actor; **LocalRunner minted per start**, **Device register-if-absent**;
   folded into `object_refs` (payload-sourced id) + a CONTRACT_VERSION bump **0.11.0→0.12.0**
   (the orchestrator authors that commit message + the EventTypeRegistry/Appendix-A rows hot).
6. **Test set (approved-in-principle; recreate as RED):** desktop-newtype + frozen-22 pin ·
   registration payload wire-contract (`deny_unknown_fields`) · `localrunner_minted_per_start`
   (2 starts → 2 distinct `lr_` + 2 events) · `device_stable_across_restarts` (1 `dev_`, 1 event) ·
   `registration_event_redacted_and_projected` (`redacted` + `actor_type=System` + object_refs).
   security-reviewer applies (INV-SEC-1-touching).

**1.6c (§17 degradable replay; brief 029) — Option C, user-ruled.** WIRES the existing
`DegradableEvent`/`read_all_degradable` primitives (1.1/1.2) into `replay`/`open`, adds a
quarantine record + an **audit-integrity event** (CONTRACT bump) + the unredacted-replay-skip,
and absorbs the re-homed `-1`-seq `DegradableEvent` typing cleanup (code-quality flag from this
session).

**Then 1.6d (#13, subscribe-serve) and 1.7 (redactor entropy fallback).**

## How to use what was built
`cargo run -p nexusopsd` starts the daemon: it cold-starts (single-instance pidlock, migrate, version-floor), stands up the write-actor + the outbox/reaper interval loops + the UDS GatewayPort accept-loop, and serves `get_projection`/`get_capabilities`/subscribe-ack over the socket until SIGTERM/SIGINT → graceful drain + exit. The ui's `MockGatewayPort` can swap to the real client for **reads** now; live **subscriptions** wait on 1.6d.

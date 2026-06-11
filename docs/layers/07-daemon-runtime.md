# Daemon runtime & bootstrap

## Executive summary

This layer is the process skeleton of `nexusopsd`, the trust-core daemon: how the binary starts, who is allowed to write the database, what background loops keep running, and how it shuts down. At launch it runs a strictly ordered cold-start (one-instance lock first, then database open/migrate, then self-registration events), and then stands up a small set of long-lived tasks: a single dedicated writer thread that owns the only writable database connection, two interval loops (outbox drainer, lease reaper), and the Unix-socket accept loop the UI connects through. It exists so that the "single mutator / single DB writer" invariant is structural — there is physically one thread that can write — rather than a convention each module must remember. Everything else in the daemon either sends commands to that writer over a channel or reads through read-only connections.

## Responsibilities

- **Cold-start ordering** — create the app-support dir, acquire the single-instance pidlock *before* any DB write, open/migrate the event store, record daemon identity (Device/LocalRunner) and quarantine audit events (`daemon/src/bootstrap.rs:134-180`). It is NOT the DB lifecycle itself — migrate/version-floor/replay all live inside `EventStore::open` and are only *composed* here (`daemon/src/bootstrap.rs:6-9`).
- **The single write-actor** — one dedicated OS thread owns the sole writable `EventStore`; every mutation in the process funnels through its `WriteHandle` (`daemon/src/runtime/writer.rs:1-9`). It is NOT a policy layer: it executes whatever typed command arrives; risk-classification/approval is the (not-yet-built) Phase-2 Action Gateway.
- **Background loops** — outbox drainer (5 s) and lease reaper (30 s) interval tasks that survive failed passes (`daemon/src/runtime/drainer.rs`).
- **UDS listen/accept** — bind the GatewayPort socket (stale-socket reclaim, 0600 perms), bound-concurrency accept loop, per-connection peer-uid read (`daemon/src/runtime/listener.rs`). It is NOT the wire protocol — framing/handshake/dispatch is the IPC layer's `serve_connection`.
- **Entry + shutdown** — `#[tokio::main]` wiring of all of the above plus SIGTERM/SIGINT graceful drain (`daemon/src/main.rs:34-103`).
- **Determinism seams** — injectable `Clock` and `IdGen` traits so tests replay byte-identical logs (`daemon/src/clock.rs`, `daemon/src/idgen.rs`).

## Key components

| Component | What it does | Where |
|-----------|--------------|-------|
| `cold_start(cfg)` | §16-ordered bootstrap: dir → pidlock → open → version facts → registration → quarantine audit | `daemon/src/bootstrap.rs:134-180` |
| `BootstrapConfig` | Injected inputs: base dir + `IdGen`/`Clock`/`Redactor` boxes | `daemon/src/bootstrap.rs:46-51` |
| `DaemonContext` / `into_parts` | Holds live `PidLock` + `EventStore` + version facts + `dev_`/`lr_` ids; consumed by the runtime | `daemon/src/bootstrap.rs:69-92` |
| `DaemonVersionInfo` | Report-only version tuple (app, protocol range, DB user_version, contract) | `daemon/src/bootstrap.rs:58-64` |
| `register_device` / `register_local_runner` | Device register-if-absent; LocalRunner minted per start — System-actor events via `EventStore::append` | `daemon/src/bootstrap.rs:184-225` |
| `system_intent` | Builds the System-actor envelope: `ActorType::System`, `WorkspaceId::system()` sentinel, `Visibility::System` | `daemon/src/bootstrap.rs:231-256` |
| `WriteActor::spawn` | Spawns the dedicated `nexusops-write-actor` OS thread owning the writable store | `daemon/src/runtime/writer.rs:145-162` |
| `WriteHandle` | Cloneable async handle: `append`/`drain_once`/`reap_leases` over mpsc + oneshot reply | `daemon/src/runtime/writer.rs:68-133` |
| `run_actor` | The actor loop: `blocking_recv` commands, execute, publish deltas after commit | `daemon/src/runtime/writer.rs:199-231` |
| `deltas_for_append` | Maps a committed intent → live `ProjectionDelta`s (currently `SessionStarted` only) | `daemon/src/runtime/writer.rs:239-252` |
| `spawn_drainer` / `spawn_reaper` | Interval loops calling the writer; failure-surviving; watch-channel stop | `daemon/src/runtime/drainer.rs:22-53,57-79` |
| `bind` | UDS unlink-before-bind + 0600 perms | `daemon/src/runtime/listener.rs:29-40` |
| `spawn_accept_loop` | Semaphore-capped accept; `getpeereid` read; blocking `serve_connection` per conn | `daemon/src/runtime/listener.rs:48-117` |
| `main` / `run` | `#[tokio::main]` entry: prod seams, cold_start, task spawn-up, signal wait, drain chain | `daemon/src/main.rs:34-103` |
| `Clock` (`SystemClock`/`FixedClock`) | RFC3339-`Z` timestamp seam (lexical-sort contract) | `daemon/src/clock.rs:14-59` |
| `IdGen` (`UlidGen`/`FixedIdGen`) | `evt_`/`out_` id seam; FixedIdGen uses separate counters per id class | `daemon/src/idgen.rs:12-62` |

## Interfaces & contracts

- **`cold_start(BootstrapConfig) -> Result<DaemonContext, BootstrapError>`** (`daemon/src/bootstrap.rs:134`). Fail-closed: a start that can't prove single-instance AND a sound DB returns an error, never a half-initialized context (`daemon/src/bootstrap.rs:94-113`). `PidLockError::AlreadyHeld` maps to the clean `BootstrapError::AlreadyRunning` refusal (`daemon/src/bootstrap.rs:115-124`).
- **`DaemonContext::into_parts() -> (PidLock, EventStore, DaemonVersionInfo)`** — the runtime MUST keep the returned `PidLock` bound for the daemon's lifetime; dropping it releases single-instance (`daemon/src/bootstrap.rs:81-92`). `device_id`/`local_runner_id` are deliberately *not* in the tuple (durably in the log; re-queryable via `first_event_of_type`, `daemon/src/eventstore/mod.rs:389`).
- **`WriteHandle`** — the process-wide mutation API: `append(AppendIntent) -> Result<EventId, RuntimeError>`, `drain_once(Arc<dyn Destination>) -> DrainSummary`, `reap_leases() -> Vec<(ResourceId, LeaseKind)>` (`daemon/src/runtime/writer.rs:93-132`). A dead actor yields the distinct `RuntimeError::ActorGone` (`daemon/src/runtime/writer.rs:36-43`).
- **`WriteHandle::subscribe()` / `delta_sender()`** — a `broadcast::Receiver<ProjectionDelta>` / a clone of the sender, handed to the accept loop so each connection mints its own receiver per subscribe request (`daemon/src/runtime/writer.rs:80-89`).
- **`Clock` contract:** `now_rfc3339()` MUST emit `Z`-suffix UTC (not `+00:00`) — outbox due-selection compares timestamps *lexically*, so a mixed offset form would strand rows (`daemon/src/clock.rs:7-13`).
- **Loop spawners:** `spawn_drainer/spawn_reaper/spawn_accept_loop` all take a `watch::Receiver<bool>` shutdown signal and return a `JoinHandle<()>` (`daemon/src/runtime/drainer.rs:22-27,57-61`; `daemon/src/runtime/listener.rs:48-55`).

## Data & state

- **Filesystem layout** (prod base dir = `$HOME/Library/Application Support/NexusOps`, `daemon/src/main.rs:108-112`): `daemon.pid` pidlock (`daemon/src/bootstrap.rs:40`), `nexusops.db` (`daemon/src/bootstrap.rs:38`), `events.jsonl` JSONL mirror sink (`daemon/src/main.rs:28`), `gateway.sock` UDS (`daemon/src/main.rs:30`).
- **Channels:** mpsc command channel depth **64** (`COMMAND_CHANNEL_DEPTH`, `daemon/src/runtime/writer.rs:25`) — backpressures mutation floods onto senders; broadcast capacity **256** (`BROADCAST_CAPACITY`, `daemon/src/runtime/writer.rs:30`) — a subscriber lagging beyond it is dropped (`Lagged`), never back-pressuring the writer.
- **Command enum:** `Append` (boxed intent — keeps channel slots small), `DrainOnce`, `ReapLeases`, `Shutdown` (`daemon/src/runtime/writer.rs:48-63`).
- **`SYSTEM_WORKSPACE_ID` sentinel** — `"ws_00000000000000000000000000"` (`shared/src/ids.rs:210`), typed via `WorkspaceId::system()` (`shared/src/ids.rs:214-217`); the workspace for workspace-less System events like `DeviceRegistered`/`LocalRunnerRegistered` (`daemon/src/bootstrap.rs:241`).
- **Intervals:** drainer 5 s, reaper 30 s, `MAX_CONNECTIONS` 64 (`daemon/src/main.rs:24-32`).
- All durable state lives in the event store; this layer holds only process state (thread/task handles, channels, the held pidlock).

## Dependencies

- **Depends on:** `eventstore` — `EventStore::open/append/drain_once/user_version/first_event_of_type/emit_quarantine_audit_events` (`daemon/src/bootstrap.rs:148-171`, `daemon/src/runtime/writer.rs:210-226`); `locks` — `PidLock::acquire` (`daemon/src/bootstrap.rs:140`) and lease reaping; `ipc` — `peer_uid` + `serve_connection` for each accepted connection (`daemon/src/runtime/listener.rs:24,100-111`); `projections` — indirectly, via the catch-up replay inside `open` (`daemon/src/eventstore/mod.rs:164`); `shared` — IDs, envelope enums, `ProjectionDelta`, protocol range (`daemon/src/bootstrap.rs:17-23`).
- **Used by:** nothing calls in yet besides `main.rs` itself — this *is* the production composition root. The future Phase-2 Action Gateway is the named next consumer of `WriteHandle` (`daemon/src/runtime/writer.rs:5-7`). Tests drive it directly (`daemon/tests/runtime.rs`, `daemon/tests/replay.rs`).

## How it works (flow)

**Cold start** (`daemon/src/bootstrap.rs:134-180`), in binding order:

1. `create_dir_all(base_dir)` — idempotent, race-safe, touches no DB (`:137`).
2. `PidLock::acquire` — the FIRST step that can refuse a second instance, strictly before any DB write, so two instances can never race a migration (forbidden #3) (`:140`).
3. `EventStore::open` — internally: WAL pragmas → DB-newer-than-supported refusal (the enforcing version floor) → migrations with backup/rollback → projection catch-up replay → outbox `in_flight`→`pending` crash recovery (`daemon/src/eventstore/mod.rs:157-167`).
4. Compose report-only `DaemonVersionInfo` (`daemon/src/bootstrap.rs:151-156`).
5. Register identity as System-actor events through the normal redaction-gated `EventStore::append` path: Device register-if-absent (reuse the first `DeviceRegistered`'s id, else mint `dev_`) and a fresh per-start LocalRunner `lr_` (`daemon/src/bootstrap.rs:158-166,184-225`).
6. `emit_quarantine_audit_events` — loud `AuditIntegrityViolation` events for rows the replay quarantined; idempotent via `audit_emitted` + idempotency key (`daemon/src/bootstrap.rs:171`, `daemon/src/eventstore/mod.rs:416`).

**Runtime spin-up** (`daemon/src/main.rs:45-103`):

```
main (#[tokio::main], main.rs:34)
  └─ run(): production_base_dir → cold_start(UlidGen, SystemClock, PrefixRedactor)  (:46-53)
       ├─ WriteActor::spawn(store, SystemClock)  (:63) ──► dedicated OS thread
       │     run_actor: blocking_recv → append/drain/reap → reply        (writer.rs:199-231)
       │     committed append ──► broadcast ProjectionDelta (AFTER commit, :209-220)
       ├─ spawn_drainer(handle, JsonlMirror, 5s, shutdown_rx)   (:72-77)
       ├─ spawn_reaper(handle, 30s, shutdown_rx)                (:78)
       ├─ bind(gateway.sock) → spawn_accept_loop(listener, db_path,
       │        current_euid(), 64, delta_sender, shutdown_rx)  (:82-91)
       │     accept → semaphore permit (at cap: REFUSE/drop, listener.rs:73-76)
       │            → spawn_blocking → into_std → getpeereid → serve_connection (:81-112)
       └─ wait_for_shutdown() [SIGTERM|SIGINT select]           (:93, :116-126)
            → shutdown_tx.send(true) → await drainer, reaper, accept
            → actor.shutdown() (FIFO drain + thread join)        (:96-100)
            → _pidlock drops → single-instance lock releases     (:101)
```

- The interval loops use `MissedTickBehavior::Delay` (no tick bursts after a slow pass) and a `biased` select so shutdown beats a ready tick; a failed pass is logged and survived — liveness over correctness of any single pass (`daemon/src/runtime/drainer.rs:29-49`).
- `bind` unlinks any leftover socket file first — safe because the pidlock already guarantees single-instance, so a present socket is from a dead daemon — then sets 0600 (`daemon/src/runtime/listener.rs:29-40`).
- `WriteActor::shutdown` is FIFO-graceful: queued commands finish, later sends get `ActorGone`, and the thread is joined off-runtime so the `EventStore` drops (WAL checkpoint) before return (`daemon/src/runtime/writer.rs:169-179`). A `Drop` impl `try_send`s `Shutdown` as a non-blocking best-effort backstop (`daemon/src/runtime/writer.rs:182-193`).

**Tests:** 13 runtime tests pin sole-writer routing, post-shutdown refusal, bounded drain (`DRAIN_BATCH_LIMIT`), loop survival, reaper effect, stale-socket reclaim, foreign-uid rejection, connection cap + permit release, real-socket reads, publish-after-commit (rolled-back append publishes nothing), and lagging-subscriber-never-stalls-writer (`daemon/tests/runtime.rs:105,151,179,211,241,267,369,381,414,447,483,513,562`). 8 replay tests pin the §17 degradable-replay behavior cold_start composes — corrupt-row recovery, unknown-version degrade, unredacted-row quarantine, healthy-log no-regression, content-free quarantine reasons, idempotent re-detection, loud-not-silent AIV emission, exactly-one-AIV-across-rebuild (`daemon/tests/replay.rs:141,175,197,226,291,310,347,391`).

## Design decisions & rationale

- **Dedicated OS thread for the writer, not a tokio task** — rusqlite is synchronous/blocking; a blocking SQLite call on a tokio worker would stall the async runtime. `blocking_recv` is correct precisely because this is not a worker thread (`daemon/src/runtime/writer.rs:1-9,195-198`). Recorded as §4.2 / forbidden #3 / LESSON §3 (Q1-ratified per the module doc).
- **Pidlock strictly before DB open** — the ordering makes concurrent migration (the corruption risk forbidden #3 guards) structurally impossible; `create_dir_all` precedes it only because the lock file needs a home (`daemon/src/bootstrap.rs:126-140`). LESSON §8; ARCHITECTURE §16.
- **Publish-after-commit** — deltas broadcast only once the append durably committed; a rolled-back append publishes nothing, so subscribers can never observe uncommitted state (`daemon/src/runtime/writer.rs:209-220`). Lag drops the subscriber rather than back-pressuring the writer (LESSON §9; resync = re-`get_projection`).
- **Self-registration as System-actor events, not Gateway Actions** — USER-RULED Option B (2026-06-08): the daemon establishing its own runtime identity is substrate, not an untrusted proposer intent (also circular pre-Phase-2); it still flows through the §15 redaction gate + projector fold via `append` (`daemon/src/bootstrap.rs:158-164`). LESSON §10; ARCHITECTURE §5.3/§16.
- **Bounded everything** — mpsc depth 64 backpressures mutation floods; the semaphore caps concurrent connections at 64 and *refuses* at cap rather than queueing unboundedly (bounds 8-MiB frame buffers + threads) (`daemon/src/runtime/writer.rs:23-25`, `daemon/src/runtime/listener.rs:42-46,73-76`). ARCHITECTURE §6.4.
- **Quarantine audit emission from the caller, not inside replay** — replay stays append-free; `cold_start` emits after `open` returns, deduped by `audit_emitted` (`daemon/src/bootstrap.rs:168-171`). ARCHITECTURE §17 Option C; LESSON §11.
- **Injectable `Clock`/`IdGen`** — ARCHITECTURE §14 determinism seams: `FixedClock`/`FixedIdGen` make golden-log replay byte-identical; `FixedIdGen` keeps a *separate* outbox counter so outbox writes never perturb the event-id sequence (`daemon/src/idgen.rs:38-41`).
- **The closed deferral chain** — 1.3 shipped `drain_once` as a unit, 1.4 `reap_once`, 1.5 `serve_connection`, each explicitly deferring its runtime wiring; `main.rs` is the production caller that closed that "ship the mechanism, wire the runtime at 1.6" reachability chain (`daemon/src/main.rs:5-7`, `daemon/src/runtime/drainer.rs:6-8`, `daemon/src/eventstore/mod.rs:176-179`).

## Gotchas & sharp edges

- **In-flight serve tasks are NOT awaited on shutdown.** `accept.await` joins only the accept *loop*; per-connection `spawn_blocking` serve tasks are detached and may still be running when `main` exits. Known, reviewer-sanctioned deferral, bundled with the subscribe-push-thread linger (a push thread blocks on `recv` after a no-delta client disconnect until the next delta or broadcast close) into a future **runtime-shutdown-hardening** slice (`IMPLEMENTATION_PLAN.md:61`, `docs/sessions/007-2026-06-10-degradable-replay-and-subscribe-serve.md:63`).
- **`getpeereid` is macOS/BSD-only with no cfg-guard** — a Linux build would not link; explicitly deferred until Linux CI exists (`daemon/src/runtime/listener.rs:10-12`, `IMPLEMENTATION_PLAN.md:61`).
- **Device/LocalRunner ids deliberately bypass the injected `IdGen`** (`DeviceId::new()` direct) — the determinism seam is scoped to event/outbox ids; registration ids are stored data read back on replay, so rebuild-equivalence still holds (`daemon/src/bootstrap.rs:191-193`).
- **`deltas_for_append` only maps `SessionStarted`** — appends of any other event type currently publish no live delta; later event types must add mappings additively (`daemon/src/runtime/writer.rs:239-252`). The delta carries only the id; subscribers re-read the row via `get_projection`.
- **`DaemonVersionInfo` is report-only** — the *enforcing* floor is the DB `user_version` refusal inside `open`; the protocol range is enforced at the IPC handshake (`daemon/src/bootstrap.rs:53-57`).
- **`main.rs` discards `_version` and the context's `device_id`/`local_runner_id`** (`daemon/src/main.rs:62`) — no live runtime consumer of the registered identity yet; flagged in code as a Step-9 TODO for Phase-3 session→runner binding (`daemon/src/bootstrap.rs:85-88`).
- **Clock `Z`-suffix contract is load-bearing** — a `Clock` impl emitting `+00:00` would silently strand outbox rows (lexical due-compare) (`daemon/src/clock.rs:7-13`).
- **`FixedIdGen` small counters encode 1970-epoch ULIDs** — they sort before any real production id; test-only by design (`daemon/src/idgen.rs:53-55`).
- **Event-type string literals (`"DeviceRegistered"` etc.) are duplicated** across `bootstrap.rs` and the projectors — a tracked Carry-forward consolidation, deferred to the first Phase-2 event-touching slice (`daemon/src/bootstrap.rs:30-33`, `IMPLEMENTATION_PLAN.md:61`).
- **No drift found** between this layer's code and ARCHITECTURE §12/§16 claims as cited in the code; the Phase-2 Action Gateway is absent by plan (the `WriteHandle` is its designated future entry point), not by omission.

## Connects to

- [02-event-store.md](02-event-store.md) — `cold_start` hands the whole DB lifecycle to `EventStore::open` (`daemon/src/bootstrap.rs:148` → `daemon/src/eventstore/mod.rs:151-174`); the write-actor calls `store.append/drain_once/reap_leases` (`daemon/src/runtime/writer.rs:210-226`).
- [03-redaction.md](03-redaction.md) — the bootstrap registration events and every actor append pass the §15 redaction gate inside `append`; prod injects `PrefixRedactor` (`daemon/src/main.rs:51`).
- [04-projections.md](04-projections.md) — catch-up replay at open (`daemon/src/eventstore/mod.rs:164`) and the post-commit `ProjectionDelta` broadcast (`daemon/src/runtime/writer.rs:209-220`).
- [05-locks.md](05-locks.md) — `PidLock::acquire` as the single-instance gate (`daemon/src/bootstrap.rs:140`); the reaper loop drives lease reaping (`daemon/src/runtime/drainer.rs:57-79`).
- [06-ipc.md](06-ipc.md) — the accept loop hands each authenticated fd to `serve_connection` with the delta sender (`daemon/src/runtime/listener.rs:100-111`); peer-auth enforcement itself lives in the IPC layer.
- [01-shared-contracts.md](01-shared-contracts.md) — `SYSTEM_WORKSPACE_ID`/`WorkspaceId::system()` (`shared/src/ids.rs:210-217`), `ProjectionDelta`, the protocol range constants (`daemon/src/bootstrap.rs:21`).
- [08-ui.md](08-ui.md) — the UI is the client on the other end of `gateway.sock` bound here (`daemon/src/main.rs:82`).

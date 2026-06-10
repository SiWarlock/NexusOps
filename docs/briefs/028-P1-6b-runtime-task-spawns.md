# /tdd brief — daemon_runtime_spawns

## Feature
The daemon's **runtime**: a `#[tokio::main]` `main.rs` entry that calls `cold_start()` (1.6a) then stands up the long-running tasks around the **single write-actor** — the outbox drainer + lease reaper interval loops, the UDS `bind()` + accept-loop (peer-auth'd `serve_connection` per connection), and the **live `subscribe` delta-source** (`EventStore::append` → broadcast → subscriber push). This is the "wire the runtime" half of the "ship the mechanism (1.3/1.4/1.5), wire the runtime (1.6)" pattern. **Registration (1.6a-L3) + §17 degradable replay (1.6c) are HELD/PENDING-USER — independent of this slice.**

## Use case + traceability
- **Task ID:** P1.6b (the runtime-task-spawns third of the split 1.6)
- **Architecture sections it implements:** `ARCHITECTURE.md §12` (daemon strategy — the runtime tasks), `§16` (cold-start "bind UDS" + accept-loop step), `§6.1`/`§6.4` (the live read/subscribe serve path), `§7.2`/`§17` (outbox drainer + reaper spawns), `§4.2`/forbidden #3 / LESSON §3 (the single-write-actor — the central design call below).
- **Related context:** depends on **1.6a-L2** landing first (the `DaemonContext` this drives). Read before authoring tests:
  - `daemon/src/eventstore/mod.rs` — `EventStore::drain_once(&mut self, clock, dest)`, `reap_leases(&mut self, clock)`, `append(&mut self, AppendIntent)` (the broadcast publish point — L4), `open_read_only(path)`.
  - `daemon/src/eventstore/outbox.rs` — `Destination` trait, `JsonlMirror::new(path)`, `DrainSummary`, `drain_once` (today selects **all** due rows → L2 adds the bounded LIMIT, the 1.3 deferral), `backoff_secs`, `MAX_RETRIES`.
  - `daemon/src/ipc/server.rs` — `serve_connection<S: Read+Write>(stream, peer_uid, daemon_uid, db_path) -> Result<(),IpcError>` (synchronous → runs on a **blocking** task; the accept-loop is "1.6-bootstrap-wired" per its own doc-comment).
  - `daemon/src/ipc/peer.rs` — `peer_uid(RawFd) -> Result<u32,IpcError>` (getpeereid), `authorize_peer(uid, daemon_uid)`.
  - `daemon/src/ipc/subscribe.rs` — `push_subscription<W: Write>(writer, deltas) -> Result<usize,IpcError>` (the per-delta push unit; the doc-comment specifies the `UnixStream::try_clone` read/write split for L4).
  - `shared/src/ipc.rs` — `ProjectionDelta`/`DeltaKind`/`SubscribeParams`/`ServerFrame`/`SUPPORTED_PROTOCOL_RANGE`.
  - Session doc `005` "How to use what was built" — the intended accept-loop → `spawn_blocking(serve_connection)` shape. LESSON §3 (single-write-actor), §7 (UDS transport).

## Acceptance criteria (what "done" means)
**L1 — write-actor runtime + `main.rs`:**
- [ ] A single owner of the writable `EventStore` (the write-actor) — every mutating call (`append`, `drain_once`, `reap_leases`, future P2 gateway appends) goes through it; all reads stay `open_read_only` (forbidden #3 / LESSON §3). Concretization per the Step-2.5 Q1 design call (dedicated writer **thread** + mpsc command channel is the default — rusqlite is blocking).
- [ ] `main.rs` (`#[tokio::main]`): `cold_start()` → start the write-actor → spawn the runtime tasks → block on a shutdown signal (SIGTERM/SIGINT) → graceful drain + exit (the §16 "drains + exits" posture; the held `PidLock` releases on exit).
- [ ] `cargo run -p nexusopsd` starts the daemon (the bin target now exists; the §16 launchd/spawn integration is Phase 10).
- [ ] Graceful shutdown is deterministic-testable: a shutdown command stops the loops + closes the writer cleanly (no half-applied state; the WAL checkpoint posture per §16/§18).

**L2 — outbox drainer + lease reaper interval loops:**
- [ ] An async task ticks on an interval and asks the write-actor to `drain_once` against the `JsonlMirror` destination (the one Phase-1 real sink); a second ticks `reap_leases`. Intervals are config-injected (deterministic tests use a short/controllable tick).
- [ ] **Bounded drain pass** — `drain_once` (or its query) takes a `LIMIT`/batch cap so a large backlog can't starve the writer in one pass (the 1.3 security-low deferral). Test: N≫limit due rows → one pass drains ≤ limit, the rest next pass.
- [ ] Drainer/reaper errors are logged + the loop survives (one failed pass never kills the task); `DrainSummary` is surfaced for logging.

**L3 — UDS bind + accept-loop:**
- [ ] `bind(socket_path)` — **reclaim a stale socket first** (unlink a leftover socket file so bind doesn't `EADDRINUSE`; safe because the 1.4 pidlock already guarantees single-instance, so any existing socket is stale). Test: a leftover socket file → bind succeeds after reclaim.
- [ ] The accept-loop accepts a `UnixStream`, reads `peer_uid` via `getpeereid`, and hands `(stream, peer_uid, daemon_uid, db_path)` to `serve_connection` on a **blocking** task (`spawn_blocking`, since the read path is synchronous rusqlite).
- [ ] **Concurrency cap** — bound concurrent live connections (a semaphore) so unbounded peers can't exhaust threads / 8-MiB frame buffers (anti-DoS at scale). Test: at-cap → a new connection is refused/queued deterministically.
- [ ] **Platform cfg-guard** — the `getpeereid`/UDS stack is gated to the macOS/BSD target (`#[cfg(...)]`) so a Linux build links (getpeereid is macOS/BSD-only; security-MEDIUM, reviewer-sanctioned defer). A non-macOS build compiles (the accept-loop is cfg'd out or a typed "unsupported platform" stub).
- [ ] End-to-end: a real client connects over the socket, handshakes, and reads a projection (the §6.1 read surface, now LIVE in-process).

**L4 — live subscribe delta-source:**
- [ ] `EventStore::append`, after a successful commit (`apply_all`), publishes the resulting `ProjectionDelta`(s) to a `tokio::sync::broadcast` sender held by the write-actor (publish-after-commit — never publish an uncommitted delta).
- [ ] A `subscribe` connection holds a `broadcast::Receiver`, `try_clone`s its write half, and runs `push_subscription` on the cloned writer while the read half blocks on the next client frame (the read/write split per the subscribe.rs doc-comment).
- [ ] A `subscription_id` handle is returned on the subscribe ack (supports >1 concurrent subscription per connection; the 1.5 deferral).
- [ ] Test (deterministic): append an event → the subscriber receives the matching `ProjectionDelta` frame; a slow/lagging subscriber is handled (broadcast lag → reconnect/resync policy, not a writer stall — forbidden #3: a subscriber must NEVER back-pressure the writer).

**Cross-cutting:**
- [ ] Tests in `daemon/tests/runtime.rs` (+ unit tests per module) pass; `/preflight` clean.
- [ ] No new `shared/` contract beyond what's already frozen unless `subscription_id` needs a wire field (flag at Step 2.5 → CONTRACT_VERSION bump if so).

## Wiring / entry point (Step 7.5)
This slice **closes the reachability gap** for the whole 1.3/1.4/1.5 deferral chain: `main.rs` is the real production entry; from it `cold_start` (1.6a), `drain_once` (1.3), `reap_leases` (1.4), `serve_connection`/`bind`/`push_subscription` (1.5) all become reachable from a production caller. **Name each at Step 7.5** — after this slice, `/wired` each of those symbols should reach `main.rs`. The out-of-process consumer is the **ui** (`MockGatewayPort` → real `UdsGatewayPort`) post-this-slice.

## Files expected to touch
**New:**
- `daemon/src/main.rs` — the `#[tokio::main]` bin entry (the bin target).
- `daemon/src/runtime/` (or `daemon/src/runtime.rs`) — the write-actor, the drainer/reaper loops, the accept-loop, shutdown. (Module layout: flag at Step 2.5.)
- `daemon/src/ipc/` — `bind` + accept-loop (extend the ipc module; the accept-loop may live in `ipc/server.rs` or a new `ipc/listener.rs`).
- `daemon/tests/runtime.rs`.

**Modified:**
- `daemon/src/lib.rs` — `pub mod runtime;`.
- `daemon/src/eventstore/mod.rs` — the broadcast publish in `append` (L4); the bounded `drain_once` LIMIT (L2).
- `daemon/src/eventstore/outbox.rs` — the LIMIT on the due-rows query.
- `daemon/Cargo.toml` — `tokio` features (`rt-multi-thread`, `macros`, `signal`, `sync`, `time`), `tokio::net::UnixListener`.

If implementation needs files beyond this, flag at Step 2.5.

## RED test outline (Step 2)
> **TDD scope note:** the deterministic UNITS are already test-first-covered (`drain_once`/`reap_once`/`serve_connection`/`push_subscription` — 1.3/1.4/1.5). This slice is **test-first for the deterministic wiring** (bounded-LIMIT, stale-socket reclaim, concurrency-cap rejection, broadcast publish-after-commit + no-writer-stall, graceful shutdown) and **integration-covered for the inherently-timing parts** (interval ticking, accept-loop liveness) via a controllable clock + a real ephemeral `UnixListener`. Call out any part you believe can't be made deterministic at Step 2.5 (it's a flag, not a TDD skip).

**L1**
1. `test_write_actor_is_sole_writer` — Asserts: all mutating ops route through the one writer; a read path uses `open_read_only`. Why: forbidden #3 / LESSON §3.
2. `test_graceful_shutdown_stops_loops_clean` — Asserts: a shutdown signal stops the loops + closes the writer with no half-applied state. Why: §16 drain+exit.

**L2**
3. `test_bounded_drain_pass_respects_limit` — Asserts: N≫limit due rows → one pass drains ≤ limit. Why: 1.3 backlog-starvation deferral.
4. `test_drain_loop_survives_a_failed_pass` — Asserts: a failing drain pass logs + the loop continues. Why: liveness.
5. `test_reaper_loop_invokes_reap_once` — Asserts: the reaper task calls `reap_leases` on its tick (controllable clock). Why: §17 reaper spawn.

**L3**
6. `test_bind_reclaims_stale_socket` — Asserts: a leftover socket file → `bind` succeeds (unlink-first). Why: §16 stale-socket reclaim.
7. `test_foreign_peer_rejected_in_accept_path` — Asserts: a connection whose uid≠daemon-uid is rejected (rule #7, end-to-end through the accept path). Why: safety rule #7.
8. `test_connection_cap_enforced` — Asserts: at the cap, a new connection is refused/queued deterministically. Why: anti-DoS.
9. `test_read_projection_over_real_socket` — Asserts: a client handshakes + reads a projection over a real `UnixListener`. Why: §6.1 live read.

**L4**
10. `test_append_publishes_delta_after_commit` — Asserts: a committed append publishes the matching `ProjectionDelta`; a rolled-back append publishes nothing. Why: publish-after-commit correctness.
11. `test_subscriber_receives_delta_frame` — Asserts: a subscribed client receives the `ServerFrame::SubscriptionPush` for an appended event. Why: §6.1 subscribe live.
12. `test_lagging_subscriber_never_stalls_writer` — Asserts: a slow subscriber → broadcast lag is dropped/resync'd, the writer is NOT back-pressured. Why: forbidden #3 (a reader must never block the writer).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none expected, UNLESS `subscription_id` needs a `shared/` wire field (then CONTRACT_VERSION bump). The write-actor + runtime are daemon-internal.
- **Orchestrator doc rows to write hot:** §12/§16 note that the runtime tasks + accept-loop + live subscribe delta-source landed; the §6.1/§6.4 row updated from "bind/accept-spawn → 1.6" to "LIVE"; LESSON candidate on the write-actor model.

## Things to flag at Step 2.5
1. **[PRIMARY — load-bearing within the locked single-writer invariant] The write-actor concretization.** rusqlite is **synchronous/blocking**, so the writer should NOT be a plain async task (blocking the runtime). Options: (a) **a dedicated OS thread owning `EventStore` + an mpsc command channel**; async loops (drainer/reaper/gateway) send commands, replies via oneshot — the classic actor; (b) `Arc<Mutex<EventStore>>` + `spawn_blocking` per op — simpler but serializes writers behind a lock and risks holding it across `.await`; (c) a `tokio::task::spawn_blocking` writer with a channel. **Default vote: (a) dedicated writer thread + mpsc** — idiomatic single-writer, no lock-across-await, clean shutdown, and the natural home for the L4 broadcast sender. This is concretization *within* the architecture-mandated single-write-actor (LESSON §3), not a new architectural decision — but it shapes the daemon's internal API, so confirm before GREEN. Escalate to the orchestrator if you think it rises to a load-bearing arch call.
2. **`subscription_id` — daemon-internal handle or `shared/` wire field?** Default vote: **daemon-internal correlation first**; only add a `shared/` field (CONTRACT_VERSION bump) if the ui needs to address a specific subscription. Flag if the ui contract needs it now.
3. **Broadcast lag policy.** A lagging subscriber overflows the `broadcast` buffer. Default vote: **drop + signal the subscriber to resync** (re-`get_projection`) — never back-pressure the writer (forbidden #3). Confirm the resync contract (a lag marker frame vs silent drop+client-poll).
4. **Runtime module layout.** `daemon/src/runtime/{mod,writer,drainer,listener}.rs` vs a flat `runtime.rs`. Default vote: **a `runtime/` submodule** (4 distinct concerns). Flag if you'd keep it flat.
5. **`main.rs` shutdown signal set.** SIGTERM + SIGINT (+ the §16 `prepare_for_update` intent is Phase 10, out of scope). Default vote: **SIGTERM+SIGINT only** this slice.

## Dependencies + sequencing
- **Depends on:** 1.6a-L2 (`DaemonContext` — the writer/EventStore handle + paths this drives). 1.3 (`drain_once`/`JsonlMirror`), 1.4 (`reap_leases`), 1.5 (`serve_connection`/`peer_uid`/`push_subscription`) — all LANDED.
- **Independent of:** 1.6a-L3 (registration — HELD/PENDING-USER) and 1.6c (§17 replay — HELD/PENDING-USER). The runtime serves reads + subscriptions regardless of whether registration events exist; it is agnostic to `open()`'s replay strategy.
- **Blocks:** the ui `MockGatewayPort` → real `UdsGatewayPort` swap (cross-track, user-timed); Phase 2 mutation methods extend `serve_connection`'s dispatch + the write-actor.

## Estimated commit count
**4** (layer→layer multi-commit; drive each RED→GREEN→commit, no idling between layers):
- **L1** write-actor runtime + `main.rs` · **L2** drainer + reaper loops (+ bounded drain) · **L3** UDS bind + accept-loop (+ concurrency cap + cfg-guard) · **L4** live subscribe delta-source.

Not bundled: each layer is a distinct concern with its own caller base + test surface, and L1 (the write-actor) is large on its own. **No layer is a §15 safety-critical pin** (rule #7 is already enforced in the landed `serve_connection`; this slice exercises it through the accept path). The single-writer invariant (forbidden #3) is pinned by L1 + L4 tests.

## Lessons-logged candidates anticipated
- **Convention candidate** — "The single write-actor is a dedicated blocking thread + mpsc command channel; reads never touch it (open_read_only); the broadcast sender publishes deltas *after* commit; subscribers never back-pressure the writer."
- **Convention candidate** — "Stale-socket reclaim is unlink-before-bind, safe because the pidlock guarantees single-instance."
- **Architecture-doc note candidate** — §12/§16: the runtime task topology (writer thread + drainer/reaper intervals + accept-loop + broadcast fanout).
- **Future TODO — operational** — backpressure/watermark control frames for the Terminal Channel (§6.4) are Phase 3; the broadcast resync policy may want refinement under real subscription load.

## How to invoke
1. Read this brief end-to-end (Q1 — the write-actor model — is the load-bearing design call; confirm at Step 2.5 before GREEN).
2. Confirm 1.6a-L2 has landed (this drives its `DaemonContext`); reconcile the actual `DaemonContext` shape into the file list.
3. `/tdd daemon_runtime_spawns` → Step 0 restate → Step 2.5 write-up (note which timing parts are integration-covered vs test-first).
4. Drive L1→L2→L3→L4 layer-by-layer (RED→GREEN→commit each; no idling between layers).
5. Step 7.5 — `/wired main.rs` reaches cold_start/drain_once/reap_leases/serve_connection/push_subscription (the deferral chain closes here).
6. Step 9 — categorized flags + ship-ask; any `subscription_id` wire field is the orchestrator's CONTRACT_VERSION hot-write.

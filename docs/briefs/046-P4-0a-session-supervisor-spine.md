# /tdd brief — session_supervisor_spine (opt-3 SessionSupervisor + SessionActor + launch seam)

## Feature
The **opt-3 session-lifecycle spine**: a `SessionSupervisor` that spawns + supervises one `SessionActor` per
agent session (each = a Tokio task + an mpsc command mailbox + the §5.1 Session status state, owning a
`HarnessAdapter` + a terminal read-pump), behind a `SessionLauncher` **seam** (daemon-owned-PTY impl now; the
B2-strict survival broker swaps in at 4.1). **FakeHarness/FakePty-driven, NO live agent, NO event emission,
NO mutation** — the real Claude launch + the live INV-SEC-1 interception + the Gateway `session.create`
executor are the **cat-1 4.0b** (deep-dive §8 cat-1 boundary).

## Use case + traceability
- **Task ID:** P4.0a (NEW — the P4 drive-loop spine, ahead of 4.1; deep-dive §8)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.1` (the Session state machine the actor owns +
  drives), `§10` (the daemon long-lived background task — the supervisor is spawned like the drainer/reaper/
  accept-loop), `§0.1` O-2 (the survival foundation the supervisor enables).
- **Widens phase scope because** the P4 drive loop reshapes the daemon-internal `§9.1` `HarnessAdapter` trait
  (the carry-forward "async-ify rides the P4 drive loop" pin) — **no §9.1 CONTRACT change** (the frozen
  `shared/` DATA types are untouched; the trait is daemon-internal + UNFROZEN per LESSON 23/25).
- **Related context:** the **P4 deep-dive §8** (the slice order + the cat-1 boundary + the opt-3 ruling);
  **LESSON §9** (the write-actor = "a task + an mpsc mailbox + a command loop" — the exact pattern the
  session-actor mirrors; the terminal read-pump's `spawn_blocking` precedent); **LESSON 25** (the
  `ClaudeAdapter` + the sync `Box<dyn>` trait this slice drives); the `runtime/` task-spawn home
  (`writer.rs`/`drainer.rs`/`listener.rs` + `main.rs` `spawn_*`).

## Acceptance criteria (what "done" means)
- [ ] A `SessionActor` drives ONE FakeHarness session through its **§5.1 status lifecycle** (creating →
  starting → active → … → a terminal state) via **mailbox commands**, deriving status from the **adapter's
  structured stream, NEVER PTY-scraped** (safety #9).
- [ ] A `SessionSupervisor` **spawns + tracks** N session-actors (by session id), routes a command to a
  specific actor, and **reaps** an actor's `JoinHandle` when it reaches a terminal §5.1 state — **with NO
  auto-restart** (restart-on-crash is the 4.2 child-death-recovery concern; 4.0a tracks + cleans up).
- [ ] A `SessionLauncher` **seam** produces a launchable session: a `FakeLauncher` (FakeHarness+FakePty, for
  tests) + a daemon-owned-PTY impl (the 3.4 `PortablePtyHost`/`PtySpawner`). **The survival-capable broker is
  the named 4.1 swap point** — not built here.
- [ ] **Cat-1 boundary held:** the supervisor path **emits no events, launches no real agent, performs no
  mutation** (a boundary test asserts no write-actor append from this path). The live Claude launch + the
  interception + the Gateway `session.create` executor are 4.0b.
- [ ] The supervisor **shuts down cleanly** on the shutdown signal (stops all actors + awaits their
  `JoinHandle`s — no orphan tasks; the LESSON 9 `JoinSet`/await-on-shutdown discipline).
- [ ] All unit/integration tests in `daemon/tests/session.rs` pass; `/preflight` clean; suite count grows by
  the new tests (320 → 320+N).
- [ ] If applicable: the daemon-internal trait reshape is reflected in the §9.1 AS-BUILT note (orchestrator
  writes hot — Cross-doc below).

## Wiring / entry point (Step 7.5)
`main.rs` **spawns the `SessionSupervisor`** as a long-lived task alongside `spawn_drainer`/`spawn_reaper`/
`spawn_accept_loop` (the §10 daemon-task home; stopped by the shutdown watch). The supervisor is **reachable**
(a running task) and its drive logic is exercised by `daemon/tests/session.rs` (FakeHarness/FakePty). **The
production session-create CALLER** — the Gateway `session.create` executor that drives the supervisor to
launch a *real* agent — is the **named 4.0b deferral** (`none — the live launch caller lands in 4.0b`; the
cat-1 slice, where the live agent + interception land together). This mirrors how 3.4/3.2/043 landed (the
machinery wired + tested; the production caller a named later slice).

## Files expected to touch
**New:**
- `daemon/src/session/mod.rs` — the `SessionSupervisor` (spawn/track/route/reap/shutdown).
- `daemon/src/session/actor.rs` — the `SessionActor` (task + mpsc mailbox + §5.1 status state + the adapter +
  the terminal read-pump drive).
- `daemon/src/session/launcher.rs` — the `SessionLauncher` seam + the daemon-owned-PTY impl + `FakeLauncher`.
- `daemon/tests/session.rs` — the supervisor + actor lifecycle tests (FakeHarness/FakePty).

**Modified:**
- `daemon/src/harness/mod.rs` — if Q1 chooses async-ify: the trait + `FakeHarness`; else unchanged.
- `daemon/src/harness/claude/mod.rs` — if Q1 chooses async-ify: `ClaudeAdapter`; else unchanged.
- `daemon/src/lib.rs` — register the new `session` module.
- `daemon/src/main.rs` — spawn the `SessionSupervisor` + wire its shutdown.
- `daemon/Cargo.toml` — `async-trait` ONLY if Q1 chooses it (else no dep change).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/session.rs` (+ unit tests inline where pure):

1. **`test_session_actor_drives_status_lifecycle`** — a `SessionActor` over a FakeHarness, fed structured
   signals, transitions the §5.1 Session status creating→…→terminal.
   - Asserts: the status sequence matches the §5.1 legal edges; status is read from the adapter stream.
   - Why: §5.1 (Session machine); safety #9 (never PTY-scraped — the FakePty bytes don't drive status).
2. **`test_supervisor_spawns_tracks_routes`** — the supervisor spawns N actors, routes a mailbox command to a
   specific session id.
   - Asserts: N tracked handles; the command reaches the addressed actor only.
   - Why: §10 (opt-3 supervisor); LESSON 9 (the actor/mailbox pattern).
3. **`test_supervisor_reaps_terminal_no_restart`** — an actor reaching a terminal §5.1 state → the supervisor
   reaps its handle + records the terminal status; **no respawn**.
   - Asserts: handle reaped; actor count decremented; NO new actor spawned.
   - Why: 4.0a tracks+cleans; restart-on-crash is 4.2 (deep-dive §8).
4. **`test_adapter_drive_object_safe`** — the chosen adapter-driving mechanism (Q1) keeps `Box<dyn
   HarnessAdapter>` object-safe; FakeHarness + (if async-ified) ClaudeAdapter satisfy it.
   - Asserts: the boxed trait drives a session end-to-end (compile + run).
   - Why: §9.1 (the daemon-internal trait; LESSON 23/25 — unfrozen, object-safe).
5. **`test_launcher_seam_fake_and_pty`** — `FakeLauncher` yields a FakeHarness+FakePty session the actor
   drives; the daemon-owned-PTY impl constructs (smoke).
   - Asserts: the seam produces a drivable session; the broker swap-point is a TODO marker (4.1), not built.
   - Why: deep-dive §8 (the seam; the survival broker swaps here at 4.1).
6. **`test_cat1_boundary_no_emission_no_agent`** — the supervisor path emits no events + launches no real
   agent.
   - Asserts: **no write-actor append** from the supervisor/actor path (a spy on the write handle); the
     launcher uses Fake/daemon-PTY only, never the live-interception hook.
   - Why: the cat-1 boundary (deep-dive §8) — the live agent + interception + `session.create` executor = 4.0b.
7. **`test_supervisor_clean_shutdown`** — on the shutdown signal, all actors stop + their handles are awaited.
   - Asserts: every actor task completes; no orphan task; no panic.
   - Why: LESSON 9 (await-in-flight-on-shutdown; never back-pressure the write-actor).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none in `shared/`** — the trait reshape (if async-ified) is daemon-internal +
  UNFROZEN (LESSON 23/25); the frozen §9.1 DATA types are untouched. **No CONTRACT bump.**
- **Orchestrator doc rows to write hot (Step 9 routing):** the **§9.1 AS-BUILT note** (the new `session/`
  module + the opt-3 `SessionSupervisor`/`SessionActor`/`SessionLauncher` seam + the adapter-drive mechanism
  chosen at Q1) + a **`daemon/CLAUDE.md` module-org row** (the new `session/` edge module). Orchestrator-written
  at the round commit. Not a contract change; no Appendix A row.
- **Shared-contract seam model touched?** No — no `shared/` model, no schema-snapshot test.

## Things to flag at Step 2.5
1. **Adapter-driving mechanism: async-ify the `HarnessAdapter` trait, or drive the SYNC trait from the async
   `SessionActor` via `spawn_blocking`?** The carry-forward names "async-ify" as the P4 deferral, but the
   leaner route may be to keep the trait **sync** and drive it from the async actor via `spawn_blocking` (the
   terminal read-pump's exact precedent — no `async-trait` dep, matches LESSON 23's "no speculative
   async-trait dep"). My default vote: **drive the sync trait from the async actor via `spawn_blocking`**
   (leaner, no new dep, the established pattern) — async-ify ONLY if the actor genuinely needs in-trait
   `await` you can't get from `spawn_blocking`. Your call as you build it — flag which you took.
2. **Module placement: a new `daemon/src/session/` module vs `daemon/src/harness/`?** The tracker names
   `harness/supervisor.rs` for 4.2 (child-death), but the opt-3 session-lifecycle spine is a distinct concern
   (it *drives* harness + terminal + proposes Gateway intents). My default vote: **a new `daemon/src/session/`
   edge module** (depends on `harness`/`terminal`/`gateway`; never writes the DB — an edge per the layer rule).
   If you'd rather co-locate in `harness/`, flag it.
3. **Cat-1 boundary scope — confirm 4.0a launches NO live agent + emits NO events + does NO mutation**
   (FakeHarness/daemon-PTY only; the Gateway `session.create` executor + the live interception + the live
   Claude launch are 4.0b). My default vote: **yes, that boundary** — a live un-intercepted agent is an
   INV-SEC-1 gap, so the live launch lands *with* the interception in the cat-1 4.0b. (Safety-relevant — flag
   any pressure to pull a live launch forward.)
4. **Supervisor-restart policy in 4.0a:** on an actor reaching a terminal/failed state, the supervisor
   **reaps + records, NO auto-restart**. My default vote: **no auto-restart here** — restart/recovery is 4.2
   (child-death) + 4.1 (survival); 4.0a is the spine + clean lifecycle. Flag if you see a reason to add a
   restart hook now (a seam is fine; the policy is 4.2).
5. **The `SessionLauncher` return shape (the 4.1 broker swap-point).** My default vote: the seam returns a
   launched-session bundle (the `Box<dyn HarnessAdapter>` + the `TerminalSession` + the session id), so the
   4.1 survival broker is a drop-in `SessionLauncher` impl behind the same seam (the daemon-owned-PTY impl is
   the non-surviving default). Shape it for that swap; flag if the broker needs more.

## Dependencies + sequencing
- **Depends on:** 3.1 (the §9.1 trait ✅), 3.2 (the ClaudeAdapter ✅), 3.4 (the terminal host + `PtySpawner`
  ✅), 1.6b (the `runtime/` task-spawn pattern + `main.rs` ✅). All landed.
- **Blocks:** **4.0b** (the cat-1 live interception + the Gateway `session.create` executor drive this
  supervisor) · **4.0c** (the telemetry pump rides the actor) · **4.1** (survival swaps the broker behind the
  4.0a launcher seam) · **4.2** (child-death recovery hooks the supervisor's reap).

## Estimated commit count
**~3** (drive layer→layer; non-safety — FakeHarness, no live agent, no mutation → bundling is safe):
- **L1** — the adapter-drive mechanism (Q1) + (if async-ified) the trait/`FakeHarness`/`ClaudeAdapter`
  reshape, object-safe; the `SessionActor` status-lifecycle drive (tests 1, 4).
- **L2** — the `SessionLauncher` seam + `FakeLauncher` + the daemon-owned-PTY impl (test 5).
- **L3** — the `SessionSupervisor` (spawn/track/route/reap) + the `main.rs` wiring + clean shutdown + the
  cat-1 boundary test (tests 2, 3, 6, 7).
**Not bundled with 4.0b** — 4.0b is cat-1 (own commit + security pass); the spine must be bisectable from the
live-interception wiring. _(Orchestrator drives the impl layer→layer — it idles after each layer commit.)_

## Lessons-logged candidates anticipated
- **Convention candidate** — the **opt-3 session-actor pattern** (LESSON §9's write-actor idiom applied to
  sessions: a task + an mpsc mailbox + the §5.1 status state, an EDGE actor that proposes intents + never
  writes the DB; the supervisor tracks/reaps/shuts-down). Likely a `daemon/LESSONS.md` entry.
- **Architecture-doc note candidate** — the §9.1 AS-BUILT (the new `session/` module + the supervisor/actor/
  launcher seam + the Q1 adapter-drive mechanism + the 4.1 broker swap-point).
- **Future TODO — belongs-to-a-phase** — the survival broker impl behind the launcher seam (4.1); the
  restart-on-crash hook (4.2); the Gateway `session.create` executor + the live launch (4.0b).

## How to invoke
1. **Read this brief end-to-end** + the **P4 deep-dive §8** (the slice order + the cat-1 boundary). Don't skip
   "Things to flag at Step 2.5" — Q1 (the adapter-drive mechanism) + Q3 (the cat-1 boundary) need answers
   before GREEN.
2. **Run `/tdd session_supervisor_spine`.**
3. **Step 2.5** — send the test-design write-up (the §5.1/#9/§10 assertions + the coverage map) + your Q1–Q5
   answers; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
4. **Step 9** — surface the adapter-drive mechanism chosen (Q1) + the module placement (Q2) for the §9.1
   AS-BUILT note, plus anything outside the anticipated lessons candidates.

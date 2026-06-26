//! P4.0a — the opt-3 session-lifecycle spine (`SessionActor` + `SessionSupervisor` + the
//! `SessionLauncher` seam). FakeHarness/FakePty-driven; **NO live agent, NO event emission, NO
//! mutation** — the cat-1 boundary (the live Claude launch + the INV-SEC-1 interception + the Gateway
//! `session.create` executor) is **4.0b** (deep-dive §8).
//!
//! **Safety #9** — the actor derives status from the adapter's structured stream, NEVER from PTY
//! bytes (the `FakePty` noise streamed in parallel does not move the §5.1 `Session` status).
//! **Cat-1 boundary** — the `session` module takes no `WriteHandle` + no live-interception hook, so
//! emission/mutation are compile-time impossible; the terminal pump's exit event lands in a
//! collecting sink (a test double), never the write-actor.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use nexusops_shared::events::TerminalProcessExited;
use nexusops_shared::harness::{
    HarnessCapabilities, NormalizedStatus, TelemetrySample, TranscriptRef,
};
use nexusops_shared::ids::SessionId;
use nexusops_shared::status::Session;
use nexusopsd::harness::{
    FakeHarness, HarnessAdapter, MutationIntercept, ResumeMode, ResumeResult,
};
use nexusopsd::session::actor::{SCROLLBACK_SAVE_INTERVAL, STATUS_POLL_INTERVAL};
use nexusopsd::session::{
    spawn_session_actor, FakeLauncher, LaunchedSession, SessionCommand, SessionLauncher,
    SessionSupervisor,
};
use nexusopsd::terminal::{
    ExitStatus, FakePty, FakeScrollbackStore, HeadlessVt, NoopScrollbackStore, PtyRead,
    ScrollbackStore, TerminalEventSink, TerminalId, TerminalSession,
};

// ---- test doubles -------------------------------------------------------------------------------

/// 075c — a no-op scrollback store for the lifecycle tests (they exercise the actor/supervisor, not
/// the scrollback axis; the producer→store→`Replayed` path is in `tests/scrollback_recovery.rs`).
fn noop_store() -> Arc<dyn ScrollbackStore> {
    Arc::new(NoopScrollbackStore)
}

/// A fully-capable `HarnessCapabilities` (every capability true) — the satisfiable baseline.
fn full_caps() -> HarnessCapabilities {
    HarnessCapabilities {
        supports_terminal: true,
        supports_resume: true,
        supports_transcript_read: true,
        supports_tool_call_parsing: true,
        supports_usage_metadata: true,
        supports_context_metadata: true,
        supports_command_injection: true,
        supports_subagents: true,
        supports_hooks: true,
        supports_cloud_tasks: true,
    }
}

/// A `HarnessAdapter` whose `stream_status` pops a scripted §5.1 sequence (one per poll), modelling
/// the adapter's own structured-stream ingestion advancing between polls. `launch` → `Starting`.
/// Interior mutability (the trait method is `&self`); `Send` via `Mutex`.
struct ScriptedHarness {
    caps: HarnessCapabilities,
    statuses: Mutex<VecDeque<Session>>,
    last: Mutex<Session>,
}

impl ScriptedHarness {
    /// `post_launch` = the §5.1 statuses streamed AFTER `launch()→Starting` (the last is terminal).
    fn new(post_launch: Vec<Session>) -> Self {
        let last = *post_launch.last().expect("a non-empty scripted sequence");
        Self {
            caps: full_caps(),
            statuses: Mutex::new(post_launch.into()),
            last: Mutex::new(last),
        }
    }
}

impl HarnessAdapter for ScriptedHarness {
    fn capabilities(&self) -> HarnessCapabilities {
        self.caps.clone()
    }
    fn launch(&mut self) -> NormalizedStatus {
        Session::Starting
    }
    fn stream_status(&self) -> NormalizedStatus {
        let mut q = self.statuses.lock().unwrap();
        match q.pop_front() {
            Some(s) => {
                *self.last.lock().unwrap() = s;
                s
            }
            None => *self.last.lock().unwrap(),
        }
    }
    fn intercept_mutation(&self) -> Option<MutationIntercept> {
        None
    }
    fn read_transcript(&self) -> Option<TranscriptRef> {
        None
    }
    fn telemetry_heartbeat(&self) -> Option<TelemetrySample> {
        None
    }
    fn resume(&self) -> ResumeResult {
        // 4.1a (Q5 split): preserve this double's deliberate value — the old `resumed_live: true`
        // (re-attached to a live process) maps to `ReattachedLive`. Unread by any assertion; safe.
        ResumeResult {
            mode: ResumeMode::ReattachedLive,
            replayed_event_count: 0,
        }
    }
}

/// A `TerminalEventSink` that COLLECTS the pump's `TerminalProcessExited` into a shared Vec — the
/// cat-1 proof surface: the exit event is collected, NEVER persisted to the write-actor.
struct CollectingTerminalSink {
    exits: Arc<Mutex<Vec<TerminalProcessExited>>>,
}

impl TerminalEventSink for CollectingTerminalSink {
    fn emit_process_exited(&self, event: TerminalProcessExited) {
        self.exits.lock().unwrap().push(event);
    }
}

/// Build a `TerminalSession` over a `FakePty` with the given scripted reads + a collecting sink.
fn fake_terminal(
    id: &str,
    reads: Vec<PtyRead>,
) -> (TerminalSession, Arc<Mutex<Vec<TerminalProcessExited>>>) {
    let exits = Arc::new(Mutex::new(Vec::new()));
    let pty = FakePty::new(
        reads,
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        },
    );
    let terminal = TerminalSession::new(
        TerminalId::from_raw(id),
        Box::new(pty),
        Box::new(CollectingTerminalSink {
            exits: exits.clone(),
        }),
    );
    (terminal, exits)
}

// ---- L1: SessionActor status-lifecycle drive (tests 1, 4) ---------------------------------------

#[tokio::test]
async fn test_session_actor_drives_status_lifecycle() {
    // spec(§5.1) + safety #9 — the actor drives the §5.1 Session status from the adapter's structured
    // stream (creating→starting→active→…→terminal) and is reaped at the terminal state; the FakePty
    // bytes streamed in parallel do NOT drive status (display-only, #9).
    let adapter = Box::new(ScriptedHarness::new(vec![
        Session::Active,
        Session::Thinking,
        Session::RunningCommand,
        Session::Completed,
    ]));
    let (terminal, exits) = fake_terminal(
        "term_t1",
        vec![PtyRead::Chunk(b"noise on the pty\n".to_vec())],
    );
    let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel();

    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx, noop_store());
    let (_id, terminal_status) = handle.join.await.expect("actor task joins");

    assert_eq!(
        terminal_status,
        Session::Completed,
        "the actor is reaped at the terminal §5.1 state"
    );

    // the recorded sequence is EXACTLY the adapter-driven §5.1 path — nothing injected by the PTY.
    let mut seq = Vec::new();
    while let Ok(s) = status_rx.try_recv() {
        seq.push(s);
    }
    assert_eq!(
        seq,
        vec![
            Session::Creating,
            Session::Starting,
            Session::Active,
            Session::Thinking,
            Session::RunningCommand,
            Session::Completed,
        ],
        "status derives from the adapter stream (§5.1), never PTY-scraped (#9)"
    );

    // the pump ran to EOF (the exit event was collected, NOT persisted — cat-1 boundary).
    assert_eq!(
        exits.lock().unwrap().len(),
        1,
        "the terminal pump observed one OS exit"
    );
}

#[tokio::test]
async fn test_adapter_drive_object_safe() {
    // spec(§9.1) — Box<dyn HarnessAdapter> drives a session end-to-end via the spawn_blocking
    // mechanism (LESSON 23/25: sync trait, no async-trait dep). FakeHarness (unchanged; never
    // terminal on its own — stream_status()→Active) is driven to a terminal state by a Kill command,
    // proving the boxed trait + the mailbox + the drive loop compose.
    let adapter: Box<dyn HarnessAdapter> = Box::new(FakeHarness::new(full_caps()));
    let (terminal, _exits) = fake_terminal("term_t4", vec![]);
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();

    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx, noop_store());
    handle
        .commands
        .send(SessionCommand::Kill)
        .await
        .expect("route a Kill to the actor mailbox");
    let (_id, terminal_status) = handle.join.await.expect("actor task joins");

    assert_eq!(
        terminal_status,
        Session::Killed,
        "the boxed adapter drove to a terminal §5.1 state via the Kill command"
    );
}

// ---- W1-exec (094): the actor handles SendMessage/Pause/Resume ----------------------------------

/// Build a `TerminalSession` over an empty-read `FakePty`, returning the recorded-input handle (and a
/// failing-writer variant for the LESSON-30 soft-degrade test). The actor grabs a cross-thread writer
/// off the terminal BEFORE the pump takes it; writes land in this `input` sink.
fn fake_terminal_with_input(id: &str, fail_writes: bool) -> (TerminalSession, Arc<Mutex<Vec<u8>>>) {
    let mut pty = FakePty::new(
        vec![],
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        },
    );
    if fail_writes {
        pty = pty.fail_writes();
    }
    let input = pty.input_sink();
    let terminal = TerminalSession::new(
        TerminalId::from_raw(id),
        Box::new(pty),
        Box::new(CollectingTerminalSink {
            exits: Arc::new(Mutex::new(Vec::new())),
        }),
    );
    (terminal, input)
}

#[tokio::test]
async fn session_actor_handles_new_commands() {
    // spec(§9.1) — the command loop PROCESSES SendMessage/Pause/Resume (not a silent drop): SendMessage
    // writes the text + submit terminator to the session PTY; Pause/Resume are handled (the actor survives
    // + keeps driving); Kill still terminates it. The mailbox preserves order, so by the time the actor
    // breaks on Kill the SendMessage write has landed.
    let adapter: Box<dyn HarnessAdapter> = Box::new(FakeHarness::new(full_caps()));
    let (terminal, input) = fake_terminal_with_input("term_w1exec", false);
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx, noop_store());

    handle
        .commands
        .send(SessionCommand::SendMessage("hello".to_string()))
        .await
        .expect("route SendMessage");
    handle
        .commands
        .send(SessionCommand::Pause)
        .await
        .expect("route Pause");
    handle
        .commands
        .send(SessionCommand::Resume)
        .await
        .expect("route Resume");
    handle
        .commands
        .send(SessionCommand::Kill)
        .await
        .expect("route Kill");

    let (_id, status) = handle.join.await.expect("actor joins");
    assert_eq!(
        status,
        Session::Killed,
        "the actor survived Pause/Resume + terminated on Kill (commands handled, not dropped)"
    );
    assert_eq!(
        input.lock().unwrap().clone(),
        b"hello\r",
        "SendMessage wrote the text + submit terminator to the session PTY"
    );
}

/// A harness that COUNTS `stream_status` polls (the deterministic observable for the pause-gating pin)
/// and never self-terminates (Active forever → the actor lives until Kill). The poll count is the
/// drive signal: while the actor is PAUSED, the status-poll arm is gated → the count must not advance.
struct PollCountingHarness {
    polls: Arc<std::sync::atomic::AtomicUsize>,
}
impl HarnessAdapter for PollCountingHarness {
    fn capabilities(&self) -> HarnessCapabilities {
        full_caps()
    }
    fn launch(&mut self) -> NormalizedStatus {
        Session::Starting
    }
    fn stream_status(&self) -> NormalizedStatus {
        self.polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Session::Active // never terminal → driven only by Kill
    }
    fn intercept_mutation(&self) -> Option<MutationIntercept> {
        None
    }
    fn read_transcript(&self) -> Option<TranscriptRef> {
        None
    }
    fn telemetry_heartbeat(&self) -> Option<TelemetrySample> {
        None
    }
    fn resume(&self) -> ResumeResult {
        ResumeResult {
            mode: ResumeMode::Relaunched,
            replayed_event_count: 0,
        }
    }
}

/// give the single-thread runtime turns to drain whatever is READY (a bounded settle, never a wall-clock
/// sleep). Under PAUSED time the only ready work is what was just enqueued/advanced, so this drains it.
async fn settle() {
    for _ in 0..10_000 {
        tokio::task::yield_now().await;
    }
}

/// yield until the poll counter reaches `target` (bounded; the `drive_until` precedent).
async fn drive_until_polls(polls: &Arc<std::sync::atomic::AtomicUsize>, target: usize) {
    for _ in 0..MAX_DRAIN_YIELDS {
        if polls.load(std::sync::atomic::Ordering::SeqCst) >= target {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("drive_until_polls: never reached {target} within the yield budget");
}

#[tokio::test(start_paused = true)]
async fn session_pause_gates_the_status_drive() {
    // spec(§9.1 — the ADD pin) — Pause GATES the actor's status-drive poll (the flag is READ, not merely
    // set): under PAUSED test-time, advancing N status-poll intervals while PAUSED drives ZERO polls; after
    // Resume, advancing one interval drives a poll again. Deterministic via the test clock + a poll-counter
    // (the `test_save_tick_periodic_checkpoint` precedent — no wall-clock flake). A broken gate is caught by
    // the FIRST processed tick incrementing the counter; a correct gate keeps it flat regardless of drain depth.
    use std::sync::atomic::Ordering;
    let polls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let adapter: Box<dyn HarnessAdapter> = Box::new(PollCountingHarness {
        polls: polls.clone(),
    });
    let (terminal, _input) = fake_terminal_with_input("term_pause_gate", false);
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx, noop_store());

    // launch + the interval's IMMEDIATE (t=0) status-poll tick → the baseline poll.
    drive_until_polls(&polls, 1).await;
    let baseline = polls.load(Ordering::SeqCst);

    // PAUSE — processed deterministically: under paused time NO ticker is ready (frozen clock), so the
    // ONLY ready work after the send is the Pause command → the recv arm runs it (sets paused).
    handle
        .commands
        .send(SessionCommand::Pause)
        .await
        .expect("route Pause");
    settle().await;

    // advance N status-poll intervals while PAUSED → N ticks fire, but the gated poll arm is skipped.
    const N: u32 = 5;
    for _ in 0..N {
        tokio::time::advance(STATUS_POLL_INTERVAL).await;
    }
    settle().await;
    assert_eq!(
        polls.load(Ordering::SeqCst),
        baseline,
        "while PAUSED, advancing {N} status-poll intervals drove ZERO polls (the pause flag GATES the drive)"
    );

    // RESUME → the gate re-opens → the actor drives status polls again. (The "the N paused intervals
    // leaked NONE" claim is already proven deterministically by the `== baseline` assertion above; this
    // phase only needs to prove the gate re-opened — `> baseline`, awaited via drive_until, is robust
    // against the `Delay` catch-up tick + advance interplay that an exact-count assertion would race on.)
    handle
        .commands
        .send(SessionCommand::Resume)
        .await
        .expect("route Resume");
    settle().await; // process Resume (un-gate); the now-un-gated ticker fires the pending tick → a poll
    drive_until_polls(&polls, baseline + 1).await;
    assert!(
        polls.load(Ordering::SeqCst) > baseline,
        "after RESUME the actor drives status polls again (the gate re-opened)"
    );

    handle
        .commands
        .send(SessionCommand::Kill)
        .await
        .expect("route Kill");
    handle.join.await.expect("actor joins");
}

#[tokio::test]
async fn session_send_message_pty_write_error_degrades_soft() {
    // spec(§15/§9.1 + LESSON 30) — a SendMessage PTY-write ERROR does NOT fail/kill the session (the
    // safety invariant is independent of the feed landing; the agent's tool calls are still intercepted).
    // The session still terminates CLEANLY on the explicit Kill (Killed, not Failed) despite the write error.
    let adapter: Box<dyn HarnessAdapter> = Box::new(FakeHarness::new(full_caps()));
    let (terminal, _input) = fake_terminal_with_input("term_w1exec_err", true); // writer errors
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx, noop_store());

    handle
        .commands
        .send(SessionCommand::SendMessage("hello".to_string()))
        .await
        .expect("route SendMessage (write will error)");
    handle
        .commands
        .send(SessionCommand::Kill)
        .await
        .expect("route Kill");

    let (_id, status) = handle.join.await.expect("actor joins");
    assert_eq!(
        status,
        Session::Killed,
        "a SendMessage write error degraded SOFT — the session was not failed by the write, and Kill terminates cleanly"
    );
}

// ---- L2: the SessionLauncher seam (test 5) ------------------------------------------------------

/// Drive a `LaunchedSession` to a terminal state via a `Kill` and assert it reaped — the proof that
/// the seam produced a *drivable* session (the actor runs it end-to-end).
async fn drive_to_kill(launched: nexusopsd::session::LaunchedSession) -> Session {
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = spawn_session_actor(
        launched.session_id,
        launched.adapter,
        launched.terminal,
        status_tx,
        noop_store(),
    );
    handle
        .commands
        .send(SessionCommand::Kill)
        .await
        .expect("route a Kill");
    let (_id, status) = handle.join.await.expect("actor task joins");
    status
}

#[tokio::test]
async fn test_launcher_seam_fake_and_pty() {
    // spec(deep-dive §8 seam) — the SessionLauncher seam produces a drivable LaunchedSession bundle
    // (adapter + terminal + session id); the 4.1 B2-strict survival broker is a drop-in impl behind
    // the same seam (a TODO(4.1) marker, NOT built here).

    // FakeLauncher → a FakeHarness+FakePty session the actor drives (Kill → terminal).
    let fake = FakeLauncher::new(full_caps());
    let launched = fake.launch_session().expect("fake launch");
    assert_eq!(
        drive_to_kill(launched).await,
        Session::Killed,
        "the FakeLauncher seam produced a drivable session"
    );

    // (The `PtyLauncher` spawn-seam smoke MOVED to `tests/session_live.rs`: P4.0b-2 Option A makes the
    // launcher spawn the live `claude` via the O-13 #10 spec, so it's pinned there over a FAKE spawner
    // — never a real `claude` in CI, never an un-intercepted live agent in this seam test.)
}

// ---- L3: the SessionSupervisor (tests 2, 3, 6, 7) -----------------------------------------------

/// A `LaunchedSession` over a `ScriptedHarness` (reaches a terminal §5.1 state on its own) — for the
/// supervisor reap test (built directly; the FakeLauncher's FakeHarness never terminates on its own).
fn scripted_launched_session(post_launch: Vec<Session>) -> LaunchedSession {
    let session_id = SessionId::new();
    let adapter = Box::new(ScriptedHarness::new(post_launch));
    let (terminal, _exits) = fake_terminal("term_scripted", vec![]);
    LaunchedSession {
        session_id,
        adapter,
        terminal,
    }
}

#[tokio::test]
async fn test_supervisor_spawns_tracks_routes() {
    // spec(§10) + LESSON 9 — the supervisor spawns + tracks N actors by session id and routes a
    // command to ONE addressed actor; the others are untouched.
    let mut sup = SessionSupervisor::new();
    let launcher = FakeLauncher::new(full_caps());
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut ids = Vec::new();
    for _ in 0..3 {
        let launched = launcher.launch_session().expect("fake launch");
        ids.push(sup.spawn_session(launched, status_tx.clone(), noop_store()));
    }
    assert_eq!(sup.live_count(), 3, "3 actors tracked by session id");

    // route Kill to the MIDDLE session only (the others are FakeHarness → Active forever).
    let target = ids[1].clone();
    assert!(
        sup.route(&target, SessionCommand::Kill).await,
        "the addressed actor received the routed command"
    );

    // only the addressed actor terminates → reap exactly it; the other two stay live + tracked.
    let (reaped_id, status) = sup.reap_next().await.expect("the Killed actor reaps");
    assert_eq!(
        reaped_id, target,
        "the routed command reached ONLY the addressed actor"
    );
    assert_eq!(status, Session::Killed);
    assert_eq!(
        sup.live_count(),
        2,
        "the other two actors are untouched + still tracked"
    );
}

#[tokio::test]
async fn test_supervisor_reaps_terminal_no_restart() {
    // spec(deep-dive §8) — an actor reaching a terminal §5.1 state is reaped (handle joined, mailbox
    // dropped, count decremented); NO auto-restart (restart-on-crash is the 4.2 concern).
    let mut sup = SessionSupervisor::new();
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    let launched = scripted_launched_session(vec![Session::Active, Session::Completed]);
    let id = sup.spawn_session(launched, status_tx, noop_store());
    assert_eq!(sup.live_count(), 1);

    let (reaped_id, status) = sup.reap_next().await.expect("the terminal actor reaps");
    assert_eq!(reaped_id, id);
    assert_eq!(
        status,
        Session::Completed,
        "reaped at the terminal §5.1 state"
    );
    assert_eq!(sup.live_count(), 0, "handle joined + mailbox dropped");

    // NO auto-restart: nothing respawned — the supervisor is empty + idle.
    assert!(
        sup.try_reap().is_empty(),
        "no new actor was spawned (no auto-restart; restart is 4.2)"
    );
}

#[test]
fn test_cat1_boundary_no_emission_no_agent() {
    // the cat-1 boundary (deep-dive §8) — 4.0a emits NO events + performs NO mutation. Enforced
    // STRUCTURALLY: the session module imports NO mutation/persistence surface (the write-actor, the
    // event store, the Gateway), so an append/emit is compile-time impossible. A forbidden-token grep
    // over src/session/ (the terminal-module "no status-derivation API" precedent). The tokens are
    // module PATHS (`crate::runtime`/`crate::eventstore`/`crate::gateway`), NOT bare words — the cat-1
    // doc comments legitimately NAME WriteHandle/the Gateway/the write-actor in prose, so matching
    // those bare words would false-positive on the very comments documenting the boundary.
    // Scope (honest): the match is `str::contains` substring-exact on rustfmt'd source (no `crate ::
    // runtime` aliasing), and `read_dir` is NON-recursive — a future `src/session/<sub>/mod.rs`
    // submodule must extend this scan to stay a complete cat-1 proof.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/session");
    let forbidden = ["crate::runtime", "crate::eventstore", "crate::gateway"];
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).expect("src/session/ present") {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let src = std::fs::read_to_string(&path).unwrap();
            for tok in forbidden {
                assert!(
                    !src.contains(tok),
                    "{path:?} must not import `{tok}` — 4.0a emits no events + does no mutation \
                     (cat-1 boundary; the live launch + interception + Gateway session.create \
                     executor are 4.0b)"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 3, "session/{{mod,actor,launcher}}.rs scanned");
}

#[tokio::test]
async fn test_supervisor_clean_shutdown() {
    // spec(LESSON 9) — on shutdown the supervisor stops every actor (Kill) + awaits every handle
    // (JoinSet drain): no orphan task, no panic. A hang here = an un-awaited (orphaned) actor.
    let mut sup = SessionSupervisor::new();
    let launcher = FakeLauncher::new(full_caps());
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    for _ in 0..3 {
        let launched = launcher.launch_session().expect("fake launch");
        sup.spawn_session(launched, status_tx.clone(), noop_store());
    }
    assert_eq!(sup.live_count(), 3);

    let drained = sup.shutdown().await;
    assert_eq!(
        drained, 3,
        "every actor was Kill'd + its handle awaited (no orphan task)"
    );
}

// ---- L3: the live-agent kill-path bounds shutdown (test) ----------------------------------------

#[tokio::test]
async fn test_kill_path_unblocks_pump() {
    // spec(§17 / P4.0b-2 L3) — a LIVE long-running agent's PTY read-pump BLOCKS forever (the looping
    // FakePty's read never EOFs on its own). The `spawn_blocking` pump can't be `abort()`ed, so on
    // Kill the actor must `pty.kill()` (the extracted `PtyKiller`) to BREAK the blocked read → the
    // pump ends → `pump.await` completes → the actor returns. WITHOUT the kill-path the actor hangs on
    // `pump.await` (this test would then TIME OUT). `FakeHarness` never reaches a terminal §5.1 state
    // → only the `Kill` ends the actor, exercising the kill-path.
    let adapter = Box::new(FakeHarness::new(full_caps()));
    let exits = Arc::new(Mutex::new(Vec::new()));
    let terminal = TerminalSession::new(
        TerminalId::from_raw("term_kill"),
        Box::new(FakePty::looping()),
        Box::new(CollectingTerminalSink {
            exits: exits.clone(),
        }),
    );
    let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx, noop_store());

    // Kill — the actor breaks its drive loop and must kill the PTY to unblock the looping pump.
    handle
        .commands
        .send(SessionCommand::Kill)
        .await
        .expect("send Kill");

    // the actor MUST terminate promptly (the kill-path unblocked the pump). A bounded wait proves it
    // does NOT hang — without the kill-path the looping read never returns and this times out.
    let (_id, status) = tokio::time::timeout(std::time::Duration::from_secs(5), handle.join)
        .await
        .expect("the kill-path unblocks the pump — the actor must NOT hang on pump.await")
        .expect("actor task joins");
    assert_eq!(status, Session::Killed, "Kill → terminal Killed");
}

// ---- 075c — the producer tap (SessionActor read-pump → per-session HeadlessVt → ScrollbackStore) -

/// 075c Test 5 — the producer tap: PTY output driven through the actor's read-pump is folded into the
/// per-session headless VT, and the saved survival snapshot reconstructs to a screen reflecting it. A
/// `ScriptedHarness` (terminal `Completed`) self-terminates the actor AFTER the fast synchronous pump
/// has drained (the deterministic `test_status_from_adapter_stream_not_pty` pattern); the FINAL save —
/// after `pump.await` — then snapshots the fully-folded VT.
#[tokio::test]
async fn test_producer_tap_feeds_vt() {
    let adapter: Box<dyn HarnessAdapter> = Box::new(ScriptedHarness::new(vec![
        Session::Active,
        Session::Completed,
    ]));
    let (terminal, _exits) = fake_terminal(
        "term_vt",
        vec![PtyRead::Chunk(b"hello scrollback\n".to_vec())],
    );
    let sid = SessionId::new();
    let fake = FakeScrollbackStore::new();
    let store: Arc<dyn ScrollbackStore> = Arc::new(fake.clone());
    let (status_tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    let handle = spawn_session_actor(sid.clone(), adapter, terminal, status_tx, store);
    handle
        .join
        .await
        .expect("actor joins at the terminal status");

    let snap = fake
        .load(&sid)
        .expect("the producer saved a survival snapshot");
    let restored = HeadlessVt::from_snapshot(&snap);
    assert!(
        restored.screen_contents().contains("hello scrollback"),
        "the saved snapshot's screen reflects the PTY output fed through the tap: {:?}",
        restored.screen_contents()
    );
}

/// 075c Test 6 — the save wiring persists a snapshot for the session: even an EMPTY session (no PTY
/// output) ends with a survival snapshot in the store. (Both triggers — the periodic `save_tick` and
/// the reap save — upsert by session id, so the store holds ONE snapshot for the session; this pins
/// that the save wiring fires, not which trigger. Test 5 pins the reap save's CONTENT capture
/// specifically — it's the final, post-`pump.await` save that reflects the fully-drained VT.)
#[tokio::test]
async fn test_producer_persists_snapshot_for_session() {
    let adapter: Box<dyn HarnessAdapter> = Box::new(ScriptedHarness::new(vec![Session::Completed]));
    let (terminal, _exits) = fake_terminal("term_vt2", vec![]);
    let sid = SessionId::new();
    let fake = FakeScrollbackStore::new();
    let store: Arc<dyn ScrollbackStore> = Arc::new(fake.clone());
    let (status_tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    assert_eq!(
        fake.saved_count(),
        0,
        "nothing saved before the session runs"
    );
    let handle = spawn_session_actor(sid.clone(), adapter, terminal, status_tx, store);
    handle.join.await.expect("actor joins");
    assert_eq!(
        fake.saved_count(),
        1,
        "the save wiring persisted one snapshot for the session (upserted by session id)"
    );
    assert!(fake.load(&sid).is_some(), "saved under the session id");
}

/// 075c Test 7 — cat-1: the producer holds the `ScrollbackStore` TRAIT (from `terminal/`, the §35
/// opaque-sink pattern), NEVER a `WriteHandle`. `src/session/actor.rs` references `ScrollbackStore`
/// but imports none of `crate::runtime`/`crate::eventstore`/`crate::gateway` — and `WriteHandle` lives
/// in `crate::runtime`, so the absence of that PATH structurally proves the producer holds no
/// `WriteHandle`. Complements the whole-`src/session/`-dir grep in `test_cat1_boundary_no_emission_no_agent`.
#[test]
fn test_producer_holds_scrollback_store_not_writehandle() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/session/actor.rs"))
        .expect("src/session/actor.rs present");
    assert!(
        src.contains("ScrollbackStore"),
        "the producer holds the ScrollbackStore trait (the survival save seam)"
    );
    for tok in ["crate::runtime", "crate::eventstore", "crate::gateway"] {
        assert!(
            !src.contains(tok),
            "actor.rs must not import `{tok}` — the producer holds the store TRAIT, never a WriteHandle (cat-1)"
        );
    }
}

// ---- 075e — the periodic scrollback save_tick cadence (075c LOW-2; §17 crash-survival checkpoint) -

/// The drain ceiling — a wiring-regression backstop, never reached on a healthy run (the ready save
/// tick is selected within a handful of loop turns). Large enough that the per-advance status-ticker
/// burst can never exhaust it before the save tick is picked.
const MAX_DRAIN_YIELDS: usize = 1_000_000;

/// Yield to the runtime until `pred` holds, BOUNDED (never an unbounded hang). The deterministic drain
/// for the paused-time cadence test: the actor's `select!` loop shares a high-frequency status ticker
/// with the save tick, so a save tick that is READY may take several loop turns to be selected — we
/// yield until it lands rather than guessing a fixed yield count. The cap only ever trips on a genuine
/// wiring regression (the predicate never becoming true), never on timing.
async fn drive_until(fake: &FakeScrollbackStore, pred: impl Fn(&FakeScrollbackStore) -> bool) {
    for _ in 0..MAX_DRAIN_YIELDS {
        if pred(fake) {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("drive_until: the predicate was never satisfied within the yield budget");
}

/// 075e Test 8 — the periodic `save_tick` cadence (075c LOW-2). Under PAUSED time, advancing the clock
/// by N × `SCROLLBACK_SAVE_INTERVAL` drives EXACTLY N periodic survival checkpoints into the store —
/// the §17 crash-survival cadence, pinned deterministically with NO wall-clock sleep. A never-terminating
/// `FakeHarness` keeps the actor's `select!` loop live (Active forever); the empty PTY's pump EOFs
/// promptly (the cadence is what's pinned, not a live byte feed). The N PERIODIC saves are isolated from
/// the interval's immediate t=0 tick (captured in `baseline`) and the final reap save (after `join`, not
/// counted) by measuring the delta across the advances. spec(§17 crash-survival checkpoint / LESSON §41).
#[tokio::test(start_paused = true)]
async fn test_save_tick_periodic_checkpoint() {
    let adapter: Box<dyn HarnessAdapter> = Box::new(FakeHarness::new(full_caps()));
    let (terminal, _exits) = fake_terminal("term_savetick", vec![]);
    let sid = SessionId::new();
    let fake = FakeScrollbackStore::new();
    let store: Arc<dyn ScrollbackStore> = Arc::new(fake.clone());
    let (status_tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    let handle = spawn_session_actor(sid.clone(), adapter, terminal, status_tx, store);

    // let the actor launch + process the interval's IMMEDIATE (t=0) tick — that first save is the
    // baseline, NOT a periodic checkpoint. Before any `advance`, paused time can't reach the next
    // 5s deadline, so EXACTLY one save lands — pin it, so the periodic delta below can't pass for the
    // wrong reason (a stray extra t=0 save would otherwise be silently absorbed into `baseline`).
    drive_until(&fake, |f| f.save_calls() >= 1).await;
    let baseline = fake.save_calls();
    assert_eq!(
        baseline, 1,
        "exactly one immediate t=0 save before any interval advance"
    );

    const N: usize = 3;
    for _ in 0..N {
        let before = fake.save_calls();
        tokio::time::advance(SCROLLBACK_SAVE_INTERVAL).await;
        // drain until THIS interval's periodic save lands (bounded; the status ticker only adds work).
        drive_until(&fake, |f| f.save_calls() > before).await;
    }
    assert_eq!(
        fake.save_calls() - baseline,
        N,
        "N advanced save-intervals → exactly N periodic survival checkpoints (075c LOW-2 cadence)"
    );

    // stop the actor; the FINAL reap save (post-join) is intentionally NOT part of the periodic count.
    handle
        .commands
        .send(SessionCommand::Kill)
        .await
        .expect("route a Kill");
    handle.join.await.expect("actor task joins");
}

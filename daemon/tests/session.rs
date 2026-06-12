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
use nexusopsd::harness::{FakeHarness, HarnessAdapter, MutationIntercept, ResumeResult};
use nexusopsd::session::{
    spawn_session_actor, FakeLauncher, PtyLauncher, SessionCommand, SessionLauncher,
};
use nexusopsd::terminal::{
    ExitStatus, FakePty, PortablePtySpawner, PtyRead, TerminalEventSink, TerminalId,
    TerminalSession,
};

// ---- test doubles -------------------------------------------------------------------------------

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
        ResumeResult {
            resumed_live: true,
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

    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx);
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

    let handle = spawn_session_actor(SessionId::new(), adapter, terminal, status_tx);
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

    // PtyLauncher (daemon-owned PTY) constructs + produces a LaunchedSession over a REAL PTY running a
    // BENIGN program (`/bin/echo`, NEVER a real claude/codex — a live un-intercepted agent is the
    // INV-SEC-1 gap the cat-1 4.0b closes). Smoke: it launches + drives to a terminal state.
    let pty_launcher = PtyLauncher::new(
        Box::new(PortablePtySpawner),
        "/bin/echo",
        vec!["ready".to_string()],
        std::env::temp_dir(),
        full_caps(),
    );
    let launched = pty_launcher
        .launch_session()
        .expect("daemon-owned-PTY launch (benign /bin/echo)");
    assert_eq!(
        drive_to_kill(launched).await,
        Session::Killed,
        "the PtyLauncher seam produced a drivable real-PTY session (benign program)"
    );
}

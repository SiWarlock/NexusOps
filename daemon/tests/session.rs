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
    spawn_session_actor, FakeLauncher, LaunchedSession, SessionCommand, SessionLauncher,
    SessionSupervisor,
};
use nexusopsd::terminal::{
    ExitStatus, FakePty, PtyRead, TerminalEventSink, TerminalId, TerminalSession,
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
        ids.push(sup.spawn_session(launched, status_tx.clone()));
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
    let id = sup.spawn_session(launched, status_tx);
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
        sup.spawn_session(launched, status_tx.clone());
    }
    assert_eq!(sup.live_count(), 3);

    let drained = sup.shutdown().await;
    assert_eq!(
        drained, 3,
        "every actor was Kill'd + its handle awaited (no orphan task)"
    );
}

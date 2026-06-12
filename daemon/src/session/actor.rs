//! The `SessionActor` (P4.0a, opt-3) — one supervised agent session as **a Tokio task + an mpsc
//! command mailbox + the §5.1 `Session` status state**, owning a `HarnessAdapter` and a terminal
//! read-pump. The daemon's own write-actor idiom (LESSON §9: *a task + a mailbox + a command loop*)
//! applied to sessions.
//!
//! **Q1 (drive mechanism, 4.0a) = the sync `HarnessAdapter` driven from the async actor via
//! `spawn_blocking`** — no `async-trait` dep, the trait is UNCHANGED (LESSON §23 "no speculative
//! async-trait dep"; LESSON §25). The blocking calls (`launch` spawns a process; the terminal
//! `read_step` blocks on PTY I/O) run on `spawn_blocking`; the cheap poll reads (`stream_status`,
//! `&self` field reads of the adapter's structured-stream state) run inline on the actor's tick.
//!
//! **Safety #9** — status derives from `adapter.stream_status()` (the structured stream), NEVER from
//! the PTY bytes (the read-pump is display-only).
//! **Cat-1 boundary (4.0a)** — this module takes no `WriteHandle` and no live-interception hook:
//! emission + mutation are compile-time impossible. The live launch + interception + the Gateway
//! `session.create` executor are 4.0b.

use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use nexusops_shared::ids::SessionId;
use nexusops_shared::status::Session;

use crate::harness::HarnessAdapter;
use crate::terminal::TerminalSession;

/// The §5.1-status poll cadence (4.0a scaffold). The sync trait is poll-based; 4.0b replaces the poll
/// with push-based hook/transcript-stream ingestion feeding the adapter.
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// The per-actor command mailbox depth (control messages: small + bursty).
const COMMAND_MAILBOX_CAPACITY: usize = 16;

/// A control message to a [`SessionActor`] via its mailbox. 4.0a carries only `Kill` (the route/reap
/// observable); pause/resume (the inbound client `{pause}`/`{resume}`) join at 6.3d.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionCommand {
    /// Abort the session → a terminal `Killed` status; the supervisor reaps the actor.
    Kill,
}

/// A handle to a spawned [`SessionActor`]: its `JoinHandle` (yields `(SessionId, terminal Session)`
/// when the actor reaches a terminal §5.1 state — the supervisor reaps it) + its command mailbox.
pub struct SessionActorHandle {
    pub join: JoinHandle<(SessionId, Session)>,
    pub commands: mpsc::Sender<SessionCommand>,
}

/// Spawn a [`SessionActor`] driving `adapter` + `terminal` for `session_id`. Each §5.1 status
/// transition is published to `status_tx` (an in-memory observer — the future projection feed; NOT
/// the write-actor, cat-1 boundary). Returns the join handle + the command mailbox.
pub fn spawn_session_actor(
    session_id: SessionId,
    adapter: Box<dyn HarnessAdapter>,
    terminal: TerminalSession,
    status_tx: mpsc::UnboundedSender<Session>,
) -> SessionActorHandle {
    let (commands_tx, commands_rx) = mpsc::channel(COMMAND_MAILBOX_CAPACITY);
    let join = tokio::spawn(run(session_id, adapter, terminal, status_tx, commands_rx));
    SessionActorHandle {
        join,
        commands: commands_tx,
    }
}

/// The actor's drive loop. Launches the adapter (blocking → `spawn_blocking`), starts the terminal
/// read-pump (blocking), then `select!`s the command mailbox against a §5.1 status-poll tick until a
/// terminal status or a `Kill`. Returns the terminal status for the supervisor's reap.
async fn run(
    session_id: SessionId,
    adapter: Box<dyn HarnessAdapter>,
    terminal: TerminalSession,
    status_tx: mpsc::UnboundedSender<Session>,
    mut commands: mpsc::Receiver<SessionCommand>,
) -> (SessionId, Session) {
    let mut current = Session::Creating;
    let _ = status_tx.send(current);

    // launch is blocking (the real launch forks a process); move the boxed adapter through
    // spawn_blocking and back (`Box<dyn HarnessAdapter>` is `Send`).
    let (adapter, launched) = tokio::task::spawn_blocking(move || {
        let mut adapter = adapter;
        let launched = adapter.launch();
        (adapter, launched)
    })
    .await
    .expect("the launch task does not panic");
    if launched != current {
        current = launched;
        let _ = status_tx.send(current);
        if current.is_terminal() {
            return (session_id, current);
        }
    }

    // the terminal read-pump on a blocking thread (`read_step` blocks on the PTY read; LESSON §9).
    // 4.0a DROPS the display output frames (no client; the UDS forward is 6.3d) and lets the pump's
    // own injected sink record the OS exit. Held to abort/await on actor exit → no orphan task.
    let pump = tokio::task::spawn_blocking(move || {
        let mut terminal = terminal;
        while !terminal.is_exited() {
            let _emits = terminal.pump();
        }
    });

    let mut ticker = tokio::time::interval(STATUS_POLL_INTERVAL);
    let mut mailbox_open = true;
    loop {
        tokio::select! {
            cmd = commands.recv(), if mailbox_open => match cmd {
                Some(SessionCommand::Kill) => {
                    current = Session::Killed;
                    let _ = status_tx.send(current);
                    break;
                }
                // every command sender dropped → stop polling the mailbox, keep driving the lifecycle.
                None => mailbox_open = false,
            },
            _ = ticker.tick() => {
                // stream_status is a cheap `&self` read of the adapter's structured-stream state
                // (safety #9 — never PTY-scraped); poll inline, no spawn_blocking.
                let next = adapter.stream_status();
                if next != current {
                    current = next;
                    let _ = status_tx.send(current);
                    if current.is_terminal() {
                        break;
                    }
                }
            }
        }
    }

    // stop the pump (a Kill before EOF aborts it; an already-finished pump is a benign no-op) — the
    // LESSON §9 await-on-shutdown discipline: no orphan blocking task.
    pump.abort();
    let _ = pump.await;
    (session_id, current)
}

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

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use nexusops_shared::ids::SessionId;
use nexusops_shared::status::Session;

use super::launcher::{COLS, ROWS};
use crate::harness::HarnessAdapter;
use crate::terminal::{
    HeadlessVt, ScrollbackStore, TerminalEmit, TerminalSession, DEFAULT_SCROLLBACK_CAPACITY,
};

/// The §5.1-status poll cadence (4.0a scaffold). The sync trait is poll-based; 4.0b replaces the poll
/// with push-based hook/transcript-stream ingestion feeding the adapter.
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// The telemetry pump cadence (4.0c) — the statusLine `refreshInterval` (§9.1/§11.4). Each tick calls
/// `adapter.poll_telemetry()` (drain the live usage source → emit a `TelemetrySampled` DELTA via the
/// injected sink). `MissedTickBehavior::Delay` so a slow tick never bursts a backlog (LESSON §9). The
/// pump emits nothing until the live `UsageSource` is wired (P4); the sink-bind + cadence are 4.0c.
const TELEMETRY_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

/// The per-actor command mailbox depth (control messages: small + bursty).
const COMMAND_MAILBOX_CAPACITY: usize = 16;

/// 075c — the scrollback-snapshot save cadence: a periodic checkpoint so a crash leaves a recent
/// survival snapshot (the 4.0c telemetry-pump precedent; `MissedTickBehavior::Delay`). A FINAL save
/// also runs on reap. With the production no-op `ScrollbackStore` both are no-ops until 075d's durable
/// store lands; the `FakeScrollbackStore` tests prove the producer→store→`Replayed` path now.
/// `pub` so the 075e cadence test advances by the canonical interval, not a magic number.
pub const SCROLLBACK_SAVE_INTERVAL: Duration = Duration::from_secs(5);

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
    scrollback_store: Arc<dyn ScrollbackStore>,
) -> SessionActorHandle {
    let (commands_tx, commands_rx) = mpsc::channel(COMMAND_MAILBOX_CAPACITY);
    let join = tokio::spawn(run(
        session_id,
        adapter,
        terminal,
        status_tx,
        commands_rx,
        scrollback_store,
    ));
    SessionActorHandle {
        join,
        commands: commands_tx,
    }
}

/// Snapshot the per-session headless VT and persist it via the injected [`ScrollbackStore`] (075c).
/// Observationally pure on the VT (the snapshot probe restores the live view). The store is a
/// `terminal/` trait object, NEVER a `WriteHandle` — the cat-1 boundary holds (LESSONS §28/§35). With
/// the production no-op store this is a no-op until 075d's durable store lands.
fn save_scrollback(
    store: &Arc<dyn ScrollbackStore>,
    session_id: &SessionId,
    vt: &Arc<Mutex<HeadlessVt>>,
) {
    let snapshot = vt
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .snapshot();
    store.save(session_id, &snapshot);
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
    scrollback_store: Arc<dyn ScrollbackStore>,
) -> (SessionId, Session) {
    let mut current = Session::Creating;
    let _ = status_tx.send(current);

    // launch is blocking (the real launch forks a process); move the boxed adapter through
    // spawn_blocking and back (`Box<dyn HarnessAdapter>` is `Send`).
    let (mut adapter, launched) = tokio::task::spawn_blocking(move || {
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

    // P4.0b-2 L3 — grab the cross-thread PTY killer BEFORE the pump takes ownership of the terminal,
    // so we can break the pump's BLOCKED read on Kill/shutdown (below). A `spawn_blocking` read can't
    // be `abort()`ed; a live long-running agent never EOFs on its own → without this the pump (and the
    // actor's `pump.await`) would hang forever, blocking daemon shutdown.
    let killer = terminal.killer();

    // 075c — the per-session headless VT (the survival snapshot source). The pump thread FEEDS it the
    // decoded display bytes; the actor SNAPSHOTS it (periodic tick + on reap) into the `ScrollbackStore`.
    // DISPLAY-ONLY (#9 — status is NEVER derived from these bytes, only the survival screen/scrollback).
    let vt = Arc::new(Mutex::new(HeadlessVt::new(
        ROWS,
        COLS,
        DEFAULT_SCROLLBACK_CAPACITY,
    )));

    // the terminal read-pump on a blocking thread (`read_step` blocks on the PTY read; LESSON §9).
    // 4.0a DROPS the display output frames (no client; the UDS forward is 6.3d) and lets the pump's
    // own injected sink record the OS exit; 075c additionally TAPS the output into the headless VT.
    // Held to abort/await on actor exit → no orphan task.
    let vt_for_pump = Arc::clone(&vt);
    let pump = tokio::task::spawn_blocking(move || {
        let mut terminal = terminal;
        while !terminal.is_exited() {
            // 075c producer tap, 075e raw-tap — fold the RAW display bytes into the headless VT (the
            // survival model). The `Output` emit carries `raw` alongside the §6.4 base64 wire `frame`, so
            // we feed the VT directly — no per-frame `base64::decode` round-trip, no silent decode-error
            // drop branch. `TerminalSession` stays a pure byte-pipe (it references no VT type; `raw` rides
            // the emit). Status is NEVER derived from these bytes (#9): this is display/survival state only.
            for emit in terminal.pump() {
                if let TerminalEmit::Output { raw, .. } = emit {
                    vt_for_pump
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .process(&raw);
                }
            }
        }
    });

    // Termination contract: the actor exits ONLY on (a) a `Kill` command → `Killed`, or (b) the
    // adapter reporting a terminal §5.1 status → that status. Losing the mailbox (all command senders
    // dropped) does NOT terminate the actor — it keeps driving the lifecycle until the adapter
    // terminates. A NON-self-terminating adapter (e.g. the bare FakeHarness → Active forever) MUST be
    // `Kill`'d to stop; the `SessionSupervisor` holds each actor's mailbox until reap + Kills every
    // actor on shutdown, so a SUPERVISED actor never orphans. (A direct caller that drops all senders
    // without a Kill on a non-terminating adapter would leave the actor polling — a documented misuse.)
    let mut ticker = tokio::time::interval(STATUS_POLL_INTERVAL);
    // the 4.0c telemetry pump tick (statusLine refresh cadence). `MissedTickBehavior::Delay` so a
    // backlog never bursts (LESSON §9); it rides the same `select!` as the status poll + the mailbox,
    // so it NEVER blocks the command mailbox (poll_telemetry is a cheap drain + a fire-and-forget emit).
    let mut telemetry_tick = tokio::time::interval(TELEMETRY_REFRESH_INTERVAL);
    telemetry_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // 075c — the periodic scrollback-snapshot checkpoint (crash-survival; a final save also runs on
    // reap). `MissedTickBehavior::Delay` (LESSON §9); rides the same `select!` so it never blocks the
    // mailbox (snapshot + save are cheap + non-blocking; no-op with the production placeholder store).
    let mut save_tick = tokio::time::interval(SCROLLBACK_SAVE_INTERVAL);
    save_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
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
            _ = telemetry_tick.tick() => {
                // 4.0c — the telemetry pump: drain the live usage source → emit a TelemetrySampled
                // DELTA via the adapter's injected sink (a non-mutation OBSERVATION; the cat-1
                // boundary holds — the sink is opaque, this actor never imports `WriteHandle`). A
                // no-op until the live `UsageSource` is wired (P4). Cheap + non-blocking (inline).
                adapter.poll_telemetry();
            }
            _ = save_tick.tick() => {
                // 075c — periodic survival checkpoint: snapshot the headless VT → ScrollbackStore (a
                // crash leaves a recent snapshot). No-op with the production placeholder store until
                // 075d. Cheap + non-blocking (inline; the snapshot probe is observationally pure).
                save_scrollback(&scrollback_store, &session_id, &vt);
            }
        }
    }

    // P4.0b-2 L3 — break the pump's BLOCKED read so its task can terminate (`spawn_blocking` can't be
    // `abort()`ed). Kill the PTY child: the blocked `read` returns EOF → the pump's loop ends → the
    // await below completes promptly even for a LIVE long-running agent (daemon shutdown stays
    // time-bounded). Idempotent + best-effort (the child may have already exited on a self-terminating
    // run). THEN await the pump's natural termination (LESSON §9 await-on-shutdown: no orphan task).
    killer.kill();
    let _ = pump.await;
    // 075c — a FINAL scrollback save AFTER the pump has fully drained (`pump.await` joined): all PTY
    // output is folded into the VT before this snapshot, so the survival snapshot reflects the
    // session's last screen + scrollback. (No-op with the production placeholder store until 075d.)
    save_scrollback(&scrollback_store, &session_id, &vt);
    (session_id, current)
}

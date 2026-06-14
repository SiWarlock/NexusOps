//! The session-lifecycle spine (P4.0a, opt-3; ARCHITECTURE §5.1 / §10 / §0.1 O-2).
//!
//! An **edge module** (depends on `harness` + `terminal` + `idgen`; **never writes the DB** — the
//! layer rule). The opt-3 shape: a [`SessionActor`](actor::SessionActorHandle) per agent session
//! (a Tokio task + an mpsc mailbox + the §5.1 status state), spawned + supervised by a
//! `SessionSupervisor` (L3), behind a `SessionLauncher` seam (L2; the B2-strict survival broker
//! swaps in at 4.1). This is the foundation the live drive loop (4.0b: launch + INV-SEC-1
//! interception + the Gateway `session.create` executor) builds on.
//!
//! **Cat-1 boundary (4.0a)** — FakeHarness/FakePty-driven; NO live agent, NO event emission, NO
//! mutation. The module takes no `WriteHandle` and no live-interception hook, so emission + mutation
//! are compile-time impossible (the live launch + interception + executor are 4.0b, deep-dive §8).

pub mod actor;
pub mod broker;
pub mod launcher;
pub mod recovery;

pub use actor::{spawn_session_actor, SessionActorHandle, SessionCommand};
#[cfg(any(test, feature = "test-support"))]
pub use launcher::FakeLauncher;
pub use launcher::{LaunchedSession, NullTerminalSink, PtyLauncher, SessionLauncher};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tokio::task::{JoinHandle, JoinSet};

use nexusops_shared::ids::SessionId;
use nexusops_shared::status::Session;

use crate::decisions::DecisionRegistry;

/// The §5.1-status observer an actor publishes each transition to — an **in-memory** feed (the future
/// projection source; NOT the write-actor, cat-1 boundary).
pub type StatusObserver = mpsc::UnboundedSender<Session>;

/// The §10 supervisor reap cadence (the background task harvests terminal actors).
const SUPERVISOR_REAP_INTERVAL: Duration = Duration::from_millis(500);

/// The opt-3 session supervisor: spawns + tracks one [`SessionActor`](actor) per session id, routes
/// commands to a specific actor, and reaps an actor when it reaches a terminal §5.1 state — **NO
/// auto-restart** (restart-on-crash is the 4.2 child-death concern). The daemon's own write-actor /
/// JoinSet idiom (LESSON §9: a task + a mailbox + a command loop; await handles on shutdown) applied
/// to sessions. An EDGE — it never writes the DB (cat-1: no `WriteHandle` in scope).
pub struct SessionSupervisor {
    /// the live actors' mailboxes, keyed by session id (route target + shutdown set).
    actors: HashMap<SessionId, mpsc::Sender<SessionCommand>>,
    /// the live actors' terminal outcomes — reaping harvests `(SessionId, terminal Session)`.
    joinset: JoinSet<(SessionId, Session)>,
}

impl Default for SessionSupervisor {
    fn default() -> Self {
        Self {
            actors: HashMap::new(),
            joinset: JoinSet::new(),
        }
    }
}

impl SessionSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of live (tracked, un-reaped) actors.
    pub fn live_count(&self) -> usize {
        self.actors.len()
    }

    /// Spawn + track a [`SessionActor`] for `launched`; its §5.1 transitions publish to `status_tx`
    /// (the in-memory observer — cat-1: NOT the write-actor). Returns the session id.
    pub fn spawn_session(
        &mut self,
        launched: LaunchedSession,
        status_tx: StatusObserver,
    ) -> SessionId {
        let LaunchedSession {
            session_id,
            adapter,
            terminal,
        } = launched;
        let SessionActorHandle { join, commands } =
            spawn_session_actor(session_id.clone(), adapter, terminal, status_tx);
        self.actors.insert(session_id.clone(), commands);
        // the JoinSet owns the actor's terminal outcome; a reap harvests its (id, terminal status). A
        // panicked / cancelled actor task → a `Failed` terminal outcome CARRYING THE ID, so the reap
        // path ALWAYS harvests it + drops the mailbox — `live_count` stays exact (never inflated by a
        // dead actor whose bare `JoinError` would otherwise carry no id to clean up by).
        let wrapper_id = session_id.clone();
        self.joinset
            .spawn(async move { join.await.unwrap_or((wrapper_id, Session::Failed)) });
        session_id
    }

    /// Route a command to a specific live actor's mailbox. `false` if no such live actor (already
    /// reaped / unknown id).
    pub async fn route(&self, session_id: &SessionId, command: SessionCommand) -> bool {
        match self.actors.get(session_id) {
            Some(mailbox) => mailbox.send(command).await.is_ok(),
            None => false,
        }
    }

    /// Reap every actor that has ALREADY reached a terminal §5.1 state (non-blocking): drop its
    /// mailbox + collect its outcome. **NO restart.** Returns the reaped `(id, status)` pairs.
    pub fn try_reap(&mut self) -> Vec<(SessionId, Session)> {
        let mut reaped = Vec::new();
        while let Some(outcome) = self.joinset.try_join_next() {
            // the wrapper maps a panicked actor to `(id, Failed)`, so a real reap is always `Ok` (the
            // mailbox is always cleaned up); the `Err` arm — a runtime-cancelled wrapper task — is
            // defensive, skip it. NO restart either way (restart-on-crash is the 4.2 concern).
            if let Ok((id, status)) = outcome {
                self.actors.remove(&id);
                reaped.push((id, status));
            }
        }
        reaped
    }

    /// Await the NEXT actor to reach a terminal state, reap it (drop its mailbox), return its outcome.
    /// `None` when no live actors remain. (For callers that want to block on a reap.)
    pub async fn reap_next(&mut self) -> Option<(SessionId, Session)> {
        match self.joinset.join_next().await {
            Some(Ok((id, status))) => {
                self.actors.remove(&id);
                Some((id, status))
            }
            // the wrapper maps a panic to `(id, Failed)` → a real reap is always `Ok`; `Err` (a
            // cancelled wrapper) / an empty set → nothing to reap.
            Some(Err(_)) | None => None,
        }
    }

    /// Clean shutdown (LESSON §9): Kill every live actor + await every handle (drain the JoinSet) —
    /// no orphan task. Returns the number of actors drained.
    pub async fn shutdown(mut self) -> usize {
        for (_id, mailbox) in self.actors.drain() {
            let _ = mailbox.send(SessionCommand::Kill).await;
        }
        let mut drained = 0;
        while self.joinset.join_next().await.is_some() {
            drained += 1;
        }
        drained
    }
}

/// A control message to the supervisor task — the session.create executor (4.0b-1) / the live driver
/// (4.0b-2) send these (spawn a session / route a command). The channel is **UNBOUNDED** so the sync
/// write-actor-thread caller's `send` is a NON-BLOCKING enqueue that can never stall the single
/// mutation path (cat-1 no-stall, P4.0b-1) — the live session count is naturally bounded, so there is
/// no unbounded-growth concern.
enum SupervisorControl {
    Spawn {
        launched: LaunchedSession,
        status_tx: StatusObserver,
    },
    Route {
        session_id: SessionId,
        command: SessionCommand,
    },
}

/// The handle the session.create executor (4.0b-1) drives the supervisor task with (spawn/route).
/// **Synchronous + non-blocking** over the UNBOUNDED control channel — so it is safe to call from the
/// write-actor's dedicated `std::thread` (OFF the tokio runtime, LESSON §9) WITHOUT ever blocking the
/// single mutation path (cat-1 no-stall, P4.0b-1; the supervisor is an edge actor — no callback into
/// the write-actor, so no cycle). The caller already holds the `SessionId` (it is on the
/// `LaunchedSession`), so no reply round-trip is needed.
#[derive(Clone)]
pub struct SupervisorHandle {
    control: mpsc::UnboundedSender<SupervisorControl>,
}

impl SupervisorHandle {
    /// Spawn a session through the supervisor task — a NON-BLOCKING enqueue. Returns the session id
    /// (read off `launched`; the supervisor task picks the Spawn up on the runtime). A send failure
    /// (the supervisor task is gone — shutdown) is benign: the id is still returned (the session is moot).
    pub fn spawn_session(&self, launched: LaunchedSession, status_tx: StatusObserver) -> SessionId {
        let session_id = launched.session_id.clone();
        let _ = self.control.send(SupervisorControl::Spawn {
            launched,
            status_tx,
        });
        session_id
    }

    /// Route a command to a session via the supervisor task — a NON-BLOCKING enqueue. `false` if the
    /// supervisor task is gone.
    pub fn route(&self, session_id: SessionId, command: SessionCommand) -> bool {
        self.control
            .send(SupervisorControl::Route {
                session_id,
                command,
            })
            .is_ok()
    }
}

/// Spawn the §10 supervisor background task (alongside the drainer/reaper/accept-loop in `main.rs`),
/// stopped by the shutdown watch (LESSON §9 await-on-shutdown). Returns its `JoinHandle` + the
/// [`SupervisorHandle`] (the 4.0b session.create driver entry). **C2 (carry-forward a):** when a
/// session reaches a terminal §5.1 state (the reaper harvests it), the supervisor
/// [`cancel_session`](DecisionRegistry::cancel_session)s its pending agent-mutation decisions —
/// DROPPING their senders so a live `PreToolUse` hook awaiting the human resolves to Deny **fast**
/// (never hanging to the 5-min wall-clock). The supervisor stays an EDGE (the registry is pure
/// in-memory coordination — no DB / write-actor / gateway, so the cat-1 import-boundary holds).
pub fn spawn_supervisor_task(
    mut shutdown: watch::Receiver<bool>,
    registry: Arc<DecisionRegistry>,
) -> (JoinHandle<()>, SupervisorHandle) {
    let (control_tx, mut control_rx) = mpsc::unbounded_channel();
    let join = tokio::spawn(async move {
        let mut supervisor = SessionSupervisor::new();
        let mut reaper = tokio::time::interval(SUPERVISOR_REAP_INTERVAL);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.changed() => break,
                ctrl = control_rx.recv() => match ctrl {
                    Some(SupervisorControl::Spawn { launched, status_tx }) => {
                        supervisor.spawn_session(launched, status_tx);
                    }
                    Some(SupervisorControl::Route { session_id, command }) => {
                        let _ = supervisor.route(&session_id, command).await;
                    }
                    // every control sender dropped → no further driver; shut down cleanly.
                    None => break,
                },
                _ = reaper.tick() => {
                    // a reaped (terminal) session → cancel its pending interceptions (fail-closed).
                    for (id, _status) in supervisor.try_reap() {
                        registry.cancel_session(id.as_str());
                    }
                }
            }
        }
        supervisor.shutdown().await;
    });
    (
        join,
        SupervisorHandle {
            control: control_tx,
        },
    )
}

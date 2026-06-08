//! The single write-actor (§4.2 / forbidden #3 / LESSON §3 — Q1 ratified).
//!
//! rusqlite is **synchronous/blocking**, so the one writable [`EventStore`] is owned by a
//! **dedicated OS thread** (never a tokio worker — a blocking SQLite call must not stall the
//! async runtime). Async callers (the drainer/reaper loops, the accept-loop, the future P2
//! gateway) hold a cloneable [`WriteHandle`] and send commands over an mpsc channel, awaiting a
//! oneshot reply. This is THE single mutation path: every mutating op (`append`, `drain_once`,
//! `reap_leases`) routes through this one thread; all reads use `open_read_only` and NEVER touch
//! the actor. The L4 subscribe broadcast sender lives here, publishing **after commit**.

use std::sync::Arc;
use std::thread::JoinHandle;

use tokio::sync::{mpsc, oneshot};

use nexusops_shared::ids::EventId;

use crate::clock::Clock;
use crate::eventstore::{AppendIntent, Destination, DrainSummary, EventStore, EventStoreError};
use crate::locks::{LeaseError, LeaseKind, ResourceId};

/// the bounded command-channel depth — backpressures a flood of mutation requests onto the
/// senders rather than growing unbounded in front of the single writer.
const COMMAND_CHANNEL_DEPTH: usize = 64;

/// Typed write-actor failures. `ActorGone` is the post-shutdown / dropped-actor case (a send or
/// reply-recv failed) — distinct from the underlying store/lease errors so a caller can tell
/// "the writer is gone" from "the write itself failed".
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("event-store error: {0}")]
    Store(#[from] EventStoreError),
    #[error("lease error: {0}")]
    Lease(#[from] LeaseError),
    #[error("the write-actor is no longer running")]
    ActorGone,
}

/// Commands the actor thread executes against the single writable `EventStore`. Each carries a
/// oneshot reply so the async caller gets the typed result back. `Shutdown` breaks the loop
/// immediately (even if handle clones remain), so the store drops + the writer closes cleanly.
enum Command {
    Append {
        // boxed: AppendIntent is large; boxing keeps the Command enum small (clippy
        // large_enum_variant) so the mpsc channel slots stay cheap.
        intent: Box<AppendIntent>,
        reply: oneshot::Sender<Result<EventId, EventStoreError>>,
    },
    DrainOnce {
        dest: Arc<dyn Destination>,
        reply: oneshot::Sender<Result<DrainSummary, EventStoreError>>,
    },
    ReapLeases {
        reply: oneshot::Sender<Result<Vec<(ResourceId, LeaseKind)>, LeaseError>>,
    },
    Shutdown,
}

/// A cloneable handle to the single write-actor. Every clone funnels into the ONE writer thread
/// (forbidden #3): there is no other way to mutate. Async methods send a command + await the
/// oneshot reply; if the actor is gone (shutdown / dropped) they return [`RuntimeError::ActorGone`].
#[derive(Clone)]
pub struct WriteHandle {
    tx: mpsc::Sender<Command>,
}

impl WriteHandle {
    /// Append an event through the single writer (the §15 redaction gate + in-band projections +
    /// outbox all run inside `EventStore::append`).
    pub async fn append(&self, intent: AppendIntent) -> Result<EventId, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::Append {
                intent: Box::new(intent),
                reply,
            })
            .await
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.await
            .map_err(|_| RuntimeError::ActorGone)?
            .map_err(RuntimeError::from)
    }

    /// Drain one outbox pass for `dest` through the single writer (the drainer loop's call).
    pub async fn drain_once(
        &self,
        dest: Arc<dyn Destination>,
    ) -> Result<DrainSummary, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::DrainOnce { dest, reply })
            .await
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.await
            .map_err(|_| RuntimeError::ActorGone)?
            .map_err(RuntimeError::from)
    }

    /// Reap expired leases through the single writer (the reaper loop's call).
    pub async fn reap_leases(&self) -> Result<Vec<(ResourceId, LeaseKind)>, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::ReapLeases { reply })
            .await
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.await
            .map_err(|_| RuntimeError::ActorGone)?
            .map_err(RuntimeError::from)
    }
}

/// Owns the write-actor thread. Hand its [`WriteHandle`] to every async task that must mutate;
/// call [`WriteActor::shutdown`] at exit to stop the loop + close the writer cleanly.
pub struct WriteActor {
    handle: WriteHandle,
    join: Option<JoinHandle<()>>,
}

impl WriteActor {
    /// Spawn the dedicated writer thread owning `store` (+ a `clock` for the timing of
    /// `drain_once`/`reap_leases`). The thread blocks on the command channel; reads never reach it.
    pub fn spawn(store: EventStore, clock: Box<dyn Clock>) -> Self {
        let (tx, rx) = mpsc::channel(COMMAND_CHANNEL_DEPTH);
        let join = std::thread::Builder::new()
            .name("nexusops-write-actor".to_string())
            .spawn(move || run_actor(store, clock, rx))
            // a daemon that cannot spawn its sole writer cannot run — fail loud at startup.
            .expect("spawn the write-actor thread");
        Self {
            handle: WriteHandle { tx },
            join: Some(join),
        }
    }

    /// A fresh handle to the actor (cloneable; share one per async task).
    pub fn handle(&self) -> WriteHandle {
        self.handle.clone()
    }

    /// Stop the actor + close the writer cleanly (§16 drain+exit). FIFO-graceful: commands already
    /// queued ahead of `Shutdown` complete first; commands sent after fail with `ActorGone`. Joins
    /// the thread (off the async runtime) so the `EventStore` is fully dropped — WAL checkpointed —
    /// before this returns.
    pub async fn shutdown(mut self) {
        // best-effort: if the actor is already gone the send fails, and the join still completes.
        let _ = self.handle.tx.send(Command::Shutdown).await;
        if let Some(join) = self.join.take() {
            let _ = tokio::task::spawn_blocking(move || join.join()).await;
        }
    }
}

/// The actor loop: block on the command channel, execute each against the single writable store,
/// reply. `blocking_recv` is correct here — this runs on a dedicated OS thread, NOT a tokio
/// worker. The loop ends on `Shutdown` (or when every sender drops); `store`/`clock` then drop,
/// closing the writer connection (WAL checkpoint on Connection drop).
fn run_actor(mut store: EventStore, clock: Box<dyn Clock>, mut rx: mpsc::Receiver<Command>) {
    while let Some(cmd) = rx.blocking_recv() {
        match cmd {
            Command::Append { intent, reply } => {
                // ignore a dropped receiver — the caller gave up waiting; the write still committed.
                let _ = reply.send(store.append(*intent));
            }
            Command::DrainOnce { dest, reply } => {
                let _ = reply.send(store.drain_once(clock.as_ref(), dest.as_ref()));
            }
            Command::ReapLeases { reply } => {
                let _ = reply.send(store.reap_leases(clock.as_ref()));
            }
            Command::Shutdown => break,
        }
    }
}

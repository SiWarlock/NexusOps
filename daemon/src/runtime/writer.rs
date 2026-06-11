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

use tokio::sync::{broadcast, mpsc, oneshot};

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::ids::EventId;
use nexusops_shared::ipc::{ActionAck, DeltaKind, ProjectionDelta, ProjectionName};

use crate::clock::Clock;
use crate::eventstore::{AppendIntent, Destination, DrainSummary, EventStore, EventStoreError};
use crate::gateway::{Gateway, GatewayError};
use crate::locks::{LeaseError, LeaseKind, ResourceId};

/// the bounded command-channel depth — backpressures a flood of mutation requests onto the
/// senders rather than growing unbounded in front of the single writer.
const COMMAND_CHANNEL_DEPTH: usize = 64;

/// the subscribe broadcast channel capacity. A subscriber that falls > this far behind is dropped
/// (sees `Lagged`) + must resync (re-`get_projection`) — the broadcast NEVER back-pressures the
/// writer (forbidden #3: a reader must not block the writer).
pub const BROADCAST_CAPACITY: usize = 256;

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
    // --- 2.1b Action Gateway mutation commands: the pipeline runs ON the write-actor (the single
    // writer, forbidden #3) so each transition's {row + authoritative event} is one atomic txn. ---
    GatewaySubmit {
        req: Box<ActionRequest>,
        reply: oneshot::Sender<Result<ActionAck, GatewayError>>,
    },
    GatewayApprove {
        approval_id: String,
        reply: oneshot::Sender<Result<ActionAck, GatewayError>>,
    },
    GatewayDeny {
        approval_id: String,
        reason: String,
        reply: oneshot::Sender<Result<ActionAck, GatewayError>>,
    },
    GatewayPreview {
        action_request_id: String,
        reply: oneshot::Sender<Result<ActionPreview, GatewayError>>,
    },
    Shutdown,
}

/// A cloneable handle to the single write-actor. Every clone funnels into the ONE writer thread
/// (forbidden #3): there is no other way to mutate. Async methods send a command + await the
/// oneshot reply; if the actor is gone (shutdown / dropped) they return [`RuntimeError::ActorGone`].
#[derive(Clone)]
pub struct WriteHandle {
    tx: mpsc::Sender<Command>,
    // a clone of the actor's broadcast sender — `subscribe()` mints receivers from it without
    // touching the writer thread (the actor holds the authoritative sender for publishing).
    deltas: broadcast::Sender<ProjectionDelta>,
}

impl WriteHandle {
    /// A handle whose write-actor is NOT running — every mutation/append returns
    /// [`RuntimeError::ActorGone`]. For the post-shutdown edge + tests of the read/handshake path
    /// (which never reach the actor). Production handles come from [`WriteActor::handle`].
    /// `#[doc(hidden)]`: a footgun outside that narrow use — it silently swallows every mutation.
    #[doc(hidden)]
    pub fn disconnected() -> Self {
        let (tx, _rx) = mpsc::channel(1); // _rx dropped → every send fails (ActorGone)
        let (deltas, _) = broadcast::channel(1);
        Self { tx, deltas }
    }

    /// Subscribe to the live `ProjectionDelta` stream (§6.1 subscribe). The returned receiver gets
    /// every delta the writer publishes AFTER it commits; a receiver that lags > `BROADCAST_CAPACITY`
    /// is dropped (`Lagged`) and must resync — it NEVER back-pressures the writer (forbidden #3).
    pub fn subscribe(&self) -> broadcast::Receiver<ProjectionDelta> {
        self.deltas.subscribe()
    }

    /// A clone of the post-commit broadcast SENDER — handed to the GatewayPort accept-loop so each
    /// served connection can mint its own subscriber receiver (`.subscribe()`) per `subscribe`
    /// request, without coupling the `ipc` serve layer to `WriteHandle` (1.6d subscribe-SERVE).
    pub fn delta_sender(&self) -> broadcast::Sender<ProjectionDelta> {
        self.deltas.clone()
    }

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

    // --- 2.1b Action Gateway mutation methods (BLOCKING) -------------------------------------
    // The synchronous `serve_connection` (a `spawn_blocking` task) cannot `.await`, so the IPC
    // mutation dispatch uses `blocking_send`/`blocking_recv`. The outer `RuntimeError` is the infra
    // failure (the write-actor is gone → the connection disconnects); the inner `Result` is the
    // typed gateway outcome (which the IPC layer maps to an `IpcErrorCode` structured response).

    /// `submit_action` through the single writer (the §6.1 mutation path; runs the gateway pipeline
    /// on the write-actor). The inputs blob is §15-redacted at rest inside the pipeline.
    pub fn submit_action_blocking(
        &self,
        req: ActionRequest,
    ) -> Result<Result<ActionAck, GatewayError>, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .blocking_send(Command::GatewaySubmit {
                req: Box::new(req),
                reply,
            })
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.blocking_recv().map_err(|_| RuntimeError::ActorGone)
    }

    /// `approve` an awaiting approval through the single writer.
    pub fn approve_blocking(
        &self,
        approval_id: String,
    ) -> Result<Result<ActionAck, GatewayError>, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .blocking_send(Command::GatewayApprove { approval_id, reply })
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.blocking_recv().map_err(|_| RuntimeError::ActorGone)
    }

    /// `deny` an awaiting approval through the single writer.
    pub fn deny_blocking(
        &self,
        approval_id: String,
        reason: String,
    ) -> Result<Result<ActionAck, GatewayError>, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .blocking_send(Command::GatewayDeny {
                approval_id,
                reason,
                reply,
            })
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.blocking_recv().map_err(|_| RuntimeError::ActorGone)
    }

    /// `preview_action` (read-only dry-run) through the single writer.
    pub fn preview_action_blocking(
        &self,
        action_request_id: String,
    ) -> Result<Result<ActionPreview, GatewayError>, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .blocking_send(Command::GatewayPreview {
                action_request_id,
                reply,
            })
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.blocking_recv().map_err(|_| RuntimeError::ActorGone)
    }
}

/// Owns the write-actor thread. Hand its [`WriteHandle`] to every async task that must mutate;
/// call [`WriteActor::shutdown`] at exit to stop the loop + close the writer cleanly.
pub struct WriteActor {
    handle: WriteHandle,
    join: Option<JoinHandle<()>>,
}

impl WriteActor {
    /// Spawn the dedicated writer thread owning `store` (with a `clock` for `drain_once`/
    /// `reap_leases` and the `gateway` whose mutation pipeline runs ON this single-writer thread).
    /// The thread blocks on the command channel; reads never reach it.
    pub fn spawn(store: EventStore, clock: Box<dyn Clock>, gateway: Gateway) -> Self {
        let (tx, rx) = mpsc::channel(COMMAND_CHANNEL_DEPTH);
        // the broadcast sender lives in the actor thread (it publishes post-commit); the handle
        // keeps a clone so `subscribe()` can mint receivers without touching the writer. The
        // initial receiver is discarded — the channel stays live as long as a Sender exists, and
        // subscribers come from `handle.subscribe()`.
        let (deltas, _) = broadcast::channel(BROADCAST_CAPACITY);
        let actor_deltas = deltas.clone();
        let join = std::thread::Builder::new()
            .name("nexusops-write-actor".to_string())
            .spawn(move || run_actor(store, clock, gateway, rx, actor_deltas))
            // a daemon that cannot spawn its sole writer cannot run — fail loud at startup.
            .expect("spawn the write-actor thread");
        Self {
            handle: WriteHandle { tx, deltas },
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

impl Drop for WriteActor {
    fn drop(&mut self) {
        // best-effort: if `shutdown()` wasn't called (early return / test panic / misuse), nudge
        // the actor to exit promptly rather than linger until every handle clone drops. `try_send`
        // is non-blocking (Drop must not block); a closed/full channel is fine — the JoinHandle
        // then detaches the (already-exiting) thread, and the EventStore still WAL-checkpoints on
        // drop. A no-op when `shutdown()` already took the join + closed the channel.
        if self.join.is_some() {
            let _ = self.handle.tx.try_send(Command::Shutdown);
        }
    }
}

/// The actor loop: block on the command channel, execute each against the single writable store,
/// reply. `blocking_recv` is correct here — this runs on a dedicated OS thread, NOT a tokio
/// worker. The loop ends on `Shutdown` (or when every sender drops); `store`/`clock` then drop,
/// closing the writer connection (WAL checkpoint on Connection drop).
fn run_actor(
    mut store: EventStore,
    clock: Box<dyn Clock>,
    gateway: Gateway,
    mut rx: mpsc::Receiver<Command>,
    deltas: broadcast::Sender<ProjectionDelta>,
) {
    while let Some(cmd) = rx.blocking_recv() {
        match cmd {
            Command::Append { intent, reply } => {
                // derive the publishable deltas from the intent BEFORE it's consumed by append.
                let pending = deltas_for_append(&intent);
                let result = store.append(*intent);
                // PUBLISH-AFTER-COMMIT: only broadcast once the append durably committed. A failed
                // (rolled-back) append publishes nothing. broadcast::send never blocks — a lagging
                // subscriber is dropped, never back-pressures this writer (forbidden #3).
                if result.is_ok() {
                    for delta in pending {
                        let _ = deltas.send(delta); // Err = no subscribers; not an error here.
                    }
                }
                // ignore a dropped receiver — the caller gave up waiting; the write still committed.
                let _ = reply.send(result);
            }
            Command::DrainOnce { dest, reply } => {
                let _ = reply.send(store.drain_once(clock.as_ref(), dest.as_ref()));
            }
            Command::ReapLeases { reply } => {
                let _ = reply.send(store.reap_leases(clock.as_ref()));
            }
            // 2.1b Action Gateway mutations run the pipeline against the single writable store
            // (each transition's row+event is its own atomic txn inside the method). The gateway's
            // events fold into projections in-band; 2.1c (Q6) accumulates the `proj_approval_queue`
            // subscribe-deltas the pipeline touched + PUBLISHES them AFTER the txn commits (an
            // Err/rolled-back op publishes nothing; broadcast::send never back-pressures the writer,
            // forbidden #3) — mirroring the `Command::Append` publish-after-commit above.
            Command::GatewaySubmit { req, reply } => {
                let mut queue_deltas = Vec::new();
                let result = gateway.submit_action_collecting(&mut store, *req, &mut queue_deltas);
                publish_after_commit(&deltas, &result, queue_deltas);
                let _ = reply.send(result);
            }
            Command::GatewayApprove { approval_id, reply } => {
                let mut queue_deltas = Vec::new();
                let result =
                    gateway.approve_collecting(&mut store, &approval_id, &mut queue_deltas);
                publish_after_commit(&deltas, &result, queue_deltas);
                let _ = reply.send(result);
            }
            Command::GatewayDeny {
                approval_id,
                reason,
                reply,
            } => {
                let mut queue_deltas = Vec::new();
                let result =
                    gateway.deny_collecting(&mut store, &approval_id, &reason, &mut queue_deltas);
                publish_after_commit(&deltas, &result, queue_deltas);
                let _ = reply.send(result);
            }
            Command::GatewayPreview {
                action_request_id,
                reply,
            } => {
                let _ = reply.send(gateway.preview_action(&mut store, &action_request_id));
            }
            Command::Shutdown => break,
        }
    }
}

/// Publish the gateway pipeline's accumulated `proj_approval_queue` deltas — but ONLY if the
/// operation committed (`result.is_ok()`); a rolled-back / errored op publishes nothing (Q6 /
/// publish-after-commit). `broadcast::send` never blocks — a lagging subscriber is dropped, never
/// back-pressures this writer (forbidden #3). `Err = no subscribers`, not an error here.
fn publish_after_commit<T>(
    deltas: &broadcast::Sender<ProjectionDelta>,
    result: &Result<T, GatewayError>,
    pending: Vec<ProjectionDelta>,
) {
    if result.is_ok() {
        for delta in pending {
            let _ = deltas.send(delta);
        }
    }
}

/// The live `ProjectionDelta`(s) an appended event produces — derived from its typed identity +
/// `event_type` (mirrors the `object_refs::derive_refs` pattern: rebuildable, decoupled from the
/// projector internals). 1.6b feeds only `SessionStarted` → a Session-projection Upsert; later
/// event types add their mappings additively. The delta carries the id; the subscriber re-reads
/// the full row via `get_projection` (row enrichment is a future improvement, consistent with the
/// lag-resync policy).
fn deltas_for_append(intent: &AppendIntent) -> Vec<ProjectionDelta> {
    let mut out = Vec::new();
    if intent.event_type == "SessionStarted" {
        if let Some(sid) = &intent.session_id {
            out.push(ProjectionDelta {
                projection: ProjectionName::Session,
                kind: DeltaKind::Upsert,
                row: None,
                id: Some(sid.as_str().to_string()),
            });
        }
    }
    out
}

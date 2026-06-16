//! The single write-actor (§4.2 / forbidden #3 / LESSON §3 — Q1 ratified).
//!
//! rusqlite is **synchronous/blocking**, so the one writable [`EventStore`] is owned by a
//! **dedicated OS thread** (never a tokio worker — a blocking SQLite call must not stall the
//! async runtime). Async callers (the drainer/reaper loops, the accept-loop, the future P2
//! gateway) hold a cloneable [`WriteHandle`] and send commands over an mpsc channel, awaiting a
//! oneshot reply. This is THE single mutation path: every mutating op (`append`, `drain_once`,
//! `reap_leases`) routes through this one thread; all reads use `open_read_only` and NEVER touch
//! the actor. The L4 subscribe broadcast sender lives here, publishing **after commit**.

use std::path::Path;
use std::sync::Arc;
use std::thread::JoinHandle;

use tokio::sync::{broadcast, mpsc, oneshot};

use nexusops_shared::actions::{ActionPlan, ActionPreview, ActionRequest};
use nexusops_shared::ids::EventId;
use nexusops_shared::ipc::{ActionAck, DeltaKind, PlanAck, ProjectionDelta, ProjectionName};
use nexusops_shared::status::WorktreeOverlay;

use crate::clock::Clock;
use crate::eventstore::{
    AppendIntent, Destination, DrainSummary, EventStore, EventStoreError, WalCheckpointMode,
    WalCheckpointSummary,
};
use crate::gateway::circuit_breaker::AuditBackboneBreaker;
use crate::gateway::{Gateway, GatewayError};
use crate::git::precedence::{derive_worktree_status, DerivedWorktreeStatus};
use crate::git::reads::read_worktree_status;
use crate::harness::claude::intercept::{
    route_intercept, route_intercept_live, HookPayload, InterceptOutcome,
};
use crate::harness::MutationVerdict;
use crate::integrity::IntegrityAlarm;
use crate::locks::{LeaseError, LeaseKind, ResourceId};
use crate::projections::WorktreeGitCache;

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
    // edges-026 — the §7.2 worktree live-read cache refresh: a NON-Gateway, NON-event maintenance
    // command (the DrainOnce/ReapLeases family) the git-watcher issues. The git2 read + the §5.1
    // status derivation run in the handler (runtime layer); the non-event proj_worktree UPDATE on the
    // single writer. NO event (the git-axis is a live-read cache, §7.1).
    RefreshWorktreeStatus {
        worktree_id: String,
        path: String,
        base: Option<String>,
        reply: oneshot::Sender<Result<usize, EventStoreError>>,
    },
    // 4.3: a bounded WAL checkpoint (§10) — the background checkpointer rides the single write-actor
    // (forbidden #3 / LESSON §9), never a rogue writable connection. A read PRAGMA, no state change.
    WalCheckpoint {
        mode: WalCheckpointMode,
        reply: oneshot::Sender<Result<WalCheckpointSummary, EventStoreError>>,
    },
    // --- 2.1b Action Gateway mutation commands: the pipeline runs ON the write-actor (the single
    // writer, forbidden #3) so each transition's {row + authoritative event} is one atomic txn. ---
    GatewaySubmit {
        req: Box<ActionRequest>,
        reply: oneshot::Sender<Result<ActionAck, GatewayError>>,
    },
    // 2.1c: bundled-plan submission (O-3). Boxed — ActionPlan is large (clippy large_enum_variant).
    GatewayPlanSubmit {
        plan: Box<ActionPlan>,
        reply: oneshot::Sender<Result<PlanAck, GatewayError>>,
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
    // C2 (the live INV-SEC-1 interception): route an intercepted agent tool-call through the Gateway
    // ON the single writer (the adjudication ActionRequest commits here — audit-before-verdict, §15
    // #5). Boxed — HookPayload carries the verbatim tool params (clippy large_enum_variant).
    GatewayIntercept {
        payload: Box<HookPayload>,
        reply: oneshot::Sender<InterceptOutcome>,
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

    /// Append a non-mutation OBSERVATION event (telemetry/lifecycle the daemon WITNESSES) through the
    /// single writer, **fire-and-forget** (4.0c). The SAME write-actor append path as [`append`](Self::append)
    /// — so the §15 redaction gate + the in-band projections run identically (NOT a redaction bypass;
    /// the `TelemetrySampled`/`DeviceRegistered` observation precedent, LESSON §23) — but `try_send` +
    /// drop-on-full instead of awaiting the reply: an observation must NEVER back-pressure the writer
    /// (forbidden #3 / LESSON §9) and is non-safety (LESSON §30 — a dropped sample is a stale meter,
    /// never a safety fault). Returns `true` if enqueued, `false` if dropped (channel full / actor
    /// gone); the reply [`EventId`] is discarded (the caller doesn't await it).
    pub fn try_append_observation(&self, intent: AppendIntent) -> bool {
        let (reply, _rx) = oneshot::channel();
        self.tx
            .try_send(Command::Append {
                intent: Box::new(intent),
                reply,
            })
            .is_ok()
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

    /// Refresh one worktree's §7.2 live-read git-axis cache through the single writer (the git-watcher
    /// loop's call). The actor reads git2 (read-only) at `path` (with `base` for ahead/behind) +
    /// re-derives `status`, then writes the non-event `proj_worktree` cache. Returns the rows updated
    /// (0 = an unknown `worktree_id` → a safe no-op).
    pub async fn refresh_worktree_status(
        &self,
        worktree_id: String,
        path: String,
        base: Option<String>,
    ) -> Result<usize, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::RefreshWorktreeStatus {
                worktree_id,
                path,
                base,
                reply,
            })
            .await
            .map_err(|_| RuntimeError::ActorGone)?;
        rx.await
            .map_err(|_| RuntimeError::ActorGone)?
            .map_err(RuntimeError::from)
    }

    /// Run a bounded WAL checkpoint through the single writer (the 4.3 checkpointer loop's call) —
    /// the checkpoint executes on the write-actor's own connection, so the WAL is bounded WITHOUT a
    /// second writable connection (forbidden #3 / LESSON §9).
    pub async fn wal_checkpoint(
        &self,
        mode: WalCheckpointMode,
    ) -> Result<WalCheckpointSummary, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Command::WalCheckpoint { mode, reply })
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

    /// `submit_action_plan` through the single writer (the §6.1 O-3 mutation path; the plan pipeline
    /// runs on the write-actor). Each step's inputs blob is §15-redacted at rest inside the pipeline.
    pub fn submit_action_plan_blocking(
        &self,
        plan: ActionPlan,
    ) -> Result<Result<PlanAck, GatewayError>, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .blocking_send(Command::GatewayPlanSubmit {
                plan: Box::new(plan),
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

    /// Route an intercepted agent tool-call through the Gateway on the single writer (C2 — the live
    /// INV-SEC-1 interception). The adjudication `ActionRequest` commits here (audit-BEFORE-verdict);
    /// when the write-actor was spawned with an [`IntegrityAlarm`], an audit-WRITE-fault raises the
    /// §17 alarm (call-2). **BLOCKING** — the sync IPC `intercept` handler can't `.await`; the returned
    /// [`InterceptOutcome`] drives the per-session `decision_sink` wait (an `AwaitingApproval` rests for
    /// the human; a `Resolved` is the verdict NOW).
    pub fn intercept_blocking(
        &self,
        payload: HookPayload,
    ) -> Result<InterceptOutcome, RuntimeError> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .blocking_send(Command::GatewayIntercept {
                payload: Box::new(payload),
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
    /// The thread blocks on the command channel; reads never reach it. **No `IntegrityAlarm`** — the
    /// `GatewayIntercept` path uses the no-alarm `route_intercept` (tests never exercise the call-2
    /// audit-fault-with-alarm path); PRODUCTION uses [`spawn_with_alarm`](Self::spawn_with_alarm).
    pub fn spawn(store: EventStore, clock: Box<dyn Clock>, gateway: Gateway) -> Self {
        Self::spawn_inner(store, clock, gateway, None, None)
    }

    /// Spawn the write-actor WITH the §17 [`IntegrityAlarm`] bound (C2): an agent-mutation
    /// adjudication's audit-WRITE-fault raises the alarm on the independent durable channel (call-2).
    /// No daemon-wide breaker — for tests that exercise the per-incident alarm without the systemic
    /// circuit-breaker. PRODUCTION uses [`spawn_with_alarm_and_breaker`](Self::spawn_with_alarm_and_breaker).
    pub fn spawn_with_alarm(
        store: EventStore,
        clock: Box<dyn Clock>,
        gateway: Gateway,
        alarm: Box<dyn IntegrityAlarm>,
    ) -> Self {
        Self::spawn_inner(store, clock, gateway, Some(Arc::from(alarm)), None)
    }

    /// Spawn the write-actor WITH both the §17 [`IntegrityAlarm`] AND the daemon-wide
    /// [`AuditBackboneBreaker`] bound (P4.0b-2c, production — `main.rs`). The breaker observes every
    /// Gateway audit-write outcome on this single chokepoint (the gate+feed seam) and, once latched,
    /// the actor fail-closed-denies every mutation WITHOUT attempting an audit-write (RULED B). The
    /// `Arc` is shared with `main` so the §17 surface can read [`is_tripped`](AuditBackboneBreaker::is_tripped).
    pub fn spawn_with_alarm_and_breaker(
        store: EventStore,
        clock: Box<dyn Clock>,
        gateway: Gateway,
        alarm: Arc<dyn IntegrityAlarm>,
        breaker: Arc<AuditBackboneBreaker>,
    ) -> Self {
        Self::spawn_inner(store, clock, gateway, Some(alarm), Some(breaker))
    }

    fn spawn_inner(
        store: EventStore,
        clock: Box<dyn Clock>,
        gateway: Gateway,
        alarm: Option<Arc<dyn IntegrityAlarm>>,
        breaker: Option<Arc<AuditBackboneBreaker>>,
    ) -> Self {
        // INVARIANT: a bound breaker implies a bound alarm. The intercept path feeds the breaker via
        // `route_intercept_live` (the alarm branch of the `GatewayIntercept` match) — with `alarm=None`
        // the no-alarm `route_intercept` runs and the intercept fault would not reach the breaker. No
        // public ctor yields (None alarm, Some breaker): `spawn`=(None,None), `spawn_with_alarm`=
        // (Some,None), `spawn_with_alarm_and_breaker`=(Some,Some). The debug_assert pins it so a future
        // ctor can't silently leave the intercept path unfed. (submit/approve/deny feed via
        // `observe_gateway_result` regardless of the alarm.)
        debug_assert!(
            alarm.is_some() || breaker.is_none(),
            "a bound breaker requires a bound alarm (the intercept feed lives on the live route)"
        );
        let (tx, rx) = mpsc::channel(COMMAND_CHANNEL_DEPTH);
        // the broadcast sender lives in the actor thread (it publishes post-commit); the handle
        // keeps a clone so `subscribe()` can mint receivers without touching the writer. The
        // initial receiver is discarded — the channel stays live as long as a Sender exists, and
        // subscribers come from `handle.subscribe()`.
        let (deltas, _) = broadcast::channel(BROADCAST_CAPACITY);
        let actor_deltas = deltas.clone();
        let join = std::thread::Builder::new()
            .name("nexusops-write-actor".to_string())
            .spawn(move || run_actor(store, clock, gateway, rx, actor_deltas, alarm, breaker))
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
    alarm: Option<Arc<dyn IntegrityAlarm>>,
    breaker: Option<Arc<AuditBackboneBreaker>>,
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
            // edges-026 — read git2 (read-only) + derive the §5.1 status in the runtime layer (the
            // persistence core stays git-free, the edges-022 rule), then write the non-event
            // proj_worktree cache via the single writer. `None` (a non-git path) stamps git_checked_at
            // only. A LOCAL read on this dedicated thread (bounded — it returns); the
            // write-actor-I/O-offload hardening is a deferred SPREAD (unified with drain_once + the
            // edges-023/024 external executors).
            Command::RefreshWorktreeStatus {
                worktree_id,
                path,
                base,
                reply,
            } => {
                let cache = compute_worktree_cache(Path::new(&path), base.as_deref());
                let _ = reply.send(store.refresh_worktree_status(
                    clock.as_ref(),
                    &worktree_id,
                    cache.as_ref(),
                ));
            }
            // 4.3: the bounded WAL checkpoint runs on this single writer's own connection — no rogue
            // writable connection is ever opened (forbidden #3 / LESSON §9). A read PRAGMA; no event,
            // no projection, no broadcast (it changes no observable state).
            Command::WalCheckpoint { mode, reply } => {
                let _ = reply.send(store.wal_checkpoint(mode));
            }
            // 2.1b Action Gateway mutations run the pipeline against the single writable store
            // (each transition's row+event is its own atomic txn inside the method). The gateway's
            // events fold into projections in-band; 2.1c (Q6) accumulates the `proj_approval_queue`
            // subscribe-deltas the pipeline touched + PUBLISHES them AFTER the txn commits (an
            // Err/rolled-back op publishes nothing; broadcast::send never back-pressures the writer,
            // forbidden #3) — mirroring the `Command::Append` publish-after-commit above.
            // P4.0b-2c (RULED B) — the audit-backbone gate+feed seam. Every Gateway mutation funnels
            // through this single chokepoint (forbidden #2/#3: the write-actor is the sole gateway
            // driver). When the breaker is LATCHED, fail-closed-deny WITHOUT attempting an audit-write
            // (`AuditBackboneDown`, before any gateway_txn — "no mutation slips the trip window");
            // otherwise run the op + FEED the breaker the audit-write outcome. Reads (Preview) +
            // get_projection/subscribe never reach the actor's mutation commands → stay live.
            Command::GatewaySubmit { req, reply } => {
                if breaker.as_ref().is_some_and(|b| b.is_tripped()) {
                    let _ = reply.send(Err(GatewayError::AuditBackboneDown));
                    continue;
                }
                let mut queue_deltas = Vec::new();
                let result = gateway.submit_action_collecting(&mut store, *req, &mut queue_deltas);
                if let Some(b) = &breaker {
                    b.observe_gateway_result(&result);
                }
                publish_after_commit(&deltas, &result, queue_deltas);
                let _ = reply.send(result);
            }
            Command::GatewayPlanSubmit { plan, reply } => {
                if breaker.as_ref().is_some_and(|b| b.is_tripped()) {
                    let _ = reply.send(Err(GatewayError::AuditBackboneDown));
                    continue;
                }
                let mut queue_deltas = Vec::new();
                let result =
                    gateway.submit_action_plan_collecting(&mut store, *plan, &mut queue_deltas);
                if let Some(b) = &breaker {
                    b.observe_gateway_result(&result);
                }
                publish_after_commit(&deltas, &result, queue_deltas);
                let _ = reply.send(result);
            }
            Command::GatewayApprove { approval_id, reply } => {
                if breaker.as_ref().is_some_and(|b| b.is_tripped()) {
                    let _ = reply.send(Err(GatewayError::AuditBackboneDown));
                    continue;
                }
                let mut queue_deltas = Vec::new();
                let result =
                    gateway.approve_collecting(&mut store, &approval_id, &mut queue_deltas);
                if let Some(b) = &breaker {
                    b.observe_gateway_result(&result);
                }
                publish_after_commit(&deltas, &result, queue_deltas);
                let _ = reply.send(result);
            }
            Command::GatewayDeny {
                approval_id,
                reason,
                reply,
            } => {
                if breaker.as_ref().is_some_and(|b| b.is_tripped()) {
                    let _ = reply.send(Err(GatewayError::AuditBackboneDown));
                    continue;
                }
                let mut queue_deltas = Vec::new();
                let result =
                    gateway.deny_collecting(&mut store, &approval_id, &reason, &mut queue_deltas);
                if let Some(b) = &breaker {
                    b.observe_gateway_result(&result);
                }
                publish_after_commit(&deltas, &result, queue_deltas);
                let _ = reply.send(result);
            }
            Command::GatewayPreview {
                action_request_id,
                reply,
            } => {
                let _ = reply.send(gateway.preview_action(&mut store, &action_request_id));
            }
            // C2 — the live INV-SEC-1 interception: the adjudication ActionRequest commits on this
            // single writer (audit-before-verdict). With a bound alarm, `route_intercept_live` raises
            // the §17 alarm on an audit-WRITE-fault (call-2); without one (tests), the no-alarm route.
            // No delta publish — the agent-mutation approval row folds in-band (the UI reads it via
            // get_projection; the live approval-queue delta on intercept is a flagged follow-on).
            Command::GatewayIntercept { payload, reply } => {
                // RULED-B gate: a latched breaker refuses the agent tool-call WITHOUT routing it
                // through the Gateway (no audit-write attempt) — fail-closed Deny (the agent is blocked
                // while the audit backbone is down).
                if breaker.as_ref().is_some_and(|b| b.is_tripped()) {
                    let _ = reply.send(InterceptOutcome::Resolved(MutationVerdict::Deny {
                        reason:
                            "audit backbone down (§17 systemic) — mutation refused, fail-closed"
                                .to_string(),
                    }));
                    continue;
                }
                let outcome = match &alarm {
                    // the live path feeds BOTH the per-incident alarm AND the daemon-wide breaker
                    // (the intercept's GatewayError is consumed here, so the breaker is fed via the route).
                    Some(a) => route_intercept_live(
                        &gateway,
                        &mut store,
                        &payload,
                        a.as_ref(),
                        breaker.as_deref(),
                    ),
                    None => route_intercept(&gateway, &mut store, &payload),
                };
                if reply.send(outcome).is_err() {
                    // the intercept connection dropped before the verdict — the adjudication audit
                    // event already committed (the hook fails closed, no verdict). Log for triage: the
                    // awaiting_approval row resolves via approve/deny or is swept by cancel_session.
                    eprintln!(
                        "nexusopsd: intercept reply channel dropped (hook gone); audit committed, verdict discarded"
                    );
                }
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

/// Read a worktree's live git truth (git2 READ-ONLY, forbidden #6) + derive its §7.2 cache values:
/// the git-sync axis wire value (`dirty_state`), ahead/behind, the HEAD sha, and the §5.1 `status`
/// recomputed against the Creating overlay. `None` = a non-git / inaccessible path (the caller stamps
/// `git_checked_at` only). Lives in the runtime layer so the persistence core stays git-free (the
/// edges-022 LESSON-17 layer rule). Pub so the live-read-cache logic is unit-testable over hermetic
/// temp-repo fixtures without spinning the full write-actor.
///
/// **Overlay-source follow-on (MIGRATION_9-deferred).** `status` is recomputed against a HARDCODED
/// `Creating` overlay because the overlay-lifecycle events (`WorktreeMerged`/`Locked`/`Prunable`/…)
/// have no emitter yet, so the resting overlay of every live worktree is `Creating` (the only emitted
/// overlay). When those emitters land a CLEAN overlay source is needed — an `overlay` column (a schema
/// change → MIGRATION_9) or an event-sourced overlay read — else e.g. a merged/locked worktree's
/// status would wrongly re-derive to a git-axis value on every refresh.
pub fn compute_worktree_cache(path: &Path, base: Option<&str>) -> Option<WorktreeGitCache> {
    let st = read_worktree_status(path, base)?;
    Some(WorktreeGitCache {
        // the within-axis git-sync status as the frozen snake_case wire value (`as_wire_str` is pinned
        // byte-for-byte to the serde serialization by precedence.rs's `derived_wire_str_matches_serde`).
        dirty_state: DerivedWorktreeStatus::Git(st.git_axis)
            .as_wire_str()
            .to_string(),
        // commit counts are tiny in practice, but guard the usize→i64 narrowing (no silent wrap — the
        // strict-typing posture; `graph_ahead_behind` is `usize`, the DDL column is i64 SQLite INTEGER).
        ahead_count: st.ahead_count.map(|n| i64::try_from(n).unwrap_or(i64::MAX)),
        behind_count: st
            .behind_count
            .map(|n| i64::try_from(n).unwrap_or(i64::MAX)),
        last_commit_sha: st.last_commit_sha,
        status: derive_worktree_status(st.git_axis, Some(WorktreeOverlay::Creating))
            .as_wire_str()
            .to_string(),
    })
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

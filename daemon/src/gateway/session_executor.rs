//! The session.create / session.kill executor (P4.0b-1 L3, CAT-1) — drives the 4.0a session
//! [`SupervisorHandle`] (over the NON-LIVE launcher) + emits the §15 #8 `SessionStarted`, delegating
//! every non-session action to the inner [`CatalogExecutor`].
//!
//! **Binding condition (4.0b-1, the lead's #1 ask, held by construction):** the launcher is NON-LIVE
//! (FakeHarness; NO live Claude adapter is constructed here — `daemon/tests/session_executor.rs`
//! greps this file's source to prove it, so the live-adapter identifiers must not even appear in
//! prose here), there is no reachable IPC `session.create` method, AND this executor is NOT wired
//! into the production Gateway (`main.rs` keeps `CatalogExecutor`). The live launch + the INV-SEC-1
//! interception + the IPC method land TOGETHER at **4.0b-2**. This is the "mechanism built, no live
//! caller" half (the 043 pattern), tested only via `submit_action`.

use std::sync::Mutex;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::events::SessionStarted;
use nexusops_shared::ids::{ExecutionProfileId, SessionId};
use nexusops_shared::status::Session;
use nexusops_shared::time::Timestamp;

use crate::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecError, ExecutionOutcome,
};
use crate::session::{SessionCommand, SessionLauncher, SupervisorHandle};

/// The session-lifecycle action types this executor handles specially (else it delegates).
const SESSION_CREATE: &str = "session.create";
const SESSION_KILL: &str = "session.kill";

/// The session.create/kill executor. Holds the NON-LIVE [`SessionLauncher`] seam (the live launcher
/// swaps in at 4.0b-2) + the [`SupervisorHandle`] (a SYNC, non-blocking, UNBOUNDED bridge — cannot
/// stall the write-actor, cat-1) + the inner [`CatalogExecutor`] for delegation.
pub struct SessionExecutor {
    /// `Mutex` ONLY to satisfy the `ActionExecutor: Sync` bound — the `SessionLauncher` is `Send` (not
    /// `Sync`) because `PtyLauncher` holds a `Send`-only `PtySpawner`. The lock is uncontended (the
    /// executor runs on the single write-actor thread) and carries no mutation (`launch_session(&self)`).
    launcher: Mutex<Box<dyn SessionLauncher>>,
    supervisor: SupervisorHandle,
    inner: CatalogExecutor,
}

impl SessionExecutor {
    pub fn new(launcher: Box<dyn SessionLauncher>, supervisor: SupervisorHandle) -> Self {
        Self {
            launcher: Mutex::new(launcher),
            supervisor,
            inner: CatalogExecutor::new(),
        }
    }

    /// §15 #8 (PIN b) — resolve the `ExecutionProfile` for the session at start. 4.0b-1 reads the
    /// requested profile id from the action inputs (the UI passes the chosen profile); a missing /
    /// invalid one mints a fresh placeholder (the `execution_profiles` registry is Phase 5). A profile
    /// CHANGE is the approval-gated `session.profile_change` (PIN c) — this routine record is no hop.
    fn resolve_profile(req: &ActionRequest) -> ExecutionProfileId {
        match req
            .inputs
            .get("execution_profile_id")
            .and_then(|v| v.as_str())
            .and_then(|s| ExecutionProfileId::parse(s).ok())
        {
            Some(id) => id,
            // 4.0b-1 placeholder — a session.create with no explicit profile mints a FRESH per-session
            // id (NOT a shared default); the `execution_profiles` registry-backed resolution is Phase 5.
            None => ExecutionProfileId::new(),
        }
    }

    fn execute_create(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition FIRST (this path runs its own
        // side effect, never reaching `inner.execute`'s validation) — a missing resource_ref → Failed,
        // never a silent skip (Q5).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // launch over the NON-LIVE launcher (FakeHarness; no live agent). The seam swaps to the real
        // launcher at 4.0b-2 — together with the live interception (no un-intercepted live agent).
        // `.unwrap()` (daemon no-bare-unwrap convention): the Mutex is UNCONTENDED (the executor runs
        // on the single write-actor thread) — it can only poison if `launch_session` panics WHILE the
        // lock is held, which would already crash the write-actor thread regardless; poison-propagation
        // adds no new failure mode. The Mutex exists ONLY to satisfy `ActionExecutor: Sync` over the
        // `Send`-only launcher (no mutation; see the field doc). Accepted.
        let launched = match self.launcher.lock().unwrap().launch_session() {
            Ok(l) => l,
            Err(e) => return ExecutionOutcome::Failed(format!("session launch failed: {e}")),
        };
        let session_id = launched.session_id.clone();
        let execution_profile_id = Self::resolve_profile(req);
        // drive the supervisor — a SYNC, NON-BLOCKING unbounded enqueue (cannot stall the write-actor,
        // cat-1). The status observer has no consumer in 4.0b-1 (the projection feed is later) → drop
        // the rx so the actor's status sends no-op on a closed channel rather than buffer unboundedly.
        let (status_tx, status_rx) = tokio::sync::mpsc::unbounded_channel();
        drop(status_rx);
        self.supervisor.spawn_session(launched, status_tx);
        // PIN a/b — emit SessionStarted (the pipeline appends it in txn-B, ATOMIC with ActionSucceeded)
        // carrying the §15 #8 execution_profile_id (recorded-at-start).
        let payload = SessionStarted {
            status: Session::Starting,
            harness: None,
            model: None,
            display_name: None,
            execution_profile_id: Some(execution_profile_id),
        };
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: format!(
                "session.create — spawned a supervised session {} (NON-LIVE launcher; 4.0b-1)",
                session_id.as_str()
            ),
            // a session-actor WAS spawned (the side effect) BEFORE txn-B → if the terminal write is
            // lost (§17), `ActionPartiallySucceeded` records the divergence (the actor is
            // orphaned-but-ephemeral) rather than a clean rollback. The honest §17 partial signal.
            side_effect_applied: true,
            emitted_events: vec![EmittedEvent::SessionStarted {
                session_id,
                payload,
            }],
        }
    }

    fn execute_kill(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog precondition FIRST (this path never reaches `inner.execute`).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // session.kill is NaturalResourceRef-keyed: the target session is the primary resource_ref.
        let Some(rref) = req.resource_refs.first() else {
            return ExecutionOutcome::Failed(
                "session.kill requires the target session resource_ref".to_string(),
            );
        };
        let target = match SessionId::parse(&rref.id) {
            Ok(id) => id,
            Err(_) => {
                return ExecutionOutcome::Failed(format!(
                    "session.kill: invalid session id '{}'",
                    rref.id
                ))
            }
        };
        let delivered = self.supervisor.route(target, SessionCommand::Kill);
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: format!("session.kill — routed Kill (delivered={delivered})"),
            // honest §17: a side effect happened ONLY if the Kill was actually delivered (a live
            // supervisor holding the session). `delivered == false` (the session is gone/unknown) →
            // nothing changed → a lost terminal write rolls back cleanly, NOT a false partial-success.
            side_effect_applied: delivered,
            emitted_events: vec![],
        }
    }
}

impl ActionExecutor for SessionExecutor {
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError> {
        self.inner.validate(req)
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            // the session paths run their OWN side effect (launch / route) — they validate the catalog
            // precondition themselves (they never reach `inner.execute`).
            SESSION_CREATE => self.execute_create(req),
            SESSION_KILL => self.execute_kill(req),
            // every non-session action delegates to the catalog executor, which validates internally
            // (`resolve`) — no double-check here.
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}

//! The session.create / session.kill executor (P4.0b, CAT-1) — drives the 4.0a session
//! [`SupervisorHandle`] (over an INJECTED launcher) + emits the §15 #8 `SessionStarted`, delegating
//! every non-session action to the inner [`CatalogExecutor`]. **Since 4.0b-2 this IS the reachable
//! production `session.create` executor** (registered under `ExecutorKind::Session` in `main.rs`); the
//! live INV-SEC-1 interception co-resides there (the `test_live_session_create_has_interception`
//! structural pin).
//!
//! **Launcher-agnostic (binding condition, test-9-enforced):** the executor constructs NO live agent
//! launch path of its own — the launcher is INJECTED via the [`SessionLauncher`] seam (a `FakeLauncher`
//! in tests; the real one built in `main.rs`). So no live-adapter identifiers appear in this file, and
//! the executor cannot smuggle an un-intercepted launch (the #10 live-spawn lives at the injected
//! launcher, not here).
//!
//! **P5.3a (§15 #8):** profile resolution is REGISTRY-BACKED + FAIL-CLOSED — an unknown requested
//! profile id refuses the launch (no silent mint); `None` resolves to the seeded default. The
//! `execution_profiles` registry write + the cold-start seed live in [`crate::profiles`].

use std::sync::Mutex;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::events::{SessionProfileChanged, SessionStarted};
use nexusops_shared::ids::{ExecutionProfileId, SessionId};
use nexusops_shared::status::Session;
use nexusops_shared::time::Timestamp;

use crate::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecError, ExecutionOutcome,
};
use crate::profiles::ProfileLookup;
use crate::session::{SessionCommand, SessionLauncher, SupervisorHandle};
use crate::terminal::TerminalSession;

/// The session-lifecycle action types this executor handles specially (else it delegates).
const SESSION_CREATE: &str = "session.create";
const SESSION_KILL: &str = "session.kill";
/// P5.3b/085 — the §15 #8 no-silent-account-hop profile rebind (the approval-gated CHANGE, PIN c).
const SESSION_PROFILE_CHANGE: &str = "session.profile_change";

/// The byte appended after an `initial_prompt` to SUBMIT it in claude's TUI (P4.0b-2-smoke, brief 053
/// Q2). `\r` (CR) — claude's TUI submits on Enter, which is CR in PTY raw mode. The deterministic test
/// asserts the exact bytes written; *which* terminator actually submits at the live `claude` is the
/// runbook's #1 0.1-HITL watch item (the `\n` fallback is documented there).
const SUBMIT_TERMINATOR: u8 = b'\r';

/// §15 #8 fail-closed profile-resolution failures (P5.3a). STRUCTURAL — the message echoes only the
/// requester-supplied id string (no new info beyond what the requester sent); execute_create maps any
/// variant → `ExecutionOutcome::Failed` BEFORE launching (no orphaned session-actor).
#[derive(Debug, thiserror::Error)]
enum ProfileResolveError {
    /// the requested profile id is not registered — no silent mint / no account-hop (§15 #8).
    #[error("execution profile not found: {0}")]
    Unknown(String),
    /// the requested profile id is not a valid `ExecutionProfileId`.
    #[error("invalid execution profile id: {0}")]
    Invalid(String),
    /// the registry lookup itself failed (a read error) — fail-closed (refuse the launch).
    #[error("execution profile lookup failed: {0}")]
    Lookup(String),
}

/// The session.create/kill executor. Holds the NON-LIVE [`SessionLauncher`] seam (the live launcher
/// swaps in at 4.0b-2) + the [`SupervisorHandle`] (a SYNC, non-blocking, UNBOUNDED bridge — cannot
/// stall the write-actor, cat-1) + the inner [`CatalogExecutor`] for delegation.
pub struct SessionExecutor {
    /// `Mutex` ONLY to satisfy the `ActionExecutor: Sync` bound — the `SessionLauncher` is `Send` (not
    /// `Sync`) because `PtyLauncher` holds a `Send`-only `PtySpawner`. The lock is uncontended (the
    /// executor runs on the single write-actor thread) and carries no mutation (`launch_session(&self)`).
    launcher: Mutex<Box<dyn SessionLauncher>>,
    supervisor: SupervisorHandle,
    /// P5.3a — the §15 #8 registry read seam: resolve the requested/default `ExecutionProfile` against the
    /// canonical `execution_profiles` registry, fail-closed on an unknown id (injected; sqlite-backed in prod).
    profile_lookup: Box<dyn ProfileLookup>,
    inner: CatalogExecutor,
}

impl SessionExecutor {
    pub fn new(
        launcher: Box<dyn SessionLauncher>,
        supervisor: SupervisorHandle,
        profile_lookup: Box<dyn ProfileLookup>,
    ) -> Self {
        Self {
            launcher: Mutex::new(launcher),
            supervisor,
            profile_lookup,
            inner: CatalogExecutor::new(),
        }
    }

    /// §15 #8 (PIN b) — resolve the `ExecutionProfile` for the session at start, REGISTRY-BACKED +
    /// FAIL-CLOSED (P5.3a). A requested id present in the registry resolves to it; a requested id NOT in
    /// the registry (or unparseable) is REFUSED — no silent mint, no account-hop; `None` (no profile
    /// requested) resolves to the seeded default. A profile CHANGE is the approval-gated
    /// `session.profile_change` (PIN c) — this routine record is no hop.
    fn resolve_profile(
        &self,
        req: &ActionRequest,
    ) -> Result<ExecutionProfileId, ProfileResolveError> {
        match req
            .inputs
            .get("execution_profile_id")
            .and_then(|v| v.as_str())
        {
            Some(s) => {
                let id = ExecutionProfileId::parse(s)
                    .map_err(|_| ProfileResolveError::Invalid(s.to_string()))?;
                if self
                    .profile_lookup
                    .exists(&id)
                    .map_err(|e| ProfileResolveError::Lookup(e.to_string()))?
                {
                    Ok(id)
                } else {
                    Err(ProfileResolveError::Unknown(s.to_string()))
                }
            }
            None => self
                .profile_lookup
                .default_id()
                .map_err(|e| ProfileResolveError::Lookup(e.to_string())),
        }
    }

    fn execute_create(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition FIRST (this path runs its own
        // side effect, never reaching `inner.execute`'s validation) — a missing resource_ref → Failed,
        // never a silent skip (Q5).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // §15 #8 (P5.3a) — resolve the ExecutionProfile BEFORE launching: a fail-closed resolution (an
        // unknown/invalid requested id, or a registry read error) refuses the session.create with NO
        // launch and NO session-actor spawned (no orphan). Resolved-at-start, recorded in SessionStarted
        // (no silent mint, no account-hop).
        let execution_profile_id = match self.resolve_profile(req) {
            Ok(id) => id,
            Err(e) => return ExecutionOutcome::Failed(e.to_string()),
        };
        // launch over the INJECTED launcher (`FakeLauncher` in tests; the real one built in `main.rs`,
        // co-resident with the live interception). `.unwrap()` (daemon no-bare-unwrap convention): the
        // Mutex is UNCONTENDED (the executor runs on the single write-actor thread) — it can only poison
        // if `launch_session` panics WHILE the lock is held, which would already crash the write-actor
        // thread regardless; poison-propagation adds no new failure mode. The Mutex exists ONLY to satisfy
        // `ActionExecutor: Sync` over the `Send`-only launcher (no mutation; see the field doc). Accepted.
        let mut launched = match self.launcher.lock().unwrap().launch_session() {
            Ok(l) => l,
            Err(e) => return ExecutionOutcome::Failed(format!("session launch failed: {e}")),
        };
        let session_id = launched.session_id.clone();
        // Option-G dev-drive (brief 053): feed an optional `initial_prompt` to the LAUNCHED session's
        // PTY, post-launch, BEFORE the session moves into the supervisor. Best-effort — see the helper.
        let prompt_note = write_initial_prompt(&mut launched.terminal, req);
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
                "session.create — spawned a supervised session {}{}",
                session_id.as_str(),
                prompt_note
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

    /// P5.3b/085 (PIN c, §15 #8) — the `session.profile_change` arm: rebind a live session's ExecutionProfile
    /// to a REGISTERED target (the no-silent-account-hop CHANGE — risk-2, approval-gated by the catalog; the
    /// policy gate denies any non-UI requester before risk). The target SESSION is the audited resource_ref;
    /// the NEW profile is `inputs.execution_profile_id` and is **REQUIRED** (unlike session.create, a change has
    /// no default fallback — an absent/unparseable/unregistered target → Failed BEFORE any swap: no mint, no
    /// hop). On success, emit the audited `SessionProfileChanged` (the swap is RECORDED, never silently
    /// switched). No external side effect — the live re-bind/restart of the running agent is a deferred
    /// follow-on; the event is the §15 #8 record.
    fn execute_profile_change(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition FIRST (the target session ref).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // the target session is the audited resource_ref (the session.kill keying precedent).
        let Some(rref) = req.resource_refs.first() else {
            return ExecutionOutcome::Failed(
                "session.profile_change requires the target session resource_ref".to_string(),
            );
        };
        let session_id = match SessionId::parse(&rref.id) {
            Ok(id) => id,
            Err(_) => {
                return ExecutionOutcome::Failed(format!(
                    "session.profile_change: invalid session id '{}'",
                    rref.id
                ))
            }
        };
        // the NEW profile is REQUIRED (a change must name its target — no default fallback) + must be
        // REGISTERED (fail-closed-on-unknown, §15 #8 no-account-hop; NO mint).
        let new_profile = match req
            .inputs
            .get("execution_profile_id")
            .and_then(|v| v.as_str())
        {
            Some(s) => match ExecutionProfileId::parse(s) {
                Ok(id) => id,
                Err(_) => {
                    return ExecutionOutcome::Failed(format!(
                        "session.profile_change: invalid execution profile id '{s}'"
                    ))
                }
            },
            None => {
                return ExecutionOutcome::Failed(
                    "session.profile_change requires inputs.execution_profile_id (no default)"
                        .to_string(),
                )
            }
        };
        match self.profile_lookup.exists(&new_profile) {
            Ok(true) => {}
            Ok(false) => {
                return ExecutionOutcome::Failed(format!(
                    "execution profile not found: {} (§15 #8 no-account-hop — no mint)",
                    new_profile.as_str()
                ))
            }
            Err(e) => {
                return ExecutionOutcome::Failed(format!("execution profile lookup failed: {e}"))
            }
        }
        // emit the audited swap record (the §15 #8 recorded-not-silent event). Serialize the frozen struct
        // here (the Namespaced emit precedent — the executor owns the payload + handles the serde fault).
        let payload = match serde_json::to_string(&SessionProfileChanged {
            session_id,
            execution_profile_id: new_profile,
        }) {
            Ok(p) => p,
            Err(e) => {
                return ExecutionOutcome::Failed(format!(
                    "session.profile_change: failed to encode the swap event: {e}"
                ))
            }
        };
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "session.profile_change — recorded the approved profile rebind".to_string(),
            // no external side effect — the SessionProfileChanged event IS the durable record (the live
            // agent re-bind/restart is the deferred follow-on). A lost terminal event rolls back cleanly.
            side_effect_applied: false,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: SessionProfileChanged::EVENT_TYPE,
                payload_json: payload,
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
            SESSION_PROFILE_CHANGE => self.execute_profile_change(req),
            // every non-session action delegates to the catalog executor, which validates internally
            // (`resolve`) — no double-check here.
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}

/// Option-G dev-drive (P4.0b-2-smoke / brief 053): write the optional `initial_prompt` (+ the
/// [`SUBMIT_TERMINATOR`]) to the LAUNCHED session's PTY **exactly once**, **post-launch**, so a real
/// `claude` self-drives a demo prompt. Returns a `detail` suffix recording the outcome.
///
/// **Best-effort / fail-SOFT (NOT a safety I/O):** a PTY write error degrades to a recorded detail —
/// it does NOT fail the session (the prompt-feed is dev convenience; every agent tool call still routes
/// through the unchanged `intercept` adjudication, the chokepoint, regardless of how claude was
/// prompted). Absent / empty prompt → nothing written (additive/opt-in — the no-prompt path is
/// byte-unchanged). This does NOT touch the O-13 launch argv / the #10 enforcement surface.
fn write_initial_prompt(terminal: &mut TerminalSession, req: &ActionRequest) -> &'static str {
    let Some(prompt) = req.inputs.get("initial_prompt").and_then(|v| v.as_str()) else {
        return "";
    };
    if prompt.is_empty() {
        return "";
    }
    let mut bytes = prompt.as_bytes().to_vec();
    bytes.push(SUBMIT_TERMINATOR);
    match terminal.write(&bytes) {
        Ok(()) => " (initial_prompt fed to the session PTY)",
        // fail-soft: the session already launched + will spawn; the prompt-feed convenience degraded.
        Err(_) => " (initial_prompt write degraded — best-effort dev-drive, session unaffected)",
    }
}

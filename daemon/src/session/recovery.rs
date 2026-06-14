//! The deterministic daemon-restart session-recovery orchestration (P4.1b-1, brief 059) — the
//! PRODUCTION caller of `decide_resume` (4.1a). On restart, after the supervisor + write-actor are up
//! (`main.rs`, post-`cold_start`), it enumerates the sessions that were live at shutdown, decides each
//! via the §8.1 B2-strict ladder, dispatches the chosen `ResumeMode` over the broker/launcher SEAM, and
//! emits a per-session System-actor recovery OBSERVATION signal (the §11.4 resumed-vs-replayed bit) —
//! preserving each session's §15 #8 ExecutionProfile.
//!
//! **Q1 = (a) System-actor recovery (lead/user-ruled).** A recovery is the daemon re-materializing its
//! OWN already-approved prior session — a realization of LESSON §10 (the §16 cold-start Device/
//! LocalRunner self-registration precedent: substrate, NOT a policy-gated Gateway Action from an
//! untrusted proposer). It grants ZERO new authority: (1) recovery touches only genuinely-live sessions
//! ([`is_recoverable_status`]; the production reader sources identity + profile from the COMMITTED prior
//! `SessionStarted`); (2) the recovery event is audited (§15-gated write-actor); (3) the §15 #8 profile
//! is PRESERVED from the original `SessionStarted` (no re-resolution / account-hop); (4) the recovered
//! agent's SUBSEQUENT tool calls STILL route through the live interception→Gateway. The orchestration
//! takes a `Broker` + a dispatch seam + an observation sink and NO `WriteHandle`/submit dependency — so
//! it can never be a 2nd mutator (INV-SEC-1, enforced structurally by the signature).
//!
//! Deterministic over a `FakeBroker` + injected seams; the REAL launcher/broker drive is 4.1b-2, and
//! the LIVE survival is the 0.1/0.3-HITL follow-on (a live-process property, not unit-testable).

use nexusops_shared::harness::{ResumeMode, ResumeResult};
use nexusops_shared::ids::{ExecutionProfileId, SessionId};
use nexusops_shared::status::Session;

use crate::harness::resume::{decide_resume, ResumeInputs};
use crate::session::broker::Broker;

/// A session that was live at shutdown — the recovery enumeration yields one per non-terminal
/// `proj_session` row (read-only WAL; the production reader sources identity + the §15 #8 profile from
/// the COMMITTED prior `SessionStarted`, C2). Carries everything the §8.1 decision + the profile
/// preservation need. Daemon-internal (not a wire type).
#[derive(Clone, Debug)]
pub struct RecoverableSession {
    pub session_id: SessionId,
    /// the §5.1 Session status at shutdown — terminal rows are filtered out ([`is_recoverable_status`]).
    pub status: Session,
    /// the §15 #8 ExecutionProfile bound at the original `SessionStarted` — PRESERVED on recovery (no
    /// silent account-hop). `None` only for a profile-less session (pre-§15-#8).
    pub execution_profile_id: Option<ExecutionProfileId>,
    /// the harness supports native resume (`--resume` / `thread/resume`) — from `HarnessCapabilities`.
    pub supports_resume: bool,
    /// a resume handle exists (transcript / thread id) — from the persisted session state.
    pub has_resume_handle: bool,
    /// serialized scrollback exists to replay.
    pub has_scrollback: bool,
    /// the scrollback event count (carried on the `Replayed` rung only).
    pub replayed_event_count: u64,
}

/// The recovery effect dispatched for a session — the §8.1 ladder rung mapped to its seam action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// `ReattachedLive` → reattach the surviving in-flight PTY via the broker (no relaunch).
    Reattach,
    /// `Resumed` → relaunch + native resume (`--resume` / `thread/resume`).
    RelaunchResume,
    /// `Replayed` → relaunch + scrollback replay.
    RelaunchReplay,
    /// `Relaunched` → a fresh relaunch + the §17 "restart session" affordance.
    Relaunch,
}

impl RecoveryAction {
    /// Map a §8.1 `ResumeMode` to its recovery dispatch action (a TOTAL map — every mode has one rung).
    fn from_mode(mode: ResumeMode) -> Self {
        match mode {
            ResumeMode::ReattachedLive => Self::Reattach,
            ResumeMode::Resumed => Self::RelaunchResume,
            ResumeMode::Replayed => Self::RelaunchReplay,
            ResumeMode::Relaunched => Self::Relaunch,
        }
    }
}

/// The per-session recovery outcome — the §8.1 decision + the dispatched action + the PRESERVED profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRecoveryOutcome {
    pub session_id: SessionId,
    pub result: ResumeResult,
    pub action: RecoveryAction,
    /// the PRESERVED §15 #8 profile (carried from the original `SessionStarted`).
    pub execution_profile_id: Option<ExecutionProfileId>,
}

/// The recovery OBSERVATION signal — the §11.4 resumed-vs-replayed bit the UI renders. The production
/// sink builds a `SessionRecovered` observation event + appends it via the §15-gated write-actor
/// (System-actor; the `DeviceRegistered`/`TelemetrySampled` precedent, LESSONS §10/§23). NOT a Gateway
/// Action (Q1=(a)). `mode == Relaunched` is the §17 "restart session" affordance the §11.4 UI renders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverySignal {
    pub session_id: SessionId,
    pub mode: ResumeMode,
    pub replayed_event_count: u64,
    pub execution_profile_id: Option<ExecutionProfileId>,
}

/// The dispatch seam — performs the recovery effect (reattach / relaunch+resume/replay/relaunch) over
/// the broker/launcher, preserving the original session id + §15 #8 profile. Abstracted so the
/// orchestration is deterministic; the PRODUCTION impl drives the real launcher (identity-preserving
/// relaunch) + broker (reattach) — 4.1b-2, LIVE survival = HITL. The recovered agent's subsequent tool
/// calls run through the SAME launcher path that enforces interception in production (Q1 condition #4).
pub trait RecoveryDispatch {
    fn dispatch(
        &self,
        session_id: &SessionId,
        action: RecoveryAction,
        profile: Option<&ExecutionProfileId>,
    );
}

/// The recovery-signal sink — one observation per recovered session. The production impl appends the
/// `SessionRecovered` event via the write-actor (Q1=(a) System-actor; never a Gateway mutation).
pub trait RecoverySignalSink {
    fn emit_recovered(&self, signal: RecoverySignal);
}

/// True for the §5.1 Session states a restart recovers — every NON-terminal state. Terminal sessions
/// ({Failed, Completed, Archived, Killed}) are history, not the live set (the production `proj_session`
/// reader filters on this predicate; condition #1 — recover only genuinely-live sessions).
pub fn is_recoverable_status(status: Session) -> bool {
    !status.is_terminal()
}

/// Recover the sessions that were live at shutdown (the §8.1 B2-strict restart ladder). For each
/// NON-terminal session: query the broker for a survivor → build `ResumeInputs` → `decide_resume`
/// (4.1a) → dispatch the chosen action over the seam → emit the recovery observation signal (profile
/// preserved). Returns the per-session outcomes. Pure orchestration over the injected seams — NO DB /
/// `WriteHandle` / Gateway (Q1=(a): never a 2nd mutator).
pub fn recover_sessions_on_restart(
    sessions: &[RecoverableSession],
    broker: &dyn Broker,
    dispatch: &dyn RecoveryDispatch,
    signal_sink: &dyn RecoverySignalSink,
) -> Vec<SessionRecoveryOutcome> {
    let mut outcomes = Vec::new();
    for s in sessions {
        // condition #1 — recover only genuinely-live sessions (the §5.1 live set, not history).
        if !is_recoverable_status(s.status) {
            continue;
        }
        let inputs = ResumeInputs {
            broker_has_live_session: broker.reattach_outcome(&s.session_id).has_live_session,
            supports_resume: s.supports_resume,
            has_resume_handle: s.has_resume_handle,
            has_scrollback: s.has_scrollback,
            replayed_event_count: s.replayed_event_count,
        };
        let result = decide_resume(&inputs);
        let action = RecoveryAction::from_mode(result.mode);
        // dispatch the effect over the seam — the ORIGINAL profile is bound (no re-resolution; §15 #8).
        dispatch.dispatch(&s.session_id, action, s.execution_profile_id.as_ref());
        // emit the audited recovery observation (System-actor; the §11.4 resumed-vs-replayed bit).
        signal_sink.emit_recovered(RecoverySignal {
            session_id: s.session_id.clone(),
            mode: result.mode,
            replayed_event_count: result.replayed_event_count,
            execution_profile_id: s.execution_profile_id.clone(),
        });
        outcomes.push(SessionRecoveryOutcome {
            session_id: s.session_id.clone(),
            result,
            action,
            execution_profile_id: s.execution_profile_id.clone(),
        });
    }
    outcomes
}

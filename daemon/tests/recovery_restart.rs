//! P4.1b-1 (brief 059) — the deterministic daemon-restart session-recovery orchestration.
//!
//! `recover_sessions_on_restart` is the PRODUCTION caller of `decide_resume` (4.1a): on restart it
//! enumerates the sessions that were live at shutdown, decides per session (reattach-live → resume →
//! replay → relaunch), dispatches the chosen `ResumeMode` over the broker/launcher SEAM, and emits a
//! per-session System-actor recovery OBSERVATION signal (the §11.4 resumed-vs-replayed bit) —
//! preserving each session's §15 #8 ExecutionProfile (no silent account-hop).
//!
//! Deterministic: a `FakeBroker` (the reattach INPUT) + a recording dispatch (the effect) + a recording
//! observation sink. The real detachable-terminal broker is 4.1b-2; LIVE survival is the 0.1/0.3-HITL
//! follow-on. The recovery is a System-actor lifecycle event, NOT a Gateway Action (Q1=(a), the §16
//! cold-start self-registration precedent, bootstrap.rs / LESSONS §10/§23).

use std::cell::RefCell;

use nexusops_shared::harness::ResumeMode;
use nexusops_shared::ids::{ExecutionProfileId, SessionId};
use nexusops_shared::status::Session;

use nexusopsd::session::broker::FakeBroker;
use nexusopsd::session::recovery::{
    is_recoverable_status, recover_sessions_on_restart, RecoverableSession, RecoveryAction,
    RecoveryDispatch, RecoverySignal, RecoverySignalSink,
};

// ---- test doubles — a recording dispatch (the seam effect) + a recording observation sink ----------

/// Records each dispatch call: (session, the chosen action, the bound profile). The PRODUCTION dispatch
/// drives the real launcher (relaunch, identity-preserving) + broker (reattach) — 4.1b-2 / HITL.
#[derive(Default)]
struct RecordingDispatch {
    calls: RefCell<Vec<(SessionId, RecoveryAction, Option<ExecutionProfileId>)>>,
}

impl RecoveryDispatch for RecordingDispatch {
    fn dispatch(
        &self,
        session_id: &SessionId,
        action: RecoveryAction,
        profile: Option<&ExecutionProfileId>,
    ) {
        self.calls
            .borrow_mut()
            .push((session_id.clone(), action, profile.cloned()));
    }
}

/// Records each recovery OBSERVATION signal. The PRODUCTION sink appends a `SessionRecovered`
/// observation event via the write-actor (System-actor; the `DeviceRegistered`/`TelemetrySampled`
/// precedent, LESSONS §10/§23) — NOT a Gateway Action (Q1=(a)).
#[derive(Default)]
struct RecordingSink {
    signals: RefCell<Vec<RecoverySignal>>,
}

impl RecoverySignalSink for RecordingSink {
    fn emit_recovered(&self, signal: RecoverySignal) {
        self.signals.borrow_mut().push(signal);
    }
}

/// A non-terminal (live-at-shutdown) session with the given profile + no resume affordances by default
/// (each test flips on the inputs its rung needs).
fn live_session(profile: Option<ExecutionProfileId>) -> RecoverableSession {
    RecoverableSession {
        session_id: SessionId::new(),
        status: Session::Active,
        execution_profile_id: profile,
        supports_resume: false,
        has_resume_handle: false,
        has_scrollback: false,
        replayed_event_count: 0,
    }
}

// ---- RED #1 — broker holds a surviving PTY → ReattachedLive + a Reattach dispatch ------------------

#[test]
fn test_reattach_live_when_broker_holds_session() {
    // spec(§8.1) — B2-strict: a live-at-shutdown session whose broker still holds a surviving in-flight
    // PTY reattaches → ReattachedLive (the TOP of the ladder) + a Reattach dispatch over the seam.
    let s = live_session(None);
    let sid = s.session_id.clone();
    let broker = FakeBroker::new().with_live(&sid); // reports has_live_session=true for sid
    let dispatch = RecordingDispatch::default();
    let sink = RecordingSink::default();

    let outcomes = recover_sessions_on_restart(&[s], &broker, &dispatch, &sink);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].result.mode, ResumeMode::ReattachedLive);
    assert_eq!(outcomes[0].action, RecoveryAction::Reattach);
    assert_eq!(
        dispatch.calls.borrow().as_slice(),
        &[(sid, RecoveryAction::Reattach, None)]
    );
    assert_eq!(sink.signals.borrow().len(), 1);
    assert_eq!(sink.signals.borrow()[0].mode, ResumeMode::ReattachedLive);
}

// ---- RED #2 — no survivor but resume-capable + a handle → Resumed + a RelaunchResume dispatch ------

#[test]
fn test_resume_when_supported_no_survivor() {
    // spec(§8) — no broker survivor + supports_resume + a resume handle → Resumed (a NEW process resumed
    // from the transcript via --resume/thread/resume) + a RelaunchResume dispatch.
    let mut s = live_session(None);
    s.supports_resume = true;
    s.has_resume_handle = true;
    let broker = FakeBroker::new(); // no survivors
    let dispatch = RecordingDispatch::default();
    let sink = RecordingSink::default();

    let outcomes = recover_sessions_on_restart(&[s], &broker, &dispatch, &sink);

    assert_eq!(outcomes[0].result.mode, ResumeMode::Resumed);
    assert_eq!(outcomes[0].action, RecoveryAction::RelaunchResume);
    assert_eq!(dispatch.calls.borrow()[0].1, RecoveryAction::RelaunchResume);
    assert_eq!(sink.signals.borrow()[0].mode, ResumeMode::Resumed);
}

// ---- RED #3 — scrollback only → Replayed (carries the count) + a RelaunchReplay dispatch -----------

#[test]
fn test_replay_when_scrollback_only() {
    // spec(§8) — no broker, no native resume, serialized scrollback present → Replayed + RelaunchReplay;
    // the count is carried onto BOTH the result and the recovery signal (the only rung with a count).
    let mut s = live_session(None);
    s.has_scrollback = true;
    s.replayed_event_count = 23;
    let broker = FakeBroker::new();
    let dispatch = RecordingDispatch::default();
    let sink = RecordingSink::default();

    let outcomes = recover_sessions_on_restart(&[s], &broker, &dispatch, &sink);

    assert_eq!(outcomes[0].result.mode, ResumeMode::Replayed);
    assert_eq!(outcomes[0].result.replayed_event_count, 23);
    assert_eq!(outcomes[0].action, RecoveryAction::RelaunchReplay);
    assert_eq!(sink.signals.borrow()[0].replayed_event_count, 23);
}

// ---- RED #4 — nothing available → Relaunched + the §17 "restart session" affordance ---------------

#[test]
fn test_relaunch_and_restart_affordance() {
    // spec(§8/§17) — nothing available → Relaunched (a fresh launch) + a Relaunch dispatch; the recovery
    // signal carries mode Relaunched, which the §11.4 UI renders as the "restart session" affordance tail.
    let s = live_session(None);
    let broker = FakeBroker::new();
    let dispatch = RecordingDispatch::default();
    let sink = RecordingSink::default();

    let outcomes = recover_sessions_on_restart(&[s], &broker, &dispatch, &sink);

    assert_eq!(outcomes[0].result.mode, ResumeMode::Relaunched);
    assert_eq!(outcomes[0].action, RecoveryAction::Relaunch);
    assert_eq!(dispatch.calls.borrow()[0].1, RecoveryAction::Relaunch);
    assert_eq!(
        sink.signals.borrow()[0].mode,
        ResumeMode::Relaunched,
        "Relaunched is the §17 restart-session affordance the §11.4 UI renders"
    );
}

// ---- RED #5 — the §15 #8 ExecutionProfile is PRESERVED on recovery (no silent account-hop) ---------

#[test]
fn test_profile_preserved_on_recovery() {
    // spec(§15 #8) — a recovered session keeps its ORIGINAL execution profile (the no-silent-account-hop
    // invariant): the outcome, the relaunch DISPATCH, and the recovery SIGNAL all carry the original id.
    let profile = ExecutionProfileId::new();
    let mut s = live_session(Some(profile.clone()));
    s.supports_resume = true;
    s.has_resume_handle = true; // → Resumed: a relaunch path that re-binds the profile
    let broker = FakeBroker::new();
    let dispatch = RecordingDispatch::default();
    let sink = RecordingSink::default();

    let outcomes = recover_sessions_on_restart(&[s], &broker, &dispatch, &sink);

    assert_eq!(outcomes[0].execution_profile_id, Some(profile.clone()));
    assert_eq!(
        dispatch.calls.borrow()[0].2,
        Some(profile.clone()),
        "the relaunch binds the ORIGINAL profile — never a fresh/default one (§15 #8)"
    );
    assert_eq!(sink.signals.borrow()[0].execution_profile_id, Some(profile));
}

// ---- RED #6 — only NON-terminal sessions are recovered (the §5.1 live set, not history) ------------

#[test]
fn test_only_live_at_shutdown_sessions_recovered() {
    // spec(§5.1) — recover the live set, not history: terminal sessions {Failed, Completed, Archived,
    // Killed} are NOT recovered. The orchestration filters via the §5.1 terminal set; the production
    // proj_session reader yields the same non-terminal set.
    let live = live_session(None);
    let live_id = live.session_id.clone();
    let mut done = live_session(None);
    done.status = Session::Completed; // terminal → skipped
    let mut killed = live_session(None);
    killed.status = Session::Killed; // terminal → skipped
    let broker = FakeBroker::new();
    let dispatch = RecordingDispatch::default();
    let sink = RecordingSink::default();

    let outcomes = recover_sessions_on_restart(&[live, done, killed], &broker, &dispatch, &sink);

    assert_eq!(
        outcomes.len(),
        1,
        "only the non-terminal session is recovered"
    );
    assert_eq!(outcomes[0].session_id, live_id);
    assert_eq!(
        dispatch.calls.borrow().len(),
        1,
        "no dispatch for a terminal session"
    );
    assert_eq!(
        sink.signals.borrow().len(),
        1,
        "no recovery signal for a terminal session"
    );

    // the pure predicate the filter keys off (and the production proj_session reader uses): exactly the
    // §5.1 Session(17) non-terminal set.
    assert!(is_recoverable_status(Session::Active));
    assert!(is_recoverable_status(Session::WaitingOnHumanInput));
    for terminal in [
        Session::Failed,
        Session::Completed,
        Session::Archived,
        Session::Killed,
    ] {
        assert!(
            !is_recoverable_status(terminal),
            "{terminal:?} is terminal — not recovered"
        );
    }
}

// ---- RED #7 — the recovery signal is an OBSERVATION (System-actor), NEVER a Gateway mutation -------

#[test]
fn test_recovery_signal_is_observation_not_gateway() {
    // spec(INV-SEC-1 / LESSONS §10/§23) — recovery emits one per-session OBSERVATION signal (the §11.4
    // resumed-vs-replayed bit) via the write-actor System-actor path, NOT a Gateway Action*. The
    // orchestration takes a Broker + a dispatch seam + an observation sink and NO WriteHandle/submit
    // dependency — so by construction it can never be a 2nd mutator (Q1=(a), the §16 self-recovery
    // precedent). This test pins the observation emission; the no-Gateway property is structural (the
    // signature carries no Gateway path).
    let a = live_session(None);
    let mut b = live_session(None);
    b.has_scrollback = true;
    b.replayed_event_count = 5;
    let broker = FakeBroker::new();
    let dispatch = RecordingDispatch::default();
    let sink = RecordingSink::default();

    let outcomes = recover_sessions_on_restart(&[a, b], &broker, &dispatch, &sink);

    assert_eq!(outcomes.len(), 2);
    assert_eq!(
        sink.signals.borrow().len(),
        2,
        "exactly one observation signal per recovered session"
    );
}

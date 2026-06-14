//! The detachable-terminal Broker seam (P4.1b-1, brief 059) — does a SURVIVING live in-flight PTY exist
//! for a session after a daemon restart? (B2-strict reattach availability, §8.1). The decision INPUT to
//! the recovery orchestration ([`recover_sessions_on_restart`](super::recovery::recover_sessions_on_restart)).
//!
//! The REAL tmux/abduco-class surviving-PTY holder is a drop-in `Broker` impl at **4.1b-2** (the
//! `SessionLauncher` `TODO(4.1)` swap-point); the LIVE broker-reattach survival is the labelled
//! 0.1/0.3-HITL verify-only follow-on (a live-process property, not deterministically unit-testable).
//! This slice introduces the seam + the deterministic [`FakeBroker`].

use nexusops_shared::ids::SessionId;

/// Whether the detachable-terminal broker still holds a live in-flight PTY for a session — the TOP rung
/// of the §8.1 B2-strict ladder (a surviving same-process turn reattaches without relaunch).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerReattach {
    pub has_live_session: bool,
}

/// The broker seam — queried per session during restart recovery to populate
/// `ResumeInputs::broker_has_live_session`. `Send` (the real impl holds OS handles, driven from the
/// recovery orchestration). The real detachable-terminal impl lands 4.1b-2.
pub trait Broker: Send {
    /// Does a surviving live in-flight PTY still exist for `session_id`?
    fn reattach_outcome(&self, session_id: &SessionId) -> BrokerReattach;
}

/// A deterministic `Broker` for tests — reports a survivor ONLY for the session ids explicitly seeded
/// via [`with_live`](FakeBroker::with_live); every other session has no survivor. `test-support`-gated
/// (the `FakeLauncher`/`FakeHarness` precedent, P4.0b-2 L3).
#[cfg(any(test, feature = "test-support"))]
pub struct FakeBroker {
    live: std::collections::HashSet<SessionId>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeBroker {
    pub fn new() -> Self {
        Self {
            live: std::collections::HashSet::new(),
        }
    }

    /// Seed `session_id` as having a surviving live PTY (the broker reports `has_live_session=true`).
    pub fn with_live(mut self, session_id: &SessionId) -> Self {
        self.live.insert(session_id.clone());
        self
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Default for FakeBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Broker for FakeBroker {
    fn reattach_outcome(&self, session_id: &SessionId) -> BrokerReattach {
        BrokerReattach {
            has_live_session: self.live.contains(session_id),
        }
    }
}

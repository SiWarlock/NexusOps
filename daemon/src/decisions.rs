//! P4.0b-2 L1b (CAT-1) — the per-session **`decision_sink` registry**: the live INV-SEC-1
//! interception transport's pending-decision table (lead call 1 + the MANDATORY dedicated
//! `decision_sink` security pass).
//!
//! A mutating agent tool, intercepted via the live hook, rests at `awaiting_approval` while a human
//! decides. The [`DecisionRegistry`] is the daemon-global table that pairs that pending adjudication
//! action with the per-call [`resolve_verdict`](crate::harness::claude::decision::resolve_verdict)
//! wait: the `intercept` transport [`register`](DecisionRegistry::register)s the action and hands the
//! returned `oneshot::Receiver` to `resolve_verdict`; the `approve`/`deny` path
//! [`resolve`](DecisionRegistry::resolve)s it (firing the §15 #5 audit-gated terminal status → the
//! verdict); and the supervisor [`cancel_session`](DecisionRegistry::cancel_session)s a dead
//! session's pendings.
//!
//! **The two L1 `decision_sink` security carry-forwards this enforces (the dedicated pass verifies):**
//!   (a) **cancel/death → DROP → Deny, fast.** `cancel_session` DROPS the sender (never sends a
//!       value) → `resolve_verdict` resolves the receiver to `Err` → Deny **immediately**, never
//!       hanging to the 5-min wall-clock timeout.
//!   (b) **no re-deliver after the waiter cleaned up.** When a wait ends (timeout / resolved /
//!       cancelled) the waiter [`remove`](DecisionRegistry::remove)s its own entry; a decision that
//!       arrives later finds nothing → a benign no-op (the single-send `oneshot` makes a second
//!       delivery structurally impossible regardless).
//!
//! **Exactly-once / first-terminal-wins** is structural: the `oneshot` sender sends at most once, and
//! `resolve`/`cancel_session`/`remove` each REMOVE the entry, so whichever fires first wins and the
//! rest are no-ops. The registry holds NO write-actor / gateway / DB handle — it is pure in-memory
//! coordination (no INV-SEC-1 mutation), safe to share across the IPC handlers + the supervisor edge.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use tokio::sync::oneshot;

use nexusops_shared::status::ActionRequest as ActionRequestStatus;

use crate::harness::claude::decision::DecisionSignal;

/// One pending agent-mutation decision: the `oneshot` sender that resolves the per-call
/// `resolve_verdict` wait, tagged with the **daemon** `session_id` it belongs to (so a session's
/// death sweeps exactly its own pendings — carry-forward (a)).
struct Pending {
    /// the **daemon** session this decision belongs to — the sweep key `cancel_session` matches on
    /// (carry-forward (a): a dead session drops exactly its own pendings → Deny).
    session_id: String,
    /// the single-send channel that resolves the per-call `resolve_verdict` wait; DROPPING it (not
    /// sending) is the unconditional fail-closed path (`resolve_verdict` sees `Err` → Deny).
    sender: oneshot::Sender<DecisionSignal>,
}

/// The daemon-global registry of pending agent-mutation decisions, keyed by the adjudication
/// `action_request_id`. SHARED (behind one [`Mutex`]) across the `intercept` connection (registers +
/// waits), the `approve`/`deny` connection (resolves), and the supervisor (cancels a dead session's
/// pendings). Every op is an O(1)/O(n) in-memory map mutation — no I/O, no DB; the lock is held only
/// for the map touch, never across a `.await`.
#[derive(Default)]
pub struct DecisionRegistry {
    pending: Mutex<HashMap<String, Pending>>,
}

impl DecisionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lock the table, RECOVERING from a poisoned mutex rather than panicking (a poisoned lock can
    /// only arise if a holder panicked mid-touch — impossible here, the critical sections are
    /// panic-free map ops — but on the cat-1 IPC/supervisor path a recover is strictly safer than an
    /// `unwrap` that could abort a connection/the reaper; the recovered map is still valid, and the
    /// fail-closed semantics hold because a dropped sender always means Deny).
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Pending>> {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Register a pending decision for the adjudication `action_request_id` (resting at
    /// `awaiting_approval`) belonging to daemon `session_id`. Returns the `oneshot::Receiver` the
    /// caller hands to [`resolve_verdict`](crate::harness::claude::decision::resolve_verdict). A
    /// duplicate id — which cannot happen, ids are minted ULIDs — REPLACES the prior entry, dropping
    /// its sender (its waiter then resolves to `Err` → Deny; fail-closed, never a lost wait).
    pub fn register(
        &self,
        action_request_id: String,
        session_id: String,
    ) -> oneshot::Receiver<DecisionSignal> {
        let (sender, rx) = oneshot::channel();
        self.lock().insert(action_request_id, Pending { session_id, sender });
        rx
    }

    /// Resolve a pending decision (the `approve`/`deny` path, post-commit): fire
    /// `DecisionSignal::Resolved(status)` on its sender + REMOVE the entry (exactly-once). A no-op
    /// when the id is not pending — a non-adjudication approve, or a decision already
    /// resolved/cancelled/timed-out (its entry removed) → carry-forward (b), no re-deliver. A send
    /// `Err` (the waiter already gave up — timed out) is benign: the verdict was already Deny.
    pub fn resolve(&self, action_request_id: &str, status: ActionRequestStatus) {
        if let Some(p) = self.lock().remove(action_request_id) {
            let _ = p.sender.send(DecisionSignal::Resolved(status));
        }
    }

    /// Drop the pending decision for `action_request_id` WITHOUT resolving it — the `intercept`
    /// waiter's cleanup after [`resolve_verdict`](crate::harness::claude::decision::resolve_verdict)
    /// returns (idempotent; ensures the table never leaks a stale sender even when the wait ended by
    /// timeout, and guarantees carry-forward (b): a later `resolve` finds nothing).
    pub fn remove(&self, action_request_id: &str) {
        self.lock().remove(action_request_id);
    }

    /// Cancel EVERY pending decision belonging to `session_id` — the supervisor's sweep on session
    /// cancel/death (carry-forward (a)): remove + DROP each matching sender so its
    /// [`resolve_verdict`](crate::harness::claude::decision::resolve_verdict) resolves to `Err` → Deny
    /// **immediately** (never the 5-min timeout). Returns the count cancelled. Dropping (not sending
    /// `Cancelled`) is the unconditional fail-closed path — a dropped `oneshot` is always `Err` → Deny.
    pub fn cancel_session(&self, session_id: &str) -> usize {
        let mut guard = self.lock();
        let before = guard.len();
        // `retain` keeps the OTHER sessions' entries and drops this session's in ONE pass (no
        // intermediate `Vec`); each dropped `Pending`'s sender drops → its receiver sees `Err` → Deny.
        guard.retain(|_, p| p.session_id != session_id);
        before - guard.len()
    }

    /// The number of pending decisions (observability / tests).
    pub fn pending_count(&self) -> usize {
        self.lock().len()
    }
}

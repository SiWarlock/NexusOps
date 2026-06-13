//! P4.0b-2 L1 — the live-interception **decision-wait** (the genuinely-new concurrency surface; lead
//! call 1 + the MANDATORY dedicated `decision_sink` security pass). **FAIL-CLOSED on every non-Allow
//! path**: the human decision races the 5-min wall-clock timeout + session cancel/death; the FIRST
//! terminal wins; only an `Approved`/`PolicyDecided` adjudication terminal opens the gate (`Allow`),
//! everything else → `Deny`. Exactly-once by construction (a single-send `oneshot` + a `FnOnce` sink).
//!
//! The live transport that produces the `DecisionSignal` (the per-session binding fired by the Gateway
//! approve/deny → action-terminal) is L2 (security-reviewed, not unit-tested); this module is the pure
//! decision logic the L1 tests pin deterministically.

use std::time::Duration;

use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use tokio::sync::oneshot;

use super::intercept::verdict_for_status;
use crate::harness::{MutationDecisionSink, MutationVerdict};

/// The terminal signal the verdict-wait resolves on. `Resolved` carries the adjudication action's
/// terminal §6.2 status (the human approved/denied, or a risk-0 auto-terminal); `Cancelled` = the
/// session was cancelled or died during the wait. A DROPPED sender (the per-session binding tore down
/// WITHOUT a signal) resolves the receiver to `Err` → Deny (fail-closed — never an allow from a
/// torn-down wait).
#[derive(Debug)]
pub enum DecisionSignal {
    Resolved(ActionRequestStatus),
    Cancelled,
}

/// Resolve the final [`MutationVerdict`] for an `AwaitingApproval` interception, **FAIL-CLOSED** (lead
/// call 1, LOCKED): race the decision signal against the wall-clock `timeout` (default 5 min). The
/// FIRST terminal wins; every non-Allow outcome → Deny:
///   - `Resolved(Approved | PolicyDecided)` → Allow (the ONLY two allow-terminals; via
///     [`verdict_for_status`], which keys the §15 #5 audit-before-verdict guarantee);
///   - `Resolved(other)` (`Denied`/`Expired`/a non-terminal) → Deny;
///   - `Cancelled` (cancel / session-death signal) → Deny;
///   - the sender DROPPED (the binding torn down) → Deny;
///   - the `timeout` elapsed → Deny.
///
/// Exactly-once: returns ONE verdict; the caller delivers it to the `FnOnce` sink once via
/// [`deliver_once`]. The single-send `oneshot` makes a second decision structurally impossible
/// (first-terminal-wins), and `tokio::time::timeout` resolves on the first of {decision, timeout}.
pub async fn resolve_verdict(
    decision: oneshot::Receiver<DecisionSignal>,
    timeout: Duration,
) -> MutationVerdict {
    match tokio::time::timeout(timeout, decision).await {
        Ok(Ok(DecisionSignal::Resolved(status))) => verdict_for_status(status),
        Ok(Ok(DecisionSignal::Cancelled)) => MutationVerdict::Deny {
            reason: "session cancelled/died during the approval wait — fail-closed".to_string(),
        },
        Ok(Err(_)) => MutationVerdict::Deny {
            reason: "approval channel dropped (session binding torn down) — fail-closed"
                .to_string(),
        },
        Err(_) => MutationVerdict::Deny {
            reason: "approval wait timed out — fail-closed".to_string(),
        },
    }
}

/// Deliver the resolved verdict to the per-session `decision_sink` — invoked EXACTLY once (the sink is
/// `FnOnce`, so a double-delivery is a compile error; this wrapper names the exactly-once contract at
/// the one call site that consumes the sink).
pub fn deliver_once(sink: MutationDecisionSink, verdict: MutationVerdict) {
    sink(verdict);
}

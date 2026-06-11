//! ActionRequest(15) transition guard (§5.1 R-9 — the execution-lifecycle legal edges).
//!
//! `status.rs` froze the value set + the terminal set; the **legal edges** are the Gateway's R-9
//! contract, pinned here. The canonical lifecycle (AG §8): submitted → previewed → policy_decided
//! → awaiting_approval → approved → queued → executing → succeeded. Branch points: policy may deny
//! or (2.2) auto-allow (skip approval → queued); approval may deny/expire; execution may
//! fail/partially-succeed. `cancelled` is reachable from any pre-execution state. Terminal states
//! are sinks. **Rollback edges (succeeded/partially_succeeded → rolled_back/rollback_failed) →
//! 2.4** (no rollback in 2.1b). The Gateway rejects any edge not listed here with a typed
//! `IllegalTransition` — never applies it silently (R-9).

use nexusops_shared::status::ActionRequest;

/// `true` iff `from → to` is a legal ActionRequest(15) edge (§5.1 R-9). A self-edge is not a
/// transition (→ `false`); terminal states + `denied` are sinks (no legal outgoing edge in 2.1b).
pub fn can_transition(from: ActionRequest, to: ActionRequest) -> bool {
    use ActionRequest::*;
    matches!(
        (from, to),
        (Submitted, Previewed | PolicyDecided | Cancelled)
            | (Previewed, PolicyDecided | Cancelled)
            // policy_decided → awaiting_approval (needs approval) | queued (2.2 risk-0 allow) | denied
            | (PolicyDecided, AwaitingApproval | Queued | Denied | Cancelled)
            | (AwaitingApproval, Approved | Denied | Expired | Cancelled)
            | (Approved, Queued | Cancelled)
            | (Queued, Executing | Cancelled)
            | (Executing, Succeeded | Failed | PartiallySucceeded)
    )
}

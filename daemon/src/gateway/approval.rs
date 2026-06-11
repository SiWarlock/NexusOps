//! Approval(10) transition guard (§5.1 R-9 — the human/policy decision-axis legal edges).
//!
//! The split-out decision machine (R-5): an approval is requested, optionally previewed, then sits
//! awaiting_approval until a human/policy resolves it (approved / denied / edited / expired /
//! escalated) or it is cancelled; policy may auto-approve straight from requested. Terminal states
//! are sinks. The Gateway rejects any unlisted edge with a typed error (R-9).

use nexusops_shared::status::Approval;

/// `true` iff `from → to` is a legal Approval(10) edge (§5.1 R-9). Terminal states are sinks; a
/// self-edge is not a transition.
pub fn can_transition(from: Approval, to: Approval) -> bool {
    use Approval::*;
    matches!(
        (from, to),
        (
            Requested,
            Previewed | AwaitingApproval | AutoApprovedByPolicy | Cancelled
        ) | (Previewed, AwaitingApproval | Cancelled)
            | (
                AwaitingApproval,
                Approved | Denied | Edited | Expired | Cancelled | Escalated
            )
    )
}

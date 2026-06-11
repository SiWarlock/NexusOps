//! Approval(10) transition guard (§5.1 R-9 — the human/policy decision-axis legal edges).
//!
//! The split-out decision machine (R-5): an approval is requested, optionally previewed, then sits
//! awaiting_approval until a human/policy resolves it (approved / denied / edited / expired /
//! escalated) or it is cancelled; policy may auto-approve straight from requested. Terminal states
//! are sinks. The Gateway rejects any unlisted edge with a typed error (R-9).

use rusqlite::Transaction;

use nexusops_shared::actions::{ActionRequest as ActionRequestModel, RequiredApprover};
use nexusops_shared::events::ActionApprovalRequested;
use nexusops_shared::gateway_ids::ApprovalId;
use nexusops_shared::status::Approval;

use crate::eventstore::AppendIntent;
use crate::gateway::{db_err, enum_wire, gateway_event_intent, GatewayError};

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

/// INSERT a new `approvals` row at `status` (DATA_MODEL §2.9), with `required_approver` serialized
/// to its JSON column + an optional `expires_at`. `decided_by`/`decided_at` start NULL. Called
/// inside the gateway txn.
pub(crate) fn insert(
    tx: &Transaction,
    approval_id: &ApprovalId,
    action_request_id: &str,
    status: Approval,
    required_approver: &RequiredApprover,
    expires_at: Option<&str>,
    created_at: &str,
) -> Result<(), GatewayError> {
    let approver_json = serde_json::to_string(required_approver)
        .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    tx.execute(
        "INSERT INTO approvals \
         (approval_id, action_request_id, status, required_approver, decided_by, decided_at, \
          expires_at, created_at) \
         VALUES (?1,?2,?3,?4,NULL,NULL,?5,?6)",
        rusqlite::params![
            approval_id.as_str(),
            action_request_id,
            enum_wire(&status)?,
            approver_json,
            expires_at,
            created_at,
        ],
    )
    .map_err(db_err)?;
    Ok(())
}

/// Build the `ActionApprovalRequested` AppendIntent (§7.1/AG§17.1): identity on the envelope
/// columns (`action_request_id` + `approval_id`); the payload echoes `approval_id` for payload-only
/// consumers.
pub(crate) fn approval_requested_intent(
    ar: &ActionRequestModel,
    approval_id: &ApprovalId,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionApprovalRequested {
        approval_id: approval_id.as_str().to_string(),
    })
    .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionApprovalRequested::EVENT_TYPE,
        payload,
        occurred_at,
        Some(approval_id.as_str().to_string()),
    ))
}

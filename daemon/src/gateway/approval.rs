//! Approval(10) transition guard (§5.1 R-9 — the human/policy decision-axis legal edges).
//!
//! The split-out decision machine (R-5): an approval is requested, optionally previewed, then sits
//! awaiting_approval until a human/policy resolves it (approved / denied / edited / expired /
//! escalated) or it is cancelled; policy may auto-approve straight from requested. Terminal states
//! are sinks. The Gateway rejects any unlisted edge with a typed error (R-9).

use rusqlite::{Connection, OptionalExtension, Transaction};

use nexusops_shared::actions::{ActionRequest as ActionRequestModel, RequiredApprover};
use nexusops_shared::events::{
    ActionApprovalRequested, ActionApproved, ActionDenied, ActionExpired,
};
use nexusops_shared::gateway_ids::ApprovalId;
use nexusops_shared::status::Approval;

use crate::eventstore::AppendIntent;
use crate::gateway::{db_err, enum_wire, gateway_event_intent, GatewayError};

/// the loaded `approvals` row the gateway acts on (approve/deny/expire).
pub(crate) struct ApprovalRow {
    pub action_request_id: String,
    pub status: Approval,
    pub expires_at: Option<String>,
}

/// Load an `approvals` row by id (the approve/deny/expire entry point). `NotFound` if absent;
/// fail-closed on a status that no longer parses.
pub(crate) fn load(conn: &Connection, approval_id: &str) -> Result<ApprovalRow, GatewayError> {
    let (action_request_id, status_wire, expires_at): (String, String, Option<String>) = conn
        .query_row(
            "SELECT action_request_id, status, expires_at FROM approvals WHERE approval_id = ?1",
            [approval_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(db_err)?
        .ok_or_else(|| GatewayError::NotFound(format!("approvals {approval_id}")))?;
    let status = serde_json::from_value(serde_json::Value::String(status_wire))
        .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(ApprovalRow {
        action_request_id,
        status,
        expires_at,
    })
}

/// Advance `approvals.status` `from → to` (R-9 guarded, optimistic on the current state). An
/// illegal edge is rejected BEFORE the write; a wrong/missing state → `NotFound`.
pub(crate) fn update_status(
    tx: &Transaction,
    approval_id: &str,
    from: Approval,
    to: Approval,
) -> Result<(), GatewayError> {
    if !can_transition(from, to) {
        return Err(GatewayError::IllegalTransition {
            machine: "Approval",
            from: format!("{from:?}"),
            to: format!("{to:?}"),
        });
    }
    let n = tx
        .execute(
            "UPDATE approvals SET status = ?1 WHERE approval_id = ?2 AND status = ?3",
            rusqlite::params![enum_wire(&to)?, approval_id, enum_wire(&from)?],
        )
        .map_err(db_err)?;
    if n == 0 {
        return Err(GatewayError::NotFound(format!(
            "approvals {approval_id} not at {from:?}"
        )));
    }
    Ok(())
}

/// Stamp the human/policy decision (`decided_by`, `decided_at`) on an approval row.
pub(crate) fn record_decision(
    tx: &Transaction,
    approval_id: &str,
    decided_by: Option<&str>,
    decided_at: &str,
) -> Result<(), GatewayError> {
    tx.execute(
        "UPDATE approvals SET decided_by = ?1, decided_at = ?2 WHERE approval_id = ?3",
        rusqlite::params![decided_by, decided_at, approval_id],
    )
    .map_err(db_err)?;
    Ok(())
}

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

/// Build the `ActionApproved` AppendIntent (a human/policy approved; identity + approval_id on the
/// envelope, `decided_by` echoed in the payload).
pub(crate) fn approved_intent(
    ar: &ActionRequestModel,
    approval_id: &str,
    decided_by: Option<&str>,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionApproved {
        approval_id: approval_id.to_string(),
        decided_by: decided_by.map(|s| s.to_string()),
    })
    .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionApproved::EVENT_TYPE,
        payload,
        occurred_at,
        Some(approval_id.to_string()),
    ))
}

/// Build the `ActionDenied` AppendIntent (terminal; the denial `reason` in the payload).
pub(crate) fn denied_intent(
    ar: &ActionRequestModel,
    approval_id: &str,
    reason: &str,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionDenied {
        approval_id: approval_id.to_string(),
        reason: reason.to_string(),
    })
    .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionDenied::EVENT_TYPE,
        payload,
        occurred_at,
        Some(approval_id.to_string()),
    ))
}

/// Build the `ActionExpired` AppendIntent (the approval lapsed; terminal, never executed).
pub(crate) fn expired_intent(
    ar: &ActionRequestModel,
    approval_id: &str,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionExpired {
        approval_id: approval_id.to_string(),
    })
    .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionExpired::EVENT_TYPE,
        payload,
        occurred_at,
        Some(approval_id.to_string()),
    ))
}

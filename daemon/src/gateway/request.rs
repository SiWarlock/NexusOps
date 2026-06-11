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

use rusqlite::Transaction;

use nexusops_shared::actions::ActionRequest as ActionRequestModel;
use nexusops_shared::events::ActionRequested;
use nexusops_shared::status::ActionRequest;

use crate::eventstore::AppendIntent;
use crate::gateway::{db_err, enum_int, enum_wire, gateway_event_intent, GatewayError};

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

/// INSERT a new `action_requests` row at `status` (DATA_MODEL §2.9). The §6.2 model's
/// resource_refs/inputs/preview serialize to the JSON columns; `risk_level` → its integer; the
/// status/requester_type enums → their snake_case wire strings. Called inside the gateway txn.
pub(crate) fn insert(
    tx: &Transaction,
    ar: &ActionRequestModel,
    status: ActionRequest,
    created_at: &str,
) -> Result<(), GatewayError> {
    let resource_refs_json = serde_json::to_string(&ar.resource_refs)
        .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    let inputs_json =
        serde_json::to_string(&ar.inputs).map_err(|e| GatewayError::Serialize(e.to_string()))?;
    let preview_json = match &ar.preview {
        Some(p) => {
            Some(serde_json::to_string(p).map_err(|e| GatewayError::Serialize(e.to_string()))?)
        }
        None => None,
    };
    tx.execute(
        "INSERT INTO action_requests \
         (action_request_id, project_id, action_type, requester_type, requester_id, \
          resource_refs_json, inputs_json, risk_level, idempotency_key, fencing_token, \
          status, preview_json, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        rusqlite::params![
            ar.action_request_id.as_str(),
            ar.project_id.as_ref().map(|x| x.as_str()),
            ar.action_type,
            enum_wire(&ar.requester_type)?,
            ar.requester_id,
            resource_refs_json,
            inputs_json,
            enum_int(&ar.risk_level)?,
            ar.idempotency_key,
            ar.fencing_token,
            enum_wire(&status)?,
            preview_json,
            created_at,
        ],
    )
    .map_err(db_err)?;
    Ok(())
}

/// Advance `action_requests.status` `from → to` (R-9 guarded). The UPDATE is conditioned on the
/// row currently being at `from` (optimistic — a wrong/missing state → `NotFound`, never a silent
/// no-op); an illegal edge is rejected BEFORE the write (never silently applied).
pub(crate) fn update_status(
    tx: &Transaction,
    action_request_id: &str,
    from: ActionRequest,
    to: ActionRequest,
) -> Result<(), GatewayError> {
    if !can_transition(from, to) {
        return Err(GatewayError::IllegalTransition {
            machine: "ActionRequest",
            from: format!("{from:?}"),
            to: format!("{to:?}"),
        });
    }
    let n = tx
        .execute(
            "UPDATE action_requests SET status = ?1 WHERE action_request_id = ?2 AND status = ?3",
            rusqlite::params![enum_wire(&to)?, action_request_id, enum_wire(&from)?],
        )
        .map_err(db_err)?;
    if n == 0 {
        return Err(GatewayError::NotFound(format!(
            "action_requests {action_request_id} not at {from:?}"
        )));
    }
    Ok(())
}

/// Build the `ActionRequested` AppendIntent (§7.1/AG§17.1): identity on the envelope columns, the
/// request-classification delta in the payload.
pub(crate) fn action_requested_intent(
    ar: &ActionRequestModel,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionRequested {
        action_type: ar.action_type.clone(),
        risk_level: ar.risk_level,
        requester_type: ar.requester_type,
    })
    .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionRequested::EVENT_TYPE,
        payload,
        occurred_at,
        None,
    ))
}

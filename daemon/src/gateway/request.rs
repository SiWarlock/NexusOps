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

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::de::DeserializeOwned;

use nexusops_shared::actions::ActionRequest as ActionRequestModel;
use nexusops_shared::events::{ActionFailed, ActionRequested, ActionStarted, ActionSucceeded};
use nexusops_shared::ids::{ActionRequestId, ProjectId};
use nexusops_shared::status::ActionRequest;
use nexusops_shared::time::Timestamp;

use crate::eventstore::{AppendIntent, GatewayTxn};
use crate::gateway::{db_err, enum_int, enum_wire, gateway_event_intent, GatewayError};

/// Parse a stored wire value (`TEXT`/`INTEGER`) back into its frozen contract type via serde
/// (snake_case strings / integer ranks). Fail-closed with context — a row that no longer parses is
/// a corruption/version error, never a silent default.
fn from_wire<T: DeserializeOwned>(v: serde_json::Value) -> Result<T, GatewayError> {
    serde_json::from_value(v).map_err(|e| GatewayError::Serialize(e.to_string()))
}

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
/// status/requester_type enums → their snake_case wire strings. `plan_id` is the step's parent plan
/// (2.1c, MIGRATION_8) — `None` for a single action. Called inside the gateway txn.
pub(crate) fn insert(
    gtx: &GatewayTxn,
    ar: &ActionRequestModel,
    status: ActionRequest,
    plan_id: Option<&str>,
    created_at: &str,
) -> Result<(), GatewayError> {
    // §15 / rule #4 (the general row-redaction gate): EVERY caller-supplied `action_requests`
    // payload column passes the SAME §15 Redactor as an event payload BEFORE it persists at rest —
    // the proposers (agents / Brain / UI) are UNTRUSTED. Both the open `inputs` blob AND a
    // `resource_ref`'s `uri`/`display_name` (e.g. a token in a git-remote URL) can carry a secret.
    // No-op for clean/keychain-ref data; masks a secret in place; fail-closed if it can't be made
    // `redacted` (§15 ruling Option A; preview_json → 2.3 with the real preview classes).
    let resource_refs_json = gtx.redact_row(
        &serde_json::to_string(&ar.resource_refs)
            .map_err(|e| GatewayError::Serialize(e.to_string()))?,
    )?;
    let inputs_json = gtx.redact_row(
        &serde_json::to_string(&ar.inputs).map_err(|e| GatewayError::Serialize(e.to_string()))?,
    )?;
    let preview_json = match &ar.preview {
        Some(p) => {
            Some(serde_json::to_string(p).map_err(|e| GatewayError::Serialize(e.to_string()))?)
        }
        None => None,
    };
    gtx.tx()
        .execute(
            "INSERT INTO action_requests \
         (action_request_id, project_id, action_type, requester_type, requester_id, \
          resource_refs_json, inputs_json, risk_level, idempotency_key, fencing_token, \
          status, preview_json, plan_id, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
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
                plan_id,
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

/// Reconstruct the frozen §6.2 [`ActionRequestModel`] from its `action_requests` row (the executor
/// and preview consume the full request). `NotFound` if the id is absent. Reads via any
/// `&Connection` (a gateway txn or a read-only conn). Fail-closed: a row that no longer
/// deserializes is an error.
pub(crate) fn load(
    conn: &Connection,
    action_request_id: &str,
) -> Result<ActionRequestModel, GatewayError> {
    // `action_request_id` is intentionally NOT in the SELECT — it's the lookup key, reconstructed
    // from the `action_request_id` parameter below; the column order here mirrors `ActionRow`.
    let row = conn
        .query_row(
            "SELECT project_id, action_type, requester_type, requester_id, resource_refs_json, \
             inputs_json, risk_level, idempotency_key, fencing_token, status, preview_json, \
             created_at FROM action_requests WHERE action_request_id = ?1",
            [action_request_id],
            |r| {
                Ok(ActionRow {
                    project_id: r.get(0)?,
                    action_type: r.get(1)?,
                    requester_type: r.get(2)?,
                    requester_id: r.get(3)?,
                    resource_refs_json: r.get(4)?,
                    inputs_json: r.get(5)?,
                    risk_level: r.get(6)?,
                    idempotency_key: r.get(7)?,
                    fencing_token: r.get(8)?,
                    status: r.get(9)?,
                    preview_json: r.get(10)?,
                    created_at: r.get(11)?,
                })
            },
        )
        .optional()
        .map_err(db_err)?
        .ok_or_else(|| GatewayError::NotFound(format!("action_requests {action_request_id}")))?;

    Ok(ActionRequestModel {
        action_request_id: ActionRequestId::parse(action_request_id)
            .map_err(|e| GatewayError::Serialize(e.to_string()))?,
        project_id: match row.project_id {
            Some(p) => {
                Some(ProjectId::parse(&p).map_err(|e| GatewayError::Serialize(e.to_string()))?)
            }
            None => None,
        },
        action_type: row.action_type,
        requester_type: from_wire(serde_json::Value::String(row.requester_type))?,
        requester_id: row.requester_id,
        resource_refs: serde_json::from_str(&row.resource_refs_json)
            .map_err(|e| GatewayError::Serialize(e.to_string()))?,
        inputs: match row.inputs_json {
            Some(j) => {
                serde_json::from_str(&j).map_err(|e| GatewayError::Serialize(e.to_string()))?
            }
            None => serde_json::Value::Null,
        },
        risk_level: from_wire(serde_json::Value::from(row.risk_level))?,
        idempotency_key: row.idempotency_key,
        fencing_token: row.fencing_token,
        status: from_wire(serde_json::Value::String(row.status))?,
        preview: match row.preview_json {
            Some(j) => {
                Some(serde_json::from_str(&j).map_err(|e| GatewayError::Serialize(e.to_string()))?)
            }
            None => None,
        },
        created_at: Timestamp::parse(&row.created_at)
            .map_err(|e| GatewayError::Serialize(e.to_string()))?,
    })
}

/// the raw `action_requests` columns (pre-reconstruction).
struct ActionRow {
    project_id: Option<String>,
    action_type: String,
    requester_type: String,
    requester_id: String,
    resource_refs_json: String,
    inputs_json: Option<String>,
    risk_level: i64,
    idempotency_key: Option<String>,
    fencing_token: Option<i64>,
    status: String,
    preview_json: Option<String>,
    created_at: String,
}

/// Build the `ActionStarted` AppendIntent (execution began; identity on the envelope).
pub(crate) fn started_intent(
    ar: &ActionRequestModel,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionStarted {})
        .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionStarted::EVENT_TYPE,
        payload,
        occurred_at,
        None,
    ))
}

/// Build the `ActionSucceeded` AppendIntent (execution completed; identity on the envelope).
pub(crate) fn succeeded_intent(
    ar: &ActionRequestModel,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionSucceeded {})
        .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionSucceeded::EVENT_TYPE,
        payload,
        occurred_at,
        None,
    ))
}

/// Build the `ActionFailed` AppendIntent (execution failed; the `error` message in the payload).
pub(crate) fn failed_intent(
    ar: &ActionRequestModel,
    error: &str,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionFailed {
        error: error.to_string(),
    })
    .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionFailed::EVENT_TYPE,
        payload,
        occurred_at,
        None,
    ))
}

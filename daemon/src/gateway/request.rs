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

use nexusops_shared::actions::{ActionError, ActionRequest as ActionRequestModel};
use nexusops_shared::events::{
    ActionFailed, ActionPartiallySucceeded, ActionRequested, ActionStarted, ActionSucceeded,
};
use nexusops_shared::ids::{ActionRequestId, ProjectId};
use nexusops_shared::status::ActionRequest;
use nexusops_shared::time::Timestamp;

use crate::eventstore::{AppendIntent, GatewayTxn};
use crate::gateway::executor::EmittedEvent;
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
            // 2.4 L5: a `queued` crash-orphan reconciles to `failed` (it never executed → no side
            // effect → safe terminal). The crash-reconcile is the only driver of this edge.
            | (Queued, Executing | Cancelled | Failed)
            | (Executing, Succeeded | Failed | PartiallySucceeded)
            // 2.4 — the §5.1 rollback edges: a settled outcome may be rolled back (the rollback seam;
            // the default executor rollback fails closed → rollback_failed). NOT a backdoor to
            // re-execute (no edge back to executing/queued). rolled_back/rollback_failed are sinks.
            | (Succeeded | PartiallySucceeded, RolledBack | RollbackFailed)
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

/// Look up an existing action by its idempotency key — the dedup-on-submit check (2.3 L1). Returns
/// the original action's `(action_request_id, status)` if a row carries this key (the `ux_action_idem`
/// UNIQUE partial index makes the hit at-most-one), else `None`. A keyed re-submit replays the
/// original (at-most-one execution) instead of creating a second row/event/execution. Reads via the
/// open gateway txn (or any `&Connection`).
pub(crate) fn find_by_idempotency_key(
    conn: &Connection,
    idempotency_key: &str,
) -> Result<Option<(String, ActionRequest)>, GatewayError> {
    let row = conn
        .query_row(
            "SELECT action_request_id, status FROM action_requests WHERE idempotency_key = ?1",
            [idempotency_key],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(db_err)?;
    match row {
        Some((id, status)) => Ok(Some((id, from_wire(serde_json::Value::String(status))?))),
        None => Ok(None),
    }
}

/// Scan ALL orphaned action rows (status `executing` or `queued`) for crash-reconcile (2.4 L5). Returns
/// `(action_request_id, status)` per orphan. **plan_id-AGNOSTIC** — a single action and a plan-cascade
/// step are the same orphan to the reconciler (it drives each to terminal BY STATUS, regardless of
/// source). Read over any `&Connection`.
pub(crate) fn scan_orphans(
    conn: &Connection,
) -> Result<Vec<(String, ActionRequest)>, GatewayError> {
    let mut stmt = conn
        .prepare(
            "SELECT action_request_id, status FROM action_requests \
             WHERE status IN ('executing','queued')",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        let (id, status) = r.map_err(db_err)?;
        out.push((id, from_wire(serde_json::Value::String(status))?));
    }
    Ok(out)
}

/// Clear the action's `idempotency_key` (2.4 L5 Q6) — a `queued` orphan definitively never ran (no side
/// effect), so clearing its dedup key makes a re-submit re-runnable (no 2.3 dedup lockout). Conditioned
/// on the row existing (a missing id → `NotFound`, never a silent no-op).
pub(crate) fn clear_idempotency_key(
    tx: &Transaction,
    action_request_id: &str,
) -> Result<(), GatewayError> {
    let n = tx
        .execute(
            "UPDATE action_requests SET idempotency_key = NULL WHERE action_request_id = ?1",
            [action_request_id],
        )
        .map_err(db_err)?;
    if n == 0 {
        return Err(GatewayError::NotFound(format!(
            "action_requests {action_request_id} (clear idempotency_key)"
        )));
    }
    Ok(())
}

/// Persist a generated `ActionPreview` (already serialized + §15-redacted) to the action's
/// `preview_json` (2.3 L2 — was stub-only / NULL). Conditioned on the row existing (a missing id →
/// `NotFound`, never a silent no-op). Called inside the preview `gateway_txn`.
pub(crate) fn update_preview(
    tx: &Transaction,
    action_request_id: &str,
    preview_json: &str,
) -> Result<(), GatewayError> {
    let n = tx
        .execute(
            "UPDATE action_requests SET preview_json = ?1 WHERE action_request_id = ?2",
            rusqlite::params![preview_json, action_request_id],
        )
        .map_err(db_err)?;
    if n == 0 {
        return Err(GatewayError::NotFound(format!(
            "action_requests {action_request_id} (preview persist)"
        )));
    }
    Ok(())
}

/// Persist the acquired fencing token to the action's `fencing_token` column (2.4 L3 — the execute
/// path acquires a lease + binds its minted token here, so a crash reconcile (L5) re-derives the
/// action's lease authority from the row). Conditioned on the row existing (missing → `NotFound`).
pub(crate) fn bind_fencing_token(
    tx: &Transaction,
    action_request_id: &str,
    token: i64,
) -> Result<(), GatewayError> {
    let n = tx
        .execute(
            "UPDATE action_requests SET fencing_token = ?1 WHERE action_request_id = ?2",
            rusqlite::params![token, action_request_id],
        )
        .map_err(db_err)?;
    if n == 0 {
        return Err(GatewayError::NotFound(format!(
            "action_requests {action_request_id} (fencing-token bind)"
        )));
    }
    Ok(())
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

/// Build the AppendIntent for an executor's additional in-txn event (PIN a/b, P4.0b-1). A
/// `SessionStarted` rides the action's envelope context (actor / project / correlation, via
/// [`gateway_event_intent`]) but OVERRIDES `session_id` with the minted session id — so the session's
/// identity lives on the ENVELOPE column the `proj_session` projector reads, AND the event stays
/// correlatable to the audited `session.create` action (it carries both `session_id` + the action's
/// `correlation_id`/`action_request_id`). Appended in txn-B, atomic with `ActionSucceeded`.
pub(crate) fn emitted_event_intent(
    ar: &ActionRequestModel,
    event: &EmittedEvent,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    match event {
        EmittedEvent::SessionStarted {
            session_id,
            payload,
        } => {
            let payload_json = serde_json::to_string(payload)
                .map_err(|e| GatewayError::Serialize(e.to_string()))?;
            let mut intent =
                gateway_event_intent(ar, "SessionStarted", payload_json, occurred_at, None);
            intent.session_id = Some(session_id.clone());
            Ok(intent)
        }
        // The generic edges family (Q1=B) — the executor already serialized its frozen event struct
        // (it owns the payload + handled the serde fault → `Failed`). Append it riding the action's
        // envelope identity (`project_id`/`correlation_id`/`action_request_id` from `ar`), through the
        // §15 gate, with no envelope-column override.
        EmittedEvent::Namespaced {
            event_type,
            payload_json,
        } => Ok(gateway_event_intent(
            ar,
            event_type,
            payload_json.clone(),
            occurred_at,
            None,
        )),
    }
}

/// Build the `ActionPartiallySucceeded` AppendIntent (§17 — a side effect was applied but its terminal
/// event could not be written; L2 best-effort record). The `reason` is a redaction-safe STRUCTURAL
/// string ONLY — never row/payload content (§15; the L2 security obligation).
pub(crate) fn partially_succeeded_intent(
    ar: &ActionRequestModel,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionPartiallySucceeded {
        reason: "side effect applied; the terminal ActionSucceeded event could not be written \
                 (§17 fail-closed audit-write)"
            .to_string(),
    })
    .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionPartiallySucceeded::EVENT_TYPE,
        payload,
        occurred_at,
        None,
    ))
}

/// Build the `ActionFailed` AppendIntent (execution failed; the structured [`ActionError`] in the
/// payload — 2.4 typed taxonomy). The pre-2.4 free-string failure maps to `ExecutorError{message}`.
pub(crate) fn failed_intent(
    ar: &ActionRequestModel,
    error: ActionError,
    occurred_at: &str,
) -> Result<AppendIntent, GatewayError> {
    let payload = serde_json::to_string(&ActionFailed { error })
        .map_err(|e| GatewayError::Serialize(e.to_string()))?;
    Ok(gateway_event_intent(
        ar,
        ActionFailed::EVENT_TYPE,
        payload,
        occurred_at,
        None,
    ))
}

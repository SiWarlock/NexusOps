//! JSON-RPC method dispatch + the §6.1 read methods (`get_projection` / `get_capabilities`).
//!
//! Reads go over a **read-only WAL** connection ([`crate::eventstore::open_read_only`]) — the
//! write-actor stays the sole writer (Forbidden #3 / LESSON §3); this layer never opens a
//! writable `Connection`. **Client errors** (unknown method, bad params) are structured
//! `WireError` responses (the connection continues); **infrastructure errors** (a failed read)
//! disconnect via [`IpcError`].

use std::path::Path;

use rusqlite::types::ValueRef;
use rusqlite::Connection;

use nexusops_shared::actions::{ActionPlan, ActionRequest};
use nexusops_shared::ipc::{
    Capabilities, GetProjectionParams, IpcErrorCode, ProjectionName, RpcRequest, RpcResponse,
    SubscribeParams, WireError,
};

use super::IpcError;
use crate::gateway::GatewayError;
use crate::runtime::WriteHandle;

/// The `proj_*` table backing each §6.1 projection name (the §7 registry → DATA_MODEL §2.3 map).
/// The mapped name is a compile-time constant — never client input — so it is safe to interpolate
/// into the read query (no SQL-injection surface). `UsageLedger`→`proj_usage_ledger` is the Q3
/// reconcile; `ProjectGraph` returns its node table (edges are a richer query, deferred).
fn projection_table(name: ProjectionName) -> &'static str {
    match name {
        ProjectionName::ProjectActivity => "proj_project_activity",
        ProjectionName::Session => "proj_session",
        ProjectionName::ApprovalQueue => "proj_approval_queue",
        ProjectionName::Worktree => "proj_worktree",
        ProjectionName::PullRequest => "proj_pull_request",
        ProjectionName::PlanProgress => "proj_plan_progress",
        ProjectionName::ProjectGraph => "proj_graph_node",
        ProjectionName::AgentTeam => "proj_agent_team",
        ProjectionName::AuditTrail => "proj_audit_trail",
        ProjectionName::UsageLedger => "proj_usage_ledger",
    }
}

/// Dispatch one §6.1 request → a response. A client error (unknown method / bad params) becomes
/// a structured `WireError` response (the loop continues); an infrastructure read failure is an
/// `Err(IpcError)` (the connection disconnects). Reads never mutate — no Action, no event.
pub(crate) fn dispatch(
    req: &RpcRequest,
    db_path: &Path,
    write: &WriteHandle,
) -> Result<RpcResponse, IpcError> {
    let outcome: Result<serde_json::Value, IpcErrorCode> = match req.method.as_str() {
        "get_capabilities" => Ok(capabilities_value()),
        "get_projection" => get_projection(&req.params, db_path)?,
        "subscribe" => subscribe_ack(&req.params),
        // §6.1 mutation methods (2.1b) — run the Gateway pipeline on the write-actor (the sole
        // mutator, forbidden #2/#3). A `GatewayError` → a structured `IpcErrorCode` response; the
        // write-actor being gone is an infra failure → `Err(IpcError)` (disconnect).
        "submit_action" => submit_action(&req.params, write)?,
        "submit_action_plan" => submit_action_plan(&req.params, write)?,
        "approve" => approve(&req.params, write)?,
        "deny" => deny(&req.params, write)?,
        "preview_action" => preview_action(&req.params, write)?,
        _ => Err(IpcErrorCode::UnknownMethod),
    };
    Ok(match outcome {
        Ok(value) => RpcResponse {
            id: req.id,
            result: Some(value),
            error: None,
        },
        Err(code) => RpcResponse {
            id: req.id,
            result: None,
            error: Some(WireError { code }),
        },
    })
}

/// `subscribe`: RECOGNIZE + validate the subscription (the projection name) and ack it. The live
/// delta stream over the connection (`push_subscription` fed by the EventStore broadcast) is
/// **1.6-wired** with the accept-loop; 1.5 acks so a client knows the subscription registered.
/// A malformed params is a client protocol violation (`protocol_error`).
fn subscribe_ack(params: &serde_json::Value) -> Result<serde_json::Value, IpcErrorCode> {
    let params: SubscribeParams =
        serde_json::from_value(params.clone()).map_err(|_| IpcErrorCode::ProtocolError)?;
    // echo the (validated) projection name; the delta stream follows once the 1.6 runtime wires it.
    // ProjectionName is a unit-enum variant → serialization is infallible.
    let name = serde_json::to_value(params.projection)
        .expect("ProjectionName serializes to a JSON string infallibly");
    Ok(serde_json::json!({ "subscribed": name }))
}

// --- §6.1 mutation methods (2.1b) — parse params, run the Gateway pipeline on the write-actor ---

/// Map a write-actor gateway result → the dispatch's (structured-response | infra-disconnect) form:
/// a `GatewayError` becomes a structured `IpcErrorCode` (the connection continues); the write-actor
/// being gone is infrastructure failure → `Err(IpcError)` (disconnect).
fn gateway_result<T: serde::Serialize>(
    r: Result<Result<T, GatewayError>, crate::runtime::RuntimeError>,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    match r {
        Err(_) => Err(IpcError::Read("write-actor unavailable".to_string())),
        Ok(Err(ge)) => Ok(Err(gateway_error_to_code(&ge))),
        // infallible: an ActionAck/ActionPreview always serializes to a JSON value.
        Ok(Ok(value)) => Ok(Ok(
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
        )),
    }
}

/// Map a daemon-internal [`GatewayError`] → the closed §6.4 `IpcErrorCode` set.
fn gateway_error_to_code(e: &GatewayError) -> IpcErrorCode {
    match e {
        GatewayError::PolicyDenied
        | GatewayError::UnsupportedPolicyDecision(_)
        // a Blocked-mode plan submission is rejected on policy grounds (Blocked is a 2.2-assigned
        // outcome, not a submittable mode) — the request parses fine, so it is policy_denied, not
        // a protocol_error.
        | GatewayError::UnsupportedApprovalMode(_) => IpcErrorCode::PolicyDenied,
        // out-of-state action / missing-or-lapsed approval / fail-closed audit write — the
        // mutation's precondition no longer holds (§6.4 precondition_stale). (Q7 carry-forward: the
        // AuditWriteFailed→internal_error correction lands with L5's UnknownOutcome→internal_error.)
        GatewayError::IllegalTransition { .. }
        | GatewayError::ApprovalExpired
        | GatewayError::NotFound(_)
        | GatewayError::AuditWriteFailed(_) => IpcErrorCode::PreconditionStale,
        // 2.4 L3 — a stale fencing token: the NEVER-auto-resolved hard-conflict card (rule #6),
        // distinct from the re-approvable precondition_stale (the Q7/§11.5 safety-card distinction).
        GatewayError::FencingConflict => IpcErrorCode::FencingConflict,
        GatewayError::Serialize(_) => IpcErrorCode::ProtocolError,
    }
}

/// `submit_action` — parse the §6.2 `ActionRequest`, run the pipeline → `ActionAck`.
fn submit_action(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let req: ActionRequest = match serde_json::from_value(params.clone()) {
        Ok(r) => r,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    gateway_result(write.submit_action_blocking(req))
}

/// `submit_action_plan` — parse the §6.2 `ActionPlan`, run the plan pipeline → `PlanAck` (O-3).
fn submit_action_plan(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let plan: ActionPlan = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    gateway_result(write.submit_action_plan_blocking(plan))
}

/// a required string field from the JSON-RPC params (a missing/non-string field is a client
/// protocol violation → `protocol_error`).
fn str_param<'a>(params: &'a serde_json::Value, key: &str) -> Result<&'a str, IpcErrorCode> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or(IpcErrorCode::ProtocolError)
}

/// `approve` — `{approval_id, step_id?}` (§6.1). Resolves the approval: a per-step / single-action
/// approval drives its action to succeeded/failed; a plan-level approve-all approval cascades over
/// the plan's non-critical steps (2.1c). The optional `step_id` is accepted at the §6.1 boundary but
/// RESERVED in 2.1c — `approve` resolves the WHOLE approval (targeting a single step of a plan-level
/// approval is a later refinement); an unrecognized field is ignored, not rejected.
fn approve(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let approval_id = match str_param(params, "approval_id") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    gateway_result(write.approve_blocking(approval_id))
}

/// `deny` — `{approval_id, reason}`.
fn deny(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let approval_id = match str_param(params, "approval_id") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    let reason = match str_param(params, "reason") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    gateway_result(write.deny_blocking(approval_id, reason))
}

/// `preview_action` — `{action_request_id}` → the catalog-class `ActionPreview` (2.3 L2).
fn preview_action(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let action_request_id = match str_param(params, "action_request_id") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    gateway_result(write.preview_action_blocking(action_request_id))
}

fn capabilities_value() -> serde_json::Value {
    let caps = Capabilities {
        protocol_version: nexusops_shared::ipc::PROTOCOL_VERSION,
        contract_version: nexusops_shared::CONTRACT_VERSION.to_string(),
    };
    // infallible: a struct of a u32 + a String always serializes to a JSON object.
    serde_json::to_value(caps).unwrap_or(serde_json::Value::Null)
}

/// `get_projection`: read the named projection's rows over a read-only WAL connection. The outer
/// `Result` is the infra/client split — `Err(IpcError)` (disconnect) on a read failure, inner
/// `Err(IpcErrorCode)` (structured response) on bad params.
fn get_projection(
    params: &serde_json::Value,
    db_path: &Path,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    // bad/malformed params is a CLIENT protocol violation → `protocol_error` (a structured
    // response; the §6.4 code added per the lead-ratified gap resolution). Distinct from an
    // unknown METHOD name, which stays `unknown_method`.
    let params: GetProjectionParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    let table = projection_table(params.name);
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|e| IpcError::Read(e.to_string()))?;
    let rows = read_table_as_json(&conn, table)?;
    Ok(Ok(rows))
}

/// `SELECT *` a projection table → a JSON array (one object per row). `table` is the compile-time
/// constant from [`projection_table`] (never client input). An unfed projection's table exists
/// (created by the 1.2 migration) but is empty → an empty array, not an error.
fn read_table_as_json(conn: &Connection, table: &str) -> Result<serde_json::Value, IpcError> {
    let sql = format!("SELECT * FROM {table}");
    let mut stmt = conn.prepare(&sql).map_err(read_err)?;
    let col_count = stmt.column_count();
    let cols: Vec<String> = stmt.column_names().iter().map(|c| c.to_string()).collect();
    let rows = stmt
        .query_map([], |row| {
            let mut obj = serde_json::Map::with_capacity(col_count);
            for (i, name) in cols.iter().enumerate() {
                obj.insert(name.clone(), sqlite_to_json(row.get_ref(i)?));
            }
            Ok(serde_json::Value::Object(obj))
        })
        .map_err(read_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(read_err)?);
    }
    Ok(serde_json::Value::Array(out))
}

fn sqlite_to_json(v: ValueRef) -> serde_json::Value {
    match v {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Integer(i) => serde_json::Value::from(i),
        ValueRef::Real(f) => serde_json::json!(f),
        ValueRef::Text(t) => serde_json::Value::String(String::from_utf8_lossy(t).into_owned()),
        // proj_* tables hold no BLOBs; surface a placeholder rather than raw bytes (defensive).
        ValueRef::Blob(b) => serde_json::Value::String(format!("<{} bytes>", b.len())),
    }
}

fn read_err(e: rusqlite::Error) -> IpcError {
    IpcError::Read(e.to_string())
}

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

use nexusops_shared::ipc::{
    Capabilities, GetProjectionParams, IpcErrorCode, ProjectionName, RpcRequest, RpcResponse,
    SubscribeParams, WireError,
};

use super::IpcError;

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
pub(crate) fn dispatch(req: &RpcRequest, db_path: &Path) -> Result<RpcResponse, IpcError> {
    let outcome: Result<serde_json::Value, IpcErrorCode> = match req.method.as_str() {
        "get_capabilities" => Ok(capabilities_value()),
        "get_projection" => get_projection(&req.params, db_path)?,
        "subscribe" => subscribe_ack(&req.params),
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

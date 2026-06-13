//! JSON-RPC method dispatch + the §6.1 read methods (`get_projection` / `get_capabilities`).
//!
//! Reads go over a **read-only WAL** connection ([`crate::eventstore::open_read_only`]) — the
//! write-actor stays the sole writer (Forbidden #3 / LESSON §3); this layer never opens a
//! writable `Connection`. **Client errors** (unknown method, bad params) are structured
//! `WireError` responses (the connection continues); **infrastructure errors** (a failed read)
//! disconnect via [`IpcError`].

use std::path::Path;
use std::time::Duration;

use rusqlite::types::ValueRef;
use rusqlite::Connection;

use nexusops_shared::actions::{
    ActionPlan, ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::ids::{ActionRequestId, ProjectId};
use nexusops_shared::ipc::{
    Capabilities, GetProjectionParams, IpcErrorCode, ProjectionName, RpcRequest, RpcResponse,
    SubscribeParams, WireError,
};
use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use nexusops_shared::time::Timestamp;

use super::IpcError;
use crate::decisions::DecisionRegistry;
use crate::gateway::GatewayError;
use crate::harness::claude::decision::resolve_verdict;
use crate::harness::claude::intercept::{HookPayload, InterceptOutcome};
use crate::harness::MutationVerdict;
use crate::runtime::WriteHandle;

/// The §6.2 wall-clock approval-wait for an intercepted mutating agent tool (call 1, LOCKED default
/// ~5 min; fail-closed on timeout/cancel/death). The agent's `PreToolUse` hook blocks for up to this
/// long while the human decides; every non-Allow terminal → Deny.
const APPROVAL_WAIT: Duration = Duration::from_secs(300);

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
    registry: &DecisionRegistry,
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
        // approve/deny ALSO fire the per-session decision_sink (resolve a pending agent-mutation
        // interception waiting on this approval) — the registry is a no-op for a non-adjudication.
        "approve" => approve(&req.params, write, registry)?,
        "deny" => deny(&req.params, write, registry)?,
        "preview_action" => preview_action(&req.params, write)?,
        // P4.0b-2 C2 (CAT-1) — the reachable session.create (the UI/IPC live-launch path) + the live
        // INV-SEC-1 interception (the Claude PreToolUse hook → adjudication → verdict, with the
        // per-session decision_sink wait). These make a live agent reachable WITH the interception
        // (the call-5 atomicity is at the main.rs register/swap; pinned by the inverted guard).
        "session.create" => session_create(&req.params, write)?,
        "intercept" => intercept(&req.params, write, registry)?,
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
        // out-of-state action / missing-or-lapsed approval / fail-closed audit write / a stale live
        // source (2.4 L4) — the mutation's precondition no longer holds (§6.4 precondition_stale, the
        // re-approvable stale card). (Q7 carry-forward: the AuditWriteFailed→internal_error correction
        // lands with L5's UnknownOutcome→internal_error.)
        GatewayError::IllegalTransition { .. }
        | GatewayError::ApprovalExpired
        | GatewayError::NotFound(_)
        | GatewayError::AuditWriteFailed(_)
        | GatewayError::StalePrecondition => IpcErrorCode::PreconditionStale,
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
    registry: &DecisionRegistry,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let approval_id = match str_param(params, "approval_id") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    let result = write.approve_blocking(approval_id);
    fire_decision_sink(registry, &result);
    gateway_result(result)
}

/// `deny` — `{approval_id, reason}`.
fn deny(
    params: &serde_json::Value,
    write: &WriteHandle,
    registry: &DecisionRegistry,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let approval_id = match str_param(params, "approval_id") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    let reason = match str_param(params, "reason") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    let result = write.deny_blocking(approval_id, reason);
    fire_decision_sink(registry, &result);
    gateway_result(result)
}

/// Fire the per-session decision_sink for a resolved approve/deny (C2). If the resolved action is an
/// agent-mutation adjudication a live `PreToolUse` hook is awaiting, the registry delivers its §6.2
/// terminal status → the waiting `resolve_verdict` yields the verdict (Approved→Allow / Denied→Deny).
/// A no-op for a non-adjudication approval (its id was never registered) — safe to call always. Only
/// fires on a committed Ok (a failed approve/deny leaves the action — and the wait — untouched).
fn fire_decision_sink(
    registry: &DecisionRegistry,
    result: &Result<
        Result<nexusops_shared::ipc::ActionAck, GatewayError>,
        crate::runtime::RuntimeError,
    >,
) {
    if let Ok(Ok(ack)) = result {
        registry.resolve(&ack.action_request_id, ack.status);
    }
}

/// `session.create` — the reachable UI/IPC live-launch path (C2). The daemon builds the
/// `session.create` `ActionRequest` server-side with `requester_type = User` (PIN e — UI/IPC-initiated
/// ONLY; an agent path is denied), the project as the catalog-required resource_ref, and the optional
/// `execution_profile_id` in inputs (the §15 #8 binding records it at start). Risk is recorded-not-
/// trusted (the §6.3 catalog reconciles it to the authoritative risk-0 at submit, LESSON §19). The
/// risk-0 auto-execute path then drives the `SessionExecutor` → the live `ClaudeAdapter` launch.
fn session_create(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let project_id = match str_param(params, "project_id") {
        Ok(s) => s.to_string(),
        Err(c) => return Ok(Err(c)),
    };
    let inputs = match params.get("execution_profile_id").and_then(|v| v.as_str()) {
        Some(p) => serde_json::json!({ "execution_profile_id": p }),
        None => serde_json::json!({}),
    };
    let req = ActionRequest {
        action_request_id: ActionRequestId::new(),
        // the envelope project_id (Option) — None if it doesn't parse; the resource_ref carries the
        // raw id for the audit either way.
        project_id: ProjectId::parse(&project_id).ok(),
        action_type: "session.create".to_string(),
        // the daemon SETS the requester (NOT client-trusted) — UI/IPC is `User` (PIN e); no agent path.
        requester_type: RequesterType::User,
        requester_id: "ui".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Project,
            id: project_id,
            uri: None,
        }],
        inputs,
        // recorded-not-trusted (§15) — `CatalogPolicy` overwrites to the authoritative locked_risk.
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        // a placeholder — `request::insert` stamps `created_at` from the daemon Clock (this constant
        // always parses); never gates anything.
        created_at: Timestamp::parse("1970-01-01T00:00:00Z")
            .expect("placeholder created_at — insert stamps the real daemon-clock time"),
    };
    gateway_result(write.submit_action_blocking(req))
}

/// `intercept` — the live INV-SEC-1 interception transport (C2, CAT-1). The Claude `PreToolUse` hook
/// (via the `nexusopsd hook` subcommand over UDS) pipes the tool call here; the daemon routes it
/// through the Gateway on the write-actor (the adjudication ActionRequest commits — audit-before-
/// verdict; an audit-fault raises the §17 alarm). A `Resolved` verdict returns NOW (a risk-0 auto-
/// allow, or any deny); a mutating tool rests at `awaiting_approval` and this handler WAITS (the
/// per-session `decision_sink`) for the human's approve/deny up to [`APPROVAL_WAIT`] — **fail-closed**
/// on timeout/cancel/session-death (every non-Allow terminal → Deny). The verdict is returned as
/// `{decision, reason}`; the hook subcommand translates it to Claude's hook output.
fn intercept(
    params: &serde_json::Value,
    write: &WriteHandle,
    registry: &DecisionRegistry,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let payload: HookPayload = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    // the daemon session id (the hook subcommand set `session_id` from NEXUSOPS_SESSION_ID) — the
    // decision_sink key. UNTRUSTED input, usable ONLY as a drop-only cancel_session predicate (a spoof
    // can only Deny another session's pendings, never Allow — the wait keys on `action_request_id`).
    let session_id = payload.session_id.clone();

    // route on the write-actor (the adjudication commits; the §17 alarm fires there on audit-fault).
    let outcome = match write.intercept_blocking(payload) {
        Ok(o) => o,
        // the write-actor being gone is infra failure → disconnect (the hook then fails closed).
        Err(_) => return Err(IpcError::Read("write-actor unavailable".to_string())),
    };

    let verdict = match outcome {
        // the verdict is known NOW (a risk-0 auto-allow, or any fail-closed deny).
        InterceptOutcome::Resolved(v) => v,
        // a mutating tool rests at awaiting_approval — register the per-session decision_sink + WAIT
        // for the human. The async `resolve_verdict` (5-min wall-clock + cancel/death) is bridged to
        // this sync handler via a tokio task + a std channel — NO `block_on` (this runs on a blocking
        // serve thread; `block_on` from one is unsound). The connection (+ its semaphore permit) is
        // held for the wait, bounded by MAX_CONNECTIONS (the dedicated-connection pattern, like subscribe).
        InterceptOutcome::AwaitingApproval { action_request_id } => {
            // the runtime to spawn the async `resolve_verdict` wait on. The production serve thread is
            // a `spawn_blocking` task → the runtime context IS present. If absent (off the accept loop)
            // there is no safe way to wait → fail closed (Deny). `try_current` never panics.
            let rt = match tokio::runtime::Handle::try_current() {
                Ok(h) => h,
                Err(_) => {
                    return Ok(Ok(verdict_response(&MutationVerdict::Deny {
                        reason: "no async runtime for the approval wait — fail-closed".to_string(),
                    })))
                }
            };
            let decision = registry.register(action_request_id.clone(), session_id);
            let (vtx, vrx) = std::sync::mpsc::sync_channel::<MutationVerdict>(1);
            rt.spawn(async move {
                let v = resolve_verdict(decision, APPROVAL_WAIT).await;
                let _ = vtx.send(v);
            });
            // block this serve thread on the bridge; a dropped bridge sender (the spawned task died)
            // fails closed to Deny. The waiter then REMOVES its registry entry (carry-forward b — a
            // late approve/deny finds nothing, no re-deliver).
            let v = vrx.recv().unwrap_or_else(|_| MutationVerdict::Deny {
                reason: "decision bridge dropped — fail-closed".to_string(),
            });
            registry.remove(&action_request_id);
            v
        }
    };
    Ok(Ok(verdict_response(&verdict)))
}

/// The `intercept` verdict as the JSON the `nexusopsd hook` subcommand consumes + translates to
/// Claude's `PreToolUse` hook output. Content-free reason (§15).
fn verdict_response(verdict: &MutationVerdict) -> serde_json::Value {
    match verdict {
        MutationVerdict::Allow => serde_json::json!({ "decision": "allow" }),
        MutationVerdict::Deny { reason } => {
            serde_json::json!({ "decision": "deny", "reason": reason })
        }
    }
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

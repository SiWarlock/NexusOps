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
    Capabilities, DiffResult, GetDiffParams, GetProjectionParams, IpcErrorCode, ProjectionName,
    RpcRequest, RpcResponse, SubscribeParams, WireError,
};
use nexusops_shared::projections::{ApprovalQueueRow, PullRequestRow, ReviewRow, SessionRow};
use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use nexusops_shared::time::Timestamp;

use super::IpcError;
use crate::decisions::DecisionRegistry;
use crate::gateway::GatewayError;
use crate::harness::claude::decision::resolve_verdict;
use crate::harness::claude::intercept::{HookPayload, InterceptOutcome};
use crate::harness::MutationVerdict;
use crate::runtime::{InterceptWaitClass, WriteHandle};

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
        ProjectionName::Review => "proj_review",
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
    wait_class: &InterceptWaitClass,
) -> Result<RpcResponse, IpcError> {
    let outcome: Result<serde_json::Value, IpcErrorCode> = match req.method.as_str() {
        "get_capabilities" => Ok(capabilities_value()),
        "get_projection" => get_projection(&req.params, db_path)?,
        // P4.0b-ui1 — the §6.1 hunk-structured diff READ (the ui-6.3e source). Resolves
        // worktree_id→proj_worktree.path (read-only WAL) then reads git2 LIVE read-only; NO mutation.
        "get_diff" => get_diff(&req.params, db_path)?,
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
        "intercept" => intercept(&req.params, write, registry, wait_class)?,
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
        // P4.0b-2c — the audit-backbone breaker is latched (systemic audit failure). The mutation is
        // refused before any audit-write; surfaced as the §6.4 fail-closed `internal_error` (the loud
        // distinguishable signal is the durable systemic alarm + the latched breaker state).
        GatewayError::AuditBackboneDown => IpcErrorCode::InternalError,
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

/// `session.create` — the reachable UI/IPC live-launch path (C2). Builds the server-side
/// `ActionRequest` ([`build_session_create_request`]) then runs the pipeline. The risk-0 auto-execute
/// path drives the `SessionExecutor` → the live `ClaudeAdapter` launch.
fn session_create(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let req = match build_session_create_request(params) {
        Ok(r) => r,
        Err(c) => return Ok(Err(c)),
    };
    gateway_result(write.submit_action_blocking(req))
}

/// Build the `session.create` `ActionRequest` server-side from the IPC params (extracted so the
/// param→inputs thread is unit-testable without a `WriteHandle`). The daemon SETS `requester_type =
/// User` (PIN e — UI/IPC-initiated ONLY; an agent path is denied), the project as the catalog-required
/// resource_ref, and the optional `execution_profile_id` (the §15 #8 binding records it at start) +
/// the optional `initial_prompt` (the Option-G dev-drive, brief 053) in inputs. Risk is recorded-not-
/// trusted (the §6.3 catalog reconciles it to the authoritative risk-0 at submit, LESSON §19).
/// `project_id` is required (the catalog resource_ref) → a missing one is a client protocol violation.
fn build_session_create_request(params: &serde_json::Value) -> Result<ActionRequest, IpcErrorCode> {
    let project_id = str_param(params, "project_id")?.to_string();
    // build inputs from the optional params present (both are ad-hoc JSON — NO frozen `shared/` type,
    // NO CONTRACT bump; same handling as the existing `execution_profile_id`).
    let mut inputs = serde_json::Map::new();
    if let Some(p) = params.get("execution_profile_id").and_then(|v| v.as_str()) {
        inputs.insert(
            "execution_profile_id".to_string(),
            serde_json::Value::String(p.to_string()),
        );
    }
    if let Some(prompt) = params.get("initial_prompt").and_then(|v| v.as_str()) {
        inputs.insert(
            "initial_prompt".to_string(),
            serde_json::Value::String(prompt.to_string()),
        );
    }
    Ok(ActionRequest {
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
        inputs: serde_json::Value::Object(inputs),
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
    })
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
    wait_class: &InterceptWaitClass,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let payload: HookPayload = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    // the daemon session id (the hook subcommand set `session_id` from NEXUSOPS_SESSION_ID) — the
    // decision_sink key. UNTRUSTED input, usable ONLY as a drop-only cancel_session predicate (a spoof
    // can only Deny another session's pendings, never Allow — the wait keys on `action_request_id`).
    let session_id = payload.session_id.clone();

    // route on the write-actor (the adjudication commits + is audited FIRST — audit-before-verdict,
    // §15 #5; the §17 alarm fires there on an audit-fault). The wait-class gate is AFTER this, so an
    // exhausted-class intercept's attempt is still AUDITED, then fail-closed-denied (F2).
    let outcome = match write.intercept_blocking(payload) {
        Ok(o) => o,
        // the write-actor being gone is infra failure → disconnect (the hook then fails closed).
        Err(_) => return Err(IpcError::Read("write-actor unavailable".to_string())),
    };

    // F2 — the wait-class permit class (§6.4/§10). A `Resolved` verdict (risk-0 auto-allow / any deny)
    // returns immediately, touching no permit. An `AwaitingApproval` mutating tool tries to PARK in the
    // intercept-wait class: saturated → fail-closed Deny WITHOUT entering the wait (no register, no
    // bridge); a slot acquired → register the per-session decision_sink + WAIT for the human (the
    // permit held across the wait, released on EVERY terminal — verdict/timeout/cancel/death/bridge-drop).
    let verdict = intercept_verdict_with_wait_class(outcome, wait_class, |action_request_id| {
        // the runtime to spawn the async `resolve_verdict` wait on. The production serve thread is a
        // `spawn_blocking` task → the runtime context IS present. If absent (off the accept loop) there
        // is no safe way to wait → fail closed (Deny). `try_current` never panics.
        let rt = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                return MutationVerdict::Deny {
                    reason: "no async runtime for the approval wait — fail-closed".to_string(),
                }
            }
        };
        let decision = registry.register(action_request_id.clone(), session_id);
        let (vtx, vrx) = std::sync::mpsc::sync_channel::<MutationVerdict>(1);
        rt.spawn(async move {
            let v = resolve_verdict(decision, APPROVAL_WAIT).await;
            let _ = vtx.send(v);
        });
        // block this serve thread on the bridge; a dropped bridge sender (the spawned task died)
        // fails closed to Deny. The waiter then REMOVES its registry entry (carry-forward b — a late
        // approve/deny finds nothing, no re-deliver).
        let v = vrx.recv().unwrap_or_else(|_| MutationVerdict::Deny {
            reason: "decision bridge dropped — fail-closed".to_string(),
        });
        registry.remove(&action_request_id);
        v
    });
    Ok(Ok(verdict_response(&verdict)))
}

/// The fail-closed reason on intercept-wait class exhaustion (§6.4/§10, F2) — content-free + DISTINCT
/// from the timeout Deny so the operator can tell "saturated" from "timed out".
pub const INTERCEPT_SATURATED_REASON: &str =
    "approval capacity saturated — fail-closed (try again)";

/// The F2 wait-class decision (the pure, testable core of [`intercept`]). A `Resolved` verdict touches
/// NO wait-class permit (the gate semantics are unchanged). An `AwaitingApproval` mutating tool tries
/// to PARK: saturated (`try_park`→None) → fail-closed Deny ([`INTERCEPT_SATURATED_REASON`], **the wait
/// is NEVER entered** — `register_and_wait` is not called, no bypass); a slot acquired → hold the permit
/// across `register_and_wait(action_request_id)` (the real register+bridge+remove), released on the
/// return (every terminal path) via the `OwnedSemaphorePermit` RAII. INV-SEC-1 preserved (fail-safe).
pub fn intercept_verdict_with_wait_class<F>(
    outcome: InterceptOutcome,
    wait_class: &InterceptWaitClass,
    register_and_wait: F,
) -> MutationVerdict
where
    F: FnOnce(String) -> MutationVerdict,
{
    match outcome {
        InterceptOutcome::Resolved(v) => v,
        InterceptOutcome::AwaitingApproval { action_request_id } => {
            let _permit = match wait_class.try_park() {
                Some(p) => p,
                None => {
                    return MutationVerdict::Deny {
                        reason: INTERCEPT_SATURATED_REASON.to_string(),
                    }
                }
            };
            register_and_wait(action_request_id)
        }
    }
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
    // P4.0b-ui2 / pin #2 — the ApprovalQueue projection is served TYPED (the frozen ApprovalQueueRow),
    // not loose JSON, because it is the safety-critical human-approval surface. Other projections keep
    // the generic row→JSON serve.
    if params.name == ProjectionName::ApprovalQueue {
        return Ok(match read_approval_queue_typed(db_path) {
            Ok(typed) => Ok(serde_json::to_value(typed).unwrap_or(serde_json::Value::Null)),
            Err(code) => Err(code),
        });
    }
    // P7.2 — the PullRequest projection is served TYPED (the frozen PullRequestRow), not loose JSON, so
    // the ui PR Review Workspace (§11.2/§7.2) consumes a contract. The ApprovalQueue precedent above.
    // Serialization of the already-validated Vec<PullRequestRow> is infallible in practice, but map a
    // serialize failure to InternalError rather than a silent `null` body — fail-closed, never a
    // corrupt response (LESSON §37; the ApprovalQueue branch's `unwrap_or(Null)` is a Step-9 consistency flag).
    if params.name == ProjectionName::PullRequest {
        return Ok(match read_pull_request_typed(db_path) {
            Ok(typed) => serde_json::to_value(typed).map_err(|_| IpcErrorCode::InternalError),
            Err(code) => Err(code),
        });
    }
    // D2/P4.4 — the Session projection served TYPED (the frozen SessionRow), not loose JSON, so the ui
    // per-session recovery indicator + RecoveryState banner (§11.4) consume a contract. The precedent above.
    if params.name == ProjectionName::Session {
        return Ok(match read_session_typed(db_path) {
            Ok(typed) => serde_json::to_value(typed).map_err(|_| IpcErrorCode::InternalError),
            Err(code) => Err(code),
        });
    }
    // D5b-1 — the Review projection served TYPED (the frozen ReviewRow), not loose JSON, so the ui PR
    // Review Workspace (§11.2) consumes a contract. The PullRequest/Session precedent above.
    if params.name == ProjectionName::Review {
        return Ok(match read_review_typed(db_path) {
            Ok(typed) => serde_json::to_value(typed).map_err(|_| IpcErrorCode::InternalError),
            Err(code) => Err(code),
        });
    }
    let table = projection_table(params.name);
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|e| IpcError::Read(e.to_string()))?;
    let rows = read_table_as_json(&conn, table)?;
    Ok(Ok(rows))
}

/// (P4.0b-ui2 / pin #2) Read `proj_approval_queue` served TYPED as the frozen [`ApprovalQueueRow`] —
/// no loose JSON on the §11.5 human-approval path. Reads the row JSON over a read-only WAL conn,
/// parses the redacted `policy_decision_json` TEXT into the typed `policy_decision: Option<PolicyDecision>`
/// (NULL → None — the plan-level approve-all case), drops the internal `sort_key`/`updated_at_seq`,
/// and deserializes each row STRICTLY (reject-unknown). A row that no longer deserializes is a
/// contract/corruption error → `InternalError` (fail-closed, never a silent skip).
pub fn read_approval_queue_typed(db_path: &Path) -> Result<Vec<ApprovalQueueRow>, IpcErrorCode> {
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|_| IpcErrorCode::InternalError)?;
    let json = read_table_as_json(&conn, "proj_approval_queue")
        .map_err(|_| IpcErrorCode::InternalError)?;
    let serde_json::Value::Array(raw_rows) = json else {
        return Err(IpcErrorCode::InternalError);
    };
    let mut out = Vec::with_capacity(raw_rows.len());
    for mut row in raw_rows {
        let serde_json::Value::Object(obj) = &mut row else {
            return Err(IpcErrorCode::InternalError);
        };
        // the redacted policy_decision_json TEXT → the typed `policy_decision` field; SQL NULL → None.
        let pd = match obj.remove("policy_decision_json") {
            Some(serde_json::Value::String(s)) => serde_json::from_str::<serde_json::Value>(&s)
                .map_err(|_| IpcErrorCode::InternalError)?,
            // NULL (plan-level approve-all) or the column absent → None.
            Some(serde_json::Value::Null) | None => serde_json::Value::Null,
            // the column is TEXT, so any other JSON type is a corrupt/mis-typed row → fail-closed
            // (never silently coerce to None on the safety-critical approval read).
            Some(_) => return Err(IpcErrorCode::InternalError),
        };
        obj.insert("policy_decision".to_string(), pd);
        // drop the internal bookkeeping columns — not on the frozen wire row (deny_unknown_fields).
        obj.remove("sort_key");
        obj.remove("updated_at_seq");
        let typed: ApprovalQueueRow =
            serde_json::from_value(row).map_err(|_| IpcErrorCode::InternalError)?;
        out.push(typed);
    }
    Ok(out)
}

/// (P7.2) Read `proj_pull_request` served TYPED as the frozen [`PullRequestRow`] — no loose JSON on the
/// ui PR Review Workspace read path (the `read_approval_queue_typed` precedent, LESSON §37). Reads the
/// row JSON over a read-only WAL conn, drops the internal `updated_at_seq` (not on the frozen wire row),
/// and deserializes each row STRICTLY (reject-unknown — `status` binds the §5.1 `PullRequest` enum). A
/// row that no longer deserializes (corrupt / contract-broken) is an integrity error → `InternalError`
/// (fail-closed, never a silent skip).
pub fn read_pull_request_typed(db_path: &Path) -> Result<Vec<PullRequestRow>, IpcErrorCode> {
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|_| IpcErrorCode::InternalError)?;
    let json =
        read_table_as_json(&conn, "proj_pull_request").map_err(|_| IpcErrorCode::InternalError)?;
    let serde_json::Value::Array(raw_rows) = json else {
        return Err(IpcErrorCode::InternalError);
    };
    let mut out = Vec::with_capacity(raw_rows.len());
    for mut row in raw_rows {
        let serde_json::Value::Object(obj) = &mut row else {
            return Err(IpcErrorCode::InternalError);
        };
        // drop the internal bookkeeping column — not on the frozen wire row (deny_unknown_fields). NB:
        // any OTHER proj_pull_request column not on the frozen row trips deny_unknown_fields → fail-closed;
        // the D5a mergeable/checks_summary columns ARE real struct fields (below), so the wire shape stays
        // the single source of truth (`PullRequestRow` in shared/src/projections.rs).
        obj.remove("updated_at_seq");
        // D5a: `mergeable` is a SQLite INTEGER (0/1) → `sqlite_to_json` yields a JSON number, but the frozen
        // `PullRequestRow.mergeable: Option<bool>` is a JSON bool. Coerce number→bool HERE (the first bool
        // projection column) so the `shared/` contract stays a pure bool; NULL stays null (→ None). The
        // `false` case (INTEGER 0) coerces to `false`, NOT absent (pinned in projections.rs).
        if let Some(m) = obj.get_mut("mergeable") {
            if let Some(n) = m.as_i64() {
                *m = serde_json::Value::Bool(n != 0);
            }
        }
        // D6: the diff-stats (additions/deletions/changed_files/commits) are INTEGER columns surfacing as
        // JSON numbers → they bind DIRECTLY into the frozen row's `Option<u64>` (no coercion, unlike the
        // bool `mergeable` above). The generic `read_table_as_json` already includes them — no SELECT change.
        // STRICT deserialize (reject-unknown; `status` binds the §5.1 PullRequest enum). A row that no
        // longer binds is corrupt/contract-broken → fail-closed, never a silent skip (LESSON §37).
        let typed: PullRequestRow =
            serde_json::from_value(row).map_err(|_| IpcErrorCode::InternalError)?;
        out.push(typed);
    }
    Ok(out)
}

/// (D5b-1) Read `proj_review` served TYPED as the frozen [`ReviewRow`] — no loose JSON on the ui PR
/// Review Workspace read path (the `read_pull_request_typed` precedent, LESSON §37). Reads the row JSON
/// over a read-only WAL conn, drops the internal `updated_at_seq` (not on the frozen wire row), and
/// deserializes each row STRICTLY (reject-unknown — `state` binds the frozen `ReviewState` enum). The
/// review_id/pr_number INTEGER columns surface as JSON numbers → `u64` binds; `body` is free-form review
/// text already §15-redacted at the event. A row that no longer deserializes (corrupt / contract-broken)
/// is an integrity error → `InternalError` (fail-closed, never a silent skip).
pub fn read_review_typed(db_path: &Path) -> Result<Vec<ReviewRow>, IpcErrorCode> {
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|_| IpcErrorCode::InternalError)?;
    let json = read_table_as_json(&conn, "proj_review").map_err(|_| IpcErrorCode::InternalError)?;
    let serde_json::Value::Array(raw_rows) = json else {
        return Err(IpcErrorCode::InternalError);
    };
    let mut out = Vec::with_capacity(raw_rows.len());
    for mut row in raw_rows {
        let serde_json::Value::Object(obj) = &mut row else {
            return Err(IpcErrorCode::InternalError);
        };
        // drop the internal bookkeeping column — not on the frozen wire row (deny_unknown_fields). Any
        // OTHER proj_review column not on the frozen row trips deny_unknown_fields → fail-closed.
        obj.remove("updated_at_seq");
        // STRICT deserialize (reject-unknown; `state` binds the frozen ReviewState enum). A row that no
        // longer binds is corrupt/contract-broken → fail-closed, never a silent skip (LESSON §37).
        let typed: ReviewRow =
            serde_json::from_value(row).map_err(|_| IpcErrorCode::InternalError)?;
        out.push(typed);
    }
    Ok(out)
}

/// The `SessionRow` wire fields — the SUBSET of `proj_session`'s ~22 columns surfaced on the typed serve
/// (basic-now + SPREAD). `read_session_typed` RETAINS only these (proj_session is far wider than the row,
/// so the drop-only-internal approach of `read_approval_queue_typed`/`read_pull_request_typed` doesn't
/// fit). Kept drift-proof by the `session_row_wire_fields_match_struct` test (this const must equal the
/// `SessionRow` serialized field set — a typo or a struct-field add fails that test, so a wire field can
/// never be silently dropped).
const SESSION_ROW_WIRE_FIELDS: &[&str] = &[
    "session_id",
    "project_id",
    "status",
    "display_name",
    "harness",
    "model",
    "execution_profile_id",
    "resume_mode",
    "replayed_event_count",
    "recovered_at",
];

/// (D2/P4.4) Read `proj_session` served TYPED as the frozen [`SessionRow`] — no loose JSON on the ui
/// per-session recovery indicator + `RecoveryState` banner read path (the `read_approval_queue_typed` /
/// `read_pull_request_typed` precedent, LESSON §37). `proj_session` carries ~22 columns vs the 10-field
/// wire row (basic-now + SPREAD), so each row is REDUCED to [`SESSION_ROW_WIRE_FIELDS`] before the STRICT
/// deserialize (additive-column-safe: a future SPREAD column won't trip `deny_unknown_fields`). A row that
/// no longer deserializes (corrupt / contract-broken — e.g. an unbindable `status`) is an integrity error
/// → `InternalError` (fail-closed, never a silent skip).
pub fn read_session_typed(db_path: &Path) -> Result<Vec<SessionRow>, IpcErrorCode> {
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|_| IpcErrorCode::InternalError)?;
    let json =
        read_table_as_json(&conn, "proj_session").map_err(|_| IpcErrorCode::InternalError)?;
    let serde_json::Value::Array(raw_rows) = json else {
        return Err(IpcErrorCode::InternalError);
    };
    let mut out = Vec::with_capacity(raw_rows.len());
    for mut row in raw_rows {
        let serde_json::Value::Object(obj) = &mut row else {
            return Err(IpcErrorCode::InternalError);
        };
        // RETAIN only the wire fields — proj_session is wider than the row; the not-yet-surfaced columns
        // are intentional SPREAD columns, not drift. The STRICT deserialize then fails closed on a bad
        // value (e.g. an unbindable `status`/`resume_mode`).
        obj.retain(|k, _| SESSION_ROW_WIRE_FIELDS.contains(&k.as_str()));
        let typed: SessionRow =
            serde_json::from_value(row).map_err(|_| IpcErrorCode::InternalError)?;
        out.push(typed);
    }
    Ok(out)
}

/// `get_diff` (§6.1; P4.0b-ui1) — the hunk-structured diff read for the ui-6.3e per-hunk review.
/// Bad/malformed params → `protocol_error`; an unresolved worktree_id → `not_found`; success → the
/// serialized [`DiffResult`]. Pure read (no write-actor, no mutation).
fn get_diff(
    params: &serde_json::Value,
    db_path: &Path,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let params: GetDiffParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    match read_worktree_diff(db_path, &params.worktree_id, &params.file) {
        Ok(diff) => Ok(Ok(
            serde_json::to_value(diff).unwrap_or(serde_json::Value::Null)
        )),
        Err(code) => Ok(Err(code)),
    }
}

/// Resolve `worktree_id → proj_worktree.path` over a READ-ONLY WAL conn, then read `file`'s
/// HEAD→workdir diff LIVE via git2 (read-only). The testable core of [`get_diff`]. **No mutation, no
/// write-actor** (the §7.2 worktree-live-read precedent / Forbidden #3). An unpopulated worktree_id
/// (`proj_worktree` fills at P5.2/edges) OR a path that isn't a readable git repo → [`IpcErrorCode::NotFound`].
pub fn read_worktree_diff(
    db_path: &Path,
    worktree_id: &str,
    file: &str,
) -> Result<DiffResult, IpcErrorCode> {
    use rusqlite::OptionalExtension as _;
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|_| IpcErrorCode::InternalError)?;
    let path: Option<String> = conn
        .query_row(
            "SELECT path FROM proj_worktree WHERE worktree_id = ?1",
            [worktree_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| IpcErrorCode::InternalError)?;
    // an unpopulated worktree_id (proj_worktree empty until P5) → typed NotFound (NOT precondition_stale
    // — that's the re-approvable mutation card; this is a read not-found).
    let Some(path) = path else {
        return Err(IpcErrorCode::NotFound);
    };
    crate::git::read_diff(Path::new(&path), file).map_err(|e| match e {
        // the worktree path isn't a readable git repo (e.g. moved/not-yet-created) → NotFound.
        crate::git::GitReadError::Open { .. } => IpcErrorCode::NotFound,
        // a genuine diff-read fault → internal.
        crate::git::GitReadError::Diff(_) => IpcErrorCode::InternalError,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_row_wire_fields_match_struct() {
        // drift-proof: the retain-whitelist MUST equal SessionRow's actual serialized field set, so a
        // typo or a struct-field add/rename can't silently drop a wire field from the typed serve.
        use nexusops_shared::harness::ResumeMode;
        use nexusops_shared::status::Session;
        use std::collections::BTreeSet;
        let sample = SessionRow {
            session_id: "s".into(),
            project_id: "p".into(),
            status: Session::Active,
            display_name: None,
            harness: None,
            model: None,
            execution_profile_id: None,
            resume_mode: Some(ResumeMode::Resumed),
            replayed_event_count: Some(0),
            recovered_at: None,
        };
        let struct_fields: BTreeSet<String> = serde_json::to_value(&sample)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let whitelist: BTreeSet<String> = SESSION_ROW_WIRE_FIELDS
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            struct_fields, whitelist,
            "read_session_typed whitelist drifted from SessionRow fields"
        );
    }

    #[test]
    fn session_create_request_threads_initial_prompt() {
        // spec(§9.1) — the IPC boundary threads `initial_prompt` from the params into the
        // `ActionRequest.inputs` (the Option-G dev-drive, brief 053). The thread is complete from the
        // wire, not just inside the executor.
        let params =
            serde_json::json!({ "project_id": "proj_x", "initial_prompt": "do the thing" });
        let req = build_session_create_request(&params).expect("builds the session.create request");
        assert_eq!(req.action_type, "session.create");
        assert_eq!(
            req.inputs.get("initial_prompt").and_then(|v| v.as_str()),
            Some("do the thing"),
            "initial_prompt is threaded from the params into inputs"
        );
    }

    #[test]
    fn session_create_request_no_prompt_omits_it() {
        // additive/opt-in — no initial_prompt param → inputs carries none (back-compat).
        let params = serde_json::json!({ "project_id": "proj_x" });
        let req = build_session_create_request(&params).expect("builds");
        assert!(
            req.inputs.get("initial_prompt").is_none(),
            "no param → no inputs.initial_prompt"
        );
    }

    #[test]
    fn session_create_request_threads_profile_and_prompt_together() {
        // both optional params coexist (the `smoke create --prompt --profile` path) — the existing
        // execution_profile_id thread (§15 #8) is preserved alongside the new initial_prompt.
        let params = serde_json::json!({
            "project_id": "proj_x",
            "execution_profile_id": "prof_y",
            "initial_prompt": "go"
        });
        let req = build_session_create_request(&params).expect("builds");
        assert_eq!(
            req.inputs
                .get("execution_profile_id")
                .and_then(|v| v.as_str()),
            Some("prof_y")
        );
        assert_eq!(
            req.inputs.get("initial_prompt").and_then(|v| v.as_str()),
            Some("go")
        );
        // forward-drift guard — ONLY the two known optional inputs are threaded (a future stray field
        // would trip this rather than silently riding into `inputs`).
        assert_eq!(
            req.inputs.as_object().map(|o| o.len()),
            Some(2),
            "exactly the two known inputs (execution_profile_id + initial_prompt) are threaded"
        );
    }

    #[test]
    fn session_create_request_missing_project_is_protocol_error() {
        // project_id is required (the catalog `requires_resource_refs` ref) — a missing one is a
        // client protocol violation, not a silent default.
        let params = serde_json::json!({ "initial_prompt": "go" });
        assert!(matches!(
            build_session_create_request(&params),
            Err(IpcErrorCode::ProtocolError)
        ));
    }
}

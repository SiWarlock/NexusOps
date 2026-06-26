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
use nexusops_shared::gateway_ids::ActionPlanId;
use nexusops_shared::ids::{ActionRequestId, ProjectId};
use nexusops_shared::ipc::{
    Capabilities, ConnectViaGhParams, ConnectViaGhResult, ConnectViaGhStatus, DiffResult,
    GetDiffParams, GetExecutionProfilesResult, GetPrDiffParams, GetProjectionParams, IpcErrorCode,
    ProfileRow, ProjectionName, RpcRequest, RpcResponse, SubscribeParams, WireError,
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch(
    req: &RpcRequest,
    db_path: &Path,
    write: &WriteHandle,
    registry: &DecisionRegistry,
    wait_class: &InterceptWaitClass,
    github: &dyn crate::integrations::github::GithubReadClient,
    gh_connector: &dyn crate::integrations::auth::GhConnector,
    // P5.3b/085 (cat-1) — the keychain store for the `profile.set_secret` inbound-secret trigger (the
    // FIRST inbound-secret surface). Reachable ONLY post-auth (the rule-#7 getpeereid gate runs first in
    // serve_connection). Reuses the SAME production `KeyringSecretStore` as the github clients (main.rs).
    secret_store: &dyn crate::integrations::keychain::SecretStore,
) -> Result<RpcResponse, IpcError> {
    let outcome: Result<serde_json::Value, IpcErrorCode> = match req.method.as_str() {
        "get_capabilities" => Ok(capabilities_value()),
        "get_projection" => get_projection(&req.params, db_path)?,
        // P4.0b-ui1 — the §6.1 hunk-structured diff READ (the ui-6.3e source). Resolves
        // worktree_id→proj_worktree.path (read-only WAL) then reads git2 LIVE read-only; NO mutation.
        "get_diff" => get_diff(&req.params, db_path)?,
        // D7 — the §6.1 remote-PR code-diff READ (the Review tab). The FIRST network read in the IPC
        // layer: resolves (repo_id,pr_number)→owner/repo (read-only WAL) then fetches the PR diff via the
        // injected GithubReadClient (block_on the captured handle + a MANDATORY timeout); NO mutation.
        "get_pr_diff" => get_pr_diff(&req.params, db_path, github)?,
        // W1-prof/093 — the §2.8 execution_profiles registry read RPC (the cockpit profile-picker source).
        // Serves the secret-free ProfileRow list over read-only WAL; §15 #4 the keychain POINTER is NEVER
        // served (only the derived has_credential). No-param; a corrupt row fails closed (internal_error).
        "get_execution_profiles" => get_execution_profiles(db_path)?,
        // P4.7/083 (C3b) — the "Connect via gh" auth-bootstrap trigger: the daemon reads `gh auth token`
        // → keychain (NO token over IPC), returns the keychain_ref POINTER. NOT a Gateway mutation (it
        // writes the keychain, not the DB/event log — the §15/LESSON §49 non-Gateway secret-write
        // mechanism; the AUDIT is the subsequent integration.connect registration the UI submits). The
        // rule-#7 getpeereid gate already authed the peer; the gateway-trusted local UI is the only caller.
        "connect_via_gh" => connect_via_gh(&req.params, gh_connector),
        // P5.3b/085 (C2, CAT-1) — the "set profile credential" inbound-secret trigger: the user-typed
        // secret arrives INBOUND over the getpeereid-authed local UDS (the ⚠️ NEW POSTURE) → the daemon
        // holds it in Zeroizing → the OS keychain under the daemon-derived per-profile ref; the result is
        // the keychain_ref POINTER only (NO secret echoed — §15 #4 / LESSON §64). Fail-closed on an
        // unknown/unparseable profile (LESSON §62 — no keychain entry for an unregistered profile). NOT a
        // Gateway mutation (it writes the keychain, not the DB — the §49 non-Gateway secret-write; the AUDIT
        // is the subsequent `profile.set_keychain_ref` pointer-record action). Peer-authed by the rule-#7 gate.
        "profile.set_secret" => profile_set_secret(&req.params, secret_store, db_path),
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
///
/// 090 de-collapse: the prior mapping collapsed FIVE distinct faults → one `precondition_stale`, so the
/// cockpit rendered a re-approvable "stale card" for non-re-approvable faults (not-found / fail-closed
/// audit write / out-of-state transition). Each fault now surfaces its honest existing code (no closed-set
/// bump). The §17/rule-#6 `fencing_conflict` distinction is PRESERVED (LESSON 21).
fn gateway_error_to_code(e: &GatewayError) -> IpcErrorCode {
    match e {
        GatewayError::PolicyDenied
        | GatewayError::UnsupportedPolicyDecision(_)
        // a Blocked-mode plan submission is rejected on policy grounds (Blocked is a 2.2-assigned
        // outcome, not a submittable mode) — the request parses fine, so it is policy_denied, not
        // a protocol_error.
        | GatewayError::UnsupportedApprovalMode(_) => IpcErrorCode::PolicyDenied,
        // genuinely re-approvable faults — a fresh approval cycle / re-submit fixes them (§6.4 the
        // re-approvable stale card): the live source changed since approval (2.4 L4), or the approval lapsed.
        GatewayError::StalePrecondition | GatewayError::ApprovalExpired => {
            IpcErrorCode::PreconditionStale
        }
        // a missing target is its own honest code — no longer masquerading as a re-approvable stale card.
        GatewayError::NotFound(_) => IpcErrorCode::NotFound,
        // daemon-internal faults — NOT user-re-approvable: a fail-closed audit write (Q7 carry-forward
        // correction), an out-of-state transition (a daemon-side fault on the submit/auto-execute path),
        // or the latched systemic audit-backbone breaker (P4.0b-2c) → §6.4 `internal_error` (the honest
        // "the daemon failed" signal; the breaker's loud signal is the durable systemic alarm + latched state).
        GatewayError::AuditWriteFailed(_)
        | GatewayError::IllegalTransition { .. }
        | GatewayError::AuditBackboneDown => IpcErrorCode::InternalError,
        // 2.4 L3 — a stale fencing token: the NEVER-auto-resolved hard-conflict card (rule #6),
        // distinct from the re-approvable precondition_stale (the §17/§11.5 safety-card distinction).
        GatewayError::FencingConflict => IpcErrorCode::FencingConflict,
        GatewayError::Serialize(_) => IpcErrorCode::ProtocolError,
    }
}

/// Parse-don't-trust the client-supplied minted ids on an inbound `ActionRequest` at the §6.1 IPC
/// boundary. `#[serde(transparent)]` newtypes do NOT validate on the wire — an empty/malformed id
/// deserializes fine → an empty/garbage audit-row PK (a 2nd empty-PK insert COLLIDES → `AuditWriteFailed`
/// masquerading as `precondition_stale`, the cockpit Add-project blocker root cause). Reject fail-closed
/// with `protocol_error` BEFORE any row insert / pipeline run (§15 audit-integrity, INV-SEC-1-adjacent).
/// Resource-ref ids stay executor-validated (out of scope here).
fn validate_request_ids(req: &ActionRequest) -> Result<(), IpcErrorCode> {
    ActionRequestId::parse(req.action_request_id.as_str())
        .map_err(|_| IpcErrorCode::ProtocolError)?;
    if let Some(pid) = req.project_id.as_ref() {
        ProjectId::parse(pid.as_str()).map_err(|_| IpcErrorCode::ProtocolError)?;
    }
    Ok(())
}

/// `submit_action` — parse the §6.2 `ActionRequest`, validate its client-supplied ids, run the pipeline → `ActionAck`.
fn submit_action(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let req: ActionRequest = match serde_json::from_value(params.clone()) {
        Ok(r) => r,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    if let Err(c) = validate_request_ids(&req) {
        return Ok(Err(c));
    }
    gateway_result(write.submit_action_blocking(req))
}

/// `submit_action_plan` — parse the §6.2 `ActionPlan`, validate the plan envelope + EVERY step's request
/// ids, run the plan pipeline → `PlanAck` (O-3).
fn submit_action_plan(
    params: &serde_json::Value,
    write: &WriteHandle,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let plan: ActionPlan = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    // parse-don't-trust the plan envelope + each step's request ids before any persist (§15 audit-integrity).
    if ActionPlanId::parse(plan.plan_id.as_str()).is_err() {
        return Ok(Err(IpcErrorCode::ProtocolError));
    }
    for step in &plan.steps {
        if let Err(c) = validate_request_ids(&step.action_request) {
            return Ok(Err(c));
        }
    }
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
        // P4.7: `head_sha` is a TEXT column → JSON string → binds DIRECTLY into the frozen row's
        // `Option<String>` (no coercion either; the generic read already includes it — no SELECT change).
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

/// (W1-prof/093) `get_execution_profiles` (§6.1) — serve the §2.8 `execution_profiles` registry as the
/// secret-free [`ProfileRow`] list the cockpit profile picker consumes. No-param (MVP single-workspace; an
/// optional `{workspace_id}` filter is a later add). A read fault / corrupt registry row → a structured
/// `internal_error` (never a disconnect; never a silent partial list). Pure read — NO mutation, NO write-actor.
fn get_execution_profiles(
    db_path: &Path,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    match read_execution_profile_rows(db_path) {
        Ok(profiles) => {
            let result = GetExecutionProfilesResult { profiles };
            // infallible: ProfileRow is plain String/enum/bool → to_value cannot fail (the DiffResult precedent).
            Ok(Ok(
                serde_json::to_value(result).unwrap_or(serde_json::Value::Null)
            ))
        }
        Err(code) => Ok(Err(code)),
    }
}

/// (W1-prof/093) Read the §2.8 `execution_profiles` registry as the secret-free [`ProfileRow`] list — over a
/// READ-ONLY WAL conn (no mutation, no write-actor; the `profile_exists`/`get_diff` precedent). **§15 #4: the
/// keychain POINTER is NEVER served** — `keychain_ref` is read ONLY to derive `has_credential`
/// (`= keychain_ref.is_some()`), it NEVER enters the row. `is_default` flags the cold-start seed = the FIRST
/// `ExecutionProfileRegistered` event (the `SqliteProfileLookup::default_id` provenance; the read FILTERS to
/// that event type — cold-start also emits Device/LocalRunner registration events first). A row whose `status`
/// TEXT is not a valid §5.1 `ExecutionProfile` wire value is an integrity error → the WHOLE read fails closed
/// (`InternalError`), never a silent partial list (the LESSON §37 typed-serve precedent).
pub fn read_execution_profile_rows(db_path: &Path) -> Result<Vec<ProfileRow>, IpcErrorCode> {
    use rusqlite::OptionalExtension as _;
    // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
    let conn =
        crate::eventstore::open_read_only(db_path).map_err(|_| IpcErrorCode::InternalError)?;
    // the default profile = the FIRST `ExecutionProfileRegistered` event (the cold-start seed; matches
    // `seed_default_profile` + `SqliteProfileLookup::default_id`). FILTER to that event type — cold-start
    // also emits DeviceRegistered/LocalRunnerRegistered first, so "the first event" would be the wrong one.
    //
    // INTENTIONAL SOFT-DEGRADE (LESSON §30 — distinct from the served-row `status` below, which fails
    // CLOSED per LESSON §37): `is_default` is a non-load-bearing UI PRE-SELECT hint, NOT consumer-bound
    // served data. No seed event (`.optional()` → None) OR a corrupt/unparseable seed payload (`.ok()` →
    // None) yields "no row flagged default" — NEVER the WRONG default, never a fail-open. Fail-closing the
    // WHOLE profile list over a cosmetic hint would blank the cockpit picker (a worse availability
    // tradeoff); the §15 #4 safety invariant does not depend on this resolution succeeding.
    let default_id: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM events WHERE event_type = ?1 ORDER BY seq LIMIT 1",
            [nexusops_shared::events::ExecutionProfileRegistered::EVENT_TYPE],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| IpcErrorCode::InternalError)?
        .and_then(|payload| {
            serde_json::from_str::<serde_json::Value>(&payload)
                .ok()
                .and_then(|v| {
                    v.get("execution_profile_id")
                        .and_then(|id| id.as_str())
                        .map(str::to_string)
                })
        });
    // ORDER stable (created_at, then id) so the served list is deterministic across reads.
    let mut stmt = conn
        .prepare(
            "SELECT execution_profile_id, provider, harness, model, account_alias, keychain_ref, status \
             FROM execution_profiles ORDER BY created_at, execution_profile_id",
        )
        .map_err(|_| IpcErrorCode::InternalError)?;
    let raw = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,         // execution_profile_id
                r.get::<_, String>(1)?,         // provider
                r.get::<_, String>(2)?,         // harness
                r.get::<_, Option<String>>(3)?, // model
                r.get::<_, Option<String>>(4)?, // account_alias
                r.get::<_, Option<String>>(5)?, // keychain_ref — read ONLY to derive has_credential (§15 #4)
                r.get::<_, String>(6)?,         // status (the §5.1 wire string)
            ))
        })
        .map_err(|_| IpcErrorCode::InternalError)?;
    let mut out = Vec::new();
    for row in raw {
        let (id, provider, harness, model, account_alias, keychain_ref, status_wire) =
            row.map_err(|_| IpcErrorCode::InternalError)?;
        // bind the §5.1 ExecutionProfile enum (reject-unknown). A mis-typed status row is corrupt → the
        // WHOLE read fails closed (never a silent partial — the LESSON §37 typed-serve precedent).
        let status: nexusops_shared::status::ExecutionProfile =
            serde_json::from_value(serde_json::Value::String(status_wire))
                .map_err(|_| IpcErrorCode::InternalError)?;
        out.push(ProfileRow {
            is_default: default_id.as_deref() == Some(id.as_str()),
            // §15 #4 — the POINTER is consumed ONLY to derive the bool; it NEVER enters the served row.
            has_credential: keychain_ref.is_some(),
            execution_profile_id: id,
            provider,
            harness,
            model,
            account_alias,
            status,
        });
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

/// (D7) `get_pr_diff` — the REMOTE-PR head-vs-base code-diff the §11.2 Review tab renders. Bad params →
/// `protocol_error`; an unresolvable/failed read → a typed `IpcErrorCode`; success → the serialized
/// [`DiffResult`]. A pure read — NO mutation, NO write-actor, NO event (LESSON 33; contrast the
/// `github.sync_reviews` EMITTING action). The `client`/`handle` are threaded from the accept-loop.
fn get_pr_diff(
    params: &serde_json::Value,
    db_path: &Path,
    client: &dyn crate::integrations::github::GithubReadClient,
) -> Result<Result<serde_json::Value, IpcErrorCode>, IpcError> {
    let params: GetPrDiffParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(_) => return Ok(Err(IpcErrorCode::ProtocolError)),
    };
    // LESSON §46 — capture the runtime handle for the async fetch's `block_on`. serve_connection runs on
    // a `spawn_blocking` task within the runtime (the `intercept` handler's precedent), so `current()` is
    // valid here; read_pr_diff's `block_on` is fine off a blocking (non-async) thread.
    let handle = tokio::runtime::Handle::current();
    match read_pr_diff(
        db_path,
        client,
        &handle,
        PR_DIFF_TIMEOUT,
        &params.repo_id,
        params.pr_number,
        params.file.as_deref(),
    ) {
        // infallible: `DiffResult` is plain `String`/`u32`/`Vec` → `to_value` cannot fail (the `get_diff`
        // precedent; a `Null` fallback would be unreachable, never a real "null-as-success").
        Ok(diff) => Ok(Ok(
            serde_json::to_value(diff).unwrap_or(serde_json::Value::Null)
        )),
        Err(code) => Ok(Err(code)),
    }
}

/// (P4.7/083 C3b) `connect_via_gh` — the "Connect via gh" auth-bootstrap trigger. The daemon SOURCES the
/// token (reads `gh auth token`) → the OS keychain under the per-account `keychain_ref`; NO token over IPC.
/// Bad params → `protocol_error`; gh-absent → a structured `gh_unavailable` RESULT (the device-flow signal,
/// NOT a wire error); a keychain backend fault → `internal_error` (the token NEVER appears in any message).
/// NOT a Gateway mutation — it writes the keychain, not the DB/event log (the §15/LESSON §49 non-Gateway
/// secret-write mechanism; the AUDIT is the subsequent `integration.connect` registration the UI submits
/// with the returned `keychain_ref`). The rule-#7 getpeereid gate already authed the peer.
fn connect_via_gh(
    params: &serde_json::Value,
    connector: &dyn crate::integrations::auth::GhConnector,
) -> Result<serde_json::Value, IpcErrorCode> {
    use crate::integrations::auth::AuthError;
    let params: ConnectViaGhParams =
        serde_json::from_value(params.clone()).map_err(|_| IpcErrorCode::ProtocolError)?;
    // the account is the keychain_ref key + the connection account — non-empty + control-char-free (no
    // injection into the keychain identity; the connect.rs `is_clean_account` discipline).
    if params.account.trim().is_empty() || params.account.chars().any(|c| c.is_control()) {
        return Err(IpcErrorCode::ProtocolError);
    }
    match connector.connect(params.provider, &params.account) {
        Ok(keychain_ref) => {
            let result = ConnectViaGhResult {
                status: ConnectViaGhStatus::Connected,
                keychain_ref: Some(keychain_ref),
            };
            // infallible: ConnectViaGhResult is plain String/enum → to_value cannot fail.
            Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
        }
        // gh not installed / not authed → a structured DOMAIN outcome (use device-flow 084), not an error.
        Err(AuthError::GhUnavailable) => {
            let result = ConnectViaGhResult {
                status: ConnectViaGhStatus::GhUnavailable,
                keychain_ref: None,
            };
            Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
        }
        // a keychain backend fault is a real infra failure → internal_error (the token is NOT in the
        // message — AuthError::Keychain wraps only a structural SecretStoreError class).
        Err(AuthError::Keychain(_)) => Err(IpcErrorCode::InternalError),
    }
}

/// (P5.3b/085 C2, CAT-1) `profile.set_secret` — the inbound-secret IPC trigger dispatch wrapper. Parses the
/// `SetProfileSecretParams` (a malformed params is a client `protocol_error`), then delegates to the testable
/// [`set_profile_secret`] core. The secret rides Zeroizing daemon-side + is dropped post-write; the result is
/// the keychain_ref POINTER only (§15 #4 / LESSON §64 no-echo). A keychain backend fault → `internal_error`
/// (the token is NEVER in the message). NB: `params.clone()` makes a transient plaintext copy of the inbound
/// secret at the JSON boundary (unavoidable — it arrives as JSON; the local-trust-boundary accepts it, the
/// ⚠️ NEW POSTURE); the clone drops at the end of this fn, and the daemon-side transient is Zeroizing-scrubbed.
fn profile_set_secret(
    params: &serde_json::Value,
    store: &dyn crate::integrations::keychain::SecretStore,
    db_path: &Path,
) -> Result<serde_json::Value, IpcErrorCode> {
    let params: nexusops_shared::ipc::SetProfileSecretParams =
        serde_json::from_value(params.clone()).map_err(|_| IpcErrorCode::ProtocolError)?;
    let result = set_profile_secret(store, db_path, params)?;
    // infallible: SetProfileSecretResult is a single String → to_value cannot fail.
    Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
}

/// The testable core of [`profile_set_secret`] (P5.3b/085 C2). Fail-closed gate order: (1) the
/// `execution_profile_id` MUST parse (unparseable → `protocol_error` — no keychain touched); (2) the profile
/// MUST be REGISTERED (`profiles::profile_exists`, read-only WAL — a read fault → `internal_error`; an
/// unregistered/unknown id → `not_found`; LESSON §62 — NEVER write a keychain entry for an unregistered
/// profile); only THEN (3) the inbound secret (held in Zeroizing, dropped post-write) is written to the OS
/// keychain under the daemon-derived `profile_keychain_ref` and the POINTER is returned (§15 #4 / LESSON §64).
pub fn set_profile_secret(
    store: &dyn crate::integrations::keychain::SecretStore,
    db_path: &Path,
    params: nexusops_shared::ipc::SetProfileSecretParams,
) -> Result<nexusops_shared::ipc::SetProfileSecretResult, IpcErrorCode> {
    use nexusops_shared::ids::ExecutionProfileId;
    // (1) the id MUST parse — an unparseable id is a malformed param, fail-closed (no keychain touched).
    let id = ExecutionProfileId::parse(&params.execution_profile_id)
        .map_err(|_| IpcErrorCode::ProtocolError)?;
    // (2) fail-closed-on-unknown (LESSON §62): never write a secret for a profile the registry doesn't know.
    if !crate::profiles::profile_exists(db_path, &id).map_err(|_| IpcErrorCode::InternalError)? {
        return Err(IpcErrorCode::NotFound);
    }
    // (3) the inbound secret rides Zeroizing — moved out of params, written to the keychain, dropped (the
    // plaintext heap allocation is scrubbed; §15). A backend fault → structural internal_error (the token is
    // NEVER in the message — SecretStoreError carries only a class).
    let secret = zeroize::Zeroizing::new(params.secret);
    crate::profiles::secret::write_profile_secret(store, &id, secret)
        .map_err(|_| IpcErrorCode::InternalError)
}

/// The MANDATORY bound on the GitHub PR-diff fetch (LESSON §46): a hung call returns a typed error,
/// never wedges the read handler (which runs on the accept-loop's blocking thread).
const PR_DIFF_TIMEOUT: Duration = Duration::from_secs(30);

/// Resolve a PR to its remote diff via the injected [`GithubReadClient`], then parse the unified diff
/// (the testable core of [`get_pr_diff`]). Resolves `(repo_id, pr_number) → proj_pull_request.project_id
/// → proj_repository.remote_url → owner/repo` over a READ-ONLY WAL conn (the EXACT PR row must exist —
/// its absence IS the `NotFound`); the live fetch runs on the captured `handle` (`block_on`) under a
/// MANDATORY `timeout`. **No mutation, no write-actor.** A timeout / GitHub failure → a typed error
/// (NEVER raw API text — §15-safe: the structural error class only); a 404-class → `NotFound`.
#[allow(clippy::too_many_arguments)]
pub fn read_pr_diff(
    db_path: &Path,
    client: &dyn crate::integrations::github::GithubReadClient,
    handle: &tokio::runtime::Handle,
    timeout: Duration,
    repo_id: &str,
    pr_number: u64,
    file: Option<&str>,
) -> Result<DiffResult, IpcErrorCode> {
    let (owner, repo) = resolve_pr_owner_repo(db_path, repo_id, pr_number)?;
    // LESSON §46: the read handler runs on the accept-loop's blocking thread (no entered runtime) →
    // drive the async client via the CAPTURED handle's `block_on`, never `Handle::current()`. A hard
    // timeout bounds it so a hung GitHub call can never wedge the handler.
    let fetched = handle.block_on(async {
        tokio::time::timeout(timeout, client.fetch_pr_diff(&owner, &repo, pr_number)).await
    });
    let diff_text = match fetched {
        // timed out → a typed internal error (structural; §15 — never raw text).
        Err(_elapsed) => return Err(IpcErrorCode::InternalError),
        // a GitHub failure → NotFound for a 404-class (the PR/repo is gone), else internal. The error
        // MESSAGE is never surfaced (§15 — only the structural class maps to a code).
        Ok(Err(e)) => {
            return Err(match e.class {
                crate::integrations::classifier::IntegrationOutcomeClass::NotFound => {
                    IpcErrorCode::NotFound
                }
                _ => IpcErrorCode::InternalError,
            })
        }
        Ok(Ok(text)) => text,
    };
    Ok(crate::git::parse_unified_diff(&diff_text, file))
}

/// Resolve `(repo_id, pr_number)` → the GitHub `(owner, repo)` for the D7 read path. A thin wrapper over
/// the SHARED [`crate::integrations::repo_resolve`] authority (P4.7 extraction — the github-write executors
/// resolve their target through the SAME helper, so audited==executed everywhere; no divergent copy).
/// Maps the neutral [`RepoResolveError`] → the ipc [`IpcErrorCode`] (NotFound→NotFound, Internal→Internal)
/// — D7 behavior byte-unchanged (the `get_diff`-unpopulated precedent).
fn resolve_pr_owner_repo(
    db_path: &Path,
    repo_id: &str,
    pr_number: u64,
) -> Result<(String, String), IpcErrorCode> {
    use crate::integrations::repo_resolve::{resolve_owner_repo_by_pr, RepoResolveError};
    resolve_owner_repo_by_pr(db_path, repo_id, pr_number).map_err(|e| match e {
        RepoResolveError::NotFound => IpcErrorCode::NotFound,
        RepoResolveError::Internal => IpcErrorCode::InternalError,
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
    fn gateway_error_de_collapse_maps_each_variant_to_an_honest_code() {
        // spec(§6.4) — the 5-distinct-GatewayError → 1-precondition_stale overload is de-collapsed so the
        // cockpit no longer renders a re-approvable "stale card" for non-re-approvable faults. Each fault
        // maps to its honest existing IpcErrorCode (no closed-set bump).
        use crate::gateway::GatewayError;
        // genuinely re-approvable (re-submit / fresh approval cycle) → precondition_stale.
        assert_eq!(
            gateway_error_to_code(&GatewayError::StalePrecondition),
            IpcErrorCode::PreconditionStale
        );
        assert_eq!(
            gateway_error_to_code(&GatewayError::ApprovalExpired),
            IpcErrorCode::PreconditionStale
        );
        // not-found is its own honest code (no longer masquerading as stale).
        assert_eq!(
            gateway_error_to_code(&GatewayError::NotFound("x".into())),
            IpcErrorCode::NotFound
        );
        // daemon-internal faults (fail-closed audit write / out-of-state transition / systemic breaker) are
        // NOT user-re-approvable → internal_error.
        assert_eq!(
            gateway_error_to_code(&crate::gateway::db_err(
                rusqlite::Error::QueryReturnedNoRows
            )),
            IpcErrorCode::InternalError,
            "AuditWriteFailed → internal_error (the Q7 carry-forward correction)"
        );
        assert_eq!(
            gateway_error_to_code(&GatewayError::IllegalTransition {
                machine: "session",
                from: "a".into(),
                to: "b".into(),
            }),
            IpcErrorCode::InternalError
        );
        assert_eq!(
            gateway_error_to_code(&GatewayError::AuditBackboneDown),
            IpcErrorCode::InternalError
        );
        // unchanged buckets (regression).
        assert_eq!(
            gateway_error_to_code(&GatewayError::PolicyDenied),
            IpcErrorCode::PolicyDenied
        );
        assert_eq!(
            gateway_error_to_code(&GatewayError::UnsupportedApprovalMode("Blocked".into())),
            IpcErrorCode::PolicyDenied
        );
        assert_eq!(
            gateway_error_to_code(&GatewayError::UnsupportedPolicyDecision("x".into())),
            IpcErrorCode::PolicyDenied
        );
        assert_eq!(
            gateway_error_to_code(&GatewayError::Serialize("x".into())),
            IpcErrorCode::ProtocolError
        );
    }

    #[test]
    fn gateway_error_fencing_conflict_stays_distinct() {
        // spec(§17 / safety rule #6) — the NEVER-auto-resolved hard-conflict card MUST stay its own code,
        // never collapsed into the re-approvable precondition_stale bucket (the safety pin; the de-collapse
        // touches only the precondition_stale overload, never the fencing distinction — LESSON 21).
        use crate::gateway::GatewayError;
        let code = gateway_error_to_code(&GatewayError::FencingConflict);
        assert_eq!(code, IpcErrorCode::FencingConflict);
        assert_ne!(
            code,
            IpcErrorCode::PreconditionStale,
            "fencing_conflict must never be the re-approvable precondition_stale"
        );
    }

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

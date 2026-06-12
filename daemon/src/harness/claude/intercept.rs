//! The Claude hook-receiver ingress + the `MutationIntercept`→Gateway routing (brief 043, INV-SEC-1).
//!
//! L2 (this layer): the **hook-receiver ingress** — parse a `PreToolUse` payload + the pure
//! `tool_name (+ permission_mode) → agent-mutation action_type` mapping, **fail-closed** (malformed /
//! unmapped / non-`default` mode → Deny; never an un-adjudicated allow). The daemon-wired
//! `PreToolUse` hook (042's generated settings, matcher `"*"`) pipes the payload on stdin; the hook
//! fires on EVERY tool call (per-call interception is sound — hooks-guide.md), so the daemon
//! adjudicates each call.
//!
//! L3 adds the routing (build an agent-mutation `ActionRequest` → the EXISTING Gateway `submit_action`
//! → policy/approval → an **adjudication-only** verdict — the action terminates at the verdict, NO
//! daemon executor runs the tool). L4 adds the O-13 coverage-gap compensation (the §9.1 matrix
//! disposition — MCP / Task subagent / background subagent DENY two-layer + the launch
//! `permissions.deny` hard-block) + the params-sensitive deny-rules.
//!
//! **Safety #9** — the receiver reads the STRUCTURED hook payload, never the PTY. **Safety #10 /
//! O-13** — a non-`default` permission mode is denied (interception isn't guaranteed outside it).

use serde::Deserialize;

use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequest as ActionRequestStatus;
use nexusops_shared::time::Timestamp;

use crate::eventstore::EventStore;
use crate::gateway::Gateway;
use crate::harness::MutationVerdict;

/// The subset of the Claude `PreToolUse` hook payload the receiver consumes. The daemon-wired hook
/// pipes the FULL JSON on stdin; the extra fields (`cwd`/`transcript_path`/`hook_event_name`/…) are
/// ignored — a tolerant read of the semi-trusted hook input, extracting only what the adjudication
/// needs. The 4 fields are REQUIRED: a payload missing any (or with a wrong type) fails to parse →
/// Deny (fail-closed — never construct an allow from garbage).
#[derive(Debug, Deserialize)]
pub struct HookPayload {
    /// the tool about to run. The interceptable DIRECT tools (`Bash`/`Write`/`Edit`/`Read`/…) map to
    /// an `agent.*` action_type; `mcp__<server>__<tool>` (MCP) and `Task` (subagent) are RECOGNIZED
    /// categories but **DENIED** (the lead-ratified coverage disposition — MCP BestEffort / subagent
    /// NotGuaranteed can't be reliably hook-intercepted; L4 formalizes the §9.1-matrix reason + the
    /// launch `permissions.deny` hard-block). An unknown tool is denied too (fail-closed).
    pub tool_name: String,
    /// the tool's input params — carried VERBATIM (any JSON value; no L2 shape validation). The L3
    /// routing redacts it into `ActionRequest.inputs`; the L4 deny-rules read it. L2 never persists it.
    pub tool_input: serde_json::Value,
    /// the Claude session id — **opaque at L2** (no adjudication logic depends on its value); the
    /// per-session `decision_sink` binding to the live session is the P4 transport.
    pub session_id: String,
    /// the session's permission mode — MUST be `default` (O-13 #10); any other mode → Deny.
    pub permission_mode: String,
}

/// Why the receiver denied a tool call WITHOUT adjudicating it (fail-closed — never a silent allow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    /// `permission_mode != "default"` — interception is only guaranteed in default mode (O-13 #10).
    NonDefaultMode,
    /// the tool is not a recognized interceptable DIRECT mutation: an unknown tool, OR a not-Direct
    /// category (MCP / `Task` subagent) which L4 denies via the §9.1 coverage matrix + the launch
    /// `permissions.deny` baseline. Conservative: anything not a covered direct tool is denied.
    UnmappedTool,
    /// the hook payload did not parse (truncated / missing a required field / wrong type).
    Malformed,
}

/// Parse a raw `PreToolUse` hook payload (the JSON the daemon-wired hook pipes on stdin). A parse
/// failure (truncated, missing a required field, wrong type) → [`DenyReason::Malformed`] (fail-closed
/// — the receiver never constructs an allow from un-parseable input).
pub fn parse_payload(json: &str) -> Result<HookPayload, DenyReason> {
    serde_json::from_str(json).map_err(|_| DenyReason::Malformed)
}

/// Map a parsed hook payload to its agent-mutation `action_type` (a catalog `agent.*` key — L1), or a
/// typed [`DenyReason`] (fail-closed). **The mode gate precedes the tool map** (O-13 #10 — a
/// non-`default` mode is a deny regardless of the tool). The DIRECT, hook-interceptable tools map to
/// their action_type; everything else (MCP / `Task` subagent / unknown) → [`DenyReason::UnmappedTool`]
/// (L4 formalizes the per-channel coverage-matrix disposition + the launch `permissions.deny`
/// hard-block — defense-in-depth). A returned action_type is the catalog key the L3 routing resolves
/// via `catalog::lookup`; a drift (a key not in the catalog) is itself fail-closed (lookup → None →
/// the policy denies it).
pub fn map_to_action_type(payload: &HookPayload) -> Result<&'static str, DenyReason> {
    if payload.permission_mode != "default" {
        return Err(DenyReason::NonDefaultMode);
    }
    match payload.tool_name.as_str() {
        "Bash" => Ok("agent.bash"),
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => Ok("agent.file_edit"),
        "Read" | "Glob" | "Grep" => Ok("agent.file_read"),
        // MCP (`mcp__*`) / `Task` (subagent) / any unknown tool → the conservative L2 deny; L4 gives
        // the §9.1-matrix reason (BestEffort / NotGuaranteed / Unsupported) + the launch hard-block.
        _ => Err(DenyReason::UnmappedTool),
    }
}

// ---- L3 — the MutationIntercept→Gateway routing + the adjudication-only verdict (INV-SEC-1) -------

/// The synchronous outcome of routing an intercepted tool call through the Gateway.
pub enum InterceptOutcome {
    /// the verdict is known NOW: a risk-0 read auto-allows (`Allow`); an ingress-deny, a Gateway error
    /// (audit-write-fail / policy-deny), or an uncatalogued type → `Deny` (fail-closed).
    Resolved(MutationVerdict),
    /// the tool is a mutation awaiting the human's decision (the action rests at `awaiting_approval`, a
    /// `proj_approval_queue` item). The verdict is [`verdict_for_status`] of the resolved status once
    /// the human approves/denies; the live wall-clock wait + the per-session `decision_sink` binding
    /// are P4. Carries the action id so that transport can poll the action's terminal status.
    AwaitingApproval { action_request_id: String },
}

/// The verdict for an adjudication action's resolved status (the §6.2 ActionRequest machine). **Allow
/// ONLY for the adjudication-allow terminals** — `PolicyDecided` (a risk-0 auto-allow) and `Approved`
/// (a human-approved mutation); EVERY other status (`Denied`/`Expired`, an UNRESOLVED
/// `AwaitingApproval` = the wait timed out, or any non-terminal) → **Deny**. The verdict DEFAULTS to
/// Deny — only the two explicit allow-terminals open the gate (fail-closed; §15 #5 — the audit event
/// for those terminals already committed by the time the status reaches them).
pub fn verdict_for_status(status: ActionRequestStatus) -> MutationVerdict {
    match status {
        ActionRequestStatus::PolicyDecided | ActionRequestStatus::Approved => {
            MutationVerdict::Allow
        }
        other => MutationVerdict::Deny {
            reason: format!("adjudication not allowed (status {other:?}) — fail-closed"),
        },
    }
}

/// Route an intercepted tool call through the EXISTING Gateway (INV-SEC-1, Option A — one chokepoint):
/// map the tool → an **adjudication-only** agent-mutation `ActionRequest` → `submit_action` → the
/// adjudication outcome. **Fail-closed (§15 #5):** an ingress-deny (non-default / unmapped / malformed)
/// or any Gateway error (audit-write-fail / policy-deny) → `Resolved(Deny)` — an Allow is gated on the
/// authoritative event committing FIRST. A risk-0 allow (the `PolicyDecided` terminal) →
/// `Resolved(Allow)`; a mutating tool → `AwaitingApproval` (the human decides). The agent-mutation
/// request carries `requester_type = AgentSession`, the session id as `requester_id`, and the tool
/// params as `inputs` (the pipeline §15-redacts them at rest); risk is recorded-not-trusted (the §6.3
/// catalog reconciles it to the authoritative locked_risk at submit — LESSON §19).
pub fn route_intercept(
    gateway: &Gateway,
    store: &mut EventStore,
    payload: &HookPayload,
) -> InterceptOutcome {
    let action_type = match map_to_action_type(payload) {
        Ok(at) => at,
        Err(reason) => return InterceptOutcome::Resolved(deny_verdict(reason)),
    };
    let req = ActionRequest {
        action_request_id: ActionRequestId::new(),
        // the session→project binding is P4; an agent-mutation intent is project-agnostic here.
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::AgentSession,
        requester_id: payload.session_id.clone(),
        resource_refs: vec![],
        inputs: payload.tool_input.clone(),
        // recorded-not-trusted (§15): the catalog reconciles risk to the AUTHORITATIVE locked_risk at
        // submit; this initial value is overwritten and never gates the decision (LESSON §19).
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        // a placeholder: `request::insert` stamps `created_at` from the daemon Clock (gtx.now), NOT
        // this field — a compile-time-constant valid RFC3339 that cannot fail to parse.
        created_at: Timestamp::parse("1970-01-01T00:00:00Z")
            .expect("placeholder created_at — request::insert stamps the real daemon-clock time; this constant always parses"),
    };
    match gateway.submit_action(store, req) {
        Ok(ack) => match ack.status {
            // risk-0 adjudication terminal → Allow (no human).
            ActionRequestStatus::PolicyDecided => {
                InterceptOutcome::Resolved(MutationVerdict::Allow)
            }
            // a mutating tool → the human decides (the verdict comes from verdict_for_status later).
            ActionRequestStatus::AwaitingApproval => InterceptOutcome::AwaitingApproval {
                action_request_id: ack.action_request_id,
            },
            // UNREACHABLE in normal operation — an adjudication submit yields ONLY PolicyDecided
            // (risk-0) or AwaitingApproval (both handled above), or an Err. This arm defends against a
            // Gateway REGRESSION that returned some other status (e.g. Queued/Succeeded — which would
            // mean an adjudication action wrongly entered execution); it fails CLOSED via the default
            // verdict (only PolicyDecided/Approved → Allow), never silently allowing.
            other => InterceptOutcome::Resolved(verdict_for_status(other)),
        },
        // a Gateway error — an audit-write fault (§15 #5) or a policy-deny — fails CLOSED to Deny.
        Err(_) => InterceptOutcome::Resolved(MutationVerdict::Deny {
            reason: "the Gateway refused the intercepted tool call (fail-closed §15 #5)"
                .to_string(),
        }),
    }
}

/// Map an ingress [`DenyReason`] to a [`MutationVerdict::Deny`] with an honest, content-free reason.
fn deny_verdict(reason: DenyReason) -> MutationVerdict {
    let r = match reason {
        DenyReason::NonDefaultMode => "non-default permission mode (O-13 #10)",
        DenyReason::UnmappedTool => {
            "tool not interceptable in default mode (coverage gap / unknown tool)"
        }
        DenyReason::Malformed => "malformed hook payload",
    };
    MutationVerdict::Deny {
        reason: r.to_string(),
    }
}

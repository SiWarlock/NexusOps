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
use crate::gateway::circuit_breaker::{classify_audit_fault, AuditBackboneBreaker, AuditOutcome};
use crate::gateway::{Gateway, GatewayError};
use crate::harness::{coverage_of, Harness, MutationChannel, MutationCoverage, MutationVerdict};
use crate::integrity::{IntegrityAlarm, IntegrityIncident};

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
    /// a RECOGNIZED tool whose §9.1 mutation channel is NOT reliably interceptable in default mode (MCP
    /// = BestEffort, `Task` subagent = NotGuaranteed, background subagent = Unsupported #27203) → the
    /// O-13 coverage-gap deny (the matrix disposition; the launch `permissions.deny` is the
    /// hook-independent second layer). Distinct from `UnmappedTool` so the operator sees WHY.
    CoverageGap,
    /// an UNKNOWN tool (not a classified Claude tool the receiver maps) — fail-closed (a new Claude tool
    /// must be classified before it is permitted).
    UnmappedTool,
    /// the hook payload did not parse (truncated / missing a required field / wrong type).
    Malformed,
}

/// The O-13 coverage-gap disposition for a §9.1 (Claude, channel) cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// the hook reliably intercepts → route the tool through the Gateway for adjudication.
    Adjudicate,
    /// the hook does NOT reliably intercept (a bypass risk) → DENY (the launch `permissions.deny` is
    /// the hook-independent second layer).
    Deny,
}

/// The §9.1 coverage-matrix disposition for a (Claude, channel) cell's `coverage` in `permission_mode`
/// (O-13). **Lead-ratified conservative ruling:** ADJUDICATE only when the hook reliably intercepts —
/// `Guaranteed` (Codex direct/mcp) or `GuaranteedInDefaultMode` (Claude direct, default mode); DENY
/// every weaker cell — `BestEffort` (Claude MCP — best-effort ≠ reliable; a default-mode hook-MISS
/// could silently proceed), `NotGuaranteed` (subagent), `Unsupported` (background subagent #27203),
/// `NotApplicable`. A non-`default` mode → Deny REGARDLESS (interception isn't guaranteed outside it).
/// The launch `permissions.deny` baseline is the hook-INDEPENDENT second layer for the denied channels.
pub fn disposition(coverage: MutationCoverage, permission_mode: &str) -> Disposition {
    if permission_mode != "default" {
        return Disposition::Deny;
    }
    match coverage {
        MutationCoverage::Guaranteed | MutationCoverage::GuaranteedInDefaultMode => {
            Disposition::Adjudicate
        }
        MutationCoverage::BestEffort
        | MutationCoverage::NotGuaranteed
        | MutationCoverage::Unsupported
        | MutationCoverage::NotApplicable => Disposition::Deny,
    }
}

/// Classify a Claude tool name into its §9.1 mutation channel + (for an interceptable DIRECT tool) its
/// agent-mutation `action_type`. `None` = an UNKNOWN tool (not a classified Claude tool → fail-closed
/// deny). The DIRECT tools carry an action_type; MCP / `Task` carry their channel but no action_type
/// (their channel's coverage denies them — only the Adjudicate disposition consumes the action_type).
fn classify_tool(tool_name: &str) -> Option<(MutationChannel, Option<&'static str>)> {
    match tool_name {
        "Bash" => Some((MutationChannel::DirectToolUse, Some("agent.bash"))),
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => {
            Some((MutationChannel::DirectToolUse, Some("agent.file_edit")))
        }
        "Read" | "Glob" | "Grep" => Some((MutationChannel::DirectToolUse, Some("agent.file_read"))),
        // P4.0b-2 (the d.2 split tool-policy) — DIRECT, interceptable in default mode → Adjudicate, so
        // each routes through the Gateway at its CATALOG risk (the catalog IS the explicit allowlist,
        // call-3 PIN): `TodoWrite` → `agent.todo_write` (risk-0 → auto-allow, the lone benign-internal);
        // `WebFetch`/`WebSearch` → `agent.web_fetch`/`agent.web_search` (risk-2 → require_approval, the
        // EGRESS/exfil dimension). An unknown tool stays `None` → `UnmappedTool` → Deny (fail-closed).
        "TodoWrite" => Some((MutationChannel::DirectToolUse, Some("agent.todo_write"))),
        "WebFetch" => Some((MutationChannel::DirectToolUse, Some("agent.web_fetch"))),
        "WebSearch" => Some((MutationChannel::DirectToolUse, Some("agent.web_search"))),
        // the DENIED channels carry NO action_type — their coverage (BestEffort / NotGuaranteed) denies
        // them, so an action_type would be dead. Returning `None` keeps it that way STRUCTURALLY: if a
        // future edit ever made these Adjudicate-eligible, the `action_type.ok_or(UnmappedTool)` on the
        // Adjudicate arm fails CLOSED (no silent resurrection of an un-interceptable channel — INV-SEC-1).
        "Task" => Some((MutationChannel::Subagent, None)),
        t if t.starts_with("mcp__") => Some((MutationChannel::McpTool, None)),
        _ => None,
    }
}

/// Parse a raw `PreToolUse` hook payload (the JSON the daemon-wired hook pipes on stdin). A parse
/// failure (truncated, missing a required field, wrong type) → [`DenyReason::Malformed`] (fail-closed
/// — the receiver never constructs an allow from un-parseable input).
pub fn parse_payload(json: &str) -> Result<HookPayload, DenyReason> {
    serde_json::from_str(json).map_err(|_| DenyReason::Malformed)
}

/// Map a parsed hook payload to its agent-mutation `action_type` (a catalog `agent.*` key — L1), or a
/// typed [`DenyReason`] (fail-closed) — **grounded in the §9.1 coverage matrix** (the O-13 disposition).
/// The gates, in order: (1) a non-`default` mode → `NonDefaultMode` (O-13 #10); (2) an UNKNOWN tool →
/// `UnmappedTool` (a new Claude tool must be classified before it is permitted); (3) the tool's §9.1
/// channel coverage drives [`disposition`] — `Adjudicate` → the DIRECT tool's action_type; `Deny` → a
/// `CoverageGap` (MCP BestEffort / `Task` NotGuaranteed — the launch `permissions.deny` is the
/// hook-independent second layer). A returned action_type is the catalog key the L3 routing resolves
/// via `catalog::lookup`; a drift (a key not in the catalog) is itself fail-closed (lookup → None →
/// the policy denies it).
pub fn map_to_action_type(payload: &HookPayload) -> Result<&'static str, DenyReason> {
    if payload.permission_mode != "default" {
        return Err(DenyReason::NonDefaultMode);
    }
    let (channel, action_type) =
        classify_tool(&payload.tool_name).ok_or(DenyReason::UnmappedTool)?;
    // the §9.1 matrix decides adjudicate-vs-deny. A missing cell (cannot happen — all 8 are defined)
    // fails closed to Unsupported → Deny.
    let coverage = coverage_of(Harness::Claude, channel).unwrap_or(MutationCoverage::Unsupported);
    match disposition(coverage, &payload.permission_mode) {
        // only DIRECT tools reach Adjudicate, and they always carry an action_type; an adjudicable
        // channel with no action_type would be a classify_tool bug → fail closed (UnmappedTool).
        Disposition::Adjudicate => action_type.ok_or(DenyReason::UnmappedTool),
        Disposition::Deny => Err(DenyReason::CoverageGap),
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
    route_intercept_classified(gateway, store, payload).0
}

/// The PRODUCTION caller's route (the write-actor's `GatewayIntercept` handler, C2): route the tool +
/// on an audit-WRITE-fault (call-2) raise the §17 [`IntegrityAlarm`] on the INDEPENDENT durable channel
/// (NOT the broken audit-write path), keyed STRUCTURALLY off `GatewayDenyKind::AuditWriteFailed` —
/// never a string-match (the L1 code-quality flag (i)). The alarm fires for the audit-fault ONLY; a
/// policy-deny is a routine block (the `Ok(Denied)` path, no alarm). The verdict is UNCHANGED — both
/// fail closed to Deny; the alarm is the loud integrity signal, never the gate. The unit tests call
/// [`route_intercept`] directly (no alarm); only the live path needs the sink.
pub fn route_intercept_live(
    gateway: &Gateway,
    store: &mut EventStore,
    payload: &HookPayload,
    alarm: &dyn IntegrityAlarm,
    breaker: Option<&AuditBackboneBreaker>,
) -> InterceptOutcome {
    let (outcome, audit_fault) = route_intercept_classified(gateway, store, payload);
    if let Some((action_type, fault_outcome)) = audit_fault {
        // the audited mutator could not AUDIT → the §17 best-practice surface: deny (in `outcome`) +
        // this durable off-DB per-incident alarm. `detail` is content-free by construction (the
        // action_type only).
        alarm.raise(IntegrityIncident::audit_write_failed(action_type));
        // P4.0b-2c (Q4) — the intercept audit-fault is ALSO part of the audit backbone the daemon-wide
        // breaker observes: feed it the recoverable/unrecoverable classification (the breaker trips +
        // raises the SYSTEMIC alarm on N-consecutive / an unrecoverable class). The intercept's
        // GatewayError is consumed here (the caller gets only an `InterceptOutcome`), so the breaker is
        // fed HERE rather than via the write-actor's `observe_gateway_result` (which sees submit/approve/
        // deny `Result`s). A `None` breaker (the no-alarm test path) skips the feed.
        if let Some(b) = breaker {
            b.observe(fault_outcome);
        }
    }
    outcome
}

/// The shared classifier: route the tool through the Gateway and return the [`InterceptOutcome`] PLUS
/// `(action_type, AuditOutcome)` IFF the Gateway failed with an **audit-write-fault** (so
/// [`route_intercept_live`] can raise the §17 per-incident alarm with the structural action_type AND
/// feed the daemon-wide breaker the recoverable/unrecoverable classification — P4.0b-2c). `None` for
/// every non-audit-fault path (an allow, an await, a policy-deny, an ingress-deny) — only the
/// audit-backbone fault is the §17 signal.
fn route_intercept_classified(
    gateway: &Gateway,
    store: &mut EventStore,
    payload: &HookPayload,
) -> (InterceptOutcome, Option<(&'static str, AuditOutcome)>) {
    let action_type = match map_to_action_type(payload) {
        Ok(at) => at,
        Err(reason) => return (InterceptOutcome::Resolved(deny_verdict(reason)), None),
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
        Ok(ack) => {
            let outcome = match ack.status {
                // risk-0 adjudication terminal → Allow (no human).
                ActionRequestStatus::PolicyDecided => {
                    InterceptOutcome::Resolved(MutationVerdict::Allow)
                }
                // a mutating tool → the human decides (the verdict comes from verdict_for_status later).
                ActionRequestStatus::AwaitingApproval => InterceptOutcome::AwaitingApproval {
                    action_request_id: ack.action_request_id,
                },
                // an adjudication submit yields PolicyDecided (risk-0) or AwaitingApproval (both handled
                // above), or — since 043 L5 — `Denied` (the deny-rule record-then-deny commits the denied
                // action, ack'd Denied), or an Err. `Denied` → verdict_for_status → Deny (the agent IS
                // blocked, the blocked attempt audited). Any OTHER status would be a Gateway REGRESSION
                // (e.g. Queued/Succeeded = an adjudication action wrongly executed); it fails CLOSED via
                // the default verdict (only PolicyDecided/Approved → Allow), never silently allowing.
                other => InterceptOutcome::Resolved(verdict_for_status(other)),
            };
            // an Ok path is never an audit-backbone fault (the audit event committed) → no §17 alarm.
            (outcome, None)
        }
        // a Gateway error fails CLOSED to Deny — but call 2 DISTINGUISHES the kind: a §17/§15 #5
        // AUDIT-WRITE-FAULT (the adjudication audit event could not commit) is a LOUD integrity alert,
        // not a routine deny. `route_intercept_live` raises the §17 `IntegrityAlarm` keyed off this
        // kind; both still fail-closed to Deny. (A policy-deny is a SEPARATE `Ok(Denied)` path above,
        // never an `Err` — `verdict_for_status(Denied)` → routine Deny, no alarm.)
        Err(e) => {
            let kind = classify_gateway_error(&e);
            let outcome = InterceptOutcome::Resolved(MutationVerdict::Deny {
                reason: match kind {
                    GatewayDenyKind::AuditWriteFailed =>
                        "AUDIT-WRITE-FAILED (§17/§15 #5 integrity alert): the adjudication audit event \
                         could not be committed — fail-closed Deny. A loud integrity fault, NOT a routine deny."
                            .to_string(),
                    GatewayDenyKind::Other =>
                        "the Gateway refused the intercepted tool call (fail-closed §15 #5)".to_string(),
                },
            });
            // surface the action_type + the recoverable/unrecoverable classification ONLY on the
            // audit-backbone fault (P4.0b-2c) — the per-incident alarm uses the action_type, the
            // daemon-wide breaker consumes the AuditOutcome.
            let audit_fault = match &e {
                GatewayError::AuditWriteFailed(inner) => {
                    Some((action_type, classify_audit_fault(inner)))
                }
                _ => None,
            };
            (outcome, audit_fault)
        }
    }
}

/// The kind of a Gateway `Err` for the call-2 audit-fault distinction. A §17/§15 #5
/// [`GatewayDenyKind::AuditWriteFailed`] (the adjudication audit event could not commit) is a LOUD
/// integrity alert the live drive loop surfaces distinctly; everything else is a routine fail-closed
/// deny. **Both fail closed to Deny** — the distinction drives the §17 signal, never the gate. (A
/// POLICY-deny never reaches here: it is the `Ok(Denied)` path → [`verdict_for_status`] → routine Deny.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayDenyKind {
    AuditWriteFailed,
    Other,
}

/// Classify a Gateway `Err` for the §17 audit-fault distinction (call 2).
pub fn classify_gateway_error(err: &GatewayError) -> GatewayDenyKind {
    match err {
        GatewayError::AuditWriteFailed(_) => GatewayDenyKind::AuditWriteFailed,
        _ => GatewayDenyKind::Other,
    }
}

/// Map an ingress [`DenyReason`] to a [`MutationVerdict::Deny`] with an honest, content-free reason.
fn deny_verdict(reason: DenyReason) -> MutationVerdict {
    let r = match reason {
        DenyReason::NonDefaultMode => "non-default permission mode (O-13 #10)",
        DenyReason::CoverageGap => {
            "tool channel not reliably interceptable in default mode (§9.1 coverage gap — MCP/subagent)"
        }
        DenyReason::UnmappedTool => "unknown tool (not classified for interception)",
        DenyReason::Malformed => "malformed hook payload",
    };
    MutationVerdict::Deny {
        reason: r.to_string(),
    }
}

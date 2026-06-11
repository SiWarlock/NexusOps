//! JSON-Schema emission (ARCHITECTURE §5.0, Option A).
//!
//! The Rust authority's enums are bundled into one schema (value sets land in
//! `$defs`); the checked-in artifact in `contracts/schema/` is regenerated from
//! here and CI-diff-gated (test 9). The Python Brain (Pydantic) + TS UI (Zod)
//! generate their consumers from this neutral artifact. Output is deterministic
//! so the diff gate is stable.

use schemars::JsonSchema;

use crate::actions::{
    ActionDependency, ActionPlan, ActionPlanStep, ActionPreview,
    ActionRequest as ActionRequestModel, ActionResult, ActionResultStatus, ActorRefBody,
    Approval as ApprovalModel, ApprovalMode, ApprovalScope, EvidenceConfidence, EvidenceRef,
    EvidenceType, PolicyDecision, PolicyDecisionStatus, RequesterType, RequiredApprover,
    RequiredApproverKind, ResourceRef, ResourceType, RiskLevel,
};
use crate::actor::ActorType;
use crate::event_envelope::{EventEnvelope, RedactionStatus, Sensitivity, SourceType, Visibility};
use crate::events::{
    ActionApprovalRequested, ActionApproved, ActionDenied, ActionExpired, ActionFailed,
    ActionRequested, ActionStarted, ActionSucceeded, AuditIntegrityViolation, DeviceRegistered,
    LocalRunnerRegistered, SensitiveOutputRedacted, SessionStarted,
};
use crate::gateway_ids::{ActionPlanId, ApprovalId, GatewayObjectKind};
use crate::ids::IdKind;
use crate::ipc::{
    ActionAck, Capabilities, DeltaKind, GetProjectionParams, HelloAck, HelloFrame, IpcErrorCode,
    ProjectionDelta, ProjectionName, ProjectionScope, RpcRequest, RpcResponse, ServerFrame,
    SubscribeParams, VersionSkewError, WireError,
};
use crate::objects::{DesktopObjectKind, DeviceId, LocalRunnerId};
use crate::status::{
    ActionRequest, AgentTeam, Approval, ProjectBrain, PullRequest, Session, Task, WorkflowInstance,
    WorktreeGit, WorktreeOverlay,
};
use crate::time::Timestamp;
use crate::CONTRACT_VERSION;

/// Bundles every frozen contract type so one `schema_for!` captures all value
/// sets under `$defs`. Never instantiated — it exists only to drive schema gen.
#[derive(JsonSchema)]
#[allow(dead_code)]
struct ContractBundle {
    session: Session,
    task: Task,
    worktree_git: WorktreeGit,
    worktree_overlay: WorktreeOverlay,
    pull_request: PullRequest,
    workflow_instance: WorkflowInstance,
    project_brain: ProjectBrain,
    approval: Approval,
    action_request: ActionRequest,
    agent_team: AgentTeam,
    actor_type: ActorType,
    id_kind: IdKind,
    desktop_object_kind: DesktopObjectKind,
    // 1.1 (L1) — Event envelope + the 3 new enums (§7.1)
    event_envelope: EventEnvelope,
    source_type: SourceType,
    sensitivity: Sensitivity,
    visibility: Visibility,
    redaction_status: RedactionStatus,
    // 1.2 — first concrete event-type payload (§7.1 EventTypeRegistry)
    session_started: SessionStarted,
    // 1.6a L3 — Device/LocalRunner registration event payloads + their id newtypes (§5.3/§16)
    device_registered: DeviceRegistered,
    local_runner_registered: LocalRunnerRegistered,
    device_id: DeviceId,
    local_runner_id: LocalRunnerId,
    // 1.6c L2 — the §17 audit-integrity event payload (Option C "loud record")
    audit_integrity_violation: AuditIntegrityViolation,
    // 1.7 L2 — the §15 quarantine-divert event payload (can't-safely-redact → divert)
    sensitive_output_redacted: SensitiveOutputRedacted,
    // 1.5 L2 — the IPC GatewayPort wire contract (§6.4) + the §6.1 projection-name enum
    hello_frame: HelloFrame,
    hello_ack: HelloAck,
    version_skew_error: VersionSkewError,
    capabilities: Capabilities,
    ipc_error_code: IpcErrorCode,
    wire_error: WireError,
    projection_name: ProjectionName,
    // 1.5 L3 — the §6.1 JSON-RPC method request/response envelopes + get_projection params
    rpc_request: RpcRequest,
    rpc_response: RpcResponse,
    get_projection_params: GetProjectionParams,
    projection_scope: ProjectionScope,
    // 1.5 L4 — frame-type multiplexing envelope + subscribe streaming (§6.4/§6.1)
    server_frame: ServerFrame,
    projection_delta: ProjectionDelta,
    delta_kind: DeltaKind,
    subscribe_params: SubscribeParams,
    // 2.1a — the §6.2 Gateway core data model freeze (shared/src/actions.rs): the 10 models +
    // 9 enums + RequiredApprover + the gateway IDs + Timestamp. RiskLevel/Timestamp emit as a
    // bounded integer / string-format (NOT enum arrays) so the §5.0 3-way verify stays exact.
    action_request_model: ActionRequestModel,
    action_plan: ActionPlan,
    action_plan_step: ActionPlanStep,
    action_dependency: ActionDependency,
    action_preview: ActionPreview,
    approval_model: ApprovalModel,
    action_result: ActionResult,
    resource_ref: ResourceRef,
    evidence_ref: EvidenceRef,
    policy_decision: PolicyDecision,
    required_approver: RequiredApprover,
    required_approver_kind: RequiredApproverKind,
    actor_ref_body: ActorRefBody,
    requester_type: RequesterType,
    risk_level: RiskLevel,
    approval_scope: ApprovalScope,
    approval_mode: ApprovalMode,
    policy_decision_status: PolicyDecisionStatus,
    action_result_status: ActionResultStatus,
    resource_type: ResourceType,
    evidence_type: EvidenceType,
    evidence_confidence: EvidenceConfidence,
    gateway_object_kind: GatewayObjectKind,
    approval_id: ApprovalId,
    action_plan_id: ActionPlanId,
    timestamp: Timestamp,
    // 2.1b — the Gateway ActionExecution* EventTypeRegistry payloads (§7.1/AG§17.1) + the §6.1
    // submit-result ActionAck. Additive event family for the INV-SEC-1 chokepoint.
    action_requested: ActionRequested,
    action_approval_requested: ActionApprovalRequested,
    action_approved: ActionApproved,
    action_denied: ActionDenied,
    action_expired: ActionExpired,
    action_started: ActionStarted,
    action_succeeded: ActionSucceeded,
    action_failed: ActionFailed,
    action_ack: ActionAck,
}

/// The canonical, versioned JSON-Schema string (trailing newline). Deterministic:
/// same Rust authority → byte-identical output (the diff-gate invariant).
pub fn emit_schema_json() -> String {
    let schema = schemars::schema_for!(ContractBundle);
    // infallible: a schemars-derived Schema is always a serializable JSON value.
    let mut value = serde_json::to_value(&schema).expect("schemars Schema → JSON value");
    if let serde_json::Value::Object(map) = &mut value {
        map.insert(
            "title".to_string(),
            serde_json::Value::String("nexusops-contract".to_string()),
        );
        map.insert(
            "x-contract-version".to_string(),
            serde_json::Value::String(CONTRACT_VERSION.to_string()),
        );
    }
    // infallible: `value` is a plain JSON object built above, always pretty-printable.
    let mut out = serde_json::to_string_pretty(&value).expect("JSON value → pretty string");
    out.push('\n');
    out
}

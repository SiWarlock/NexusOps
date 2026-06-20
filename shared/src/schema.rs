//! JSON-Schema emission (ARCHITECTURE §5.0, Option A).
//!
//! The Rust authority's enums are bundled into one schema (value sets land in
//! `$defs`); the checked-in artifact in `contracts/schema/` is regenerated from
//! here and CI-diff-gated (test 9). The Python Brain (Pydantic) + TS UI (Zod)
//! generate their consumers from this neutral artifact. Output is deterministic
//! so the diff gate is stable.

use schemars::JsonSchema;

use crate::actions::{
    ActionDependency, ActionError, ActionPlan, ActionPlanStep, ActionPreview,
    ActionRequest as ActionRequestModel, ActionResult, ActionResultStatus, ActorRefBody,
    Approval as ApprovalModel, ApprovalMode, ApprovalScope, EvidenceConfidence, EvidenceRef,
    EvidenceType, PolicyDecision, PolicyDecisionStatus, RequesterType, RequiredApprover,
    RequiredApproverKind, ResourceRef, ResourceType, RiskLevel,
};
use crate::actor::ActorType;
use crate::catalog::{ActionTypeCatalogEntry, ExecutorKind, IdempotencyFormula, PreviewClass};
use crate::event_envelope::{EventEnvelope, RedactionStatus, Sensitivity, SourceType, Visibility};
use crate::events::{
    ActionApprovalRequested, ActionApproved, ActionDenied, ActionExpired, ActionFailed,
    ActionPartiallySucceeded, ActionRequested, ActionStarted, ActionSucceeded,
    AuditIntegrityViolation, BranchCreated, DeviceRegistered, GithubSyncFailed,
    IntegrationConnectionRegistered, LinearSyncFailed, LocalRunnerRegistered, ProjectRescanned,
    Provider, PullRequestMerged, PullRequestSynced, ReviewSubmitted, ReviewSynced,
    SensitiveOutputRedacted, SessionFailed, SessionRecovered, SessionStarted, TelemetrySampled,
    TerminalProcessExited, WorktreeCreated, WorktreeDeleted, WorktreeLocked, WorktreeMerged,
    WorktreePrunable,
};
use crate::gateway_ids::{ActionPlanId, ApprovalId, GatewayObjectKind};
use crate::harness::{
    HarnessCapabilities, MetricQuality, RecoveryState, ResumeMode, ResumeResult, TelemetrySample,
    TranscriptRef,
};
use crate::ids::IdKind;
use crate::ipc::{
    ActionAck, Capabilities, DeltaKind, DiffLine, DiffLineKind, DiffResult, GetDiffParams,
    GetPrDiffParams, GetProjectionParams, HelloAck, HelloFrame, Hunk, IpcErrorCode, PlanAck,
    PlanStepAck, ProjectionDelta, ProjectionName, ProjectionScope, RpcRequest, RpcResponse,
    ServerFrame, SubscribeParams, TerminalControlFrame, TerminalControlKind, TerminalInputFrame,
    TerminalOutputFrame, VersionSkewError, WireError,
};
use crate::objects::{DesktopObjectKind, DeviceId, LocalRunnerId};
use crate::projections::{ApprovalQueueRow, PullRequestRow, ReviewRow, SessionRow};
use crate::status::{
    ActionRequest, AgentTeam, Approval, ExecutionProfile, ProjectBrain, PullRequest, ReviewState,
    Session, Task, WorkflowInstance, WorktreeGit, WorktreeOverlay,
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
    // 0.5b (P4.0b-1) — the 10th §5.1 machine: ExecutionProfile runtime state (9 values).
    execution_profile: ExecutionProfile,
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
    // P4.0b-ui1 — the §6.1 get_diff RPC wire types (the ui-6.3e diff source). DiffResult transitively
    // pulls Hunk/DiffLine/DiffLineKind into $defs; listed explicitly for a stable named snapshot.
    get_diff_params: GetDiffParams,
    // D7 — the §6.1 get_pr_diff RPC params (remote-PR head-vs-base); reuses DiffResult/Hunk/DiffLine.
    get_pr_diff_params: GetPrDiffParams,
    diff_result: DiffResult,
    hunk: Hunk,
    diff_line: DiffLine,
    diff_line_kind: DiffLineKind,
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
    // 2.4 L1 — the §17 failure-mode additions: the side-effect-applied-but-event-unwritable event +
    // the structured ActionError taxonomy now carried on ActionFailed. Additive (CONTRACT 0.19.0).
    action_partially_succeeded: ActionPartiallySucceeded,
    action_error: ActionError,
    action_ack: ActionAck,
    // 2.1c — the §6.1 `submit_action_plan` result wire type (O-3 bundled plans). Additive.
    plan_ack: PlanAck,
    plan_step_ack: PlanStepAck,
    // 2.2 — the §6.3 ActionTypeCatalog contract (the per-type entry + its enums). The catalog DATA
    // (the per-type table) lives in `catalog::lookup`; the schema captures the entry TYPE. Additive.
    action_type_catalog_entry: ActionTypeCatalogEntry,
    preview_class: PreviewClass,
    executor_kind: ExecutorKind,
    idempotency_formula: IdempotencyFormula,
    // 3.1 — the §9.1 HarnessAdapter normalized return types (shared/src/harness.rs) + the §7.1
    // TelemetrySampled telemetry-observation event. NormalizedStatus is the frozen `Session` $def
    // (already registered above), not a new type. The HarnessAdapter trait + the mutation-coverage
    // matrix are DAEMON-INTERNAL (not a wire contract); ResumeResult freezes at 4.1a (below).
    // Additive (CONTRACT 0.20.0).
    telemetry_sample: TelemetrySample,
    metric_quality: MetricQuality,
    transcript_ref: TranscriptRef,
    harness_capabilities: HarnessCapabilities,
    telemetry_sampled: TelemetrySampled,
    // P4.1a (CONTRACT 0.29.0) — the §8/§9.1 survival contract freeze: ResumeMode(4) + RecoveryState(3)
    // + ResumeResult{mode,replayed_event_count} (the deep-dive §7.2 B2-strict freeze; reconciles the ui
    // provisional). The decide_resume ladder + the broker are DAEMON-INTERNAL (4.1a c2 / 4.1b). Additive.
    resume_mode: ResumeMode,
    recovery_state: RecoveryState,
    resume_result: ResumeResult,
    // 3.4 — the §6.4 Terminal Channel wire contract (shared/src/ipc.rs): the 3 terminal frames +
    // the TerminalControlKind flow-control enum + the §7.1 TerminalProcessExited observation event.
    // ServerFrame (already registered above) gains the TerminalOutput variant — the reserved slot
    // filled (JSON-base64 MVP; binary fast-path deferred to 3.5). Additive (CONTRACT 0.21.0).
    terminal_output_frame: TerminalOutputFrame,
    terminal_input_frame: TerminalInputFrame,
    terminal_control_frame: TerminalControlFrame,
    terminal_control_kind: TerminalControlKind,
    terminal_process_exited: TerminalProcessExited,
    // P4.0b-R1b (CONTRACT 0.26.0) — the Phase-5/7 wiring EventTypeRegistry payloads (edges-R1
    // §2.5-seam): P5.1 project detection, P5.2 worktree/branch lifecycle (+ 4 empty-payload overlay
    // transitions), P7.1 integration reads + non-auth sync failures + the closed `Provider` enum
    // (github|linear, flat enum → 3-way verify exact). shared/-only; edges emits at P5/P7. Additive.
    project_rescanned: ProjectRescanned,
    worktree_created: WorktreeCreated,
    branch_created: BranchCreated,
    worktree_merged: WorktreeMerged,
    worktree_prunable: WorktreePrunable,
    worktree_deleted: WorktreeDeleted,
    worktree_locked: WorktreeLocked,
    provider: Provider,
    pull_request_synced: PullRequestSynced,
    integration_connection_registered: IntegrationConnectionRegistered,
    github_sync_failed: GithubSyncFailed,
    linear_sync_failed: LinearSyncFailed,
    // P4.0b-ui2 (CONTRACT 0.30.0) — the FIRST frozen projection-row: ApprovalQueueRow (the
    // proj_approval_queue read model the §11.5 approval card consumes, typed; risk_level +
    // policy_decision: Option<PolicyDecision>). Additive (shared/src/projections.rs).
    approval_queue_row: ApprovalQueueRow,
    // P4.1b-1 (CONTRACT 0.31.0) — the §8.1 daemon-restart recovery OBSERVATION event (the §11.4
    // resumed-vs-replayed bit + the §17 "restart session" affordance). System-actor, write-actor, NOT
    // a Gateway Action (Q1=(a)); §15 #8 profile preserved. Additive (shared/src/events.rs).
    session_recovered: SessionRecovered,
    // P4.2 (CONTRACT 0.32.0) — the §17 supervised-child-death OBSERVATION event (a session's child died,
    // daemon alive → proj_session status=Failed → the §11.4 restart affordance). Empty-payload
    // (WorktreeMerged precedent); System-actor, write-actor, NOT a Gateway Action. Additive (events.rs).
    session_failed: SessionFailed,
    // P7.2 (CONTRACT 0.34.0) — the 2nd frozen projection-row: PullRequestRow (the proj_pull_request
    // GitHub-authoritative read cache the ui PR Review Workspace consumes, typed; the BASIC columns the
    // edges-P7.1 projector folds — mergeable/checks_summary are a later SPREAD). Additive
    // (shared/src/projections.rs).
    pull_request_row: PullRequestRow,
    // D2/P4.4 (CONTRACT 0.35.0) — the 3rd frozen projection-row: SessionRow (the proj_session read model
    // the ui per-session recovery indicator + RecoveryState banner consume, typed; status: Session +
    // the §8.1/§11.4 recovery fields folded from the now-consumed SessionRecovered). Additive
    // (shared/src/projections.rs).
    session_row: SessionRow,
    // D5b-1/P4.6 (CONTRACT 0.37.0) — the structured-review vertical: the ReviewSynced event + the
    // ReviewState value enum + the 4th frozen projection-row ReviewRow (the proj_review read model the
    // ui PR Review Workspace consumes, typed; the live GitHub producer is D5b-2). ProjectionName::Review
    // rides the existing ProjectionName field above. Additive (events/status/projections).
    review_synced: ReviewSynced,
    review_state: ReviewState,
    review_row: ReviewRow,
    // D9/P4.7 (CONTRACT 0.41.0) — the cat-1 github.merge_pr mutation's OBSERVATION event: PullRequestMerged
    // (the gateway emits it on a successful merge; the PullRequestProjector folds → terminal Merged). The
    // catalog entry rides the existing ActionTypeCatalogEntry registration. Additive (shared/src/events.rs).
    pull_request_merged: PullRequestMerged,
    // D10/P4.7 (CONTRACT 0.42.0) — the cat-1 github.submit_review mutation's OBSERVATION event:
    // ReviewSubmitted (the gateway emits it on a successful review-submit; the ReviewProjector folds →
    // proj_review). Reuses the frozen ReviewState; the catalog entry rides the existing
    // ActionTypeCatalogEntry registration. Additive (shared/src/events.rs).
    review_submitted: ReviewSubmitted,
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

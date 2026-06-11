//! §6.2 Gateway core data model — the frozen action contract (ARCHITECTURE §6.2, AG §9.x).
//!
//! The typed `ActionRequest` / `ActionPlan` / `Approval` / `ActionResult` family + their enums.
//! **Binding field sets:** `ARCHITECTURE.md` Appendix A (rows ActionRequest/ActionPlan/Approval/
//! ActionResult/ResourceRef·EvidenceRef·PolicyDecision) + `DATA_MODEL §2.9` (the
//! `action_requests`/`approvals` DDL). `docs/domains/ACTION_GATEWAY.md §9.x` is ORIGIN/RATIONALE
//! (richer) — this freezes the **reconciled** binding set; AG-reconciled-out fields and the
//! sub-types whose shapes AG never pinned are **deferred to their owning slice** (documented at
//! each `// DEFERRED →` marker), never silently dropped:
//!   - `ActionPlan.rollback_plan` → 2.4 · `Approval.constraints` → 2.2 (RollbackPlan/
//!     ApprovalConstraint shapes are unpinned — not invented here).
//!   - `ActionPreview` per-class previews (command/diff/api/session/workflow/rollback) → 2.3.
//!   - `PolicyDecision.{required_approvals,constraints,safer_alt}` → 2.2 (the policy engine).
//!   - `ActionResult.error` structured taxonomy → 2.4 (`Option<String>` message for now).
//!
//! All deferrals are additive-later (a new optional field = an additive `CONTRACT_VERSION` bump,
//! the `events.rs` accretion convention). Reject-unknown end-to-end via
//! `#[serde(deny_unknown_fields)]` (§5.0/§15). Optionals serialize as `null` (NO
//! `skip_serializing_if`) so the wire field set is **stable** for the generated Zod/Pydantic +
//! the §2.5-seam snapshot guard.

use std::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::actor::ActorType;
use crate::gateway_ids::{ActionPlanId, ApprovalId};
use crate::ids::{ActionRequestId, EventId, ProjectId};
use crate::status::{ActionRequestStatus, ApprovalStatus};
use crate::time::Timestamp;

/// A frozen, snake_case-wire, reject-unknown value enum + its `ALL` (mirrors
/// `event_envelope::wire_enum!`). Closed set: an unknown wire value fails closed at parse (§15).
macro_rules! wire_enum {
    ($(#[$m:meta])* $name:ident { $($v:ident),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($v),+ }
        impl $name {
            /// Every variant, declaration order.
            pub const ALL: &'static [Self] = &[ $(Self::$v),+ ];
        }
    };
}

wire_enum! {
    /// Request-time requester identity (§6.2 / R-2). The AG §9.7 `ActorRef` variants +
    /// `remote_client` (§6). Mapped to the audit [`ActorType`] via [`RequesterType::to_actor_type`].
    RequesterType { User, ProjectBrain, AgentSession, WorkflowPack, SystemPolicy, RemoteClient }
}

wire_enum! {
    /// The approval's authority scope (§6.2 Approval).
    ApprovalScope { SingleAction, Plan, PolicyGrant }
}

wire_enum! {
    /// How a bundled [`ActionPlan`] is approved (§6.2, O-3).
    ApprovalMode { ApproveAll, StepByStep, Mixed, Blocked }
}

wire_enum! {
    /// The policy engine's decision (§6.2 / AG §12.2). The full decision payload
    /// (required_approvals / constraints / safer_alt) is the 2.2 policy-engine deliverable.
    PolicyDecisionStatus {
        Allow, RequireApproval, RequireStepApproval, Deny, Downgrade, NeedsMoreContext
    }
}

wire_enum! {
    /// Terminal outcome of an executed action (§6.2 ActionResult).
    ActionResultStatus { Succeeded, Failed, PartiallySucceeded, Cancelled }
}

wire_enum! {
    /// The kind of resource an action touches (§6.2 ResourceRef; AG §9.8's 20 values VERBATIM).
    /// (§6.2/Appendix-A say "21 types"; the 20-vs-21 gap is a tracked cross-doc note — the 21st
    /// is NOT invented here.)
    ResourceType {
        Project, Repo, Worktree, Branch, File, Diff, Session, AgentTeam, PlanTask,
        ImplementationPlan, WorkflowPack, WorkflowInstance, WorkflowCommand, LinearIssue,
        GithubIssue, PullRequest, ExecutionProfile, MemorySource, Decision, Artifact,
    }
}

wire_enum! {
    /// The kind of evidence backing an action/plan (§6.2 EvidenceRef; AG §9.9's 11 values).
    EvidenceType {
        ProjectBrainEvidenceItem, FileAnchor, ArchitectureAnchor, PlanTask, SessionEpisode,
        Commit, PrCheck, TerminalEvent, Ticket, WorkflowManifest, UserInstruction,
    }
}

wire_enum! {
    /// Evidence-link confidence (§6.2 EvidenceRef; AG §9.9).
    EvidenceConfidence { Exact, Likely, Loose, Unverified }
}

/// Action risk 0–4 (§6.2 / AG §7) — drives preview depth + approval. A closed 5-variant enum
/// (type-safe + `Ord` for the §6.3 risk-range comparisons), serialized as the **integer** 0–4
/// (the DDL `risk_level INTEGER` / AG wire). Its schema is a **bounded integer**
/// (`{type:integer, minimum:0, maximum:4}`), NOT an integer `enum` array — the §5.0 3-way verify
/// reflects string enums only, so an integer enum-array would diverge from the generated
/// Pydantic/Zod; min/max is consistently skipped across all three. Out-of-range integers fail
/// closed at parse (§15).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RiskLevel {
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
}

impl RiskLevel {
    /// Every level, ascending (== integer rank).
    pub const ALL: &'static [Self] = &[
        Self::Level0,
        Self::Level1,
        Self::Level2,
        Self::Level3,
        Self::Level4,
    ];
}

impl Serialize for RiskLevel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for RiskLevel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match u8::deserialize(deserializer)? {
            0 => Ok(Self::Level0),
            1 => Ok(Self::Level1),
            2 => Ok(Self::Level2),
            3 => Ok(Self::Level3),
            4 => Ok(Self::Level4),
            other => Err(serde::de::Error::custom(format!(
                "risk_level out of range 0..=4: {other}"
            ))),
        }
    }
}

impl JsonSchema for RiskLevel {
    fn schema_name() -> Cow<'static, str> {
        "RiskLevel".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::RiskLevel").into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "integer",
            "minimum": 0,
            "maximum": 4
        })
    }
}

impl RequesterType {
    /// The audit [`ActorType`] this request-time requester maps to (Appendix A / R-2): the 3
    /// aliases (`agent_session→session_adapter`, `workflow_pack→workflow_runtime`,
    /// `system_policy→automation_policy`) + 3 straight-throughs (`user`/`project_brain`/
    /// `remote_client`). The Gateway stamps the audited event with this `ActorType`.
    pub fn to_actor_type(self) -> ActorType {
        match self {
            Self::User => ActorType::User,
            Self::ProjectBrain => ActorType::ProjectBrain,
            Self::AgentSession => ActorType::SessionAdapter,
            Self::WorkflowPack => ActorType::WorkflowRuntime,
            Self::SystemPolicy => ActorType::AutomationPolicy,
            Self::RemoteClient => ActorType::RemoteClient,
        }
    }
}

wire_enum! {
    /// Which class of approver a §6.2 [`Approval`] requires (AG §9.5 `ActorRef | 'current_user' |
    /// 'project_owner'`). A flat closed enum → a clean top-level schema `enum`, reflected
    /// uniformly by the §5.0 Zod/Pydantic generators (a data-carrying tagged enum's *inline*
    /// value-set is lifted inconsistently across the 3-way verify — so the discriminant is a
    /// named enum, and the specific actor lives in [`RequiredApprover::actor`]).
    RequiredApproverKind { CurrentUser, ProjectOwner, Actor }
}

/// Who must approve (§6.2 Approval; AG §9.5). `kind` selects the approver class; `actor` carries
/// the specific actor IFF `kind == actor` (the Gateway enforces that pairing at 2.1b). Modeled as
/// a `{kind, actor?}` object (not a data-carrying enum) so the wire stays uniform + 3-way
/// verifiable (the §5.0 cross-language equality gate).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct RequiredApprover {
    pub kind: RequiredApproverKind,
    pub actor: Option<ActorRefBody>,
}

impl RequiredApprover {
    /// the current interactive user must approve
    pub fn current_user() -> Self {
        Self {
            kind: RequiredApproverKind::CurrentUser,
            actor: None,
        }
    }

    /// the project owner must approve
    pub fn project_owner() -> Self {
        Self {
            kind: RequiredApproverKind::ProjectOwner,
            actor: None,
        }
    }

    /// a specific actor must approve
    pub fn actor(actor: ActorRefBody) -> Self {
        Self {
            kind: RequiredApproverKind::Actor,
            actor: Some(actor),
        }
    }
}

/// A specific approving actor (§6.2 Approval; the `actor` of a [`RequiredApprover`]).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActorRefBody {
    pub actor_type: RequesterType,
    pub actor_id: String,
}

/// A reference to a resource an action reads/mutates (§6.2 ResourceRef; AG §9.8). The leaner
/// binding set `{type, id, uri?}` — AG's `displayName?` is resolvable from `id` (DEFERRED;
/// additive-later if a consumer needs it).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ResourceRef {
    #[serde(rename = "type")]
    pub resource_type: ResourceType,
    pub id: String,
    pub uri: Option<String>,
}

/// A reference to evidence backing an action/plan (§6.2 EvidenceRef; AG §9.9). `label` is the
/// requester-supplied natural-language evidence text (required; not reconstructable from
/// `{type,id}` — the Brain/PR-review evidence surface depends on it, TWEAK 2026-06-11).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct EvidenceRef {
    #[serde(rename = "type")]
    pub evidence_type: EvidenceType,
    pub id: String,
    pub label: String,
    pub confidence: Option<EvidenceConfidence>,
}

/// The policy engine's decision (§6.2 / AG §12.2). Frozen as the stable core `{status, reasons}`
/// — `required_approvals` / `constraints` / `safer_alt` (which reference the unpinned
/// ApprovalRequirement/ApprovalConstraint shapes + the safer-alt union) are the **2.2** policy-
/// engine deliverable, added additively. DEFERRED → 2.2.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct PolicyDecision {
    pub status: PolicyDecisionStatus,
    pub reasons: Vec<String>,
}

/// A dry-run preview of an action (§6.2 ActionPreview envelope; AG §9.4). The 6 typed per-class
/// previews (command/diff/api/session/workflow/rollback) are **2.3**'s deliverable. DEFERRED → 2.3.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionPreview {
    pub action_request_id: ActionRequestId,
    pub generated_at: Timestamp,
    pub risk_level: RiskLevel,
    pub risk_reasons: Vec<String>,
    pub summary: String,
    pub changed_resources: Vec<ResourceRef>,
    pub cannot_preview_reason: Option<String>,
}

/// A single typed intent submitted to the Gateway (§6.2 ActionRequest; binding = Appendix A +
/// DATA_MODEL §2.9). `inputs` is the open per-action-type payload (`params`; the §6.3 catalog
/// pins per-type schemas in 2.2 — `action_type` is a `String` here). `status` binds the frozen
/// §5.1 [`ActionRequestStatus`] (15). NO daemon behavior here — the pipeline that constructs +
/// persists these is 2.1b.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionRequest {
    pub action_request_id: ActionRequestId,
    pub project_id: Option<ProjectId>,
    pub action_type: String,
    pub requester_type: RequesterType,
    pub requester_id: String,
    pub resource_refs: Vec<ResourceRef>,
    /// the open per-action-type input payload (`params`); the §6.3 catalog pins per-type
    /// schemas in 2.2 (free JSON object here, reject-unknown does not apply to its contents)
    pub inputs: serde_json::Value,
    pub risk_level: RiskLevel,
    pub idempotency_key: Option<String>,
    /// the lease fencing token this execution must present (§17 / §2.6 — bound at execute in 2.4)
    pub fencing_token: Option<i64>,
    pub status: ActionRequestStatus,
    pub preview: Option<ActionPreview>,
    pub created_at: Timestamp,
}

/// A step-ordering edge within an [`ActionPlan`] (§6.2 ActionDependency). AG §9.2 references it
/// but never defines its shape — frozen as the minimal step-ordering edge (chosen shape recorded
/// for AG/Appendix-A adoption at Step-9).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionDependency {
    pub step_id: String,
    pub depends_on_step_ids: Vec<String>,
}

/// One step of a bundled [`ActionPlan`] (§6.2 ActionPlanStep; AG §9.3). Wraps a full
/// [`ActionRequest`]; `status` is the step's own §5.1 [`ActionRequestStatus`] within the plan.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionPlanStep {
    pub step_id: String,
    pub label: String,
    pub action_request: ActionRequest,
    pub required: bool,
    pub can_skip: bool,
    pub rollback_action_type: Option<String>,
    pub status: ActionRequestStatus,
}

/// A bundled multi-step plan (§6.2 ActionPlan; O-3). `rollback_plan` (whose RollbackPlan shape
/// AG never pins) is DEFERRED → 2.4; `summary`/`created_at`/`evidence_refs` are additive-later
/// (the 2.1b `action_plans` table can carry `created_at` as a column without the model).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionPlan {
    pub plan_id: ActionPlanId,
    pub title: String,
    pub steps: Vec<ActionPlanStep>,
    pub dependencies: Vec<ActionDependency>,
    pub overall_risk: RiskLevel,
    pub approval_mode: ApprovalMode,
}

/// A human/policy decision over an action or plan (§6.2/§5.1 Approval; binding = Appendix A).
/// `status` binds the frozen §5.1 [`ApprovalStatus`] (10). `constraints` (unpinned
/// ApprovalConstraint shape) is DEFERRED → 2.2.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct Approval {
    pub approval_id: ApprovalId,
    pub action_request_id: Option<ActionRequestId>,
    pub plan_id: Option<ActionPlanId>,
    pub required_approver: RequiredApprover,
    pub status: ApprovalStatus,
    pub risk_level: RiskLevel,
    pub scope: ApprovalScope,
    pub decided_by: Option<String>,
    pub decided_at: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
}

/// The outcome of an executed action (§6.2 ActionResult; binding = Appendix A). `emitted_events`
/// are typed [`EventId`]s (uniform with the ID discipline). `error` is a message (the structured
/// ActionError taxonomy is DEFERRED → 2.4).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionResult {
    pub action_request_id: ActionRequestId,
    pub status: ActionResultStatus,
    pub created_resources: Vec<ResourceRef>,
    pub changed_resources: Vec<ResourceRef>,
    pub emitted_events: Vec<EventId>,
    pub error: Option<String>,
    pub rollback_available: bool,
}

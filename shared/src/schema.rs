//! JSON-Schema emission (ARCHITECTURE §5.0, Option A).
//!
//! The Rust authority's enums are bundled into one schema (value sets land in
//! `$defs`); the checked-in artifact in `contracts/schema/` is regenerated from
//! here and CI-diff-gated (test 9). The Python Brain (Pydantic) + TS UI (Zod)
//! generate their consumers from this neutral artifact. Output is deterministic
//! so the diff gate is stable.

use schemars::JsonSchema;

use crate::actor::ActorType;
use crate::event_envelope::{EventEnvelope, Sensitivity, SourceType, Visibility};
use crate::ids::IdKind;
use crate::objects::DesktopObjectKind;
use crate::status::{
    ActionRequest, AgentTeam, Approval, ProjectBrain, PullRequest, Session, Task, WorkflowInstance,
    WorktreeGit, WorktreeOverlay,
};
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

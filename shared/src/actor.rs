//! Audit actor enum (ARCHITECTURE §7.1 / R-2 — 10 values).
//!
//! The canonical `Event.actor_type`. EM §7's legacy `remote_device` is unified to
//! `remote_client` here (R-2; DATA_MODEL §6 line 576). Request-time
//! `ActionRequest.requester_type` aliases (`agent_session→session_adapter`,
//! `workflow_pack→workflow_runtime`, `system_policy→automation_policy`) are a
//! separate binding mapping (Appendix A) — not part of this audit enum.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The 10 canonical audit actors (R-2). Serialized as snake_case `TEXT`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    User,
    ProjectBrain,
    ActionGateway,
    WorkflowRuntime,
    LocalRunner,
    SessionAdapter,
    IntegrationSyncer,
    System,
    RemoteClient,
    AutomationPolicy,
}

impl ActorType {
    /// Every actor, declaration order.
    pub const ALL: &'static [Self] = &[
        Self::User,
        Self::ProjectBrain,
        Self::ActionGateway,
        Self::WorkflowRuntime,
        Self::LocalRunner,
        Self::SessionAdapter,
        Self::IntegrationSyncer,
        Self::System,
        Self::RemoteClient,
        Self::AutomationPolicy,
    ];
}

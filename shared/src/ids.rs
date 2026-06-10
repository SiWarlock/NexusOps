//! Shared IDs + prefixed-ULID format (ARCHITECTURE §5.2 — 22 shared IDs).
//!
//! Platform-minted IDs are `<prefix><ULID>` (Crockford base32, lexicographically
//! sortable → `seq`-aligned ordering; self-describing in logs/audit/Brain
//! citations). External IDs (branch/commit/anchor/Linear/GitHub/PR) keep their
//! native provider values. The prefix→kind map is total + unique (frozen here —
//! §5.2's "single `id_kind` const").

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The 22 shared ID kinds (§5.2). Serialized snake_case (== the ID field name).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IdKind {
    WorkspaceId,
    ProjectId,
    RepoId,
    WorktreeId,
    BranchName,
    CommitSha,
    SessionId,
    AgentTeamId,
    ExecutionProfileId,
    WorkflowPackId,
    WorkflowInstanceId,
    WorkflowCommandId,
    ImplementationPlanId,
    PlanTaskId,
    ArchitectureAnchor,
    LinearIssueId,
    GithubIssueNumber,
    PrNumber,
    ActionRequestId,
    EventId,
    ArtifactId,
    EvidenceItemId,
}

impl IdKind {
    /// All 22 kinds, declaration order.
    pub const ALL: &'static [Self] = &[
        Self::WorkspaceId,
        Self::ProjectId,
        Self::RepoId,
        Self::WorktreeId,
        Self::BranchName,
        Self::CommitSha,
        Self::SessionId,
        Self::AgentTeamId,
        Self::ExecutionProfileId,
        Self::WorkflowPackId,
        Self::WorkflowInstanceId,
        Self::WorkflowCommandId,
        Self::ImplementationPlanId,
        Self::PlanTaskId,
        Self::ArchitectureAnchor,
        Self::LinearIssueId,
        Self::GithubIssueNumber,
        Self::PrNumber,
        Self::ActionRequestId,
        Self::EventId,
        Self::ArtifactId,
        Self::EvidenceItemId,
    ];

    /// The canonical ULID prefix for a platform-minted kind; `None` for the 6
    /// external (native-valued) kinds. Frozen in 0.5 (de-collided at Step-2.5).
    pub fn prefix(self) -> Option<&'static str> {
        Some(match self {
            Self::WorkspaceId => "ws_",
            Self::ProjectId => "proj_",
            Self::RepoId => "repo_",
            Self::WorktreeId => "wt_",
            Self::SessionId => "sess_",
            Self::AgentTeamId => "team_",
            Self::ExecutionProfileId => "prof_",
            Self::WorkflowPackId => "pack_",
            Self::WorkflowInstanceId => "wfi_",
            Self::WorkflowCommandId => "cmd_",
            Self::ImplementationPlanId => "plan_",
            Self::PlanTaskId => "task_",
            Self::ActionRequestId => "act_",
            Self::EventId => "evt_",
            Self::ArtifactId => "artf_",
            Self::EvidenceItemId => "evid_",
            // external — native values, no prefix (§5.2)
            Self::BranchName
            | Self::CommitSha
            | Self::ArchitectureAnchor
            | Self::LinearIssueId
            | Self::GithubIssueNumber
            | Self::PrNumber => return None,
        })
    }

    /// Reverse of [`IdKind::prefix`]: the kind owning `prefix`, if any.
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|k| k.prefix() == Some(prefix))
    }
}

/// Parse failure for a platform-minted ID (fail-closed, §15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// the string did not start with the kind's required prefix
    WrongPrefix,
    /// the post-prefix body was not a valid ULID
    BadUlid,
}

impl std::fmt::Display for IdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdError::WrongPrefix => f.write_str("id has the wrong prefix"),
            IdError::BadUlid => f.write_str("id body is not a valid ULID"),
        }
    }
}

impl std::error::Error for IdError {}

/// A platform-minted, type-safe `<prefix><ULID>` newtype.
macro_rules! minted_id {
    ($(#[$meta:meta])* $name:ident, $kind:expr, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// the [`IdKind`] this newtype represents
            pub const KIND: IdKind = $kind;
            /// the ULID prefix this newtype carries
            pub const PREFIX: &'static str = $prefix;

            /// Mint a fresh `<prefix><ULID>`.
            pub fn new() -> Self {
                Self(format!("{}{}", $prefix, ulid::Ulid::new()))
            }

            /// The full string form.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Parse + validate: correct prefix and a well-formed ULID body.
            pub fn parse(s: &str) -> Result<Self, IdError> {
                let body = s.strip_prefix($prefix).ok_or(IdError::WrongPrefix)?;
                ulid::Ulid::from_string(body).map_err(|_| IdError::BadUlid)?;
                Ok(Self(s.to_string()))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// An external, native-valued ID newtype (never re-minted, §5.2).
macro_rules! external_id {
    ($(#[$meta:meta])* $name:ident, $inner:ty, $kind:expr) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(pub $inner);

        impl $name {
            /// the [`IdKind`] this newtype represents
            pub const KIND: IdKind = $kind;
        }
    };
}

// --- the 16 platform-minted newtypes (one per kind, §5.2) --------------------
minted_id!(WorkspaceId, IdKind::WorkspaceId, "ws_");
minted_id!(ProjectId, IdKind::ProjectId, "proj_");
minted_id!(RepoId, IdKind::RepoId, "repo_");
minted_id!(WorktreeId, IdKind::WorktreeId, "wt_");
minted_id!(SessionId, IdKind::SessionId, "sess_");
minted_id!(AgentTeamId, IdKind::AgentTeamId, "team_");
minted_id!(ExecutionProfileId, IdKind::ExecutionProfileId, "prof_");
minted_id!(WorkflowPackId, IdKind::WorkflowPackId, "pack_");
minted_id!(WorkflowInstanceId, IdKind::WorkflowInstanceId, "wfi_");
minted_id!(WorkflowCommandId, IdKind::WorkflowCommandId, "cmd_");
minted_id!(ImplementationPlanId, IdKind::ImplementationPlanId, "plan_");
minted_id!(PlanTaskId, IdKind::PlanTaskId, "task_");
minted_id!(ActionRequestId, IdKind::ActionRequestId, "act_");
minted_id!(EventId, IdKind::EventId, "evt_");
minted_id!(ArtifactId, IdKind::ArtifactId, "artf_");
minted_id!(EvidenceItemId, IdKind::EvidenceItemId, "evid_");

/// The reserved system-workspace sentinel (§16 / Option B) — a well-known `ws_` id for
/// workspace-less **System-actor** lifecycle events (e.g. bootstrap Device/LocalRunner
/// registration, which run before any project Workspace exists). The all-zero ULID body
/// keeps it a VALID `ws_` id, so the §15 fail-closed envelope parse still holds — this is a
/// reserved value, NOT a nullable-column schema change to the frozen envelope.
pub const SYSTEM_WORKSPACE_ID: &str = "ws_00000000000000000000000000";

impl WorkspaceId {
    /// The reserved [`SYSTEM_WORKSPACE_ID`] sentinel as a typed [`WorkspaceId`].
    pub fn system() -> Self {
        // infallible: SYSTEM_WORKSPACE_ID is a compile-time-fixed valid `ws_<ULID>` (pinned by
        // `test_system_workspace_sentinel`); a parse failure here would be a contract bug.
        Self::parse(SYSTEM_WORKSPACE_ID).expect("SYSTEM_WORKSPACE_ID is a valid ws_ ULID")
    }
}

// --- the 6 external (native-valued) newtypes (§5.2) --------------------------
external_id!(BranchName, String, IdKind::BranchName);
external_id!(CommitSha, String, IdKind::CommitSha);
external_id!(ArchitectureAnchor, String, IdKind::ArchitectureAnchor);
external_id!(LinearIssueId, String, IdKind::LinearIssueId);
external_id!(GithubIssueNumber, u64, IdKind::GithubIssueNumber);
external_id!(PrNumber, u64, IdKind::PrNumber);

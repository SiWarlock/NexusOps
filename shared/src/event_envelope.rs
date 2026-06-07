//! Event envelope contract (ARCHITECTURE §7.1) + the 3 new enums.
//!
//! `source_type` = EM §8 source model (`docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md`
//! §8) — frozen as the CLOSED MVP set of 15 (the EM doc's "Examples" wording is
//! resolved to closed at /arch-finalize; reject-unknown per §5.0/§15). `sensitivity`
//! = §7.1/§15 (EM §9); `visibility` = §7.1. Reuses the 0.5-frozen IdKind newtypes +
//! actor enum. `redaction_status` is added with the redaction sub-feature (a+).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::actor::ActorType;
use crate::ids::{ActionRequestId, AgentTeamId, EventId, ProjectId, SessionId, WorkspaceId};

/// A frozen, snake_case-wire, reject-unknown value enum + its `ALL`.
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
    /// EM §8 source model — the subsystem that emitted the event (CLOSED MVP set, 15).
    SourceType {
        DesktopUi, LocalDaemon, ActionGateway, TerminalSupervisor, ClaudeCodeAdapter,
        CodexAdapter, GitExecutor, GithubSyncer, LinearSyncer, WorkflowPackRuntime,
        ProjectBrainIndexer, ProjectBrainRetriever, ProjectBrainActionPlanner,
        UsageMeter, RemoteRelay,
    }
}

wire_enum! {
    /// §7.1/§15 sensitivity classification (EM §9). Terminal output defaults `restricted`.
    Sensitivity { Public, Internal, Confidential, Secret, Restricted }
}

wire_enum! {
    /// §7.1 visibility scope.
    Visibility { User, Project, Workspace, System }
}

/// A normalized event→object edge (`object_refs[]`, §7.1 / DATA_MODEL §2.2).
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ObjectRef {
    pub object_type: String,
    pub object_id: String,
}

/// The §7.1 Event envelope. `seq` is the canonical total order (not `occurred_at`;
/// clocks skew — both kept). Required fields are non-`Option`; the rest are optional
/// per §7.1. `payload_hash`/`previous_event_hash` are reserved (R-10). The
/// `redaction_status` field + `RedactionStatus` enum land with the redaction
/// sub-feature commit (alongside the gate that enforces them).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct EventEnvelope {
    // --- required (§7.1) ---
    pub event_id: EventId,
    pub seq: i64,
    pub event_type: String,
    pub event_version: u32,
    pub occurred_at: String,
    pub recorded_at: String,
    pub workspace_id: WorkspaceId,
    pub actor_type: ActorType,
    pub actor_id: String,
    pub source_type: SourceType,
    pub source_id: String,
    pub correlation_id: String,
    pub sensitivity: Sensitivity,
    pub payload_json: String,
    pub schema_version: String,
    // --- optional (§7.1) ---
    pub project_id: Option<ProjectId>,
    pub session_id: Option<SessionId>,
    pub agent_team_id: Option<AgentTeamId>,
    /// the immediate prior event_id (typed as the frozen newtype, per §2.1)
    pub causation_id: Option<EventId>,
    pub action_request_id: Option<ActionRequestId>,
    pub approval_id: Option<String>,
    pub workflow_run_id: Option<String>,
    /// optional (§7.1) — omitted when empty, so the schema does not mark it required
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub object_refs: Vec<ObjectRef>,
    pub idempotency_key: Option<String>,
    pub visibility: Option<Visibility>,
    pub payload_hash: Option<String>,        // reserved (R-10)
    pub previous_event_hash: Option<String>, // reserved (R-10)
}

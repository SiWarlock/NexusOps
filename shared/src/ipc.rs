//! IPC `GatewayPort` wire contract (ARCHITECTURE §6.1/§6.4 [LOCKED — ADR-004], §5.0).
//!
//! The handshake + method/error surface the daemon serves over UDS and the ui/Brain generate
//! validators for (Zod/Pydantic). Authored here in the Rust authority so both sides read ONE
//! contract (matching the ui's pinned `ui/src/gateway-client/types.ts` + `connection/version.ts`).
//!
//! **Two version axes (don't conflate):** [`PROTOCOL_VERSION`] is the §6.4 *wire-handshake* skew
//! check (pinned `1`, the ui's `SUPPORTED_PROTOCOL_RANGE {1,1}`); `CONTRACT_VERSION` is the §5.0
//! *schema/codegen* version (bumped when this surface changes). They move independently.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The wire protocol version this binary speaks (§6.4). The daemon is the authoritative source;
/// the ui's `SUPPORTED_PROTOCOL_RANGE` must agree. SEPARATE from `CONTRACT_VERSION` (§5.0).
pub const PROTOCOL_VERSION: u32 = 1;
/// Inclusive lower bound of the protocol versions this daemon accepts (§6.4 / §16 compat).
pub const SUPPORTED_PROTOCOL_MIN: u32 = 1;
/// Inclusive upper bound of the protocol versions this daemon accepts (§6.4 / §16 compat).
pub const SUPPORTED_PROTOCOL_MAX: u32 = 1;

/// `true` iff `v` is within `[SUPPORTED_PROTOCOL_MIN, SUPPORTED_PROTOCOL_MAX]` — the daemon-side
/// twin of the ui's `checkVersionCompat` (one constant, both sides).
pub fn protocol_in_range(v: u32) -> bool {
    (SUPPORTED_PROTOCOL_MIN..=SUPPORTED_PROTOCOL_MAX).contains(&v)
}

/// The first client→daemon frame (§6.4 handshake). No method is served until a successful
/// handshake. `deny_unknown_fields` rejects a non-Hello first frame (reject-unknown, §15).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HelloFrame {
    pub protocol_version: u32,
    pub client_kind: String,
    pub app_version: String,
}

/// daemon→client handshake success (§6.4): the negotiated protocol version + the daemon's
/// build version + its capabilities.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HelloAck {
    pub protocol_version: u32,
    pub daemon_version: String,
    pub capabilities: Capabilities,
}

/// daemon→client handshake failure on version skew (§6.4): the daemon's supported range + the
/// client's offered version, so the ui can render "update required" (its `update_required` path).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionSkewError {
    pub supported_min: u32,
    pub supported_max: u32,
    pub client_protocol_version: u32,
}

/// `get_capabilities` / `HelloAck` capabilities (§6.4). Matches the ui's pinned `Capabilities`
/// (`protocol_version` + `contract_version`).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub protocol_version: u32,
    pub contract_version: String,
}

/// The §6.4 structured wire error codes (closed set; reject-unknown). snake_case wire values.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    VersionSkew,
    FrameTooLarge,
    UnknownMethod,
    UnauthorizedPeer,
    PolicyDenied,
    PreconditionStale,
}

impl IpcErrorCode {
    /// every code, declaration order (the closed §6.4 set).
    pub const ALL: &'static [Self] = &[
        Self::VersionSkew,
        Self::FrameTooLarge,
        Self::UnknownMethod,
        Self::UnauthorizedPeer,
        Self::PolicyDenied,
        Self::PreconditionStale,
    ];
}

/// A structured daemon→client error frame (§6.4). Carries one of the closed [`IpcErrorCode`]s.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WireError {
    pub code: IpcErrorCode,
}

/// The canonical closed projection-name enum (§6.1) — both daemon + ui reject-unknown. **Wire
/// values are PascalCase** (the variant names verbatim — NO `rename_all`), matching the ui's
/// pinned `get_projection("Session"|…)` literals + the §7 registry / R-5/R-6 architecture labels.
/// `UsageLedger` is the architecture-canonical name (the ui's provisional `Usage` reconciles to it).
// INTENTIONALLY no `#[serde(rename_all)]` — PascalCase wire values (§6.1); do not "fix" to
// snake_case (it would break the ui's pinned `get_projection("Session")` literals).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
pub enum ProjectionName {
    ProjectActivity,
    Session,
    ApprovalQueue,
    Worktree,
    PullRequest,
    PlanProgress,
    ProjectGraph,
    AgentTeam,
    AuditTrail,
    UsageLedger,
}

impl ProjectionName {
    /// every projection name, declaration order (the §6.1 closed set).
    pub const ALL: &'static [Self] = &[
        Self::ProjectActivity,
        Self::Session,
        Self::ApprovalQueue,
        Self::Worktree,
        Self::PullRequest,
        Self::PlanProgress,
        Self::ProjectGraph,
        Self::AgentTeam,
        Self::AuditTrail,
        Self::UsageLedger,
    ];
}

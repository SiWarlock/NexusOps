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
/// `protocol_error` (a bad-first-frame / handshake-required / malformed-frame violation, distinct
/// from `unknown_method`) was added in 1.5 per the lead-ratified §6.4 gap resolution (Option B).
/// `fencing_conflict` + `internal_error` were added in 2.4 L1 (lead-ruled Option A, away-authority):
/// the §17 failure modes need honest codes that drive the THREE distinct §11.5 safety cards with
/// DIFFERENT resolution semantics — `fencing_conflict` → the **never-auto-resolved** hard-conflict
/// card (rule #6); `precondition_stale` → the **re-approvable** stale card; `internal_error` → the
/// **fail-closed** integrity alert (`AuditWriteFailed`/`UnknownOutcome`). Collapsing them to
/// `precondition_stale` would render a fencing conflict as re-approvable → break rule #6 at the UI.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    VersionSkew,
    FrameTooLarge,
    UnknownMethod,
    UnauthorizedPeer,
    PolicyDenied,
    PreconditionStale,
    FencingConflict,
    ProtocolError,
    InternalError,
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
        Self::FencingConflict,
        Self::ProtocolError,
        Self::InternalError,
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

/// A JSON-RPC-style method request (§6.1). `id` correlates the response; `params` is the
/// method-specific argument object (the daemon validates it per-method).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RpcRequest {
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub id: u64,
}

/// A JSON-RPC-style response (§6.1), correlated by `id`: exactly one of `result` / `error`.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RpcResponse {
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<WireError>,
}

/// `submit_action` result (§6.1) — the ack the ui/Brain intent seam receives: the minted
/// `action_request_id` (so the client can `preview_action`/track it) + the action's current
/// `status` (§5.1 ActionRequest(15); in 2.1b the stub policy lands it at `awaiting_approval`).
/// 2.1b L2: defined + returned over the wire; it joins the published schema + the 3-way verify
/// with the rest of the §6.1 mutation surface at L3's CONTRACT_VERSION 0.16.0 bump (`PlanAck` →
/// 2.1c with `submit_action_plan`).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionAck {
    pub action_request_id: String,
    pub status: crate::status::ActionRequestStatus,
}

/// `submit_action_plan` result (§6.1, O-3) — the bundled-plan ack: the minted `plan_id` + a
/// per-step ack (mirrors [`ActionAck`]) so the client can `preview_action`/track each step + hold
/// the plan handle. Richer fields (`approval_ids`/`overall_status`) are additive-later (2.1c L2,
/// Q1 default). Reject-unknown; optionals (none today) would serialize as `null`.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct PlanAck {
    pub plan_id: String,
    pub steps: Vec<PlanStepAck>,
}

/// One step's ack inside a [`PlanAck`] (§6.1, O-3): the step's plan-local `step_id` + the minted
/// `action_request_id` (so the client can `preview_action`/track it) + its current `status`.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct PlanStepAck {
    pub step_id: String,
    pub action_request_id: String,
    pub status: crate::status::ActionRequestStatus,
}

/// `get_projection` params (§6.1). `page` (pagination) is provisional and omitted for MVP.
///
/// **MVP NOTE:** `scope` is **accepted but NOT YET enforced** — the daemon returns the full
/// projection table regardless of `scope.project_id` (the local same-uid peer has no tenancy
/// boundary in MVP). Do not build client filtering that assumes the daemon honors `scope`;
/// scope filtering (and the `ProjectId` newtype on the field) land when multi-project scoping does.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetProjectionParams {
    pub name: ProjectionName,
    /// accepted but not yet enforced (see the struct note) — the read is unscoped in MVP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ProjectionScope>,
}

/// `get_projection` scope filter (§6.1; provisional — widens as scoping lands).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// `subscribe` params (§6.1). `filter` is provisional (a per-projection scope; widens later).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubscribeParams {
    pub projection: ProjectionName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
}

/// The kind of projection change in a [`ProjectionDelta`] (§6.1). snake_case wire values.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeltaKind {
    Upsert,
    Remove,
}

/// A streamed projection delta (§6.1 subscribe) — matches the ui's `ProjectionDelta`: the changed
/// projection, the change kind, and the row (on upsert) or just the id (on remove).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionDelta {
    /// the changed projection (closed enum — reject-unknown both ways; serializes to the same
    /// PascalCase string the ui's `ProjectionDelta` parses, but the daemon can't push a typo).
    pub projection: ProjectionName,
    pub kind: DeltaKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

// ---- the §6.4 Terminal Channel frames (3.4 — the reserved slot filled, JSON-base64 MVP) --------
//
// Raw PTY bytes ride the EXISTING 4-byte-len+JSON codec by carrying `data` as base64 (LESSON §7 —
// no new framing machinery; the binary fast-path is a deferred 3.5 decision, made with throughput
// data). `terminal_id` is an opaque daemon-minted runtime handle (a `String` on the wire — NOT one
// of the frozen-22 IDs; re-minted per attach/resume, the `subscription_id` precedent — lead-ratified
// 3.4). The PTY is **display-only** (safety #9): these frames move bytes + an OS-derived exit signal,
// never a status the UI infers from output.

/// The kind of explicit app-level **flow-control** frame in a [`TerminalControlFrame`] (§6.4
/// backpressure). snake_case wire values. The client emits `pause`/`resume` to throttle a chatty
/// PTY; the L3 watermark pump also produces these as its flow-control decision (display-only #9).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerminalControlKind {
    Pause,
    Resume,
}

impl TerminalControlKind {
    /// every variant — the contract value-set guard (`check_values`).
    pub const ALL: &'static [TerminalControlKind] =
        &[TerminalControlKind::Pause, TerminalControlKind::Resume];
}

/// A daemon→client terminal **output** frame (§6.4). `data` = base64 raw PTY output bytes. `seq`
/// increments per emitted frame (the L3 ~30 fps batch pump assigns it) so a gap is client-detectable.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerminalOutputFrame {
    /// the opaque daemon-minted terminal runtime handle (NOT a frozen-22 ID; re-minted on resume).
    pub terminal_id: String,
    pub seq: u64,
    /// base64-encoded raw PTY output bytes.
    pub data: String,
}

/// A client→daemon terminal **input** frame (§6.4): keystrokes/paste for the PTY child. `data` base64.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerminalInputFrame {
    pub terminal_id: String,
    /// base64-encoded raw input bytes written to the PTY child.
    pub data: String,
}

/// A terminal **control** frame (§6.4): the explicit app-level flow-control signal (pause/resume).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerminalControlFrame {
    pub terminal_id: String,
    pub kind: TerminalControlKind,
}

/// The server→client **multiplexed frame envelope** (§6.4 frame-type tag). The internally-tagged
/// `frame_type` discriminant lets the client demultiplex one connection: an RPC response, a
/// subscription push, or raw terminal output. **AS-BUILT (3.4):** the reserved Terminal-Channel slot
/// is filled with the **JSON-base64 MVP** `TerminalOutput` variant (`data` base64 over the unchanged
/// codec, LESSON §7); the **binary fast-path stays a deferred 3.5 decision**, made with the
/// terminal-attach throughput benchmark (additive — a future variant + a CONTRACT bump, not a reshape).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "frame_type", rename_all = "snake_case")]
pub enum ServerFrame {
    RpcResponse(RpcResponse),
    SubscriptionPush(ProjectionDelta),
    /// raw terminal output (§6.4 Terminal Channel) — the reserved slot, filled 3.4 (JSON-base64 MVP).
    TerminalOutput(TerminalOutputFrame),
}

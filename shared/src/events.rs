//! Concrete event-type payloads (ARCHITECTURE §7.1 EventTypeRegistry, §5.0).
//!
//! The registry accretes per phase — it is NOT defined all-at-once. 1.2 folds only
//! [`SessionStarted`] (the demo-step-7 fan-out); Phase 2/3 add their payloads
//! additively (a minor `CONTRACT_VERSION` bump, drift-caught by the schema gate).
//! Payloads live in `shared/` (not `daemon/`) because event shapes are consumer
//! surface: the golden-log tests, the UI session view, and the Brain indexer read
//! them (§5.0 — Rust authority → schema → Zod/Pydantic).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::harness::{ResumeMode, TelemetrySample};
use crate::ids::{ExecutionProfileId, WorkspaceId, WorktreeId};
use crate::objects::{DeviceId, LocalRunnerId};
use crate::status::{ExecutionProfile, PullRequest, ReviewState, Session};
use crate::time::Timestamp;

/// `SessionStarted` payload. The session's **identity** (`session_id`/`project_id`)
/// lives on the [`crate::event_envelope::EventEnvelope`] typed fields (so projectors
/// derive `object_refs` + read-models from columns that survive a rebuild); this
/// payload carries the type-specific session attributes the `proj_session` projector
/// folds. `status` binds to the frozen §5.1 [`Session`] machine — an unknown wire
/// value fails closed at the parse boundary (reject-unknown, §15).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct SessionStarted {
    pub status: Session,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// the §15 #8 ExecutionProfile binding — the profile RESOLVED at session.create + recorded here
    /// (P4.0b-1). `None` until the session-create executor populates it (4.0b-1 L3); a profile CHANGE
    /// is approval-gated (the no-silent-account-hop gate lives on the change, not this routine record).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_profile_id: Option<ExecutionProfileId>,
}

/// `DeviceRegistered` payload (§5.3 Device / §16 bootstrap). The device's `dev_` identity is
/// NOT an envelope typed column, so it lives in the payload — the `object_refs` projector
/// sources the edge from here (the payload is the identity home; rebuild-safe). Register-if-
/// absent: the stable desktop host registers once and is reused across daemon restarts.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct DeviceRegistered {
    pub device_id: DeviceId,
}

/// `LocalRunnerRegistered` payload (§5.3 LocalRunner / §16 bootstrap). Minted fresh per
/// daemon start; the `lr_` identity lives in the payload (not an envelope column) so the
/// `object_refs` projector sources the edge from it (rebuild-safe).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct LocalRunnerRegistered {
    pub local_runner_id: LocalRunnerId,
}

/// `ExecutionProfileRegistered` payload (§15 #8 / §5.3 / DATA_MODEL §2.8 / P5.3a). The System-actor
/// registration event for the FIRST durable OBJECT registry (`execution_profiles`, Option B — the
/// canonical ROW is the source of truth; this event is the audit TRAIL, NOT a projection source). Carries
/// the durable identity the row stores. **§15 rule #4 — load-bearing:** `keychain_ref` is a nullable
/// NON-SECRET POINTER into the OS keychain, NEVER the token (the secret WRITE + startup self-test are
/// 5.3b). `status` binds to the frozen §5.1 [`ExecutionProfile`] machine (reject-unknown). `provider`/
/// `harness` are open wire `String`s (the DATA_MODEL §2.8 TEXT columns; a closed enum is a 5.3b/future
/// tightening when the set gates adapter dispatch — the `terminal_id` bare-String precedent).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ExecutionProfileRegistered {
    pub execution_profile_id: ExecutionProfileId,
    pub workspace_id: WorkspaceId,
    pub provider: String,
    pub harness: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_alias: Option<String>,
    /// **§15 rule #4:** a NON-SECRET keychain POINTER (the keychain entry name/ref), NEVER the token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keychain_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_policy_json: Option<String>,
    pub status: ExecutionProfile,
    pub created_at: Timestamp,
}

impl ExecutionProfileRegistered {
    /// The EventTypeRegistry name — ONE home (the P5.3a profiles register-mutator + the seed).
    pub const EVENT_TYPE: &'static str = "ExecutionProfileRegistered";
}

/// `AuditIntegrityViolation` payload (§17 Option C). Emitted when startup replay QUARANTINES a
/// corrupt / unredacted event row — the LOUD, consumer-visible record that the audit history has
/// a gap (the core of why Option C beats a silent skip). Carries the affected `seq` + a
/// redaction-safe **structural** `reason` — NEVER the corrupt payload or any row content (§15).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct AuditIntegrityViolation {
    pub seq: i64,
    pub reason: String,
}

impl AuditIntegrityViolation {
    /// The EventTypeRegistry name — ONE home, referenced by the daemon emit path + the audit
    /// projector (so the NEW type adds no bare string-literal to the dup set; the existing
    /// SessionStarted/Device/LocalRunner literals consolidate in a later non-safety slice).
    pub const EVENT_TYPE: &'static str = "AuditIntegrityViolation";
}

/// `SensitiveOutputRedacted` payload (§15 redaction-before-persist). Emitted when the Redactor
/// detects a high-confidence secret it CANNOT safely redact in place — the original event is
/// **diverted (NOT persisted)** and this record is appended in its place (`event_envelope.rs`
/// RedactionStatus doc). Carries forensic metadata ONLY: the diverted event's `event_type`, a
/// redaction-safe **structural** `reason`, and the `detector` that fired — NEVER the secret or
/// any byte of the diverted payload (§15; mirrors the 1.6c AuditIntegrityViolation content-free
/// record). In MVP the prefix+entropy Redactor masks in place and never triggers this; it is
/// the §15 "can't safely redact → divert" safety net for an alternate/future Redactor.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct SensitiveOutputRedacted {
    pub original_event_type: String,
    pub reason: String,
    pub detector: String,
}

impl SensitiveOutputRedacted {
    /// The EventTypeRegistry name — ONE home, referenced by the daemon divert path + the audit
    /// projector (the new type adds no bare string-literal to the dup set).
    pub const EVENT_TYPE: &'static str = "SensitiveOutputRedacted";
}

/// `ActionRequested` payload (§6.2/§7.1; AG §17.1) — the Action Gateway's FIRST event for a
/// submitted action (2.1b). The action's IDENTITY (`action_request_id`, `correlation_id`,
/// `project_id`, `actor`) lives on the [`crate::event_envelope::EventEnvelope`] typed columns — the
/// Gateway populates them. This payload carries only the request-classification DELTA: the
/// `action_type` (§6.3 catalog, String until 2.2), the requester-supplied `risk_level` (RECORDED
/// for audit but NOT trusted for the decision in 2.1b — the policy stub is risk-blind, require-
/// approval-for-all; 2.2 makes risk catalog-authoritative), and the `requester_type` (R-2; the
/// envelope `actor_type` is its mapped audit actor).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionRequested {
    pub action_type: String,
    pub risk_level: crate::actions::RiskLevel,
    pub requester_type: crate::actions::RequesterType,
}

impl ActionRequested {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit/queue projectors).
    pub const EVENT_TYPE: &'static str = "ActionRequested";
}

/// `ActionApprovalRequested` payload (§6.2/§7.1; AG §17.1) — emitted when the policy decision routes
/// an action to human approval (2.1b). The action + approval IDENTITY are on the envelope columns
/// (`action_request_id`/`approval_id`); the payload echoes `approval_id` so a payload-only consumer
/// (the Brain indexer) gets the linkage without joining the envelope.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionApprovalRequested {
    pub approval_id: String,
}

impl ActionApprovalRequested {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the approval-queue projector).
    pub const EVENT_TYPE: &'static str = "ActionApprovalRequested";
}

/// `ActionApproved` payload (§6.2/§7.1; AG §17.1) — a human/policy approved the action (2.1b L3).
/// Identity (`action_request_id`/`approval_id`) on the envelope; the payload echoes `approval_id` +
/// the optional `decided_by` (the approver, when known).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionApproved {
    pub approval_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
}

impl ActionApproved {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit/queue projectors).
    pub const EVENT_TYPE: &'static str = "ActionApproved";
}

/// `ActionDenied` payload (§6.2/§7.1; AG §17.1) — the action was denied (terminal). Carries the
/// denial `reason` (the audit record of WHY) + an **OPTIONAL** `approval_id`. A HUMAN-deny carries
/// `Some(appr_…)`; a 043/A1 agent deny-rule **policy-deny** (denied at submit, BEFORE any approval
/// object) carries `None` (the record-then-deny forensic event — there is no approval to reference).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionDenied {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    pub reason: String,
}

impl ActionDenied {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit projector).
    pub const EVENT_TYPE: &'static str = "ActionDenied";
}

/// `ActionExpired` payload (§6.2/§7.1; AG §17.1) — the approval lapsed past `expires_at` before a
/// decision (§17); the action is terminal, never executed. Carries the `approval_id`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionExpired {
    pub approval_id: String,
}

impl ActionExpired {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit projector).
    pub const EVENT_TYPE: &'static str = "ActionExpired";
}

/// `ActionStarted` payload (§6.2/§7.1; AG §17.1) — the executor began running the approved action
/// (queued→executing). Identity on the envelope; no delta beyond the event itself.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionStarted {}

impl ActionStarted {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit projector).
    pub const EVENT_TYPE: &'static str = "ActionStarted";
}

/// `ActionSucceeded` payload (§6.2/§7.1; AG §17.1) — the executor completed the action successfully
/// (executing→succeeded). Identity on the envelope; the created/changed resources land in the
/// `ActionResult` surface (2.3). No payload delta in 2.1b.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionSucceeded {}

impl ActionSucceeded {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit projector).
    pub const EVENT_TYPE: &'static str = "ActionSucceeded";
}

/// `ActionFailed` payload (§6.2/§7.1; AG §17.1) — the executor failed (executing→failed). Carries
/// the structured [`crate::actions::ActionError`] taxonomy (2.4 — replaced the 2.1b free-string `error`): the §17
/// failure modes (`audit_write_failed`/`stale_precondition`/`fencing_conflict`/`unknown_outcome`)
/// + `executor_error{message}` (the home the prior free string maps into).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionFailed {
    pub error: crate::actions::ActionError,
}

impl ActionFailed {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit projector).
    pub const EVENT_TYPE: &'static str = "ActionFailed";
}

/// `ActionPartiallySucceeded` payload (§7.1; AG §17) — the §17 "side effect APPLIED but its terminal
/// event could NOT be written" record (executing→partially_succeeded). Emitted best-effort when a
/// real executor reported a side effect but the `ActionSucceeded` write failed (the fail-closed
/// audit-write path, L2). Carries a redaction-safe **structural** `reason` ONLY — never row/payload
/// content (§15; mirrors [`AuditIntegrityViolation`]). A loud, consumer-visible audit-integrity record.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ActionPartiallySucceeded {
    pub reason: String,
}

impl ActionPartiallySucceeded {
    /// The EventTypeRegistry name — ONE home (the Gateway emit path + the audit projector).
    pub const EVENT_TYPE: &'static str = "ActionPartiallySucceeded";
}

/// `TelemetrySampled` payload (§9.1/§7.1; §18 usage rollups) — a per-heartbeat usage OBSERVATION
/// event the harness adapters (3.2/3.3) emit. **NOT a Gateway Action** (Q1, lead-confirmed): INV-SEC-1
/// (#1) governs FS/git/external/session-STATE mutations; a telemetry observation mutates none — so it
/// follows the System/adapter-actor non-mutation-event precedent (`DeviceRegistered`/
/// `AuditIntegrityViolation`): written via the single write-actor through the §15 redaction gate (#2/#3
/// hold), never through the Gateway pipeline. The rollup IDENTITY/time (`session_id`/`project_id`/
/// `occurred_at`) lives on the [`crate::event_envelope::EventEnvelope`] columns; this payload carries
/// only the dims the envelope LACKS — the [`TelemetrySample`] + the `model` and `execution_profile_id`
/// the `proj_usage_ledger` projector buckets by (3.1). Optionals serialize as explicit `null` (no
/// `skip_serializing_if`) so the §2.5-seam field-name snapshot is stable (LESSON §15 trap 3).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct TelemetrySampled {
    pub sample: TelemetrySample,
    pub model: Option<String>,
    pub execution_profile_id: Option<String>,
}

impl TelemetrySampled {
    /// The EventTypeRegistry name — ONE home (the 3.2/3.3 adapter emit path + the usage projector).
    pub const EVENT_TYPE: &'static str = "TelemetrySampled";
}

/// The §17 PTY-death record (3.4): a terminal child process exited. A **non-mutation OBSERVATION
/// event** — the daemon WITNESSES the child exit (via portable-pty waitpid / reader EOF) and records
/// it; it follows the System/supervisor-actor non-mutation-event precedent (`DeviceRegistered`/
/// `TelemetrySampled`, LESSON §10/§23): written via the single write-actor through the §15 redaction
/// gate (#2/#3 hold), NEVER through the Gateway pipeline (INV-SEC-1 governs state *mutations*; a
/// process-exit notification is none). **Safety #9** — `exit_code`/`signal` come from the OS exit
/// status, NEVER inferred by parsing terminal output (the PTY is display-only). The session identity
/// (`session_id`/`occurred_at`) lives on the [`crate::event_envelope::EventEnvelope`] columns; this
/// payload carries only `terminal_id` + the OS exit detail. The §17 SessionFailed → ActionFailed →
/// lease-release cascade is **Phase 4** (it CONSUMES this event). Optionals serialize as explicit
/// `null` (no `skip_serializing_if`) for a stable §2.5-seam field-name snapshot (LESSON §15 trap 3).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct TerminalProcessExited {
    /// the opaque daemon-minted terminal runtime handle (NOT a frozen-22 ID; matches the wire frames).
    pub terminal_id: String,
    /// the child's exit code (None when killed by a signal, or unavailable).
    pub exit_code: Option<i32>,
    /// the terminating signal name (None when it exited normally).
    pub signal: Option<String>,
}

impl TerminalProcessExited {
    /// The EventTypeRegistry name — ONE home (the 3.4 terminal-host emit path; §7.1 single-home).
    pub const EVENT_TYPE: &'static str = "TerminalProcessExited";
}

/// The §8.1 daemon-restart recovery record (4.1b-1): how a session that was live at shutdown was brought
/// back — the §11.4 "resumed-(live) vs replayed-(relaunched)" indicator + the §17 "restart session"
/// affordance tail (`mode == Relaunched`). A **non-mutation OBSERVATION event**: a recovery is the daemon
/// re-materializing its OWN already-approved prior session, so it follows the System-actor non-mutation-
/// event precedent (`DeviceRegistered`/`TelemetrySampled`, LESSON §10/§23) — written via the single
/// write-actor through the §15 redaction gate (#2/#3 hold), **NEVER through the Gateway** (Q1=(a),
/// lead/user-ruled: INV-SEC-1 governs proposer-intent state *mutations*; the daemon re-materializing its
/// own session is a lifecycle event, not a proposer intent). The §15 #8 `execution_profile_id` is
/// PRESERVED from the original `SessionStarted` (no silent account-hop). The session IDENTITY
/// (`session_id`/`occurred_at`) lives on the [`crate::event_envelope::EventEnvelope`] columns; this
/// payload carries only the recovery outcome. Optionals serialize as explicit `null` (no
/// `skip_serializing_if`) for a stable §2.5-seam field-name snapshot (LESSON §15 trap 3).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct SessionRecovered {
    /// how the session was recovered (the §8.1 B2-strict ladder outcome).
    pub mode: ResumeMode,
    /// the scrollback event count replayed (non-zero ONLY on `Replayed`, mirroring `ResumeResult`).
    pub replayed_event_count: u64,
    /// the §15 #8 ExecutionProfile PRESERVED from the original `SessionStarted` (no account-hop). `None`
    /// only for a profile-less prior session (pre-§15-#8).
    pub execution_profile_id: Option<ExecutionProfileId>,
}

impl SessionRecovered {
    /// The EventTypeRegistry name — ONE home (the 4.1b recovery-sink emit path; §7.1 single-home).
    pub const EVENT_TYPE: &'static str = "SessionRecovered";
}

/// The §17 supervised-child-death record (4.2): a session's agent/PTY/app-server child died (daemon
/// alive) — the supervisor reaped it as `Session::Failed`. A **non-mutation OBSERVATION event**: the
/// daemon WITNESSES the death and records it, so it follows the System-actor non-mutation-event
/// precedent (`SessionRecovered`/`TerminalProcessExited`, LESSON §10/§23) — written via the single
/// write-actor through the §15 redaction gate (#2/#3 hold), **NEVER through the Gateway** (a death
/// notification is not a state mutation; INV-SEC-1 governs mutations). **Empty payload** (the
/// `WorktreeMerged`/`ActionStarted {}` precedent): the death fact + the envelope session_id + the
/// folded `proj_session.status=Failed` fully drive the §11.4 "restart session" affordance; a structural
/// `reason` is an additive-later follow-on (at the reap everything is indistinguishably `Failed` — the
/// forensic "why" already lives in the correlated `TerminalProcessExited{exit_code,signal}`). The §17
/// cascade ARMS (fail-in-flight-action, release-lease, the Codex pipe-drop distinction) attach to this
/// head when their producers land. `deny_unknown_fields` reject-unknown (§5.0/§15 fail-closed).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct SessionFailed {}

impl SessionFailed {
    /// The EventTypeRegistry name — ONE home (the 4.2 death-driver emit path; §7.1 single-home).
    pub const EVENT_TYPE: &'static str = "SessionFailed";
}

// =================================================================================================
// P4.0b-R1b (CONTRACT 0.26.0) — the Phase-5/7 wiring EventTypeRegistry payloads (edges-R1 §2.5-seam).
// `shared/`-only: edges' (dormant) Project/Git/Github/Linear executors EMIT these via `EmittedEvent`
// when each namespace lands at P5/P7; edges' projectors CONSUME them. Identity (actor/project_id/
// resource_refs) on the envelope; payload = delta; `deny_unknown_fields`; optionals = explicit `null`
// (no `skip_serializing_if`) for a stable §2.5-seam field-name snapshot (LESSON §15 trap 3). No daemon
// emission here — these are the consumable + emittable shapes (the §5.0 mechanism + snapshots bind them).
// =================================================================================================

/// `ProjectRescanned` payload (§7.1; P5.1 — edges' project-detection wiring). Emitted by edges'
/// `project.rescan` executor after its detection engine runs; consumed by `proj_project_activity` + the
/// project graph + a private `projects`/`repositories` registry (the projector splits this ONE coarse
/// event into rows — lead-ruled coarse: a rescan is atomic). The project IDENTITY (`project_id`/actor)
/// lives on the [`crate::event_envelope::EventEnvelope`] columns; this payload carries the detection delta.
///
/// **§15 `remote_url` (rule #5/#3/#4) — load-bearing:** the git remote URL can embed credentials
/// (`https://user:token@host`). A generic URL password has NO key-prefix → it is OUTSIDE the §15
/// prefix+entropy Redactor's recall envelope (LESSON §13), so the Redactor is the **backstop**, not the
/// primary control. The `user:token@` userinfo MUST be stripped **at the emit source** (edges' project
/// executor, BEFORE this event is constructed). The strip-at-source enforcement + its test are edges'
/// P5.1 emission slice — R1b ships the contract, no emitter.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ProjectRescanned {
    pub is_git: bool,
    pub repo_root: Option<String>,
    /// the git remote URL. **§15:** userinfo (`user:token@`) MUST be stripped at the emit source (edges)
    /// — the Redactor is the backstop only (a generic URL password has no prefix; LESSON §13).
    pub remote_url: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub is_dirty: bool,
    pub workflow_pack: bool,
    pub cc_crew: bool,
    pub plan_file: Option<String>,
    pub brain: bool,
    pub scanned_at: Timestamp,
}

impl ProjectRescanned {
    /// The EventTypeRegistry name — ONE home (edges' project executor emit path + its projectors).
    pub const EVENT_TYPE: &'static str = "ProjectRescanned";
}

/// `WorktreeCreated` payload (§7.1; P5.2 — from edges' `git.create_worktree`). Feeds `proj_worktree`
/// (status via edges' `derive_worktree_status`). Identity on the envelope; payload = the creation delta.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct WorktreeCreated {
    pub worktree_id: WorktreeId,
    pub path: String,
    pub branch_name: String,
    pub base_branch: Option<String>,
}

impl WorktreeCreated {
    /// The EventTypeRegistry name — ONE home (edges' git executor emit path + `proj_worktree`).
    pub const EVENT_TYPE: &'static str = "WorktreeCreated";
}

/// `BranchCreated` payload (§7.1; P5.2 — from edges' `git.create_branch`).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct BranchCreated {
    pub branch_name: String,
    pub base: Option<String>,
}

impl BranchCreated {
    /// The EventTypeRegistry name — ONE home (edges' git executor emit path).
    pub const EVENT_TYPE: &'static str = "BranchCreated";
}

/// `WorktreeMerged` payload (§7.1; P5.2 overlay-axis transition). EMPTY payload — the worktree identity
/// is on the envelope `resource_refs`, the transition IS the `event_type` (the `ActionStarted{}`
/// precedent); feeds the overlay axis of edges' `derive_worktree_status`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct WorktreeMerged {}

impl WorktreeMerged {
    /// The EventTypeRegistry name — ONE home (edges' git executor emit path + the overlay axis).
    pub const EVENT_TYPE: &'static str = "WorktreeMerged";
}

/// `WorktreePrunable` payload (§7.1; P5.2 overlay-axis transition). EMPTY payload — see [`WorktreeMerged`].
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct WorktreePrunable {}

impl WorktreePrunable {
    /// The EventTypeRegistry name — ONE home (edges' git executor emit path + the overlay axis).
    pub const EVENT_TYPE: &'static str = "WorktreePrunable";
}

/// `WorktreeDeleted` payload (§7.1; P5.2 overlay-axis transition). EMPTY payload — see [`WorktreeMerged`].
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct WorktreeDeleted {}

impl WorktreeDeleted {
    /// The EventTypeRegistry name — ONE home (edges' git executor emit path + the overlay axis).
    pub const EVENT_TYPE: &'static str = "WorktreeDeleted";
}

/// `WorktreeLocked` payload (§7.1; P5.2 overlay-axis transition). EMPTY payload — see [`WorktreeMerged`].
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct WorktreeLocked {}

impl WorktreeLocked {
    /// The EventTypeRegistry name — ONE home (edges' git executor emit path + the overlay axis).
    pub const EVENT_TYPE: &'static str = "WorktreeLocked";
}

/// The integration provider (§7.2 — `github` | `linear`). A NEW closed wire enum: snake_case,
/// reject-unknown (§5.0/§15); a flat `enum` schema so the §5.0 3-way verify stays exact (LESSON §15
/// trap 2 — string enums emit as flat `enum`, not a discriminated union).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Github,
    Linear,
}

impl Provider {
    /// Every variant, declaration order.
    pub const ALL: &'static [Self] = &[Self::Github, Self::Linear];
}

/// `PullRequestSynced` payload (§7.1; P7.1 — from edges' github PR-sync executor). Feeds
/// `proj_pull_request` (the GitHub-authoritative cache, §7.2). `status` REUSES the frozen §5.1
/// [`PullRequest`] machine (no fork — adapters map INTO it). `pr_number` is the GitHub-native PR id
/// (external integer, not a frozen-22 ID). Identity on the envelope; payload = the synced-PR delta.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct PullRequestSynced {
    /// the GitHub-native PR number — a non-negative EXTERNAL natural (edges parses it from API
    /// responses; `u64` rejects a negative at the parse boundary; NOT a frozen-22 ID).
    pub pr_number: u64,
    pub status: PullRequest,
    pub branch: String,
    pub base: String,
    pub mergeable: Option<bool>,
    pub checks_summary: Option<String>,
    /// D6 — the GitHub diff-stats the §11.2 PR card renders. GET-only octocrab fields
    /// (`models::pulls::PullRequest`, all `Option<u64>`, absent on the list endpoint) → bounded
    /// integers in schemars (LESSON §15 trap 2); `None` when GitHub omitted them.
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    pub commits: Option<u64>,
    /// P4.7 — the PR's current head commit SHA (the anti-race SHA-pin SOURCE the UI reads to form a cat-1
    /// merge/review `sha`/`commit_id`). Captured ONLY from `pr.head.sha` on the authoritative octocrab GET
    /// (`extract_pr_signals`), NEVER from a proposer/UI (§7.2 safety). `None` when GitHub omitted it / the
    /// head has no commits yet. The daemon's anti-race remains the LIVE GitHub 409 on the requester-supplied
    /// value — this field is display/pin-FORMATION only (a stale value yields a fail-closed 409, never a
    /// wrong merge).
    pub head_sha: Option<String>,
    pub pr_checked_at: Timestamp,
}

impl PullRequestSynced {
    /// The EventTypeRegistry name — ONE home (edges' github sync executor emit path + `proj_pull_request`).
    pub const EVENT_TYPE: &'static str = "PullRequestSynced";
}

/// `PullRequestMerged` payload (§7.1; D9/P4.7 — the cat-1 `github.merge_pr` mutation). Emitted by the
/// `GithubExecutor`'s merge arm on a SUCCESSFUL octocrab `pulls().merge()` (SHA-pinned to the approved
/// head); the `PullRequestProjector` folds it → terminal `Merged` (§5.1) on the `proj_pull_request` row.
/// The PR/repo IDENTITY is on the [`crate::event_envelope::EventEnvelope`] columns + the action's Repo
/// `resource_ref` (the projector's repo_id sibling-read); `pr_number` is the GitHub-native PR id echoed
/// from the merge inputs (the projector keys the `{repo_id}#{pr_number}` row). `merge_commit_sha` is the
/// resulting merge-commit SHA (None if the API omitted it). Optionals serialize as explicit `null` (no
/// `skip_serializing_if`) for a stable §2.5-seam field-name snapshot (LESSON §15 trap 3).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct PullRequestMerged {
    /// the GitHub-native PR number merged (echoed from the merge inputs; a non-negative external natural).
    pub pr_number: u64,
    /// the resulting merge-commit SHA (None when the API response omitted it).
    pub merge_commit_sha: Option<String>,
    pub merged_at: Timestamp,
}

impl PullRequestMerged {
    /// The EventTypeRegistry name — ONE home (the D9 `github.merge_pr` emit path + `proj_pull_request`).
    pub const EVENT_TYPE: &'static str = "PullRequestMerged";
}

/// `ReviewSynced` payload (§7.1; D5b-1 — the structured-review vertical). Feeds `proj_review` (§7.2 — the
/// per-review read model the §11.2 PR Review Workspace renders). `review_id`/`pr_number` are GitHub-native
/// externals (non-negative naturals → `u64`, not frozen-22 IDs). `state` is the frozen [`ReviewState`]
/// value enum (no fork). `body` is FREE-FORM user review text — §15-redacted at persist like every payload
/// (the projector folds the persisted/redacted event; the row serves redacted). `submitted_at` is `None`
/// for a pending review. The live GitHub producer is D5b-2; D5b-1 is fixture-fed.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ReviewSynced {
    /// the GitHub-native review id — a globally-unique non-negative external natural (the proj_review PK).
    pub review_id: u64,
    /// the PR this review belongs to (GitHub-native PR number; an FK display column on the row).
    pub pr_number: u64,
    pub reviewer: String,
    pub state: ReviewState,
    pub submitted_at: Option<Timestamp>,
    pub body: Option<String>,
    pub review_synced_at: Timestamp,
}

impl ReviewSynced {
    /// The EventTypeRegistry name — ONE home (the D5b-2 github review-sync emit path + `proj_review`).
    pub const EVENT_TYPE: &'static str = "ReviewSynced";
}

/// `ReviewSubmitted` payload (§7.1; D10/P4.7 — the cat-1 `github.submit_review` mutation). The WRITE
/// counterpart to [`ReviewSynced`] (a *read* sync): emitted by the `GithubExecutor`'s submit arm on a
/// successful octocrab `create_review` (SHA-pinned to the reviewed head via `commit_id`); the
/// `ReviewProjector` folds it → `proj_review` (upsert by `review_id`, identical to the `ReviewSynced`
/// fold). A NEW event, NOT a `ReviewSynced` reuse — "the user SUBMITTED this verdict" ≠ "we synced an
/// existing review" (the D9 `PullRequestMerged`-not-`PullRequestSynced` audit-semantics precedent). The
/// review/repo IDENTITY is on the envelope + the action's Repo `resource_ref`; `pr_number` echoes the
/// reviewed PR. `state` reuses the frozen [`ReviewState`] value enum. `body` is FREE-FORM user review
/// text — §15-redacted at persist like every payload (the `ReviewSynced.body` precedent). `commit_id` =
/// the reviewed head SHA (audit-integrity). Optionals serialize as explicit `null` (no
/// `skip_serializing_if`) for a stable §2.5-seam field-name snapshot (LESSON §15 trap 3).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ReviewSubmitted {
    /// the GitHub-native review id from the create-response (the proj_review PK).
    pub review_id: u64,
    /// the PR this review was submitted to (GitHub-native PR number; a display column on the row).
    pub pr_number: u64,
    pub reviewer: String,
    pub state: ReviewState,
    pub body: Option<String>,
    pub submitted_at: Option<Timestamp>,
    /// the reviewed head SHA the verdict is pinned to (audit-integrity; None only pre-pin).
    pub commit_id: Option<String>,
}

impl ReviewSubmitted {
    /// The EventTypeRegistry name — ONE home (the D10 `github.submit_review` emit path + `proj_review`).
    pub const EVENT_TYPE: &'static str = "ReviewSubmitted";
}

/// `IntegrationConnectionRegistered` payload (§7.1; P7.1 — from connecting GitHub/Linear). Feeds a
/// private `integration_connections` registry. `connection_id` is an edges-private id (NOT a frozen-22
/// `IdKind` — the `terminal_id`/`out_` bare-String precedent). **§15 rule #4 — load-bearing:**
/// `keychain_ref` is a NON-SECRET POINTER into the OS keychain (the entry name/ref), NEVER the token
/// itself — the secret stays in the keychain; events hold pointers only.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct IntegrationConnectionRegistered {
    pub connection_id: String,
    pub provider: Provider,
    /// **§15 rule #4:** a NON-SECRET keychain POINTER (the keychain entry name/ref), NEVER the token.
    pub keychain_ref: String,
    pub account: Option<String>,
}

impl IntegrationConnectionRegistered {
    /// The EventTypeRegistry name — ONE home (edges' connect executor emit path + the connections registry).
    pub const EVENT_TYPE: &'static str = "IntegrationConnectionRegistered";
}

/// `IntegrationLiveWritesSet` payload (§7.1; P4.7/083 Q3) — the per-connection live-writes AUTHORIZATION
/// flip the `integration.set_live_writes` Gateway action emits. The `IntegrationConnection` projector
/// folds it → `proj_integration_connection.live_writes_enabled` (derive-from-event, default OFF,
/// rebuild-safe — LESSON §17), gating whether the authed GitHub clients go live. **§15 — load-bearing:**
/// an authorization-state flip carries NO secret — the payload is `{connection_id, enabled}` only, never
/// a token (the keychain pointer already lives on the connection's registration; this event never touches
/// it). The `off`-flip is a real authorization mutation → it is audited too (both directions emit).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct IntegrationLiveWritesSet {
    pub connection_id: String,
    /// the live-writes authorization state this flip sets (true = ON / authed writes permitted).
    pub enabled: bool,
}

impl IntegrationLiveWritesSet {
    /// The EventTypeRegistry name — ONE home (the `integration.set_live_writes` executor emit path + the
    /// `proj_integration_connection` fold).
    pub const EVENT_TYPE: &'static str = "IntegrationLiveWritesSet";
}

/// `GithubSyncFailed` payload (§7.1/§17; P7.1 — the **non-auth** `*SyncFailed`). Driven by edges'
/// integration classifier's TERMINAL non-auth class (a `ClientError` sync failure, NO profile
/// mutation). **§15 — load-bearing:** `reason` is a redaction-safe STRUCTURAL class-name (the
/// classifier's terminal class), NEVER raw API response text. The `auth_expired` variant (the
/// classifier's `AuthFailed` class → `ExecutionProfile→auth_expired`) is **DEFERRED** — its 0.5b
/// ExecutionProfile gate + a §17/INV-SEC re-review; R1b is the non-auth variant ONLY.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct GithubSyncFailed {
    pub provider: Provider,
    /// **§15:** a STRUCTURAL class-name (the classifier's terminal class), NEVER raw API text.
    pub reason: String,
    pub failed_at: Timestamp,
}

impl GithubSyncFailed {
    /// The EventTypeRegistry name — ONE home (edges' github sync executor emit path).
    pub const EVENT_TYPE: &'static str = "GithubSyncFailed";
}

/// `LinearSyncFailed` payload (§7.1/§17; P7.1 — the **non-auth** `*SyncFailed`, Linear). Same contract
/// as [`GithubSyncFailed`] — see its doc for the §15 `reason` structural-class-name discipline + the
/// deferred `auth_expired` variant.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct LinearSyncFailed {
    pub provider: Provider,
    /// **§15:** a STRUCTURAL class-name (the classifier's terminal class), NEVER raw API text.
    pub reason: String,
    pub failed_at: Timestamp,
}

impl LinearSyncFailed {
    /// The EventTypeRegistry name — ONE home (edges' linear sync executor emit path).
    pub const EVENT_TYPE: &'static str = "LinearSyncFailed";
}

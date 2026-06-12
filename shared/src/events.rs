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

use crate::harness::TelemetrySample;
use crate::objects::{DeviceId, LocalRunnerId};
use crate::status::Session;

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

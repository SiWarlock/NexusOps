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

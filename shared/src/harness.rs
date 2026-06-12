//! §9.1 HarnessAdapter normalized return types (ARCHITECTURE §9.1, LOCKED ADR-006; §5.0).
//!
//! The driver-agnostic data both the Claude (3.2) + Codex (3.3) adapters map INTO — frozen
//! here as the next §2.5-seam shared contract (the UI/Brain consume them: the UsageMeter reads
//! `TelemetrySample`/`MetricQuality`, per-capability degradation reads `HarnessCapabilities`).
//! `schemars` → versioned JSON-Schema → generated Zod/Pydantic (§5.0). Reject-unknown end-to-end.
//!
//! **`NormalizedStatus` is NOT a new type** — it is the frozen §5.1 [`crate::status::Session`]
//! machine (the 17 states; status.rs:45 names it "the driver-agnostic normalized vocabulary the
//! §9.1 harness adapters map INTO"). Re-exported here so the §9.1 vocabulary reads as named,
//! without forking the value set (pinned == `Session` by `test_normalized_status_is_session`).
//!
//! `ResumeResult` is deliberately ABSENT — it is **daemon-internal** (`daemon/src/harness/mod.rs`),
//! NOT frozen here: resume/survival is a Phase-4 §8/§17 design and the ui's provisional already
//! pins a `ResumeMode` enum; freezing a shape now would collide at P4 (a breaking reshape the
//! §2.5-seam freeze discipline exists to prevent). It freezes in `shared/` with the P4 survival schema.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The §5.1 [`Session`](crate::status::Session) machine, named as the §9.1 normalized status
/// vocabulary. A re-export, NOT a fork — the adapters map their driver-specific lifecycles INTO
/// these 17 states (status derived from SDK/app-server streams, never PTY-scraped — safety #9).
pub use crate::status::Session as NormalizedStatus;

/// Confidence in a telemetry metric (§9.1/§7.2). Carried on ALL telemetry so the UsageMeter
/// degrades honestly (§11.7) — e.g. Codex has no context window metadata → `context_pct` is
/// None and quality is `Unavailable`, never a faked number. snake_case wire; reject-unknown (§15).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricQuality {
    /// authoritative (e.g. an SDK `ResultMessage.usage` figure)
    Exact,
    /// derived/approximated (e.g. computed from a transcript)
    Estimated,
    /// the harness cannot report this metric (e.g. Codex context-%)
    Unavailable,
}

impl MetricQuality {
    /// Every variant, declaration order.
    pub const ALL: &'static [Self] = &[Self::Exact, Self::Estimated, Self::Unavailable];
}

/// A single telemetry sample (§9.1) — a per-heartbeat usage observation. `context_pct` is an
/// explicit `Option` (None = "unknown", never 0% — §11.4); `cost_estimate` is DERIVED (§9.1 prose
/// `cost` → `cost_estimate`, the "estimate" honesty). `tokens_in`/`tokens_out` are the per-sample
/// DELTAS the `proj_usage_ledger` projector SUMs (the adapter emits deltas, not cumulative totals —
/// a 3.2/3.3 acceptance pin). Optionals serialize as explicit `null` (no `skip_serializing_if`) so
/// the §2.5-seam field-name snapshot is stable (LESSON §15 trap 3).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct TelemetrySample {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub context_pct: Option<f32>,
    pub cost_estimate: f64,
    pub metric_quality: MetricQuality,
}

/// A reference to a harness transcript (§9.1) — the JSONL the adapter tails (Claude
/// `~/.claude/projects/.../<id>.jsonl`). `is_in_place` = the harness writes its own file we read
/// (vs a copy we manage). The daemon stores the `hash` for change detection; never the content (§15).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct TranscriptRef {
    pub path: String,
    pub hash: String,
    pub is_in_place: bool,
}

/// Per-harness capability matrix (§9.1; PRD HARN-5's 10 fields) — drives per-capability UI
/// degradation (e.g. `supports_context_metadata=false` for Codex → render "unknown"). The UI
/// consumes THIS (not the mutation-coverage matrix, which is daemon-internal). Reject-unknown (§15).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct HarnessCapabilities {
    pub supports_terminal: bool,
    pub supports_resume: bool,
    pub supports_transcript_read: bool,
    pub supports_tool_call_parsing: bool,
    pub supports_usage_metadata: bool,
    pub supports_context_metadata: bool,
    pub supports_command_injection: bool,
    pub supports_subagents: bool,
    pub supports_hooks: bool,
    pub supports_cloud_tasks: bool,
}

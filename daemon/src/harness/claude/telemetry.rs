//! Claude adapter telemetry (brief 044) — the Claude-specific usage PARSER.
//!
//! **Brief 074 hoist:** the harness-NEUTRAL primitives (`telemetry_sample`, `UsageReading`,
//! `TelemetryEventSink`, `UsageSource`, `TelemetrySinkFactory`) moved to
//! [`crate::harness::telemetry`] (shared by Claude + Codex; LESSON §27 "generalizes to Codex"). They
//! are RE-EXPORTED here so every existing consumer (`claude/mod.rs`, `runtime::telemetry_sink`, the
//! `claude_telemetry.rs`/`telemetry_pump.rs` suites) keeps importing them from this path unchanged.
//! This module now owns ONLY the Claude-specific parser: Claude's transcript `ResultMessage` carries an
//! UPSTREAM-reported `total_cost_usd` (Exact), unlike Codex which derives cost locally (3.3d).
//!
//! The PRODUCTION sink-binding, the periodic pump, and the live transcript/statusLine ingestion I/O
//! land at the P4 drive loop.

use serde::Deserialize;

// the harness-neutral primitives (hoisted, brief 074) — re-exported so `claude::telemetry::{…}` paths
// resolve unchanged for every existing importer + the parser's `UsageReading` return type.
pub use crate::harness::telemetry::{
    telemetry_sample, TelemetryEventSink, TelemetrySinkFactory, UsageReading, UsageSource,
};

// ---- the parser: external Claude usage JSON → a typed UsageReading (defensive, fail-closed) -------

/// The `usage` object of a Claude `ResultMessage` (the documented token fields). NOT a frozen
/// contract — an EXTERNAL upstream shape we parse defensively, so extra fields (cache tokens, …) are
/// tolerated (no `deny_unknown_fields`); the two token fields are REQUIRED.
#[derive(Deserialize)]
struct ResultUsage {
    input_tokens: u64,
    output_tokens: u64,
}

/// A Claude structured usage message (the transcript `ResultMessage` / statusLine merge). External —
/// tolerant of unknown fields; `usage` + `total_cost_usd` are required, `model`/`context_pct` optional.
#[derive(Deserialize)]
struct ResultMessage {
    usage: ResultUsage,
    total_cost_usd: f64,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    context_pct: Option<f32>,
}

/// Parse a Claude structured usage source into a typed cumulative [`UsageReading`]. The Claude
/// transcript is an EXTERNAL upstream we parse DEFENSIVELY: extra/unknown fields are tolerated
/// (forward-compat — the real `ResultMessage` carries many), but a missing or wrong-typed REQUIRED
/// field fails CLOSED → `None` (never a partial/fabricated reading; §7.2 harness-derived SoT, §15
/// fail-closed). **NOTE (Q5):** the exact field names are validated against a real Claude session at
/// the impl-fixture/P4 boundary (the 042/043 precedent) — a wrong name is a one-line fix here, never a
/// design change. The live transcript/statusLine I/O that FEEDS this parser is P4. Claude's
/// `total_cost_usd` is the UPSTREAM cost (Exact-quality), in contrast to Codex's locally-derived cost.
pub fn parse_usage_reading(json: &str) -> Option<UsageReading> {
    let m: ResultMessage = serde_json::from_str(json).ok()?;
    Some(UsageReading {
        tokens_in: m.usage.input_tokens,
        tokens_out: m.usage.output_tokens,
        context_pct: m.context_pct,
        cost: m.total_cost_usd,
        model: m.model,
    })
}

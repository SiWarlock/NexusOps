//! Claude adapter telemetry (brief 044) — per-heartbeat token/cost DELTAS + the emission seam.
//!
//! The harness adapter WITNESSES usage and emits it as a non-mutation `TelemetrySampled` OBSERVATION
//! event (NOT a Gateway Action — INV-SEC-1 routes state *mutations*; LESSON §23). The load-bearing
//! distinction: `tokens_in`/`tokens_out`/`cost_estimate` are per-heartbeat **DELTAS**
//! (current cumulative − last emitted) because the `proj_usage_ledger` projector SUMs them — a
//! cumulative snapshot would over-count; `context_pct` rides as the **CURRENT gauge** (pass-through),
//! because the projector takes its MAX (`context_pct_max`), so a delta would be meaningless.
//!
//! This module is the pure delta-derivation + the parser (deterministic, test-first) + the
//! [`TelemetryEventSink`] seam (the 3.4 `TerminalEventSink` precedent). The PRODUCTION sink-binding
//! (the write-actor `WriteHandle::append`), the periodic pump (driven from the statusLine
//! `refreshInterval` heartbeat), the `HarnessAdapter` trait async-ify, and the live transcript/
//! statusLine ingestion I/O all land at the **P4** session-lifecycle drive loop.

use serde::Deserialize;

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::{MetricQuality, TelemetrySample};
use nexusops_shared::ids::{ProjectId, SessionId};

/// A single CUMULATIVE usage reading from Claude's structured source (the transcript `ResultMessage`
/// `usage`/`total_cost_usd` + the statusLine input). A SNAPSHOT — [`push_usage`](super::ClaudeAdapter::push_usage)
/// deltas the next reading against the prior. `cost` is Claude's REPORTED cumulative cost (the
/// "estimate" honesty — no local pricing table); `model` is the rollup dim the statusLine/ResultMessage
/// names. `context_pct` is the current context-window gauge (`None` = unknown, never a faked 0%).
///
/// **Assumption (live-validated at P4):** `cost`/`tokens_*` are cumulative + monotonic non-negative
/// (Claude's `total_cost_usd` never decreases). The delta path guards a non-monotonic *delta* (tokens
/// `saturating_sub`, cost `.max(0.0)`); validating/clamping a non-monotonic upstream *cumulative* (an
/// implausible negative-cost reading inflating the next delta) is a P4 live-data-quality concern.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageReading {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub context_pct: Option<f32>,
    pub cost: f64,
    pub model: Option<String>,
}

/// The pure cumulative→DELTA derivation (§9.1). `prev` is the last emitted cumulative reading (`None`
/// before the first → a full delta from zero). Tokens/cost are differenced; `context_pct` is passed
/// through as the CURRENT gauge (NEVER differenced — the projector MAXes it). `metric_quality`
/// degrades honestly: `Exact` when context is present, `Estimated` when it is unknown (never `0%`).
pub fn telemetry_sample(prev: Option<&UsageReading>, reading: &UsageReading) -> TelemetrySample {
    let (prev_in, prev_out, prev_cost) = match prev {
        Some(p) => (p.tokens_in, p.tokens_out, p.cost),
        None => (0, 0, 0.0),
    };
    TelemetrySample {
        // saturating_sub: cumulative readings are monotonic; a non-monotonic reading (a counter reset
        // / misread) floors the delta at 0 rather than underflowing (u64 underflow panics in debug,
        // wraps in release) — the honest fail-safe, never a phantom huge delta.
        tokens_in: reading.tokens_in.saturating_sub(prev_in),
        tokens_out: reading.tokens_out.saturating_sub(prev_out),
        // context_pct is a CURRENT gauge — pass it through (the projector takes context_pct_max);
        // NEVER a delta (a differenced gauge would mis-bucket).
        context_pct: reading.context_pct,
        // cost_estimate = the delta of Claude's REPORTED cumulative cost; floored at 0 for the same
        // non-monotonic guard.
        cost_estimate: (reading.cost - prev_cost).max(0.0),
        // honest degradation (§11.4): context present → Exact; absent → Estimated, never a faked 0%.
        // (Claude supports context metadata → normally Exact; `Unavailable` is the Codex 3.3 case.)
        metric_quality: if reading.context_pct.is_some() {
            MetricQuality::Exact
        } else {
            MetricQuality::Estimated
        },
    }
}

/// The seam through which the adapter emits its [`TelemetrySampled`] observation event (the 3.4
/// [`TerminalEventSink`](crate::terminal::TerminalEventSink) precedent — `Send`, single-thread-owned
/// by the per-session drive loop). The production impl binds the write-actor's `WriteHandle::append`
/// (the §15-gated, non-mutation observation write — INV-SEC-1 governs *mutations*, a usage observation
/// is none; LESSON §10/§23) at the P4 drive loop; tests use a collecting double. **P4 NOTE:** if the
/// production binding shares the sink ACROSS threads (vs moving it into the one drive-loop thread),
/// add a `+ Sync` bound here — `Send` alone suffices for the single-owner cadence 044 + P4 target.
pub trait TelemetryEventSink: Send {
    fn emit_telemetry(&self, event: TelemetrySampled);
}

/// The per-tick cumulative-usage SOURCE the telemetry pump drains (4.0c). The session-actor's
/// telemetry tick calls [`poll_usage`](Self::poll_usage); each `Some` cumulative reading is deltaed +
/// emitted via the bound sink ([`ClaudeAdapter::poll_telemetry`](super::ClaudeAdapter)). The
/// PRODUCTION source is the **P4** live hook-receiver/statusLine ingestion feed (the deferred ingress
/// seam — 4.0c binds the sink + the pump-tick + this trait, but the live transcript/statusLine I/O
/// that FEEDS it lands at P4, the "build the mechanism, wire the ingress later" pattern); tests drive
/// a scripted source. `Send` — owned by the one drive-loop thread (the [`TelemetryEventSink`] precedent).
pub trait UsageSource: Send {
    /// The next cumulative reading available this tick, or `None` if none (no reading yet / unchanged).
    /// A `None` is a no-op tick — never a fabricated reading (§11.4). The cumulative delta inherently
    /// coalesces a faster-than-tick source (the next poll's delta covers the gap).
    fn poll_usage(&mut self) -> Option<UsageReading>;
}

/// Builds a per-session [`TelemetryEventSink`] (4.0c). The PRODUCTION impl (`runtime::WriteActorTelemetrySinkFactory`)
/// closes over the write-actor `WriteHandle` + the daemon `Clock`; the launcher (in `session/`, the
/// cat-1 boundary) holds this as an OPAQUE `Box<dyn>` and never imports `WriteHandle` — so emission is
/// injected from OUTSIDE `session/` (the §15 redaction gate + the write-actor append run in the
/// production impl, NOT a bypass — LESSON §23). `Send + Sync` — shared across the launcher's
/// `launch_session` calls (one sink minted per launched session).
pub trait TelemetrySinkFactory: Send + Sync {
    /// Mint the telemetry sink for one launched session (its `TelemetrySampled` events carry this
    /// session/project identity on the envelope; the `proj_usage_ledger` buckets by them).
    fn make_sink(
        &self,
        session_id: &SessionId,
        project_id: Option<&ProjectId>,
    ) -> Box<dyn TelemetryEventSink>;
}

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
/// design change. The live transcript/statusLine I/O that FEEDS this parser is P4.
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

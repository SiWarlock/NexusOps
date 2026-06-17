//! Harness-neutral telemetry primitives (brief 074 hoist — the 044 machinery, harness-agnostic).
//!
//! The per-heartbeat cumulative→DELTA derivation + the emission seam are shared by EVERY harness
//! adapter (Claude 044, Codex 3.3d) — they have nothing harness-specific. LESSON §27 anticipated this
//! ("generalizes to Codex"); this module is that neutral home. Each harness keeps only its own
//! vendor-specific PARSER (`claude::telemetry::parse_usage_reading`; `codex::telemetry` pricing) and
//! re-exports / imports these primitives.
//!
//! The load-bearing distinction (LESSON §27): `tokens_in`/`tokens_out`/`cost_estimate` are per-heartbeat
//! **DELTAS** (current cumulative − last emitted) because `proj_usage_ledger` SUMs them — a cumulative
//! snapshot would over-count; `context_pct` rides as the **CURRENT gauge** (pass-through), because the
//! projector takes its MAX (`context_pct_max`), so a delta would be meaningless. Telemetry is a
//! non-mutation OBSERVATION event (LESSON §23) — NOT a Gateway Action.

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::{MetricQuality, TelemetrySample};
use nexusops_shared::ids::{ProjectId, SessionId};

/// A single CUMULATIVE usage reading from a harness's structured source. A SNAPSHOT — `push_usage`
/// deltas the next reading against the prior. `cost` is the harness's cumulative cost: Claude REPORTS
/// it upstream (`total_cost_usd`); Codex has no cost field, so it is LOCALLY DERIVED (a per-model rate
/// table, `codex::telemetry::codex_cost_estimate`) — the source builds this struct with the derived
/// cumulative cost (3.3d). `model` is the rollup dim; `context_pct` is the current context-window gauge
/// (`None` = unknown, never a faked 0%).
///
/// **Assumption (live-validated):** `cost`/`tokens_*` are cumulative + monotonic non-negative. The
/// delta path guards a non-monotonic *delta* (tokens `saturating_sub`, cost `.max(0.0)`); validating a
/// non-monotonic upstream *cumulative* is a live-data-quality concern at the drive loop.
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
///
/// **Harness-neutral:** this sets quality from the CONTEXT signal only. A harness whose cost is
/// LOCALLY DERIVED (Codex) further caps the quality to `Estimated` AFTER this fn
/// (`codex::telemetry::apply_cost_honesty`) — a local cost estimate is not authoritative (§11.4).
pub fn telemetry_sample(prev: Option<&UsageReading>, reading: &UsageReading) -> TelemetrySample {
    let (prev_in, prev_out, prev_cost) = match prev {
        Some(p) => (p.tokens_in, p.tokens_out, p.cost),
        None => (0, 0, 0.0),
    };
    TelemetrySample {
        // saturating_sub: cumulative readings are monotonic; a non-monotonic reading (a counter reset /
        // misread) floors the delta at 0 rather than underflowing — the honest fail-safe, never a
        // phantom huge delta.
        tokens_in: reading.tokens_in.saturating_sub(prev_in),
        tokens_out: reading.tokens_out.saturating_sub(prev_out),
        // context_pct is a CURRENT gauge — pass it through (the projector takes context_pct_max);
        // NEVER a delta (a differenced gauge would mis-bucket).
        context_pct: reading.context_pct,
        // cost_estimate = the delta of the cumulative cost; floored at 0 for the same non-monotonic guard.
        cost_estimate: (reading.cost - prev_cost).max(0.0),
        // honest degradation (§11.4): context present → Exact; absent → Estimated, never a faked 0%.
        metric_quality: if reading.context_pct.is_some() {
            MetricQuality::Exact
        } else {
            MetricQuality::Estimated
        },
    }
}

/// The seam through which an adapter emits its [`TelemetrySampled`] observation event (the 3.4
/// `TerminalEventSink` precedent — `Send`, single-thread-owned by the per-session drive loop). The
/// production impl binds the write-actor's `WriteHandle::append` (the §15-gated, non-mutation
/// observation write — INV-SEC-1 governs *mutations*, a usage observation is none; LESSON §10/§23) at
/// the drive loop; tests use a collecting double.
pub trait TelemetryEventSink: Send {
    fn emit_telemetry(&self, event: TelemetrySampled);
}

/// The per-tick cumulative-usage SOURCE the telemetry pump drains. The session-actor's telemetry tick
/// calls [`poll_usage`](Self::poll_usage); each `Some` cumulative reading is deltaed + emitted via the
/// bound sink. The PRODUCTION source is the live ingestion feed (the deferred ingress seam — the
/// "build the mechanism, wire the ingress later" pattern); tests drive a scripted source. `Send` —
/// owned by the one drive-loop thread (the [`TelemetryEventSink`] precedent).
pub trait UsageSource: Send {
    /// The next cumulative reading available this tick, or `None` if none (no reading yet / unchanged).
    /// A `None` is a no-op tick — never a fabricated reading (§11.4). The cumulative delta inherently
    /// coalesces a faster-than-tick source (the next poll's delta covers the gap).
    fn poll_usage(&mut self) -> Option<UsageReading>;
}

/// Builds a per-session [`TelemetryEventSink`]. The PRODUCTION impl
/// (`runtime::WriteActorTelemetrySinkFactory`) closes over the write-actor `WriteHandle` + the daemon
/// `Clock`; the launcher (in `session/`, the cat-1 boundary) holds this as an OPAQUE `Box<dyn>` and
/// never imports `WriteHandle` — so emission is injected from OUTSIDE `session/` (the §15 redaction gate
/// and the write-actor append both run in the production impl, NOT a bypass — LESSON §23). `Send + Sync`
/// — shared across the launcher's `launch_session` calls (one sink minted per launched session).
pub trait TelemetrySinkFactory: Send + Sync {
    /// Mint the telemetry sink for one launched session (its `TelemetrySampled` events carry this
    /// session/project identity on the envelope; the `proj_usage_ledger` buckets by them).
    fn make_sink(
        &self,
        session_id: &SessionId,
        project_id: Option<&ProjectId>,
    ) -> Box<dyn TelemetryEventSink>;
}

//! Codex telemetry pricing (brief 074, the genuinely-new-vs-044 piece).
//!
//! Codex's `token_count` stream carries NO cost field (unlike Claude's upstream-reported
//! `total_cost_usd`), so cost is derived LOCALLY from a per-model rate table. The honesty rule (§11.4):
//! a locally-derived cost is NEVER authoritative → its `metric_quality` is capped to `Estimated` (the
//! load-bearing Claude-Exact vs Codex-Estimated divergence). An UNKNOWN/absent model derives `0.0`
//! (conservative — the sample still carries the token deltas; a flagged 0.0 beats a fabricated cost),
//! and leaves the context-driven quality intact.
//!
//! The pure delta-emit / gauge / sink machinery is the harness-neutral [`crate::harness::telemetry`]
//! (the 044 hoist); this module adds only the Codex pricing + the honesty downgrade.

use nexusops_shared::harness::{MetricQuality, TelemetrySample};

/// A per-model price (USD per 1M tokens).
struct CodexRate {
    input_per_1m: f64,
    output_per_1m: f64,
}

/// The per-model rate table. **🔴 INDICATIVE rates, as of 2026-06-17 — NOT authoritative.** Every
/// derived cost is flagged `metric_quality=Estimated` ([`apply_cost_honesty`]); these consts are an
/// easily-updatable starting point. **FUTURE-TODO (Step-9 carry-forward):** refresh against current
/// OpenAI published Codex pricing before any accurate-cost *display* surface ships. An unknown/absent
/// model → `None` → `codex_cost_estimate` yields `0.0` (never fabricated).
fn rate_for(model: Option<&str>) -> Option<CodexRate> {
    match model? {
        // the codebase's referenced Codex models (config default + the launch profile). Indicative.
        "gpt-5.1-codex-max" | "gpt-5.1-codex" => Some(CodexRate {
            input_per_1m: 1.25,
            output_per_1m: 10.00,
        }),
        // same rates as gpt-5.1-codex as of 2026-06-17 (intentional, not a copy-paste error); verify
        // independently on the next pricing refresh — they may diverge.
        "gpt-5.5" => Some(CodexRate {
            input_per_1m: 1.25,
            output_per_1m: 10.00,
        }),
        _ => None,
    }
}

/// The PURE Codex cost derivation: `tokens × per-model rate`. An unknown/absent model → **0.0**
/// (conservative — never a fabricated cost; the sample still carries its token deltas). Pricing is
/// LINEAR, so deriving the CUMULATIVE cost here and letting `telemetry_sample` delta it equals
/// `rate × token_delta` (brief Q4). f64 arithmetic — large token counts stay finite + non-negative
/// (no overflow).
pub fn codex_cost_estimate(tokens_in: u64, tokens_out: u64, model: Option<&str>) -> f64 {
    match rate_for(model) {
        Some(r) => {
            (tokens_in as f64 / 1_000_000.0) * r.input_per_1m
                + (tokens_out as f64 / 1_000_000.0) * r.output_per_1m
        }
        None => 0.0,
    }
}

/// Whether `model` has a rate (i.e. a cost was/would be locally derived). Drives the honesty downgrade.
pub fn is_priced_model(model: Option<&str>) -> bool {
    rate_for(model).is_some()
}

/// The §11.4 cost-honesty downgrade: when a cost is LOCALLY derived (the model is priced), cap the
/// sample's `metric_quality` at `Estimated` — a local pricing estimate is never authoritative (unlike
/// Claude's upstream cost). When the model is unknown (no cost derived, 0.0) the context-driven quality
/// from `telemetry_sample` stands. **Only `Exact` is capped** (the approved Q3 rule): `Estimated` is
/// already at the cap, and `Unavailable` is LEFT as-is by design — `metric_quality` reflects the WEAKEST
/// axis, and "no context window" (Unavailable) is weaker than a cost estimate, so a derived cost does
/// not LIFT it (the `cost_estimate` field still carries the value; the ledger SUMs it regardless of the
/// quality flag). The mixed `Unavailable`-quality-with-a-nonzero-cost case is pinned by a test.
pub fn apply_cost_honesty(mut sample: TelemetrySample, model: Option<&str>) -> TelemetrySample {
    if is_priced_model(model) && sample.metric_quality == MetricQuality::Exact {
        sample.metric_quality = MetricQuality::Estimated;
    }
    sample
}

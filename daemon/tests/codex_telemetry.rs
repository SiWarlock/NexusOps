//! Brief 074 — Codex telemetry emission (the 044 analog + Codex local pricing). RED-first.
//!
//! ARCHITECTURE §9.1 (HarnessAdapter `telemetry_heartbeat`), §7.2 (harness-derived SoT), §18 (usage
//! rollups). REUSES the 044 machinery — the pure cumulative→DELTA `telemetry_sample`, the
//! `TelemetryEventSink` seam, `push_usage`/`telemetry_heartbeat`/`poll_telemetry` — hoisted to the
//! harness-neutral `harness::telemetry` (Step-2.5 Q1). **The genuinely-new piece vs 044:** Codex
//! `token_count` carries NO cost field, so a PURE local per-model pricing fn derives `cost_estimate`,
//! and `metric_quality` is honestly downgraded to `Estimated` for any locally-derived cost (the
//! §11.4 Claude-Exact vs Codex-Estimated divergence). NON-safety (a non-mutation OBSERVATION event
//! through the existing §15-gated write-actor append — LESSON 23/27 — not a Gateway Action).

use std::sync::{Arc, Mutex};

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::MetricQuality;

use nexusopsd::harness::codex::telemetry::{
    apply_cost_honesty, codex_cost_estimate, is_priced_model,
};
use nexusopsd::harness::codex::CodexAdapter;
use nexusopsd::harness::telemetry::{TelemetryEventSink, UsageReading, UsageSource};
use nexusopsd::harness::HarnessAdapter;

// a known model present in the rate table (the exact rates are an updatable const; the tests pin the
// FORMULA + this model's derived value, so a rate change is an intentional test update).
const MODEL: &str = "gpt-5.1-codex-max";

/// a collecting [`TelemetryEventSink`] double — records every emitted event (the 044 test pattern).
#[derive(Clone, Default)]
struct CollectingSink {
    events: Arc<Mutex<Vec<TelemetrySampled>>>,
}
impl CollectingSink {
    fn events(&self) -> Vec<TelemetrySampled> {
        self.events.lock().unwrap().clone()
    }
}
impl TelemetryEventSink for CollectingSink {
    fn emit_telemetry(&self, event: TelemetrySampled) {
        self.events.lock().unwrap().push(event);
    }
}

/// a scripted [`UsageSource`] double — yields its queued cumulative readings in order, then None.
struct ScriptedSource {
    readings: std::collections::VecDeque<UsageReading>,
}
impl UsageSource for ScriptedSource {
    fn poll_usage(&mut self) -> Option<UsageReading> {
        self.readings.pop_front()
    }
}

/// a cumulative Codex usage reading with the cost DERIVED from the local table (the source's job, Q4 —
/// pricing is linear so `telemetry_sample` deltas the cumulative cost == `rate × token_delta`).
fn reading(
    tokens_in: u64,
    tokens_out: u64,
    context_pct: Option<f32>,
    model: Option<&str>,
) -> UsageReading {
    UsageReading {
        tokens_in,
        tokens_out,
        context_pct,
        cost: codex_cost_estimate(tokens_in, tokens_out, model),
        model: model.map(|s| s.to_string()),
    }
}

fn adapter() -> CodexAdapter {
    CodexAdapter::new(
        std::path::PathBuf::from("/tmp/rollout.jsonl"),
        "sess_x".to_string(),
    )
}

// ---- RED #1 — per-heartbeat DELTAS, not cumulative (the proj_usage_ledger SUMs) -----------------

#[test]
fn test_codex_heartbeat_emits_deltas_not_cumulative() {
    // spec(§9.1, LESSON §27) — two CUMULATIVE readings → the second heartbeat returns the DELTA
    // (current − last), not the cumulative snapshot, so the SUMming ledger doesn't over-count.
    let mut a = adapter();
    a.push_usage(reading(100, 20, Some(30.0), Some(MODEL)));
    a.push_usage(reading(250, 60, Some(55.0), Some(MODEL)));
    let s = a
        .telemetry_heartbeat()
        .expect("a sample after two readings");
    assert_eq!(
        s.tokens_in, 150,
        "delta = 250 − 100 (not the cumulative 250)"
    );
    assert_eq!(s.tokens_out, 40, "delta = 60 − 20 (not the cumulative 60)");
}

// ---- RED #2 — the first reading is a full delta from zero; None before any reading ---------------

#[test]
fn test_codex_first_reading_full_delta_from_zero() {
    // spec(§11.4) — telemetry_heartbeat is None pre-reading (no fabricated pre-reading); the first
    // push_usage emits the reading as-is (delta from 0) — no double-count on session start.
    let mut a = adapter();
    assert!(a.telemetry_heartbeat().is_none(), "None before any reading");
    a.push_usage(reading(100, 20, Some(30.0), Some(MODEL)));
    let s = a
        .telemetry_heartbeat()
        .expect("a sample after the first reading");
    assert_eq!(
        (s.tokens_in, s.tokens_out),
        (100, 20),
        "first delta == first cumulative"
    );
}

// ---- RED #3 — context_pct is a CURRENT gauge (pass-through), never a delta ----------------------

#[test]
fn test_codex_context_pct_is_gauge_not_delta() {
    // spec(§9.1/§11.4) — context_pct rides as the CURRENT gauge (the projector MAXes context_pct_max),
    // NEVER differenced; absent model_context_window → Unavailable (never a faked 0%).
    let mut a = adapter();
    a.push_usage(reading(100, 20, Some(30.0), Some(MODEL)));
    a.push_usage(reading(250, 60, Some(55.0), Some(MODEL)));
    let s = a.telemetry_heartbeat().unwrap();
    assert_eq!(
        s.context_pct,
        Some(55.0),
        "the CURRENT gauge 55, not a 25 delta"
    );
    // a reading with no context → context_pct None (gauge unknown, never faked).
    a.push_usage(reading(300, 80, None, Some(MODEL)));
    assert_eq!(
        a.telemetry_heartbeat().unwrap().context_pct,
        None,
        "no window → None, never 0%"
    );
}

// ---- RED #4 — one push_usage emits exactly one TelemetrySampled via the injected sink ------------

#[test]
fn test_codex_push_usage_emits_one_telemetry_sampled_via_sink() {
    // spec(LESSON §23/§27) — the 044 TelemetryEventSink seam: one push_usage → exactly one
    // TelemetrySampled{sample(delta), model}. The sink double records it.
    let sink = CollectingSink::default();
    let mut a = adapter().with_telemetry_sink(Box::new(sink.clone()));
    a.push_usage(reading(100, 20, Some(30.0), Some(MODEL)));
    let events = sink.events();
    assert_eq!(
        events.len(),
        1,
        "exactly one TelemetrySampled per push_usage"
    );
    assert_eq!(events[0].sample.tokens_in, 100, "carries the delta");
    assert_eq!(
        events[0].model.as_deref(),
        Some(MODEL),
        "carries the model rollup dim"
    );
}

// ---- RED #5 — Codex cost derivation + the §11.4 honesty downgrade (the new-vs-044 piece) ---------

#[test]
fn test_codex_cost_derivation_and_honesty() {
    // spec(§11.4) — a known model derives cost == rate × token_delta; and ANY locally-derived cost
    // downgrades metric_quality Exact→Estimated (a local estimate is NOT authoritative, unlike Claude's
    // upstream-reported cost). Context present would otherwise be Exact — the derived cost caps it.
    let mut a = adapter().with_telemetry_sink(Box::new(CollectingSink::default()));
    a.push_usage(reading(100, 20, Some(30.0), Some(MODEL)));
    a.push_usage(reading(250, 60, Some(55.0), Some(MODEL)));
    let s = a.telemetry_heartbeat().unwrap();
    // the delta cost == the table-derived delta (cumulative cost is linear in tokens).
    let expected_delta =
        codex_cost_estimate(250, 60, Some(MODEL)) - codex_cost_estimate(100, 20, Some(MODEL));
    assert!(
        (s.cost_estimate - expected_delta).abs() < 1e-12,
        "cost delta == rate × token_delta"
    );
    assert!(
        s.cost_estimate > 0.0,
        "a known model derives a positive cost"
    );
    assert_eq!(
        s.metric_quality,
        MetricQuality::Estimated,
        "a locally-derived cost is NEVER Exact (§11.4) — capped to Estimated even with context present"
    );

    // the apply_cost_honesty helper directly: an unknown model (cost 0.0) leaves the context-driven
    // quality (Exact) intact; a priced model caps Exact→Estimated.
    use nexusops_shared::harness::TelemetrySample;
    let exact = TelemetrySample {
        tokens_in: 1,
        tokens_out: 1,
        context_pct: Some(10.0),
        cost_estimate: 0.0,
        metric_quality: MetricQuality::Exact,
    };
    assert_eq!(
        apply_cost_honesty(exact.clone(), None).metric_quality,
        MetricQuality::Exact,
        "unknown model → quality stands"
    );
    assert_eq!(
        apply_cost_honesty(exact, Some(MODEL)).metric_quality,
        MetricQuality::Estimated,
        "priced model → Exact downgraded"
    );
    // the mixed case (the approved Q3 rule, pinned): a priced model with NO context (Unavailable) is
    // LEFT as Unavailable — only Exact is capped; the weakest axis (no context) stands, the cost still
    // rides cost_estimate. A derived cost does NOT lift Unavailable.
    let unavail = TelemetrySample {
        tokens_in: 1,
        tokens_out: 1,
        context_pct: None,
        cost_estimate: 5.0,
        metric_quality: MetricQuality::Unavailable,
    };
    assert_eq!(
        apply_cost_honesty(unavail, Some(MODEL)).metric_quality,
        MetricQuality::Unavailable,
        "a derived cost does NOT lift Unavailable (only Exact is capped — the weakest-axis rule)"
    );
}

// ---- RED #6 — the pure pricing fn edges (a fabricated cost is worse than a flagged 0.0) ----------

#[test]
fn test_codex_pricing_fn_edges() {
    // spec(§11.4) — codex_cost_estimate: 0 tokens → 0.0; an unknown/absent model → 0.0 (conservative,
    // tokens still carried by the sample); large counts → no overflow; cost always ≥ 0.
    assert_eq!(
        codex_cost_estimate(0, 0, Some(MODEL)),
        0.0,
        "0 tokens → 0.0"
    );
    assert_eq!(
        codex_cost_estimate(1000, 1000, Some("totally-unknown-model")),
        0.0,
        "unknown model → 0.0"
    );
    assert_eq!(
        codex_cost_estimate(1000, 1000, None),
        0.0,
        "absent model → 0.0"
    );
    assert!(
        !is_priced_model(Some("totally-unknown-model")),
        "unknown model not priced"
    );
    assert!(!is_priced_model(None), "absent model not priced");
    assert!(is_priced_model(Some(MODEL)), "the known model is priced");
    // every rate-table entry is priced + derives a non-zero cost (guards a future typo silently
    // dropping a model to the `_ => None` arm).
    for m in ["gpt-5.1-codex-max", "gpt-5.1-codex", "gpt-5.5"] {
        assert!(is_priced_model(Some(m)), "{m} is in the rate table");
        assert!(
            codex_cost_estimate(1_000, 1_000, Some(m)) > 0.0,
            "{m} derives a positive cost"
        );
    }
    let big = codex_cost_estimate(u64::MAX, u64::MAX, Some(MODEL));
    assert!(
        big.is_finite() && big >= 0.0,
        "large counts → finite, non-negative (no overflow)"
    );
}

// ---- RED #8 — poll_telemetry drains the UsageSource → push_usage (the pump plumbing) -------------

#[test]
fn test_codex_poll_telemetry_drains_source() {
    // spec(§9.1, the 044 pump seam) — poll_telemetry drains one cumulative reading from the injected
    // UsageSource + routes it through push_usage (delta + emit). One tick → one emitted sample == that
    // reading's delta; a subsequent empty-source tick is a no-op (no fabricated sample).
    let sink = CollectingSink::default();
    let source = ScriptedSource {
        readings: std::collections::VecDeque::from(vec![
            reading(100, 20, Some(30.0), Some(MODEL)),
            reading(250, 60, Some(55.0), Some(MODEL)),
        ]),
    };
    let mut a = adapter()
        .with_telemetry_sink(Box::new(sink.clone()))
        .with_usage_source(Box::new(source));
    a.poll_telemetry(); // drains reading #1 → full delta from 0
    a.poll_telemetry(); // drains reading #2 → delta 150/40
    a.poll_telemetry(); // source empty → no-op
    let events = sink.events();
    assert_eq!(
        events.len(),
        2,
        "one emitted sample per non-empty tick; the empty tick is a no-op"
    );
    assert_eq!(
        (events[0].sample.tokens_in, events[1].sample.tokens_in),
        (100, 150),
        "deltas, not cumulative"
    );
    assert_eq!(
        a.telemetry_heartbeat().unwrap().tokens_in,
        150,
        "heartbeat == the last delta"
    );
}

//! P3.2 part 2 (telemetry — brief 044) — the Claude adapter's telemetry emission path.
//!
//! Per-heartbeat token/cost **DELTAS** (current cumulative − last emitted), NOT cumulative
//! snapshots, so the SUMming `proj_usage_ledger` projector does not over-count (§9.1 / the 3.1
//! `shared/src/harness.rs:48` "adapter emits deltas" pin). `context_pct` rides as a CURRENT gauge
//! (pass-through, projector-MAX'd), never a delta. Emission is a non-mutation `TelemetrySampled`
//! OBSERVATION event through an injected sink (the 3.4 `TerminalEventSink` precedent, LESSON §23/§24).
//! NON-safety — no §15/INV-SEC-1 mechanism touched (the write-actor append already gates it).

use std::sync::{Arc, Mutex};

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::MetricQuality;
use nexusopsd::harness::claude::telemetry::{
    parse_usage_reading, telemetry_sample, TelemetryEventSink, UsageReading,
};
use nexusopsd::harness::claude::ClaudeAdapter;
use nexusopsd::harness::HarnessAdapter;

// ---- test doubles -------------------------------------------------------------------------------

/// a collecting telemetry sink (the `TerminalEventSink` `FakeEventSink` precedent): captures every
/// emitted `TelemetrySampled` so a test can assert the count + payload.
#[derive(Clone, Default)]
struct CollectingSink {
    events: Arc<Mutex<Vec<TelemetrySampled>>>,
}
impl TelemetryEventSink for CollectingSink {
    fn emit_telemetry(&self, event: TelemetrySampled) {
        self.events.lock().unwrap().push(event);
    }
}

fn adapter() -> ClaudeAdapter {
    // Option A (P4.0b-2): the adapter no longer spawns/holds a PTY — `new(cwd, session_id)`.
    ClaudeAdapter::new(
        std::path::PathBuf::from("/Users/x/proj"),
        "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
    )
}

/// a cumulative usage reading (a snapshot — `push_usage` deltas it against the prior).
fn reading(tokens_in: u64, tokens_out: u64, context_pct: Option<f32>, cost: f64) -> UsageReading {
    UsageReading {
        tokens_in,
        tokens_out,
        context_pct,
        cost,
        model: Some("claude-opus-4-8".to_string()),
    }
}

// ---- RED #1 — heartbeat emits DELTAS, not cumulative (§9.1; the proj_usage_ledger SUMs) ----------

#[test]
fn test_telemetry_heartbeat_emits_deltas_not_cumulative() {
    // spec(§9.1) — the adapter emits per-heartbeat DELTAS (shared/src/harness.rs:48); cumulative
    // would over-count the SUMming proj_usage_ledger.
    let mut a = adapter();
    assert!(
        a.telemetry_heartbeat().is_none(),
        "no sample before any reading"
    );
    a.push_usage(reading(100, 20, Some(30.0), 0.01));
    a.push_usage(reading(250, 60, Some(55.0), 0.03));
    let s = a.telemetry_heartbeat().expect("a sample after readings");
    assert_eq!(
        (s.tokens_in, s.tokens_out),
        (150, 40),
        "DELTA (250-100 / 60-20), NOT the cumulative 250/60"
    );
    assert!(
        (s.cost_estimate - 0.02).abs() < 1e-9,
        "cost DELTA 0.03-0.01=0.02, not the cumulative 0.03"
    );
}

// ---- RED #2 — the first reading is a full delta from zero; None before any (§9.1/§11.4) ----------

#[test]
fn test_first_reading_is_full_delta_from_zero() {
    // spec(§9.1) — no prior → delta from 0 (the cumulative as-is); no double-count on session start,
    // no fabricated pre-reading sample (§11.4 honesty).
    let mut a = adapter();
    assert!(
        a.telemetry_heartbeat().is_none(),
        "None before any push_usage (never a fabricated reading)"
    );
    a.push_usage(reading(100, 20, Some(30.0), 0.01));
    let s = a
        .telemetry_heartbeat()
        .expect("a sample after the first reading");
    assert_eq!(
        (s.tokens_in, s.tokens_out),
        (100, 20),
        "first delta == the first cumulative (from zero)"
    );
    assert!(
        (s.cost_estimate - 0.01).abs() < 1e-9,
        "first cost delta == 0.01"
    );
}

// ---- RED #3 — context_pct is a CURRENT gauge (pass-through), never a delta (§9.1) ----------------

#[test]
fn test_context_pct_is_gauge_not_delta() {
    // spec(§9.1) — the projector takes context_pct_max (MAX gauge); a delta would mis-bucket. The
    // load-bearing tokens-delta-vs-context-gauge distinction.
    let sink = CollectingSink::default();
    let mut a = adapter().with_telemetry_sink(Box::new(sink.clone()));
    a.push_usage(reading(100, 20, Some(30.0), 0.01));
    a.push_usage(reading(250, 60, Some(55.0), 0.03));
    let ev = sink.events.lock().unwrap();
    assert_eq!(
        ev[0].sample.context_pct,
        Some(30.0),
        "current gauge, pass-through"
    );
    assert_eq!(
        ev[1].sample.context_pct,
        Some(55.0),
        "current gauge 55, NOT a 25 delta"
    );
    // the CONTRAST that makes the distinction load-bearing: while context_pct is a gauge
    // (pass-through above), tokens ARE a delta — the SECOND EMITTED event carries the incremental
    // 150/40, not the cumulative 250/60 (pins delta-via-sink, not just the stored heartbeat sample).
    assert_eq!(
        (ev[1].sample.tokens_in, ev[1].sample.tokens_out),
        (150, 40),
        "second emitted event carries the incremental token delta, not cumulative"
    );
}

// ---- RED #4 — one push_usage emits exactly one TelemetrySampled via the sink (§7.1; LESSON 23) ---

#[test]
fn test_push_usage_emits_one_telemetry_sampled_via_sink() {
    // spec(§7.1) — the 3.4 TerminalEventSink emission-seam precedent (LESSON §23/§24); model +
    // execution_profile_id are the payload dims proj_usage_ledger buckets by (usage.rs:44-46).
    let sink = CollectingSink::default();
    let mut a = adapter().with_telemetry_sink(Box::new(sink.clone()));
    a.push_usage(reading(250, 60, Some(55.0), 0.03));
    let ev = sink.events.lock().unwrap();
    assert_eq!(ev.len(), 1, "exactly one push_usage → one emit");
    assert_eq!(
        ev[0].sample.tokens_in, 250,
        "the delta sample (first, from zero)"
    );
    assert_eq!(
        ev[0].model.as_deref(),
        Some("claude-opus-4-8"),
        "model dim carried in the payload"
    );
    assert_eq!(
        ev[0].execution_profile_id, None,
        "None until the P4 drive loop sets it (safety #8 — resolved at launch)"
    );
}

// ---- RED #5 — cost = reported-cost delta (no pricing table); metric_quality honest (§9.1/§11.4) --

#[test]
fn test_cost_and_metric_quality_derivation() {
    // spec(§9.1) — cost_estimate = the delta of Claude's REPORTED cumulative cost ("estimate"
    // honesty, no local pricing table); quality degrades honestly, never a faked 0% (§11.4).
    let prev = reading(100, 20, Some(30.0), 0.01);
    let cur = reading(250, 60, Some(55.0), 0.03);
    let s = telemetry_sample(Some(&prev), &cur);
    assert!(
        (s.cost_estimate - 0.02).abs() < 1e-9,
        "cost is a reported-cost delta, no pricing table"
    );
    assert_eq!(
        s.metric_quality,
        MetricQuality::Exact,
        "context present → Exact"
    );

    // a reading whose context_pct degrades to unknown → Estimated, context None (never a faked 0%).
    let cur_noctx = UsageReading {
        context_pct: None,
        ..cur.clone()
    };
    let s2 = telemetry_sample(Some(&prev), &cur_noctx);
    assert_eq!(
        s2.metric_quality,
        MetricQuality::Estimated,
        "context absent → Estimated (honest degradation, never faked 0%)"
    );
    assert_eq!(s2.context_pct, None, "absent context stays None, not 0.0");
}

// ---- RED #6 — the parser binds the documented usage source; malformed fails closed (§7.2/§15) ----

#[test]
fn test_usage_parser_binds_fixture_rejects_malformed() {
    // spec(§7.2) — harness-derived SoT; the structured-signal seam (the derive_status precedent).
    // Documented field set (validated against a real Claude session at the impl-fixture/P4 boundary,
    // Q5); a malformed/absent-usage input fails closed (None), never a fabricated reading (§15).
    let json = r#"{"usage":{"input_tokens":250,"output_tokens":60},"total_cost_usd":0.03,"model":"claude-opus-4-8","context_pct":55.0}"#;
    let r = parse_usage_reading(json).expect("binds the documented fields");
    assert_eq!((r.tokens_in, r.tokens_out), (250, 60), "usage token fields");
    assert!((r.cost - 0.03).abs() < 1e-9, "total_cost_usd → cost");
    assert_eq!(r.context_pct, Some(55.0), "context gauge");
    assert_eq!(r.model.as_deref(), Some("claude-opus-4-8"), "model dim");

    // extra/unknown fields are tolerated (an external transcript carries many) — only the required
    // usage shape is load-bearing.
    assert!(
        parse_usage_reading(
            r#"{"usage":{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":9},"total_cost_usd":0.0,"type":"result"}"#
        )
        .is_some(),
        "unknown fields tolerated (external source); model/context default absent"
    );

    // fail-closed: missing usage, non-JSON, and a bad token type all → None (never fabricated).
    assert!(
        parse_usage_reading(r#"{"total_cost_usd":0.03}"#).is_none(),
        "missing usage → None (fail-closed)"
    );
    assert!(
        parse_usage_reading("not json at all").is_none(),
        "non-JSON → None"
    );
    assert!(
        parse_usage_reading(
            r#"{"usage":{"input_tokens":"oops","output_tokens":1},"total_cost_usd":0.0}"#
        )
        .is_none(),
        "bad token type → None (fail-closed)"
    );
}

//! P3.2 part 2 (telemetry — brief 044) — the Claude adapter's telemetry emission path.
//!
//! Per-heartbeat token/cost **DELTAS** (current cumulative − last emitted), NOT cumulative
//! snapshots, so the SUMming `proj_usage_ledger` projector does not over-count (§9.1 / the 3.1
//! `shared/src/harness.rs:48` "adapter emits deltas" pin). `context_pct` rides as a CURRENT gauge
//! (pass-through, projector-MAX'd), never a delta. Emission is a non-mutation `TelemetrySampled`
//! OBSERVATION event through an injected sink (the 3.4 `TerminalEventSink` precedent, LESSON §23/§24).
//! NON-safety — no §15/INV-SEC-1 mechanism touched (the write-actor append already gates it).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::MetricQuality;
use nexusopsd::harness::claude::telemetry::{
    parse_usage_reading, telemetry_sample, TelemetryEventSink, UsageReading, UsageSource,
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

// ===== 4.0c (brief 056) — the live telemetry pump + the non-monotonic-cumulative clamp ============

/// a scripted [`UsageSource`] (4.0c) — the pump's per-tick cumulative-reading source. Yields its
/// queued readings one per `poll_usage`, then `None` (exhausted). The PRODUCTION source (the live
/// hook-receiver/statusLine feed) is the P4 deferred seam; tests drive this scripted double.
#[derive(Default)]
struct ScriptedSource {
    readings: VecDeque<UsageReading>,
}
impl ScriptedSource {
    fn new(readings: Vec<UsageReading>) -> Self {
        Self {
            readings: readings.into(),
        }
    }
}
impl UsageSource for ScriptedSource {
    fn poll_usage(&mut self) -> Option<UsageReading> {
        self.readings.pop_front()
    }
}

// ---- RED #7 — a transient DECREASING cumulative must not OVER-count (the §18 baseline clamp) ------

#[test]
fn test_non_monotonic_cumulative_does_not_overcount() {
    // spec(§9.1/§18) — the existing per-delta floor (`saturating_sub` / `cost.max(0)`) only stops a
    // NEGATIVE delta; a transient DOWN-glitch in the cumulative would still inflate the NEXT delta
    // (the climb back up is double-counted — the proj_usage_ledger SUMs). 4.0c clamps the STORED
    // baseline monotonic (max per field) so the SUM of emitted deltas == the TRUE final cumulative.
    let sink = CollectingSink::default();
    let mut a = adapter().with_telemetry_sink(Box::new(sink.clone()));
    a.push_usage(reading(100, 20, Some(30.0), 0.10)); // cumulative 100/20/0.10
    a.push_usage(reading(40, 8, Some(20.0), 0.02)); // glitch DOWN — delta 0, baseline HELD at 100/20/0.10
    a.push_usage(reading(150, 30, Some(55.0), 0.11)); // back up — delta vs the held baseline, not the dip
    let ev = sink.events.lock().unwrap();
    let sum_in: u64 = ev.iter().map(|e| e.sample.tokens_in).sum();
    let sum_out: u64 = ev.iter().map(|e| e.sample.tokens_out).sum();
    let sum_cost: f64 = ev.iter().map(|e| e.sample.cost_estimate).sum();
    assert_eq!(
        sum_in, 150,
        "SUM of token_in deltas == final cumulative 150 (un-clamped would over-count 100+0+110=210)"
    );
    assert_eq!(
        sum_out, 30,
        "SUM of token_out deltas == final cumulative 30 (un-clamped 20+0+22=42)"
    );
    assert!(
        (sum_cost - 0.11).abs() < 1e-9,
        "SUM of cost deltas == final cumulative 0.11 (un-clamped 0.10+0+0.09=0.19)"
    );
    // the glitch-down reading itself emits a non-negative (zero) delta (AC3 as written, never negative).
    assert_eq!(
        ev[1].sample.tokens_in, 0,
        "the down-glitch reading emits a 0 token delta, never negative"
    );
    assert!(
        ev[1].sample.cost_estimate >= 0.0,
        "the down-glitch reading emits a non-negative cost delta"
    );
}

// ---- RED #8 — the pump tick (poll_telemetry) drains ONE source reading → ONE emit (§9.1 AC2) ------

#[test]
fn test_pump_poll_drains_source_emits_per_reading() {
    // spec(§9.1) — the pump's per-tick behavior (the session-actor telemetry tick calls
    // poll_telemetry()): drain ONE cumulative reading from the injected UsageSource + emit its DELTA
    // via the bound sink. N source readings → N poll_telemetry() calls → N emits; an exhausted source
    // → no emit (never a phantom re-emit / over-count). The live source = the P4 hook/statusLine feed.
    let sink = CollectingSink::default();
    let src = ScriptedSource::new(vec![
        reading(100, 20, Some(30.0), 0.01),
        reading(250, 60, Some(55.0), 0.03),
    ]);
    let mut a = adapter()
        .with_telemetry_sink(Box::new(sink.clone()))
        .with_usage_source(Box::new(src));
    a.poll_telemetry(); // reading 1 → emit (delta from zero)
    a.poll_telemetry(); // reading 2 → emit (incremental delta)
    a.poll_telemetry(); // source exhausted → NO emit
    let ev = sink.events.lock().unwrap();
    assert_eq!(
        ev.len(),
        2,
        "two source readings → two emits; the exhausted poll emits nothing (no over-count)"
    );
    assert_eq!(
        (ev[0].sample.tokens_in, ev[0].sample.tokens_out),
        (100, 20),
        "first emit = the delta from zero"
    );
    assert_eq!(
        (ev[1].sample.tokens_in, ev[1].sample.tokens_out),
        (150, 40),
        "second emit = the incremental DELTA 150/40 (not the cumulative 250/60)"
    );
}

// ---- RED #9 — the pump is a no-op when source-less or sink-less (044-safe; 4.0c prod has no source) -

#[test]
fn test_poll_telemetry_source_less_or_sink_less_is_noop() {
    // spec(§9.1) — 044-safe degradation. NO source → poll_telemetry is a no-op (this IS production
    // 4.0c: the sink is bound but the live source = P4, so the pump ticks emit nothing yet — never a
    // panic). A source but NO sink → still drains+stores (telemetry_heartbeat stays correct) but emits
    // nothing (the 044 sink-less invariant), never a panic.
    let mut a = adapter(); // no source, no sink
    a.poll_telemetry();
    assert!(
        a.telemetry_heartbeat().is_none(),
        "no source → nothing drained → no sample, no panic"
    );

    let src = ScriptedSource::new(vec![reading(100, 20, Some(30.0), 0.01)]);
    let mut b = adapter().with_usage_source(Box::new(src)); // source, no sink
    b.poll_telemetry();
    let s = b
        .telemetry_heartbeat()
        .expect("the drained reading advances the heartbeat even sink-less (044-safe)");
    assert_eq!(
        (s.tokens_in, s.tokens_out),
        (100, 20),
        "delta stored sink-less; the bound-sink path would also emit it"
    );
}

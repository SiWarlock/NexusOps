//! P4.0c (brief 056) — the live telemetry pump + the PRODUCTION sink-bind (integration).
//!
//! The 044 emit path (`ClaudeAdapter::push_usage` → `TelemetryEventSink`) was built + unit-tested but
//! UNREACHABLE in production (the launcher built the adapter with no sink; nothing drove it). 4.0c
//! binds the production `WriteHandle`-backed sink + the session-actor pump tick. **NON-safety** — a
//! `TelemetrySampled` is a non-mutation OBSERVATION event written via the single write-actor, NOT the
//! Gateway (LESSON §23/§27); INV-SEC-1 is untouched. The sink emit is fire-and-forget (soft-degrade,
//! LESSON §9/§30 — a dropped sample is a stale meter, never a safety fault; it never back-pressures
//! the writer). These tests drive the REAL write-actor + read back the persisted row + the ledger.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::{MetricQuality, TelemetrySample};
use nexusops_shared::ids::{ProjectId, SessionId};

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, EventStore, PrefixRedactor};
use nexusopsd::harness::telemetry::{
    telemetry_sample, TelemetryEventSink, TelemetrySinkFactory, UsageReading,
};
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::{WriteActor, WriteActorTelemetrySink, WriteActorTelemetrySinkFactory};
use nexusopsd::session::{PtyLauncher, SessionLauncher};
use nexusopsd::terminal::{EnvMutation, ExitStatus, FakePty, Pty, PtyRead, PtySpawner};

// ---- integration scaffold (the runtime.rs pattern) ----------------------------------------------

fn temp_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open_store(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .unwrap()
}

/// a no-side-effect stub Gateway for the write-actor (Append never touches the gateway).
fn stub_gateway() -> nexusopsd::gateway::Gateway {
    nexusopsd::gateway::Gateway::new(
        Box::new(nexusopsd::gateway::policy::StubPolicy),
        Box::new(nexusopsd::gateway::executor::StubExecutor),
    )
}

// ---- RED #10 — the production sink appends a TelemetrySampled OBSERVATION via the write-actor ------

#[tokio::test]
async fn test_production_sink_appends_telemetry_sampled_observation() {
    // spec(§7.1/§9.1, AC1/AC5) — the production WriteActorTelemetrySink appends a TelemetrySampled
    // OBSERVATION event via the single write-actor (NOT the Gateway — LESSON §23); occurred_at is the
    // injected daemon-Clock UTC-Z (gates bucket_day, LESSON §27). The append is fire-and-forget.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T12:00:00Z")),
        stub_gateway(),
    );
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let sink = WriteActorTelemetrySink::new(
        actor.handle(),
        sid.clone(),
        Some(pid.clone()),
        Arc::new(FixedClock::new("2026-06-08T12:00:00Z")),
    );
    sink.emit_telemetry(TelemetrySampled {
        sample: TelemetrySample {
            tokens_in: 250,
            tokens_out: 60,
            context_pct: Some(55.0),
            cost_estimate: 0.03,
            metric_quality: MetricQuality::Exact,
        },
        model: Some("claude-opus-4-8".to_string()),
        execution_profile_id: Some("prof_x".to_string()),
    });
    // fire-and-forget: the Append is enqueued ahead of Shutdown (FIFO) → it commits before the join.
    actor.shutdown().await;

    let conn = open_read_only(&path).unwrap();
    let (etype, occurred, evt_session): (String, String, Option<String>) = conn
        .query_row(
            "SELECT event_type, occurred_at, session_id FROM events ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        etype, "TelemetrySampled",
        "the observation event landed via the write-actor"
    );
    assert!(
        occurred.ends_with('Z'),
        "occurred_at is daemon-Clock UTC-Z (gates bucket_day, LESSON §27): {occurred}"
    );
    assert_eq!(
        evt_session.as_deref(),
        Some(sid.as_str()),
        "carries the session identity"
    );
    // it is an OBSERVATION, never a Gateway mutation — no Action* event in the log (INV-SEC-1 untouched).
    let action_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_type LIKE 'Action%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        action_events, 0,
        "telemetry is an observation event, never a Gateway action"
    );
}

// ---- RED #11 — the production sink feeds proj_usage_ledger; deltas SUM to the cumulative (§18) -----

#[tokio::test]
async fn test_production_sink_ledger_sums_deltas_utc() {
    // spec(§18, AC6) + LESSON §27 — drive the PRODUCTION sink with the two adapter DELTA samples
    // (cumulative 100/20/0.01 → 250/60/0.03); the REAL projector SUMs them to the cumulative
    // (250/60/0.03), MAXes the context gauge (55), and buckets by the UTC-Z date. Differential
    // negative controls: a cumulative-emit would SUM 350/80; a deltaed gauge would MAX 30.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T12:00:00Z")),
        stub_gateway(),
    );
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let sink = WriteActorTelemetrySink::new(
        actor.handle(),
        sid.clone(),
        Some(pid.clone()),
        Arc::new(FixedClock::new("2026-06-08T12:00:00Z")),
    );
    let ur = |tokens_in, tokens_out, context_pct, cost| UsageReading {
        tokens_in,
        tokens_out,
        context_pct,
        cost,
        model: Some("claude-opus-4-8".to_string()),
    };
    let r1 = ur(100, 20, Some(30.0), 0.01);
    let r2 = ur(250, 60, Some(55.0), 0.03);
    for sample in [
        telemetry_sample(None, &r1),
        telemetry_sample(Some(&r1), &r2),
    ] {
        sink.emit_telemetry(TelemetrySampled {
            sample,
            model: Some("claude-opus-4-8".to_string()),
            execution_profile_id: Some("prof_x".to_string()),
        });
    }
    actor.shutdown().await;

    let conn = open_read_only(&path).unwrap();
    let (ti, to, ctx, cost, day): (i64, i64, Option<f64>, f64, String) = conn
        .query_row(
            "SELECT tokens_in, tokens_out, context_pct_max, cost_estimate, bucket_day \
             FROM proj_usage_ledger WHERE session_id=?1",
            [sid.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(
        (ti, to),
        (250, 60),
        "SUM-of-deltas == the cumulative (a cumulative-emit would double-count to 350/80)"
    );
    assert!((cost - 0.03).abs() < 1e-9, "cost SUM-of-deltas == 0.03");
    assert_eq!(
        ctx,
        Some(55.0),
        "context_pct_max = the MAX gauge 55 (a deltaed gauge would MAX 30)"
    );
    assert_eq!(
        day, "2026-06-08",
        "bucket_day == the UTC-Z date of occurred_at"
    );
}

// ---- RED #12 — the launcher injects a per-session telemetry sink via the opaque factory (AC1) -----

/// a recording [`TelemetrySinkFactory`] — records the session id each `make_sink` is called with and
/// returns a discarding sink. Proves the launcher's per-session injection without a write-actor.
#[derive(Clone, Default)]
struct RecordingFactory {
    calls: Arc<Mutex<Vec<String>>>,
}
impl TelemetrySinkFactory for RecordingFactory {
    fn make_sink(
        &self,
        session_id: &SessionId,
        _project_id: Option<&ProjectId>,
    ) -> Box<dyn TelemetryEventSink> {
        self.calls
            .lock()
            .unwrap()
            .push(session_id.as_str().to_string());
        Box::new(DiscardSink)
    }
}
struct DiscardSink;
impl TelemetryEventSink for DiscardSink {
    fn emit_telemetry(&self, _event: TelemetrySampled) {}
}

/// a succeeding spawner (the launcher's ONE spawn) returning a scripted FakePty.
struct OkSpawner;
impl PtySpawner for OkSpawner {
    fn spawn(
        &self,
        _program: &str,
        _args: &[String],
        _cwd: &Path,
        _rows: u16,
        _cols: u16,
        _env: &[EnvMutation],
    ) -> std::io::Result<Box<dyn Pty>> {
        Ok(Box::new(FakePty::new(
            vec![PtyRead::Eof],
            ExitStatus {
                exit_code: Some(0),
                signal: None,
            },
        )))
    }
}

#[test]
fn test_launcher_injects_telemetry_sink_per_session() {
    // spec(§9.1, AC1) — PtyLauncher injects a per-session telemetry sink via the opaque
    // TelemetrySinkFactory (built in main.rs over the WriteHandle → session/ never imports WriteHandle,
    // cat-1 clean). The factory is invoked once per launched session, with the minted session id.
    let factory = RecordingFactory::default();
    let launcher = PtyLauncher::new(
        Box::new(OkSpawner),
        PathBuf::from("/tmp/proj"),
        "/usr/local/bin/nexusops-hook",
    )
    .with_telemetry_sink_factory(Box::new(factory.clone()));
    let launched = launcher.launch_session().expect("launch");
    let calls = factory.calls.lock().unwrap();
    assert_eq!(
        calls.len(),
        1,
        "the factory is invoked once per launched session"
    );
    assert_eq!(
        calls[0],
        launched.session_id.as_str(),
        "with the minted session id (per-session sink)"
    );
}

/// a production-factory smoke: the WriteActorTelemetrySinkFactory makes a real sink (closes over the
/// WriteHandle) — the type main.rs constructs. Proves the factory→sink construction compiles + runs.
#[tokio::test]
async fn test_production_factory_makes_a_write_actor_sink() {
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T12:00:00Z")),
        stub_gateway(),
    );
    let factory = WriteActorTelemetrySinkFactory::new(
        actor.handle(),
        Arc::new(FixedClock::new("2026-06-08T12:00:00Z")),
    );
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let sink = factory.make_sink(&sid, Some(&pid));
    sink.emit_telemetry(TelemetrySampled {
        sample: telemetry_sample(
            None,
            &UsageReading {
                tokens_in: 10,
                tokens_out: 2,
                context_pct: Some(5.0),
                cost: 0.001,
                model: None,
            },
        ),
        model: None,
        execution_profile_id: None,
    });
    actor.shutdown().await;
    let conn = open_read_only(&path).unwrap();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_type='TelemetrySampled' AND session_id=?1",
            [sid.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        n, 1,
        "the production factory's sink appends via the write-actor"
    );
}

// ---- RED #13 — the DEFERRED factory (the production construction): pre-bind drops, post-bind lands --

#[tokio::test]
async fn test_deferred_factory_drops_before_bind_appends_after() {
    // spec(§9.1, AC1) — main.rs builds the factory DEFERRED (the launcher→write-actor bootstrap cycle):
    // a sink minted BEFORE the handle is bound snapshots a disconnected handle → drops its samples
    // (soft-degrade, LESSON §30, never a panic); once `bind` supplies the handle a freshly-minted sink
    // appends via the write-actor. (Production always binds before any make_sink — this pins both sides.)
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T12:00:00Z")),
        stub_gateway(),
    );
    let (factory, slot) =
        WriteActorTelemetrySinkFactory::deferred(Arc::new(FixedClock::new("2026-06-08T12:00:00Z")));
    let sample = || TelemetrySampled {
        sample: telemetry_sample(
            None,
            &UsageReading {
                tokens_in: 10,
                tokens_out: 2,
                context_pct: Some(5.0),
                cost: 0.001,
                model: None,
            },
        ),
        model: None,
        execution_profile_id: None,
    };

    // BEFORE bind: a minted sink has a disconnected handle → emit is dropped (soft-degrade, no panic).
    let before = SessionId::new();
    factory.make_sink(&before, None).emit_telemetry(sample());

    // bind the handle (the main.rs post-spawn step), then a freshly minted sink appends.
    slot.bind(actor.handle());
    let after = SessionId::new();
    factory.make_sink(&after, None).emit_telemetry(sample());

    actor.shutdown().await;
    let conn = open_read_only(&path).unwrap();
    let count = |sid: &str| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM events WHERE event_type='TelemetrySampled' AND session_id=?1",
            [sid],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        count(before.as_str()),
        0,
        "a sink minted before bind drops its sample (disconnected → soft-degrade, never panics)"
    );
    assert_eq!(
        count(after.as_str()),
        1,
        "a sink minted after bind appends via the write-actor"
    );
}

// ---- 3.3d RED #7 — the Codex emission path SUMs deltas in proj_usage_ledger, UTC-bucketed ---------

#[tokio::test]
async fn test_codex_usage_ledger_sums_deltas_utc() {
    // spec(§18, brief-074 AC6) + LESSON §27 — drive the CodexAdapter emission path (push_usage →
    // the PRODUCTION WriteActorTelemetrySink) with two CUMULATIVE Codex readings; the REAL projector
    // SUMs the per-heartbeat DELTAS to the cumulative, MAXes the context gauge, buckets by the UTC-Z
    // date. The Codex cost is LOCALLY derived (codex_cost_estimate) — no upstream cost field.
    use nexusopsd::harness::codex::telemetry::codex_cost_estimate;
    use nexusopsd::harness::codex::CodexAdapter;

    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-17T09:00:00Z")),
        stub_gateway(),
    );
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let model = "gpt-5.1-codex-max";
    let sink = WriteActorTelemetrySink::new(
        actor.handle(),
        sid.clone(),
        Some(pid.clone()),
        Arc::new(FixedClock::new("2026-06-17T09:00:00Z")),
    );
    let cumulative = |tokens_in: u64, tokens_out: u64, ctx: Option<f32>| UsageReading {
        tokens_in,
        tokens_out,
        context_pct: ctx,
        cost: codex_cost_estimate(tokens_in, tokens_out, Some(model)),
        model: Some(model.to_string()),
    };
    let mut adapter = CodexAdapter::new(
        std::path::PathBuf::from("/tmp/r.jsonl"),
        sid.as_str().to_string(),
    )
    .with_telemetry_sink(Box::new(sink));
    adapter.push_usage(cumulative(100, 20, Some(30.0)));
    adapter.push_usage(cumulative(250, 60, Some(55.0)));
    actor.shutdown().await;

    let conn = open_read_only(&path).unwrap();
    let (ti, to, ctx, cost, day): (i64, i64, Option<f64>, f64, String) = conn
        .query_row(
            "SELECT tokens_in, tokens_out, context_pct_max, cost_estimate, bucket_day \
             FROM proj_usage_ledger WHERE session_id=?1",
            [sid.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(
        (ti, to),
        (250, 60),
        "SUM-of-deltas == the cumulative (a cumulative-emit would double-count)"
    );
    let expected_cost = codex_cost_estimate(250, 60, Some(model));
    assert!(
        (cost - expected_cost).abs() < 1e-9,
        "cost SUM-of-deltas == the cumulative derived cost"
    );
    assert_eq!(
        ctx,
        Some(55.0),
        "context_pct_max = the MAX gauge 55 (a deltaed gauge would MAX 30)"
    );
    assert_eq!(
        day, "2026-06-17",
        "bucket_day == the UTC-Z date of occurred_at"
    );
}

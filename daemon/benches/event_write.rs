//! P2.5 — the §18 event-write / reader-under-load benchmark (the `ARCHITECTURE.md §18` CI
//! regression guards, asserted against the AS-BUILT production submit path).
//!
//! **NON-TDD benchmark slice** (the Phase-2 `§18 (benchmark)` waiver) — the benchmark IS the
//! coverage; there is no RED→GREEN test. It **NEVER runs inside `cargo test --workspace`** (timing
//! assertions flake in the per-slice loop). It is a `[[bench]] harness = false` target, so it is
//! invisible to the default suite and runs only at its own cadence:
//!
//! ```text
//! cargo bench --bench event_write        # the /phase-exit perf-budgets runner + 2.6 nightly CI
//! ```
//!
//! ## What it measures — the AS-BUILT production submit path
//! N=20 concurrent submitters drive [`WriteHandle::submit_action_blocking`] (the exact blocking API
//! the synchronous `serve_connection` uses) against ONE real write-actor owning an on-disk WAL
//! [`EventStore`] — with the production redactor-v3 ([`PrefixRedactor`] = engine `prefix-entropy-v3`,
//! the same struct `main.rs` wires), [`CatalogPolicy`], the [`CatalogExecutor`] framework, in-band
//! projections, and the transactional outbox all in the loop. Each submit is a **risk-0
//! `project.rescan` auto-execute** (no human approval — the true single-shot intent→committed path
//! the §18 write-latency SLO governs). `project.rescan` is `IdempotencyFormula::None`, so every
//! submit (a fresh `ActionRequestId`) is a real full write, never a dedup-replay short-circuit.
//!
//! ## The re-baselined §18 guards (USER-RULED Option A, 2026-06-12) — READ BEFORE EDITING ASSERTIONS
//! The brief-038 guards (30 ms p95 / 75 ms p99 / 1,500 commits/s) came from the 0.4 spike's
//! **1-transaction RAW-rusqlite model** (5.35 ms / ~5 k/s). That model **does not exist in
//! production**: every commit carries the §15 redactor + in-band projections + the outbox, and a
//! risk-0 submit is **3 such commits**, so each commit is ~3–4× heavier. Measuring the as-built path
//! under the spike's own closed-loop-zero-think methodology gave 57–135 ms p95 / 744–1,366 commits/s
//! — missing the raw-append guards while still meeting the real user-facing SLO (p95 < 100 ms). The
//! user re-baselined to the realistic operating point:
//! - **Latency: gated `< 25 ms` p95 under REALISTIC NON-SATURATING load** (Pass A) — N=20 sources
//!   each pausing between submits ("an agent does seconds of LLM work between actions"), so the
//!   single writer is not queue-saturated. p95 is the full submit (intent→committed-and-executed).
//!   **p99 is REPORTED, not gated** (the standalone p99 guard was retired in the ruling).
//! - **Throughput: gated `≥ 1,000 events/s` at SUSTAINED CAPACITY** (Pass B) — a *saturating* run
//!   where the writer is the binding constraint, at a representative steady-state size. events/s ==
//!   commits/s here (1 event per commit), the §18 basis unit; submits/s is reported alongside.
//! - **Reader: gated `< 10 ms` p95 under (saturating) write load** (Pass B) — unchanged; the reader
//!   path was never the mis-calibration (measured 0.025 ms).
//! - The **saturating N=20 / 30k-event numbers** (≈135 ms p95 / ~744 events/s) are kept as
//!   **REPORTED stress characterization** (Pass C, printed not gated) so the worst-case ceiling stays
//!   documented; N=100 fresh + @1M are nightly-only (Pass D / 2.6 CI).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
use nexusops_shared::ids::{ActionRequestId, ProjectId};
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::SystemClock;
use nexusopsd::eventstore::{open_read_only, EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::CatalogExecutor;
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::{WriteActor, WriteHandle};

// ---- the re-baselined §18 CI regression guards (ARCHITECTURE.md §18; USER-RULED Option A,
//      2026-06-12). The retired raw-append numbers (30/75 ms, 1,500/s) are gone — see the module
//      header. These consts and the §18 doc must agree (25 ms / 1,000 events/s). ------------------
/// Event-write p95 under REALISTIC non-saturating load (Pass A) — GATED `< 25 ms`.
const WRITE_P95_MAX: Duration = Duration::from_millis(25);
/// Reader p95 under (saturating) write load (Pass B) — GATED `< 10 ms` (unchanged; measured 0.025 ms).
const READER_P95_MAX: Duration = Duration::from_millis(10);
/// Sustained throughput at capacity (Pass B), events==commits/s — GATED floor `≥ 1,000 events/s`
/// (the as-built 3-txn-per-submit pipeline sustains ~1,366/s fresh).
const THROUGHPUT_FLOOR_EVENTS_PER_SEC: f64 = 1_000.0;
/// Documented single-writer ceiling — p95 `< 100 ms` holds through ≥ N=100 (Pass D, REPORTED, not a
/// hard gate; nightly-only). (p99 is likewise REPORTED, not gated — the ruling kept p95 + throughput.)
const CEILING_P95_MAX: Duration = Duration::from_millis(100);

// ---- load model + sizing (USER-RULED Option A: realistic NON-SATURATING for latency; a separate
//      SATURATING binding-constraint pass for throughput; the saturating stress ceiling REPORTED).
//      Env-overridable for smoke runs / regression isolation without recompiling: `BENCH_WRITERS` /
//      `BENCH_SUBMITS` / `BENCH_THINK_MS` (Pass A), `BENCH_READER=0` (disable the reader probe),
//      `BENCH_CEILING=1` (run the nightly N=100 sweep — OFF by default). ---------------------------

/// Pass A — REALISTIC latency: N=20 sources, each pausing `THINK_TIME` between submits so the
/// aggregate offered rate stays under writer capacity (non-saturating; no queue wait). 100 ms is
/// CONSERVATIVE — real agents pause seconds between actions, so this over-estimates contention.
const REALISTIC_WRITERS: usize = 20;
const REALISTIC_SUBMITS_PER_WRITER: usize = 150; // 3,000 submits @ N=20 → a solid p95 sample
const THINK_TIME_MS: usize = 100;

/// Pass B — SUSTAINED CAPACITY: a SATURATING run (writer = binding constraint) at a representative
/// steady-state size (~15k events, the N=1-sustained reference) → the honest max commit rate.
const CAPACITY_WRITERS: usize = 4; // enough to keep the writer's queue non-empty
const CAPACITY_SUBMITS_PER_WRITER: usize = 1_250; // 5,000 submits → ~15,000 events

/// Pass C — SATURATING STRESS CEILING (REPORTED): N=20 zero-think to ~30k events — the conservative
/// worst-case upper bound (≈135 ms p95 / ~744 events/s), kept documented.
const STRESS_WRITERS: usize = 20;
const STRESS_SUBMITS_PER_WRITER: usize = 500; // 10,000 submits → ~30,000 events

/// Pass D — N=100 fresh ceiling (REPORTED; nightly-only via `BENCH_CEILING=1`). @1M-events stays a
/// nightly / 2.6-CI job (too heavy for the /phase-exit runner).
const N_CEILING_WRITERS: usize = 100;
const CEILING_SUBMITS_PER_WRITER: usize = 100;

/// Discarded warm-up submits (prime the WAL + page cache) before each measured window.
const WARMUP_SUBMITS: usize = 500;

/// Read a `usize` env override, falling back to `default` when unset/unparseable.
fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// `false` only when the env var is explicitly set to `0` (the reader probe is ON by default).
fn env_on(key: &str) -> bool {
    std::env::var(key).map(|v| v != "0").unwrap_or(true)
}

/// `true` only when the env var is explicitly `1`/`true` (the heavy nightly sweep is OFF by default).
fn env_explicit_on(key: &str) -> bool {
    std::env::var(key)
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
}

/// A measured pass's computed metrics.
struct PassResult {
    writers: usize,
    submits: usize,
    events_delta: i64,
    storm: Duration,
    write_p50: Duration,
    write_p95: Duration,
    write_p99: Duration,
    write_max: Duration,
    reader_samples: usize,
    reader_p95: Duration,
    events_per_sec: f64,
    submits_per_sec: f64,
    events_per_submit: f64,
}

fn main() {
    println!("=== P2.5 §18 event-write / reader benchmark (AS-BUILT Gateway submit path) ===");
    println!(
        "path under measurement: WriteHandle::submit_action_blocking → single write-actor → \
         CatalogPolicy → risk-0 `project.rescan` auto-execute → CatalogExecutor → \
         EventStore(append + redactor-v3 + in-band projections + outbox)\n"
    );

    let reader_on = env_on("BENCH_READER");
    let think = Duration::from_millis(env_usize("BENCH_THINK_MS", THINK_TIME_MS) as u64);

    // --- Pass A: REALISTIC non-saturating latency (GATES write p95) — reader off (the reader guard
    //     is measured under real write load on the saturating Pass B). -------------------------
    let dir_a = tempfile::tempdir().expect("tempdir A");
    let a = run_pass(
        &dir_a.path().join("nexusops.db"),
        env_usize("BENCH_WRITERS", REALISTIC_WRITERS),
        env_usize("BENCH_SUBMITS", REALISTIC_SUBMITS_PER_WRITER),
        WARMUP_SUBMITS,
        false,
        Some(think),
    );
    report(
        &a,
        &format!(
            "PASS A — REALISTIC non-saturating (N={}, {}ms think-time) — GATES write p95 < 25ms",
            a.writers,
            think.as_millis(),
        ),
    );

    // --- Pass B: SUSTAINED CAPACITY (GATES throughput + reader p95) — saturating, writer = the
    //     binding constraint, at the ~15k-event steady-state reference. ------------------------
    let dir_b = tempfile::tempdir().expect("tempdir B");
    let b = run_pass(
        &dir_b.path().join("nexusops.db"),
        CAPACITY_WRITERS,
        CAPACITY_SUBMITS_PER_WRITER,
        WARMUP_SUBMITS,
        reader_on,
        None,
    );
    report(
        &b,
        "PASS B — SUSTAINED CAPACITY (saturating, binding constraint) — GATES throughput ≥ 1000 ev/s + reader p95 < 10ms",
    );

    // --- Pass C: SATURATING STRESS CEILING (REPORTED, not gated) — the documented worst case. ---
    let dir_c = tempfile::tempdir().expect("tempdir C");
    let c = run_pass(
        &dir_c.path().join("nexusops.db"),
        STRESS_WRITERS,
        STRESS_SUBMITS_PER_WRITER,
        WARMUP_SUBMITS,
        false,
        None,
    );
    report(
        &c,
        "PASS C — SATURATING STRESS CEILING (N=20 zero-think, ~30k events) — REPORTED, not gated",
    );

    // --- Pass D: N=100 fresh ceiling (REPORTED; nightly-only via BENCH_CEILING=1). -------------
    if env_explicit_on("BENCH_CEILING") {
        let dir_d = tempfile::tempdir().expect("tempdir D");
        let d = run_pass(
            &dir_d.path().join("nexusops.db"),
            N_CEILING_WRITERS,
            CEILING_SUBMITS_PER_WRITER,
            WARMUP_SUBMITS,
            false,
            None,
        );
        report(&d, "PASS D — N=100 FRESH CEILING (reported; nightly)");
        println!(
            "  ceiling (soft): write p95 {:.3}ms vs < {:.3}ms → {}",
            ms(d.write_p95),
            ms(CEILING_P95_MAX),
            if d.write_p95 < CEILING_P95_MAX {
                "OK"
            } else {
                "OVER (documented worst case; not a hard gate)"
            }
        );
    } else {
        println!(
            "PASS D — N=100 fresh ceiling + @1M-events: nightly-only (set BENCH_CEILING=1; → 2.6 CI)\n"
        );
    }

    // --- the hard §18 gates (assert LAST, after every number is printed, so a breach still
    //     captures the measured baseline for the /phase-exit perf-row Finding) -----------------
    println!("=== §18 CI guard assertions (USER-RULED Option A re-baseline) ===");
    println!(
        "  (reported, not gated) realistic p99 {:.3}ms · stress ceiling p95 {:.3}ms @ {} events / {:.0} events/s",
        ms(a.write_p99),
        ms(c.write_p95),
        c.events_delta,
        c.events_per_sec,
    );
    check(
        "event-write p95 (realistic non-saturating)",
        a.write_p95,
        WRITE_P95_MAX,
    );
    check(
        "reader p95 (under saturating write load)",
        b.reader_p95,
        READER_P95_MAX,
    );
    check_floor(
        "sustained throughput at capacity (events/s)",
        b.events_per_sec,
        THROUGHPUT_FLOOR_EVENTS_PER_SEC,
    );
    println!("\nALL §18 GUARDS PASSED (Option A re-baseline: 25ms p95 / 1000 ev-s / 10ms reader).");
}

/// Run one measured pass against a fresh `path`: spin up the real write-actor, warm up, then storm
/// `writers` submitters (each doing `submits_per_writer` risk-0 auto-executes) while a concurrent
/// reader probes the most-contended projection (when `reader_on`). `think_time = Some(d)` paces each
/// submitter (a NON-saturating realistic load); `None` saturates (the writer = binding constraint).
/// Returns the metrics.
fn run_pass(
    path: &Path,
    writers: usize,
    submits_per_writer: usize,
    warmup: usize,
    reader_on: bool,
    think_time: Option<Duration>,
) -> PassResult {
    let store = EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(SystemClock),
        Box::new(PrefixRedactor),
    )
    .expect("open the event store");
    let gateway = Gateway::new(Box::new(CatalogPolicy), Box::new(CatalogExecutor::new()));
    let actor = WriteActor::spawn(store, Box::new(SystemClock), gateway);

    // warm-up (discarded) — prime the WAL + page cache so the measured window isn't cold-start-biased.
    {
        let h = actor.handle();
        for _ in 0..warmup {
            submit_one(&h);
        }
    }

    let events_before = event_count(path);

    // the reader-under-load probe: one long-lived read-only WAL connection looping a
    // `get_projection`-style read against `proj_audit_trail` (the AuditProjector folds EVERY event
    // → it is the most write-contended projection during the storm: the honest worst case). It runs
    // until the writers finish (`stop` flipped), so its p95 reflects reads taken under full load.
    let stop = Arc::new(AtomicBool::new(false));
    let reader = reader_on.then(|| {
        let path = path.to_path_buf();
        let project_id = bench_project().as_str().to_string();
        let stop = Arc::clone(&stop);
        thread::spawn(move || reader_probe(&path, &project_id, &stop))
    });

    // the N concurrent submitters, released together off a barrier so they truly contend.
    let barrier = Arc::new(Barrier::new(writers + 1));
    let mut handles = Vec::with_capacity(writers);
    for _ in 0..writers {
        let h = actor.handle();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let mut samples = Vec::with_capacity(submits_per_writer);
            barrier.wait();
            for i in 0..submits_per_writer {
                let t0 = Instant::now();
                submit_one(&h);
                samples.push(t0.elapsed());
                // pace between submits (non-saturating realistic load); skip after the LAST so a
                // trailing think-sleep doesn't inflate the throughput wall-clock.
                if let Some(d) = think_time {
                    if i + 1 < submits_per_writer {
                        thread::sleep(d);
                    }
                }
            }
            samples
        }));
    }

    barrier.wait();
    let storm_start = Instant::now();
    let mut write_samples: Vec<Duration> = Vec::with_capacity(writers * submits_per_writer);
    for h in handles {
        write_samples.extend(h.join().expect("a writer thread panicked"));
    }
    let storm = storm_start.elapsed();

    stop.store(true, Ordering::Relaxed);
    let mut reader_samples = match reader {
        Some(r) => r.join().expect("the reader thread panicked"),
        None => Vec::new(),
    };

    let events_after = event_count(path);
    let events_delta = events_after - events_before;
    drop(actor); // nudge the write-actor to shut down + WAL-checkpoint on drop.

    write_samples.sort_unstable();
    reader_samples.sort_unstable();
    let submits = writers * submits_per_writer;
    let secs = storm.as_secs_f64();

    PassResult {
        writers,
        submits,
        events_delta,
        storm,
        write_p50: percentile(&write_samples, 50.0),
        write_p95: percentile(&write_samples, 95.0),
        write_p99: percentile(&write_samples, 99.0),
        write_max: write_samples.last().copied().unwrap_or(Duration::ZERO),
        reader_samples: reader_samples.len(),
        reader_p95: percentile(&reader_samples, 95.0),
        events_per_sec: events_delta as f64 / secs,
        submits_per_sec: submits as f64 / secs,
        events_per_submit: events_delta as f64 / submits as f64,
    }
}

/// Submit ONE risk-0 `project.rescan` auto-execute through the write-actor, asserting it ran the
/// full pipeline to a terminal ack (a broken path must fail the bench loudly, not skew the numbers).
fn submit_one(h: &WriteHandle) {
    h.submit_action_blocking(rescan_req())
        .expect("write-actor alive")
        .expect("risk-0 project.rescan submit succeeds end-to-end");
}

/// The one project all bench submits target — a stable id (rescans of a single project), so the
/// reader's scoped read hits the `ix_audit_scope (project_id, seq)` index instead of a full scan.
fn bench_project() -> ProjectId {
    static P: OnceLock<ProjectId> = OnceLock::new();
    P.get_or_init(ProjectId::new).clone()
}

/// A fresh risk-0 `project.rescan` request — catalog risk-0, no resource refs required,
/// `IdempotencyFormula::None` (never deduped). The unique `ActionRequestId` makes each a real write.
fn rescan_req() -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: Some(bench_project()),
        action_type: "project.rescan".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_bench".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({}),
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T00:00:00Z").expect("valid RFC3339"),
    }
}

/// Inter-read pace for the reader probe: a real projection reader polls (re-reads on a delta /
/// tick), it does NOT spin a read transaction continuously. A continuous no-sleep loop monopolizes
/// a core AND keeps a WAL read-snapshot almost always open, starving the writer's autocheckpoint →
/// the WAL bloats → EVERY write degrades (a measurement artifact, not a real reader). This modest
/// pace samples thousands of reads/s while leaving the writer + checkpoint unblocked.
const READER_PACE: Duration = Duration::from_micros(500);

/// The reader-under-load loop: a paced, project-scoped `get_projection`-style read against the
/// storm's most write-contended projection (`proj_audit_trail` — the `AuditProjector` folds EVERY
/// event), timed per read, until `stop` is set. The `WHERE project_id = ?1 ORDER BY seq DESC LIMIT
/// 50` uses `ix_audit_scope (project_id, seq)` → a bounded index range scan (a realistic scoped
/// audit-tail read), NOT a full-table scan that worsens as the projection grows.
fn reader_probe(path: &Path, project_id: &str, stop: &AtomicBool) -> Vec<Duration> {
    let conn = open_read_only(path).expect("read-only WAL connection");
    let mut samples = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        let t0 = Instant::now();
        // `query_map` + drain forces the rows to actually materialize (a true read cost).
        let mut stmt = conn
            .prepare_cached(
                "SELECT event_id, seq, headline FROM proj_audit_trail \
                 WHERE project_id = ?1 ORDER BY seq DESC LIMIT 50",
            )
            .expect("prepare the reader query");
        let n: usize = stmt
            .query_map([project_id], |row| row.get::<_, i64>(1))
            .expect("run the reader query")
            .count();
        std::hint::black_box(n);
        samples.push(t0.elapsed());
        thread::sleep(READER_PACE);
    }
    samples
}

/// `SELECT COUNT(*) FROM events` over a fresh read-only connection (the throughput denominator).
fn event_count(path: &Path) -> i64 {
    let conn = open_read_only(path).expect("read-only WAL connection");
    conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .expect("count events")
}

/// Nearest-rank percentile of an ALREADY-SORTED slice (ascending). `p` is in `[0, 100]`.
fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let n = sorted.len();
    let rank = ((p / 100.0) * n as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    sorted[idx]
}

/// Print a pass's full measured baseline — BOTH throughput units + the events-per-submit ratio
/// (so the §18 as-built note records why the floor is asserted on events/s, per the Step-2.5 ask).
fn report(r: &PassResult, title: &str) {
    println!("--- {title} ---");
    println!(
        "  storm: {} submits across N={} writers in {:.3}s ({} events committed)",
        r.submits,
        r.writers,
        r.storm.as_secs_f64(),
        r.events_delta,
    );
    println!(
        "  write latency (full submit, intent→committed-and-executed): \
         p50 {:.3}ms · p95 {:.3}ms · p99 {:.3}ms · max {:.3}ms",
        ms(r.write_p50),
        ms(r.write_p95),
        ms(r.write_p99),
        ms(r.write_max),
    );
    if r.reader_samples > 0 {
        println!(
            "  reader-under-load (proj_audit_trail, {} samples): p95 {:.3}ms",
            r.reader_samples,
            ms(r.reader_p95),
        );
    } else {
        println!("  reader-under-load: (probe off this pass)");
    }
    println!(
        "  throughput: {:.0} events/s (== commits/s) · {:.0} submits/s · {:.2} events/submit",
        r.events_per_sec, r.submits_per_sec, r.events_per_submit,
    );
    println!();
}

/// milliseconds as `f64` for printing a `Duration`.
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000.0
}

/// Assert a measured `value` is under a committed `max` ceiling; print the verdict either way.
fn check(label: &str, value: Duration, max: Duration) {
    let ok = value < max;
    println!(
        "  [{}] {label}: {:.3}ms < {:.3}ms",
        if ok { "PASS" } else { "FAIL" },
        ms(value),
        ms(max),
    );
    assert!(
        ok,
        "§18 guard breach — {label} {:.3}ms is NOT < {:.3}ms (a /phase-exit perf-row Finding)",
        ms(value),
        ms(max),
    );
}

/// Assert a measured `value` is at or above a committed `floor`; print the verdict either way.
fn check_floor(label: &str, value: f64, floor: f64) {
    let ok = value >= floor;
    println!(
        "  [{}] {label}: {value:.0} ≥ {floor:.0}",
        if ok { "PASS" } else { "FAIL" },
    );
    assert!(
        ok,
        "§18 guard breach — {label} {value:.0} is NOT ≥ {floor:.0} (a /phase-exit perf-row Finding)",
    );
}

//! P3.5 — the §18 terminal-attach-latency benchmark + the §6.4 Terminal-Channel throughput /
//! JSON-base64 encode-overhead measurement (asserted against the AS-BUILT 3.4 host).
//!
//! **NON-TDD benchmark slice** (the Phase-3 `§18 (benchmark)` waiver) — the benchmark IS the
//! coverage; there is no RED→GREEN test. It **NEVER runs inside `cargo test --workspace`** (timing
//! assertions flake in the per-slice loop). It is a `[[bench]] harness = false` target, so it is
//! invisible to the default suite and runs only at its own cadence:
//!
//! ```text
//! cargo bench --bench terminal_attach    # the /phase-exit perf-budgets runner + the nightly CI job
//! ```
//!
//! ## What it measures — the AS-BUILT 3.4 Terminal Channel host
//! - **PASS A — attach latency (GATED `< 250 ms` p95).** N fresh iterations each spawn a real
//!   [`PortablePtyHost`] (via the production [`PortablePtySpawner`] seam) running `/bin/echo` — a
//!   minimal always-present exec that writes a deterministic first line — wrap it in the AS-BUILT
//!   [`TerminalSession`], and time **attach intent (just before spawn) → the first non-empty
//!   `flush()`-produced [`TerminalOutputFrame`]** (the daemon's "ready-to-emit" boundary, Q1). This
//!   is the **terminal-host attach FLOOR** (spawn → exec → pipe → first read → flush → base64),
//!   excluding agent cold-start; the cross-track UDS hop + ui xterm.js render (6.3d) are
//!   `not-measured-because` they are downstream + cross-track (folded into the §18 end-to-end budget
//!   when 6.3d lands). p95 over N is the SLO basis (absorbs spawn/scheduler variance).
//! - **PASS B — Terminal-Channel throughput (REPORTED, not gated).** A high-volume [`FakePty`] corpus
//!   driven through `read_step`→`flush`: sustained MB/s + frames/s through the real batch pump. §18
//!   *defers* the throughput budget TO this bench, so there is no committed number to assert yet — the
//!   orchestrator commits the §18 budget from the measured number at the round seal.
//! - **JSON-base64 encode overhead (REPORTED).** base64 (STANDARD — the 3.4 wire codec) size
//!   expansion + encode CPU + the full frame JSON-serialize cost: the data that decides the §6.4
//!   binary-vs-JSON fast-path (justify it or defer it).
//! - **Watermark / backpressure observation (REPORTED; Q6).** A small run with tiny watermarks
//!   confirms the LESSON §24 DoS-bound empirically (the bounded buffer holds ≤ high + one chunk; the
//!   pause/resume cycle is clean) — **measured, not tuned** (tuning is a behavior change with its own
//!   RED tests, out of a measurement slice).
//!
//! Env overrides (smoke runs without recompiling): `BENCH_ATTACH_ITERS` (must be > 0) /
//! `BENCH_ATTACH_WARMUP` (Pass A), `BENCH_TPUT_MIB` (Pass B + the encode pass; MiB = 1024²).

use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use nexusops_shared::events::TerminalProcessExited;
use nexusops_shared::ipc::{TerminalControlKind, TerminalOutputFrame};
use nexusopsd::terminal::{
    ExitStatus, FakePty, PortablePtySpawner, PtyRead, PtySpawner, TerminalEmit, TerminalEventSink,
    TerminalId, TerminalSession,
};

// ---- the §18 gate (ARCHITECTURE.md §18 committed budget) -----------------------------------------
/// Terminal attach latency (attach intent → first rendered frame) — GATED `< 250 ms` p95. The only
/// asserted number; everything else is REPORTED (§18 defers the throughput budget to this bench).
const ATTACH_P95_MAX: Duration = Duration::from_millis(250);

// ---- Pass A sizing (env-overridable) ------------------------------------------------------------
/// N fresh spawn→first-frame iterations the p95 is computed over (spawn cost varies with FS cache +
/// scheduler; p95 over N absorbs the variance, the SLO basis).
const DEFAULT_ATTACH_ITERS: usize = 100;
/// Discarded warm-up spawns (prime the loader/dyld cache) before the measured window.
const DEFAULT_ATTACH_WARMUP: usize = 10;
/// Spin guard: the max `read_step`/`flush` steps to the first frame before declaring a measurement
/// failure. `/bin/echo` produces output in one read, so this is never hit in practice — but a silent
/// hang under a CI/env quirk is worse than a loud failure (orchestrator Step-2.5 refinement).
const ATTACH_MAX_STEPS: usize = 10_000;

// ---- Pass B / encode sizing ---------------------------------------------------------------------
/// Default throughput corpus size (MiB; 1024²-based — see [`mib`]) pushed through `read_step`→`flush`.
const DEFAULT_TPUT_MIB: usize = 64;
/// Per-read chunk size — matches `PortablePtyHost::read`'s 8 KiB buffer (a realistic PTY chunk).
const TPUT_CHUNK: usize = 8192;
/// Flush cadence for the flowing throughput pass: 16 × 8 KiB = 128 KiB per batched frame, kept
/// BELOW the 256 KiB default high watermark so the pump stays flowing (no pause) — measuring the
/// pure read→flush→base64→frame pipeline, not the backpressure path (that is the watermark sub-run).
const TPUT_FLUSH_EVERY: usize = 16;

fn main() {
    println!("=== P3.5 §18 terminal-attach benchmark (AS-BUILT 3.4 Terminal Channel host) ===");
    println!(
        "path under measurement: PortablePtySpawner::spawn(/bin/echo) → TerminalSession::read_step→flush \
         → first base64 TerminalOutputFrame\n"
    );

    let iters = env_usize("BENCH_ATTACH_ITERS", DEFAULT_ATTACH_ITERS);
    let warmup = env_usize("BENCH_ATTACH_WARMUP", DEFAULT_ATTACH_WARMUP);
    let tput_mib = env_usize("BENCH_TPUT_MIB", DEFAULT_TPUT_MIB);
    // a zero-iteration Pass A would make the §18 p95 gate VACUOUSLY pass (percentile of [] is 0 <
    // 250ms); refuse it so an env smoke-run can never silently disarm the only committed guard.
    assert!(
        iters > 0,
        "BENCH_ATTACH_ITERS must be > 0 (the §18 p95 gate needs at least one sample)"
    );

    // --- PASS A: attach latency (GATES p95 < 250 ms) -----------------------------------------------
    let mut samples = attach_latency_pass(iters, warmup);
    samples.sort_unstable();
    let p50 = percentile(&samples, 50.0);
    let p95 = percentile(&samples, 95.0);
    let p99 = percentile(&samples, 99.0);
    let max = samples.last().copied().unwrap_or(Duration::ZERO);
    println!(
        "--- PASS A — ATTACH LATENCY (N={iters} fresh spawns, {warmup} warm-up) — GATES p95 < 250ms ---"
    );
    println!(
        "  attach intent → first rendered frame (daemon-side proxy = first non-empty flush() TerminalOutputFrame):"
    );
    println!(
        "    p50 {:.3}ms · p95 {:.3}ms · p99 {:.3}ms · max {:.3}ms",
        ms(p50),
        ms(p95),
        ms(p99),
        ms(max),
    );
    println!(
        "    floor scope: terminal-host attach (echo spawn→first frame); excludes agent cold-start.\n    \
         not-measured-because: the UDS hop + ui xterm.js render are cross-track (6.3d) — small + downstream, \
         folded into the §18 end-to-end budget when 6.3d lands.\n"
    );

    // --- PASS B: Terminal-Channel throughput (REPORTED) --------------------------------------------
    let t = throughput_pass(tput_mib);
    let secs = t.elapsed.as_secs_f64();
    let mibps = mib(t.raw_bytes) / secs;
    let fps = t.frame_count as f64 / secs;
    println!(
        "--- PASS B — TERMINAL-CHANNEL THROUGHPUT ({tput_mib} MiB corpus via FakePty → read_step→flush) — REPORTED, not gated ---"
    );
    println!(
        "  sustained: {:.1} MiB/s · {:.0} frames/s · {} frames in {:.3}s · mean frame {:.1} KiB raw",
        mibps,
        fps,
        t.frame_count,
        secs,
        (t.raw_bytes as f64 / t.frame_count.max(1) as f64) / 1024.0,
    );
    println!(
        "  reference: the §18 xterm.js render ceiling is ~5–35 MB/s (the consuming bar; SI MB — the \
         ~4.9% MiB/MB gap is immaterial at this headroom); the daemon pipeline's margin above it \
         bounds the §6.4 binary fast-path's marginal value.\n"
    );

    // --- JSON-base64 encode overhead (REPORTED) ----------------------------------------------------
    let e = encode_overhead_pass(tput_mib);
    let b64_mibps = mib(e.raw_bytes) / e.b64_encode.as_secs_f64();
    let json_mibps = mib(e.raw_bytes) / e.json_serialize.as_secs_f64();
    println!(
        "--- JSON-BASE64 ENCODE OVERHEAD (base64 variant: STANDARD, matching the 3.4 wire codec) — REPORTED ---"
    );
    println!(
        "  base64 size:   {} raw → {} b64 bytes = {:.4}× wire expansion",
        e.raw_bytes,
        e.b64_bytes,
        e.b64_bytes as f64 / e.raw_bytes as f64,
    );
    println!(
        "  base64 encode: {:.1} MiB/s ({:.1} ms for {} MiB)",
        b64_mibps,
        e.b64_encode.as_secs_f64() * 1000.0,
        tput_mib,
    );
    println!(
        "  full frame JSON: {} raw → {} json bytes = {:.4}× ; serialize {:.1} MiB/s",
        e.raw_bytes,
        e.json_bytes,
        e.json_bytes as f64 / e.raw_bytes as f64,
        json_mibps,
    );
    println!(
        "  → a §6.4 binary fast-path would save the {:.4}× wire expansion + the encode CPU; this quantifies its value.\n",
        e.json_bytes as f64 / e.raw_bytes as f64,
    );

    // --- watermark / backpressure observation (REPORTED; Q6) ---------------------------------------
    let w = watermark_observation();
    println!(
        "--- WATERMARK / BACKPRESSURE OBSERVATION (LESSON §24 DoS-bound; high={} low={} chunk={}) — REPORTED ---",
        w.high, w.low, w.chunk,
    );
    println!(
        "  max buffered {} bytes (bound = high + one chunk = {}) · {} pauses · {} resumes-on-flush · clean cycle: {}",
        w.max_buffered,
        w.high + w.chunk,
        w.pauses,
        w.resumes,
        w.clean,
    );
    if !w.clean {
        println!("  ⚠ watermark pathology observed — flag at Step 9 (NOT tuned here; a behavior-change slice).");
    }
    println!();

    // --- the hard §18 gate (assert LAST, after every number is printed, so a breach still captures
    //     the measured baseline for the /phase-exit perf-row Finding) ----------------------------
    println!("=== §18 CI guard assertion ===");
    check(
        "terminal attach latency p95 (terminal-host floor; echo spawn→first frame)",
        p95,
        ATTACH_P95_MAX,
    );
    println!(
        "\n§18 ATTACH GUARD PASSED (p95 < 250ms). Throughput + encode-overhead + watermark REPORTED \
         (no committed budget — §18 defers it to this bench)."
    );
}

// ---- PASS A: attach latency ---------------------------------------------------------------------

/// Warm up (discarded), then measure `iters` fresh spawn→first-frame latencies.
fn attach_latency_pass(iters: usize, warmup: usize) -> Vec<Duration> {
    for _ in 0..warmup {
        let _ = measure_one_attach();
    }
    (0..iters).map(|_| measure_one_attach()).collect()
}

/// Spawn a fresh real PTY child (`/bin/echo ready`) via the production [`PortablePtySpawner`] seam,
/// wrap it in the AS-BUILT [`TerminalSession`], and time **attach intent (just before spawn) → the
/// first non-empty `flush()`-produced [`TerminalOutputFrame`]**. The EOF-drain + child reap happen
/// OUTSIDE the timed window, so N iterations don't leak children.
fn measure_one_attach() -> Duration {
    let spawner = PortablePtySpawner;
    let cwd = std::env::temp_dir();

    let t0 = Instant::now();
    let pty = spawner
        .spawn("/bin/echo", &["ready".to_string()], &cwd, 24, 80)
        .expect("spawn /bin/echo in a PTY");
    let mut session =
        TerminalSession::new(TerminalId::from_raw("term_bench"), pty, Box::new(BenchSink));

    let mut elapsed: Option<Duration> = None;
    for _ in 0..ATTACH_MAX_STEPS {
        // a PTY read returns the buffered data CHUNK before it ever returns EOF (they never co-occur
        // in one read), so the first iteration captures the first frame via `flush()` and breaks here
        // with `is_exited()` still false — `finish()`/`child.wait()` is reached only by the post-loop
        // drain below, i.e. OUTSIDE this timed window.
        let mut emits = session.read_step();
        emits.extend(session.flush());
        if emits.iter().any(|e| matches!(e, TerminalEmit::Output(_))) {
            elapsed = Some(t0.elapsed());
            break;
        }
        // EOF before any output → a real measurement failure (echo produced nothing); fail loud
        // rather than spin or silently record a bogus sample.
        if session.is_exited() {
            break;
        }
    }
    let elapsed = elapsed
        .expect("attach: the first terminal output frame never arrived (echo produced no output?)");

    // drain to EOF + reap the child (blocking `child.wait()` via `finish`) OUTSIDE the timed window.
    while !session.is_exited() {
        session.read_step();
        session.flush();
    }
    elapsed
}

// ---- PASS B: Terminal-Channel throughput --------------------------------------------------------

/// Throughput measured through the real `read_step`→`flush` batch pump.
struct ThroughputResult {
    raw_bytes: usize,
    frame_count: usize,
    elapsed: Duration,
}

/// Drive a `total_mib` [`FakePty`] corpus through the AS-BUILT pump at a flowing cadence (flush every
/// [`TPUT_FLUSH_EVERY`] reads, kept below the high watermark) and time the whole pipeline.
fn throughput_pass(total_mib: usize) -> ThroughputResult {
    let n_chunks = (total_mib * 1024 * 1024) / TPUT_CHUNK;
    let raw_bytes = n_chunks * TPUT_CHUNK;
    let mut session = corpus_session("term_tput", n_chunks, TPUT_CHUNK);

    let mut frame_count = 0usize;
    let mut steps = 0usize;
    let t0 = Instant::now();
    loop {
        let mut emits = session.read_step();
        steps += 1;
        if steps.is_multiple_of(TPUT_FLUSH_EVERY) {
            emits.extend(session.flush());
        }
        frame_count += count_outputs(&emits);
        if session.is_exited() {
            // a trailing flush coalesces any remaining buffered bytes into the final frame.
            frame_count += count_outputs(&session.flush());
            break;
        }
    }
    let elapsed = t0.elapsed();
    ThroughputResult {
        raw_bytes,
        frame_count,
        elapsed,
    }
}

// ---- JSON-base64 encode overhead ----------------------------------------------------------------

/// Encode-cost measurement: base64 size/CPU + the full frame JSON-serialize cost.
struct EncodeResult {
    raw_bytes: usize,
    b64_bytes: usize,
    b64_encode: Duration,
    json_bytes: usize,
    json_serialize: Duration,
}

/// Over a `total_mib` corpus, time (a) base64 (STANDARD) encode only, and (b) the full
/// [`TerminalOutputFrame`] JSON-serialize (base64 `data` + the envelope) — the §6.4 binary-vs-JSON
/// decision input.
fn encode_overhead_pass(total_mib: usize) -> EncodeResult {
    let n_chunks = (total_mib * 1024 * 1024) / TPUT_CHUNK;
    let chunk = vec![b'x'; TPUT_CHUNK];
    let raw_bytes = n_chunks * TPUT_CHUNK;

    // (a) base64 encode only.
    let mut b64_bytes = 0usize;
    let t0 = Instant::now();
    for _ in 0..n_chunks {
        let s = STANDARD.encode(&chunk);
        b64_bytes += s.len();
        std::hint::black_box(&s);
    }
    let b64_encode = t0.elapsed();

    // (b) full frame JSON-serialize (base64 `data` + the `terminal_id`/`seq` envelope).
    let mut json_bytes = 0usize;
    let t1 = Instant::now();
    for seq in 0..n_chunks as u64 {
        let frame = TerminalOutputFrame {
            terminal_id: "term_bench".to_string(),
            seq,
            data: STANDARD.encode(&chunk),
        };
        let s = serde_json::to_string(&frame).expect("serialize a TerminalOutputFrame");
        json_bytes += s.len();
        std::hint::black_box(&s);
    }
    let json_serialize = t1.elapsed();

    EncodeResult {
        raw_bytes,
        b64_bytes,
        b64_encode,
        json_bytes,
        json_serialize,
    }
}

// ---- watermark / backpressure observation (Q6) --------------------------------------------------

/// The observed backpressure behavior under deliberately tiny watermarks.
struct WatermarkResult {
    high: usize,
    low: usize,
    chunk: usize,
    max_buffered: usize,
    pauses: usize,
    resumes: usize,
    clean: bool,
}

/// Feed a corpus through a session with tiny watermarks WITHOUT a flush cadence, so the buffer
/// crosses the high watermark and the pump pauses; flush on each pause (drains → resume). Records the
/// max buffered (must stay ≤ high + one chunk — the LESSON §24 bound) + that every pause is matched by
/// a resume. **Observed, not tuned.**
fn watermark_observation() -> WatermarkResult {
    let high = 16 * 1024; // 16 KiB
    let low = 4 * 1024; //    4 KiB
    let chunk = 5000; // NOT a divisor of `high` → exercises the real "+ one chunk" overshoot.
    let n_chunks = 64;
    let reads: Vec<PtyRead> = (0..n_chunks)
        .map(|_| PtyRead::Chunk(vec![b'y'; chunk]))
        .collect();
    let pty = FakePty::new(
        reads,
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        },
    );
    let mut session = TerminalSession::with_watermarks(
        TerminalId::from_raw("term_wm"),
        Box::new(pty),
        Box::new(BenchSink),
        high,
        low,
    );

    let mut max_buffered = 0usize;
    let mut pauses = 0usize;
    let mut resumes = 0usize;
    loop {
        let emits = session.read_step();
        max_buffered = max_buffered.max(session.buffered_bytes());
        if has_control(&emits, TerminalControlKind::Pause) {
            pauses += 1;
            // drain the buffer → the classifier resumes once back at/below `low` (here: empty).
            if has_control(&session.flush(), TerminalControlKind::Resume) {
                resumes += 1;
            }
        }
        if session.is_exited() {
            session.flush();
            break;
        }
    }

    // Decompose the verdict so a `false` is diagnosable AND so a never-paused run (e.g. a watermark
    // raised above the corpus) reads as "backpressure not exercised", NOT as a buffer-bound pathology:
    let bound_ok = max_buffered <= high + chunk; // LESSON §24 — buffer ≤ high + one in-flight chunk
    let engaged = pauses > 0; // the high watermark actually tripped (the run stimulated backpressure)
    let cycle_ok = pauses == resumes; // every pause matched by a drain-driven resume
    let clean = bound_ok && engaged && cycle_ok;
    WatermarkResult {
        high,
        low,
        chunk,
        max_buffered,
        pauses,
        resumes,
        clean,
    }
}

// ---- shared helpers -----------------------------------------------------------------------------

/// A no-op event sink — the benchmark drives byte I/O, not the exit-event path (the production sink
/// binds the write-actor at P4). `TerminalProcessExited` is dropped.
struct BenchSink;
impl TerminalEventSink for BenchSink {
    fn emit_process_exited(&self, _event: TerminalProcessExited) {}
}

/// A [`TerminalSession`] over a [`FakePty`] scripted with `n_chunks` × `chunk_sz`-byte output chunks
/// (default watermarks). The shared fixture for the throughput pass.
fn corpus_session(id: &str, n_chunks: usize, chunk_sz: usize) -> TerminalSession {
    let chunk = vec![b'x'; chunk_sz];
    let reads: Vec<PtyRead> = (0..n_chunks)
        .map(|_| PtyRead::Chunk(chunk.clone()))
        .collect();
    let pty = FakePty::new(
        reads,
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        },
    );
    TerminalSession::new(TerminalId::from_raw(id), Box::new(pty), Box::new(BenchSink))
}

/// Count the output (data) frames in an emit list.
fn count_outputs(emits: &[TerminalEmit]) -> usize {
    emits
        .iter()
        .filter(|e| matches!(e, TerminalEmit::Output(_)))
        .count()
}

/// True if the emit list contains a control frame of `kind`.
fn has_control(emits: &[TerminalEmit], kind: TerminalControlKind) -> bool {
    emits
        .iter()
        .any(|e| matches!(e, TerminalEmit::Control(c) if c.kind == kind))
}

/// Read a `usize` env override, falling back to `default` when unset/unparseable.
fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
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

/// milliseconds as `f64` for printing a `Duration`.
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000.0
}

/// bytes as mebibytes (`f64`) — the throughput numerator.
fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
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

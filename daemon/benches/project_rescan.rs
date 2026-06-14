//! P5.4 — the §18 `project.rescan` detection-latency benchmark (the `ARCHITECTURE.md §18` CI
//! regression guard, asserted against the AS-BUILT detection scan core).
//!
//! **NON-TDD benchmark slice** (the §18 benchmark waiver, the `benches/event_write.rs` precedent +
//! LESSON 22) — the benchmark IS the coverage; there is no RED→GREEN test. It **NEVER runs inside
//! `cargo test --workspace`** (timing assertions flake in the per-slice loop). It is a `[[bench]]
//! harness = false` target (a plain `fn main()`), so it is invisible to the default suite and runs
//! only at its own cadence:
//!
//! ```text
//! cargo bench --bench project_rescan     # the /phase-exit perf-budgets runner + nightly CI
//! ```
//!
//! ## What it measures — the AS-BUILT detection scan core
//! N single-shot detections of the production `project.rescan` SCAN composition: `detect_git` (git2
//! read-only — `Repository::discover` + origin-remote read + HEAD/branch + the `statuses()` worktree
//! walk, the heaviest part) + `detect_workflow` (filesystem presence checks). This is exactly what
//! `ProjectExecutor::execute_rescan` runs minus the trivial emit (the `ProjectRescanned` serialize +
//! `strip_userinfo` + the timestamp parse — microseconds, NOT the §18-SLO concern). The §18 `< 3 s`
//! SLO governs the SCAN latency, which this measures directly. Driven over a representative committed
//! temp git repo (~15 tracked files + an origin remote + the workflow signals — a small-project
//! operating point, not a huge/empty edge), warm-cache (a rescan of a KNOWN project, the realistic
//! single-shot op — not a saturating burst).
//!
//! ## The §18 guard — READ BEFORE EDITING ASSERTIONS
//! - **SLO (§18):** `project.rescan < 3 s`. The reference baseline median is **~1.029 ms** (the
//!   discarded edges-007 run); this bench's measured median is **~0.45 ms** (p95 ~0.50 ms, p99 ~0.60 ms,
//!   max ~1.07 ms; 1000 iterations, warm cache, on a dev box — a 15-file repo: git2 discover + a small
//!   `statuses()` walk + 4 FS stats). Both are THREE-PLUS ORDERS OF MAGNITUDE under the 3 s SLO
//!   (~6700× here) — the exact number is machine/fixture-dependent; the GUARD, not the raw number, is
//!   the contract.
//! - **Guard (LESSON 22 — calibrated TIGHTER than the SLO, with margin, NOT the raw 3 s):** the
//!   **median** is gated `< 50 ms`. That is ~50-100× the measured baseline (so a real regression — a
//!   git2 / FS pathology, an accidental network or recursive walk — trips it) yet ~60× UNDER the SLO
//!   (so CI scheduling noise never flakes it). The **median**, not p95, is gated: a single-shot op's
//!   tail is dominated by OS-scheduling jitter on a shared CI box, so the central tendency is the
//!   stable regression signal; p95/p99/max are REPORTED for visibility, not gated. A breach is a
//!   `/phase-exit` perf-row Finding (a §18-budget re-baseline is a load-bearing escalation, LESSON 22).
//! - **Env overrides (smoke / isolation, no recompile):** `BENCH_RESCAN_ITERS` (measured iterations,
//!   default 1000) · `BENCH_RESCAN_WARMUP` (discarded warm-up, default 100).

use std::path::Path;
use std::time::{Duration, Instant};

use git2::{Repository, RepositoryInitOptions, Signature};

use nexusopsd::git::detect::detect_git;
use nexusopsd::workflow::detect::detect_workflow;

/// The §18 CI regression guard (ARCHITECTURE.md §18) — the `project.rescan` detection MEDIAN ceiling.
/// Calibrated tighter than the 3 s SLO with margin over the ~1.029 ms baseline (LESSON 22). See the
/// module header "READ BEFORE EDITING ASSERTIONS" before changing this.
const RESCAN_DETECTION_MEDIAN_MAX: Duration = Duration::from_millis(50);
/// The §18 SLO itself — printed as the reference ceiling (NOT the guard; the guard is far tighter).
const RESCAN_SLO: Duration = Duration::from_secs(3);

/// Default measured iterations + discarded warm-up (env-overridable for smoke runs).
const DEFAULT_ITERS: usize = 1_000;
const DEFAULT_WARMUP: usize = 100;

/// Read a `usize` env override, falling back to `default` when unset/unparseable.
fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    println!(
        "=== P5.4 §18 project.rescan detection-latency benchmark (AS-BUILT detect_git + detect_workflow) ==="
    );
    println!(
        "path under measurement: detect_git (git2 discover + origin remote + HEAD/branch + statuses() \
         worktree walk) + detect_workflow (FS presence) — the ProjectExecutor::execute_rescan scan core\n"
    );

    let warmup = env_usize("BENCH_RESCAN_WARMUP", DEFAULT_WARMUP);
    let iters = env_usize("BENCH_RESCAN_ITERS", DEFAULT_ITERS);

    let dir = tempfile::tempdir().expect("tempdir");
    build_fixture(dir.path());
    let scan_path = dir.path();

    // sanity: the fixture must exercise the REAL detection signals — a broken fixture would measure a
    // no-op (a missing-path early-out), skewing the number. Fail the bench loudly, not silently fast.
    let g = detect_git(scan_path);
    let w = detect_workflow(scan_path);
    assert!(
        g.is_git,
        "fixture is a git repo (detect_git runs the full read)"
    );
    assert!(
        g.remote_url.is_some() && g.branch.is_some(),
        "fixture has an origin remote + a branch (detect_git reads both)"
    );
    assert!(
        w.workflow_pack && w.cc_crew && w.plan_file.is_some() && w.brain,
        "fixture carries all workflow signals (detect_workflow runs every presence check)"
    );
    println!(
        "fixture: is_git={} branch={:?} remote={} dirty={} | workflow_pack={} cc_crew={} plan_file={} brain={}\n",
        g.is_git,
        g.branch,
        g.remote_url.is_some(),
        g.is_dirty,
        w.workflow_pack,
        w.cc_crew,
        w.plan_file.is_some(),
        w.brain,
    );

    // warm-up (discarded) — prime the page cache (a rescan of a KNOWN project hits warm cache; the
    // realistic operating point, not a cold-start that would bias the first samples).
    for _ in 0..warmup {
        std::hint::black_box((detect_git(scan_path), detect_workflow(scan_path)));
    }

    // measured single-shot detections (the realistic single-scan op — not a saturating burst).
    let mut samples: Vec<Duration> = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        let g = detect_git(scan_path);
        let w = detect_workflow(scan_path);
        samples.push(t0.elapsed());
        std::hint::black_box((g, w)); // defeat dead-code elimination of the detection results.
    }
    samples.sort_unstable();

    let p50 = percentile(&samples, 50.0);
    let p95 = percentile(&samples, 95.0);
    let p99 = percentile(&samples, 99.0);
    let max = samples.last().copied().unwrap_or(Duration::ZERO);

    println!("--- {iters} measured single-shot detections ({warmup} warm-up discarded) ---");
    println!(
        "  detection latency (detect_git + detect_workflow): \
         p50 {:.4}ms · p95 {:.4}ms · p99 {:.4}ms · max {:.4}ms",
        ms(p50),
        ms(p95),
        ms(p99),
        ms(max),
    );
    println!(
        "  §18 SLO reference: {:.0} ms ({:.0} s) — the measured median is ~{:.0}× under it\n",
        ms(RESCAN_SLO),
        RESCAN_SLO.as_secs_f64(),
        ms(RESCAN_SLO) / ms(p50).max(f64::MIN_POSITIVE),
    );

    // the hard §18 gate (assert LAST, after every number is printed, so a breach still captures the
    // measured baseline for the /phase-exit perf-row Finding). MEDIAN-gated; p95/p99/max reported.
    println!(
        "=== §18 CI guard assertion (median, calibrated tighter than the 3 s SLO — LESSON 22) ==="
    );
    println!(
        "  (reported, not gated) p95 {:.4}ms · p99 {:.4}ms · max {:.4}ms",
        ms(p95),
        ms(p99),
        ms(max),
    );
    check(
        "project.rescan detection median",
        p50,
        RESCAN_DETECTION_MEDIAN_MAX,
    );
    println!(
        "\nALL §18 GUARDS PASSED (median {:.4}ms < {:.0}ms guard; ≪ the {:.0}s SLO).",
        ms(p50),
        ms(RESCAN_DETECTION_MEDIAN_MAX),
        RESCAN_SLO.as_secs_f64(),
    );
}

/// Build a representative committed temp git repo (the scan target): ~15 tracked files across a couple
/// dirs + an `origin` remote + every workflow signal (`.scaffolding/manifest.json`, `.claude/`,
/// `IMPLEMENTATION_PLAN.md`, `.brain`). ALL committed → a clean tree (deterministic `is_dirty=false`),
/// so the `statuses()` worktree walk does real work without a flaky dirty-state. A small-project
/// operating point (matching the ~1 ms baseline), not a huge/empty edge.
fn build_fixture(root: &Path) {
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(root, &opts).expect("init repo");

    // a representative small project file set — committed so the worktree scan walks a real tree.
    std::fs::create_dir_all(root.join("src")).expect("mkdir src");
    std::fs::create_dir_all(root.join(".scaffolding")).expect("mkdir .scaffolding");
    std::fs::create_dir_all(root.join(".claude")).expect("mkdir .claude");

    let mut rel_paths: Vec<String> = vec![
        "README.md".to_string(),
        "Cargo.toml".to_string(),
        "IMPLEMENTATION_PLAN.md".to_string(), // detect_workflow: plan_file
        ".scaffolding/manifest.json".to_string(), // detect_workflow: workflow_pack
        ".claude/settings.json".to_string(),  // detect_workflow: cc_crew (.claude/ is_dir)
        ".brain".to_string(),                 // detect_workflow: brain (presence)
    ];
    for i in 0..12 {
        rel_paths.push(format!("src/mod_{i}.rs"));
    }
    for p in &rel_paths {
        std::fs::write(root.join(p), format!("// {p}\n")).expect("write fixture file");
    }

    // one commit of the whole tree → a clean worktree (deterministic is_dirty=false).
    let mut index = repo.index().expect("index");
    for p in &rel_paths {
        index.add_path(Path::new(p)).expect("add path");
    }
    index.write().expect("write index");
    let tree_oid = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");
    let sig = Signature::now("Bench", "bench@example.com").expect("sig");
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .expect("commit");

    // an `origin` remote so detect_git reads a remote URL (the production rescan reads origin).
    repo.remote("origin", "https://github.com/example/nexusops.git")
        .expect("set origin remote");
}

/// Nearest-rank percentile of an ALREADY-SORTED slice (ascending). `p` is in `[0, 100]`. (The
/// event_write.rs helper, verbatim — same percentile convention across the §18 benches.)
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

/// Assert a measured `value` is under a committed `max` ceiling; print the verdict either way (the
/// event_write.rs guard form — a breach prints the measured baseline before it panics).
fn check(label: &str, value: Duration, max: Duration) {
    let ok = value < max;
    println!(
        "  [{}] {label}: {:.4}ms < {:.4}ms",
        if ok { "PASS" } else { "FAIL" },
        ms(value),
        ms(max),
    );
    assert!(
        ok,
        "§18 guard breach — {label} {:.4}ms is NOT < {:.4}ms (a /phase-exit perf-row Finding)",
        ms(value),
        ms(max),
    );
}

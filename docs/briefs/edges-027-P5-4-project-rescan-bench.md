# /tdd brief — project_rescan_bench (NON-TDD bench slice)

## Feature
The P5.4 §18 `project.rescan` **detection-latency benchmark** — a new `[[bench]] harness = false` target
driving the AS-BUILT project-detection path over a representative repo, asserting the §18 SLO
(`project.rescan` < 3 s) with a CI guard calibrated on the known **median 1.029 ms** baseline + margin.

> **NON-TDD bench slice** (the §18 benchmark waiver — the `benches/event_write.rs` precedent): the
> benchmark IS the coverage, there is NO RED→GREEN test. It NEVER runs inside `cargo test --workspace`
> (timing flakes the per-slice loop); it runs only at its own cadence (`cargo bench --bench project_rescan`
> = the `/phase-exit` perf row + nightly). **Author the bench, run it ONCE to confirm ~1.029 ms, set the
> guard.** No `/tdd` RED→GREEN — this is a bench-authoring slice.

## Use case + traceability
- **Task ID:** P5.4 (the §18 `project.rescan` perf budget — deferred to the phase-exit cadence in R3/R4;
  baseline already measured in the discarded edges-007 run: **median 1.029 ms ≪ 3 s**).
- **Architecture sections it implements:** `ARCHITECTURE.md §18` (the perf budgets / CI regression guards),
  `§9` (the project-detection engine) — within P5 scope.
- **Related context:**
  - **`benches/event_write.rs`** — the EXACT precedent + LESSON 22: a `[[bench]] harness = false` target
    (`fn main()`, invisible to `cargo test --workspace`); drives the AS-BUILT production path under a defined
    load model (realistic-gated + saturating-reported); the CI guard is calibrated on the as-built
    measurement + margin (TIGHTER than the SLO), proxy numbers retired; runs at its own cadence. **Mirror its
    structure + its "READ BEFORE EDITING ASSERTIONS" guard-rationale doc-comment style.**
  - **`benches/terminal_attach.rs`** — the second bench precedent (P3.5).
  - **`project/executor.rs::ProjectExecutor::execute_rescan`** — the production `project.rescan` entry;
    composes the read-only detection engine `detect_git` (git2 reads) + `detect_workflow` (FS presence:
    workflow_pack / cc_crew / plan_file / brain). The DETECTION is the §18-SLO-heavy part (the emit is trivial).
  - **`daemon/Cargo.toml`** lines 100-115 — the `[[bench]]` pattern (`name` + `harness = false`).
  - **LESSON 22** — perf-budget bench discipline (AS-BUILT path; own cadence; guard tighter than SLO;
    a §18-budget re-baseline is a load-bearing escalation).

## Acceptance criteria (what "done" means)
- [ ] A new `daemon/benches/project_rescan.rs` (`fn main()`, NOT the libtest harness) + the
      `[[bench]] name = "project_rescan" harness = false` entry in `daemon/Cargo.toml`.
- [ ] Drives the AS-BUILT detection path (Q1) over a representative repo fixture (Q2) under a defined load
      model (N iterations; report median + p95; the event_write.rs methodology).
- [ ] Confirms the baseline (**~1.029 ms median**, ≪ the §18 3 s SLO) — run once; record the measured number
      in the bench's doc-comment.
- [ ] A CI guard assertion (Q3) calibrated TIGHTER than the 3 s SLO with margin over ~1.029 ms (LESSON 22 —
      NOT the raw 3 s SLO; a guard that catches a regression without flaking). The guard rationale is
      documented in the bench (the event_write.rs "READ BEFORE EDITING ASSERTIONS" style).
- [ ] **Invisible to `cargo test --workspace`** (verify: `cargo test` does NOT run it); runs via
      `cargo bench --bench project_rescan`.
- [ ] `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` clean (the bench compiles under the
      lint gate). No `shared/` change (CONTRACT 0.26.0 held).

## Wiring / entry point (Step 7.5)
**Reached via** `cargo bench --bench project_rescan` — the `/phase-exit` perf-budgets runner + the nightly CI
bench cadence (NOT a production code path; NOT `cargo test --workspace`). The benchmark IS the coverage (no
production wiring — the §18 bench waiver, the event_write.rs precedent). **Held-for-merge:** registering
`project_rescan` in the `/phase-exit` perf row + `.github/nightly.yml` rides the final merge (CI files are
shared-root; the impl adds the `[[bench]]` target + the bench file; the nightly/phase-exit registration is a
held-for-merge note — flag at Step 9).

## Files expected to touch
**New:**
- `daemon/benches/project_rescan.rs` — the bench.

**Modified:**
- `daemon/Cargo.toml` — the `[[bench]] name = "project_rescan" harness = false` entry.

(NO `shared/`, NO production-code change, NO `.github/` edit in-worktree — the CI registration is a
held-for-merge note.) If implementation needs files beyond this, flag at Step 9.

## Things to flag (Step-2.5-lite — no RED tests, but the design surface)
1. **The bench entry point (AS-BUILT fidelity).** Drive (a) the full `ProjectExecutor::execute_rescan`
   (an `ActionRequest` in, incl. the `ProjectRescanned` serialize — most as-built) vs (b) the detection
   composition `detect_git` + `detect_workflow` directly (the §18-SLO-heavy scan core, less setup). My
   default vote: **(b) the detection composition** — the §18 `< 3 s` SLO governs the SCAN latency (git2 +
   FS), which (b) measures directly; the emit is trivial + not the SLO concern. (If the baseline 1.029 ms
   was measured via the executor, match THAT for comparability — note which the edges-007 run used.)
2. **The repo fixture (the scan target).** A representative temp git repo (hermetic, a realistic project
   layout: a git repo + a workflow-pack/plan-file presence) vs the daemon's own repo vs synthetic. My
   default vote: **a representative temp git repo fixture** (hermetic + deterministic-ish + realistic).
   Avoid a huge/empty edge — match the 1.029 ms operating point.
3. **The CI guard value (LESSON 22).** Calibrate on the re-measured median + margin, TIGHTER than 3 s. My
   default vote: a generous bound that's ≪ 3 s but won't flake (e.g. assert median < ~50 ms, ~50× the
   1.029 ms baseline yet 60× under the SLO) — re-measure + pick the number; document the rationale. A
   tighter regression guard is better than the raw SLO (the event_write.rs lesson).
4. **Load model.** N iterations (warm + measured), median + p95 reported (the event_write.rs methodology);
   realistic-gated (a single-scan operating point, not a saturating burst — detection is a single-shot op).

## Dependencies + sequencing
- **Depends on:** edges-019 (`c739278`, the `ProjectExecutor` + `detect_git`/`detect_workflow`); the
  event_write.rs bench precedent + LESSON 22.
- **Blocks:** R7 close-out. Last R7 slice = `cargo audit` → then seal R7 → edges PAUSES for the user-gated
  `/phase-exit 5`+`7` + edges→main merge (the `/phase-exit` perf row runs this bench).

## Estimated commit count
**1.** A focused NON-TDD bench slice. NO reviewer required (a bench, no INV-SEC-1 surface, no production
code) — `code-quality` optional (the impl's call; it's bench harness code).

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the §18 `project.rescan` perf budget is benched + guarded
  (median ~1.029 ms ≪ 3 s); the guard is calibrated tighter than the SLO (LESSON 22).
- **Held-for-merge** — register `project_rescan` in the `/phase-exit` perf row + `nightly.yml` at the merge.

## How to invoke
1. **Read this brief** — it's a NON-TDD bench slice (no RED→GREEN); the 4 design points have default votes.
2. **Author `daemon/benches/project_rescan.rs`** + the `[[bench]]` Cargo.toml entry (mirror event_write.rs).
3. **Run `cargo bench --bench project_rescan` ONCE** — confirm ~1.029 ms; set + document the CI guard.
4. **Verify** `cargo test --workspace` does NOT run it; `fmt`/`clippy -D` clean.
5. **Step 9** — flag: the §18-budget arch note + the held-for-merge nightly/phase-exit registration.

# /tdd brief — project_scan_benchmark (NON-TDD — §18 benchmark)

> **Non-TDD slice.** This is the §18 project-scan benchmark (Phase 5.4) — infrastructure, not test-first logic. There is no RED→GREEN; the **bench harness + the §18 budget assertion** are the deliverable, and the **Step-2.5 review surface is the bench DESIGN** (load model + budget + cadence), exactly like the daemon's 2.5 `event_write` bench. Self-contained, single commit.

## Feature
A Criterion-free `[[bench]] harness=false` benchmark (`daemon/benches/project_scan.rs`) that drives the **landed detection engine** (`git::detect::detect_git` + `workflow::detect::detect_workflow`) on a typical repo and asserts the §18 budget **< 3 s**. Runs at its **own cadence** (`/phase-exit` + nightly), NEVER inside `cargo test --workspace`.

## Use case + traceability
- **Task ID:** P5.4
- **Architecture sections it implements:** `ARCHITECTURE.md §18` (performance budgets — the project-scan `< 3 s` SLO), `§9` (the git2 + workflow detection it benchmarks).
- **Related context:** edges-001 (the detection engine being benched — `detect_git` + `detect_workflow`); the daemon's 2.5 `daemon/benches/event_write.rs` (the `[[bench]] harness=false` + env-tunable pattern to mirror); LESSON 22 (perf benches drive the REAL path at their own cadence, calibrate on the as-built measurement).

## Acceptance criteria (what "done" means)
- [ ] `daemon/benches/project_scan.rs` (NEW) drives `detect_git` + `detect_workflow` on a synthesized typical repo, end-to-end (the real detection path, not a proxy).
- [ ] Asserts the §18 budget: the project-scan latency is **< 3 s** (the SLO; detection is ms-scale so the margin is large — assert + **report the measured time**).
- [ ] `[[bench]] name = "project_scan" harness = false` in `daemon/Cargo.toml` — **off** the default `cargo test --workspace` suite (own cadence: `cargo bench --bench project_scan`, wired to `/phase-exit` + `nightly.yml`).
- [ ] Env-tunable where it helps (repo size / file count), mirroring the 2.5 bench knobs.
- [ ] `cargo bench --bench project_scan --no-run` compiles clean; `/preflight` unaffected (the bench is off the test suite).
- [ ] **No `shared/` / `gateway/` / `eventstore/` / migration touch; no new Cargo dep** (uses `tempfile` + the existing detection engine; Brain detection is the `.brain` presence check — no real Brain ping, that's Phase 8).

## Wiring / entry point (Step 7.5)
**Entry point = the bench cadence** (`cargo bench --bench project_scan` via `/phase-exit` perf row + `nightly.yml`), NOT app code. It drives the real `detect_git`/`detect_workflow` production path end-to-end. (Same reachability shape as the daemon's 2.5 bench — infra entry, not a tested-but-unwired app feature.) The `project.rescan` *executor* that will call these in production stays gated; the bench drives the engine functions directly, which is the as-built measurable path today.

## Files expected to touch
**New:**
- `daemon/benches/project_scan.rs` — the §18 project-scan benchmark

**Modified:**
- `daemon/Cargo.toml` — `[[bench]] name = "project_scan" harness = false`

No other files. **Do NOT touch `gateway/`, `shared/`, `eventstore/`, or any migration.**

## Bench design (Step-2.5 review surface — NO RED tests; review the DESIGN)
1. **The "typical repo" fixture.** A `tempfile::tempdir()` git repo (via `git2::Repository::init`) seeded with realistic content: a handful of commits, ~N tracked files across a few dirs, + the signal markers (`.scaffolding/manifest.json`, `.claude/`, a plan file, `.brain`). Default `N` ~ a few hundred files (a "typical project"); env-tunable (`BENCH_PROJECT_FILES`).
2. **What's measured.** One full scan = `detect_git(path)` + `detect_workflow(path)`. Measure wall-clock per scan over K iterations; report min/median/p95 + assert median (or p95) **< 3 s**.
3. **The budget assertion.** Assert `< 3 s` (the §18 SLO). Since detection is ms-scale, this is a generous floor — the value is the **baseline + the regression guard at `/phase-exit`** (a future detection change that blows past 3 s is caught). Report the measured number (don't over-tighten the CI guard beyond the SLO + a margin — LESSON 22).
4. **Cadence.** `[[bench]] harness=false`, off `cargo test`; runs at `/phase-exit` + nightly. (The `/phase-exit` perf row + `nightly.yml` wiring is integration-owned — I note the wiring need for the integration owner; the bench itself is self-contained here.)

## Things to flag at Step 2.5
1. **Fixture realism.** My default vote: a synthesized tempdir repo (hermetic, deterministic, env-tunable file count). Alt: bench against a real checked-in repo (less deterministic). **Default: synthesized** — confirm the file-count default + whether to vary it.
2. **Assert on median or p95?** My default vote: **assert median < 3 s** (the SLO is a typical-scan budget), report p95 too. Confirm.
3. **Brain-status path.** The plan's 5.4 names a "Brain-status ping path (Brain faked)" — but edges-001's Brain detection is `.brain` presence-only (no ping; the real Brain-status is Phase 8). My default vote: **bench the `.brain` presence check** (what exists); note the real Brain-ping path as a Phase-8 bench extension. Confirm.
4. **`/phase-exit` + nightly wiring.** The bench file is self-contained here; the `/phase-exit` perf-row + `nightly.yml` entries that RUN it are integration-owned (the daemon's `nightly.yml` has the 2.5 bench). My default vote: **note the wiring need for the integration owner** (don't edit `nightly.yml`/CI in this worktree — it's shared/integration territory). Confirm.

## Cross-doc invariant impact
- **Model field changes:** none. The bench touches no model.
- **Shared-contract seam model touched?** **NO** → no schema-snapshot, no CONTRACT_VERSION.
- **Orchestrator doc rows to write hot:** none. (The `/phase-exit` perf-row + `nightly.yml` wiring is integration-owned — I route the note at close-out.)

## Dependencies + sequencing
- **Depends on:** edges-001 (the detection engine being benched). No Gateway / `shared/` dependency.
- **Blocks:** the Phase-5 `/phase-exit` perf-budgets row (this bench is what it runs).

## Estimated commit count
**1.** A self-contained bench file + the `Cargo.toml` `[[bench]]` entry — one infra unit, single commit, clean boundary.

## Lessons-logged candidates anticipated
- **Convention candidate** — likely covered by LESSON 22 (the daemon's 2.5 bench lesson): perf benches drive the real path at their own cadence; the project-scan bench is the §18-detection analog. A one-line extension at most.
- **Future TODO — operational** — the real Brain-status ping path (Phase 8) extends this bench; the `/phase-exit`+nightly wiring lands with the integration owner.

## How to invoke
1. **Read this brief end-to-end** — note it's NON-TDD (bench design, not RED/GREEN).
2. **Run `/tdd project_scan_benchmark`** — the `/tdd` flow adapts to the non-deterministic-coverage path (the bench-design review at Step 2.5 replaces RED/GREEN; the green bench run + the budget assertion are the coverage).
3. **Step 0 (Restate)** — confirm: the §18 project-scan bench (non-TDD), self-contained, single commit.
4. **Step 1 (files)** — confirm `benches/project_scan.rs` + the `Cargo.toml` `[[bench]]` entry only.
5. **Step 2.5** — send the **bench-design** write-up + the 4 design answers; wait for `APPROVED.`
6. **Step 9** — report the measured baseline (for my close-out note) + the `/phase-exit`+nightly wiring need (integration-owned).

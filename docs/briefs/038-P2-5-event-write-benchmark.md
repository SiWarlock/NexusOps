# /tdd brief — event_write_benchmark (§18 perf budgets vs the AS-BUILT submit path)

> **NON-TDD slice — benchmark build, not a RED→GREEN behavioral slice.** The Phase-2 `Spec anchors:` line carries `§18 (benchmark)` as an explicit **waiver class** — this task is covered by *the benchmark itself*, not by a `spec(§18)`-tagged unit test. There is **no RED test outline**. Treat the "RED outline" slot below as the **benchmark-design review surface** the implementer sends at Step 2.5 (structure + thresholds + the design-question answers), reviewed against `ARCHITECTURE.md §18` exactly like a test design. The benchmark **NEVER runs inside a per-slice RED/GREEN loop** (timing assertions are flaky there) — it runs at its own cadence (`/phase-exit` + nightly).

## Feature
A single discrete event-write/reader benchmark that asserts the committed `ARCHITECTURE.md §18` CI regression guards against the **AS-BUILT production submit path** (`Gateway::submit_action` → catalog-policy → risk-0 auto-execute → the write-actor append → eventstore, with redactor-v3 + in-band projections + outbox in the loop) — not the raw sqlite path the 0.4 spike measured.

## Use case + traceability
- **Task ID:** P2.5
- **Architecture sections it implements:** `ARCHITECTURE.md §18` (the committed perf budgets + the 5 §14 CI regression guards), `§14` (the perf-tier assertion that runs in CI).
- **Related context:**
  - The **0.4 spike** `daemon/spikes/sqlite-loadtest/` (`docs/spikes/OQ-DATA-SPIKE-3.md`) — a **throwaway standalone-workspace crate** that measured the **raw rusqlite** write path (NOT the Gateway). 2.5 **reuses its methodology** (N=20 concurrent submitters; p95/p99 percentile measurement; sustained-throughput sweep) **re-pointed at the real Gateway submit path**. Do **NOT** import the spike crate — it's a separate workspace measuring the wrong path; it's reference methodology only.
  - The production entry: `Gateway::submit_action` (`daemon/src/gateway/pipeline.rs:77`); the risk-0 auto-execute path (`submit_action_collecting`, `pipeline.rs:90`) is the cleanest intent→committed single-shot (no human-approval wait, which is not part of the write-latency budget).
  - `/phase-exit 2` perf-budgets row is currently BLOCKED *pending this benchmark* — it can't tick `n/a` because Phase 2 HAS §18 budgets.

## The committed §18 thresholds this benchmark asserts (verbatim — `ARCHITECTURE.md §18`)
| §14 CI regression guard | Committed threshold | Basis (0.4 raw-store measured) |
|---|---|---|
| Event-write p95 @ N=20 | **< 30 ms** | 5.35 ms fresh / 8.44 ms @1M |
| Event-write p99 @ N=20 | **< 75 ms** | ~47 ms @1M incl. WAL-checkpoint stalls |
| Reader p95 under write load @ N=20 | **< 10 ms** | ≤ 0.38 ms |
| Sustained single-writer throughput | **floor ≥ 1,500 commits/s** | ~4,000/s @1M, ~5,350/s fresh |
| Documented single-writer ceiling | p95 < 100 ms holds through **≥ N=100** | sweep, not saturated at 5× target |

> These are the **§14 CI guards** (measured-baseline + margin), tighter than the **PRD/§18 user-facing SLO** (p95 < 100 ms). The benchmark hard-asserts the four N=20 guards; the N=100 ceiling is a **documented sweep** (report it, don't hard-gate it — see Step-2.5 Q5).

## Acceptance criteria (what "done" means)
- [ ] A benchmark at `daemon/benches/event_write.rs` (or the Step-2.5-chosen gating form) drives **`Gateway::submit_action`** with **risk-0 auto-execute** actions at **N=20** concurrent submitters and measures **intent→committed** latency through the real write-actor.
- [ ] It computes + asserts: **event-write p95 < 30 ms**, **p99 < 75 ms**, **reader-under-write-load p95 < 10 ms** (a concurrent reader issuing `get_projection`/a read query while the N=20 writers run), **sustained throughput ≥ 1,500 commits/s** — all @ N=20, fresh DB.
- [ ] A **`/phase-exit`-callable runner** exists (a documented one-line invocation — `cargo bench --bench event_write`, or `cargo test -p daemon --test <name> -- --ignored`, per Q2) so the perf-budgets row can run it and capture the measured numbers.
- [ ] The benchmark is **excluded from the default `cargo test --workspace`** (the per-slice RED/GREEN suite) — `#[ignore]`-gated or a `[[bench]]` target with `harness = false`. Running `/preflight` (incl. `cargo test --workspace`) stays clean (247 → 247, no new default-suite tests; the bench compiles but does not run).
- [ ] **`/preflight` clean** (the bench compiles under `cargo test --no-run` / `cargo bench --no-run`; clippy `-D warnings` clean on the bench file).
- [ ] The **measured AS-BUILT numbers are recorded** (a one-line measured-baseline note in the Step-9 summary → orchestrator folds into a §18 as-built note; see Lessons candidates) — the production path adds redactor-v3 + in-band-projection + outbox overhead over the 0.4 raw-store path, so the measured p95 may differ from 5.35 ms; what matters is it stays < 30 ms.

## Wiring / entry point (Step 7.5)
This is infrastructure — the "production entry point" is the **`/phase-exit` perf-budgets row + the nightly cadence**, not an app code path. The benchmark **drives the real `Gateway::submit_action` pipeline** (`daemon/src/gateway/pipeline.rs:77`) end-to-end (policy → executor → write-actor → eventstore), so it exercises production code, but is **invoked only at bench cadence** (the `/phase-exit` runner + 2.6's nightly CI job), never on the per-slice path. The 2.6 CI brief (next) wires the runner into the nightly job; **this slice delivers the runner + the documented invocation**, 2.6 schedules it.

## Files expected to touch
**New:**
- `daemon/benches/event_write.rs` — the benchmark (N=20 concurrent submit harness + percentile/throughput computation + the §18 threshold assertions). _(or the Q2-chosen location, e.g. `daemon/tests/perf_event_write.rs` if `#[ignore]`-gated.)_

**Modified:**
- `daemon/Cargo.toml` — a `[[bench]]` entry (`name = "event_write"`, `harness = false`) and/or a dev-dependency if a harness lib is chosen (Q1). If `#[ignore]`-test form is chosen instead, no `[[bench]]` is needed.
- Possibly a tiny runner script (e.g. `scripts/bench-event-write.sh`) OR a documented `cargo bench` one-liner recorded in the brief's Step-9 + the §18 note — confirm the form at Step 2.5 (Q2).

If implementation needs files beyond this list (e.g. a shared bench-fixture builder for the Gateway), **flag at Step 2.5** before building.

## Benchmark design review surface (Step 2.5 — the "test outline" equivalent)
The implementer sends, instead of a RED outline:
1. **The harness shape** — how N=20 concurrent submitters are spawned, how intent→committed is timed (per-submit wall-clock from `submit_action` call to the committed event / the ack), how percentiles (p95/p99) + sustained throughput are computed.
2. **The measured path** — confirm risk-0 auto-execute is the path under measurement; the action type used (a real catalog risk-0 type, side-effect-free executor stub) + how the Gateway + EventStore are stood up for the bench (real on-disk WAL SQLite in a tempdir, the write-actor running).
3. **The reader-under-load probe** — what read it issues concurrently (a `get_projection` / a projection-table query) and how its p95 is measured during the write storm.
4. **The threshold assertions** — the four N=20 guards as named constants pinned to the §18 table, with the assertion posture from Q4.
5. **Coverage map** — each acceptance bullet → the part of the bench that satisfies it (or a `not-measured-because:` note, e.g. the N=100 ceiling sweep if deferred to nightly).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none.
- **Orchestrator doc rows to write hot (Step 9 routing):** an **§18 as-built measured-baseline note** (the production-path measured p95/p99/throughput vs the 0.4 raw-store numbers) — orchestrator-written into `ARCHITECTURE.md §18` at the round commit. Not a contract change; no Appendix A row.
- **§2.5-seam (shared-contract) model touched?** No — no `shared/` model, no schema-snapshot test.

## Things to flag at Step 2.5
1. **Harness: hand-rolled percentile loop vs `criterion`?** The 0.4 spike was hand-rolled (gave per-op p95/p99 + sustained throughput at N-concurrency); `criterion` is geared to micro-benchmark statistics and doesn't natively give N-concurrent percentile SLOs + adds a heavy dev-dep. My default vote: **hand-rolled** (mirror the 0.4 spike's percentile methodology) — it directly yields the §18 metrics and keeps deps lean. Raise if criterion buys something concrete.
2. **Gating form: `[[bench]]` (`harness=false`, `cargo bench`) vs an `#[ignore]`-d integration test (`cargo test -- --ignored`)?** Both keep it off the default suite. My default vote: **`[[bench]]` with `harness=false`** — cleanest isolation from `cargo test --workspace` and the natural home for `daemon/benches/event_write.rs`; the `/phase-exit` runner calls `cargo bench --bench event_write`. (An `#[ignore]` test is acceptable if standing up the Gateway is easier from the test harness — your call, you're building it.)
3. **Measured path: risk-0 auto-execute vs an approved risk-1 action?** My default vote: **risk-0 auto-execute** — it's the true single-shot intent→committed path the §18 write-latency SLO governs; approval injects a human-wait that isn't part of the budget. Use a real catalog risk-0 action type with a side-effect-free executor stub.
4. **Assertion posture: hard-fail the bench on a breach, or measure-and-report?** My default vote: **hard-assert at the committed thresholds at bench cadence** — a breach is a `/phase-exit` perf-row **Finding** (escalated to the human). But the assertions live ONLY in the bench (never in `cargo test --workspace`), so they never flake a per-slice loop. Report the measured numbers regardless of pass/fail.
5. **N=100 ceiling sweep + the @1M-events condition: in-scope now or nightly-only?** §18 frames the N=100 (p95 < 100 ms) and @1M as a *documented sweep*, not a hard N=20 gate. My default vote: **assert the four N=20 guards now (fresh DB); document the N=100 sweep + @1M as an optional nightly extension** (2.6 can schedule the heavier sweep). The 4× headroom on the N=20 guards absorbs machine variance, so the fresh-DB N=20 run is the reliable gate.
6. **Reusing vs deleting the 0.4 spike crate.** The spike (`daemon/spikes/sqlite-loadtest/`) is throwaway raw-rusqlite scaffolding in its own workspace — **don't import it**; the new bench lives in the daemon crate against the real Gateway. My default vote: **leave the spike in place** (its `OQ-DATA-SPIKE-3` provenance is referenced by §18); a later cleanup slice can delete it. Don't delete it in this slice.

## Dependencies + sequencing
- **Depends on:** 2.1 (the `Gateway::submit_action` production path ✅), 2.2/2.3 (catalog risk-0 auto-execute + the executor framework ✅), and the committed `§18` guards (✅, written at 0.4).
- **Blocks:** the `/phase-exit 2` **perf-budgets row** (currently BLOCKED on this); 2.6's nightly CI perf job (schedules this runner).

## Estimated commit count
**1.** A focused benchmark + runner. Non-safety, non-contract, single concern. Not bundled with 2.6 (CI) — different code area (`daemon/benches` vs `.github/workflows`), and keeping the benchmark commit isolated keeps the as-built §18 measurement bisectable.

## Lessons-logged candidates anticipated
- **Convention candidate** — "Benchmarks assert at their own cadence (`/phase-exit` + nightly), NEVER inside the per-slice RED/GREEN `cargo test` loop — timing assertions flake there." (Likely a `daemon/LESSONS.md` entry + a forbidden-pattern note.)
- **Architecture-doc note candidate** — the **§18 as-built measured baseline** (production submit-path p95/p99/throughput vs the 0.4 raw-store numbers); records what the redactor-v3 + in-band-projection + outbox overhead costs on the hot path.
- **Future TODO — operational** — the N=100 ceiling sweep + @1M-events condition as a nightly-only heavier run (hand to 2.6 CI).

## How to invoke
1. **Read this brief end-to-end** — note this is a **non-TDD benchmark slice** (no RED outline; the benchmark is the coverage).
2. **Run `/tdd event_write_benchmark`** — at Step 2 / Step 2.5, send the **benchmark-design review surface** above (harness shape + measured path + thresholds + the Q1–Q6 answers) instead of a RED test outline.
3. **Step 2.5** — wait for orchestrator sign-off (`APPROVED.` / `TWEAK:` / `ADD:`) on the benchmark design before building.
4. **Step 9** — surface the **measured AS-BUILT numbers** (p95/p99/reader-p95/throughput) for the §18 note, plus any design deltas.

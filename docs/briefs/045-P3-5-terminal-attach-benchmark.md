# /tdd brief — terminal_attach_benchmark (§18 terminal-attach latency + the §6.4 Terminal-Channel throughput)

> **NON-TDD slice — benchmark build, not a RED→GREEN behavioral slice.** The Phase-3 `Spec anchors:` line
> carries `§18 (benchmark)` as an explicit **waiver class** — this task is covered by *the benchmark itself*,
> not by a `spec(§18)`-tagged unit test. There is **no RED test outline**. Treat the "design review surface"
> slot below as what the implementer sends at Step 2.5 (harness shape + measured path + thresholds + the
> design-question answers), reviewed against `ARCHITECTURE.md §18` / `§6.4` exactly like a test design. The
> benchmark **NEVER runs inside a per-slice RED/GREEN loop** (timing assertions flake there) — it runs at its
> own cadence (`/phase-exit` + nightly). This mirrors brief 038 (the 2.5 event-write benchmark) + LESSON 22.

## Feature
A single terminal-attach benchmark that (1) **hard-asserts** the committed `ARCHITECTURE.md §18` terminal-attach
latency budget (**< 250 ms**, attach intent → first rendered frame) against the AS-BUILT 3.4 Terminal Channel
host, and (2) **measures + reports** the Terminal-Channel **throughput** characteristics (sustained MB/s,
frames/s, the JSON-base64 encode overhead) — numbers §18 explicitly **defers a budget for until this benchmark
lands**, and which decide the §6.4 Terminal-variant encoding (JSON-base64 MVP vs a binary fast-path).

## Use case + traceability
- **Task ID:** P3.5
- **Architecture sections it implements:** `ARCHITECTURE.md §18` (the committed **< 250 ms** terminal-attach
  budget **+** the deliberately-deferred Terminal-Channel-throughput budget — the "Unbudgeted hot paths" note
  names "(a) Terminal Channel throughput/backpressure … numbers are committed with real throughput data when
  the Phase-3 Terminal-Channel benchmark task lands" = THIS task), `§6.4` (the Terminal Channel frames + the
  JSON-base64-vs-binary encoding decision this benchmark's throughput data informs). The perf-tier assertion
  runs at bench cadence under the `§18 (benchmark)` waiver, not in the per-slice suite.
- **Related context:**
  - **Brief 038 / `daemon/benches/event_write.rs`** — the precedent benchmark. **Reuse its shape**: a
    hand-rolled percentile harness, a `[[bench]] harness = false` target (invisible to `cargo test
    --workspace`), runner = `cargo bench --bench <name>`, multi-pass (a GATED pass + a REPORTED pass). Read it
    before building — it is the house style for a §18 benchmark (LESSON 22).
  - The AS-BUILT 3.4 host (brief 041, LESSON 24): `TerminalSession` (`daemon/src/terminal/mod.rs:163`) —
    `TerminalSession::new(terminal_id, pty, sink)` → `read_step()` reads one PTY chunk into the bounded buffer
    (watermark backpressure) → `flush()` produces the seq-numbered batched `TerminalOutputFrame`s. The live
    child is `PortablePtyHost::spawn` (`terminal/mod.rs`); `FakePty` is the deterministic seam.
  - The §6.4 encoding is **JSON-base64 MVP** (LESSON 24) with the binary fast-path additively deferred to
    "3.5-with-throughput" — i.e. this benchmark's throughput + encode-overhead numbers are the data that
    justifies (or defers) the binary path.
  - `/phase-exit 3` perf-budgets row will need this runner to tick the terminal rows (today the §18
    terminal-throughput row reads `n/a — deferred`; the attach-latency row has a committed budget that needs
    this measurement).

## The thresholds this benchmark handles (verbatim — `ARCHITECTURE.md §18`)
| Measurement | §18 status | This benchmark |
|---|---|---|
| **Terminal attach latency** (attach intent → first rendered frame) | **Committed budget: < 250 ms** | **GATED** — hard-assert p95 < 250 ms (daemon-side proxy; see Step-2.5 Q1) |
| **Terminal-Channel throughput** (sustained MB/s, frames/s) | **Deferred** — "committed with real throughput data when the Phase-3 Terminal-Channel benchmark task lands" | **REPORTED** — measure + print; the orchestrator writes the now-committable §18 budget from the number |
| **JSON-base64 encode overhead** (bytes-in → frame-bytes-out ratio + encode CPU) | informs the §6.4 binary-vs-JSON decision | **REPORTED** — quantifies the binary fast-path's value |

> Only the **attach-latency** assertion is gated. The throughput + encode-overhead numbers are **reported, not
> gated** — by design: §18 *defers* the terminal-throughput budget *to this benchmark*, so there is no
> committed number to assert against yet. The orchestrator commits the §18 budget + a CI guard from the
> measured number at the round seal (Cross-doc impact below).

## Acceptance criteria (what "done" means)
- [ ] A benchmark at `daemon/benches/terminal_attach.rs` drives the **AS-BUILT 3.4 host** — a real
  `PortablePtyHost::spawn` of a minimal real program (Step-2.5 Q2) → `TerminalSession` → the first
  `flush()`-produced `TerminalOutputFrame` — and times **attach intent → first output frame ready**.
- [ ] It computes + **hard-asserts attach-latency p95 < 250 ms** over N iterations (fresh spawn each), at
  bench cadence only.
- [ ] It **measures + prints** the Terminal-Channel **sustained throughput** (MB/s + frames/s through the
  `read_step`→`flush` pipeline under a high-volume source) and the **JSON-base64 encode overhead**
  (frame-bytes-out / raw-bytes-in ratio + encode CPU) — **reported, not asserted**.
- [ ] A **`/phase-exit`-callable runner** exists (one-line `cargo bench --bench terminal_attach`) so the
  perf-budgets row can run it + capture the measured numbers.
- [ ] The benchmark is **excluded from the default `cargo test --workspace`** (`[[bench]] harness = false`);
  `/preflight` (incl. `cargo test --workspace`) stays clean (320 → 320, no new default-suite tests; the bench
  compiles but does not run).
- [ ] **`/preflight` clean** (the bench compiles under `cargo bench --no-run`; clippy `-D warnings` clean on
  the bench file).
- [ ] The **measured numbers are recorded** in the Step-9 summary (attach p95 + throughput MB/s/frames/s +
  encode-overhead %) → the orchestrator folds them into the §18 as-built note + commits the now-deferred §18
  terminal-throughput budget.

## Wiring / entry point (Step 7.5)
This is infrastructure — the "production entry point" is the **`/phase-exit` perf-budgets row + the nightly
cadence**, not an app code path. The benchmark **drives the real `TerminalSession` / `PortablePtyHost` host**
(`daemon/src/terminal/mod.rs:163`) end-to-end (spawn → read-pump → flush → frame), so it exercises production
code, but is **invoked only at bench cadence**, never on the per-slice path. A later CI brief schedules it into
nightly (the 2.6-analogue for Phase 3); **this slice delivers the runner + the documented invocation**.

## Files expected to touch
**New:**
- `daemon/benches/terminal_attach.rs` — the benchmark (the attach-latency p95 harness + the throughput /
  encode-overhead measurement + the < 250 ms assertion).

**Modified:**
- `daemon/Cargo.toml` — a `[[bench]]` entry (`name = "terminal_attach"`, `harness = false`), mirroring the
  `event_write` entry.

If implementation needs files beyond this list (e.g. exposing a tiny helper on the terminal host to observe the
first frame, or a shared bench-fixture), **flag at Step 2.5** before building — a *test-only observation helper*
on the host is acceptable; a behavior change to the host is not (this is a measurement slice).

## Benchmark design review surface (Step 2.5 — the "test outline" equivalent)
The implementer sends, instead of a RED outline:
1. **The attach-latency harness** — how N iterations each spawn a fresh `PortablePtyHost` + `TerminalSession`,
   how "attach intent → first frame ready" is timed (the wall-clock from the spawn call to the first non-empty
   `flush()` frame), how p95 is computed.
2. **The "first rendered frame" proxy** — the answer to Q1: what exact moment stands in for the cross-track ui
   xterm.js render at the daemon boundary, and the `not-measured-because` note for the excluded downstream
   (UDS hop + xterm render).
3. **The throughput harness** — the high-volume source, how MB/s + frames/s through `read_step`→`flush` are
   measured, and how the JSON-base64 encode overhead is computed.
4. **The assertion posture** — the single gated `< 250 ms` p95 constant pinned to the §18 table; everything
   else printed.
5. **Coverage map** — each acceptance bullet → the part of the bench that satisfies it (or a
   `not-measured-because:` note).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none.
- **Orchestrator doc rows to write hot (Step 9 routing):** (1) an **§18 as-built note** for the measured
  terminal-attach p95 (vs the < 250 ms budget); (2) **commit the deferred §18 Terminal-Channel-throughput
  budget** from the measured number — this flips the §18 "Unbudgeted hot paths (a)" row from `n/a — deferred`
  to a committed budget + CI guard, and feeds the §6.4 binary-vs-JSON encoding note. Both are
  orchestrator-written `ARCHITECTURE.md §18`/`§6.4` edits at the round commit. **Not** a contract change; no
  Appendix A row.
- **Shared-contract seam model touched?** No — no `shared/` model, no schema-snapshot test.

## Things to flag at Step 2.5
1. **The "first rendered frame" daemon-side proxy (the crux).** The §18 budget is end-to-end (attach intent →
   ui xterm.js render), but the ui xterm.js host is cross-track (6.3d) — the daemon can't render. **My default
   vote:** measure to the **first non-empty `TerminalOutputFrame` produced by `flush()`** (the daemon's
   "ready-to-emit" boundary), and record a `not-measured-because` for the excluded downstream (UDS hop +
   xterm render — small + cross-track; folded into the budget when 6.3d lands). The daemon-side spawn+exec+
   first-read dominates the budget and has huge headroom under 250 ms, so a breach here is a real signal.
   Raise if you'd rather measure to the first frame *serialized for the wire* instead.
2. **Real `PortablePtyHost` vs `FakePty` for the latency pass.** `FakePty` has no real spawn cost, so it can't
   measure attach latency. **My default vote:** a **real `PortablePtyHost::spawn`** of a minimal, always-present
   program that writes a deterministic first line (e.g. `printf "ready\n"` / a tiny echo) — so first-frame is
   deterministic and the measurement captures the real spawn+exec+pipe+read+flush cost (the event_write bench
   used a real on-disk WAL for the same reason). `FakePty` is fine for the *throughput* pass (a fixed
   high-volume corpus), where real spawn cost isn't the variable.
3. **Gating form: `[[bench]] harness = false` vs `#[ignore]` test.** **My default vote:** **`[[bench]]
   harness = false`** — the `event_write` precedent; cleanest isolation from the default suite; runner =
   `cargo bench --bench terminal_attach`.
4. **Latency aggregate: p95 over N vs single-shot.** **My default vote:** **p95 over N fresh iterations** —
   spawn cost varies (FS cache, scheduler); p95 is the SLO's basis and absorbs machine variance. N ~ the
   event_write bench's order (tens–hundreds).
5. **Throughput source + what to report for the §6.4 call.** **My default vote:** drive a high-volume source
   through `read_step`→`flush` and report **sustained MB/s + frames/s + the JSON-base64 encode overhead**
   (frame-bytes-out / raw-bytes-in + encode CPU time), so the binary fast-path's value is *quantified* — that
   number is exactly what decides whether the binary path is worth building. The xterm.js 5–35 MB/s ceiling
   (§18 note) is the reference bar to compare against.
6. **Watermark/batch tuning as a side output?** The carry-forward names "watermark/tick tuning + partial-flush
   hysteresis" as a 3.5-adjacent concern. **My default vote:** **measure + report** the watermark behavior the
   throughput run exercises (does the bounded buffer hold; does pause/resume cycle cleanly) but **don't tune**
   in this slice — tuning is a behavior change with its own RED tests, out of a measurement slice's scope.
   Flag any pathology you observe.

## Dependencies + sequencing
- **Depends on:** 3.4 (the Terminal Channel host ✅ — `TerminalSession`/`PortablePtyHost`/`FakePty` landed),
  the committed `§18` < 250 ms attach budget (✅), the `event_write` bench precedent (✅).
- **Blocks:** the `/phase-exit 3` terminal perf rows; the §6.4 binary-vs-JSON encoding decision (this
  benchmark's throughput number is its input); the nightly Phase-3 CI perf job (schedules this runner).
- **Fork-free** — independent of every P4 fork; safe to build now while the P4 forks are with the user.

## Estimated commit count
**1.** A focused benchmark + `[[bench]]` entry. Non-safety, non-contract, single concern. Not bundled — the
as-built §18 terminal measurement stays bisectable, and there is no related slice to bundle with (P4 is gated).

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the **§18 as-built terminal baseline** (measured attach p95 + the
  now-committed Terminal-Channel-throughput budget) + the **§6.4 encoding-decision input** (the JSON-base64
  encode overhead → whether the binary fast-path is justified).
- **Convention candidate** — likely none net-new (LESSON 22 already covers "benchmarks assert at their own
  cadence"); if the "first rendered frame" daemon-side proxy generalizes to the future P4 daemon-restart-
  recovery-latency benchmark, note it.
- **Future TODO — operational** — watermark/tick tuning + partial-flush hysteresis (observed-not-tuned here) +
  the binary-encoding fast-path (justified-or-deferred by this benchmark's number).

## How to invoke
1. **Read this brief end-to-end** — note this is a **non-TDD benchmark slice** (no RED outline; the benchmark
   is the coverage). Read `daemon/benches/event_write.rs` first — it's the house style.
2. **Run `/tdd terminal_attach_benchmark`** — at Step 2 / Step 2.5, send the **benchmark-design review
   surface** above (harness shape + the "first frame" proxy + thresholds + the Q1–Q6 answers) instead of a RED
   test outline.
3. **Step 2.5** — wait for orchestrator sign-off (`APPROVED.` / `TWEAK:` / `ADD:`) on the benchmark design
   before building.
4. **Step 9** — surface the **measured numbers** (attach p95 + throughput MB/s/frames/s + encode-overhead %)
   for the §18 note + the deferred-budget commit, plus any watermark pathology observed.

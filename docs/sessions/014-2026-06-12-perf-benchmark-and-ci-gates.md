# Session 014 — Phase 2.5/2.6: §18 perf benchmark + the §14 CI merge gates (Phase-2 close-out)

- **Date:** 2026-06-12
- **Phase:** Phase 2.5 (§18 event-write benchmark) + 2.6 (CI merge gates) — the two non-TDD close-out slices; `/phase-exit 2` → **CLEAR, Phase 2 complete**.
- **Predecessor:** [013 — the §17 safety capstone (L1–L5)](013-2026-06-11-gateway-17-safety-capstone.md)
- **Successor:** [015 — the §9.1 HarnessAdapter contract freeze + the proj_usage_ledger projector](015-2026-06-12-harness-adapter-contract-and-usage-ledger.md)

## Why this session existed
Phase 2 (the Action Gateway) was code-complete at the 2.4 §17 capstone. Two ops tasks remained to close the phase: **2.5** — the §18 performance budget benchmark over the real submit path (the `/phase-exit 2` perf-budgets row was blocked on it); **2.6** — the first CI workflow wiring the §14 merge-gate tiers + the §5.0 contract gates. Both are **non-TDD** (the benchmark / green CI run IS the coverage). Briefs `038` + `039`.

## What was built

**Files created**
- `daemon/benches/event_write.rs` — the §18 event-write/reader benchmark (`[[bench]] harness=false`, off the default `cargo test --workspace` suite). Drives the **AS-BUILT** submit path: N submitters → `WriteHandle::submit_action_blocking` → single write-actor → `CatalogPolicy` → risk-0 `project.rescan` auto-execute → `CatalogExecutor` → `EventStore` (redactor-v3 + in-band projections + outbox). 4 passes: A realistic-latency (gated p95) · B sustained-capacity (gated throughput + reader p95) · C saturating stress ceiling (reported) · D N=100/@1M (nightly, `BENCH_CEILING=1`). Env-tunable.
- `.github/workflows/ci.yml` — 5 parallel merge-gate jobs: **daemon** (fmt/clippy/test = local `/preflight`; incl. §5.0 test-9 schema-diff + the fault-injection tier) · **ui** (typecheck/oxlint/vitest; advisory) · **contract-3way** (`shared/contracts/verify/run.sh`, §5.0 test-8) · **dep-audit** (`cargo audit` + `pnpm audit --prod`) · **spec-lint** (advisory).
- `.github/workflows/nightly.yml` — the §18 perf bench (`BENCH_CEILING=1 cargo bench --bench event_write`), non-merge-gating; commented live-agent-smoke + Codex-schema-regen placeholders → P3.

**Files modified**
- `daemon/Cargo.toml` — `[[bench]] name="event_write" harness=false` (no new deps; uses the existing `tempfile` dev-dep + the `nexusopsd` self-dev-dep).

## Decisions made
- **§18 CI guards re-baselined — USER-RULED Option A (realistic-load).** The 2.5 benchmark surfaced a **Finding**: brief-038's guards (30 ms p95 / 75 ms p99 / 1,500 commits/s) were calibrated on the 0.4 spike's **1-transaction raw-store model**, which doesn't exist in production — every commit carries the §15 redactor + in-band projections + outbox, and a risk-0 submit is **3 commits** (events/submit = 3.00 confirmed). Measured AS-BUILT: N=1 healthy (1.78 ms / 3943 ev/s); under the spike's closed-loop-zero-think methodology 57–135 ms / 744–1366 ev/s — missing the raw-append guards but meeting the real user-facing SLO (p95 < 100 ms). The user re-baselined to: **write p95 < 25 ms (realistic non-saturating load)** · **throughput ≥ 1,000 events/s (sustained capacity)** · **reader p95 < 10 ms**; p99 reported-not-gated; the saturating numbers kept as reported stress ceiling. Measured post-re-baseline: **6.08 ms / 1,378 ev/s / 0.022 ms — all pass.** (Full rationale in the tracker's Decisions-tabled, orch-written.)
- **CI ≡ local `/preflight`** for daemon + ui (no drift between what CI checks and what the implementer checks pre-commit).
- **ui CI job is advisory (`continue-on-error`)** — ORCH-RULED (A). The ui track is paused at an older contract (generated.ts 0.12.0) while the daemon advanced `shared/` to 0.19.0; the §5.0 ui-Zod drift sentinel fires expectedly. Advisory keeps the gate wired + visible without a standing-red job; **promotes to blocking at the ui-track resume** (rides the generated.ts regen — in Carry-forward).
- **Dropped `prettier --check`** from the ui gate (oxlint is the project's format authority; no prettier in `ui/`) — the one place the brief diverged from the tracker's literal command list (clerical).
- **Self-caught + fixed a benchmark artifact:** the first reader probe was a continuous no-sleep full-table-scan → starved WAL autocheckpoint + monopolized a core, tanking throughput 6× and inflating p95 to 408 ms. Fixed: paced + project-scoped via `ix_audit_scope` → reader p95 0.022 ms. The reported numbers are post-fix.

## Decisions explicitly NOT made
- **The FTS-per-event size-degradation** (throughput halving 15k→30k events; the audit projector's FTS5 DELETE+INSERT per event is O(index)) — logged as a **future perf-hardening task** (orch Carry-forward), accept-and-document at MVP scale (realistic load is non-saturating + small data). Not optimized this session.
- **ui-Zod regen to 0.19.0** — cross-track / ui territory, gated on the ui-track resume (Carry-forward). Not touched here.

## TDD compliance
**Clean — both slices are explicit non-TDD waivers** (the Phase-2 `Spec anchors:` line carries `§18 (benchmark)` + `§14/§5.0 (CI config)` as waiver classes). No RED→GREEN; the benchmark assertions + the green CI run are the coverage. The non-deterministic/infra coverage path was followed (benchmark-design + CI-design review surfaces at Step 2.5, orch-approved before build).

## Reachability
- **2.5 benchmark** — entry point = the `/phase-exit` perf-budgets row + the nightly cadence (infra, not app code). Reachable via `cargo bench --bench event_write` (the `/phase-exit` runner + `nightly.yml`). It drives the real `Gateway::submit_action` production path end-to-end.
- **2.6 CI** — entry point = GitHub Actions (`push`/`pull_request`/`schedule` triggers). The workflow wires existing runners (the §5.0 harnesses, cargo/pnpm, spec-lint, the 2.5 bench). Live Actions run validates on the user's authorized push (recorded deferral — see below).
- No tested-but-unwired app feature. No `shared/` model, no schema-snapshot.

## Open follow-ups
- **Live CI run deferred to the user's authorized push** (origin is 22+ commits behind; push user-gated). Step-9 verification this session = every job's commands run green locally (daemon 247 · contract-3way 33-enums@0.19.0 · cargo+pnpm audit 0 vulns · spec-lint phase-2 PASS) + actionlint clean on both yaml. The first GitHub Actions run is coherent with the same push that resolves the phase-exit push row. (Orch recorded as a known deferral, not a gap.)
- **ui CI job → promote to blocking** at the ui-track resume (rides the ui-Zod regen to 0.19.0). In Carry-forward (orch).
- **FTS-per-event perf-hardening** (see Decisions-NOT-made). In Carry-forward (orch).
- **Step-9 cross-doc routing (orch-owned, already flagged + handled):** the §18 + §14 CI-guard re-baseline → `ARCHITECTURE.md` + Decisions-tabled (orch-written, rides its `/orchestrate-end` seal); the Carry-forward "Wire the §5.0 contract gates into CI" item → RESOLVED by 2.6 (orch marks done).

## How to use what was built
- **Run the perf bench:** `cargo bench --bench event_write` (gated §18 guards) · `BENCH_CEILING=1 cargo bench --bench event_write` (+ N=100 sweep). Env knobs: `BENCH_WRITERS` / `BENCH_SUBMITS` / `BENCH_THINK_MS` / `BENCH_READER=0` for smoke/diagnosis.
- **CI:** fires on push-to-main + every PR; nightly perf on cron + `workflow_dispatch`. To accept a `cargo audit` advisory: `cargo audit --ignore RUSTSEC-xxxx` in `ci.yml` + record it in Decisions-tabled.

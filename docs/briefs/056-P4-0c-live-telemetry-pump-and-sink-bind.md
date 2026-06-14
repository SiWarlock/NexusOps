# /tdd brief — live_telemetry_pump_and_sink_bind

## Feature
Bind the Claude adapter's telemetry emission to the **production** path: a `WriteHandle`-backed `TelemetryEventSink` (constructed in the runtime, injected into the `ClaudeAdapter`) + a **periodic pump** that drives `telemetry_heartbeat`→emit at the statusLine refresh cadence off the session-actor + a **non-monotonic-cumulative-cost clamp (→ ≥0)** + **`metric_quality` degrade-on-guard** (Estimated/Unavailable when context-% is missing/guard-tripped — never a faked 0%, §11.4). This is the **044 P4 deferral** (the seam landed sync + sink-less at 044; this wires the live production sink + the pump). **NON-safety** — telemetry is a non-mutation OBSERVATION event (write-actor, not the Gateway; LESSONS §23/§27), so INV-SEC-1 is untouched.

## Use case + traceability
- **Task ID:** P4.0c (the `### 4.0c` Phase-4 row — live telemetry pump + sink-bind; the 044 P4 deferral, non-safety)
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (the `telemetry_heartbeat` emission / the production sink-bind), `§11.4` (the usage/metric-quality degradation — never a faked 0%), `§18` (the usage ledger feed). NON-safety (an observation event, not a Gateway mutation).
- **Phase-scope note — this brief WIDENS phase scope because** §11.4 (usage/metric-quality degradation) + §18 (the usage-ledger feed) are the downstream behaviors of the §9.1 telemetry emission this slice wires live; §9.1 is in Phase-4's `Spec anchors`, §11.4/§18 are its consumers (NON-safety — an observation event, not a Gateway mutation).
- **Related context:**
  - `daemon/src/harness/claude/telemetry.rs` — the `TelemetryEventSink` trait (`:78`, `Send`-only) + `telemetry_sample(prev, reading) -> TelemetrySample` (`:44`, the per-heartbeat DELTA computation) + `UsageReading`. The module doc (`:11`) names "the PRODUCTION sink-binding" as this slice.
  - `daemon/src/harness/claude/mod.rs` — `ClaudeAdapter.telemetry_sink: Option<Box<dyn TelemetryEventSink>>` (`:356`) + the **`with_telemetry_sink(sink)` builder** (`:379`, the injection seam — 044) + `telemetry_heartbeat() -> Option<TelemetrySample>` (`:467`).
  - `daemon/src/session/actor.rs` — the `SessionActor` (the read-pump + the §5.1 status-poll tick, `:69`). **Cat-1 boundary (4.0a, `:14`/`:98-101`):** the session module takes **NO `WriteHandle`** (import-grep-enforced — emission/mutation compile-time impossible). The pump rides this actor but must NOT import `WriteHandle`.
  - **LESSONS §23** (telemetry = a non-mutation observation event via the write-actor, NOT the Gateway — INV-SEC-1 routes mutations, not observations), **§27** (emit per-heartbeat DELTAS not cumulative; context-% a gauge; via an injected sink; daemon-Clock UTC-Z gates `bucket_day`), **§9** (the write-actor idiom + the session-actor tick).
  - **Carry-forward (the 044→P4 telemetry-hardening pins, origin 2026-06-12 044):** (a) the production sink-bind + the periodic pump + the trait async-ify + live transcript/statusLine ingestion — TOGETHER; (b) the non-monotonic-cost clamp→≥0; (c) `metric_quality` degrade-on-guard; (d) `+ Sync` on the sink IF shared across the drive loop's threads.

## Acceptance criteria (what "done" means)
- [ ] **The production `TelemetryEventSink` appends `TelemetrySampled` via the write-actor.** A `WriteHandle`-backed sink impl (e.g. `WriteActorTelemetrySink`) constructed in the runtime/main.rs (where the `WriteHandle` lives) + injected into the `ClaudeAdapter` via `with_telemetry_sink` BEFORE the adapter reaches the session launcher. The session module's **cat-1 import-grep stays clean** (no `WriteHandle` in `session/` — the sink is an opaque `Box<dyn>`, constructed outside).
- [ ] **The pump emits at the refresh cadence.** A periodic tick on the session-actor calls `telemetry_heartbeat()`→(adapter emits via the injected sink) at the statusLine refresh interval; absent a sink the heartbeat is still correct (044 — sink-less safe). The pump never blocks the actor's command mailbox (rides the existing tick / a `MissedTickBehavior::Delay` interval, LESSONS §9).
- [ ] **Non-monotonic cumulative cost clamps to ≥0.** If the external upstream reports a DECREASING cumulative cost, the emitted cost DELTA is clamped to **≥0** (never a negative delta — the `proj_usage_ledger` SUMs; a negative would corrupt the rollup). Pinned by a test (a decreasing-cumulative reading → delta 0, not negative).
- [ ] **`metric_quality` degrades on guard (§11.4).** When context-% is missing OR a guard trips, the sample's `metric_quality` is `Estimated`/`Unavailable` (never a faked `context_pct = 0.0` — a real 0% and "unknown" must be distinguishable). Pinned by a test.
- [ ] **The emission stays a non-mutation observation event** (write-actor `append`, NOT a Gateway action) — INV-SEC-1 untouched; the `TelemetrySampled` envelope `occurred_at` is daemon-Clock **UTC-Z** (gates `bucket_day`, LESSONS §27/§5).
- [ ] **A real-append integration test** drives the production sink → asserts a `TelemetrySampled` row lands + `proj_usage_ledger` SUMs the deltas correctly (differential negative controls per LESSONS §27).
- [ ] All unit/integration tests pass; `/preflight` clean.
- [ ] Cross-doc: none expected (the §9.1 emission path was frozen at 044; this is the production wiring — confirm at Step 9 whether any §9.1/§11.4 AS-BUILT note is warranted).

## Wiring / entry point (Step 7.5)
The production sink is constructed + injected in the runtime (`main.rs` / the session-launch path where the `WriteHandle` is available) → handed to the `ClaudeAdapter` via `with_telemetry_sink` → the live drive loop / session-actor pump drives the periodic emit. Production-reachable on the live session path (the same path 4.0b-2 launches). The session module stays cat-1-clean (the sink is injected from outside; no `WriteHandle` import).

## Files expected to touch
**New / Modified:**
- `daemon/src/harness/claude/telemetry.rs` — the production `WriteActorTelemetrySink` (holds the `WriteHandle`, appends `TelemetrySampled`) + the cost-clamp in/around `telemetry_sample` + the `metric_quality` degrade-on-guard.
- `daemon/src/session/actor.rs` — the periodic telemetry pump tick (calls `telemetry_heartbeat`→emit; NO `WriteHandle` import — cat-1 clean).
- `daemon/src/main.rs` (+ the runtime session-launch path) — construct the production sink + inject via `with_telemetry_sink`; the refresh-interval constant.
- `daemon/tests/<claude_telemetry|projections>.rs` — the cost-clamp, the metric_quality-degrade, the real-append + ledger-SUM integration test.

If implementation needs files beyond this list (esp. if the pump needs the `HarnessAdapter` trait async-ified — see Step 2.5 Q5), **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`test_production_sink_appends_telemetry_sampled`** — the `WriteHandle`-backed sink → a `TelemetrySampled` row lands via the write-actor (observation event, not Gateway).
2. **`test_pump_emits_at_cadence`** — the periodic tick drives `telemetry_heartbeat`→emit; N ticks → N samples (or the delta-correct count); sink-less is a no-op (044-safe).
3. **`test_non_monotonic_cost_clamps_to_zero`** — a decreasing cumulative-cost reading → the emitted cost delta is `0` (≥0), never negative.
   - Why: the ledger SUMs; a negative delta corrupts the rollup.
4. **`test_metric_quality_degrades_on_guard`** — context-% missing / guard-tripped → `metric_quality = Estimated/Unavailable`, `context_pct = None` (never a faked 0.0).
   - Why: §11.4 — a real 0% and "unknown" must be distinguishable.
5. **`test_emit_is_observation_not_gateway`** — the emit path appends via the write-actor, never the Gateway (grep/seam-pinned); `occurred_at` UTC-Z.
6. **`test_real_append_ledger_sums_deltas`** — integration: drive the production sink → `proj_usage_ledger` SUMs `tokens_in`/`tokens_out`/`cost_estimate` correctly (the LESSONS §27 differential negative controls — cumulative mis-SUMs, deltaed-ctx mis-MAXes).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — `TelemetrySample`/`MetricQuality`/`TelemetrySampled` were frozen at 3.1/044 (CONTRACT 0.20.0). This is the production WIRING → **no CONTRACT bump**, no schema-snapshot.
- **Orchestrator doc rows to write hot:** possibly a §9.1/§11.4 AS-BUILT note (the production sink-bind + pump + cost-clamp + metric_quality-degrade are now live) → the §9.1 HarnessAdapter row. Confirm at Step 9.
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **The production sink construction + injection.** A `WriteActorTelemetrySink` holding the `WriteHandle`, constructed in main.rs/runtime, injected via `with_telemetry_sink` before the session takes the adapter — so `session/` never imports `WriteHandle` (cat-1 import-grep clean). My vote: **yes** — this is the established injection seam (044) + keeps the boundary. Confirm the construction site (where the `WriteHandle` + the adapter meet pre-launch).
2. **Pump cadence + location.** The periodic tick on the session-actor (reuse the §5.1 status-poll tick, or a dedicated telemetry interval). Default interval: ~the statusLine refresh (a few seconds). Confirm the value + whether to reuse the existing tick or add one.
3. **Cost-clamp placement.** Clamp the cost DELTA to ≥0 in `telemetry_sample` (on a decreasing cumulative). My vote: clamp the delta (not the cumulative) — the cumulative is the upstream's; we emit a non-negative delta. Confirm token deltas get the same guard (tokens shouldn't decrease, but a clamp is cheap insurance).
4. **`metric_quality` degrade source.** `Estimated`/`Unavailable` when context-% is missing or a guard trips — never a faked `0.0`. Confirm the exact guard conditions (missing statusLine field, parse failure, capability `supports_context_metadata=false`).
5. **[Possible scope] `HarnessAdapter` trait async-ify?** The 044 pin bundled "the trait async-ify" with the production wiring. But the pump rides the session-actor's `spawn_blocking`/tick + the `WriteHandle::append` is an mpsc send (sync from the caller). **My vote: NO async-ify needed** — the sync sink + the session-actor tick suffice (the trait stays sync, LESSONS §23/§28; async-ify would be churn with no caller). Confirm — if the pump genuinely needs async, flag it (it widens scope).
6. **`+ Sync` on the sink?** Today `Send`-only (the `TerminalEventSink` precedent). If the sink is shared across the drive loop's threads, it needs `+ Sync`. My vote: keep `Send`-only unless the wiring shares it (re-evaluate at the construction site). Confirm.

## Dependencies + sequencing
- **Depends on:** 4.0a (the session-actor the pump rides — landed ✅) + 044 (the `TelemetryEventSink` seam + `telemetry_sample` + `with_telemetry_sink` — landed ✅) + the live drive loop (4.0b-2 — the production session path).
- **Blocks:** the Usage Dashboard live data (Phase-6 ui consumes `proj_usage_ledger`).

## Estimated commit count
**1-2.** The production sink + the pump + the cost-clamp + the metric_quality-degrade are one coherent telemetry-wiring unit. Split to 2 only if the pump-on-the-actor wiring grows large enough to bisect from the sink/clamp/quality logic. **NON-safety** (an observation event) → `security-reviewer` is `off`/not-required by policy (the `invariant`-gated policy doesn't fire — no safety invariant touched); `code-quality-reviewer` runs per the `every-slice` policy.

## Lessons-logged candidates anticipated
- **Convention candidate** — "the production observation-event sink holds the `WriteHandle` + is constructed in the runtime + injected into the adapter (the cat-1 EDGE module never imports `WriteHandle`); the pump rides the session-actor tick; emit DELTAS clamped ≥0; degrade `metric_quality` on guard, never a faked 0%."
- **Architecture-doc note candidate** — the §9.1/§11.4 production telemetry path is now live (sink-bind + pump + clamp + degrade).

## How to invoke
1. **Read this brief end-to-end** — especially Q1 (the sink injection / cat-1 boundary) + Q5 (the possible async-ify scope).
2. **Run `/tdd live_telemetry_pump_and_sink_bind`**.
3. **Step 2.5** — send the test-design write-up + answers to Q1-Q6. (NON-safety — no safety fork expected; surface a scope-widen if the pump needs the trait async-ify.)
4. **Step 8** — `code-quality-reviewer` (every-slice); `security-reviewer` not required (no safety invariant touched — confirm the observation-event / non-Gateway path holds).
5. **Step 9** — surface any §9.1/§11.4 AS-BUILT note + the lessons-logged candidates.

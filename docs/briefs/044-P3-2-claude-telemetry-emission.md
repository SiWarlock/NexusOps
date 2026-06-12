# /tdd brief — claude_telemetry_emission

## Feature
The Claude adapter's telemetry path (the last 3.2 piece): derive **per-heartbeat token/cost DELTAS** (never cumulative snapshots) from Claude's structured usage readings, and emit them as `TelemetrySampled` OBSERVATION events through an injected sink (the 3.4 `TerminalEventSink` precedent) so the daemon-Clock UTC-`Z` `occurred_at` lets `proj_usage_ledger` bucket + SUM correctly. **NON-safety** (a non-mutation observation event, LESSON 23 — not a Gateway Action).

## Use case + traceability
- **Task ID:** P3.2 (part 2, telemetry — the remaining piece after the 043 interception)
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (HarnessAdapter `telemetry_heartbeat`), `§7.1` (`TelemetrySampled` EventTypeRegistry), `§7.2` (harness-derived SoT), `§18` (usage rollups). Spec anchors per the Phase-3 line.

> **The cited anchor set widens phase scope because** the genuine IMPLEMENTATION anchors are all in Phase-3's set (§9.1 the adapter `telemetry_heartbeat`, §7.2 harness-derived SoT, §18 usage rollups). The other §refs are **mechanism/invariant cross-references, NOT new implementation:** §7.1 (the `TelemetrySampled` EventTypeRegistry entry — already frozen @ 3.1 / 0.20.0, only EMITTED here), §15 (the redaction-before-persist gate the write-actor append already enforces — reused), §11.4 (capability-gated degradation — reused), §2.5 (the seam discipline — confirms NO new frozen model), §8 (ExecutionProfile binding — deferred to P4, only referenced). No new contract surface; no §2.5-seam model field change.

- **Related context:**
  - **LESSON 23** — telemetry = a non-mutation OBSERVATION event (System/adapter-actor → write-actor → §15 redactor → projector), NOT a Gateway Action; read fold semantics off the DDL (`*_max`=MAX gauge, plain=SUM).
  - **LESSON 5** — daemon `Clock` is UTC-`Z`; the `proj_usage_ledger` `bucket_day` (first-10-chars) relies on it + fail-closes on a non-`Z` source.
  - **LESSON 24/25** — the 3.4 `TerminalEventSink` emission seam (the production impl binds `WriteHandle::append` at the drive loop; `CollectingSink` in tests) + the 3.2 PTY-primary adapter `push_signal` ingestion seam. 044 mirrors both.
  - Prior slices: 3.1 froze `TelemetrySample`/`TelemetrySampled` @ CONTRACT **0.20.0** + the `proj_usage_ledger` projector (`daemon/src/projections/usage.rs`); 042 built the observe path; 043 built the interception. The `telemetry_heartbeat()` body is currently a named `None` stub (`daemon/src/harness/claude/mod.rs:425`).
  - **Carry-forward "P3.1 forward pins":** the 3.2/3.3 emission acceptance pins (i) DELTAS-not-cumulative, (ii) UTC-`Z` `occurred_at`, (iii) the live emission path + the trait async-ify (the async-ify + the production pump ride **P4** — see Dependencies).

## Acceptance criteria (what "done" means)
- [ ] The adapter tracks cumulative usage and `telemetry_heartbeat()` returns a **per-heartbeat DELTA** `TelemetrySample` (`tokens_in`/`tokens_out`/`cost_estimate` = current_cumulative − last_emitted_cumulative), **not** the cumulative snapshot — so the SUMming `proj_usage_ledger` does not over-count. `None` before the first reading.
- [ ] `context_pct` is carried as the **CURRENT gauge value** (pass-through), **NOT** a delta — the projector takes its MAX (`context_pct_max`), so a delta would be meaningless. (tokens/cost = delta; context_pct = gauge — the load-bearing distinction.)
- [ ] A new cumulative reading emits exactly one `TelemetrySampled{sample(delta), model, execution_profile_id}` through the injected telemetry-event sink (the `TerminalEventSink` precedent); the production sink-binding + the periodic pump = P4 (named deferral).
- [ ] **End-to-end DELTA + UTC-`Z` pin:** appending the emitted delta `TelemetrySampled` events through the REAL write-actor (fake Clock, UTC-`Z`) → `proj_usage_ledger` rollup has `tokens_in`/`tokens_out`/`cost_estimate` == the cumulative total (SUM-of-deltas) and `bucket_day` == the UTC date. (Proves delta-not-cumulative AND UTC-`Z` bucketing together.)
- [ ] `cost_estimate` = the delta of Claude's **reported** cumulative cost (the "estimate" honesty — no local model-pricing table); `metric_quality` = `Exact` when `context_pct` is present, `Estimated` when it degrades (the 3.1 `FakeHarness` precedent + §11.4 — never a faked 0%).
- [ ] The pure usage parser binds Claude's structured usage source (transcript `ResultMessage` `usage`/`total_cost_usd` + statusLine input) → a typed `UsageReading`, fixture-tested (reject-malformed fails closed, never a fabricated reading).
- [ ] All unit tests in `daemon/tests/claude_telemetry.rs` pass; the projector integration in `daemon/tests/projections.rs` passes.
- [ ] `test_observe_path_stubs_marked` (`daemon/tests/claude_adapter.rs:462`) updated — `telemetry_heartbeat` is no longer an always-`None` stub.
- [ ] `/preflight` clean.
- [ ] **Cross-doc invariant: NONE** — `TelemetrySample`/`TelemetrySampled` are already frozen @ 0.20.0; 044 only EMITS them. New types (`UsageReading`, the telemetry sink trait) are daemon-internal + unfrozen (the `ClaudeSignal`/`TerminalEventSink` precedent). No CONTRACT bump, no §2.5-seam schema-snapshot test.

## Wiring / entry point (Step 7.5)
**none — the production pump + sink-binding land in P4 (the session-lifecycle drive loop), per the 3.4 `TerminalEventSink` precedent (whose production impl "binds the write-actor at the drive loop").** 044 builds the delta-derivation + the emission seam, reachable via `telemetry_heartbeat()` + `push_usage()` + an injected sink, and exercised by (a) the adapter unit tests and (b) the real write-actor + `proj_usage_ledger` projector integration test. This is the same NAMED-deferral shape as 043's `route_intercept` (tested-but-unwired-until-P4) and 3.4's terminal host. State it explicitly at Step 7.5; it is NOT a silent wiring gap.

## Files expected to touch
**New:**
- `daemon/src/harness/claude/telemetry.rs` — the `UsageReading` type (cumulative) + the pure cumulative→delta logic + `telemetry_sample(prev_cumulative, reading) -> TelemetrySample` + the `TelemetryEventSink` trait (mirrors `TerminalEventSink`) + the pure usage parser (transcript `ResultMessage`/statusLine JSON → `UsageReading`, reject-malformed). Mirrors `status.rs`/`intercept.rs`.
- `daemon/tests/claude_telemetry.rs` — the RED tests (delta-not-cumulative · context-gauge-not-delta · emit-once-via-sink · cost/metric_quality derivation · parser fixtures).

**Modified:**
- `daemon/src/harness/claude/mod.rs` — `ClaudeAdapter`: add `last_cumulative: Option<UsageReading>` + an optional `execution_profile_id` field (None until P4 sets it) + the injected telemetry sink (constructor or builder — Step-2.5 Q3); `push_usage(&mut self, UsageReading)` (computes the delta, updates `last_cumulative`, stores the latest delta sample, emits via the sink); fill the `telemetry_heartbeat()` body (return the stored latest delta sample). `pub mod telemetry`.
- `daemon/tests/claude_adapter.rs` — update `test_observe_path_stubs_marked` (telemetry no longer always-`None`).
- `daemon/tests/projections.rs` — the SUM-of-deltas + UTC-`Z` bucket integration (extends the 3.1 usage-ledger projector tests) — OR co-locate in `claude_telemetry.rs` (Step-2.5 Q4).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/claude_telemetry.rs` (+ the projector pin in `daemon/tests/projections.rs`):

1. **`test_telemetry_heartbeat_emits_deltas_not_cumulative`** — feed two CUMULATIVE readings (e.g. {in:100,out:20,cost:0.01} then {in:250,out:60,cost:0.03}); the second `telemetry_heartbeat()` (after `push_usage`) returns the DELTA {in:150,out:40,cost:0.02}, not the cumulative.
   - Asserts: delta == current − last_emitted.
   - Why: `proj_usage_ledger` SUMs (`usage.rs` `+ excluded.tokens_in`); cumulative would over-count — the §9.1/3.1 "adapter emits deltas" pin (`shared/src/harness.rs:48`).

2. **`test_first_reading_is_full_delta_from_zero`** — the FIRST `push_usage` (no prior) emits the reading as-is (delta from 0); `telemetry_heartbeat()` is `None` before any reading.
   - Asserts: first delta == first cumulative; pre-reading == None.
   - Why: no double-count on session start, no fabricated pre-reading sample (§11.4 honesty).

3. **`test_context_pct_is_gauge_not_delta`** — readings with context_pct 30% then 55%; the emitted samples carry context_pct 30 then 55 (pass-through), NOT a 25 delta.
   - Asserts: context_pct == the reading's current value, never differenced.
   - Why: `usage.rs` takes `context_pct_max` (MAX gauge); a delta would mis-bucket — the load-bearing tokens-delta-vs-context-gauge distinction.

4. **`test_push_usage_emits_one_telemetry_sampled_via_sink`** — a `CollectingSink` double; one `push_usage` → exactly one `TelemetrySampled{sample(delta), model:Some, execution_profile_id}` captured.
   - Asserts: sink called once; payload carries the delta sample + model + profile (the projector's payload dims).
   - Why: the 3.4 `TerminalEventSink` emission-seam precedent (LESSON 23/24); `model`/`execution_profile_id` are the dims `usage.rs` reads from the PAYLOAD (`usage.rs:44-46`).

5. **`test_cost_and_metric_quality_derivation`** — cost_estimate == delta of Claude's reported cumulative cost; metric_quality == `Exact` when context_pct present, `Estimated` when absent.
   - Asserts: cost is a reported-cost delta (no pricing table); quality degrades honestly.
   - Why: §9.1 `cost`→`cost_estimate` honesty + §11.4 (the 3.1 `FakeHarness` precedent, `harness/mod.rs:255`).

6. **`test_usage_parser_binds_fixture_rejects_malformed`** — a fixture Claude `ResultMessage`/statusLine JSON → the expected `UsageReading`; a malformed/absent-usage input → `None`/error (fail-closed), never a fabricated reading.
   - Asserts: documented fields parse; malformed fails closed.
   - Why: §7.2 harness-derived SoT + §15 reject-unknown; the parser is the structured-signal seam (the `derive_status` precedent).

7. **`test_usage_ledger_sums_deltas_utc_bucketed`** (`daemon/tests/projections.rs`) — append the two delta `TelemetrySampled` events from #1 through the REAL write-actor (fake Clock, UTC-`Z`) → the `proj_usage_ledger` row has tokens_in 250 / tokens_out 60 / cost 0.03 (SUM-of-deltas == cumulative) and bucket_day == the UTC date.
   - Asserts: SUM(deltas) == cumulative total; bucket_day == UTC date prefix.
   - Why: the end-to-end DELTA + UTC-`Z` pin (LESSON 5; `usage.rs` `utc_bucket_day` fail-closed) — proves both invariants through the real path.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NONE. `TelemetrySample`/`TelemetrySampled` frozen @ 3.1 / CONTRACT 0.20.0; 044 only emits them. `UsageReading` + the `TelemetryEventSink` trait are daemon-internal + unfrozen (`ClaudeSignal`/`TerminalEventSink` precedent).
- **CONTRACT bump:** NONE. No `shared/` change → no schema regen, no §2.5-seam snapshot test.
- **Orchestrator doc rows to write hot (Step 9 routing):** none are cross-doc-invariant rows. At the 044 `/orchestrate-end` round seal the orchestrator flips the prose AS-BUILT markers (NOT contract rows): the `daemon/CLAUDE.md` §9.1 row "Telemetry emission = 044" → AS-BUILT; the `ARCHITECTURE.md §9.1` AS-BUILT note + the `usage.rs`/`harness.rs`/`events.rs` "emission lands 3.2/3.3" prose comments. LESSON 27 candidate (the delta-emit + gauge-pass-through + emission-seam pattern; generalizes to Codex 3.3).
- **§2.5-seam (shared-contract) model touched?** No — no Appendix-A model field changes; no schema-snapshot test required.

## Things to flag at Step 2.5
1. **How much of the emission lands in 044 vs P4?** Options: (A) 044 builds the delta-derivation + the `TelemetryEventSink` seam + a fake double + the real-append integration pin; P4 binds the production sink (write-actor) + the periodic pump + the live transcript/statusLine I/O + the trait async-ify. (B) 044 also wires a production sink now. My default vote: **(A)** — it mirrors the 3.4 `TerminalEventSink` precedent EXACTLY (production impl binds the write-actor at the drive loop) and matches the carry-forward ("the live emission path + the trait async-ify ride the P4 drive loop"); the periodic pump cadence is a P4 drive-loop concern. The full path is still deterministically proven via the real write-actor + projector integration test.
2. **Usage ingestion — a separate `push_usage(UsageReading)` or a new `ClaudeSignal::Usage` variant?** My default vote: **separate `push_usage` + a `UsageReading` struct** — keeps telemetry orthogonal to status derivation, so `derive_status` (safety #9, the pure status fold) stays untouched and a usage reading can't accidentally drive a status transition.
3. **Where is the telemetry sink injected — constructor or a `with_telemetry_sink` builder?** My default vote: **constructor parameter** (mirrors `TerminalSession::new(.., sink)`), with the production binding deferred to P4. If it churns too many call sites, a builder/`Option<sink>` is acceptable — flag the choice.
4. **The integration pin — `projections.rs` or `claude_telemetry.rs`?** My default vote: **`projections.rs`** (extends the existing 3.1 usage-ledger projector tests; the SUM/MAX/bucket semantics live there). Adapter-side delta/emission units stay in `claude_telemetry.rs`.
5. **The exact Claude usage schema (`ResultMessage.usage` field names / `total_cost_usd` / the statusLine input cost object) — confirm against a real Claude session.** My default vote: **author the parser against the documented fields + a committed fixture; validate the exact field names at the impl-fixture step** (the 042/043 precedent — live-Claude-surface specifics validated at the impl/P4 boundary, never blocking the deterministic core). If a field name is wrong, it's a one-line parser fix, not a design change. The receiver/tailer I/O that FEEDS the parser is P4.
6. **`execution_profile_id` source.** It's resolved at approval/launch time (safety #8) by the P4 drive loop; for 044 it's an optional adapter field defaulting to `None` (the projector keys `None`→`~` sentinel). Default vote: **thread it as `Option`, None in 044's standalone path, set by P4.** Confirm no over-reach into profile-binding (that's P4/§8).

## Dependencies + sequencing
- **Depends on:** 3.1 (the frozen `TelemetrySample`/`TelemetrySampled` @ 0.20.0 + the `proj_usage_ledger` projector — landed); 042 (the ClaudeAdapter observe path + `push_signal` ingestion seam — landed); 3.4 (the `TerminalEventSink` emission-seam precedent — landed).
- **Blocks:** **P4** (the drive loop binds the production telemetry sink to the write-actor + drives the periodic pump from the statusLine `refreshInterval` heartbeat + async-ifies the trait + wires the live transcript/statusLine ingestion). **3.3** (the Codex adapter reuses the delta-emit + gauge-pass-through + sink pattern; `supports_context_metadata=false` → context_pct None, metric_quality Estimated).
- **Not safety-blocking:** NON-safety (the lead's framing) — telemetry is a non-mutation observation event routed through the EXISTING §15-gated write-actor append (no new mutation path, no §15/INV-SEC-1 mechanism touched).

## Estimated commit count
**1–2.** One focused concern (Claude telemetry emission) in one area (`daemon/src/harness/claude/`), NON-safety, < ~100 lines. Either one bundled slice, or two thin layers (L1 = `UsageReading` + the pure delta/parser logic + `telemetry_heartbeat` body; L2 = the `TelemetryEventSink` seam + the projector integration pin). The implementer picks at Step 2.5 by size; no safety-critical pin forces its own commit here. **Reviewer policy (Step 8):** `code-quality-reviewer` every-slice; **`security-reviewer` NOT required** by the `invariant` policy — 044 touches no §15/INV-SEC-1 mechanism (it consumes the already-gated observation-event append). If Step 2.5 surfaces an invariant concern, escalate and add it.

## Lessons-logged candidates anticipated
- **Convention candidate (LESSON 27)** — "A harness telemetry adapter emits per-heartbeat token/cost DELTAS (cumulative − last_emitted) so the SUMming usage projector doesn't over-count; context_pct rides as a CURRENT gauge (pass-through, projector-MAX'd), never a delta; the emission is a non-mutation `TelemetrySampled` OBSERVATION event through an injected sink (the `TerminalEventSink` precedent), the daemon-Clock UTC-`Z` `occurred_at` gating the projector's `bucket_day` — production sink-binding + the periodic pump at the drive loop." Generalizes to Codex 3.3.
- **Future TODO — belongs to P4** — bind the production `TelemetryEventSink` to the write-actor + drive the periodic pump from the statusLine `refreshInterval` heartbeat + async-ify the `HarnessAdapter` trait + wire the live transcript/statusLine ingestion I/O.
- **Architecture-doc note candidate** — the §9.1 telemetry emission AS-BUILT (flip "= 044" → landed) at the round seal.

## How to invoke
1. **Read this brief end-to-end.** Don't skip "Things to flag at Step 2.5" — answer Q1 (the 044/P4 emission boundary) before tests.
2. **Run `/tdd claude_telemetry_emission`.**
3. **Step 0 (Restate)** — confirm against the Feature line (delta-emit + gauge-pass-through + the emission seam; NON-safety; no contract bump).
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — ping back the test-design write-up (one `Asserts: <invariant> (§anchor)` per test + the acceptance→test coverage map) with answers to the design questions (or defaults).
6. **Step 7.5** — state the P4 wiring deferral explicitly (the named-deferral shape).
7. **Step 9 (summarize)** — surface anything beyond the anticipated lessons-logged candidates; confirm "Cross-doc invariant: none / no CONTRACT bump."

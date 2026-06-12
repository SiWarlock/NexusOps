# Session 015 — Phase 3.1: the §9.1 HarnessAdapter contract freeze + the proj_usage_ledger projector

- **Date:** 2026-06-12
- **Phase:** Phase 3.1 (harness adapters & embedded terminal — the first slice: the §9.1 contract freeze + the re-homed usage projector). First Phase-3 slice; the critical-path successor to Phase 2.
- **Predecessor:** [014 — §18 perf benchmark + §14 CI merge gates (Phase-2 close-out)](014-2026-06-12-perf-benchmark-and-ci-gates.md)
- **Successor:** _(next session — Phase 3.2 Claude adapter ∥ 3.3 Codex adapter ∥ 3.4 terminal)_

## Why this session existed
Phase 2 (the Action Gateway) sealed; Phase 3 (harness adapters + embedded terminal) is the critical-path successor (P4 survival depends on adapters existing). **3.1** is the foundation slice: freeze the one `HarnessAdapter` contract both the Claude (3.2) and Codex (3.3) adapters implement — its normalized return types, capabilities, and the per-harness mutation-coverage matrix — as the next §2.5-seam shared-contract freeze, and land the `proj_usage_ledger` projector (re-homed from P1.2 to its producing phase). Brief `040` (spec-lint PASS @3c8ffad5). Two layers, two commits.

## What was built

### Files created
- **`shared/src/harness.rs`** — the §9.1 normalized return types (the §2.5-seam freeze): `TelemetrySample{tokens_in,tokens_out,context_pct:Option<f32>,cost_estimate,metric_quality}` · `MetricQuality{exact|estimated|unavailable}` · `TranscriptRef{path,hash,is_in_place}` · `HarnessCapabilities` (the 10 PRD-HARN-5 bool fields). `NormalizedStatus` re-exports the frozen `status::Session` (17 states) — adapters map INTO it, no forked enum. All `deny_unknown_fields`; optionals serialize explicit `null` (stable snapshot, LESSON §15 trap 3).
- **`daemon/src/harness/mod.rs`** — the **daemon-internal + UNFROZEN** layer: the `HarnessAdapter` trait (sync, object-safe `Box<dyn>`, `Send+Sync` — the `ActionExecutor`/`PolicyEngine` seam precedent) · `MutationIntercept{tool,params,decision_sink}` + `MutationVerdict`/`MutationDecisionSink` (non-serializable callback → daemon-side) · `MUTATION_COVERAGE_MATRIX` (8 cells, verbatim §9.1/O-13) + `coverage_of` · `ResumeResult` (daemon-internal; freezes in `shared/` at Phase 4) · `FakeHarness` (capability-respecting test double).
- **`daemon/src/projections/usage.rs`** — the `UsageLedgerProjector`: folds `TelemetrySampled` → `proj_usage_ledger`; `ledger_id` = deterministic composite `(project|session|profile|day|model)` (model LAST → collision-free over free-form model strings); tokens/cost SUM, `context_pct_max` MAX (NULL-safe CASE), `metric_quality` worst-wins; `bucket_day` = UTC date of `occurred_at` (fail-closed on non-UTC-Z, the Flag-4 guard).

### Files modified
- **`shared/src/events.rs`** — `TelemetrySampled{sample, model?, execution_profile_id?}` + `EVENT_TYPE` const (the §7.1 EventTypeRegistry accretion; a non-mutation observation event).
- **`shared/src/lib.rs`** — `pub mod harness;` + `CONTRACT_VERSION` 0.19.0 → **0.20.0** (additive, no frozen type reshaped).
- **`shared/src/schema.rs`** — registered the 4 new shared types + the event in `ContractBundle`.
- **`shared/contracts/schema/nexusops-contract.schema.json`** — regenerated (`cargo run -p nexusops-shared --bin emit_schema`); test-9 byte-diff green.
- **`daemon/src/lib.rs`** — `pub mod harness;`.
- **`daemon/src/projections/mod.rs`** — `mod usage;` + `UsageLedgerProjector` registered in `projectors()`.
- **Tests** — `shared/tests/contract.rs` (6 L1 contract tests + 3 version-pin updates), `shared/tests/envelope.rs` (version pin), `daemon/tests/projections.rs` (8 L2 projector tests, reusing the existing harness).

### Commits
- **L1 `93a1141`** — `feat(harness)`: the §9.1 contract freeze + TelemetrySampled (CONTRACT 0.20.0).
- **L2 `a40ac00`** — `feat(projections)`: the proj_usage_ledger projector.

## Decisions made
- **Q1 (load-bearing) — `TelemetrySampled` is a non-mutation OBSERVATION event, NOT a Gateway Action** (lead-confirmed at Step 2.5; security-reviewer CLEAR). INV-SEC-1 (#1) governs FS/git/external/session-STATE mutations; a telemetry observation mutates none → it follows the System/adapter-actor non-mutation precedent (`DeviceRegistered`/`AuditIntegrityViolation`): written via the single write-actor through the §15 redaction gate, never the Gateway pipeline. #2 holds (only the daemon writes the DB; only the Gateway executes *mutations* — this isn't one); #3 holds (the projector reads only already-redacted events).
- **Q2 — `NormalizedStatus = pub use status::Session`** (re-export, not a fork); pinned `== Session` by a compile-time identity fn + `ALL == Session::ALL` (17).
- **Q3 — `ledger_id` = delimited composite, model LAST** (`project|session|profile|day|model`), `None`→`~` sentinel. The 4 leading dims are delimiter-free by contract (ULIDs + date), so a `|` in the one free-form dim (model) can't shift a boundary → collision-free for well-formed inputs.
- **Q4 — `metric_quality` worst-wins** (any `unavailable`→unavailable, else any `estimated`→estimated, else `exact`) via an upsert CASE.
- **Q5 — shared vs daemon split:** the 4 data types + the event → `shared/`; the trait + `MutationIntercept` + the coverage matrix + `ResumeResult` → daemon-only.
- **Q6 — `cost_estimate`/`metric_quality`** field names (DDL match); §9.1 prose `cost`/`quality` is a benign naming reconcile (arch-note).
- **Sync object-safe trait** (orchestrator-approved Flag 2) — mirrors the established `ActionExecutor`/`PolicyEngine`/`PreconditionOracle` `Box<dyn>` seams; no speculative `async-trait` dep. The trait is daemon-internal + UNFROZEN → 3.2/3.3 reshape/async-ify the drive loop freely. Only the normalized DATA types are the §2.5-seam freeze.
- **Flag-4 UTC-Z guard** — `bucket_day` fails closed (Decode → degrade, loud) on a non-UTC-Z `occurred_at` rather than silently mis-bucketing into a local day (the daemon Clock emits UTC-Z, LESSON §5).
- **L2 tests in `daemon/tests/projections.rs`** (orchestrator-approved Flag 1) — reuse the existing harness; drive the projector through the REAL append path (proves `apply_all` reachability + exercises `rebuild()`/degrade-skip), rather than inline unit tests.

## Decisions explicitly NOT made (deferred)
- **`ResumeResult` NOT frozen in `shared/`** (the one Step-2.5 TWEAK) — kept daemon-internal; it freezes in `shared/` at **Phase 4** with the §8/§17 survival schema. Freezing `{resumed_live,…}` now would collide with the ui's provisional `ResumeMode` enum at P4 = a breaking reshape the §2.5-seam discipline exists to prevent.
- **The live `TelemetrySampled` emission path** (`telemetry_heartbeat` → write-actor) — lands with the real adapters (3.2/3.3); this slice defines the event + folds synthetic ones.
- **The `HarnessAdapter` trait's production drive callers** (launch/stream_status/resume I/O + async) — 3.2/3.3.
- **tokens `as i64` cast hardening** (TryFrom/saturating) — deferred; tokens are daemon-emitted realistic deltas (a low; accepted-and-documented).

## TDD compliance
**Clean.** All three layers were test-first: the L1 contract tests, the daemon trait/matrix/FakeHarness inline tests, and the L2 projector tests were written and confirmed RED (compile-level — the referenced types did not exist) before any production type was written. One additional test (`test_usage_projector_context_pct_max_null_orderings`) was added **post-GREEN at code-review** — it pins a NULL-handling path the reviewer flagged as untested; the impl already handled it correctly (the reviewer confirmed the SQL was correct), so this is a coverage addition, **not** a TDD violation (no implementation was written to satisfy a back-filled test).

## Reachability
- **`UsageLedgerProjector` — WIRED.** Registered in `projectors()` → reached by `apply_all` (`daemon/src/eventstore/mod.rs:728`, the in-band fan-out on every event-commit txn) + `catch_up_replay`/`rebuild`. The L2 tests drive it through the real `EventStore::append` path with synthetic `TelemetrySampled` events. Reachable from the production write entry.
- **The `HarnessAdapter` trait + normalized types — contract-only this slice (intentional).** The shared types are reachable via `ContractBundle` (schema gen) + the snapshot tests; the trait is satisfied by `FakeHarness` (object-safety + satisfiability proven). The trait's production drive callers (launch/stream_status/…) land in 3.2/3.3 — stated honestly per the brief's Step-7.5. **Not a silent gap** — it is the next-slices' wiring.

## Open follow-ups
Step-9 categorized items were routed hot to the orchestrator (its `/orchestrate-end` is the single verify pass). For the record:
- **Cross-doc invariant change** (orchestrator hot-writes, present in the working tree): CONTRACT 0.19.0→0.20.0 + the new shared types + `TelemetrySampled` → ARCHITECTURE.md Appendix A rows 566 (EventTypeRegistry) + 569–571 (HarnessAdapter trait / HarnessCapabilities / coverage matrix, AS-BUILT) + `daemon/CLAUDE.md` cross-doc table.
- **Architecture-doc note:** the §9.1/§7.1 telemetry-as-observation-event note (Q1) + the §9.1 `cost`→`cost_estimate` / `quality`→`metric_quality` naming reconcile + the ResumeResult-deferred-to-P4 note.
- **Convention candidate (LESSON §23):** telemetry/usage is event-sourced as an observation event (adapter/System-actor → single write-actor → §15 redactor → projector), NOT a Gateway action; a daemon-internal adapter seam stays sync + object-safe (`Box<dyn>`), only the normalized DATA types are the §2.5-seam freeze.
- **Future TODO — belongs to Phase 3.2/3.3 (acceptance pins):** (i) adapters emit per-heartbeat token **deltas**, not cumulative totals (so the projector SUM is correct); (ii) adapters emit **UTC-Z** `occurred_at` (the bucket_day guard fails closed otherwise); (iii) the trait async-ifies / the real launch/stream_status/resume I/O + the live `TelemetrySampled` emission; (iv) `ResumeResult` freezes in `shared/` at **Phase 4** with the survival schema.
- **Future TODO — bounded residuals:** tokens `as i64` cast (TryFrom guard if a non-trusted telemetry source ever appears); `ledger_id` (switch to a hash key if a free-form non-ULID profile dim is ever introduced).

## How to use what was built
- A new harness adapter (3.2 Claude / 3.3 Codex) implements `daemon::harness::HarnessAdapter` and returns the frozen `shared::harness` types; its `telemetry_heartbeat` emits a `TelemetrySampled` event via the write-actor, which the `UsageLedgerProjector` folds into `proj_usage_ledger`.
- Per-capability UI degradation reads `HarnessCapabilities` (e.g. `supports_context_metadata=false` → render context-% as "unknown"); `MetricQuality` carried on every sample tells the UsageMeter when a figure is estimated.
- The `MUTATION_COVERAGE_MATRIX` is the binding §9.1 contract the §14 conformance suite (3.2/3.3) asserts per category per harness.

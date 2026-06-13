# /tdd brief — harness_adapter_contract_and_usage_ledger

## Feature
Freeze the §9.1 `HarnessAdapter` contract — the one trait both Claude + Codex adapters implement, its normalized return types (reusing the frozen `Session` status enum), `HarnessCapabilities`, and the per-harness mutation-coverage matrix — as the next §2.5-seam shared-contract freeze (one `CONTRACT_VERSION` bump), and land the `proj_usage_ledger` projector that folds the new telemetry event into per-day usage rollups.

## Use case + traceability
- **Task ID:** P3.1
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (HarnessAdapter layer — LOCKED ADR-006), `§7.1` (EventTypeRegistry — the new `TelemetrySampled` payload), `§7`/`§7.2` (projector fold; UsageLedger projection), `§18`/`§11` (usage rollups), `§5.0` (Rust-authority schema freeze + 3-way verify), Appendix A rows: **HarnessAdapter trait + normalized types**, **HarnessCapabilities**, **Per-harness mutation-coverage matrix** (§9.1).
- **Related context:** session doc 014 (Phase-2 seal); the §2.5-seam list in `ARCHITECTURE.md` line 138 (HarnessAdapter normalized types are a listed shared-contract model → snapshot test mandatory); the existing freeze precedents — `shared/src/events.rs` (the event-payload pattern + `EVENT_TYPE` const), `shared/src/schema.rs` (`ContractBundle` registration + `emit_schema_json`), `shared/tests/contract.rs` (the `expect_fields`/`check_values`/version-pin test forms — **mirror these exactly**), `daemon/src/projections/session.rs` (the projector body pattern), `daemon/src/projections/mod.rs` (`Projector` trait + `projectors()` registry), `daemon/src/eventstore/schema.rs:253` (the `proj_usage_ledger` DDL — table already migrated in M3).

> **Widens phase scope because** this P3.1 task implements two anchors that sit on Phase-1's §7 surface but which the architecture **explicitly accretes per producing phase**: **§7.1** (the new `TelemetrySampled` EventTypeRegistry payload — `events.rs:3` "the registry accretes per phase… Phase 2/3 add their payloads additively"; the Phase-2 briefs added the §7.1 `ActionExecution*` family the same way) and **§7** (the `proj_usage_ledger` projector body — `projections/mod.rs::projectors()` states "the later-phase projectors get their bodies with the phase that emits their feeding events"; the UsageLedger projection was re-homed to its producing phase = P3.1 per the tracker, `(origin: 2026-06-07 P1.2)`). The other cited sections are **cross-references, not new implementation scope:** §5.0 (the schema-authority freeze *mechanism* every §2.5-seam slice runs), §2.5 (the §2.5-*seam* dependency-edge concept, not arch §2.5), §15 (redaction — the projector only ever reads already-redacted events), §11.7/§13.1 (the UI/Brain *consumers* of the frozen types). No safety invariant is newly *enforced* here — only the contract those mechanisms bind to is frozen.

> **Safety rules quoted by name (do not paraphrase):** **#9** — *Never scrape the PTY for machine state; derive status from SDK/app-server streams; PTY is display-only (§9.1).* **#10** — *Brain proposes, never executes; Claude-driven sessions run default permission mode only; no background subagents until #27203 is fixed (§9.1/§13.1, O-13).* These are encoded as **types + the coverage matrix** here (the contract); their *enforcement* is 3.2/3.3 behavior. The brief freezes the contract so the enforcement has a shape to bind to.

## Acceptance criteria (what "done" means)

**L1 — the §9.1 contract freeze (one commit; `CONTRACT_VERSION` 0.19.0 → 0.20.0):**
- [ ] `shared/src/harness.rs` (NEW) defines the **normalized return types** as serializable contract structs (`#[serde(deny_unknown_fields)]`, `JsonSchema`, snake_case): `TelemetrySample{tokens_in, tokens_out, context_pct: Option<f32>, cost_estimate, metric_quality}`, `MetricQuality` enum (`exact | estimated | unavailable`), `TranscriptRef{path, hash, is_in_place}`, `ResumeResult`, `HarnessCapabilities` (the 10 PRD HARN-5 bool fields: `supports_terminal, supports_resume, supports_transcript_read, supports_tool_call_parsing, supports_usage_metadata, supports_context_metadata, supports_command_injection, supports_subagents, supports_hooks, supports_cloud_tasks`). **`NormalizedStatus` is NOT a new type — it is the frozen `status::Session` enum** (status.rs:45 already names it "the driver-agnostic normalized vocabulary the §9.1 harness adapters map INTO"); re-export it as `pub use crate::status::Session as NormalizedStatus` or reference `Session` directly (Step-2.5 Q2).
- [ ] `shared/src/events.rs` gains the **`TelemetrySampled`** event payload (`#[serde(deny_unknown_fields)]`, `EVENT_TYPE = "TelemetrySampled"` const) carrying `{sample: TelemetrySample, model: Option<String>, execution_profile_id: Option<String>}` — the rollup dims the envelope lacks (`session_id`/`project_id`/`occurred_at` come from the envelope columns; `model`/`execution_profile_id` do not — confirmed against `event_envelope.rs`).
- [ ] `daemon/src/harness/mod.rs` (NEW) defines the **`HarnessAdapter` trait**: `{launch, stream_status, intercept_mutation, read_transcript, telemetry_heartbeat, resume, capabilities}` (async where the lifecycle implies it; object-safe so `Box<dyn HarnessAdapter>` works), plus **`MutationIntercept{tool, params, decision_sink}`** (the `decision_sink` is a daemon-side callback — NOT serializable → lives here, not in `shared/`), and the **per-harness mutation-coverage matrix** as a typed const asserting the §9.1 table verbatim (Claude `can_use_tool` × Codex `app-server` × {direct, subagent, background-subagent, mcp} → `Guaranteed | BestEffort | Unsupported`).
- [ ] Every new `shared/` type is registered in `ContractBundle` (`shared/src/schema.rs`) and the checked-in artifact `shared/contracts/schema/nexusops-contract.schema.json` is **regenerated** (`cargo run -p nexusops-shared --bin emit_schema`) so `test_schema_artifact_matches_rust` (test 9) passes.
- [ ] `CONTRACT_VERSION` bumped to **`0.20.0`** (`shared/src/lib.rs`); the version-pin test updated (add a `test_contract_version_bumped_0_20_0` mirroring `test_contract_version_bumped_0_19_0`).
- [ ] **§2.5-seam schema-snapshot tests** (`spec(§9.1)`-tagged) pin the field-name set of every new shared struct + the `MetricQuality` value set + reject-unknown + round-trip + the `TelemetrySampled::EVENT_TYPE` const — mirroring the `expect_fields`/`check_values` forms in `contract.rs`.
- [ ] A minimal **`FakeHarness`** test double implements `HarnessAdapter` (proves the trait is satisfiable + object-safe; the real adapters land 3.2/3.3) — in `daemon/src/harness/` test code or a `fake` submodule.

**L2 — the `proj_usage_ledger` projector (one commit; no contract change):**
- [ ] `daemon/src/projections/usage.rs` (NEW) — `UsageLedgerProjector` folds `TelemetrySampled` events into `proj_usage_ledger`: row keyed by `ledger_id` = deterministic composite of `(project_id, session_id, execution_profile_id, model, bucket_day)`; `bucket_day` derived from `env.occurred_at` (UTC date); **`tokens_in`/`tokens_out`/`cost_estimate` accumulate (SUM)**, **`context_pct_max` takes the MAX** (the DDL column names are the contract — `context_pct_max` is a gauge, the rest accumulate), `metric_quality` = worst-quality-wins across the bucket, `updated_at_seq = env.seq`.
- [ ] Registered in `projectors()` (`daemon/src/projections/mod.rs`) so it runs in the in-band fan-out (reachable on every event-commit txn). Non-`TelemetrySampled` events are a healthy no-op (the `session.rs` `if env.event_type != ... { return Ok(()) }` precedent).
- [ ] Idempotent under replay: a full `rebuild()` reproduces identical rollups (the upsert keys on `ledger_id`; re-folding the same events yields the same SUM/MAX because each event is folded exactly once by the engine).
- [ ] Reject-unknown: a `TelemetrySampled` payload that doesn't bind → `ProjectionError::Decode` (degrade-skip, reason MUST NOT echo payload bytes — §15), never stored raw.
- [ ] All unit tests in `shared/tests/` (L1) + `daemon/src/projections/usage.rs` tests (L2) pass; `/preflight` clean (fmt + clippy `-D warnings` + full suite + `cargo run --bin emit_schema` produces no diff).
- [ ] Cross-doc invariant rows written by the orchestrator at Step 9 (see below).

## Wiring / entry point (Step 7.5)
- **L1 contract types** — the shared types are reachable via `ContractBundle` (schema gen) + the snapshot tests + `FakeHarness`; the `HarnessAdapter` trait's production callers (`launch`/`stream_status`/…) land in **3.2/3.3** (the real adapters) and the daemon runtime that drives them — `none for the trait's drive loop — wiring lands in 3.2/3.3`. State this honestly: the trait + types are the contract; the live drive path is the next slices.
- **L2 projector** — `UsageLedgerProjector` IS wired this slice (added to `projectors()` → runs inside `apply_all` on every event-commit txn, the §7 in-band fan-out). Its **feeding event emission** (`TelemetrySampled` written via the write-actor from a real adapter's `telemetry_heartbeat`) lands in **3.2/3.3** — so the projector is reachable + registered now but its `TelemetrySampled` branch only fires once the adapters emit (exactly the `proj_session`-folds-`SessionStarted`-before-lifecycle-events precedent noted in `projectors()`). Tests drive it with synthetic `TelemetrySampled` envelopes.

## Files expected to touch
**New:**
- `shared/src/harness.rs` — the normalized data types + `HarnessCapabilities` + `MetricQuality`.
- `daemon/src/harness/mod.rs` — the `HarnessAdapter` trait + `MutationIntercept` (with `decision_sink`) + the coverage matrix const + `FakeHarness`.
- `daemon/src/projections/usage.rs` — `UsageLedgerProjector`.

**Modified:**
- `shared/src/lib.rs` — `pub mod harness;` + `CONTRACT_VERSION = "0.20.0"`.
- `shared/src/events.rs` — `TelemetrySampled` payload + `EVENT_TYPE` const.
- `shared/src/schema.rs` — register the new types in `ContractBundle`.
- `shared/contracts/schema/nexusops-contract.schema.json` — regenerated artifact (committed).
- `shared/tests/contract.rs` (or a new `shared/tests/harness.rs`) — the §2.5-seam snapshot tests + version-pin update.
- `daemon/src/lib.rs` — `pub mod harness;` (register the new daemon module).
- `daemon/src/projections/mod.rs` — `mod usage;` + `Box::new(usage::UsageLedgerProjector)` in `projectors()`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)

**L1 — `shared/tests/contract.rs` (or `shared/tests/harness.rs`):**
1. **`test_harness_normalized_type_field_names_snapshot`** — `expect_fields` on a fully-populated `TelemetrySample`, `TranscriptRef`, `ResumeResult`, `HarnessCapabilities`, `TelemetrySampled` (every Option = Some so the snapshot sees every key).
   - Asserts: exact field-name set per type == the checked-in freeze. Tag `spec(§9.1)`.
   - Why: §2.5-seam freeze guard (line 138 lists HarnessAdapter normalized types as a shared-contract model).
2. **`test_metric_quality_wire_values`** — `check_values(MetricQuality::ALL, &["exact","estimated","unavailable"])` + reject-unknown.
   - Why: §9.1/§7.2 `metric_quality` carried on all telemetry; the UI degrades off it (UsageMeter §11.7).
3. **`test_telemetry_sampled_wire_contract`** — round-trip + `deny_unknown_fields` rejects an extra key + `TelemetrySampled::EVENT_TYPE == "TelemetrySampled"` (the `AuditIntegrityViolation` precedent).
   - Why: §7.1 EventTypeRegistry single-home + §5.0/§15 reject-unknown.
4. **`test_normalized_status_is_session`** — assert `NormalizedStatus` resolves to `status::Session` (e.g. the 17-value set / a type-identity check), so the adapter status vocabulary is the frozen machine, not a fork.
   - Why: status.rs:45 — adapters map INTO `Session`; pin that no second status enum is introduced.
5. **`test_contract_version_bumped_0_20_0`** — `CONTRACT_VERSION == "0.20.0"`.
6. **`test_schema_artifact_matches_rust`** (existing test 9) — passes after regen (the byte-diff gate).
7. **`test_harness_capabilities_ten_fields`** — `expect_fields` pins exactly the 10 HARN-5 fields (count + names).
   - Why: Appendix A "HarnessCapabilities — 10 fields (PRD HARN-5)".

**L1 — `daemon/src/harness/mod.rs` tests:**
8. **`test_coverage_matrix_matches_spec`** — the const matrix == the §9.1 table verbatim: Claude direct=Guaranteed-in-default-mode, Claude subagent=Unsupported/NotGuaranteed, Claude background-subagent=Unsupported (#27203), Claude mcp=BestEffort; Codex direct=Guaranteed, Codex mcp=Guaranteed, Codex subagent=n/a.
   - Why: §9.1 binding coverage matrix + O-13 (safety #10) — the matrix is the contract the conformance suite asserts.
9. **`test_fake_harness_satisfies_trait`** — `Box<dyn HarnessAdapter>` constructs from `FakeHarness` + each method returns its normalized type (object-safety + satisfiability).

**L2 — `daemon/src/projections/usage.rs` tests** (synthetic `TelemetrySampled` envelopes, the `session.rs` test style):
10. **`test_usage_projector_folds_single_sample`** — one `TelemetrySampled` → one `proj_usage_ledger` row with the right dims + values.
11. **`test_usage_projector_accumulates_tokens_and_cost`** — two samples, same `(project,session,profile,model,day)` → `tokens_in`/`tokens_out`/`cost_estimate` SUM; `context_pct_max` = MAX of the two; `metric_quality` = worst.
12. **`test_usage_projector_buckets_by_day`** — two samples, different `occurred_at` UTC dates → two rows.
13. **`test_usage_projector_distinct_model_distinct_row`** — same session, different `model` → distinct `ledger_id` rows.
14. **`test_usage_projector_rebuild_idempotent`** — fold a stream, `rebuild()`, assert identical rollups.
15. **`test_usage_projector_ignores_other_event_types`** — a `SessionStarted` env → healthy no-op (no row, no degrade).
16. **`test_usage_projector_rejects_unbinding_payload`** — a malformed `TelemetrySampled` payload → `ProjectionError::Decode` (degrade-skip), reason carries NO payload bytes (§15).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW shared types (`TelemetrySample`, `MetricQuality`, `TranscriptRef`, `ResumeResult`, `HarnessCapabilities`, `TelemetrySampled` event) + `CONTRACT_VERSION` 0.19.0→0.20.0.
- **Orchestrator doc rows to write hot (Step 9 routing):**
  - `daemon/CLAUDE.md` cross-doc table + `ARCHITECTURE.md` Appendix A rows 569–571 — these rows **already exist** (the §9.1 surface was pre-specified); update them with the **AS-BUILT** detail: the realized field names, the `metric_quality` enum values, `TelemetrySampled` added to the EventTypeRegistry row (566), and the `CONTRACT_VERSION → 0.20.0` stamp.
  - `ARCHITECTURE.md §7.1` / §9.1 prose: an **architecture-doc note** that §9.1 telemetry is recorded as a **non-mutation observation event** (`TelemetrySampled`) written via the single write-actor — NOT routed through the Action Gateway, because INV-SEC-1 governs FS/git/external/**session-state mutations** and a telemetry observation is none of those (precedent: the System-actor lifecycle events `DeviceRegistered`/`AuditIntegrityViolation`). (Orchestrator-written; flagged to lead in dispatch.)
- **§2.5-seam (shared-contract) model touched? YES** — HarnessAdapter normalized types are on the line-138 list. The RED outline includes the schema-snapshot tests (1, 3, 7) authored this cycle.

## Things to flag at Step 2.5
1. **Telemetry-event emission path + INV-SEC-1 (the load-bearing one).** `TelemetrySampled` is written via the daemon's single write-actor, **not** through the Action Gateway. **My default vote: non-Gateway System/`session_adapter`-actor observation event** — INV-SEC-1 (#1) governs *state mutations* (FS/git/external/session-state); a telemetry observation mutates none, so it follows the existing non-mutation-event precedent (`DeviceRegistered`/`AuditIntegrityViolation` bypass the Gateway but still go through the single write-actor + the §15 redactor). It is **NOT** an exception to single-mutator (#2 — "only the daemon writes the DB" still holds; "only the Gateway executes *mutations*" still holds because this isn't a mutation). The projector reads only already-redacted events (#3 holds). **Confirm this framing** before GREEN — if you or `security-reviewer` read it as a Gateway-bypass risk, stop and re-escalate. (Emission itself is 3.2/3.3; this slice only defines the event + folds synthetic ones.)
2. **`NormalizedStatus` representation.** Re-export `pub use crate::status::Session as NormalizedStatus`, or just reference `Session` in the trait signature? My default vote: **a `NormalizedStatus` type alias/re-export in `harness.rs`** so the §9.1 vocabulary reads as named, with a test pinning it == `Session` (no forked enum). Rationale: matches the doc language without duplicating the value set.
3. **`ledger_id` composite-key construction.** A delimited string (`project|session|profile|model|day`) vs a one-way hash of the tuple. My default vote: **a deterministic delimited composite** (readable, debuggable, collision-free over ULID dims; the idempotency-key SHA-256 precedent is a *security* construct, not needed for a rollup key). Handle `NULL` dims with an explicit sentinel so `NULL` model and a literal "null" model don't collide.
4. **`metric_quality` rollup combination.** Worst-quality-wins (`exact` > `estimated` > `unavailable`) vs last-write. My default vote: **worst-wins** — a bucket containing any estimated sample is estimated (the UsageMeter must not show "exact" over partially-estimated data; §11.7).
5. **Which normalized types are `shared/` vs daemon-only.** My default vote: `TelemetrySample`, `MetricQuality`, `TranscriptRef`, `ResumeResult`, `HarnessCapabilities` → `shared/` (UI/Brain-consumable, schema-pinned); `MutationIntercept` (carries the non-serializable `decision_sink`) + the `HarnessAdapter` trait + the coverage-matrix enum → **daemon-only** (`harness/mod.rs`). The coverage matrix stays daemon-side (the conformance suite + docs are its consumers; the UI degrades off `HarnessCapabilities`, not the matrix) — flag if you think the UI needs a shared matrix representation.
6. **`cost`/`cost_estimate` naming.** §9.1 says `cost`; the DDL column is `cost_estimate`. My default vote: field name **`cost_estimate`** on both `TelemetrySample` and the event, matching the DDL + the "estimate" honesty (cost is derived, not authoritative). Note the §9.1 prose says `cost` — a benign naming reconcile (architecture-doc note).

## Dependencies + sequencing
- **Depends on:** Phase 2 (the EventTypeRegistry + projector engine + `ContractBundle` freeze mechanism — all landed); the `proj_usage_ledger` DDL (M3, landed 1.2). No 0.1/0.3 HITL gate — those gate 3.2/3.3 drive-mode, not this contract.
- **Blocks:** 3.2 (Claude adapter — implements the trait, emits `TelemetrySampled`), 3.3 (Codex adapter — same), 3.4 (terminal — binds to the session host). The ui track's UsageLedger/usage-shape reconcile (Carry-forward P6.4b) consumes the frozen `TelemetrySample`/`MetricQuality`/`HarnessCapabilities` at its resume — **flag the freeze to the lead for the ui track.**

## Estimated commit count
**2.** **L1** = the §9.1 contract freeze (one atomic freeze + one `CONTRACT_VERSION` bump — the seam freezes as a unit, never half). **L2** = the `proj_usage_ledger` projector (deterministic event-fold, no contract change). Not bundled into one commit: L1 is a §2.5-seam contract freeze (cross-doc-invariant + snapshot traceability wants its own commit) and L2 is independent projector logic with its own caller base. Neither ships interception *behavior*, so no safety-pin atomicity split is forced — but **`security-reviewer` runs on L1** (the mutation-interception + capability + telemetry-event contract surface; safety #9/#10 adjacency). `code-quality-reviewer` per the every-slice policy.

## Lessons-logged candidates anticipated
- **Architecture-doc note** — §9.1 telemetry as a non-mutation observation event written via the write-actor (not the Gateway); the §9.1 `cost` → `cost_estimate` naming reconcile.
- **Convention candidate** — "telemetry/usage is event-sourced as an observation event, not a Gateway action; observation events go System/adapter-actor → single write-actor → redactor, never the Gateway pipeline."
- **Future TODO — operational** — the FTS-per-event size-degradation already carried (Carry-forward); watch whether the usage projector adds per-event cost at scale (it's a simple upsert, should be cheap).
- **Future TODO — phase** — the real `TelemetrySampled` emission path + delta-vs-cumulative computation lands in 3.2/3.3 (the adapter must emit per-heartbeat token *deltas* so the SUM is correct, not cumulative snapshots — record this as a 3.2/3.3 acceptance pin).

## How to invoke
1. **Read this brief end-to-end** — especially Step-2.5 Q1 (the telemetry/INV-SEC-1 framing) before writing any test.
2. **Run `/tdd harness_adapter_contract_and_usage_ledger`** in the implementer session.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line (two layers: §9.1 freeze + usage projector).
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5 (test review pause)** — send the test-design write-up with answers to the 6 design questions (Q1 is load-bearing — do not take it silently). Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` on L1 (invariant-adjacent), `code-quality-reviewer` per policy.
7. **Step 9 (summarize)** — surface the cross-doc rows (Appendix A 569–571 + the EventTypeRegistry row 566 + the §9.1/§7.1 telemetry-event arch-note + `CONTRACT_VERSION`→0.20.0) for the orchestrator to write hot.

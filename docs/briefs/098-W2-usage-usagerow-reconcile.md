# /tdd brief — usage_ledger_usagerow_reconcile (WAVE-2 projection-honesty)

## Feature
Freeze `UsageRow` as the 6th typed `shared/` projection-row for `proj_usage_ledger`, served typed/fail-closed (the W2-audit `AuditEventRow` precedent) — closing the UsageLedger provisional→generated reconcile so the cockpit Usage tile consumes the REAL tokens/cost/context rollup instead of a Mock. **Plus** address the user-flagged **`creditPool` Mock-vs-real gap** (the UI Mock fakes a `creditPool` the daemon's projection page never serves) — but `creditPool` has no daemon source today, so its disposition is a Step-2.5 design fork (serve-real vs honest-omit), NOT assumed. WAVE-2.

## Use case + traceability
- **Task ID:** W2-usage
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (projections / `proj_usage_ledger`), `§6.1` (typed `get_projection(UsageLedger)` serve), `§5.0` (Rust-authority contract → schema → Zod, the §2.5-seam), `§9.1` (the `TelemetrySample` that feeds the ledger).
- **Related context:** `daemon/src/projections/usage.rs` (the `UsageProjector` — SUMs tokens/cost, MAXes context_pct from `TelemetrySampled`; LESSONS §23/§27) · `daemon/src/eventstore/schema.rs:253` (`proj_usage_ledger` DDL: `ledger_id` PK, project_id, session_id, execution_profile_id, model, bucket_day, tokens_in, tokens_out, context_pct_max, cost_estimate, metric_quality, updated_at_seq) · the **W2-audit precedent** (`9f7e396`): `AuditEventRow` frozen typed/fail-closed via `read_audit_typed` + the `AUDIT_ROW_WIRE_FIELDS` retain-whitelist + the §37 fail-closed serve · `shared/src/harness.rs` `MetricQuality`(exact|estimated|unavailable) — ALREADY a frozen enum (CONTRACT 0.20.0) · `shared/src/projections.rs` (5 frozen rows today: ApprovalQueue/PullRequest/Session/Review/Audit). **`UsageLedger` already has a `ProjectionName` variant** (served loose today via the generic `read_table_as_json`) → NO new `ProjectionName`, NO migration (all columns exist; this is a freeze+typed-serve, NOT an enrichment).
- **Next number:** `CONTRACT_VERSION` 0.49.0→**0.50.0**. NO migration (no new column for the UsageRow freeze; only if Step-2.5 rules `creditPool` needs one).

## Acceptance criteria (what "done" means)
- [ ] **Freeze `UsageRow`** in `shared/src/projections.rs` (the 6th typed row; `deny_unknown_fields`) — field set Step-2.5-locked, default the proj_usage_ledger columns minus the internal `updated_at_seq`: `{ledger_id: String, project_id: Option<String>, session_id: Option<String>, execution_profile_id: Option<String>, model: Option<String>, bucket_day: Option<String>, tokens_in: Option<i64>, tokens_out: Option<i64>, context_pct_max: Option<f64>, cost_estimate: Option<f64>, metric_quality: Option<…>}`.
- [ ] **Serve typed/fail-closed:** `get_projection(UsageLedger)` branches to a new `read_usage_typed` (the `read_audit_typed`/`read_session_typed` precedent; a `USAGE_ROW_WIRE_FIELDS` retain-whitelist drops `updated_at_seq`; a corrupt/mis-typed row fails the read closed — the LESSONS §37 pattern; the UI's degradable Usage tile handles a read error via its honest-banner).
- [ ] **CONTRACT 0.49.0→0.50.0** + the §2.5-seam schema snapshot + the 3-way verify (off-loop /phase-exit, LESSONS §29). **Paired UI regen REQUIRED (LESSONS §69)** — coordinated at Step-2.5; the lead push-gates on ui-regen-green. **Sequencing:** the ui-usage regen lands AFTER ui-090 (the 0.49 AuditEventRow regen) — the orchestrator sequences it so the UI contract version steps 0.49→0.50 cleanly.
- [ ] **`creditPool` — Step-2.5 disposition (see the fork).** Do NOT invent a synthetic value. Either (a) the daemon has NO real credit source → the honest fix is the UI drops the fake (shows "unavailable"/omits it) + record `creditPool` as a deferred daemon concern; or (b) a real source exists (the profile `usage_policy_json` budget − consumed `cost_estimate`) → fold/serve it (likely its own column + migration). Default = (a) honest-omit unless a real source is confirmed.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`get_projection(UsageLedger)` (`daemon/src/ipc/methods.rs`) → the new `read_usage_typed` (alongside `read_audit_typed`/`read_session_typed`/`read_review_typed`). The `UsageProjector` fold path is already production-reachable (folds `TelemetrySampled` on every such event-commit txn; `/wired` the UsageLedger serve branch). No new `ProjectionName`.

## Files expected to touch
**Modified:** `shared/src/projections.rs` (`UsageRow`) · `daemon/src/ipc/methods.rs` (`USAGE_ROW_WIRE_FIELDS` + `read_usage_typed` + the UsageLedger branch + the drift unit test) · `shared/src/lib.rs` (CONTRACT 0.50.0) · `shared/contracts/schema/` (regen) · `shared/tests/contract.rs` (the UsageRow snapshot @0.50.0 + reject-unknown) · `daemon/tests/projections.rs` (typed-serve round-trip + fail-closed). **New:** none (unless Step-2.5 (b) adds a creditPool column + migration).

## RED test outline (Step 2)
1. **`read_usage_typed_round_trips`** — a populated `proj_usage_ledger` serves `Vec<UsageRow>` typed; `updated_at_seq` dropped by the whitelist. Why: §6.1 typed serve.
2. **`read_usage_typed_fails_closed_on_corrupt_row`** — a mis-typed numeric (e.g. a non-numeric `cost_estimate`) → the read fails closed (`InternalError`), no loose leak. Why: LESSONS §37.
3. **`usage_row_wire_fields_match_struct`** (methods unit) — `USAGE_ROW_WIRE_FIELDS` == UsageRow field set (drift guard; the AuditEventRow/SessionRow precedent).
4. **`test_usage_row_frozen_shape`** (shared) — the 6th frozen row field set + round-trip + reject-unknown @0.50.0. Why: §5.0/§15.
5. **(if Step-2.5 (b))** a creditPool fold/serve test; **(if (a))** no daemon test — the honest-omit is a UI change (record the deferral).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **NEW frozen row `UsageRow`** → the orchestrator writes the daemon/CLAUDE.md "MVP projections" row + the ARCHITECTURE.md frozen-projection-row table (the 6th entry) + the CONTRACT bump.
- **CONTRACT 0.49.0→0.50.0 → paired UI regen (LESSONS §69)** — sequenced AFTER ui-090; the lead push-gates on ui-regen-green.
- **`creditPool` disposition** — if (a) honest-omit: a cross-track note (the UI drops the fake; the daemon-side credit-pool source is a deferred concern, possibly user-facing). If (b): a new column + migration + the fold.

## Things to flag at Step 2.5
1. **`creditPool` fork (the load-bearing one).** Investigate whether a REAL credit-pool source exists daemon-side: the `ExecutionProfile.usage_policy_json` budget (a configured pool?) minus consumed `cost_estimate`, and/or the SDK `credit_exhausted` signal. If NO real source → default **(a) honest-omit** (the UI stops faking it; flag the daemon credit-pool source as a deferred — likely a user/lead question about whether the SDK pool is even daemon-observable). If a real source exists → **(b)** fold a real `credit_pool`/`credit_remaining` (its own column + migration). Do NOT synthesize. Flag your finding — I route the (a)-honest-omit cross-track + escalate the source question if it's a product call.
2. **`metric_quality` typing** — bind the already-frozen `MetricQuality` enum (exact|estimated|unavailable; clean, no new enum into the verify) vs a plain `String` (the W2-audit degradable-tile precedent). Default: **bind `MetricQuality`** (it's a fixed 3-value enum already frozen at 0.20.0 — unlikely to grow; typed is better here). Flag if a forward-compat concern surfaces.
3. **Numeric typing** — `tokens_in`/`tokens_out` as `Option<i64>` (the projector binds i64), `context_pct_max`/`cost_estimate` as `Option<f64>` (REAL columns). Confirm the SQLite-REAL→JSON-number→`Option<f64>` serve needs no coercion (contrast the LESSONS §53 bool case).
4. **All-Option vs some-non-Option** — the DDL has every data column nullable except `ledger_id`/`updated_at_seq`. Default: `ledger_id` non-Option, the rest `Option` (DDL-match, the SessionRow nullability-match precedent).

## Dependencies + sequencing
- **Depends on:** the `UsageProjector` + `TelemetrySample`/`MetricQuality` (frozen 0.20.0) + the typed-row serve pattern (W2-audit `read_audit_typed`). All landed.
- **Sequencing:** ui-090 (0.49 AuditEventRow regen) lands FIRST; the paired ui-usage regen (0.50) follows — clean version stepping. The lead push-gates each round on ui-regen-green.
- **Then:** W2-plan-team (Plan/Team projectors — no projector registered today → empty views), then WAVE-3.

## Estimated commit count
**1** daemon+shared commit (the LESSONS §53 LOCKSTEP for a frozen typed row: row + typed-serve + CONTRACT bump + snapshot atomic). NO migration for the UsageRow freeze (columns exist) unless Step-2.5 (b) adds a creditPool column. `code-quality-reviewer` per `every-slice`; **security-reviewer NOT mandatory** (usage data = non-secret rollups; no INV-SEC surface) — run only if Step-2.5 (b)'s credit source touches §15. The paired UI regen is a separate ui-track commit.

## Lessons-logged candidates anticipated
- Likely NONE NEW (composes LESSONS §37 typed-fail-closed-row + §69 contract-bump-pairs-UI-regen + the W2-audit AuditEventRow precedent). Flag if the creditPool fork surfaces a reusable "don't synthesize a value the daemon can't back — honest-omit over fake" principle worth banking.

## How to invoke
1. Read this brief end-to-end (esp. Step-2.5 Q1 — the `creditPool` real-source-vs-honest-omit fork).
2. Run `/tdd usage_ledger_usagerow_reconcile`.
3. Step 2.5 — ping back with the locked UsageRow shape + your creditPool finding (I route the cross-track + escalate the source question if needed; I pre-stage the ui regen sequenced after ui-090).
4. Step 9 — flag the `UsageRow` cross-doc freeze + the CONTRACT bump + the creditPool disposition.

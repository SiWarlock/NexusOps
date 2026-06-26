# /tdd brief — W2_usage_usagerow_reconcile (the W2-usage UI half)

## Feature
Reconcile the provisional `UsageRow` shadow to the daemon-frozen 11-field shape (CONTRACT 0.50.0),
reconcile the Usage consumer (dashboard + view-model) to the new fields, and **drop the faked
`creditPool`** (lead-ruled HONEST-OMIT — the daemon has no credit-balance source) so the Usage tile
renders the REAL usage projection and the credit pool is honestly absent.

## Use case + traceability
- **Task ID:** W2-usage (the UI half — pairs with the daemon `W2-usage` #27 / CONTRACT 0.50.0).
- **Architecture sections it implements:** `ARCHITECTURE.md §7` (projections / §7.2 usage), `§5.0`
  (contract SoT + the regen drift gate), `§11.4` (usage/credit display), `§11.7` (honest degrade —
  never a fabricated value/meter), `§9.1` (`metric_quality` / `supportsContextMetadata`).
- **Related context:** LESSON [[24]] (frozen projection-row → `.strict()` shadow drift-pinned to
  `shared/src/projections.rs`; the AuditEventRow/ApprovalQueueRow precedents), [[44]] (daemon-rendered
  field as forward-compat plain string), [[38]] (ui-079 per-projection resilience — the Usage tile's
  honest degraded banner this slice un-degrades), [[15]] (the CreditPool `kind` required discriminator
  gating `hard_stop` — its SUBJECT is removed by this slice), [[11]] (never a silent/false safety state).
  Daemon side = daemon `W2-usage` (#27); daemon LESSONS §69 (paired UI regen).

### 🔒 LOCKED `UsageRow` shape (daemon-frozen at #27 Step-2.5; CONTRACT 0.49.0→0.50.0; relayed)
The 6th frozen typed projection row (`deny_unknown_fields` → the UI shadow is `.strict()`):
```
{
  ledger_id: string,
  project_id: string | null,
  session_id: string | null,
  execution_profile_id: string | null,
  model: string | null,
  bucket_day: string | null,
  tokens_in: number | null,
  tokens_out: number | null,
  context_pct_max: number | null,
  cost_estimate: number | null,
  metric_quality: ("exact"|"estimated"|"unavailable") | null   // BIND to the frozen MetricQuality enum (nullable)
}
```
**`metric_quality` IS the frozen `MetricQuality` enum → bind it** (`bundle.shape.MetricQuality`, nullable).
The rest are plain `z.string().nullable()` / `z.number().nullable()` — NOT enums. The old provisional
`{subject_id, harness, tokens, cost, context_pct}` is fully replaced: `subject_id`→`ledger_id` (PK) + the
`project_id`/`session_id`/`execution_profile_id`/`model` dimensions; `harness`→`model`; `tokens`→`tokens_in`+`tokens_out`;
`cost`→`cost_estimate`; `context_pct`→`context_pct_max`.

### ⚠️ creditPool = HONEST-OMIT (lead-ruled)
The daemon has **NO credit-balance source** (`usage_policy_json` opaque, `credit_exhausted` is binary, the
SDK pool isn't telemetry-observable) → `UsageRow` carries **NO creditPool**. In REAL data `creditPool`
was already `null` (the meter only ever rendered from the **Mock fixture** — the 4th Mock-vs-real gap). So:
**remove `UsageProjectionPage.creditPool` + the `CreditPool` type + `creditPoolState`/`CreditPoolState`/
`CreditPoolKind` (model.ts) + the credit-pool meter section (UsageDashboard.tsx:120-149) + the Mock fake +
every other `creditPool` consumer site.** The §11.4 SDK `hard_stop` display is removed (it was a Mock-only,
potentially-FALSE safety signal — exactly the risk [[15]] warned of; honest-omit RESOLVES it, not regresses).
There is an open product question (is a real pool EVER daemon-observable) routed to the user; for now the
honest fix is dropping the fake.

## Acceptance criteria (what "done" means)
- [ ] **Regen** (gated — see Dependencies): `generated.ts` → `CONTRACT_VERSION` 0.49→**0.50.0** (drift test green; **value-set HELD at 42** — `metric_quality` reuses the existing `MetricQuality` enum, no new `$def`).
- [ ] `UsageRow` shadow reconciled to the frozen 11-field shape above, **`.strict()`**, `metric_quality` bound to `MetricQuality` (nullable), the rest plain nullable string/number; **drift-pinned** to the frozen daemon source (schema `$defs.UsageRow` per the ui-090 `profile_row`/`AuditEventRow` precedent, else `projections.rs`).
- [ ] `creditPool` FULLY removed: the `CreditPool` Zod type, `UsageProjectionPage.creditPool`, `model.ts` `creditPoolState`/`CreditPoolState`/`CreditPoolKind`, the UsageDashboard meter section, the Mock fixture's `creditPool`, + every other consumer site (Shell/Settings/CommandCenter — tsc-forced). No reference to `CreditPool`/`creditPool`/`hard_stop`-from-pool remains (tsc + grep clean).
- [ ] `UsageDashboard.tsx` + `model.ts` reconciled: `buildUsageRows` maps the new fields; `isContextUnknown(row)` = `row.context_pct_max == null` (harness gone — the daemon serves null for no-context-metadata, §9.1); the stat cards (spend = Σ`cost_estimate`, tokens = Σ(`tokens_in`+`tokens_out`) — the real split, retire the "split lands later" sub); the table columns (display-label / model / tokens / cost / context / accuracy); the context-consumers filter on `context_pct_max != null`.
- [ ] A null `metric_quality` renders honestly (Step-2.5 Q3) — never a crash / blank accuracy.
- [ ] `proj_usage.ts` fixtures → the frozen 11-field shape (NO creditPool); the boundary `parseProjectionPage("UsageLedger")` accepts the frozen row + rejects an unknown key; **the Usage tile no longer degrades**.
- [ ] `/preflight` clean. Cross-doc invariant (the `UsageRow` 0.50 row + the creditPool removal) flagged at Step 9.

## Wiring / entry point (Step 7.5)
The Usage dashboard is mounted in the Shell (Settings → Usage tab; fed by the existing
`get_projection("UsageLedger")` settle-load + the ui-079 per-projection wrapper). This slice changes the
SHAPE the wired path parses + renders (and removes the creditPool prop threaded Shell→UsageDashboard); no
new entry point. The un-degrade is reached when the reconciled shadow matches the daemon's 0.50 served shape.

## Files expected to touch
**Modified:** `src/contracts/generated.ts` (regen → 0.50, gated) · `src/contracts/provisional.ts` (UsageRow → frozen 11-field; DELETE CreditPool + UsageProjectionPage.creditPool) · `src/contracts/provisional.test.ts` (UsageRow drift-pin; delete the CreditPool tests) · `src/views/usage/model.ts` (buildUsageRows reconcile; DELETE creditPoolState/CreditPoolState/CreditPoolKind) · `src/views/usage/model.test.ts` · `src/views/usage/UsageDashboard.tsx` (DELETE the meter section; reconcile cards/table/context) · `src/views/usage/UsageDashboard.test.tsx` · `src/projections/fixtures/proj_usage.ts` (→ 11-field, no creditPool) · `src/gateway-client/boundary.test.ts` · `src/shell/Shell.tsx` + `src/views/settings/Settings.tsx` + `src/views/command/CommandCenter.tsx` (+ their tests) — the other `creditPool` consumer sites (tsc-forced; the rename/removal surfaces them, as ui-090's CommandCenter/EventDock did).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
`src/contracts/provisional.test.ts`:
1. **`usage_row_frozen_11_field_strict_shadow`** — the 11-field `.strict()` shadow accepts a valid row, REJECTS an unknown key, the nullable fields accept null, `metric_quality` enum-validated (+ accepts null). Why: [[24]] + the daemon `deny_unknown_fields`.
2. **`usage_row_field_set_matches_frozen_schema`** — field-set drift-pin vs the frozen `$defs.UsageRow` (the ui-090 pattern). Why: §5.0.
3. **`credit_pool_type_removed`** — `CreditPool` is no longer exported / `UsageProjectionPage` has no `creditPool` (a compile-time + a `.strict()`-rejects-creditPool assertion). Why: the honest-omit.

`src/views/usage/model.test.ts`:
4. **`build_usage_rows_maps_frozen_fields`** — `buildUsageRows` maps ledger_id/model/tokens_in+out/cost_estimate/context_pct_max; `context_pct_max == null` → context "unknown" (§9.1, forbidden #4); `metric_quality === "unavailable"` (and null, Q3) → value "unknown".
5. **`credit_pool_state_removed`** — `creditPoolState`/`CreditPoolState` no longer exported (compile-time). Why: the hard_stop gating removal.

`src/views/usage/UsageDashboard.test.tsx`:
6. **`renders_no_credit_pool_meter`** — no `credit-pool-state` element renders (the meter section gone); an honest credit-unavailable note IF Q4 chooses one. Why: honest-omit ([[11]]/[[38]]).
7. **`stat_cards_and_table_use_frozen_fields`** — spend = Σ cost_estimate, tokens = Σ(in+out); the table renders the new columns; context-consumers filter on context_pct_max. Why: the consumer reconcile.

`boundary`:
8. **`usage_ledger_page_accepts_frozen_row_rejects_unknown`** — accepts the 11-field row, rejects unknown (parse-don't-trust [[22]], un-degrade).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `UsageRow` 6→11 fields (full reshape); `CreditPool` + `UsageProjectionPage.creditPool` REMOVED. **CONTRACT 0.49.0→0.50.0.** The provisional `Harness` enum: **dropped from UsageRow** (→ `model`); **keep the `Harness` enum IF still consumed** (e.g. `SessionRow.harness`) — tsc-check, don't remove a live enum.
- **Orchestrator doc rows to write hot:** the `ui/CLAUDE.md` "Generated Zod contract layer" 0.50 regen note (value-set HELD 42; UsageRow the 6th frozen row; creditPool removed) + the `UsageRow` row. I write hot.
- **§2.5-seam:** `UsageRow` is a `projections.rs` frozen row → the RED outline INCLUDES the field-set drift-pin (test 2).

## Things to flag at Step 2.5
1. **Per-row display label** — what replaces `subject_id`? The frozen row has `ledger_id` (PK) + `model`/`project_id`/`session_id`/`execution_profile_id`. Default vote: **`model` as the primary label** (the most meaningful per-row dimension for a usage breakdown) with a `ledger_id` fallback when `model` is null; keep `ledger_id` as the React key + `data-item-id`. (Or a `project · model` combo — flag if you prefer.)
2. **tokens_in / tokens_out display** — separate columns vs a combined "in→out" cell vs a sum. Default vote: **one Tokens cell `Σ(in+out)` in the table + the real split surfaced in the "Tokens today" stat-card sub** (retire the "split lands later" note) — keeps the table compact, honors the daemon split. Flag if you want explicit in/out columns.
3. **null `metric_quality`** — the accuracy label + the unavailable-value logic key on it; null isn't a `MetricQuality`. Default vote: **null → treat as `"unavailable"`** (the honest degrade — render "unavailable" accuracy + "unknown" values), via a null-coalescing helper (NOT a forced default that hides genuine absence). Flag.
4. **creditPool-omit UX** — a brief honest "Credit balance not available (not reported by the daemon)" note vs a silent omit. Default vote: **a brief honest note** (§11.7 — explain the absence; it's a deliberate product omit, not a load error) rather than silently dropping the section. Flag if the lead/product prefers a silent omit.
5. **The §11.4 hard_stop removal is safety-RELEVANT** — it removes a (Mock-only, daemon-unobservable, potentially-FALSE) safety-state display. It is a REMOVAL of a fake, lead-ruled honest-omit — NOT authoring a new invariant. Consider isolating the creditPool-drop in its OWN commit (see Estimated commit count) for review clarity; flag if you read it differently.

## Dependencies + sequencing
- **GATED on the daemon `W2-usage` (#27) sealing CONTRACT 0.50.0** — the regen source (`nexusops-contract.schema.json@0.50.0`) + the drift-pin target (`projections.rs` `UsageRow`) land with that seal. The daemon-orch pings the commit hash. **Do NOT dispatch until that seal.** **Pushes are USER-GATED now** (the user is back, live-validating; the lead relays push OK) — so this round commits LOCAL, the lead pushes on the user's OK.
- **Blocks:** nothing. The real usage live-producer (`TelemetrySampled` ingress) stays daemon-P4-dormant ([[29]]) — the tile connects + renders the served rows (empty until the producer lands); out of scope here. The "is a real credit pool ever observable" product question is the user's, tracked in handoff 010.

## Estimated commit count
**1–2.** The 0.50 regen forces the whole `UsageProjectionPage` reconcile together, so a single cohesive
commit is defensible. BUT the creditPool honest-omit removes a §11.4 safety-state display — consider
**2 commits**: (A) the UsageRow shape reconcile + regen + the row consumer; (B) the creditPool honest-omit
(the meter + `creditPoolState` + the Mock fake removal) — isolating the safety-relevant removal for a
focused code-quality review. Implementer's call at Step-2.5. code-quality reviewer (every-slice); no
security-reviewer trigger (no §15 invariant authored — removing a fake; the daemon owns the real serve).

## Lessons-logged candidates anticipated
- **Cross-doc invariant change** — `UsageRow` 6→11 + creditPool removal, 0.49→0.50 (orchestrator writes hot).
- **Convention candidate** — honest-OMIT a display the daemon cannot source (vs faking it from the Mock): delete the field + the consumer + the Mock fake, render an honest "not reported" note; a Mock-faked field that's null in real data is a Mock-vs-real gap → resolve by deletion, not by a null-guard that hides the gap. Extends [[11]]/[[38]].
- **Architecture-doc note candidate** — §7.2/§11.4: `proj_usage_ledger` serves `UsageRow` (per-ledger tokens_in/out + cost_estimate + context_pct_max + metric_quality, dimensioned by project/session/profile/model/day); NO credit-pool (not daemon-observable).

## How to invoke
1. **Read this brief end-to-end** — the LOCKED shape + the creditPool HONEST-OMIT + Step-2.5 Q1–Q5.
2. **Run `/tdd W2_usage_usagerow_reconcile`** AFTER the daemon 0.50 seal (the regen needs the frozen schema + the `projections.rs` drift target).
3. **Step 2.5** — ping the test-design write-up + the Q1–Q5 decisions (esp. Q5 the safety-relevant creditPool removal) before GREEN.
4. **Step 9** — flag the CONTRACT 0.49→0.50 cross-doc invariant + the creditPool removal + the honest-omit convention.

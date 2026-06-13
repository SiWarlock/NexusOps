# /tdd brief — two_pool_credit_pool_reconcile

## Feature
Reconcile the ui credit-pool model to the **CONFIRMED** §11.4/§9.1 **two-pool
semantics** — the **SDK/`-p` pool** (a separate capped *monthly* pool that
**HARD-STOPS** with no fallback) vs the **interactive pool** (auto-resetting
rolling-window limits, **exempt** from hard-stop). Make `creditPoolState`
**pool-kind-aware** so `hard_stop` is reachable **only** for the SDK pool; the
interactive pool's exhaustion is the recoverable rolling-window signal, **never**
a hard-stop. The existing SDK credit-pool meter (the only pool the daemon surfaces
today) renders unchanged; the interactive-pool awareness is baked into the model
for when the daemon surfaces rate-limit data.

> **The semantics are settled, not open** — the daemon's June-15-2026 billing
> verification confirmed the split (`ARCHITECTURE.md §9.1` AS-BUILT, the
> PTY-PRIMARY rationale: "a control plane cannot let a supervised run die on the
> capped pool"). This slice encodes that confirmed asymmetry into the ui's
> provisional credit-pool model. Pure model logic → fully `/tdd`-able.

## Use case + traceability
- **Task ID:** P6.4b
- **Architecture sections it implements:** `ARCHITECTURE.md §11.4` (the Agent-SDK
  credit-pool meter — near-exhaustion + hard-stop, distinct from token spend),
  `§9.1` (the credit-pool two-pool billing semantics; PTY-PRIMARY), `§11.7`
  (honest degradation).
- **Widens phase scope because** the §11.4 credit-pool meter (a Phase-6 Usage
  surface) consumes the **§9.1** harness credit-pool/billing semantics — the
  SDK-vs-interactive two-pool split confirmed by the daemon's June-15 billing
  verification. Citing §9.1 from Phase 6 is the same cross-phase
  contract-consumption widen as the 040/041 §5.0/§6.4 adoption. *(The `§2`/`§2.5`/`§13`
  tokens elsewhere in this brief are `ui/LESSONS.md` / template references — not
  architecture anchors.)*
- **Related context:** `ARCHITECTURE.md §9.1` AS-BUILT (the June-15 billing split:
  SDK/`-p` hard-stops, interactive auto-resets); the Carry-forward Usage spread
  ("the credit-pool meter thresholds reconcile when the daemon defines credit-pool
  semantics"); `ui/src/views/usage/model.ts` (the current **single-pool**
  `creditPoolState`); `ui/LESSONS.md §11` (degraded/safety states fail-closed,
  never silent/color-alone) + `§13` (UI logic over frozen/local state — no daemon
  dep, no intent gate).

## Acceptance criteria (what "done" means)
- [ ] `CreditPool` provisional gains a **pool-kind discriminator** (Q1 default:
      `kind: "sdk" | "interactive"`) — banner-marked provisional (§11.4 pins no
      shape; Lesson §2). No frozen contract exists for credit-pool — it stays UI-local.
- [ ] `creditPoolState` is **kind-aware**: `kind="sdk"` + remaining ≤ 0 → `hard_stop`
      (the capped-monthly no-fallback semantics); `kind="interactive"` + remaining
      ≤ 0 → **NEVER `hard_stop`** — the recoverable rolling-window state (Q2 default:
      `near_exhaustion`).
- [ ] The `near_exhaustion` threshold (≤ 15% remaining) applies to **both** kinds;
      **only `hard_stop` is kind-gated** (SDK-only).
- [ ] The existing SDK credit-pool meter (`UsageDashboard`) renders **unchanged** —
      the fixture + the dashboard pass `kind="sdk"` (Q4: no visual reframe this slice).
- [ ] All `model.test.ts` credit-pool tests pass (existing retargeted to pass `kind`
      + the new kind-aware cases); **whole suite green** (217 → 217+N).
- [ ] `/preflight` clean (oxlint + tsc + test:run).
- [ ] Cross-doc invariant flagged at Step 9: the `CreditPool` provisional is now
      kind-discriminated + the §9.1 two-pool semantics are encoded; **MetricQuality
      is now frozen (0.23.0, `oneOf`-of-`const`) but the generator skips it** —
      route to a follow-up (orchestrator writes the row / carry-forward).

## Wiring / entry point (Step 7.5)
The kind-aware `creditPoolState` is consumed by **`UsageDashboard`** (already
wired — the credit-pool meter section). The **`"interactive"` kind is
exposed-ahead-of-consumer**: there is no daemon rate-limit data source yet (the
frozen `TelemetrySample` carries `{tokens_in, tokens_out, cost_estimate,
metric_quality, context_pct}` — **no `rate_limits`**), so the interactive-pool
meter UI is a future slice gated on the daemon surfacing `statusLine
rate_limits.{five_hour,seven_day}`. Flag at 7.5 as **expected, not a wiring miss**
— same exposed-ahead pattern as 040/041.

## Files expected to touch
**Modified:**
- `ui/src/contracts/provisional.ts` — `CreditPool` gains the `kind` discriminator (Q1).
- `ui/src/views/usage/model.ts` — `creditPoolState(used, limit, kind)` kind-aware
  (only `kind="sdk"` can reach `hard_stop`).
- `ui/src/views/usage/model.test.ts` — the kind-aware RED tests + retarget existing.
- `ui/src/views/usage/UsageDashboard.tsx` — pass `kind` (the existing meter →
  `kind="sdk"`); no visual change (Q4).
- `ui/src/projections/fixtures/proj_usage.ts` — `creditPool` fixture gains `kind: "sdk"`.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
Tests in `ui/src/views/usage/model.test.ts`:

1. **`credit_pool_sdk_exhaustion_is_hard_stop`** — `kind="sdk"`, remaining ≤ 0 → `hard_stop`.
   - Asserts: the SDK pool hard-stops at exhaustion.
   - Why: §9.1 — the capped monthly SDK/`-p` pool has no fallback.
2. **`credit_pool_interactive_exhaustion_is_never_hard_stop`** — `kind="interactive"`,
   remaining ≤ 0 → `near_exhaustion` (NOT `hard_stop`).
   - Asserts: the interactive pool never hard-stops (it auto-resets).
   - Why: §9.1 — interactive runs on auto-resetting rolling-window limits, exempt.
3. **`credit_pool_near_exhaustion_both_kinds`** — ≤ 15% remaining → `near_exhaustion`
   for both kinds.
   - Asserts: the near-exhaustion warning is kind-independent.
   - Why: §11.4 near-exhaustion threshold.
4. **`credit_pool_normal_both_kinds`** — > 15% remaining → `normal` for both.
   - Asserts: the baseline state.
   - Why: §11.4 the meter's non-warning band.
5. **(retarget)** the existing `creditPoolState` tests pass the `kind` arg (SDK).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `CreditPool` provisional gains `kind` (**UI-provisional** —
  no frozen contract; the daemon has not frozen credit-pool). **`MetricQuality` is
  now frozen at 0.23.0** (encoded `oneOf`-of-`const`, since its variants carry
  doc-comments) **but `gen-contracts.mjs` only emits flat `.enum` $defs → it is not
  generated.** Its provisional→generated reconcile is a **separate generator-extension
  follow-up**, NOT this slice.
- **Orchestrator doc rows to write hot (Step 9):** if a Usage cross-doc row exists in
  `ui/CLAUDE.md`, note the kind-discriminated credit-pool + the §9.1 two-pool
  semantics. No `ARCHITECTURE.md` edit (the §9.1/§11.4 spec already states the
  semantics; this slice consumes them).
- **§2.5-seam model touched?** **No** — `CreditPool` is UI-provisional, not a frozen
  daemon model crossing the §2.5 seam.

## Things to flag at Step 2.5
1. **The pool-kind shape.** (A) add `kind: "sdk" | "interactive"` to `CreditPool`;
   (B) two separate fields (`creditPool` SDK + a new `interactivePool`); (C) a
   `CreditPool[]` array of kinded pools. **My default vote: (A)** — minimal, the
   meter stays parameterized by one shape, and the interactive pool joins when daemon
   data exists. (B)/(C) are premature — there's no interactive-pool data source yet,
   so they'd add a second field/array for no consumer.
2. **The interactive pool's exhaustion state.** **Default: reuse `near_exhaustion`**
   (the honest "rolling window is high but auto-resets" signal) — never `hard_stop`.
   Alternative: a distinct `"rolling_reset"`/`"throttled"` state (more precise, but
   adds a `CreditPoolState` member + a render label + a glyph + the descriptor sweep,
   for no current consumer). **Default vote: reuse `near_exhaustion`** — the `kind`
   gates `hard_stop`, and `near_exhaustion` already means "watch this"; add a distinct
   state only if you judge the rolling-reset semantics need their own label NOW.
3. **MetricQuality reconcile.** It's frozen at 0.23.0 (`oneOf`-of-`const`) but the
   generator (`gen-contracts.mjs:38`) only handles `Array.isArray(def.enum)`. **Default:
   DEFER** — a separate "generator: `oneOf`-of-`const` support + reconcile
   MetricQuality" follow-up (carry-forward); do **NOT** bundle generator work into a
   §11.4 logic slice. **Optional in-slice (cheap):** a drift-pin test asserting the
   provisional `MetricQuality` members == the frozen schema's `MetricQuality` `oneOf`
   const set (so the provisional is drift-caught against the now-frozen def, **without**
   the generator change). Flag if you want that pin in 042.
4. **Rendering scope.** **Default: model + the deterministic VM only** — the SDK meter
   renders unchanged (`kind="sdk"`); the section's `aria-label="Agent-SDK credit pool"`
   stays accurate. No visual gate needed (no rendering change). Flip to "generalize the
   meter label/section now" only if you judge it shouldn't stay SDK-specific — but with
   no interactive-pool data, SDK-specific is the honest present state.

## Dependencies + sequencing
- **Depends on:** slice 041 (landed `63ebb1d` — 0.23.0). The §9.1 June-15 billing
  verification (confirmed the two-pool semantics). Nothing else.
- **Blocks:** the future **interactive-pool meter** (when the daemon surfaces
  `statusLine rate_limits`); the **MetricQuality reconcile** (separate generator
  follow-up). Neither is in this slice.

## Estimated commit count
**1.** A focused §11.4 credit-pool model reconcile (kind-aware `creditPoolState` +
the provisional shape + the fixture/dashboard `kind` wiring). **Not safety-critical**
— UI render-policy over a provisional shape; no mutation / INV-SEC-1 path (Lesson §13:
UI logic over frozen/local state, no daemon dep, no intent gate). **No
`security-reviewer`.** Pure model logic; bundles cleanly.

## Lessons-logged candidates anticipated
- **Convention candidate** — "credit-pool exhaustion semantics are **pool-kind-
  dependent**: `hard_stop` only for the capped/no-fallback SDK pool; the auto-resetting
  interactive pool never hard-stops" (extends §11 fail-closed + §13 UI-logic-over-state).
- **Architecture-doc note candidate** — the §9.1 two-pool billing semantics are now
  encoded in the ui credit-pool model; `MetricQuality` is frozen at 0.23.0
  (`oneOf`-of-`const`) but generator-pending.
- **Future TODO — next-brief working set** — (a) **generator `oneOf`-of-`const`
  support + MetricQuality reconcile** (the daemon froze MetricQuality but the ui can't
  yet consume it — a cross-track contract-consumption gap); (b) the **interactive-pool
  meter** when the daemon surfaces `rate_limits`; (c) the broader
  `UsageRow`/`CreditPool`/`Harness` provisional→generated reconcile when the daemon
  freezes the usage *projection* schema.

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" (4 design questions).
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd two_pool_credit_pool_reconcile`.**
4. Step 0 (Restate) — confirm against the Feature line.
5. Step 1 (Identify files) — confirm against "Files expected to touch".
6. **Step 2.5** — answer the 4 design questions (or take defaults) and send the
   test-design write-up; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
7. Step 9 — surface the cross-doc flags (the kind-discriminated credit-pool + the
   MetricQuality-frozen-but-generator-pending follow-up).

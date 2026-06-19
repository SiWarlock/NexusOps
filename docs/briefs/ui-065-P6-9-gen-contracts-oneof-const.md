# /tdd brief — gen_contracts_oneof_const (retire the drift-pinned shadows)

## Feature
Extend the ui contract generator (`ui/scripts/gen-contracts.mjs`) to emit `oneOf`-of-`const` `$def`s as Zod enums (today it handles flat `.enum` only), then **retire the 3 hand-declared drift-pinned provisional shadows** (`ResumeMode` / `RecoveryState` / `MetricQuality`) — they become real generated enums. A clean **representation swap** (the generated value-sets are VERIFIED identical to the shadows), removing the manual-shadow drift hazard. **CONTRACT-neutral, daemon-independent, NON-cat-1.**

## Verify-before-build findings (the lead's guardrail is CLEAR — read first)
Pre-orient mapped the generator, the schema, the shadows, the drift gate, and the consumers. The load-bearing equivalence check (the lead's guardrail: "if the generated output is NOT equivalent to the shadows → STOP + Finding"):

| Enum | Schema `oneOf`-const (`shared/contracts/schema/nexusops-contract.schema.json`) | Shadow (`provisional.ts`) | Verdict |
|---|---|---|---|
| **MetricQuality** | `exact / estimated / unavailable` (L1523-1542) | `exact / estimated / unavailable` (L310-313) | **IDENTICAL** |
| **ResumeMode** | `resumed / replayed / relaunched / reattached_live` (L2065-2089) | same, same order (L40-46) | **IDENTICAL** |
| **RecoveryState** | `recovering / recovered / recovery_failed` (L1944-1963) | same (L34-35) | **IDENTICAL** |

**→ CLEAN REPRESENTATION SWAP, NOT a Finding.** All 3 value-sets match exactly; retiring the shadows is lossless. Also verified: **CONTRACT-neutral** (`CONTRACT_VERSION` reads the schema's `x-contract-version` = `0.38.0`, which this slice does NOT touch — generator-only change; the emitted VALUES are unchanged); the §5.0 drift gate (`generated.test.ts`) **auto-derives `validators` from the bundle** (`= shape`), so it self-extends once the generator emits the 3 — but its `enumDefs` filter (L19, `Array.isArray(d.enum)`) must widen to include oneOf-const $defs; consumers all import from the unified `contracts/index` surface (no import-site churn — the re-export source moves, the surface doesn't).

## Use case + traceability
- **Task ID:** P6.9 (the `gen-contracts.mjs` oneOf-const generator extension — the long-carried carry-forward, origin 2026-06-14 053)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (the contract SoT mechanism — Rust authority → schemars JSON-Schema → generated Zod; the generator is the §5.0 ui-side codegen path)
- **Related context:** `ui/LESSONS.md` §1 (generated contract enums — never hand-declared) + §2 (provisional shapes — reconcile to generated on the next bump) + §14 (regen discipline — derive `validators` from the bundle, never hand-list); the `gen-contracts.mjs` enum-handling (L32-46); the schema oneOf-const $defs; the drift-pin tests (`provisional.test.ts` `metricquality_provisional_matches_frozen_schema` L129-143, `resume_mode_drift_pinned_to_schema_oneof_four_values` L305-313, `recovery_state_drift_pinned_to_schema_oneof` L315-321).

## Acceptance criteria (what "done" means)
- [ ] **Generator emits oneOf-const** — `gen-contracts.mjs` detects a `$def` with `oneOf: [{const}, …]` and emits it as a Zod enum (extract the `const` values → a synthetic flat enum for `json-schema-to-zod`, or the equivalent). `pnpm gen:contracts` regenerates `generated.ts` with `MetricQuality` / `ResumeMode` / `RecoveryState` present.
- [ ] **§5.0 drift gate widened + GREEN** — `generated.test.ts`'s `enumDefs` filter (L19) includes oneOf-const $defs; the member-set/drift test now auto-pins all 3 (the schema's const-set === the generated `.options`). `CONTRACT_VERSION` still `0.38.0` (NO bump).
- [ ] **Shadows retired** — the 3 `z.enum(...)` shadow declarations deleted from `provisional.ts`; their 3 drift-pin tests deleted from `provisional.test.ts` (now covered by the generated drift gate — NOT dropped coverage, MOVED to the authoritative gate). The named exports (`MetricQuality`/`ResumeMode`/`RecoveryState` — Zod schema + type) plumbed from the generated source via `contracts/index` so consumers resolve unchanged.
- [ ] **Consumers unchanged + green** — `views/usage/model.ts`, `recovery/model.ts`, `shell/Sidebar.tsx`, `recovery/RecoveryBanner.tsx` (+ tests) still import `{ResumeMode/RecoveryState/MetricQuality}` from `contracts/index` and typecheck/run unchanged.
- [ ] The full ui suite stays green, `tsc --noEmit` + `oxlint` clean, `/preflight` clean.
- [ ] **Guardrail (lead):** CONTRACT-neutral (NEVER bump `CONTRACT_VERSION`); the §5.0 drift gate GREEN. The equivalence is pre-verified clean — but if GREEN reveals any value-set divergence (a consumer needing real reconciliation, not a representation swap), **STOP + flag a Step-9 Finding** (do NOT paper over it).
- [ ] Cross-doc: none (the generated Zod layer is a drift-caught consumer; flag at Step 9 only if the generator approach surfaces a convention).

## Wiring / entry point (Step 7.5)
The generator is run via `pnpm gen:contracts` (`gen-contracts.mjs`) — the §5.0 ui codegen entry; `generated.ts` is the committed artifact the app imports through `contracts/index`. The 3 enums become reachable as generated named exports consumed by the existing `recovery`/`usage`/`Sidebar` surfaces (already wired — this swaps their source from the provisional shadow to the generated bundle). `/wired` target: the 3 enums' consumers (`recovery/model.ts` etc.) resolve from `contracts/index` (unchanged import; new source).

## Files expected to touch
**Modified:**
- `ui/scripts/gen-contracts.mjs` — the oneOf-const emission branch.
- `ui/src/contracts/generated.ts` — REGENERATED (never hand-edit — `pnpm gen:contracts`; the 3 enums appear; `CONTRACT_VERSION` unchanged).
- `ui/src/contracts/generated.test.ts` — widen the `enumDefs` filter to include oneOf-const (the const-extraction for the comparison).
- `ui/src/contracts/provisional.ts` — DELETE the 3 shadow declarations.
- `ui/src/contracts/provisional.test.ts` — DELETE the 3 drift-pin tests (coverage moves to the generated gate).
- `ui/src/contracts/index.ts` — ensure the 3 named exports (schema + type) resolve from the generated source (the re-export plumbing; Step-2.5 #1).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **Widen `generated_zod_member_sets_equal_frozen_schema`** (generated.test.ts) — extend `enumDefs` to include oneOf-const $defs + extract `const` values for the comparison. **RED:** the generator hasn't emitted the 3 → `Object.keys(validators)` ≠ the widened `enumDefs` set (the 3 oneOf-const names are missing from `validators`) → the member-set assertion fails. [§5.0]
2. **(GREEN drives)** extend `gen-contracts.mjs` → regenerate → the 3 appear in `validators` → test GREEN; the per-enum member-drift loop pins each const-set === `.options`.
3. **Shadow-retirement is a deletion** (no new test) — deleting the 3 `provisional.test.ts` drift-pin tests is safe because the widened generated gate (test 1) now pins the same value-sets against the same schema source (the authoritative gate). Confirm the suite stays green after deletion (no consumer references the deleted shadow exports — they resolve from the generated source).

> The RED is the widened drift gate failing because the generator skips oneOf-const. A broader RED (a consumer that breaks on the source move) is the Step-2.5 plumbing check.

## Cross-doc invariant impact
- **Model field changes:** none. No CONTRACT_VERSION bump (generator-only; values unchanged). The 3 enums move provisional→generated (a representation swap).
- **Orchestrator doc rows (Step 9):** likely none — the `ui/CLAUDE.md` "Generated Zod contract layer" row already anticipates this ("Doc-commented enums freeze as `oneOf`-of-`const` … live as drift-pinned provisional shadows pending a generator `oneOf`-const extension"); at completion I update that row to mark the extension DONE + the shadows retired. **Implementer does NOT edit `ui/CLAUDE.md`.**
- **2.5-seam:** none.

## Things to flag at Step 2.5
1. **The named-export plumbing** (the one real wiring detail). After deleting the provisional `export const MetricQuality = z.enum(...)`, consumers import `{MetricQuality}` (schema + type) from `contracts/index`. My default: re-export the 3 from the generated source via `index.ts` (e.g. derive `export const MetricQuality = validators.MetricQuality` + `export type MetricQuality = z.infer<…>`, mirroring how `index.ts` already exposes generated validators) so the surface is unchanged. Confirm the exact mechanism against the current `index.ts` structure (the generated bundle's shape vs named exports).
2. **Generator approach for oneOf-const → Zod.** My default: extract the `const` values and feed `json-schema-to-zod` a synthetic flat `enum` (the simplest equivalence-preserving path — `z.enum([...])`); confirm `json-schema-to-zod` emits an identical `z.enum` (not a `z.union` of literals — if it differs, the `.options` drift-pin still holds, but flag it). Alternative: emit `z.union([z.literal(...), …])` — only if the flat-enum path mis-renders.
3. **Delete vs keep the shadow drift-pin tests.** My default: DELETE (the widened generated gate is the authoritative drift-pin now; keeping both is redundant). Confirm the generated gate genuinely covers the same assertion (schema const-set === generated `.options`) before deleting.

## Dependencies + sequencing
- **Depends on:** none (daemon-independent ui-tooling; the schema already defines the 3 as oneOf-const).
- **Blocks:** nothing hard. Retires a long-carried drift hazard (the manual shadows).

## Estimated commit count
**1** — a focused, cohesive generator + drift-gate + shadow-retirement unit (one logical change: "the generator now handles oneOf-const, so the manual shadows retire"). NON-cat-1, daemon-independent, no safety pin, no contract bump.

## Lessons-logged candidates anticipated
- **Convention candidate** — the generator now emits oneOf-const → the `ui/LESSONS.md` §1/§2/§14 "never hand-declare; reconcile shadows to generated" discipline is now mechanically realized for doc-commented (oneOf-const) enums (one fewer manual-shadow class). Likely a one-line §14 reinforcement rather than a new lesson — your call at Step 9.

## How to invoke
1. Read this brief — especially the verify-before-build equivalence verdict (clean swap) + the guardrail (stop+Finding only if GREEN reveals a real divergence — not expected).
2. Confirm RED: `pnpm test src/contracts/generated.test.ts` (after widening the filter, before extending the generator).
3. `/tdd gen_contracts_oneof_const`.
4. Step 2.5 → the 3 plumbing/approach calls + the coverage map.
5. GREEN → `pnpm gen:contracts` regen + full suite + `/preflight` (confirm CONTRACT_VERSION still 0.38.0).
6. Step 9 → the `ui/CLAUDE.md` generated-layer-row update (I write hot) + any generator convention note.

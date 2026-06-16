# /tdd brief — regen_ui_zod_contract_0_33_boundary

## Feature
Regenerate the ui Zod contract layer from the merged **0.33.0** frozen schema (post the main→ui boundary merge `f1bdf0d`), restoring the §5.0 drift gate to GREEN. The entire ui-relevant 0.31→0.33 delta is: `ExecutorKind` enum **+`integration`** (11→12 values) and `x-contract-version` **0.31.0→0.33.0**. A regen-to-green slice — the existing drift tests ARE the spec.

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport task — the §5.0 contract layer it consumes; this is the boundary-merge regen, the same home as the 047/053 regens; NON-cat-1)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (the generated, drift-caught Zod consumer of the frozen Rust schema — regen + drift pins), `§5.1` (the status/value-set enums the layer mirrors)
- **Related context:** the boundary merge `f1bdf0d` (main@95df2e0, CONTRACT 0.31→0.33); the prior regens `docs/briefs/047-P6-3e-regen-0280-and-diff-read-surface.md` + `docs/briefs/053-P6-L2prep-regen-0.31-and-approval-row-swap.md` (the established regen pattern); `ui/LESSONS.md §14` (contract-bump regen discipline) + §2 (provisional-shadow-on-consume). The merge touched **no `ui/` frontend file** — only `shared/` (the 0.33 schema) — so the ui suite is byte-identical to its pre-merge green state except the contract drift tests (which read the schema).

## Acceptance criteria (what "done" means)
- [ ] `pnpm gen:contracts` regenerates `ui/src/contracts/generated.ts` — `ExecutorKind` now includes `"integration"` (12 values) and `export const CONTRACT_VERSION = "0.33.0"`. **Regenerated, never hand-edited** (`ui/LESSONS.md` §1/§14).
- [ ] `generated.test.ts` GREEN: the version gate (`CONTRACT_VERSION === schema["x-contract-version"]` → 0.33.0===0.33.0), the member-set equality (`ExecutorKind.options` === the frozen 12), and accept/reject all pass.
- [ ] `provisional.test.ts` GREEN with **no shadow change** — verified: no existing shadow's `$defs` shape drifted 0.31→0.33 (only `ExecutorKind` value-set + the new `SessionFailed` object $def changed). `SessionFailed` is NOT in any shadow `cases` list → no completeness trip.
- [ ] The full ui suite stays green (the pre-merge 361 TS), `tsc --noEmit` + `oxlint` clean.
- [ ] `/preflight` clean.
- [ ] `SessionFailed` shadow decision recorded (default: **defer** — see Step 2.5 Q1) — flagged at Step 9 as a carry-forward (orchestrator records).
- [ ] Cross-doc invariant flagged at Step 9 (orchestrator writes the `ui/CLAUDE.md` "Generated Zod contract layer" row hot: `CONTRACT_VERSION 0.31.0 → 0.33.0`, `ExecutorKind` 12 values, `SessionFailed` shadow-deferred). **Implementer does NOT edit `ui/CLAUDE.md`.**

## Wiring / entry point (Step 7.5)
None new — this is a contract-layer regen, not a new feature. The regenerated `validators`/value-sets are already consumed on the live path: `ui/src/contracts/index.ts` derives the validators from the generated bundle (`= shape`), consumed at `ui/src/gateway-client/boundary.ts` (parse-don't-trust) and throughout the cockpit. The new `ExecutorKind` `integration` value flows through the generated bundle with no new consumer (verified: `ExecutorKind` is referenced ONLY in `generated.ts` — not re-exported in `index.ts`, not in descriptors/views). The regen restores the already-wired §5.0 layer to 0.33; **no new reachable symbol** is introduced. (No `/wired` target — the wiring is the existing contract layer.)

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` — **regenerated** via `pnpm gen:contracts` (ExecutorKind +`integration`; CONTRACT_VERSION→"0.33.0"). The ONLY production file that changes.

**Not touched (verified):** `index.ts` (no new individual enum re-export — `ExecutorKind` has no consumer), the test files (all drift asserts are schema-dynamic → auto-green), any provisional shadow. If the regen surfaces a file beyond `generated.ts`, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
This is a **regen-to-green** slice: the failing tests already exist — the post-merge drift. Confirm RED for the right reason, then `pnpm gen:contracts` turns them GREEN. No new test is authored (the dynamic drift pins ARE the spec; ExecutorKind+integration is covered by the existing member-set assertion).

1. **`generated_contract_version_matches_frozen_schema`** (existing, `generated.test.ts:55`) — currently **RED**: `CONTRACT_VERSION` "0.31.0" ≠ schema "0.33.0".
   - Asserts: the generated version const === the frozen `x-contract-version`.
   - Why: `ARCHITECTURE.md §5.0` version gate (the §5.0 tripwire).
2. **`generated_zod_member_sets_equal_frozen_schema`** (existing, `generated.test.ts:42`) — currently **RED**: generated `ExecutorKind` (11) ≠ frozen (12, +`integration`).
   - Asserts: each generated enum's members === the frozen `$defs` enum (set equality), and `Object.keys(validators)` === the flat-enum $defs.
   - Why: `ARCHITECTURE.md §5.0`/§5.1 — generated value-sets mirror the frozen schema (`ui/LESSONS.md` §14).
3. **`generated_zod_accepts_every_canonical_enum_value`** (existing) — confirm green post-regen (ExecutorKind accepts `"integration"`).

> Confirm the RED is exactly (1)+(2) and nothing else before regen — a broader RED means an unanticipated consumer drift (flag at Step 2.5).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/` (the ui consumes the daemon-frozen 0.33 schema; no ui-authored contract change). The generated value-set `ExecutorKind` grows by one daemon-frozen value.
- **Orchestrator doc rows to write hot (Step 9 routing):** the `ui/CLAUDE.md` cross-doc "Generated Zod contract layer" row — `CONTRACT_VERSION 0.31.0 → 0.33.0`; note `ExecutorKind` 12 values (+`integration`); `SessionFailed` shadow-deferred. (Orchestrator writes at `/orchestrate-end`.)
- **2.5-seam (subsystem-boundary / shared-contract) model touched?** No NEW/extended ui-authored invariant on a 2.5-seam model — the regen consumes the daemon's frozen schema. No schema-snapshot test to author (the generated drift tests already pin the value-sets; `SessionFailed` is un-shadowed by design).

## Things to flag at Step 2.5
1. **`SessionFailed` (new 0.32 object $def + root `session_failed` event) — add a ui provisional shadow now, or defer?** The ui reads `proj_session` (where the `failed` session status already exists + is mapped in `descriptors.ts`), not the raw `SessionFailed` event; no ui consumer exists yet (the session-card "Failed + restart affordance" / recovery-UX surface is a future slice). My default vote: **DEFER** — shadow-on-consume (`ui/LESSONS.md` §2); a shadow with no consumer is dead surface. Record a carry-forward (`origin: this regen`) so the future session-failure-UX slice picks it up. (The lead flagged SessionFailed for consideration — this is the considered answer.)
2. **Add an explicit hardcoded "ExecutorKind includes integration" assertion, or rely on the existing dynamic member-set pin?** My default vote: **rely on the dynamic pin** — `generated_zod_member_sets_equal_frozen_schema` already proves it against the schema; a hardcoded duplicate is redundant and drifts (matches the 047/053 regen pattern — no bespoke per-value test).
3. **Any consumer reconcile for the new `integration` ExecutorKind value?** My default vote: **NONE** — verified `ExecutorKind` is consumed only inside `generated.ts` (no `.extract`/`.options` completeness pin, no descriptors entry, no view). If the impl finds a consumer I missed, flag it.

## Dependencies + sequencing
- **Depends on:** the boundary merge `f1bdf0d` (landed — schema is 0.33.0 in the tree).
- **Blocks:** Phase-7-UI (7.2 PR Review Workspace / 7.3 Task Inbox) — the rich PR projection contract is now in `track/ui`; Phase-7-UI becomes buildable once this regen greens the contract layer (a later round, not this one).

## Estimated commit count
**1** (atomic regen). One `git add ui/src/contracts/generated.ts` + the manifest if pnpm touches a lockfile (it shouldn't — `gen:contracts` runs `node`, no install). NON-cat-1, no safety pin → no split.

## Lessons-logged candidates anticipated
- **No new lesson likely** — the regen pattern is already `ui/LESSONS.md §14` (contract-bump regen discipline) + §2 (provisional-shadow-on-consume, the SessionFailed defer). If anything, a one-line reinforcement that a boundary-merge regen can be value-set-only (no shadow work) when the incoming types are un-consumed.
- **Architecture-doc note candidate** — none (the ui implements no new §; the daemon owns the 0.33 contract).
- **Future TODO (carry-forward)** — the `SessionFailed` ui shadow + its session-failure-UX consumer (deferred here).

## How to invoke
1. **Read this brief end-to-end** — especially the Step 2.5 SessionFailed defer.
2. **Confirm the post-merge RED first:** `pnpm test ui/src/contracts/generated.test.ts` (or the suite) shows (1)+(2) RED for the version + ExecutorKind drift — and ONLY those.
3. **Run `/tdd regen_ui_zod_contract_0_33_boundary`.**
4. **Step 2.5** — ping back with the SessionFailed call (default: defer) + any unanticipated RED.
5. **GREEN** = `pnpm gen:contracts`, then full suite + `/preflight`.
6. **Step 9** — surface the cross-doc row (CONTRACT_VERSION→0.33) + the SessionFailed carry-forward.

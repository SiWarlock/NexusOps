# /tdd brief — regen_ui_zod_contract_0_38_boundary

## Feature
Regenerate the ui Zod contract layer from the merged **0.38.0** frozen schema (post the main→ui boundary merge `2106864`), restoring the §5.0 drift gate to GREEN. The entire ui-relevant 0.33→0.38 delta is **enum-set + version only** (generated.ts is enum-only): a NEW string enum `ReviewState` (5 values: `approved`/`changes_requested`/`commented`/`dismissed`/`pending`), `ProjectionName` gains `"Review"` (enum `$def` count 37→38), and `x-contract-version` **0.33.0→0.38.0**. A regen-to-green slice — the existing drift tests ARE the spec.

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport task — the §5.0 contract layer it consumes; this is the boundary-merge regen, the same home as the 047/053/ui-058 regens; NON-cat-1)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (the generated, drift-caught Zod consumer of the frozen Rust schema — regen + drift pins), `§5.1` (the status/value-set enums the layer mirrors)
- **Related context:** the boundary merge `2106864` (main@3d0cba5, CONTRACT 0.33→0.38 — the daemon D1–D5 UI-unblock work order); the prior regen `docs/briefs/ui-058-regen-0.33-contract-boundary.md` (the immediate precedent — same shape, slightly smaller delta) + the session doc `docs/sessions/ui-014-2026-06-15-boundary-merge-0.33-regen.md`; `ui/LESSONS.md §14` (contract-bump regen discipline) + §1 (generated.ts never hand-edited) + §2 (provisional-shadow-on-consume). The merge touched **no `ui/` frontend file** — only `shared/` + `daemon/` + `docs/` — so the ui suite is byte-identical to its pre-merge green state except the contract drift tests (which read the schema) + the hardcoded §5.0 version tripwire.

## Acceptance criteria (what "done" means)
- [ ] `pnpm gen:contracts` regenerates `ui/src/contracts/generated.ts` — the `z.object` default export now includes `ReviewState` (5 values), `ProjectionName` includes `"Review"`, and `export const CONTRACT_VERSION = "0.38.0"`. **Regenerated, never hand-edited** (`ui/LESSONS.md` §1/§14).
- [ ] `generated.test.ts` GREEN: the version gate (`generated_contract_version_matches_frozen_schema` → 0.38.0===0.38.0), the member-set + keys equality (`generated_zod_member_sets_equal_frozen_schema` — `ProjectionName.options` includes `Review`, `ReviewState.options` === the frozen 5, and `Object.keys(validators)` === the flat-enum $defs now incl. `ReviewState`), and accept/reject (`generated_zod_accepts_every_canonical_enum_value` / `generated_zod_rejects_unknown_enum_value`) all pass.
- [ ] The `mock.test.ts:115` §5.0 version tripwire (`mock_get_capabilities_reports_contract_version`) bumped `expect(caps.contract_version).toBe("0.33.0")` → `"0.38.0"`, and its explanatory changelog comment (mock.test.ts:108–114) appended the 0.34→0.38 additions (D1 PullRequestRow 0.34 · D2 SessionRow recovery 0.35 · D5a mergeable/checks 0.36 · D5b-1 review vertical 0.37 · D5b-2 sync_reviews 0.38). This is the tripwire's designed maintenance (the hardcoded assertion is green pre-regen, so it is NOT in the Step-2 RED — bump it at GREEN).
- [ ] `provisional.test.ts` GREEN with **no shadow change** — verified: no existing shadow's `$defs` shape drifted, and `ProjectionPageByName` (the hand-maintained 6-key page registry: Session/ProjectActivity/PullRequest/ApprovalQueue/AuditTrail/UsageLedger) is deliberately a SUBSET of the frozen `ProjectionName` enum → `"Review"` does NOT trip it. The new object `$defs` (`PullRequestRow`/`SessionRow`/`ReviewRow`/`ReviewSynced`) are NOT shadowed in this slice (feature-arc work; shadow-on-consume per `ui/LESSONS.md` §2).
- [ ] The full ui suite stays green (the pre-merge ~367 TS), `tsc --noEmit` + `oxlint` clean.
- [ ] `/preflight` clean (note: the ui prettier-gate is a known structural no-op — `ui-014` follow-up #4; not introduced here).
- [ ] New-object-row + feature-arc reconcile decisions recorded (default: **defer all** — see Step 2.5) — flagged at Step 9 as carry-forward (orchestrator records).
- [ ] Cross-doc invariant flagged at Step 9 (orchestrator writes the `ui/CLAUDE.md` "Generated Zod contract layer" row hot: `CONTRACT_VERSION 0.33.0 → 0.38.0`, enum-`$def` count 37→38 (+`ReviewState`), `ProjectionName` +`Review`, the new object rows shadow-deferred). **Implementer does NOT edit `ui/CLAUDE.md`.**

## Wiring / entry point (Step 7.5)
None new — this is a contract-layer regen, not a new feature. The regenerated `validators`/value-sets are already consumed on the live path: `ui/src/contracts/index.ts` derives the validators from the generated bundle (`= bundle.shape`), consumed at `ui/src/gateway-client/boundary.ts` (parse-don't-trust) and throughout the cockpit. The new `ReviewState` enum and the new `ProjectionName` `Review` value flow through the generated bundle with **no new consumer** (verified: `ReviewState` is exposed-ahead — like `ExecutionProfile`/`Provider`/`DiffLineKind`-ahead — NOT re-exported in `index.ts` until a typed consumer lands; the provisional `ProjectionName` type = `keyof ProjectionPageByName` is decoupled and unchanged). The regen restores the already-wired §5.0 layer to 0.38; **no new reachable symbol** is introduced. (No `/wired` target — the wiring is the existing contract layer.)

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` — **regenerated** via `pnpm gen:contracts` (+`ReviewState`; `ProjectionName` +`Review`; CONTRACT_VERSION→"0.38.0"). The ONLY production file that changes.
- `ui/src/gateway-client/mock.test.ts` — the §5.0 version tripwire assertion (`:115`) 0.33.0→0.38.0 + its changelog comment (test-only; the designed tripwire maintenance).

**Not touched (verified):** `index.ts` (no new individual enum re-export — `ReviewState` has no consumer; `ProjectionNameEnum`/`validators` are derived from `.shape` and self-maintain), the other test files (all drift asserts are schema-dynamic → auto-green), any provisional shadow, `mock.ts` FIXTURES (keyed by the provisional `ProjectionName` subset, unchanged). If the regen surfaces a file beyond `generated.ts` + the `mock.test.ts` tripwire, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
This is a **regen-to-green** slice: the failing tests already exist — the post-merge drift. Confirm RED for the right reason, then `pnpm gen:contracts` turns them GREEN. No new test is authored (the dynamic drift pins ARE the spec; `ReviewState`+`ProjectionName`+`Review` are covered by the existing member-set + keys-equal assertions).

1. **`generated_contract_version_matches_frozen_schema`** (existing, `generated.test.ts:55`) — currently **RED**: `CONTRACT_VERSION` "0.33.0" ≠ schema "0.38.0".
   - Asserts: the generated version const === the frozen `x-contract-version`.
   - Why: `ARCHITECTURE.md §5.0` version gate (the §5.0 tripwire).
2. **`generated_zod_member_sets_equal_frozen_schema`** (existing, `generated.test.ts:42`) — currently **RED**: generated bundle is missing `ReviewState`, and generated `ProjectionName` (10) ≠ frozen (11, +`Review`).
   - Asserts: each generated enum's members === the frozen `$defs` enum (set equality), and `Object.keys(validators)` === the flat-enum $defs.
   - Why: `ARCHITECTURE.md §5.0`/§5.1 — generated value-sets mirror the frozen schema (`ui/LESSONS.md §14`).
3. **`generated_zod_accepts_every_canonical_enum_value`** (existing, `:23`) / **`generated_zod_rejects_unknown_enum_value`** (`:36`) — confirm green post-regen (accepts the new `ReviewState` values + `ProjectionName` `Review`; still rejects unknown).

> Confirm the RED is exactly (1)+(2) and nothing else before regen — a broader RED means an unanticipated consumer drift (flag at Step 2.5). The `mock.test.ts:115` tripwire is GREEN pre-regen (it asserts the *served* mock version, hardcoded 0.33.0) and is NOT part of the RED — it is bumped at GREEN as the tripwire's designed maintenance (ui-058 precedent).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/` (the ui consumes the daemon-frozen 0.38 schema; no ui-authored contract change). The generated value-set grows by one enum (`ReviewState`) + one `ProjectionName` value (`Review`), all daemon-frozen.
- **Orchestrator doc rows to write hot (Step 9 routing):** the `ui/CLAUDE.md` cross-doc "Generated Zod contract layer" row — `CONTRACT_VERSION 0.33.0 → 0.38.0`; enum-`$def` count 37→38 (+`ReviewState`); `ProjectionName` +`Review`; the new object rows (`PullRequestRow`/`SessionRow`/`ReviewRow`/`ReviewSynced`) shadow-deferred. (Orchestrator writes at `/orchestrate-end`.)
- **2.5-seam (subsystem-boundary / shared-contract) model touched?** No NEW/extended ui-authored invariant on a 2.5-seam model — the regen consumes the daemon's frozen schema. No schema-snapshot test to author (the generated drift tests already pin the value-sets; the new object rows are un-shadowed by design).

## Things to flag at Step 2.5
1. **The new object `$defs` (`PullRequestRow`, `SessionRow`, `ReviewRow`, `ReviewSynced`) — add ui provisional shadows now, or defer?** My default vote: **DEFER ALL** — shadow-on-consume (`ui/LESSONS.md §2`); each is dead surface until its feature arc builds (Phase-7-UI consumes `PullRequestRow`; survival UI consumes `SessionRow`; the review surface consumes `ReviewRow`/`ReviewState`). generated.ts is enum-only so these object rows never enter the regen anyway. Record carry-forwards (`origin: this regen`) for the three feature arcs.
2. **Add a hardcoded "ReviewState / ProjectionName includes Review" assertion, or rely on the existing dynamic member-set pin?** My default vote: **rely on the dynamic pin** — `generated_zod_member_sets_equal_frozen_schema` already proves both against the schema; a hardcoded duplicate is redundant and drifts (the 047/053/ui-058 pattern — no bespoke per-value test).
3. **Any consumer reconcile for the new `ProjectionName` `Review` value or the new `ReviewState` enum?** My default vote: **NONE this round** — verified the provisional `ProjectionPageByName` registry is a hand-maintained 6-key subset decoupled from the frozen enum (it already omits ProjectGraph/PlanProgress/Worktree/AgentTeam), so `"Review"` doesn't trip it, `mock.ts` FIXTURES (keyed by that subset) is unchanged, and `ReviewState` is exposed-ahead with no `index.ts` re-export. If the impl finds a consumer I missed, flag it.

## Dependencies + sequencing
- **Depends on:** the boundary merge `2106864` (landed — schema is 0.38.0 in the tree).
- **Blocks:** the now-unblocked feature arcs (each its own later round): Phase-7-UI (consume `PullRequestRow` incl. the rich D5 `mergeable`/`checks_summary`/structured reviews; reconcile `pr_number` str→num + `toPrItems` id→`pr_id`), survival UI (consume `SessionRow` recovery fields → re-derive the daemon-wide `RecoveryState` banner), whole-cockpit-live (apply refetch-on-nudge to the now-delta-emitting projections), + the Session refetch-on-nudge carry-forward (UI half of D3) and the Session serve reconcile (`title`→`display_name`, `project_id` non-Option). None of these are this round.

## Estimated commit count
**1** (atomic regen). One `git add ui/src/contracts/generated.ts ui/src/gateway-client/mock.test.ts`; no lockfile churn (`gen:contracts` runs `node`, no install). NON-cat-1, no safety pin → no split.

## Lessons-logged candidates anticipated
- **No new lesson likely** — the regen pattern is already `ui/LESSONS.md §14` (contract-bump regen discipline) + §1 (generated never hand-edited) + §2 (shadow-on-consume, the object-row defers). Possible one-line reinforcement: a boundary-merge regen can absorb a NEW enum `$def` (not just a new value in an existing enum) with zero consumer work when the enum is exposed-ahead and the page registry is a decoupled subset.
- **Architecture-doc note candidate** — none (the ui implements no new §; the daemon owns the 0.38 contract).
- **Future TODO (carry-forward)** — the three object-row shadows + their feature-arc consumers (deferred here); the D4b future-CONTRACT flag (proj_project/proj_repository/proj_integration_connection have no `ProjectionName` variant → not live-subscribe-able without a contract change).

## How to invoke
1. **Read this brief end-to-end** — especially the Step 2.5 object-row defers + the tripwire-is-not-in-RED note.
2. **Confirm the post-merge RED first:** `pnpm test src/contracts/generated.test.ts` (or the suite) shows (1)+(2) RED for the version + `ReviewState`/`ProjectionName` drift — and ONLY those.
3. **Run `/tdd regen_ui_zod_contract_0_38_boundary`.**
4. **Step 2.5** — ping back with the object-row defers (default: defer all) + any unanticipated RED.
5. **GREEN** = `pnpm gen:contracts`, then bump the `mock.test.ts:115` tripwire (+comment), then full suite + `/preflight`.
6. **Step 9** — surface the cross-doc row (CONTRACT_VERSION→0.38, +ReviewState, +ProjectionName.Review) + the object-row carry-forwards.

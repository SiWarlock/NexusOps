# /tdd brief — regen_to_green_0.42_plus_D6_pr_card_diff_stats

## Feature
Reconcile the UI contract layer to the post-merge **CONTRACT 0.42.0** boundary (the §4.7 daemon wave D6/D7/D9/D10 landed via the `track/ui ← main` merge `aa1731a`), then **consume the D6 diff-stats** — render the real `+additions / −deletions · N files · M commits` in the PR Review Workspace, retiring the honest D6 "unavailable" placeholder. NON-cat-1, read-only (no mutation, no new Gateway surface).

## Use case + traceability
- **Task ID:** P7.2 (the ui-half Full PR Review Workspace; consumes the daemon P4.7 / D6 work-order item) — see `docs/planning/daemon-unblock-work-order.md` "D6".
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (PR Review Workspace surface), `§7.2` (PR-projection-row consumption / GitHub SoT), `§5.0` (generated, drift-caught Zod consumer of the frozen Rust authority).
- **Widens phase scope because:** this P7.2 slice folds in the **§5.0 contract-boundary regen-to-green** (CONTRACT 0.38→0.42) — a cross-phase enabler (the established boundary-merge-regen pattern, ui-060 precedent) — and references the **§2.5-seam** frozen `PullRequestRow` and the **§4.7** daemon work-order origin; the dominant deliverable is the §11.2 PR-card D6 consumption.
- **Related context:** the merge brought CONTRACT 0.38→0.42; `pnpm gen:contracts` has **already been run** by the orchestrator during the merge sync → `ui/src/contracts/generated.ts` is regenerated to `0.42.0` **in the working tree, uncommitted** (this slice commits it). Precedents: **ui-060** (regen-0.38 boundary), **ui-061** (PullRequestRow shadow reconcile 4→11), **ui-064** (the PR Workspace shell + the D6/D7 placeholders), LESSONS **[[2]]/[[14]]/[[24]]** (regen + frozen-shadow discipline), **[[32]]** (null-safe at every render site).

## The frozen surface this consumes (verified in the merged tree, 2026-06-20)
`shared/src/projections.rs` `PullRequestRow` gained the **4 D6 diff-stat fields** (all `Option<u64>`):
```rust
pub additions: Option<u64>,
pub deletions: Option<u64>,
pub changed_files: Option<u64>,
pub commits: Option<u64>,
```
`None`/NULL where GitHub omitted them or for pre-D6 rows (the producer-side capture is daemon-gated for already-synced PRs — the D7 auth-bootstrap / PR-status-refresh follow-on; a freshly-synced PR carries them). **`head_sha` is NOT on the frozen row** — it is a live-mutation go-live prereq, explicitly OUT of this slice.

## Acceptance criteria (what "done" means)
- [ ] `ui/src/contracts/generated.ts` is the **regenerated** 0.42.0 artifact (re-run `pnpm gen:contracts` to confirm idempotent; **never hand-edit** — [[1]]/[[14]]).
- [ ] `PullRequestRow` provisional shadow (`ui/src/contracts/provisional.ts`) gains `additions` / `deletions` / `changed_files` / `commits`, each a nullable-optional non-negative integer (mirroring `pr_number`'s `z.number().int().nonnegative().nullable().optional()`); `.strict()` preserved → `provisional.test.ts` `pull_request_row_field_set_matches_frozen_schema` (line 330) GREEN.
- [ ] `mock.test.ts` `mock_get_capabilities_reports_contract_version` (line 118) tripwire bumped `"0.38.0"` → `"0.42.0"` with a comment line documenting the 0.38→0.42 §4.7 bump (D6 diff-stats 0.39 / D7 get_pr_diff 0.40 / D9 merge_pr 0.41 / D10 submit_review 0.42).
- [ ] `PrWorkspace.tsx` renders the **real** diff-stats from the row — additions (`+`), deletions (`−`), changed_files (files), commits — and **retires** the `pr-diffstats-unavailable` placeholder (lines 84-90).
- [ ] **Null-safe (per [[32]]):** a field that is `null`/absent is never rendered as a fabricated `0` or a bare glyph; if ALL four are null, render an honest "diff stats unavailable for this PR" state (NOT fabricated numbers) — the daemon hasn't captured them (pre-D6 / unsynced row).
- [ ] **Never color alone** (§11/forbidden #5): +/− carry glyph + label, not just green/red.
- [ ] The **D7** `pr-diff-unavailable` placeholder (lines 91-101) **STAYS** (retired in the D7 slice).
- [ ] New render tests in `ui/src/views/code/PrWorkspace.test.tsx` pass (the existing `pr-diffstats-unavailable` assertion at line 61 is replaced by the real-stats assertion).
- [ ] `/preflight` clean (the 2 currently-failing tests go green; 393/393 restored).

## Wiring / entry point (Step 7.5)
`PrWorkspace` is **already wired + reachable** (ui-064): it is rendered by `DiffReview.tsx` for a PR selected from the live "Pull requests" Kanban tab, with `pr: PullRequestRow` + `reviews: ReviewRow[]` props sourced from the live `get_projection("PullRequest")` / `reviewsByPr`. This slice adds NO new wiring — the D6 fields flow through the existing `pr` prop into the existing render. Confirm at Step 7.5 that the new diff-stats render is on that production path (the PR Workspace opened from the Kanban), not test-only.

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` — the already-regenerated 0.42.0 artifact (stage; do not hand-edit).
- `ui/src/contracts/provisional.ts` — `PullRequestRow` shadow + the 4 D6 fields (lines ~182-196).
- `ui/src/gateway-client/mock.test.ts` — version tripwire 0.38→0.42 (line 118) + comment.
- `ui/src/views/code/PrWorkspace.tsx` — retire `pr-diffstats-unavailable`; render real diff-stats null-safe (lines 84-90).
- `ui/src/views/code/PrWorkspace.test.tsx` — replace the D6-placeholder assertion with the real-stats + all-null-honest render pins.

If the implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
`ui/src/contracts/provisional.test.ts` (already present — these go RED→GREEN as the shadow grows):
1. **`pull_request_row_field_set_matches_frozen_schema`** (existing, line 330) — Asserts: shadow key-set == frozen `$defs.PullRequestRow.properties`. Why: §5.0 drift-pin / [[24]] frozen-shadow ([[14]]).
2. **`pull_request_row_diff_stats_are_uint_nullable`** (new) — Asserts: a present `additions: 12` parses; `additions: "12"` (string) and `additions: -1` (negative) reject; `additions: null`/absent tolerated. Why: §5.0 uint contract (mirror the `pr_number` uint pin at line ~336).

`ui/src/gateway-client/mock.test.ts`:
3. **`mock_get_capabilities_reports_contract_version`** (existing, line 118) — Asserts: `caps.contract_version === "0.42.0"`. Why: the §5.0 version tripwire chained to the regen.

`ui/src/views/code/PrWorkspace.test.tsx`:
4. **`pr_card_renders_real_diff_stats`** (new) — Asserts: a row with `{additions:40,deletions:7,changed_files:3,commits:2}` renders +40 / −7 / 3 files / 2 commits AND `pr-diffstats-unavailable` is gone. Why: §11.2 D6 consumption.
5. **`pr_card_diff_stats_null_safe`** (new) — Asserts: a row with all 4 diff-stats `null` renders an honest unavailable state, NO fabricated `0`/`+`/`−`. Why: [[32]] null-safe + forbidden-#2 no-fabrication.
6. **`pr_card_diff_stats_never_color_alone`** (new, light) — Asserts: +/− deltas carry a glyph/label, not color-only. Why: §11/forbidden #5.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `PullRequestRow` shadow `11 → 15` fields (the 4 D6 diff-stats). The frozen authority is `shared/src/projections.rs` (daemon-owned; landed) — this is the UI consumer catching up.
- **§2.5-seam (shared-contract) model touched?** YES — `PullRequestRow` is a §2.5-seam frozen projection-row. The **schema-snapshot test is the existing `pull_request_row_field_set_matches_frozen_schema`** (drift-pinned to `shared/src/projections.rs`) — no NEW snapshot test needed; this slice makes it pass at the new field-set.
- **Orchestrator doc rows to write hot (Step 9 routing):** the `ui/CLAUDE.md` cross-doc "Generated Zod contract layer" row — update the `CONTRACT_VERSION 0.38.0 → 0.42.0` note + the `PullRequestRow` field-count (`11 → 15`) + a 0.38→0.42 boundary-regen note (D6/D7/D9/D10). **Orchestrator-territory — do NOT edit `ui/CLAUDE.md` yourself; flag at Step 9.**

## Things to flag at Step 2.5
1. **All-null diff-stats rendering.** Some PRs (pre-D6 / unsynced) carry all four `null`. Options: (a) honest "diff stats unavailable for this PR" line; (b) render nothing; (c) render zeros. My default vote: **(a)** — honest unavailable, mirrors the existing placeholder honesty + forbidden-#2 (never fabricate), and is distinguishable from a real "0 changes" PR.
2. **Partial-null (some fields present, some null).** Default vote: **render each present field, omit the absent one** (per-field null-safe, [[32]]); don't suppress the whole row because one field is missing.
3. **Diff-stat field type.** `z.number().int().nonnegative().nullable().optional()` (mirror `pr_number`). Default vote: **yes** — matches the frozen `Option<u64>` (integer, minimum 0) and the established uint shadow shape.
4. **Commit-count split.** This brief is 2 layers (see Estimated commit count). Default vote: **2 commits** (regen-to-green, then D6 consumption) for bisectability; take 1 if you judge it one tight unit.

## Dependencies + sequencing
- **Depends on:** the `track/ui ← main` merge `aa1731a` + the regen (both landed; generated.ts in tree).
- **Blocks:** the **D7** full PR-detail view (get_pr_diff consumer — the next read-only slice); the **D9/D10** cat-1 PR-mutations arc (HELD — orchestrator escalates the cat-1 design to lead→user before authoring).

## Estimated commit count
**2.** This is a 2-layer slice; the implementer idles after each layer and the orchestrator drives layer→layer ([[7]]):
- **Layer 1 — regen-to-green:** stage the regen'd `generated.ts` + add the 4 fields to the `PullRequestRow` shadow + bump the `mock.test.ts` version tripwire → the suite is GREEN at 0.42.0 (393/393). Independently meaningful (the contract boundary is reconciled).
- **Layer 2 — D6 consumption:** retire `pr-diffstats-unavailable`, render the real null-safe diff-stats in `PrWorkspace.tsx` + the new render tests.

Both layers are read-only, same code area (contracts + views/code), no safety invariant → a clean 2-commit slice (not split into separate briefs).

## Lessons-logged candidates anticipated
- **Convention candidate** — "On a CONTRACT boundary merge, the regen artifact is staged-not-hand-edited and the version tripwire + frozen-shadow field-set are the two reconcile points; consumption follows in the same slice's 2nd layer." (likely a refinement of [[14]]/[[24]], not a new lesson.)
- **Future TODO — operational** — pre-D6 / unsynced rows carry null diff-stats until the daemon PR-status-refresh sync (D7 auth-bootstrap follow-on) backfills them; the honest-unavailable state is the correct interim.
- **Architecture-doc note candidate** — none expected (consumes the already-frozen 0.42 row).

## How to invoke
1. Read this brief end-to-end (don't skip "Things to flag at Step 2.5").
2. Run `/tdd regen_to_green_0.42_plus_D6_pr_card_diff_stats`.
3. Step 0 (Restate) — confirm it matches the Feature line.
4. Step 2.5 — ping back the test design (one `Asserts: <invariant> (§anchor)` line per test + the coverage map) with answers to the 4 design questions (or take defaults).
5. Step 9 — surface the cross-doc invariant flag (the `ui/CLAUDE.md` row) + anything beyond the anticipated lessons.

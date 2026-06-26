# /tdd brief — wire_real_head_sha_pin

## Feature
Wire the **real `head_sha` pin source**: the daemon now exposes `head_sha` on `PullRequestRow` (CONTRACT 0.44) + resolves owner/repo from `repo_id` (ruling A landed). Reconcile the UI to 0.44 (regen-to-green: shadow + version tripwire) and retire the `prHeadSha` = null deferral — `prHeadSha(pr)` now returns the real `pr.head_sha`. The Merge/Approve/Request-changes/Comment controls become **real-pinned** but **stay DISABLED** behind the default-off `enabledPrMutations` gate. **NON-cat-1** (sourcing the pin only — the gate + intent safety core are unchanged). **Do NOT flip the go-live.**

## Use case + traceability
- **Task ID:** P7.2 (the PR Review Workspace cat-1 PR-mutations arc — wiring the daemon-exposed pin source).
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (the PR Workspace), `§7.2` (PR projection consumption), `§5.0` (regen — the drift-caught Zod consumer).
- **Widens phase scope because:** folds the **§5.0 contract-boundary regen-to-green** (CONTRACT 0.42→0.44 — the daemon `head_sha` exposure + the ExecutionProfileRegistered/5.3a work + the 4-site ruling-A resolution) — the established boundary-merge-regen pattern (ui-060/ui-068 precedent); the dominant deliverable is the §11.2 head_sha pin wiring.
- **Related context:** the daemon merge brought `head_sha: Option<String>` on `PullRequestRow` (`shared/src/projections.rs:81` — "Display/pin-FORMATION source only; the daemon's anti-race is the LIVE GitHub 409") + the 4-site confused-deputy closure (`execute_merge_pr`/`submit_review` now resolve owner/repo from the audited identity via `daemon/src/integrations/repo_resolve.rs` — ruling A AS-BUILT, no longer reads `inputs["owner"]`/`["repo"]`). The UI stub to retire: `prHeadSha(pr)` in `ui/src/views/code/PrWorkspace.tsx` (currently returns null; imported by `DiffReview.tsx:25`, drives `canMerge`/`canReview` at `:618`/`:621`). ui-068 (the 0.42 regen-to-green precedent — the shadow + tripwire pattern). The regen is ALREADY RUN (orchestrator ran `pnpm gen:contracts` → `generated.ts` @0.44 in the tree, uncommitted).

## Acceptance criteria (what "done" means)
- [ ] `ui/src/contracts/generated.ts` is the regenerated 0.44.0 artifact (stage; re-run `pnpm gen:contracts` to confirm idempotent — never hand-edit).
- [ ] `PullRequestRow` provisional shadow (`provisional.ts`) gains `head_sha: z.string().nullable().optional()` → `pull_request_row_field_set_matches_frozen_schema` (15→16) GREEN. (It's a TEXT/string field, not a uint like the diff-stats.)
- [ ] `mock.test.ts` `mock_get_capabilities_reports_contract_version` tripwire bumped `"0.42.0"` → `"0.44.0"` + a comment noting the 0.42→0.44 bump (0.43 ExecutionProfileRegistered/5.3a · 0.44 head_sha exposure).
- [ ] `prHeadSha(pr)` returns the real `pr.head_sha` (`pr.head_sha ?? null` — null only when the daemon omitted it / a pre-P4.7 row), retiring the `= null` stub + its comment.
- [ ] **The go-live stays HELD (the load-bearing pin):** with a non-null `head_sha` AND `enabledPrMutations` EMPTY (production default), `canMerge`/`canReview` are STILL false → all controls disabled. A test asserts head_sha being real does NOT enable any control (only the per-action gate flip does). **Do NOT change `enabledPrMutations` defaults.**
- [ ] Conversely (Mock/dev), with head_sha real AND the action enabled (Mock's full set), the control enables — proving the pin no longer blocks (head_sha was the last UI-side blocker; the daemon auth-bootstrap is the remaining DAEMON-side go-live blocker, out of scope).
- [ ] `/preflight` clean (the 2 drift failures → green; 437/437-ish).

## Wiring / entry point (Step 7.5)
`prHeadSha` is already wired into `DiffReview.tsx`'s `canMerge`/`canReview` (ui-070/071); this slice only changes its RETURN (null stub → real `pr.head_sha`). No new wiring — confirm the merge/review enablement now reads the real field on the production path.

## Files expected to touch
**Modified:**
- `ui/src/contracts/generated.ts` — the regenerated 0.44 artifact (stage; no hand-edit).
- `ui/src/contracts/provisional.ts` — `PullRequestRow` shadow + `head_sha`.
- `ui/src/gateway-client/mock.test.ts` — version tripwire 0.42→0.44.
- `ui/src/views/code/PrWorkspace.tsx` — `prHeadSha` returns `pr.head_sha`.
- test files (`provisional.test.ts`, `PrWorkspace.test.tsx`/`DiffReview.test.tsx`) — the field-set + the head_sha-real-but-gate-held pins.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. `pull_request_row_field_set_matches_frozen_schema` (existing) — Asserts: shadow key-set == frozen 16-field `$defs.PullRequestRow`. Why: §5.0 drift-pin.
2. `pull_request_row_head_sha_is_string_nullable` (new) — Asserts: a string `head_sha` parses; null/absent tolerated; a non-string rejects. Why: §5.0 string contract.
3. `mock_get_capabilities_reports_contract_version` (existing, bumped) — Asserts: `"0.44.0"`. Why: the §5.0 tripwire.
4. `pr_head_sha_returns_real_field` (new, PrWorkspace/DiffReview test) — Asserts: `prHeadSha(pr)` returns `pr.head_sha` when present; null when absent. Why: the pin source wiring.
5. `real_head_sha_does_not_enable_controls_while_gate_empty` (new — the load-bearing held-flip pin) — Asserts: a PR with a real `head_sha` + `enabledPrMutations` EMPTY → `canMerge`/`canReview` false → all controls disabled. Why: head_sha is NOT the go-live switch; the per-action gate is ([[27]]/[[28]]).
6. `real_head_sha_with_enabled_action_enables_control` (new) — Asserts: real head_sha + the action in the set → the control enables (the pin no longer blocks). Why: prove head_sha was the UI-side blocker, now sourced.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `PullRequestRow` shadow 15→16 (+`head_sha`). Frozen authority `shared/src/projections.rs` (daemon-landed). The existing `pull_request_row_field_set_matches_frozen_schema` snapshot is the §2.5-seam pin (no NEW snapshot test).
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` generated-Zod row (CONTRACT 0.42→0.44 + `PullRequestRow` 15→16 +head_sha) + a note on the mutation row that the pin is now real-sourced (controls still gate-held). **Flag, don't edit.**

## Things to flag at Step 2.5
1. **head_sha field type.** `z.string().nullable().optional()` (a TEXT field, direct passthrough — NOT a uint). Default vote: **yes** (mirrors the frozen `Option<String>`; no coercion).
2. **The held-flip guard.** A test that real head_sha + empty gate → still disabled. Default vote: **yes, load-bearing** — the whole point is that head_sha sourcing is NOT a go-live (the lead emphasized: do NOT flip). Pin it.
3. **Commit count.** Default vote: **1** — a focused regen-to-green + 1-line helper wire (the shadow + tripwire + prHeadSha + the 2 guard tests are one logical unit; no cat-1 safety core to isolate — the gate/intent are unchanged).

## Dependencies + sequencing
- **Depends on:** the `track/ui ← main` merge (the daemon head_sha + ruling-A resolution; landed `a5cedd9`); ui-070/071 (the gate + the controls; landed).
- **Blocks:** the **go-live flip** (now ALL UI-side blockers cleared — head_sha real + ruling-A daemon-side; the ONLY remaining blocker is the DAEMON auth-bootstrap for live authenticated writes + its mandatory security re-review, still daemon-side + user sign-off).

## Estimated commit count
**1.** A focused regen-to-green + pin-source wire (non-cat-1; the gate + intent safety core unchanged). No layer split — one logical unit, one commit.

## Lessons-logged candidates anticipated
- **Convention candidate** — "a deferred-field shadow (the `prHeadSha`=null A3 deferral) is retired in a focused regen-to-green slice once the daemon field lands — the field + the tripwire + the helper-return + a 'sourcing-the-field is NOT a go-live' guard." (a refinement of [[14]]/[[32]].)
- **Architecture-doc note** — the UI head_sha is display/pin-FORMATION only; the daemon's anti-race is the LIVE GitHub 409 (not this field).

## How to invoke
1. Read this brief end-to-end.
2. Run `/tdd wire_real_head_sha_pin`.
3. Step 0 (Restate) — confirm it matches the Feature line.
4. Step 2.5 — ping back the test design + the 3 design-Q answers (or take defaults).
5. Step 9 — the cross-doc flag (the `ui/CLAUDE.md` row) + the ship-ask. **NON-cat-1 — security-reviewer not required (no §15/INV-SEC path; the gate/intent are unchanged); code-quality-reviewer per the every-slice policy.**

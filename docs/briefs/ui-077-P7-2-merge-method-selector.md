# /tdd brief — pr_merge_method_selector

## Feature
Add a **merge-method selector** (Merge commit / Squash and merge / Rebase and merge) to the PR
Workspace Merge control, threading the selected method through `onMerge(method)` →
`buildMergePrActionRequest({…, merge_method})` → `inputs.merge_method`. Completes §7.2's
"Approve/**Merge/Squash/Rebase**" surface on the already-live merge. USER-approved (all three);
**NON-cat-1 — rides the EXISTING live-merge gate** (no new go-live, no new mutation type; the daemon
already accepts merge|squash|rebase via `map_merge_method`, validated fail-closed; every merge stays
per-action approved + audited regardless of method).

## Use case + traceability
- **Task ID:** P7.2 (the PR Review Workspace — "Approve/Merge/**Squash/Rebase**/Request-changes"; `IMPLEMENTATION_PLAN.md` §7.2).
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (PR Review Workspace merge actions), `§7.2` (merge routes through the Gateway at risk≥3), `§6.3` (the `github.merge_pr` catalog action — `merge_method` is a catalog input).
- **Widens phase scope because** the new control rides the cross-cutting UI invariants every PR-Workspace control honors — `§11.4` (READ-ONLY/degraded gating — the method controls are `canMerge`-gated, method-agnostic) and `§11.6` (a11y MUSTs — the selector is keyboard-reachable + labeled, never color-alone). No new code in those subsystems; the selector inherits the existing gate + a11y patterns.
- **Related context:** the merge went LIVE @ui-075 (`b0ddc39`). The daemon's `github.merge_pr` (D9, `daemon/src/integrations/github_write.rs:map_merge_method`) ALREADY maps `merge`/`squash`/`rebase` → octocrab fail-closed (unknown → rejected). The UI currently hardcodes `merge_method: "merge"` (`DiffReview.tsx` `onMerge`). This slice threads the user's chosen method. USER-ruled (via lead, 2026-06-25): add all three; it rides the existing gate (non-cat-1).

## Acceptance criteria (what "done" means)
- [ ] The Merge control offers **all three** methods (`merge`/`squash`/`rebase`), GitHub-labeled (**Merge commit** / **Squash and merge** / **Rebase and merge**); keyboard-reachable + labeled (§11.6 — never color-alone; a labeled control).
- [ ] Selecting a method + Merge → `onMerge(method)` → `buildMergePrActionRequest({…, merge_method: <selected>})` → the chosen method rides into `inputs.merge_method` **verbatim** (for all three).
- [ ] **Default = `"merge"`** (merge-commit — the prior behavior preserved if no explicit selection).
- [ ] `canMerge` gating is **unchanged + uniform** across methods (`canSubmit && isPrMutationEnabled("github.merge_pr") && headSha != null`) — a gated PR can't be merged by any method; you can't pick a method to bypass the gate.
- [ ] The method value is constrained to the closed set `{merge, squash, rebase}` UI-side (a closed selector); the daemon is the authority (`map_merge_method` rejects unknown fail-closed) — the UI doesn't re-validate beyond offering the closed options.
- [ ] No new gate / no new `enabledPrMutations` entry / no contract change — the chosen method rides the EXISTING `github.merge_pr` live path → the GatewayModal approval reflects the method (the daemon's preview); every merge stays per-action approved.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`PrWorkspace.tsx` Merge control (currently `<Button onClick={onMerge}>Merge</Button>`, line ~303) → the new method selector + `onMerge(method)` → `PrWorkspaceContainer.onMerge` (`DiffReview.tsx:644`) accepts the method → `buildMergePrActionRequest` (`intent/pr-mutation-request.ts:48`, already takes `merge_method`) → the existing live L2 seam → `GatewayModal`. Same live path as ui-070; this only threads the method param. No new entry point.

## Files expected to touch
**Modified:**
- `src/views/code/PrWorkspace.tsx` — the merge-method selector control + `onMerge(method)` signature.
- `src/views/code/DiffReview.tsx` — `PrWorkspaceContainer.onMerge(method)` → pass to `buildMergePrActionRequest`.
- `src/intent/pr-mutation-request.ts` — a closed `MERGE_METHODS`/`MergeMethod` type (the 3 values) the selector + builder share (the builder already accepts `merge_method`).
- `src/views/code/PrWorkspace.test.tsx` · `src/views/code/DiffReview.test.tsx` · `src/intent/pr-mutation-request.test.ts` — the pins.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`merge_control_offers_all_three_methods`** — the selector renders merge/squash/rebase, GitHub-labeled. Why: §7.2 full merge surface.
2. **`selected_method_rides_into_merge_request`** — selecting `squash` (and `rebase`) + Merge → `buildMergePrActionRequest` forms `inputs.merge_method === "squash"` (resp. `"rebase"`). Why: §6.3 the catalog input rides verbatim.
3. **`merge_method_defaults_to_merge`** — no explicit selection → `inputs.merge_method === "merge"`. Why: §7.2 preserve prior behavior.
4. **`merge_method_controls_gated_with_canMerge`** — when `!canMerge`, the selector + Merge are disabled (no method bypasses the gate). Why: §11.2/§11.4 the gate is method-agnostic.
5. **`merge_method_value_is_closed_set`** (builder-level) — `buildMergePrActionRequest` only accepts the 3 typed values (tsc-enforced via `MergeMethod`); a round-trip pins each. Why: §6.3 closed input; the daemon validates fail-closed.

## Cross-doc invariant impact
- **Model field changes:** none — `merge_method` is an existing `github.merge_pr` input; the daemon already accepts all 3. No `shared/` change, no contract bump, no regen.
- **Orchestrator doc rows to write hot:** none (no cross-doc invariant). A minor convention candidate (a method/parameter selector on an already-live gated mutation rides the existing gate — non-cat-1; the daemon validates the parameter fail-closed; the per-action approval surfaces the chosen parameter) — orchestrator decides at Step 9.
- **Shared-contract seam model touched?** No.

## Things to flag at Step 2.5
1. **Control shape.** A native `<select>` (labeled "Merge method") + the Merge button · a split-button dropdown · 3 separate buttons. My default vote: **a native `<select>` + the Merge button** — keyboard-reachable + labeled + closed options out of the box (a11y-clean MVP; GitHub's split-button is a nicety, not required). Confirm.
2. **Default method.** My default vote: **`"merge"`** (merge-commit) — preserves the ui-075 behavior.
3. **Squash/rebase destructiveness affordance.** Squash/rebase rewrite history. My default vote: **standard GitHub labels, no extra danger styling** — the per-action `GatewayModal` approval + the daemon risk-3 is the gate (the method shows in the approval card), and the user explicitly approved all three. Add a subtle history-rewrite hint only if you think it earns its keep (flag).
4. **Per-method enablement.** My default vote: **uniform `canMerge`** (no per-method gating) — the gate is method-agnostic.

## Dependencies + sequencing
- **Depends on:** ui-075 (✅ the live merge), ui-076 (queued before — slice atomicity; no shared files). The daemon `map_merge_method` (✅ accepts all 3, on track/ui via the 0.45 sync).
- **Blocks:** nothing.

## Estimated commit count
**1.** A focused method-selector threading one input through the existing live merge path.

## Visual gate (orchestrator-ruled: a SCOPED UI-correctness gate — NOT a sign-off)
The user's sign-off on all three methods is DONE (lead-confirmed 2026-06-25) — there is **NO new cat-1 go-live sign-off** (the merge is already live; this exposes daemon-supported methods). The only open item is **build-discipline**: the method selector is a NEW rendered control, so per [[10]]/[[12]] (green ≠ looks right) it gets a **small SCOPED HITL visual gate** — a UI-correctness pixel-check, NOT a sign-off.
- **Scope:** ONLY the merge-method selector + the Merge control region, vs the Graphite-Arc prototype (alignment + kit-consistent styling + the selector doesn't crowd the verdict controls).
- **When:** post-build (after GREEN), via the existing ui-075 `dev:mock` harness (`pnpm dev:mock` → Code/Diff → Pull requests → a PR). The orchestrator requests it through the lead → user when you signal Step-7.5/ready.
- **Blocking?** It does NOT block the code landing (push is HELD; nothing ships externally; non-cat-1) — but it IS the "looks right" confirmation; a fail → a quick polish follow-up, not a revert.
- Pick a **kit-consistent control** at Step-2.5 Q1 (the existing dropdown/select primitive) to keep the visual risk low.

## Security review
**RECOMMENDED at Step 8** (the `invariant` policy) even though the lead classified the slice non-cat-1: it touches the LIVE `github.merge_pr` request-formation. Cheap insurance — the reviewer confirms the UI stays a pure submitter (the method is the daemon's to validate/approve/audit; no new gate; no method bypasses `canMerge`). Surface any finding as a Step-9 Finding.

## Lessons-logged candidates anticipated
- **Convention candidate (minor)** — a method/parameter selector on an ALREADY-live gated mutation rides the existing gate (non-cat-1, no new go-live); the daemon validates the parameter fail-closed; the per-action approval surfaces the chosen parameter; the gate stays parameter-agnostic. Extends [[27]]/[[36]].

## How to invoke
1. **Read this brief end-to-end** + the Merge control (`PrWorkspace.tsx:295-312`) + `onMerge`/`buildMergePrActionRequest` (`DiffReview.tsx:644`, `pr-mutation-request.ts:48`).
2. **Run `/tdd pr_merge_method_selector`.**
3. **Step 2.5** — ping me with the test-design write-up + the 4 design-Q answers/defaults.
4. **Step 8** — run the security-reviewer (recommended; touches the live merge path).
5. **Step 9** — surface anything beyond the anticipated convention candidate.

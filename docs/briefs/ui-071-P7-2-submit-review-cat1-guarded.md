# /tdd brief — submit_review_cat1_guarded_disabled

## Feature
The **`github.submit_review` (Approve / Request-changes / Comment) cat-1 mutation, built GUARDED-DISABLED**, AND a **per-action gate refactor** (user fork-1 = 1b): replace the single `prMutationsEnabled` boolean with an **action-type-keyed enablement set** so the future go-live flip can stage lowest-risk-first (enable review-submit before merge), folding the ui-070 merge_pr flag into the same mechanism. Form the typed `submit_review` `ActionRequest` (all THREE verdicts; SHA-pinned to the **displayed** head via `commit_id`; conditional-required `body`), wire the PrWorkspace Approve/Request-changes/**Comment** controls through the existing L2 seam + GatewayModal, all HELD. **No go-live flip.** cat-1 — **security-reviewer mandatory**.

## User-ruled decisions baked in (2026-06-20, via lead)
- **Fork 1 = 1b (per-action gate):** refactor the gate from `prMutationsEnabled: boolean` to an **action_type-keyed map/set** (e.g. `enabledPrMutations: ReadonlySet<string>`); **fold merge_pr into the SAME structure** (no separate mechanisms). Both `github.merge_pr` + `github.submit_review` default DISABLED (empty set on Uds). This only shapes the FUTURE flip granularity — both stay HELD now.
- **Fork 2 = 2b (all three verdicts):** ship **approve | request_changes | comment** — a THIRD "Comment" control beyond the prototype's two, rendered disabled like the others. **Body-required:** optional for `approve`; **non-empty required for `request_changes` AND `comment`** (GitHub 422s otherwise) → a body text input + the deny-reason-precedent trim guard ([[33]]) applies to both.
- **Ruling A (carried):** the UI sends a `repo` resource_ref + `inputs:{pr_number, commit_id, event, body?}` and **NEVER names owner/repo** — the daemon resolves owner/repo from `repo_id` (the bundled cross-track daemon resolution covers BOTH github writes; already routed). 
- **SHA-pin (D2):** `commit_id` = the **displayed** head_sha (via the same `prHeadSha` helper — null until the daemon head_sha field + ruling-A resolution land cross-track → controls disabled). A daemon 409 (head moved) → the honest re-review affordance.
- **A3:** build guarded NOW; go-live flip HELD for explicit user sign-off + the daemon auth-bootstrap re-review.

## Use case + traceability
- **Task ID:** P7.2 (the PR Review Workspace; the cat-1 PR-mutations arc — daemon D10 `github.submit_review`; B2 second slice after merge_pr).
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (the PR Workspace review controls), `§7.2` (PR is GitHub-authoritative).
- **Widens phase scope because:** a cat-1 mutation extends the **§6.1** GatewayPort mutation surface + the **§6.2** ActionRequest/Approval flow + the **§15/INV-SEC-1** chokepoint (UI = pure intent-submitter, defense-in-depth) + the **§11.5** approval card — the same cat-1 anchor set as ui-070/L2. The daemon `github.submit_review` catalog/event are frozen @0.42 — **no new contract, no regen**.
- **Related context:** **ui-070** (`docs/briefs/ui-070-P7-2-merge-pr-cat1-guarded.md`) is the verbatim pattern — this slice MIRRORS it + generalizes the gate. The merge_pr files to refactor: `ui/src/intent/merge-pr-request.ts` (`PR_MUTATION_ACTION_TYPES` + `buildMergePrActionRequest`), `ui/src/gateway-client/{types,uds,mock}.ts` (the `prMutationsEnabled` field + the `uds.ts:341` port guard), `ui/src/views/code/{PrWorkspace,DiffReview}.tsx` (the `canMerge` enablement read at `DiffReview.tsx:610`). The daemon executor inputs (daemon LESSON 61): `SubmitReviewArgs{owner, repo, pr_number, commit_id, event, body}` — owner/repo daemon-resolved (ruling A); `event` ∈ approve|request_changes|comment; `body` conditional. LESSONS [[16]]/[[17]] (pure submitter / daemon-driven card), [[19]] (submitted==displayed SHA), [[27]]/[[28]] (the guarded-disabled→flip mechanism), [[33]] (trim client input feeding an audited field — the `body`).

## Acceptance criteria (what "done" means)
**Layer 1 — the gate refactor + intent-formation (the cat-1 safety core; own commit):**
- [ ] **Gate refactor (1b):** replace `prMutationsEnabled: boolean` (GatewayPort + UdsGatewayPort + MockGatewayPort) with an action-type-keyed enablement — `enabledPrMutations: ReadonlySet<string>` (the SET of enabled PR-mutation action types). **UdsGatewayPort default = empty set** (all HELD); **MockGatewayPort default = the full `PR_MUTATION_ACTION_TYPES`** (dev/test works). A helper `isPrMutationEnabled(gateway, actionType): boolean`.
- [ ] **`PR_MUTATION_ACTION_TYPES` extends** to `{"github.merge_pr", "github.submit_review"}`.
- [ ] **Port guard (map-based, both types):** `UdsGatewayPort.submit_action` throws + never invokes when `PR_MUTATION_ACTION_TYPES.has(action_type) && !enabledPrMutations.has(action_type)` — independent of `mutationsEnabled`. Covers merge_pr AND submit_review uniformly.
- [ ] **merge_pr call sites updated** to the new mechanism (the `canMerge` read at `DiffReview.tsx:610` → `isPrMutationEnabled(gateway,"github.merge_pr")`). **No behavior change for merge** (still disabled in production; the ui-070 merge tests stay green, adjusted to the new field).
- [ ] `buildSubmitReviewActionRequest({repo_id, pr_number, head_sha, event, body})` → a typed `ActionRequest` (mirror `buildMergePrActionRequest`): `action_type:"github.submit_review"`, `requester_type:"user"`/`"current_user"`, `action_request_id:""`, `resource_refs:[{type:"repo", id:repo_id}]`, `inputs:{pr_number, commit_id:head_sha, event, body}` (body **trimmed** before inclusion, [[33]]; `body` included as `""` for approve-with-no-body, the trimmed non-empty text otherwise). **head_sha REQUIRED.**
- [ ] **The no-production-flip pin generalizes** — a repo-wide scan asserts no production source constructs `UdsGatewayPort` with a non-empty `enabledPrMutations` (the ui-070 [[28]]-analogue, now covering both mutations).

**Layer 2 — the consumer (guarded-disabled wiring):**
- [ ] PrWorkspace gains **three** review controls — **Approve PR** / **Request changes** / **Comment** (the first two exist disabled; add Comment) — each raising an `onSubmitReview(event)` callback; the container forms `buildSubmitReviewActionRequest` + submits via the seam + opens the **GatewayModal** (reuse).
- [ ] A **body text input** (shared across the verdicts; the deny-reason-precedent control). **Conditional-required enablement:** a verdict control is enabled ONLY when `canSubmitIntent && isPrMutationEnabled(gateway,"github.submit_review") && headSha != null` **AND** (`event === "approve"` OR `body.trim()` non-empty). approve enables with an empty body; request_changes/comment require a non-empty trimmed body.
- [ ] **cat-1 invariants (verbatim):** no-optimistic-"done" (status from the daemon ack only); §6.4 codes routed VERBATIM (reuse `describeRejection`); a daemon failure (incl. 409 head-moved) → the honest re-review affordance (reuse the ui-070 `pr-merge-rereview` pattern → generalize to a per-mutation re-review); `policy_grant` "always allow" stays disabled-pinned (submit_review is daemon-non-standing-grantable).
- [ ] **PrWorkspace stays display-oriented** (the cat-1 submission lives in the container — the ui-064/ui-070 pin); **no gateway prop** on PrWorkspace.
- [ ] **NO production path reaches a live review-submit** — verified repo-wide (control disabled / container guards / port throw-never-invoke). **The go-live flip is NOT in this slice.**
- [ ] `/preflight` clean; **security-reviewer CLEAR both layers** (mandatory).

## Wiring / entry point (Step 7.5)
The Approve/Request-changes/Comment controls in `PrWorkspace` (Review-tab PR Workspace) → `onSubmitReview(event)` → the container forms + submits the `github.submit_review` `ActionRequest` → GatewayModal. **Gated OFF** (`enabledPrMutations` empty by default + head_sha absent). Confirm at Step 7.5 that no production construction populates `enabledPrMutations`.

## Files expected to touch
**New:**
- `ui/src/intent/submit-review-request.ts` — `buildSubmitReviewActionRequest` + tests. (Or extend `merge-pr-request.ts` → rename to a generic `pr-mutation-request.ts`; implementer's call at Step 2.5.)

**Modified:**
- `ui/src/intent/merge-pr-request.ts` — `PR_MUTATION_ACTION_TYPES` += `github.submit_review`.
- `ui/src/gateway-client/types.ts` — `prMutationsEnabled: boolean` → `enabledPrMutations: ReadonlySet<string>` + `isPrMutationEnabled`.
- `ui/src/gateway-client/uds.ts` — default empty set; the map-based port guard.
- `ui/src/gateway-client/mock.ts` — default full set.
- `ui/src/views/code/PrWorkspace.tsx` — the 3 review controls + the body input + per-verdict enablement; the `canMerge` read via `isPrMutationEnabled`.
- `ui/src/views/code/DiffReview.tsx` (container) — `onSubmitReview` form+submit+GatewayModal; the merge call-site update; the re-review affordance generalization.
- the ui-070 + Shell test files — adjust to the new field (tsc surfaces them).
- test files for each.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
**Layer 1:**
1. `pr_mutation_action_types_contains_both` — Asserts: `PR_MUTATION_ACTION_TYPES` = {merge_pr, submit_review}. Why: the gate set covers both.
2. `enabled_pr_mutations_defaults_empty_uds_full_mock` — Asserts: `new UdsGatewayPort().enabledPrMutations` is empty; Mock is the full set. Why: 1b held-by-default / dev-works.
3. `is_pr_mutation_enabled_per_action` — Asserts: `isPrMutationEnabled` is true only for a type IN the set. Why: per-action gate.
4. `uds_submit_review_throws_never_invokes_when_not_enabled` — Asserts: a `github.submit_review` submit with submit_review NOT in `enabledPrMutations` throws + never invokes, even with `mutationsEnabled:true` AND merge_pr enabled (independence). Why: [[27]] provably-unreachable + per-action independence.
5. `uds_merge_pr_still_gated_after_refactor` — Asserts: merge_pr throws-never-invokes unless merge_pr ∈ the set (no regression). Why: fold-in correctness.
6. `build_submit_review_action_request_shape` — Asserts: action_type `github.submit_review`, `resource_refs:[{type:"repo",id}]`, `inputs:{pr_number, commit_id, event, body}`, no owner/repo. Why: §6.2 / ruling A / daemon LESSON 61.
7. `submit_review_pins_displayed_head_sha_as_commit_id` — Asserts: `inputs.commit_id` == the displayed head_sha. Why: [[19]]/D2.
8. `submit_review_body_trimmed` — Asserts: body is `.trim()`'d in the formed inputs (whitespace→empty for approve; the trimmed text otherwise). Why: [[33]].
9. `production_construction_never_enables_pr_mutations` — Asserts (repo-wide): no production source sets a non-empty `enabledPrMutations`. Why: the held-flip discipline (both mutations).

**Layer 2:**
10. `review_controls_disabled_unless_enabled_and_head_sha_and_can_submit` — Asserts: each verdict control disabled when any of canSubmit/`isPrMutationEnabled(submit_review)`/headSha is false. Why: defense-in-depth layer 1.
11. `request_changes_and_comment_require_nonempty_body` — Asserts: request_changes/comment controls disabled when `body.trim()` is empty; approve enabled with an empty body. Why: fork-2b conditional-required.
12. `submit_review_click_forms_and_submits_then_opens_gateway_modal` — Asserts: clicking a verdict (enabled) forms `buildSubmitReviewActionRequest(event)` + submits + opens the GatewayModal; PrWorkspace takes NO gateway prop. Why: §11.2 + the ui-064/070 pin.
13. `submit_review_no_optimistic_done` — Asserts: the UI never shows a "reviewed" success until a confirming daemon `ActionResult`. Why: [[16]]/[[17]].
14. `submit_review_failure_is_honest_re_review` — Asserts: a daemon failure → the verbatim §6.4 verdict + the honest re-review affordance, never a fabricated success. Why: §11.7/D2.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none to the frozen `shared/` contract (consumes the frozen `github.submit_review` catalog/event @0.42; no regen). The `prMutationsEnabled`→`enabledPrMutations` change is a UI-side GatewayPort field refactor.
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` "GatewayPort mutation surface" + "Live UdsGatewayPort transport client" rows — update the merge_pr note to the per-action gate + add the `github.submit_review` cat-1 (guarded-disabled) consumer + the 3 verdicts. **Orchestrator-territory — flag, don't edit.**
- **§2.5-seam:** none new.

## Things to flag at Step 2.5
1. **Gate shape.** `enabledPrMutations: ReadonlySet<string>` + `isPrMutationEnabled` vs a `Record<actionType, boolean>` vs a port method. My default vote: **`ReadonlySet<string>`** (cleanest; the guard is a `.has`; Mock = the full set). Confirm.
2. **Intent-file organization.** A new `submit-review-request.ts` vs renaming `merge-pr-request.ts` → a generic `pr-mutation-request.ts` holding both builders + the shared set. My default vote: **rename to `pr-mutation-request.ts`** (the gate set + both builders are one concept now — fold the enrichHunkAction→enrichActionApproval cleanup if cheap, else leave it). Implementer's call.
3. **Body input placement.** A single shared body `<textarea>` in PrWorkspace (raised to the container on submit) vs in the GatewayModal. My default vote: **in PrWorkspace** (the body is part of forming the intent, before the modal — mirror how the verdict is chosen pre-submit; the GatewayModal stays the daemon-verdict renderer).
4. **Comment control placement/labeling.** A 3rd button alongside Approve PR / Request changes. My default vote: **yes, a sibling button** (no nested-interactive; disabled like the others; glyph+label).
5. **Re-review affordance generalization.** ui-070's `pr-merge-rereview` button → a per-mutation re-review (merge + review). My default vote: **generalize it** (one honest re-review affordance keyed to the failed mutation).

## Dependencies + sequencing
- **Depends on:** ui-070 (the merge_pr cat-1 gate + GatewayModal wiring; landed `9af3718`); the daemon `github.submit_review` catalog/event (landed via merge). **Parallel (A3):** the daemon head_sha + ruling-A owner/repo-resolution (routed cross-track) — the `commit_id` pin source once it lands.
- **Blocks:** the **go-live flip** (a future user-signed-off slice — now per-action via `enabledPrMutations`, stage-able); PR-per-hunk (rides D7's diff + a daemon PR-per-hunk action).

## Estimated commit count
**2** (cat-1, multi-commit; I drive layer→layer; security-reviewer reviews the whole slice diff at Step 8):
- **Layer 1 (own commit — the cat-1 safety core):** the gate refactor (bool→per-action set, fold merge_pr in) + `buildSubmitReviewActionRequest` + the map-based port guard + the merge call-site update + the generalized no-flip pin. Exposed-ahead; provably unreachable.
- **Layer 2:** the 3 review controls + the body input + the container submit/GatewayModal + per-verdict enablement + the honest re-review. Still guarded-disabled.

## Lessons-logged candidates anticipated
- **Convention candidate** — "a 2nd cat-1 mutation in the same family generalizes the go-live gate from a boolean to a per-action set (stage-able flip) + folds the 1st mutation into it — one mechanism, independent enablement." (extends [[27]]/[[28]].)
- **Future TODO — next-brief working set** — the go-live flip (now per-action, user-signed-off + auth-bootstrap re-review); the `prHeadSha` reconcile when the daemon field lands; the `enrichHunkAction→enrichActionApproval` rename (if not folded here).
- **Architecture-doc note** — the review verdict pins the displayed head via `commit_id`; the daemon 422/409s on a body/head violation.

## How to invoke
1. Read this brief end-to-end (don't skip "Things to flag at Step 2.5").
2. Run `/tdd submit_review_cat1_guarded_disabled`.
3. Step 0 (Restate) — confirm it matches the Feature line.
4. Step 2.5 — ping back the test design (one `Asserts: <invariant> (§anchor)` per test + the coverage map) with answers to the 5 design questions (or take defaults). **Escalate any safety-design question before GREEN.**
5. **Step 8 — security-reviewer (MANDATORY, cat-1).** Surface its findings at Step 9.
6. Step 9 — categorized flags + the cross-doc note + the ship-ask.

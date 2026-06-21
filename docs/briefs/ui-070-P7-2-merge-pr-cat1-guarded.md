# /tdd brief — merge_pr_cat1_guarded_disabled

## Feature
The **`github.merge_pr` (Merge) cat-1 mutation, built GUARDED-DISABLED** — form the typed `ActionRequest` (SHA-pinned to the **displayed** head_sha; fixed merge-commit method), wire the PrWorkspace **Merge** control through the EXISTING L2 mutation seam + the GatewayModal approval, gated behind a **NEW `prMutationsEnabled` flag (default FALSE)** so NO production path reaches a live merge. **The go-live flip is NOT in this slice** — it stays HELD for explicit user sign-off (+ the daemon auth-bootstrap re-review). cat-1 — **security-reviewer mandatory**.

## User-ruled decisions baked in (2026-06-20, via lead)
- **B2:** `github.merge_pr` (Merge) ONLY this slice; `github.submit_review` follows as its own later cat-1 slice.
- **C1:** a **NEW `prMutationsEnabled`** GatewayPort flag, **default false** — SEPARATE from the already-live L2 `mutationsEnabled` flag (which is TRUE in production; riding it would make merge go live the instant it's built — wrong for a cat-1). Provably unreachable in production.
- **D2:** pin the **DISPLAYED** head_sha (submitted == displayed, [[19]]); a daemon 409 (head moved) → an honest "PR moved — re-review" treatment, never a silent/forced merge of an unreviewed head.
- **merge-method:** fixed **merge-commit** (`"merge"`); the squash/rebase selector is DEFERRED.
- **A3:** build guarded-disabled NOW; the daemon `head_sha`-on-`PullRequestRow` exposure is routed cross-track in PARALLEL (user-authorized) — the control consumes it as the pin source once it lands + a follow-up shadow-reconcile adds the field; **the disabled control does NOT block on it** (it renders disabled while head_sha is absent AND the flag is false).

## Use case + traceability
- **Task ID:** P7.2 (the PR Review Workspace; the cat-1 PR-mutations arc — daemon D9 `github.merge_pr`).
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (the PR Workspace Merge control), `§7.2` (PR is GitHub-authoritative).
- **Widens phase scope because:** a cat-1 mutation extends the **§6.1** GatewayPort mutation surface + the **§6.2** ActionRequest/Approval flow + the **§15/INV-SEC-1** chokepoint (UI = pure intent-submitter, defense-in-depth) + the **§11.5** approval card — the established L2/6.3e cat-1 anchor set (the GatewayPort-mutation-surface + intent-seam cross-doc rows already carry §6.1/§6.2/§4.2/§15/§11.5). The daemon `github.merge_pr` catalog/event are frozen @0.41–0.42 — **no new contract, no regen**.
- **Related context:** the **L2 precedents** — LESSONS [[16]] (the intent-submission seam: pure submitter, no-optimistic-render, §6.4 verbatim), [[17]] (the GatewayModal daemon-driven card), [[26]]/[[27]]/[[28]] (the mutation transport + the `mutationsEnabled` guarded-disabled→user-flip pattern), [[19]] (submitted==displayed resource precision); the **6.3e** DiffReview per-hunk-action precedent (forms a typed `ActionRequest` through the seam + opens its own GatewayModal). The intent-forming precedent to MIRROR: `ui/src/intent/hunk-resource-ref.ts` `buildHunkActionRequest` (action_request_id="" daemon-mints, requester "user"/"current_user", risk_level a never-displayed hint, created_at = submit time). The flag to mirror: `mutationsEnabled` (`types.ts:111` readonly field · `uds.ts` default false / throw-never-invoke · `mock.ts` default true). The daemon executor inputs: `MergePrArgs{owner,repo,pr_number,sha,merge_method}`. **⚠️ FINDING-resolved (ruling A, 2026-06-20):** the as-built `execute_merge_pr` currently reads `inputs["owner"]`/`["repo"]` LITERALLY (the create_pr precedent) — but **under ruling A the daemon will resolve owner/repo from the `repo_id`** (the D7 `get_pr_diff` / daemon-LESSON-59 pattern; a bundled cross-track daemon change to `execute_merge_pr`, routed with the head_sha ask). So the UI sends ONLY `resource_refs:[{type:"repo", id:repo_id}]` + `inputs:{pr_number, sha:<head_sha>, merge_method:"merge"}` and **NEVER names the GitHub target** (no confused-deputy/owner-spoof surface — the merged target == the audited identity by construction). The UI shape here is forward-correct to the A-daemon (daemon LESSON 60 + 59).

## Acceptance criteria (what "done" means)
**Layer 1 — intent-formation + the guard (the cat-1 safety core; own commit):**
- [ ] `buildMergePrActionRequest({repo_id, pr_number, head_sha, merge_method})` → a typed `ActionRequest` (mirror `buildHunkActionRequest`): `action_type:"github.merge_pr"`, `requester_type:"user"`, `requester_id:"current_user"`, `action_request_id:""` (daemon mints), `resource_refs:[{type:"repo", id:repo_id}]`, `inputs:{pr_number, sha:head_sha, merge_method}` (opaque passthrough), `created_at`=submit time. **head_sha is REQUIRED** (no head_sha → the caller cannot form the intent → the control is disabled).
- [ ] A NEW `prMutationsEnabled: boolean` readonly GatewayPort field — **default FALSE on `UdsGatewayPort`** (production), **default true on `MockGatewayPort`** (test/dev) — mirroring `mutationsEnabled`.
- [ ] **Port guard (provably-unreachable layer):** `UdsGatewayPort.submit_action` **throws + never invokes** when the request's `action_type` is a PR-mutation type (`github.merge_pr`) AND `prMutationsEnabled` is false — independent of `mutationsEnabled`. (A `PR_MUTATION_ACTION_TYPES` set, extensible for `submit_review` later.)

**Layer 2 — the consumer (guarded-disabled wiring):**
- [ ] The PrWorkspace **Merge** control (currently disabled, `PrWorkspace.tsx:104-110`) raises an `onMerge` callback; the **container** (DiffReview/PrWorkspaceContainer — owns the gateway) forms `buildMergePrActionRequest` + submits via the seam + opens the **GatewayModal** (reuse) for the daemon's `PolicyDecision`/`ActionPreview`/approve/deny.
- [ ] **Enablement (defense-in-depth layer 1):** Merge is enabled ONLY when `canSubmitIntent && prMutationsEnabled && headSha != null`. **PrWorkspace stays display-oriented** (the cat-1 submission lives in the container, not PrWorkspace — the ui-064/ui-069 pattern).
- [ ] **head_sha source:** a single `prHeadSha(pr): string | null` helper returns **null today** (the field isn't on `PullRequestRow` yet) → the control renders disabled. A code comment marks it: "returns `pr.head_sha` once the daemon D-head_sha freeze + a follow-up shadow-reconcile add the field (A3 parallel work)." NO change to the `PullRequestRow` shadow this slice.
- [ ] **D2 head-moved ([[19]]):** the intent pins the displayed head_sha; a merge failure surfaces the daemon's verdict honestly via the GatewayModal (no-optimistic-done) + an honest "PR may have moved — re-review" affordance (re-fetch the PR). **No fabricated distinct card** if the daemon returns a generic failure (verify the daemon's 409 surface at Step 2.5).
- [ ] **cat-1 invariants (verbatim):** no-optimistic-"done" (status from the daemon ack only; "merged" only on the confirming `ActionResult`/the `PullRequestMerged` projection fold); §6.4 codes routed VERBATIM by `describeRejection` (reuse); the `policy_grant` "always allow" stays disabled-pinned (merge_pr is daemon-non-standing-grantable).
- [ ] **NO production path reaches a live merge** — verified repo-wide (the L2 [[27]] discipline): control disabled / container guards on `prMutationsEnabled` / port throw-never-invoke. **The go-live flip is NOT in this slice.**
- [ ] `/preflight` clean; **security-reviewer CLEAR** (Step 8, mandatory).

## Wiring / entry point (Step 7.5)
The Merge control in `PrWorkspace` (reached from the Review-tab PR Workspace) → `onMerge` → the container forms + submits the `github.merge_pr` `ActionRequest` → GatewayModal. **Gated OFF in production** (`prMutationsEnabled` false by default + head_sha absent). The flip (`new UdsGatewayPort({prMutationsEnabled:true})`) is a future user-signed-off slice — NOT here. Confirm at Step 7.5 that no production construction sets `prMutationsEnabled:true`.

## Files expected to touch
**New:**
- `ui/src/intent/merge-pr-request.ts` — `buildMergePrActionRequest` + `PR_MUTATION_ACTION_TYPES` + tests.

**Modified:**
- `ui/src/gateway-client/types.ts` — `prMutationsEnabled` field + the `submit_action` doc.
- `ui/src/gateway-client/uds.ts` — default false + the PR-mutation port guard (throw-never-invoke).
- `ui/src/gateway-client/mock.ts` — default true.
- `ui/src/views/code/PrWorkspace.tsx` — the Merge control raises `onMerge`; enablement gate; `prHeadSha` helper.
- `ui/src/views/code/DiffReview.tsx` (the container) — form + submit + GatewayModal; the 409 re-review affordance.
- the `Shell`/any GatewayPort stub test files — add `prMutationsEnabled` (tsc will surface them).
- test files for each.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
**Layer 1:**
1. `build_merge_pr_action_request_shape` — Asserts: the ActionRequest carries `action_type:"github.merge_pr"`, `resource_refs:[{type:"repo",id}]`, `inputs:{pr_number,sha,merge_method:"merge"}`, requester user/current_user, empty id. Why: §6.2 / daemon LESSON 60.
2. `merge_pr_intent_pins_displayed_head_sha` — Asserts: the `inputs.sha` == the head_sha passed (the displayed one), NOT re-fetched. Why: [[19]] / D2.
3. `uds_submit_action_throws_never_invokes_merge_pr_when_pr_mutations_disabled` — Asserts: a `github.merge_pr` submit_action with `prMutationsEnabled:false` THROWS + the underlying invoke is never called (even with `mutationsEnabled:true`). Why: [[27]] provably-unreachable.
4. `uds_submit_action_invokes_merge_pr_when_pr_mutations_enabled` — Asserts: with `prMutationsEnabled:true` it invokes. Why: the enabled path works.
5. `non_pr_mutation_submit_action_unaffected_by_pr_flag` — Asserts: an L2 approve/deny/other submit_action is gated by `mutationsEnabled` only, NOT `prMutationsEnabled`. Why: no regression to L2.

**Layer 2:**
6. `merge_control_disabled_unless_enabled_and_head_sha_and_can_submit` — Asserts: Merge is disabled when any of `canSubmitIntent`/`prMutationsEnabled`/`headSha` is false/absent; enabled only when all three hold. Why: defense-in-depth layer 1.
7. `merge_click_forms_and_submits_then_opens_gateway_modal` — Asserts: clicking Merge (enabled) forms `buildMergePrActionRequest` + calls submit + opens the GatewayModal; PrWorkspace takes NO gateway prop (the container owns it). Why: §11.2 + the ui-064 pin.
8. `merge_no_optimistic_done` — Asserts: the UI never shows "merged" until a confirming `ActionResult`/projection; a pending/failed daemon state is rendered honestly. Why: [[16]]/[[17]] no-optimistic-done.
9. `merge_failure_is_honest_re_review_not_fabricated` — Asserts: a daemon merge failure surfaces the verbatim §6.4/verdict via the GatewayModal + the honest re-review affordance, never a fabricated success/card. Why: §11.7/D2.
10. `production_construction_never_sets_pr_mutations_enabled` — Asserts (repo-wide grep/test): no production `new UdsGatewayPort({prMutationsEnabled:true})`. Why: the held-flip cat-1 discipline.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none to the frozen `shared/` contract (consumes the already-frozen `github.merge_pr` catalog/event @0.41–0.42; no regen). The `prMutationsEnabled` field is a UI-side GatewayPort addition.
- **Orchestrator doc rows to write hot (Step 9):** the `ui/CLAUDE.md` "GatewayPort mutation surface + the intent seam" + "Live UdsGatewayPort transport client" rows gain a `github.merge_pr` cat-1 (guarded-disabled, `prMutationsEnabled`) note. **Orchestrator-territory — flag, don't edit.**
- **§2.5-seam:** none new (no shadow/contract change).

## Things to flag at Step 2.5
1. **The daemon 409/head-moved surface.** Does `github.merge_pr` return a DISTINGUISHABLE signal on a SHA-mismatch (head moved), or a generic ActionResult Failed? My default vote: **treat it as the honest GatewayModal failure + a re-fetch "re-review" affordance**; only add a distinct "PR moved" card IF the daemon returns a distinguishable code (verify first — don't fabricate).
2. **head_sha source helper.** `prHeadSha(pr)` returns null until the daemon field + reconcile land. My default vote: **isolate to one helper** (the A3 dependency seam); control disabled when null.
3. **Where the gate lives.** Both the port (`submit_action` throw-never-invoke for PR-mutation types when `!prMutationsEnabled`) AND the control (disabled). My default vote: **both** (defense-in-depth, the L2 [[27]] pattern).
4. **Container ownership.** PrWorkspace raises `onMerge`; the container (DiffReview) forms+submits+GatewayModal. My default vote: **yes** — preserves PrWorkspace display-orientation (the ui-064/ui-069 pin + the 6.3e per-hunk precedent).
5. **merge_method.** Fixed `"merge"` in `inputs`, no selector (user-ruled). Confirm.

## Dependencies + sequencing
- **Depends on:** ui-069 (the PR Workspace consumer; landed `de1996a`); the daemon `github.merge_pr` catalog/event (landed via merge). **Parallel (A3):** the daemon `head_sha`-on-`PullRequestRow` exposure (routed cross-track) — the control's pin source once it lands (a follow-up shadow-reconcile wires `prHeadSha`).
- **Blocks:** the **go-live flip** (a future user-signed-off slice — `prMutationsEnabled:true` + the auth-bootstrap re-review); `github.submit_review` (the next cat-1 slice, reuses `PR_MUTATION_ACTION_TYPES` + the GatewayModal); PR-per-hunk (rides D7's diff + a daemon PR-per-hunk action).

## Estimated commit count
**2** (cat-1, multi-commit; I drive layer→layer; **security-reviewer reviews the whole slice diff at Step 8**):
- **Layer 1 (own commit — the cat-1 safety core):** `buildMergePrActionRequest` + `PR_MUTATION_ACTION_TYPES` + the `prMutationsEnabled` field + the port throw-never-invoke guard. Exposed-ahead; provably unreachable.
- **Layer 2:** the PrWorkspace Merge control + the container form/submit/GatewayModal + the enablement gate + the honest 409 re-review. Still guarded-disabled.
The safety-critical gate/intent-formation is its OWN commit (the safety-slice-own-commit rule); the consumer is the second. Neither flips the go-live.

## Lessons-logged candidates anticipated
- **Convention candidate** — "a cat-1 mutation that reuses the L2 seam needs its OWN guard flag when the base `mutationsEnabled` is already live — the new flag (default false) + a `*_ACTION_TYPES` port guard preserves the held→flip discipline." (extends [[27]]/[[28]].)
- **Architecture-doc note** — the merge intent pins the displayed head_sha; the daemon 409s on a moved head (the SHA-pin's purpose).
- **Future TODO — next-brief working set** — the go-live flip (user-signed-off + auth-bootstrap re-review); `github.submit_review` (B2 second slice); the `prHeadSha` reconcile when the daemon field lands.

## How to invoke
1. Read this brief end-to-end (don't skip "Things to flag at Step 2.5").
2. Run `/tdd merge_pr_cat1_guarded_disabled`.
3. Step 0 (Restate) — confirm it matches the Feature line.
4. Step 2.5 — ping back the test design (one `Asserts: <invariant> (§anchor)` per test + the coverage map) with answers to the 5 design questions (or take defaults). **Escalate any safety-design question before GREEN.**
5. **Step 8 — security-reviewer (MANDATORY, cat-1).** Surface its findings at Step 9.
6. Step 9 — categorized flags + the cross-doc note + the ship-ask.

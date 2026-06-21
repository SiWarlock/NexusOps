# ui-022 — PR Review Workspace: D6 diff-stats · D7 code-diff · merge_pr + submit_review cat-1 (guarded)

- **Date:** 2026-06-21
- **Phase:** Phase 7 (Integrations / PR Review Workspace) — the ui-half P7.2 cat-1 PR-mutations arc
- **Track:** ui (`track/ui`)
- **Predecessor:** [ui-021](ui-021-2026-06-17-quality-hardening-bundle.md)
- **Successor:** _(none yet — track pauses post-arc, daemon-gated on head_sha/owner-repo)_

## Why this session existed

The daemon §4.7 PR-unblock wave (D6/D7/D9/D10) landed on `main` (HEAD `2bd8db9`, CONTRACT 0.42), so the ui track resumed (post the ui-004 daemon-gated pause) to consume it: the PR-card diff-stats (D6), the PR code-diff (D7), and the two cat-1 PR-mutations (D9 `github.merge_pr`, D10 `github.submit_review`) — all built **guarded-disabled** (no go-live flip this round).

## What was built (4 slices, 8 commits)

### ui-068 — regen-to-green 0.42 + D6 PR-card diff-stats (`24d9e07`, `1eaf496`)
- **Modified:** `src/contracts/generated.ts` (staged the merge-synced regen → CONTRACT 0.38→0.42, idempotent) · `src/contracts/provisional.ts` (`PullRequestRow` shadow 11→15: +`additions`/`deletions`/`changed_files`/`commits`, u64 nullable-optional, `.strict()` preserved) · `src/gateway-client/mock.test.ts` (version tripwire 0.38→0.42) · `src/views/code/PrWorkspace.tsx` (a `DiffStats` component — real null-safe `+additions/−deletions/N files/M commits`, D6 placeholder retired) · the 2 test files.

### ui-069 — D7 get_pr_diff PR code-diff detail view (`3c8c86d`, `de1996a`)
- **New:** none (transport mirrored existing get_diff). **Modified:** `gateway-uds/src/lib.rs` (`get_pr_diff` crate helper) · `src-tauri/src/commands.rs` + `src-tauri/src/lib.rs` (`gateway_get_pr_diff` command + allowlist) · `src/gateway-client/{types,uds,mock}.ts` (`get_pr_diff` GatewayPort method, reuses `parseDiff`) · `src/views/code/PrWorkspace.tsx` (`PrDiffState` union + `PrCodeDiff` read-only render via kit `DiffHunk`, no action bar) · `src/views/code/DiffReview.tsx` (`PrWorkspaceContainer` owns the fetch, keyed on stable `(repo_id, pr_number)`, remounted per `pr_id`) · `src/shell/Shell.test.tsx` (stub). No contract bump (frozen @0.40).

### ui-070 — github.merge_pr cat-1 (guarded-disabled) (`dce1339`, `9af3718`)
- **New:** `src/intent/merge-pr-request.ts` (`buildMergePrActionRequest` + `PR_MUTATION_ACTION_TYPES`). **Modified:** `src/gateway-client/{types,uds,mock}.ts` (the `prMutationsEnabled` flag + the `submit_action` throw-never-invoke PR guard) · `src/views/code/PrWorkspace.tsx` (Merge control `canMerge`/`onMerge` + honest rejection region + a dedicated re-review button + `prHeadSha` helper) · `src/views/code/DiffReview.tsx` (the container submit→GatewayModal wiring). cat-1; security-reviewer CLEAR both layers.

### ui-071 — github.submit_review cat-1 + per-action gate refactor (`0c1b036`, `8ceea17`)
- **Renamed:** `merge-pr-request.{ts,test.ts}` → `pr-mutation-request.{ts,test.ts}` (both builders + the shared set + `isPrMutationEnabled` are one concept). **Modified:** the gate `prMutationsEnabled: boolean` → per-action `enabledPrMutations: ReadonlySet<string>` (Uds empty / Mock full) across `types/uds/mock.ts`; `PR_MUTATION_ACTION_TYPES` += `github.submit_review`; the map-based port guard (per-action, fold merge_pr in); `buildSubmitReviewActionRequest`; `src/views/code/PrWorkspace.tsx` (3 verdict controls Approve/Request-changes/Comment + a shared body `<textarea>` + per-verdict enablement; the merge result region generalized → shared `mutationResult`/`pr-mutation-*`) · `src/views/code/DiffReview.tsx` (`canReview` + `onSubmitReview` + a shared `submitMutation` helper). cat-1; security-reviewer CLEAR both layers.

## Decisions made

- **ui-070 ruling A (lead→user):** the daemon resolves owner/repo from `repo_id` (the D7/§59 pattern) — the UI sends `inputs:{pr_number,sha,merge_method}` + a repo `resource_ref`, NEVER names owner/repo (removes a confused-deputy/owner-spoof surface). The daemon-side resolution is bundled cross-track with the head_sha exposure (already routed). Carried to submit_review (`inputs:{pr_number,commit_id,event,body}`).
- **ui-071 fork-1b (per-action gate):** `prMutationsEnabled` boolean → `enabledPrMutations: ReadonlySet<string>` so the future go-live flip can stage lowest-risk-first; both writes default disabled (empty on Uds).
- **ui-071 fork-2b (all three verdicts):** approve | request_changes | comment; body optional for approve, non-empty required for request_changes/comment (GitHub's 422 rule), trimmed at build ([[33]]).
- **SHA-pin (D2):** merge pins the displayed head via `inputs.sha`; review via `inputs.commit_id` (the reviewed head) — the daemon 409/422s on a moved head; `prHeadSha` returns null until the daemon field lands → controls disabled.
- **Generalized re-review + shared outcome region:** one `mutationResult`/`pr-mutation-*` region shared by merge + review (the ui-070 `pr-merge-*` ids renamed); a dedicated re-review button (present even for non-reapprovable fencing_conflict).
- **reviewBody persists post-submit** (intentional): a rejected/re-review retry keeps the user's text; the daemon dedups identical intents (LESSON 20) — clearing would lose input on a rejection.

## Decisions explicitly NOT made

- **The go-live flip is HELD** (`enabledPrMutations` stays empty in production) — a future USER-signed-off slice + the daemon auth-bootstrap re-review. Now per-action (stage-able).
- **The `enrichHunkAction` → `enrichActionApproval` rename** deferred (a standing cross-area cleanup flag — it's now reused by merge + review + per-hunk).
- **The `prHeadSha` shadow-reconcile** (add `head_sha` to `PullRequestRow`) deferred — pending the daemon head_sha field + ruling-A owner/repo resolution (routed cross-track).
- **No `shared/` contract change / no regen** across all 4 slices (D6/D7/D9/D10 frozen @0.39–0.42).

## TDD compliance

**Clean.** All 8 slice commits followed RED→Step-2.5→GREEN; the 2 cat-1 slices (ui-070/071) ran security-reviewer (MANDATORY) over the combined Layer-1 and over Layer-2 — CLEAR every layer. code-quality-reviewer ran every slice; all findings folded in-slice (or routed as a standing flag). No TDD violations; no safety-critical TDD skips. One load-bearing cat-1 Finding caught pre-GREEN (the ui-070 merge_pr inputs-shape contradiction vs the as-built daemon) → escalated → ruling A.

## Reachability (Step-7.5 carried)

- **D6 diff-stats:** PrWorkspace `DiffStats` renders in the Review-tab PR Workspace (DiffReview → PrWorkspaceContainer → PrWorkspace), via the live `PullRequest` projection `pr` prop. Reachable.
- **D7 get_pr_diff:** DiffReview → PrWorkspaceContainer → `gateway.get_pr_diff` → `UdsGatewayPort` → `invoke("gateway_get_pr_diff")` → allowlist → `call_daemon` → `connect_and_call`. Reachable (the L1 exposed-ahead transport now consumed).
- **merge_pr / submit_review (cat-1):** the Merge + 3 verdict controls reachable from the PR Workspace; **GUARDED-DISABLED** in production (`canMerge`/`canReview` false: `enabledPrMutations` empty + `prHeadSha` null) → controls disabled; the port throw-never-invoke is the 2nd layer. No production path reaches a live mutation (repo-wide held-flip pin).

## Open follow-ups

- **[Cross-doc — orchestrator writes hot]** `ui/CLAUDE.md` generated-Zod row (CONTRACT 0.38→0.42 + `PullRequestRow` 11→15) + the mutation/transport rows (`get_pr_diff` read method; bool→per-action `enabledPrMutations` gate; the merge_pr + submit_review cat-1 consumers). Flagged at each Step 9; orchestrator confirmed.
- **[Future TODO — next-brief / HELD]** the go-live flip (per-action, user-signed-off + daemon auth-bootstrap re-review) · `github.submit_review` is done; PR-per-hunk Accept/Reject rides D7's diff + a daemon PR-per-hunk action.
- **[Future TODO — cleanup]** `enrichHunkAction` → `enrichActionApproval` rename · the `prHeadSha` shadow-reconcile when the daemon head_sha + ruling-A owner/repo fields land · the per-file PR-diff file-tree (D7 is a flat changeset).
- **[Architecture-doc note]** the merge/review intents pin the displayed head (sha / commit_id); the daemon 409/422s on a moved head / body violation.
- **[Visual gate]** manual lead/visual sign-off — the PR code-diff + the Merge/verdict controls + the approval modal vs the prototype.

## Cross-doc invariant audit

Multi-track memory check (the orchestrator's `ui/CLAUDE.md` edits live in its checkout, invisible here): the only frozen-shadow field change this session was **`PullRequestRow` 11→15** (ui-068) — flagged at Step 9, orchestrator confirmed it writes the row. The `prMutationsEnabled`→`enabledPrMutations` change is a **UI-side GatewayPort field** (not a frozen `shared/` contract) — flagged at Step 9. **No frozen `shared/` contract changed** (no regen, no CONTRACT bump). No drift.

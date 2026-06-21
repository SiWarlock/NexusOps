# ui-023 — wire the real head_sha pin source (regen-to-green 0.44)

- **Date:** 2026-06-21
- **Phase:** Phase 7 (PR Review Workspace) — the cat-1 PR-mutations arc, UI-side go-live prereq
- **Track:** ui (`track/ui`)
- **Predecessor:** [ui-022](ui-022-2026-06-21-pr-workspace-d6-d7-merge-review-cat1.md)
- **Successor:** _(none yet — UI-side arc complete; pause pending the daemon auth-bootstrap)_

## Why this session existed

ui-070/071 built the cat-1 merge/review controls guarded-disabled with `prHeadSha` deferred to `=null` (the A3 deferral — the daemon `head_sha` field hadn't landed). The daemon merge (`a5cedd9`, CONTRACT 0.42→0.44) brought `head_sha` on `PullRequestRow` + the ruling-A owner/repo resolution AS-BUILT, so this slice retires the stub and sources the real pin — clearing the last UI-side go-live blocker, **without flipping the go-live**.

## What was built (1 slice, 1 commit `98fddab`)

### ui-072 — real head_sha pin source (NON-cat-1)
- **Modified:** `src/contracts/generated.ts` (staged the orchestrator's merge-synced regen → CONTRACT 0.44, idempotent) · `src/contracts/provisional.ts` (`PullRequestRow` shadow +`head_sha: z.string().nullable().optional()`, 15→16; doc comment) · `src/contracts/provisional.test.ts` (+`pull_request_row_head_sha_is_string_nullable`) · `src/gateway-client/mock.test.ts` (version tripwire 0.42→0.44 + comment) · `src/views/code/PrWorkspace.tsx` (`prHeadSha(pr)` → `pr.head_sha ?? null`, the `=null` A3 stub + comment retired) · `src/views/code/PrWorkspace.test.tsx` (+`pr_head_sha_returns_real_field`, incl. the absent-field leg) · `src/views/code/DiffReview.test.tsx` (+the 2 held-flip pins; the `PR_MERGEABLE` `as unknown as PullRequestRow` cast dropped → clean `: PullRequestRow`).

## Decisions made

- **head_sha = `z.string().nullable().optional()`** — mirrors the frozen `Option<String>` (a TEXT field, direct passthrough, no coercion unlike `mergeable`).
- **The held-flip guard is load-bearing + pinned** — `real_head_sha_does_not_enable_controls_while_gate_empty`: a real head_sha + empty `enabledPrMutations` → Merge AND all 3 verdict controls STILL disabled (the body typed to isolate the gate as the only blocker). head_sha sourcing is NOT a go-live; the per-action gate is the switch ([[27]]/[[28]]). The converse (`real_head_sha_with_enabled_action_enables_control`) proves head_sha was the last UI-side blocker.
- **1 commit** — a focused regen-to-green + 1-line helper wire; no cat-1 safety core to isolate (the gate + intent are unchanged).
- **Cast-drop** — `PR_MERGEABLE` no longer needs `as unknown as PullRequestRow` now that `head_sha` is a typed field.

## Decisions explicitly NOT made

- **NO go-live flip** — `enabledPrMutations` defaults UNCHANGED (empty on Uds). The remaining go-live blocker is the **daemon auth-bootstrap** (live authenticated writes) + its mandatory security re-review — daemon-side + user sign-off.
- **No `shared/` contract change** beyond consuming the daemon-landed 0.44 (no regen authored here — staged the orchestrator's).

## TDD compliance

**Clean.** RED→Step-2.5→GREEN; the 2 expected drift tests (field-set 15→16, tripwire) were the RED baseline. code-quality-reviewer ran (every-slice); 3 findings folded in-slice (absent-field `prHeadSha` leg · the converse strengthened to all 4 controls · a no-action count confirmation). security-reviewer correctly **not required** (NON-cat-1 — the gate + intent safety core are untouched; only the pin SOURCE changed null→real). No violations.

## Reachability

`prHeadSha` is already wired into `canMerge`/`canReview` (ui-070/071) — this slice changed only its RETURN (null stub → real `pr.head_sha`). The merge/review enablement now reads the real field on the production path (DiffReview → PrWorkspaceContainer). No new wiring; no tested-but-unwired gaps.

## Open follow-ups

- **[Cross-doc — orchestrator writes hot]** `ui/CLAUDE.md` generated-Zod row: CONTRACT 0.42→0.44 + `PullRequestRow` 15→16 (+head_sha) + a note that the cat-1 pin is now real-sourced (controls still gate-held). Flagged at Step 9; orchestrator confirmed.
- **[Future TODO — go-live, HELD]** all UI-side go-live blockers cleared (head_sha real + ruling-A daemon-side); the ONLY remaining blocker is the daemon auth-bootstrap + its mandatory security re-review (daemon-side + user sign-off). The flip is per-action via `enabledPrMutations` (stage-able lowest-risk-first).
- **[Future TODO — cleanup]** the standing `enrichHunkAction` → `enrichActionApproval` rename (unchanged).
- **[Architecture-doc note]** the UI head_sha is display/pin-FORMATION only; the daemon's anti-race is the LIVE GitHub 409, not this field.

## Cross-doc invariant audit

Multi-track memory check: the only frozen-shadow field change this session was **`PullRequestRow` 15→16 (+head_sha)** — flagged at Step 9, orchestrator confirmed it writes the `ui/CLAUDE.md` 0.44 row. No other model field changed; no `shared/` contract authored here (consumed the daemon-landed 0.44). No drift.

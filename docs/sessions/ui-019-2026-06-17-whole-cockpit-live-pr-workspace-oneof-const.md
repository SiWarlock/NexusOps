# ui-019 — whole-cockpit-live + read-only PR Review Workspace + gen-contracts oneOf-const (ui-063 · ui-064 · ui-065)

- **Date:** 2026-06-17
- **Phase:** Phase 6/7 — **P6.8** (whole-cockpit-live, the live `ProjectionDelta` transport spread) · **P7.2** (the read-only PR Review Workspace, §11.2/§7.2) · **P6.9** (the `gen-contracts.mjs` oneOf-const extension, §5.0)
- **Predecessor:** [ui-018](ui-018-2026-06-17-session-live-and-survival-shadow.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`

## Why this session existed

Resume the ui track post the ui-003 cycle-close: three in-lane arcs that were verified-ready in the handoff —
1. **whole-cockpit-live (the 6.8-tail).** Session went live @ui-062; the remaining live-relevant served projections (ProjectActivity/PullRequest/UsageLedger) still used reconnect-only refresh. Spread the proven refetch-on-nudge pattern to them.
2. **The read-only PR Review Workspace (P7.2).** The Kanban "Pull requests" tab is already live; the PR-detail panel (reviews/checks/mergeability/diff) was the missing surface. Build it read-only over the frozen `PullRequestRow` + the daemon-served `Review` projection, with the genuinely-absent parts (PR diff-stats, PR code-diff) as honest daemon-gap placeholders.
3. **gen-contracts oneOf-const cleanup (P6.9).** Retire a long-carried drift hazard — the 3 hand-declared `oneOf`-of-`const` shadows (ResumeMode/RecoveryState/MetricQuality) the flat-enum-only generator couldn't emit.

## What was built (4 commits across 3 arcs)

### ui-063 — whole-cockpit-live · `7dd11fa`

**Modified:**
- `ui/src/shell/Shell.tsx` — 3 new subscribe effects (ProjectActivity/PullRequest/UsageLedger), each mirroring the ApprovalQueue 2nd-stream (runSubscriptionSupervisor → subscribe → `coalescer.nudge()` → coalesced `get_projection` re-read → `setData`; per-stream `notifyConnectionState`). **Recount discipline:** ProjectActivity + PullRequest recompute the switcher `counts` (both are `deriveProjectSwitcherCounts` inputs); UsageLedger is a plain replace (`usage` + `creditPool`). **AuditTrail deliberately EXCLUDED** (the daemon blanket-nudges it on every event → a whole-page refetch storm; the daemon seq-cursor enrichment is the fix).
- `ui/src/gateway-client/mock.ts` — `subscribe()` extended with 3 `row:None` branches.
- `ui/src/projections/fixtures/{proj_project_activity,proj_pull_request,proj_usage}.ts` — 3 daemon-shaped `row:None` delta fixtures (UsageLedger id-less = `TelemetrySampled` None).
- `ui/src/shell/Shell.subscribe.test.tsx` — 3 refetch-on-nudge tests (+ helpers). **+3 tests.**

### ui-064 — read-only PR Review Workspace shell · L1 `723f90e` + L2 `a28cb06`

**New:**
- `ui/src/views/code/review-model.ts` — `describeReviewState` (a ReviewState→{glyph,label,tone} verdict-badge descriptor; `Record<ReviewState>` completeness; kept OUT of the cross-doc status→attention-rank table — a value-enum verdict, not a status machine).
- `ui/src/views/code/ReviewsList.tsx` (+ `.test.tsx`) — the reviews-list component (a ReviewRow → verdict card: reviewer + badge + body + submitted_at; empty distinct from absent).
- `ui/src/views/code/PrWorkspace.tsx` (+ `.test.tsx`) — the PR-detail panel: header (number/title/branches/status) + mergeability label + checks_summary (from `PullRequestRow`) + reviews-list + honest D6 diff-stats + D7 code-diff placeholders + DISABLED Merge/Approve-PR/Request-changes/Ask-Brain. **A PURE display component (NO gateway prop) → `get_diff`-as-PR-diff is impossible by construction (the structural D7 guarantee).**

**Modified:**
- `ui/src/shell/Shell.tsx` — load `get_projection("Review")` → `ShellData.reviews` + a Review refetch-on-nudge subscribe effect (plain-replace; completes the live-relevant served set) + pass `reviews` to DiffReview.
- `ui/src/projections/items.ts` (+ test) — `reviewsByPr` (client-side join on `pr_number`; null excluded).
- `ui/src/views/code/DiffReview.tsx` (+ test) — a required `reviews` prop; `selectedPrId` UI state; the PR card header is now a SELECTING `<button>` (keyboard-reachable, carries `data-item-id`) with the disabled Merge as a SIBLING (no nested-interactive); the Review tab renders PrWorkspace when a PR is selected, else the preserved 6.3e worktree ReviewTab; a wired "← Worktree diff" deselect.
- `ui/src/gateway-client/mock.ts` + `proj_review.ts` — Review subscribe branch + `reviewDeltaFixture`.
- `ui/src/shell/Shell.{test,subscribe.test,uds-swap.test}.tsx` — `shell_loads_review_projection`, the rendered-flow integration test, the transport-fixture-map Review entry. **+13 tests across the two layers.**

### ui-065 — gen-contracts oneOf-const cleanup · `c97d652`

**Modified:**
- `ui/scripts/gen-contracts.mjs` — a oneOf-const branch: synthesize a flat `{type:string, enum:[consts]}` from a pure-const-string `oneOf` → `json-schema-to-zod` emits an identical `z.enum`. Object-discriminated unions (ActionError/ServerFrame) skipped via `every(m && m.const is string)`.
- `ui/src/contracts/generated.ts` — REGEN (purely additive: +3 enums in alphabetical position; the 38 existing unchanged; idempotent; **CONTRACT_VERSION held 0.38.0**).
- `ui/src/contracts/generated.test.ts` — the §5.0 drift gate `enumDefs` filter widened to normalize flat + oneOf-const → `[name, values][]`; auto-pins all 41.
- `ui/src/contracts/provisional.ts` — the 3 shadow exports → local `const X = bundle.shape.X`.
- `ui/src/contracts/provisional.test.ts` — the 3 shadow drift-pin tests DELETED (coverage moved to the widened gate) + unused imports removed.
- `ui/src/contracts/index.ts` — the 3 re-exported from the generated source (`export const X = shape.X` + `export type X = z.infer<…>`; added `import { z }`). **Net −3 tests** (coverage moved, not dropped).

## Decisions made

- **AuditTrail excluded from the live-subscribe spread** (ui-063) — a paged/forensic projection the daemon blanket-nudges on every event; subscribing = a refetch storm. Stays refresh-on-open; the daemon seq-cursor delta enrichment is the right fix.
- **PrWorkspace takes no gateway** (ui-064) — makes the "never `get_diff`-as-PR-diff" (D7) and "no mutation" guarantees STRUCTURAL, not runtime-pinned. The strongest read-only guarantee.
- **Selecting-button a11y** (ui-064) — the PR card header is a `<button>` (keyboard-reachable) carrying `data-item-id`; the disabled Merge is a SIBLING (no button-in-button). Selection is UI state (LESSON §13); a stale `selectedPrId` re-resolves to none (falls back to the worktree diff).
- **The ReviewState verdict-badge descriptor lives in `review-model.ts`, NOT `status/descriptors.ts`** (ui-064) — ReviewState is a frozen VALUE enum (a verdict), not a status machine; it does not belong in the cross-doc-tracked (machine,status)→attention-rank table. `ReviewRow["state"]` is the type-derivation idiom for the value-only-exported generated enum.
- **gen-contracts synthesizes a flat enum from oneOf-const** (ui-065) — the simplest equivalence-preserving path; the §5.0 drift gate set-compares `.options` so order is moot. The `every(m && m.const is string)` guard is the correctness keystone (skips the object-discriminated unions).

## Decisions explicitly NOT made / deferred

- **The ui-064 visual gate** — DEFERRED HITL (lead-logged). Green tests ≠ looks right (LESSON §10); the ui worktree can't render the production cockpit headlessly (`main.tsx` mounts the production `UdsGatewayPort`, no Mock-injection/Storybook — LESSON §22/§23, a manual cross-track operator step). Comparison spec handed to the orchestrator: the PR Workspace panel vs `kit-views2.jsx`'s DiffReview PR-detail.
- **Per-hunk-accept-reject on the PR diff rides the D7 code-diff placeholder** — there is no separate PR-level per-hunk control to disable; the disabled PR-level set is Merge/Approve-PR/Request-changes + Ask-Brain. (Per the orchestrator's Step-9 ask.)
- **The PR-review mutations go-live** (Merge/Approve-PR/per-hunk/request-fix) — a FUTURE cat-1 arc (own checkpoint, like the L2 go-live). This read-only shell is its foundation.
- **The `/preflight` ui-mode prettier-honesty no-op** — DEFERRED HITL (a permission-blocked self-mod of `.claude/commands/`; orchestrator-handled separately).
- **The daemon asks D6/D7** — D6 (PR diff-stats producer capture) + D7 (`get_pr_diff(repo_id, pr_number)` RPC). D7 NARROWED: the reviews-LIST is buildable now (daemon serves Review); only the code-DIFF needs `get_pr_diff`. USER-routed in the work-order.
- **The ui→main merge** — a separate D-3 coordination (lead/user-gated); not this session.

## TDD compliance

**Clean — RED-first throughout.** Every slice confirmed RED for the right reason before GREEN:
- ui-063: the 3 refetch-on-nudge tests failed on the missing subscribe effect (reads/counts at baseline).
- ui-064: the mapper/component/PrWorkspace tests failed on stub returns; the Shell-load + the live-subscribe (per the orchestrator's explicit RED-first directive) + the integration tests failed pre-wiring.
- ui-065: the widened §5.0 drift gate failed (validators 38 vs 41) before the generator extension; the shadow-retirement was a deletion (coverage moved to the widened gate, verified equivalent).
- RED scaffolds (returning-empty/null stubs, optional props) were used to keep failures precise (assertion failures, not collection errors) — not behavior-before-test.

## Reachability

- **ui-063** — the 3 subscribe effects run on Shell mount (same path as the Session/ApprovalQueue effects) → daemon D4 deltas → refetch → setData. Reachable from mount.
- **ui-064** — Shell mount → code view → DiffReview → PR card select → PrWorkspace → ReviewsList(`reviewsByPr.get(pr_number)`); the integration + deselect tests pin it end-to-end. The Review load + subscribe run on mount.
- **ui-065** — the 3 enums are generated named exports consumed UNCHANGED by `recovery/model.ts`, `views/usage/model.ts`, `shell/Sidebar.tsx`, `recovery/RecoveryBanner.tsx` (same `contracts/index` import, new source).

No tested-but-unwired gaps.

## Open follow-ups (Step-9 categorized — routed hot during the session; orchestrator-owned at `/orchestrate-end`)

- **[Architecture doc note — orchestrator hot]** `ui/CLAUDE.md`: the live-`UdsGatewayPort`-transport row (refetch-on-nudge spread COMPLETE for the live-relevant served set incl. Review — whole-cockpit-live; AuditTrail excluded-by-design) + a NEW PR-Review-Workspace consumer row (PullRequestRow + ReviewRow; Review loaded+live; D6/D7 placeholders; mutations+Brain DISABLED = future cat-1) + the generated-layer row (oneOf-const extension DONE; the 3 shadows retired → 41 $defs; the caveat resolved).
- **[Convention candidate]** the refetch-on-nudge uniform mechanism + recount-iff-a-counts-input discipline; the read-only-shell-with-honest-daemon-gap-placeholders pattern + the value-enum verdict-badge descriptor; a §14 reinforcement (the generator now handles doc-commented oneOf-const → one fewer manual-shadow class).
- **[Future TODO — daemon asks]** D6 (PR diff-stats) · D7 (`get_pr_diff` RPC) — already in the work-order.
- **[Future arc]** the PR-review mutations go-live (cat-1).
- **[Carry-forward CONSUMED]** the long-carried gen-contracts oneOf-const extension (origin 2026-06-14 053) — DONE.
- **[Deferred HITL]** the ui-064 visual gate (lead manual sign-off) · the `/preflight` prettier no-op.
- **Cross-doc invariant change:** NONE this session (CONTRACT-neutral; no shadow/contract field add/remove/rename — ui-065 is a representation swap, value-sets unchanged).

## Quality gate

**389/389 green** · `tsc --noEmit` clean · `oxlint` clean · `/preflight` clean · `CONTRACT_VERSION` 0.38.0 held.

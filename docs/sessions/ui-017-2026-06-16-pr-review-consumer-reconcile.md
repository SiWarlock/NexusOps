# ui-017 — Phase-7-UI L1: the PR + Review consumable-surface reconcile (ui-061)

- **Date:** 2026-06-16
- **Phase:** Phase 7 (Phase-7-UI) — **P7.2 / §11.2** (the PR Review Workspace — its L1 data foundation: making the frozen PR + Review projection-rows typed + consumable + drift-pinned)
- **Predecessor:** [ui-016](ui-016-2026-06-16-boundary-merge-regen-0.38.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`

## Why this session existed

The 0.38 boundary merge + the ui-060 regen brought `ReviewState` + `ProjectionName.Review` into `generated.ts`, but the OBJECT rows were deliberately shadow-deferred (shadow-on-consume, `ui/LESSONS.md` §2). With Phase-7-UI now the active arc (the lead's sequence Phase-7 → survival → whole-cockpit), the PR + Review surface needs to become real + drift-pinned so the L2 PR Review Workspace can render it. Verify-before-build had already flagged the gaps: the provisional `PullRequestRow` shadow was stale (4 fields, `pr_number` typed as `string`) vs the frozen 11 (`shared/src/projections.rs`); there was no `ReviewRow` shadow (frozen at D5b-1, a separate `get_projection("Review")` page joined client-side on `pr_number`); and `toPrItems` keyed items on `pr_number` instead of the `pr_id` PK.

This slice (ui-061) reconciles both shadows to the frozen shapes (drift-pinned), adds the Review projection page + a pure client-side PR↔reviews join helper (exposed-ahead for L2), fixes `toPrItems`, and updates the fixtures. **L1 only** — the L2 workspace component is HELD on a layout/housing design call escalated to the lead→user (no prototype exists); L1's data surface is layout-independent and proceeds now.

## Why the scope expanded (the verify-before-build catch)

Step-2.5 reconnaissance surfaced a **broader RED the brief under-scoped**: the `pr_number` `string→number` change + the `toPrItems` id-source change break THREE consumers the brief didn't list — all tsc-forced (the tree can't go tsc-green without them, so they can't be a separate commit). The orchestrator ruled **scope (a): fold all in-scope, 1 atomic commit** (a tsc-RED-until-fixed consumer reconcile is not separable). The expansion stayed mechanical + NON-cat-1.

## What was built (1 atomic commit — `b894a12`)

### Files created

- `ui/src/projections/pr-reviews.ts` — the pure `reviewsByPr(reviews: ReviewRow[]): Map<number, ReviewRow[]>` client-side join helper (group by `pr_number`; null-`pr_number` reviews dropped). Exposed-ahead for the L2 workspace (no production consumer yet — the ui-059 L1 precedent).
- `ui/src/projections/pr-reviews.test.ts` — grouping / null-drop / empty + a fixtures-parse-and-join pin.
- `ui/src/projections/fixtures/proj_review.ts` — the NEW Review projection fixture (`ReviewProjectionPage`, reviews across states, joinable to the PR fixture on `pr_number`).

### Files modified

- `ui/src/contracts/provisional.ts` — `PullRequestRow` shadow reconciled **4→11** (`.strict()`; `pr_id` PK, `repo_id`, head/base branch, `pr_checked_at`, `mergeable` bool, `checks_summary`; `pr_number` `string→number` u64 shadow) + NEW `ReviewRow` (8 fields; `state`→generated `ReviewState`) + `ReviewProjectionPage` + `Review` in `ProjectionPageByName`. Drift-pinned to the frozen schema (the ApprovalQueueRow §37/§24 precedent).
- `ui/src/contracts/provisional.test.ts` — field-set drift pins (vs `$defs`) + uint/`.strict()`/state-delegate pins for both rows (+4 tests).
- `ui/src/contracts/index.ts` — re-export `ReviewState` (exposed-ahead→re-export-on-consume, the `DiffLineKind`@0.28 pattern).
- `ui/src/gateway-client/boundary.ts` + `mock.ts` — register `Review` in `PAGE_SCHEMAS` + the `Record<ProjectionName>` `FIXTURES` (the latter a tsc exhaustiveness check, now that `ProjectionName` gained `Review`).
- `ui/src/projections/items.ts` + `items.test.ts` — `toPrItems` keys `id` on `pr.pr_id` (the PK; was `pr_number`); null-safe label `title ?? (pr_number!=null ? \`PR #<n>\` : pr_id)`.
- `ui/src/projections/fixtures/proj_pull_request.ts` — reshaped to the 11-field row (numeric `pr_number`, `pr_id = {repo_id}#{pr_number}`).
- `ui/src/views/code/DiffReview.tsx` — PRsTab consumer reconcile: key on `pr_id`, `prDisplayFixture[String(pr_number ?? "")]` nullable guard, **null-safe label** (the "PR #null" regression fix — see Decisions).
- `ui/src/views/graph/model.test.ts` — PR node-id assertions updated to pr_id-based (`toPrItems` now keys on the PK — intended behavior).
- `ui/src/shell/active-project.ts` + `active-project.test.tsx` — `filterByActiveProject` generic constraint widened `{project_id?: string}` → `{project_id?: string | null}` (the 3rd tsc-forced consumer of the nullable projection-row `project_id`) + a null-`project_id` test pin.

## Decisions made

1. **Both shadows are `.strict()` frozen-shadows, fields present-and-nullable** — mirror the frozen `deny_unknown_fields` + the daemon's explicit-`null` serialization (the ApprovalQueueRow §24/§37 precedent). `pr_id`/`status` (PR) and `review_id`/`state` (Review) are the non-Option required core; the rest `.nullable().optional()`.
2. **`pr_number`/`review_id` u64 shadow = `z.number().int().nonnegative()`** — the existing Hunk-offset/`seq` precedent (Q4); no new convention invented.
3. **`toPrItems` keys on `pr_id` (the PK), `pr_number` is display-only** — a stable composite PK identity for graph nodes / locators; the GitHub-native `pr_number` (nullable) is the human label only.
4. **`reviewsByPr` drops null-`pr_number` reviews** (Q3 default) — a review with no `pr_number` can't attach to a PR row; the workspace shows attached reviews only.
5. **`ReviewState` NOT attention-ranked** (Q2) — a review is a fixed verdict (a VALUE enum), not a lifecycle needing sidebar weight; no `status/descriptors.ts` entry.
6. **The "PR #null" regression — fixed in-slice.** The nullable `pr_number` made `DiffReview`'s inline label `title ?? \`PR #${pr_number}\`` render the literal `"PR #null"` for a title-absent + null-`pr_number` row (a NEW bug the reconcile introduced; code-quality-reviewer medium). Fixed to the null-safe form (falls to `pr_id`, `toPrItems` parity). This is a correctness fix, not a baseline enhancement.
7. **Scope (a): fold the 3 tsc-forced consumers in-scope, 1 atomic commit** (orchestrator-ruled) — a tsc-RED-until-fixed consumer can't be a separate green commit.

## Decisions explicitly NOT made (deferred)

- **The L2 PR Review Workspace component** — HELD on the lead→user layout/housing design call (no prototype). `ReviewProjectionPage` + `reviewsByPr` are exposed-ahead for it. Next round in the lead's sequence is **survival UI**, not L2.
- **DiffReview PRsTab enhancement** — kept minimal (the collapsed baseline L2 replaces). The chip `#{pr_number}` still renders a bare `#` for a null `pr_number` (code-quality low, reviewer-deferred) → a baseline-only carry-forward (L2 supersedes the whole PRsTab).
- **Narrowing `ProjectionItem.machine`/`status` to the generated enum unions** — the pre-existing provisional→generated reconcile carry-forward (`items.ts` comment), unchanged here.

## TDD compliance

**Clean on the deterministic core; two reviewer-driven consumer fixes pinned reactively (noted honestly).**
- The `PullRequestRow` field-set drift pin was authored + confirmed **RED-first** (4≠11) before the GREEN shadow reconcile. The net-new `ReviewRow`/`reviewsByPr` landed with their pins (the frozen-shadow precedent — the drift-pin IS the spec for a net-new shadow; the ApprovalQueueRow/diff-shapes pattern).
- The two reviewer-flagged consumer fixes were pinned reactively: the `active-project` null-`project_id` case got an explicit test pin (added at the code-quality finding); the `DiffReview` null-safe label has no DiffReview-unit assertion (the collapsed baseline has no label test) but is covered by `toPrItems` parity (which IS pinned, 3 label branches). No TDD violation on deterministic logic; the render-fix is in the visual-baseline TDD-exempt layer.

## Reachability

- **The reconciled `PullRequestRow`** — reachable via the live `get_projection("PullRequest")` read path → the Code-view `DiffReview` PRsTab + the graph's `toPrItems` PR nodes + the command-center PR items. The reconcile flows through the existing consumers (no new wiring; the DiffReview PRsTab + graph stay green with the pr_id id-fix).
- **`ReviewProjectionPage` + `reviewsByPr`** — exposed-ahead (registered in `PAGE_SCHEMAS`/`FIXTURES`/`ProjectionPageByName`, but no production view calls `get_projection("Review")` or `reviewsByPr` yet — the L2 workspace consumer is held). Intended L1-only exposed-ahead state (the ui-059 precedent).
- No tested-but-unwired regression. No `/wired` target beyond the existing PR consumers.

## Open follow-ups

Step-9 categorized list (routed hot to the orchestrator; it writes the doc rows at `/orchestrate-end` — listed for continuity, NOT for me to re-route):

- **[Cross-doc invariant change — orchestrator writes hot]** `ui/CLAUDE.md` "Generated Zod contract layer" row: the `PullRequestRow` shadow 4→11 + the NEW `ReviewRow`/`ReviewProjectionPage`/`Review` page (consume-on-shadow §2) + the `index.ts` `ReviewState` re-export. **No new generated value-set** (`ReviewState`/`ProjectionName.Review` already recorded @ ui-060).
- **[Future TODO — L2, held]** the PR Review Workspace component (consumes `ReviewProjectionPage` + `reviewsByPr`) — HELD on the lead→user layout design call. The read-scope is bounded to projected fields (header / mergeable / checks_summary / reviews).
- **[Future TODO — baseline-only, deferred low]** the `DiffReview` PRsTab chip `#{pr_number}` renders a bare `#` for a null `pr_number` (the collapsed baseline; L2 supersedes the PRsTab).
- **[Architecture doc note]** none — the ui implements no new §; the daemon owns the 0.38 contract.

## How to use what was built

A consumer of the PR or Review surface (the L2 workspace, or any PR view) reads `get_projection("PullRequest")` / `get_projection("Review")` → the `boundary.ts` parse-don't-trust → the typed `PullRequestProjectionPage`/`ReviewProjectionPage`. To join a PR to its reviews, call `reviewsByPr(reviewPage.rows).get(pr.pr_number)`. The shadows are drift-pinned to `shared/src/projections.rs` — a daemon field change fails `provisional.test.ts` loudly. `ReviewState` is now re-exported from `contracts/index.ts` for any typed review-state consumer.

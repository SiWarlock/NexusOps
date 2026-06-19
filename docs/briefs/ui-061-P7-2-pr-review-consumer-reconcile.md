# /tdd brief — pr_review_consumer_reconcile (Phase-7-UI L1)

## Feature
Make the frozen PR + Review **consumable surface real and drift-pinned** — the foundation L2 (the PR Review Workspace component) renders. The 0.38 merge brought the frozen `PullRequestRow` (11 fields, incl. the rich D5 `mergeable`/`checks_summary`) + a NEW `ReviewRow` (8 fields, a separate `get_projection("Review")` page joined client-side on `pr_number`). The UI's provisional shadow is **stale (4 fields, `pr_number` typed as string)**, has **no `ReviewRow`**, and `toPrItems` keys items on `pr_number` instead of the PK `pr_id`. This slice reconciles the shadows to the frozen shapes (drift-pinned), adds the Review projection page + a pure client-side PR↔reviews join helper (exposed-ahead for L2), fixes `toPrItems`, and updates the fixtures. **Consumer-reconcile of daemon-frozen contract — NON-cat-1, read-only (no mutation surface).**

## Use case + traceability
- **Task ID:** P7.2 (Full PR Review Workspace, O-6 — the daemon read-contract prereq is `[x]`; this is the ui-side consumer prerequisite the L2 workspace renders)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (the PR Review Workspace surface — its data: PR header / checks / reviews / mergeability — this slice makes that data typed + consumable)
- **Related context:** the 0.38 boundary merge `2106864` + the ui-060 regen `8c373a0` (CONTRACT 0.38; `ReviewState` + `ProjectionName.Review` are now in `generated.ts`); the frozen shapes `shared/src/projections.rs:58-70` (`PullRequestRow`) + `:111-120` (`ReviewRow`); the daemon serve `daemon/src/ipc/methods.rs` (`get_projection("PullRequest")`→`Vec<PullRequestRow>`, `get_projection("Review")`→`Vec<ReviewRow>`); the established frozen-shadow drift-pin precedent = `ApprovalQueueRow` (`provisional.ts` ~`:188` + the 14-field snapshot pin `provisional.test.ts:266`); `ui/LESSONS.md §2` (provisional-shadow-on-consume — these shadows now HAVE a consumer: L2). The L2 workspace layout/housing is an **open design call escalated to the lead→user** — L1 is layout-independent (pure data surface), so it proceeds now.

## Acceptance criteria (what "done" means)
- [ ] `PullRequestRow` provisional shadow (`ui/src/contracts/provisional.ts`) reconciled **4→11 fields**, matching `shared/src/projections.rs` `PullRequestRow` / schema `$defs.PullRequestRow`: `pr_id: z.string()` (PK, non-optional), `project_id`/`repo_id`/`title`/`head_branch`/`base_branch`/`pr_checked_at`/`checks_summary` = `z.string().nullable().optional()`, `pr_number` = the u64 shadow `.nullable().optional()` (**string→number — the work-order drift**), `mergeable` = `z.boolean().nullable().optional()`, `status` delegates to the generated `PullRequest` enum. `.strict()` per the frozen `deny_unknown_fields`. Mirrors the `ApprovalQueueRow` present-and-nullable convention.
- [ ] NEW `ReviewRow` provisional shadow matching `shared/src/projections.rs:111-120` / `$defs.ReviewRow` (8 fields): `review_id` = the u64 shadow (PK, non-optional), `pr_number` = u64 `.nullable().optional()`, `project_id`/`repo_id`/`reviewer`/`submitted_at`/`body` = `z.string().nullable().optional()`, `state` delegates to the generated `ReviewState` enum. `.strict()`.
- [ ] `ui/src/contracts/index.ts` re-exports `ReviewState` (the now-consumed generated enum — the exposed-ahead→re-export-on-consume pattern, like `DiffLineKind`@0.28). `PullRequest` is already exported.
- [ ] `ReviewProjectionPage` added (`{ projection: z.literal("Review"), rows: z.array(ReviewRow), cursor: z.string().nullable().optional() }`) + `ProjectionPageByName` gains `Review: ReviewProjectionPage` (the provisional `ProjectionName` type auto-gains `"Review"`). `ui/src/gateway-client/mock.ts` `FIXTURES` gains a `Review` entry (the `Record<ProjectionName>` stays exhaustive).
- [ ] `toPrItems` (`ui/src/projections/items.ts:40`) keys `id` on **`pr.pr_id`** (was `pr.pr_number`); the human label uses `pr_number` (`pr.title ?? \`PR #${pr.pr_number}\``, null-safe). `items.test.ts:29` updated to the new id + a null-`pr_number` label case.
- [ ] A pure client-side **PR↔reviews join helper** (NEW, e.g. `ui/src/projections/pr-reviews.ts` + `.test.ts`) — `reviewsByPr(reviews: ReviewRow[]): Map<number, ReviewRow[]>` (group by `pr_number`; reviews with null `pr_number` are dropped/bucketed — see Step 2.5 Q3). Exposed-ahead for L2; pure + unit-pinned.
- [ ] Fixtures: `ui/src/projections/fixtures/proj_pull_request.ts` updated to the 11-field shape (`pr_number` as a **number**, `pr_id` = the `{repo_id}#{pr_number}` composite) + NEW `ui/src/projections/fixtures/proj_review.ts` (`ReviewProjectionPage` with a few `ReviewRow`s across states, joinable to the PR fixture on `pr_number`). Both parse clean against the new shadows.
- [ ] Full ui suite green (the ~372 + the new tests), `tsc --noEmit` + `oxlint` clean, `/preflight` clean.
- [ ] Cross-doc flagged at Step 9 (orchestrator writes the `ui/CLAUDE.md` row hot: the `PullRequestRow` shadow 4→11 + the NEW `ReviewRow`/`Review` projection page; **no new generated value-set** — `ReviewState`/`ProjectionName.Review` already landed @ ui-060). **Implementer does NOT edit `ui/CLAUDE.md`.**

## Wiring / entry point (Step 7.5)
The reconciled shadows are consumed on the existing read path: `get_projection("PullRequest")` / `get_projection("Review")` → the `boundary.ts` parse-don't-trust → `ProjectionPageByName`. `toPrItems` is already wired (the PR list / command-center PR items). The **new `ReviewProjectionPage` + the `reviewsByPr` join helper are exposed-ahead** — their production consumer is the L2 PR Review Workspace component (held on the layout design call); no production caller at L1 (the ui-059 L1 precedent — expose the pure core ahead of the L2 wiring). Reviewers confirm: no tested-but-unwired *regression* (the existing PR-list path stays green with the `toPrItems` id fix). No `/wired` target beyond the existing PR-list consumer.

## Files expected to touch
**Modified:** `ui/src/contracts/provisional.ts` (PullRequestRow 4→11 + ReviewRow + ReviewProjectionPage + ProjectionPageByName) · `ui/src/contracts/provisional.test.ts` (the drift-pins) · `ui/src/contracts/index.ts` (re-export ReviewState) · `ui/src/gateway-client/mock.ts` (FIXTURES += Review) · `ui/src/projections/items.ts` + `items.test.ts` (toPrItems id fix) · `ui/src/projections/fixtures/proj_pull_request.ts` (11-field).
**New:** `ui/src/projections/pr-reviews.ts` + `.test.ts` (the join helper) · `ui/src/projections/fixtures/proj_review.ts`.
**Not touched:** `generated.ts` (ReviewState/Review already there @ui-060 — do NOT regen) · `ui/CLAUDE.md` (orchestrator-territory) · `status/descriptors.ts` (all 11 PR states already covered; ReviewState is a VALUE enum, NOT attention-ranked — see Step 2.5 Q2).

## RED test outline (Step 2)
1. **`pullrequestrow_fields_match_frozen_schema`** (NEW, `provisional.test.ts`, mirrors the ApprovalQueueRow 14-field pin) — the shadow's field-set === `schema.$defs.PullRequestRow` properties (both directions); an extra field FAILS `.strict()`. **RED:** current shadow = 4 fields ≠ frozen 11. Asserts: §11.2/Appendix-A PR row contract mirrored. 
2. **`reviewrow_fields_match_frozen_schema`** (NEW) — the new shadow's field-set === `$defs.ReviewRow` (8); extra field FAILS. **RED:** no shadow exists. Asserts: the D5b-1 ReviewRow contract mirrored.
3. **`pr_number is a number, not a string`** (NEW or folded into #1) — a frozen-shaped row with numeric `pr_number` parses; a string `pr_number` FAILS. **RED:** current `z.string()` accepts string, rejects number (inverted). Asserts: the work-order str→number drift fixed.
4. **`toPrItems keys on pr_id`** (UPDATE `items.test.ts:29`) — `toPrItems(rows)[i].id === rows[i].pr_id`; label uses `pr_number` (incl. a null-`pr_number` case → `PR #` fallback or title). **RED:** current returns `id = pr_number`.
5. **`reviewsByPr` groups by pr_number** (NEW, `pr-reviews.test.ts`) — N reviews across M pr_numbers → grouped map; a null-`pr_number` review handled per Q3; empty → empty. **RED:** helper missing.
6. **Fixtures parse** — `pullRequestFixture` (11-field) + `reviewFixture` parse clean against the shadows; the review fixture joins to the PR fixture on `pr_number`. **RED:** current PR fixture (4-field, string pr_number) fails the new shadow.

> Confirm RED is exactly the shadow/mapper/helper drift above — a broader RED (e.g. an existing consumer of the old 4-field shadow that breaks) is a Step-2.5 flag.

## Cross-doc invariant impact
- **Model field changes:** none in `shared/` (consuming the daemon-frozen 0.38 shapes). The ui adds two provisional frozen-shadows (PullRequestRow extended, ReviewRow new) — drift-pinned to the schema, NOT ui-authored contract.
- **Orchestrator doc row (Step 9, I write hot):** `ui/CLAUDE.md` — note the `PullRequestRow` shadow 4→11 + the new `ReviewRow`/`ReviewProjectionPage` (consume-on-shadow per `ui/LESSONS.md` §2); `ReviewState`/`ProjectionName.Review` value-sets already recorded @ ui-060.
- **2.5-seam:** the shadows mirror a `shared/` 2.5-seam contract (the projection rows) — pinned by the field-set drift tests (the schema-snapshot equivalent). No ui-authored invariant on the seam.

## Things to flag at Step 2.5
1. **The daemon serve shape (verify against the real serve).** The frozen serve returns `Vec<PullRequestRow>` / `Vec<ReviewRow>`; the UI wraps it in a `…ProjectionPage` ({projection, rows, cursor}) — **mirror the existing `PullRequestProjectionPage` handling for `ReviewProjectionPage`** (the established boundary pattern). Confirm the existing PullRequest page parse against the real serve before adding Review the same way; flag if the serve isn't page-wrapped as the existing code assumes.
2. **`ReviewState` is a VALUE enum, NOT attention-ranked** — do NOT add it to `status/descriptors.ts` (no `(machine,status)→attentionRank`; a review is a fixed verdict, not a lifecycle needing sidebar weight). My default: confirm no descriptors entry. (PullRequest's 11 states are already covered — no change.)
3. **Null-`pr_number` reviews in `reviewsByPr`** — drop them (can't join) vs an "unattached" bucket. My default: **drop with a count** (a null-`pr_number` review can't attach to a PR row; L2 shows attached reviews only). Flag if you see a reason to keep them.
4. **u64 shadow convention** — match how existing u64 fields are shadowed (`z.number().int().nonnegative()` vs a bounded form); mirror the existing numeric-field precedent (e.g. the Hunk-offset uint pins / ApprovalQueueRow numerics) — don't invent a new one.

## Dependencies + sequencing
- **Depends on:** the 0.38 merge + ui-060 (landed — `ReviewState`/`ProjectionName.Review` in `generated.ts`).
- **Blocks:** **ui-062 (Phase-7-UI L2 — the PR Review Workspace component)**, which renders this surface. L2 is **HELD on the layout/housing design call** (escalated to the lead→user) + the scope-boundary confirm (read workspace = {header, mergeable, checks_summary, reviews}); L1 is layout-independent and proceeds now.

## Estimated commit count
**1** (atomic — the shadows + page + helper + mapper + fixtures are one cohesive consumable-surface reconcile; all deterministic, RED-first). NON-cat-1, no safety pin → no split. If the join-helper + shadows feel separable at Step 2.5, a 2-commit split (L1a shadows/fixtures, L1b mapper/join) is acceptable — your call.

## Lessons-logged candidates anticipated
- Likely a one-line reinforcement of `ui/LESSONS.md §2` (shadow-on-consume): the object rows deferred at ui-060 are shadowed HERE, when their L2 consumer is in sight — and the drift-pin (field-set vs `$defs`) is what makes the deferred-then-consumed shadow safe.
- **Future TODO (carry-forward):** L2 PR Review Workspace (held on the design call); the deferred non-projected surfaces (per-check detail, Brain evidence, agent-session, merge controls = future cat-1).

## How to invoke
1. Read this brief end-to-end — especially the Step-2.5 serve-shape + null-pr_number calls.
2. Confirm RED: `pnpm test src/contracts/provisional.test.ts src/projections/items.test.ts` shows the shadow/mapper drift RED.
3. `/tdd pr_review_consumer_reconcile`.
4. Step 2.5 → ping with the serve-shape verification + the 3 default calls.
5. GREEN → full suite + `/preflight`.
6. Step 9 → the cross-doc note + the L2 carry-forward.

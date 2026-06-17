# /tdd brief — pr_review_workspace_shell (Phase-7-UI L2, READ-ONLY)

## Feature
Build the **read-only PR Review Workspace** into the cockpit's "Review" surface (the prototype's PR-detail panel — A-screens.json item 10: "PR Review Workspace COLLAPSED into Code 'Review' tab, **missing checks/reviews/comments/mergeability panels + merge controls**"). For a PR selected from the **already-live** Kanban "Pull requests" tab, render the missing panels: the **reviews-list** (backed by `ReviewRow`), the **PR header + mergeability/checks** (from `PullRequestRow`), with the **diff-stats** (D6) and the **PR code-diff** (D7) as documented daemon-gap placeholders. **ALL mutations** (Merge / Approve-PR / per-hunk accept-reject) render **DISABLED** (a future cat-1 arc); **Ask Brain / Ask why** render **DISABLED** (the deferred Brain sibling). **NON-cat-1, read-only.**

## Verify-before-build findings (the scope is narrower + more buildable than the handoff implied — read this first)
The pre-orient surface map (`Shell.tsx`, `DiffReview.tsx`, `provisional.ts`, `items.ts`) established:
1. **The Kanban "Pull requests" tab is ALREADY built + wired to REAL data** — `Shell.tsx:684` renders `<DiffReview prs={filterByActiveProject(data.pullRequests, activeProjectId)} />`; `PRsTab` (`DiffReview.tsx:404-511`) renders the real `PullRequestRow[]` in the 3-lane Kanban (`laneOf(status)` → open/ready/merged), D6 diff-stats via a `prDisplayFixture` side-map, Merge DISABLED. **So this slice does NOT rebuild the Kanban** — it builds the Review-tab vertical.
2. **The Review projection is BUILDABLE NOW — NOT a daemon gap.** The daemon serves `ProjectionName::Review` (`read_review_typed`, landed D5b @ui-060); `boundary.ts` PAGE_SCHEMAS already has `ReviewProjectionPage`. The Shell just doesn't LOAD it yet (the initial `get_projection` set is the 6 at `Shell.tsx:183-191`; Review is absent). So `client.get_projection("Review")` returns a parseable page today — **the reviews-list is buildable**; only the PR **code-diff** is the genuine D7 gap.
3. **`mergeable`/`checks_summary` ARE in the frozen `PullRequestRow`** (D5a) — only the **diff-stats** (additions/deletions/changed_files/commits) are the D6 gap. So mergeability + checks display is buildable now from the row.
4. **No `ReviewRow` consumer / no `reviewsByPr` mapper exists** — net-new (the ui-061 "reviewsByPr exposed-ahead" was the shadow + drift-pin only; no mapper landed). `toPrItems` exists (`items.ts:41-50`).
5. The existing `DiffReview` "Review" tab (`ReviewTab`, `DiffReview.tsx:118-274`) is the **6.3e worktree per-hunk diff** (`get_diff(worktree_id, file)`, hardcoded single file) — conceptually distinct from a PR's diff. Its coexistence with the PR workspace is a Step-2.5 design Q (below).

## Use case + traceability
- **Task ID:** P7.2 (Full PR Review Workspace — O-6; the ui-side workspace, the daemon read-contract prerequisite ✅ landed `e748874`/D5a/D5b)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (the PR Review Workspace surface), `§7.2` (PR is GitHub-authoritative / SoT)
- **Related context:** the prototype `NexusOps-ui-kit/ui_kits/control-plane/kit-views2.jsx` (the `DiffReview`/`ReviewTab`/`PRsTab` reference); `docs/ui-review/A-screens.json` item 10 (the missing-panels gap); the frozen `PullRequestRow` (11 fields, `provisional.ts:187-216`) + `ReviewRow` (8 fields, `provisional.ts:218-244`); the `ReviewState` value-enum (5: approved/changes_requested/commented/dismissed/pending — `generated.ts`); the ui-063 refetch-on-nudge pattern (`ui/LESSONS.md` §29) if the optional Review live-subscribe is included; the read-only/disabled-control pattern (`useCanSubmitIntent`, `ui/LESSONS.md` §4/§11; wire-or-disable §11.6 §13).

## Build in 2 layers (`ui/LESSONS.md` §7 — multi-commit ui slice; I drive layer→layer)
**Layer 1 — the reviews data vertical (commit 1).** The `ReviewRow` consumer that doesn't exist yet:
- Load `client.get_projection("Review")` in the Shell (add to the `Shell.tsx:183` `Promise.all` + `ShellData.reviews: ReviewRow[]`); parses via the existing `boundary.ts` `ReviewProjectionPage`.
- A pure `reviewsByPr(reviews: ReviewRow[]): Map<number, ReviewRow[]>` (or `groupReviewsByPr`) mapper in `items.ts` — group by `pr_number` (the client-side join key; the ui-061 `ReviewRow` model), excluding null `pr_number`.
- A net-new **reviews-list component** (e.g. `views/code/ReviewsList.tsx`) rendering each `ReviewRow` as a card: `reviewer`, a **`ReviewState` badge** (the 5-value verdict — glyph+label, **never color alone** forbidden #5; reuse a descriptor), `body` (the daemon-redacted review text), `submitted_at`. Empty state distinct from absent.

**Layer 2 — the PR Workspace assembly (commit 2).** Wire the panels into the Review surface for the selected PR:
- The PR header: `pr_number`/`title`/`head_branch`→`base_branch`/`status` (StatusPill) from the selected `PullRequestRow`.
- **Mergeability + checks** from `PullRequestRow.mergeable`/`checks_summary` (present in the frozen row — buildable; never-color-alone).
- The **reviews-list** (Layer 1) for the selected PR (`reviewsByPr.get(pr_number)`).
- The **D6 diff-stats placeholder** (+/−/files/commits) — an honest "unavailable — needs the daemon diff-stats capture" affordance, NEVER a fabricated number (honest-degrade; D6 work-order).
- The **D7 code-diff placeholder** — an honest "PR diff unavailable — needs `get_pr_diff(repo_id, pr_number)`" panel (the worktree-scoped `get_diff` is NOT a PR diff; D7 confirmed gap, work-order). NEVER reuse `get_diff` to fake it.
- **DISABLED controls** (wire-or-disable — never a dead click): **Merge / Approve-PR / per-hunk accept-reject** render disabled (future cat-1 mutation arc) + **Ask Brain / Ask why** render disabled (deferred Brain sibling). Use the established disabled affordance (a `disabled` button with an accessible "coming soon / requires sign-off" name), NOT a hidden control.

## Acceptance criteria (what "done" means)
- [ ] **Layer 1 — Review load:** `ShellData.reviews: ReviewRow[]` populated from `get_projection("Review")` (parsed at the boundary); the Shell renders without error when Review is empty.
- [ ] **Layer 1 — `reviewsByPr` mapper:** groups `ReviewRow[]` by `pr_number` (null `pr_number` excluded); pure + unit-pinned.
- [ ] **Layer 1 — reviews-list component:** renders a `ReviewRow` as `{reviewer, ReviewState badge (glyph+label, never color alone), body, submitted_at}`; the 5-value `ReviewState` all render (no unknown→blank); empty-list state distinct from absent.
- [ ] **Layer 2 — PR header + mergeability/checks** render from the selected `PullRequestRow` (status via StatusPill; mergeable/checks_summary shown; never color alone).
- [ ] **Layer 2 — reviews-list wired** to the selected PR (`reviewsByPr.get(pr_number)`).
- [ ] **Layer 2 — D6 + D7 placeholders** render as honest "unavailable — needs daemon <X>" affordances (honest-degrade, never stale-as-live); NO fabricated diff-stats, NO `get_diff`-as-PR-diff.
- [ ] **Layer 2 — mutations + Brain controls DISABLED** (Merge/Approve-PR/per-hunk/Ask-Brain): rendered `disabled` with an accessible name (wire-or-disable); a test pins each is non-interactive.
- [ ] The full ui suite stays green (the new tests), `tsc --noEmit` + `oxlint` clean, `/preflight` clean.
- [ ] **Visual gate** (the standing UI gate, `ui/LESSONS.md` §10/§12) — the Review Workspace matches the prototype's PR-detail panel (dev server vs `kit-views2.jsx`); flag for the lead/visual sign-off.
- [ ] Cross-doc flagged at Step 9 (orchestrator writes the `ui/CLAUDE.md` rows hot: a NEW PR-Review-Workspace consumer row [PullRequestRow + ReviewRow consumed; D6/D7 placeholders; mutations disabled = future cat-1]; the Review projection now loaded). **Implementer does NOT edit `ui/CLAUDE.md`.**

## Deferred / out of scope (flagged, not built)
- **The PR code-diff** (D7) — `get_pr_diff(repo_id, pr_number) → DiffResult` does not exist (`get_diff` is worktree-scoped; a PR is a remote entity). The code-diff panel is a placeholder; the RPC is a daemon ask (work-order).
- **The PR diff-stats** (D6) — additions/deletions/changed_files/commits absent from `PullRequestRow`; placeholder; daemon ask (work-order).
- **The PR-review MUTATIONS** (Merge / Approve-PR / per-hunk accept-reject / request-fix) — a **future cat-1 arc** (own checkpoint, like the L2 go-live). Rendered DISABLED here.
- **Brain controls** (Ask Brain / Ask why) — the deferred `brain/` sibling. Rendered DISABLED.
- **The 6.3e worktree per-hunk diff** (the existing `ReviewTab` `get_diff` path) is NOT removed — see Step-2.5 #1 for coexistence.

## Wiring / entry point (Step 7.5)
The PR Review Workspace renders inside the cockpit's existing "code" content-view (`Shell.tsx:683`, `DiffReview`) — the prototype-faithful home (A-screens item 10 collapses the PR workspace into the Code "Review" tab). The selected PR drives it (the Kanban PR card click → the Review surface for that PR — the prototype's interaction, `PRsTab` card `data-item-id="PullRequest:<pr_id>"`). `ShellData.reviews` is loaded at the Shell mount (the new 7th `get_projection`). `/wired` target: the PR card click → the Review Workspace render path (reachable from the Shell's "code" view); the reviews-list reachable from `data.reviews` → `reviewsByPr` → the selected PR.

## Files expected to touch
**New:**
- `ui/src/views/code/ReviewsList.tsx` (+ `.test.tsx`) — the `ReviewRow` reviews-list component (Step-2.5 #2 on placement).
- (Layer 2) possibly `ui/src/views/code/PrWorkspace.tsx` or an extension within `DiffReview.tsx` — the PR-detail assembly (Step-2.5 #1/#2).

**Modified:**
- `ui/src/shell/Shell.tsx` — add `get_projection("Review")` to the initial load + `ShellData.reviews`; pass `reviews` (+ the selected PR) into the Review surface. (Optional Step-2.5 #4: a Review refetch-on-nudge subscribe effect, the ui-063 pattern.)
- `ui/src/projections/items.ts` (+ `items.test.ts`) — the `reviewsByPr` mapper.
- `ui/src/views/code/DiffReview.tsx` (+ `.test.tsx`) — wire the PR Workspace panels (PR header + mergeability/checks + reviews-list + D6/D7 placeholders + DISABLED controls) for the selected PR.
- A status-descriptor for `ReviewState` if a badge descriptor is needed (`status/descriptors.ts` is orchestrator-territory? NO — it's UI render policy, implementer-owned; but confirm it's not the cross-doc table). _(Flag at Step 2.5 if a `ReviewState`→descriptor mapping is added.)_

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
**Layer 1:**
1. **`reviews_by_pr_groups_by_pr_number`** (items.test.ts) — `reviewsByPr` groups by `pr_number`; null `pr_number` excluded; multiple reviews per PR retained. **RED:** mapper absent. [§7.2/§11.2]
2. **`reviews_list_renders_review_row`** (ReviewsList.test.tsx) — a `ReviewRow` renders reviewer + `ReviewState` badge (glyph+label) + body + submitted_at; all 5 `ReviewState` values render (no unknown→blank); empty-list distinct from absent. **RED:** component absent. [§11.2/forbidden #5]
3. **`shell_loads_review_projection`** (Shell test) — the Shell's initial load includes `get_projection("Review")` → `ShellData.reviews` populated; empty Review → no error. **RED:** Review not loaded. [§7.2]

**Layer 2:**
4. **`pr_workspace_renders_header_and_mergeability`** — the selected `PullRequestRow` → PR header (number/title/branch/status pill) + mergeable/checks_summary rendered (never color alone). **RED:** panels absent. [§11.2/§7.2]
5. **`pr_workspace_shows_reviews_for_selected_pr`** — the workspace shows `reviewsByPr.get(pr_number)` for the selected PR. **RED:** not wired. [§11.2]
6. **`pr_workspace_diff_and_stats_are_honest_placeholders`** — the D6 diff-stats + D7 code-diff render an "unavailable — needs daemon <X>" affordance, NOT a number / NOT a `get_diff` call. **RED:** placeholder absent (or fabricated). [honest-degrade/§7.2]
7. **`pr_workspace_mutations_and_brain_disabled`** — Merge/Approve-PR/per-hunk/Ask-Brain render `disabled` with accessible names; non-interactive (wire-or-disable). **RED:** controls absent or enabled. [a11y wire-or-disable / forbidden #6]

> The Step-2.5 coverage map ties each acceptance bullet to a test or a not-tested-because (the visual gate + /preflight are Step 7/8; the cross-doc row is Step 9).

## Cross-doc invariant impact
- **Model field changes:** none. Consumes the daemon-frozen `PullRequestRow` (0.34/D5a) + `ReviewRow` (0.37/D5b) shadows already in the tree; no shadow/contract change, no regen.
- **Orchestrator doc rows (Step 9, I write hot):** a `ui/CLAUDE.md` row for the NEW **PR Review Workspace consumer** (`views/code/` — consumes `PullRequestRow` + `ReviewRow`; the Review projection now loaded; D6 diff-stats + D7 code-diff placeholders; Merge/Approve-PR/per-hunk + Brain controls DISABLED = a future cat-1 arc). No new generated value-set.
- **2.5-seam:** none touched (no shadow/contract change).

## Things to flag at Step 2.5
1. **Coexistence with the 6.3e worktree per-hunk diff** (the load-bearing structural Q). The existing `ReviewTab` (`get_diff`, worktree-scoped) is conceptually distinct from a PR's diff. My default: **the PR Review Workspace renders when a PR is selected** (from the Kanban); the 6.3e worktree per-hunk diff is **preserved** (e.g. shown when no PR is selected, or under the Worktrees tab) — do NOT delete it. Flag if you'd rather a cleaner split (a dedicated PR-detail surface vs in the "Review" tab) — if this turns load-bearing, I route it to the lead (away-mode adjudicates #4).
2. **Component placement + assembly shape.** My default: `views/code/ReviewsList.tsx` + assemble within `DiffReview.tsx` (co-located, prototype-faithful) rather than a new `views/pr-review/` dir. Flag if a separate dir reads cleaner.
3. **PR selection mechanism.** My default: the Kanban PR card click selects the PR → the Review Workspace renders for it (the prototype interaction); a UI selection state (`selectedPrId`, the local-UI-state lessons family [`ui/LESSONS.md` §13], mirror `selectedSessionId`). Flag alternatives.
4. **Include the Review refetch-on-nudge live-subscribe?** My default: **yes, fold into Layer 1** — it's the mechanical ui-063 pattern, Review nudges ARE emitted (`deltas_for_event` → ReviewSynced), and it completes the live-relevant served set (Review was the one deferred from ui-063). Frame it as serving the §11.2 live display (not a new transport-layer citation). Flag if you'd rather load-once now + live-subscribe as a tiny follow-on.
5. **D6/D7 placeholder treatment.** My default: an honest, labeled "unavailable — needs `get_pr_diff` / diff-stats capture" affordance (honest-degrade) — NEVER a fabricated stat, NEVER `get_diff`-as-PR-diff. Confirm.

## Dependencies + sequencing
- **Depends on:** ui-061 (PullRequestRow + ReviewRow shadows frozen + drift-pinned) + the daemon read-contract (D5a `mergeable`/`checks_summary` + D5b `ReviewRow`/`proj_review`/`ProjectionName::Review`, all on track/ui). The Kanban tab (already live) is the entry point.
- **Blocks:** the PR-review MUTATIONS go-live (a future cat-1 arc — this shell is its read-only foundation). The D6/D7 daemon asks (work-order) unblock the diff-stats + code-diff panels.

## Estimated commit count
**2** (`ui/LESSONS.md` §7 multi-commit): **Layer 1** the reviews data vertical (Review load + `reviewsByPr` + the reviews-list component) → **Layer 2** the PR Workspace assembly (header/mergeability/reviews-list/placeholders/disabled controls). I drive layer→layer (wake at each layer commit; the impl idles between). NON-cat-1, read-only — no safety pin. If Layer 1 grows (the optional Review subscribe), keep it cohesive; if Layer 2's assembly is large, a 3-commit split is acceptable (your call).

## Lessons-logged candidates anticipated
- **Convention candidate** — the read-only-shell-with-honest-daemon-gap-placeholders pattern (render the backed parts from the frozen rows; the unbuildable parts [D6/D7] as honest "unavailable — needs daemon <X>" affordances, never fabricated, never a wrong-source fake like `get_diff`-as-PR-diff); the future-cat-1 mutations rendered DISABLED (wire-or-disable). The client-side `pr_number` join (ReviewRow→PullRequestRow).
- **Future TODO — daemon asks (already in the work-order):** D6 (PR diff-stats producer capture), D7 (`get_pr_diff` RPC). Re-flag the **narrowed** D7 scope: the reviews-LIST is buildable now (daemon serves Review); only the code-DIFF needs `get_pr_diff`.
- **Future arc:** the PR-review mutations go-live (cat-1).

## How to invoke
1. Read this brief end-to-end — especially the **verify-before-build findings** (the Kanban is already done; Review is buildable now; D7 is just the code-diff) + the **2-layer** structure + the **5 Step-2.5 calls**.
2. Confirm RED (Layer 1 first): `pnpm test src/projections/items.test.ts src/views/code/ReviewsList.test.tsx`.
3. `/tdd pr_review_workspace_shell`.
4. Step 2.5 → the 5 design calls + the coverage map. (Layer 1 RED first; I drive you to Layer 2 at the Layer-1 commit.)
5. GREEN per layer → full suite + the visual gate + `/preflight`.
6. Step 9 → the `ui/CLAUDE.md` PR-Workspace-consumer row (I write hot) + the D6/D7 work-order re-flags.

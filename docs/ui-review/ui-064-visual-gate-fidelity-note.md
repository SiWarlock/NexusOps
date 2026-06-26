# ui-064 Visual Gate — Design-Fidelity Cross-Check (structural)

- **Date:** 2026-06-17
- **Track:** `track/ui` · **Gate:** the long-deferred ui-064 visual gate (HITL) — user-chosen 2026-06-17
- **Kind:** read-only structural cross-check (NO build, NO daemon, NO code change). **NOT a `/tdd` slice.**
- **Scope:** shipped `ui/src/views/code/{PrWorkspace,DiffReview,ReviewsList}.tsx` vs the prototype PR-detail in `NexusOps-ui-kit/ui_kits/control-plane/kit-views2.jsx` (`PRsTab` / `ReviewTab` / PR-detail) + `kit-data.js` `prs` fixture.
- **Purpose:** pre-screen the user's **live pixel pass** (the live render needs the daemon; the user eyeballs that in their env). This note is the design-fidelity half + the durable audit artifact.
- **Verdict:** **CLEAN — no material Finding** (no fabricated/dishonest state, no enabled-mutation leak). Divergences are all intentional ui-064 decisions; the two load-bearing ones trace to the daemon **D6/D7** gaps.

---

## Load-bearing framing

The prototype has ONE diff-centric **"Review · PR #84"** detail (a 2-column changed-files list + real `GDiff` hunks, with enabled Approve / Ask-Brain). The build intentionally **SPLITS** that into:

- **(a) the default Review tab** = the **LIVE worktree per-hunk diff** (6.3e, `get_diff`-sourced; per-hunk git intents are live post-L2-C) — the worktree surface.
- **(b) the read-only `PrWorkspace`** (selection-driven from the Kanban "Pull requests" tab) = the **PR-detail shell**.

So for dims 2–5 the comparison axis is **prototype-PR-detail ↔ shipped-`PrWorkspace`**. (Per-hunk git mutations live in the SEPARATE default Review tab — the worktree surface, out of this gate's PR-detail scope.)

---

## Element-by-element

### Dim 1 — Pull-requests Kanban + Review-tab layout
- **Tab strip — MATCH.** git-pull-request icon + `Review / Worktrees / Pull requests`, active inset-underline, count badges. Shipped adds `aria-pressed`.
- **Kanban lanes — MATCH.** Same 3 lanes (open / ready / merged) + tones; dot + uppercase-label + count header; "None" dashed empty-state — identical.
- **Kanban card — MATCH on skeleton** (chip / title / stats / branch rows). Divergences:
  - (i) **checks indicator** — prototype `GPill(checks pass/fail)` → shipped `StatusPill(machine=PullRequest, real status enum)`; the checks proxy moved into the PR-detail's `checks_summary`. *(semantic shift)*
  - (ii) **per-card stats/age/branch are `prDisplayFixture`-gated** (keyed by `pr_number`) — shown only when a fixture entry exists, vs the prototype always showing them → a real, fixture-less PR renders a **sparser** card. *(D6-adjacent enrichment gap)*
  - (iii) **Merge disabled** (vs prototype enabled). *(see dim 5)*
  - (iv) **null-safe `#` chip** (ui-067) omitted when `pr_number` is null.
- **Review-tab default — DIVERGENCE (intentional):** prototype default = the PR-detail; shipped default = the live worktree diff.

### Dim 2 — PR header / mergeability / checks (`PrWorkspace`) — PARTIAL (additions)
Both have a `#`pr-chip + status pill. Shipped **adds** (real `PullRequestRow` data, not prototype elements): title `h1` + wired "← Worktree diff" back + branch line (head→base) + **mergeability glyph+label** (✓/✗/? · Mergeable / Conflicts / Mergeability unknown — never color-alone; `null` = honest unknown) + **`checks_summary`** text. Prototype header was "Review · PR #84" + a failing `GPill` + `#84` + (Ask-Brain / Approve-PR). Mergeability + checks_summary are **ADDITIONS**.

### Dim 3 — reviews-list verdict badges — ADDITION (no prototype counterpart)
The prototype PR-detail had **no** reviews list. Shipped "Reviews" section → `ReviewsList` → per-`ReviewRow` card: reviewer + Badge(glyph+label, additive tone) + redacted body + submitted_at; empty-state "No reviews yet." All 5 `ReviewState` verdicts covered (approved ✓ / changes_requested ✗ / commented 🗩 / dismissed ⊘ / pending ◷), glyph+label **never color alone**. The D5b review vertical — a net add, not a divergence-from a prototype element.

### Dim 4 — D6 diff-stats + D7 code-diff placeholders — PASS (both honest)
- **D6** `pr-diffstats-unavailable` — dashed box: "unavailable — needs the daemon's PR diff-stats capture (D6). No numbers shown rather than fabricated ones."
- **D7** `pr-diff-unavailable` — FileDiff glyph: names the missing `get_pr_diff(repo_id, pr_number)` RPC + states worktree-scoped `get_diff` ≠ a PR diff.
- **`PrWorkspace` takes NO gateway prop → cannot reach `get_diff` by construction.** Both honest; no fabricated stats (the prototype showed `+476/−110` + real `GDiff`; the build refuses to fake).

### Dim 5 — mutations + Brain DISABLED — PASS
`PrWorkspace`: **Merge / Approve PR / Request changes / Ask Brain** all `disabled`, each title-tooltip'd to the future cat-1 arc / Phase-8 Brain. Structural: **gateway-free → no mutation reachable.** Kanban **Merge disabled** ✓. **Per-hunk in the PR context = ABSENT by construction** (the PR code-diff isn't rendered → no Stage/Unstage/Discard/RequestFix). *(Per-hunk worktree git intents DO exist live in the separate default Review tab — the worktree surface, out of scope.)*

#### Dim 1/5 supplement — Merge-as-sibling (deliberate a11y divergence)
- **Prototype (`PRsTab`):** the **whole card is the click target** — `<div onClick={openReview}>` — with the Merge button **nested INSIDE** it using `e.stopPropagation()` (nested-interactive + stopPropagation pattern).
- **Shipped (`DiffReview.tsx` PRsTab):** the card `<div>` is a plain container; the selecting affordance is an **explicit `<button>`** (carries `data-item-id`, keyboard-reachable, `onClick={onSelect}`) and the (disabled) Merge button is a **SIBLING below it** — NOT nested (comment at `:456`; the §11.6 no-nested-interactive / WAI-ARIA decision; no `stopPropagation` hack).
- **A deliberate divergence-from-prototype = a correctness/a11y improvement, not a pixel-match.**

---

## Honest caveats for the user's live pixel pass (flag, not fault)

1. **Biggest departure:** `PrWorkspace` is a **single-column read-only SUMMARY**, not the prototype's 2-col changed-files + live-diff layout — because the PR code-diff is the unfilled **daemon D7** gap. Intentional + honest, but a real visual divergence (a reviewer expecting the diff-centric detail sees placeholders).
2. **Kanban card sparseness on real (fixture-less) PRs** — stats/branch/age are `prDisplayFixture`-sourced → traces to the **daemon D6** diff-stats gap.
3. Card **checks-pill → PR-status-pill** semantic shift (checks moved into the detail's `checks_summary`).
4. Review-tab **default content** differs (live worktree diff vs the prototype's PR-detail).
5. **Additions beyond the prototype** (mergeability line, checks_summary, the Reviews verdict-list) — richer real-data, not regressions, but mean `PrWorkspace` ≠ a pixel-match.

---

## Verdict + recommendation

- **Kanban + tab strip:** HIGH structural fidelity (minus the checks-pill semantic shift + the fixture-gating sparseness).
- **PR-detail (`PrWorkspace`):** INTENTIONALLY re-shaped read-only shell — **NOT a prototype pixel-match.** D6/D7 placeholders honest + present; all mutations/Brain disabled (structurally gateway-free); reviews verdict-list added (D5b).
- **No dishonest/fabricated state, no enabled-mutation leak found.**
- **Key insight (lead-surfaced to the user):** caveats **1 + 2** both trace to the **daemon D6/D7 gaps** — they decide whether the full PR-detail (diff-centric) visual experience is worth unblocking. The user's live pixel pass should focus on caveat **1 (the single-column read-only layout)** + caveat **2 (fixture-less card sparseness)**.
- **The user does the live pixel pass in their (daemon-connected) env.** This note is the structural pre-screen.

# /tdd brief — pull_request_status_derivation

## Feature
A pure, deterministic **`derive_pull_request_status`** fn — mapping daemon-internal GitHub PR signals (state / merged / draft / mergeability / review-decision / checks-conclusion) to the **frozen 11-state `PullRequest` status enum** via a precedence, plus the daemon-internal `PullRequestSignals` input model. The deterministic core of the `proj_pull_request` cache derivation (the projector + the octocrab adapter that *feed* it stay gated). Same pure-fn pattern as edges-002's worktree precedence.

## Use case + traceability
- **Task ID:** P7.1 (the PR-status-derivation portion — the logic the `proj_pull_request` projector will call; the projector + octocrab sync + PR-sync events are gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §7.2` (PullRequest SoT = **GitHub (octocrab)**; `proj_pull_request` is a synced cache w/ `pr_checked_at`; **re-fetch before merge/checks decision**), `§9` (octocrab typed reads — context for the gated adapter), `§11.2` (the PR Review Workspace that renders the derived status — context).
- **Widens phase scope because** the derivation produces a **§5.1** value — the `PullRequest` status machine is the cross-doc invariant Phase-7 task 7.2 extends ("PullRequest status, Appendix A, §5.1"); this slice **consumes** the frozen enum read-only and does not modify it.
- **Related context:** the frozen enum `shared/src/status.rs:81` (`PullRequest { Draft, Open, ChecksPending, ChecksFailing, NeedsReview, ChangesRequested, Approved, Mergeable, Conflict, Merged, Closed }`, terminal = Merged/Closed); the `proj_pull_request` DDL (`daemon/src/eventstore/schema.rs:198`, M3 — `pr_status`/`pr_checked_at`); the worktree precedence fn (edges-002 `git/precedence.rs`) — the **template** for this derivation; the §17 classifier (edges-003) — the sibling P7.1 in-lane piece.

## Acceptance criteria (what "done" means)
**Derivation (`daemon/src/integrations/pull_request.rs`):**
- [ ] `derive_pull_request_status(signals: &PullRequestSignals) -> PullRequest` returns the single most-salient §5.1 status.
- [ ] **Terminal wins:** `state=Closed & merged=true` → `Merged`; `state=Closed & merged=false` → `Closed` (both dominate every open-state signal).
- [ ] `state=Open & draft=true` → `Draft`.
- [ ] Merge conflict (mergeable = conflicting) → `Conflict`.
- [ ] Checks failing → `ChecksFailing`; checks pending → `ChecksPending`.
- [ ] Review `ChangesRequested` → `ChangesRequested`; review `Approved` + clean + checks-pass → `Mergeable`; review `Approved` but not-yet-clean → `Approved`; review required/none → `NeedsReview`.
- [ ] A bare open PR with no other signal → `Open`.
- [ ] The derived value is a **frozen `PullRequest` enum** variant (no new status, no contract change).
- [ ] **Total + deterministic** — every signal combination resolves to exactly one status (table-pinned), no panic, no `unwrap`.

**General:**
- [ ] Unit tests pass; `/preflight` clean. **No `shared/` touch, no migration, no `gateway/` touch, no `eventstore` change, no new Cargo dep** (`PullRequestSignals` is daemon-internal; consumes the frozen `PullRequest` enum read-only).

## Wiring / entry point (Step 7.5)
**`none — wiring lands in the gated 7.1 adapter/projector slice.`** The fn is pure; its consumers are the `proj_pull_request` projector (folds `PullRequestSynced` events → rows, *calling* this derivation) and the octocrab adapter that maps a GitHub API response → `PullRequestSignals` — both **gated** (octocrab dep + the `PullRequestSynced` shared event type + the projector). Reachability intentionally deferred (named).

## Files expected to touch
**New:**
- `daemon/src/integrations/pull_request.rs` — `PullRequestSignals { state, merged, draft, mergeable, review, checks }` (+ its daemon-internal sub-enums) + `derive_pull_request_status(...)`
- Test file: `daemon/tests/pull_request_status.rs` (or inline — Step-1 choice)

**Modified:**
- `daemon/src/integrations/mod.rs` — `pub mod pull_request;`

No `Cargo.toml` change. **Do NOT touch `gateway/`, `shared/`, `eventstore/`, or any migration.**

## RED test outline (Step 2)
1. **`pr_merged_terminal`** — `Closed + merged` → `Merged`; dominates even with conflict/failing-checks set. Why: §5.1 terminal.
2. **`pr_closed_terminal`** — `Closed + !merged` → `Closed`; dominates open signals. Why: §5.1 terminal.
3. **`pr_draft`** — `Open + draft` → `Draft`. Why: draft precedes the review flow.
4. **`pr_conflict`** — mergeable=conflicting → `Conflict`. Why: blocks merge.
5. **`pr_checks_failing`** — checks failing → `ChecksFailing`. Why: §5.1 CI state.
6. **`pr_checks_pending`** — checks pending → `ChecksPending`. Why: §5.1 CI state.
7. **`pr_changes_requested`** — review=ChangesRequested → `ChangesRequested`. Why: §5.1 review state.
8. **`pr_mergeable`** — Approved + clean + checks-pass → `Mergeable`. Why: ready-to-merge.
9. **`pr_approved_not_clean`** — Approved but not-yet-clean/checks-pending → `Approved` (not `Mergeable`). Why: the approved-vs-mergeable distinction.
10. **`pr_needs_review`** — review required/none, open, no blocker → `NeedsReview`. Why: §5.1 review state.
11. **`pr_open_baseline`** — open, no signals → `Open`. Why: baseline.
12. **`pr_full_precedence`** — a table test over the agreed precedence (asserts the total order). Why: §7.2-derived-cache totality.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none.** `PullRequestSignals` + its sub-enums are **daemon-internal**; the derivation **consumes** the frozen `PullRequest` enum read-only.
- **Shared-contract seam model touched?** **NO** — consuming the frozen `PullRequest` enum ≠ changing it; no new status/field/value → **no schema-snapshot, no CONTRACT_VERSION**.
- **Orchestrator doc rows to write hot:** one **arch note** — the GitHub-signals → `PullRequest` derivation precedence (the architecture lists the 11 states but does not pin the mapping; the daemon defines it, like the worktree precedence). I route it at the edges round close-out (multi-track: ARCHITECTURE.md integration-owned). **Flag the final precedence at Step 9.**

## Things to flag at Step 2.5
1. **The derivation precedence (THE design question). [CORRECTED 2026-06-12 — the original flat default put `ChecksPending` above the review states, which contradicted this brief's own test 9 (`Approved`+checks-pending → `Approved`); the implementer caught it at Step 2.5 and reconciled via a HARD/SOFT checks asymmetry.]** Reconciled precedence: `Merged > Closed > Draft > Conflict > ChecksFailing > {review-block: ChangesRequested | Mergeable | Approved | NeedsReview} > ChecksPending > Open`. **Key asymmetry:** `ChecksFailing` is a **HARD** blocker (above the review block — a red build outranks approval); `ChecksPending` is **SOFT** (below the review block — surfaces only when review=None, so `Approved`+checks-pending reads `Approved`, per test 9). The **review block is a match-cascade, not a flat rank** (the review states are mutually exclusive; `Mergeable` = review=Approved AND clean AND checks-success; `Approved` = review=Approved but not-yet-clean). **Must-hold pins:** terminal (Merged/Closed) dominates; Draft precedes review. The architecture lists the 11 states but does NOT pin the mapping (unlike worktree's §5.1-R7) → the daemon defines this precedence (recorded as an arch note). Low-stakes headline (the checks/review detail is preserved in `proj_pull_request`'s own columns).
2. **`PullRequestSignals` shape + its sub-enums.** My default vote: `{ state: PrState{Open,Closed}, merged: bool, draft: bool, mergeable: Mergeability{Clean,Conflicting,Unknown}, review: ReviewDecision{Approved,ChangesRequested,ReviewRequired,None}, checks: ChecksConclusion{Success,Failing,Pending,None} }` — daemon-internal (the octocrab adapter maps GitHub's API into these later). Confirm the sub-enum granularity (esp. whether `Mergeability` needs GitHub's `Blocked`/`Behind`/`Unstable` now or can collapse to Clean/Conflicting/Unknown for the MVP derivation).
3. **`Unknown` mergeability / missing signals.** GitHub returns `mergeable_state=unknown` while computing. My default vote: `Unknown` mergeability is **not** treated as `Conflict` (it falls through to the review/checks signals); a PR with all-unknown/none signals → `Open`. Confirm (conservative — don't show `Conflict` on a not-yet-computed merge).
4. **Where it lives.** My default vote: `daemon/src/integrations/pull_request.rs` (the integrations module, alongside the classifier). Alt: a `git/`-adjacent home. Default vote: `integrations/` (it's GitHub-derived).

## Dependencies + sequencing
- **Depends on:** nothing blocking — pure logic consuming the frozen `PullRequest` enum. (Sits in the `integrations/` module edges-003 created.)
- **Blocks:** the gated 7.1 adapter/projector slice (the octocrab adapter mapping GitHub → `PullRequestSignals`; the `proj_pull_request` projector calling `derive_pull_request_status`; the `PullRequestSynced` event) — gated on octocrab + the shared event type + the projector.

## Estimated commit count
**1.** A single pure derivation + its input model + the precedence table tests — one cohesive concern, no safety pin.

## Lessons-logged candidates anticipated
- **Convention candidate** — "GitHub→§5.1 `PullRequest` status is a pure precedence fn (signals as a daemon-internal input struct, table-tested) producing the frozen enum read-only — the §7.2 derived-cache value; same pattern as the worktree precedence fn."
- **Architecture-doc note candidate** — the GitHub-signals → `PullRequest` derivation precedence (the architecture lists the 11 states but doesn't pin the mapping).
- **Future TODO — belongs-to-a-phase (gated 7.1 adapter slice)** — the octocrab adapter (GitHub API → `PullRequestSignals`) + the `proj_pull_request` projector (`PullRequestSynced` → rows via this fn) + the `PullRequestSynced` shared event type.

## How to invoke
1. **Read this brief end-to-end** — Step-2.5 Q1 (the derivation precedence) is the load-bearing one.
2. **Run `/tdd pull_request_status_derivation`.**
3. **Step 0 (Restate)** — confirm: the pure derivation + input model only; projector/octocrab deferred.
4. **Step 1 (files)** — confirm; do NOT touch `gateway/`, `shared/`, `eventstore/`, migrations.
5. **Step 2.5** — send the test-design write-up + the 4 design answers (esp. the precedence); wait for `APPROVED.`
6. **Step 9** — restate the final precedence (for my arch-note carry) + surface anything beyond the anticipated candidates.

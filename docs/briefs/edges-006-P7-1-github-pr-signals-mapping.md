# /tdd brief — github_pr_signals_mapping

## Feature
A pure **`PullRequestSignals::from_github(...)`** — aggregating GitHub PR API facts (state / draft / merged / mergeable-state + the **reviews list** + the **check-runs list**) into the daemon-internal `PullRequestSignals` that edges-004's `derive_pull_request_status` consumes. Closes the GitHub-PR read chain: **raw GitHub → signals → §5.1 status**, all in-lane. **No octocrab / async / network** — the inputs are daemon-internal representations the heavier octocrab *client* (gated follow-on) will populate.

## Use case + traceability
- **Task ID:** P7.1 (the GitHub→signals aggregation; the octocrab network client + the proj_pull_request projector + the `PullRequestSynced` event stay gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (GitHub via octocrab — issues/PRs/**checks**/reviews), `§7.2` (PullRequest SoT = GitHub; `proj_pull_request` is the derived cache).
- **Widens phase scope because** the end-to-end chain tests compose with edges-004's derivation to produce a **§5.1** `PullRequest` value — the cross-doc invariant Phase-7 task 7.2 extends ("PullRequest status, §5.1"); this slice **consumes** the frozen enum read-only (via edges-004) and does not modify it.
- **Related context:** edges-004 (`integrations/pull_request.rs` — `PullRequestSignals` + `derive_pull_request_status`, which this *feeds*; this slice extends that module); the MVP `Mergeability{Clean,Conflicting,Unknown}` / `ReviewDecision{Approved,ChangesRequested,ReviewRequired,None}` / `ChecksConclusion{Success,Failing,Pending,None}` sub-enums (edges-004) are the **outputs** of this aggregation.

## Acceptance criteria (what "done" means)
**Aggregation (`daemon/src/integrations/pull_request.rs` extension):**
- [ ] `from_github(...)` builds `PullRequestSignals` from GitHub facts + the reviews/check-runs lists.
- [ ] **Review aggregation:** any `ChangesRequested` → `ChangesRequested`; else any `Approved` → `Approved`; else (review required / none) → `ReviewRequired` / `None`. (MVP-simple; the "required-approvals count" policy is deferred.)
- [ ] **Checks aggregation:** any failing → `Failing`; else any pending → `Pending`; else all success → `Success`; empty → `None`.
- [ ] **mergeable-state mapping:** GitHub `dirty` → `Conflicting`; `clean` → `Clean`; `blocked`/`behind`/`unstable`/`unknown` → `Unknown` (the MVP collapse — matches edges-004's `Mergeability`).
- [ ] state/draft/merged carried through (open/closed; draft bool; merged bool).
- [ ] **Total + deterministic** — every input combination resolves to one `PullRequestSignals`, no panic, no `unwrap`. Composing with `derive_pull_request_status` yields the right §5.1 status end-to-end (a couple of chain tests).

**General:**
- [ ] Unit tests pass; `/preflight` clean. **No `shared/` touch, no migration, no `gateway/` touch, no `eventstore` change, no new Cargo dep** (no octocrab yet — daemon-internal inputs; the octocrab client is the gated follow-on).

## Wiring / entry point (Step 7.5)
**`none — wiring lands in the gated 7.1 octocrab-client slice.`** Pure aggregation; its consumer is the octocrab adapter (fetches the PR + reviews + check-runs → calls `from_github` → `derive_pull_request_status` → the `proj_pull_request` projector). Reachability intentionally deferred (named).

## Files expected to touch
**New:**
- (none — extends `integrations/pull_request.rs`)

**Modified:**
- `daemon/src/integrations/pull_request.rs` — add the daemon-internal input enums (`GitHubMergeableState`, `ReviewState`, `CheckConclusion`) + `PullRequestSignals::from_github(...)`
- Test file: `daemon/tests/pull_request_status.rs` (extend) or a new `daemon/tests/github_pr_signals.rs` — Step-1 choice

No `Cargo.toml` change. **Do NOT touch `gateway/`, `shared/`, `eventstore/`, or any migration.**

## RED test outline (Step 2)
**Review aggregation:**
1. **`review_changes_requested_wins`** — `[Approved, ChangesRequested]` → `ChangesRequested`. Why: CHANGES_REQUESTED dominates.
2. **`review_approved`** — `[Approved]` (no changes-requested) → `Approved`. Why: approval.
3. **`review_required_when_none`** — `[]` / `[Commented]` → `ReviewRequired` (or `None` per the agreed default). Why: no decision yet.
**Checks aggregation:**
4. **`checks_failing_wins`** — `[Success, Failure]` → `Failing`. Why: a red check dominates.
5. **`checks_pending`** — `[Success, Pending]` (no failure) → `Pending`. Why: still running.
6. **`checks_all_success`** — `[Success, Success]` → `Success`. Why: green.
7. **`checks_empty_none`** — `[]` → `None`. Why: no CI.
**mergeable mapping:**
8. **`mergeable_dirty_conflicting`** — `dirty` → `Conflicting`; `clean` → `Clean`; `blocked`/`behind`/`unstable`/`unknown` → `Unknown`. Why: the MVP collapse.
**carry-through + end-to-end:**
9. **`state_draft_merged_carried`** — open/closed/draft/merged carried into the signals. Why: passthrough.
10. **`chain_merged_to_status`** — a merged PR `from_github` → `derive_pull_request_status` → `Merged`. Why: end-to-end chain.
11. **`chain_approved_clean_success_to_mergeable`** — approved + clean + all-checks-success → `Mergeable` end-to-end. Why: the happy chain.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none.** The input enums + `from_github` are **daemon-internal**; `PullRequestSignals` is already daemon-internal (edges-004).
- **Shared-contract seam model touched?** **NO** — no envelope/ID/status-machine/catalog/`EventTypeRegistry` change → **no schema-snapshot, no CONTRACT_VERSION**.
- **Orchestrator doc rows to write hot:** the review/checks aggregation rules + the mergeable-state collapse are an **arch note** (the GitHub→signals mapping, alongside the edges-004 derivation note). Flag at Step 9 for my close-out routing.

## Things to flag at Step 2.5
1. **Review aggregation rule (MVP scope).** My default vote: `ChangesRequested` (any) > `Approved` (any) > `ReviewRequired` (when reviews are required but undecided) / `None` (no reviews). **The MVP does NOT model the "required-approvals count" / per-reviewer-latest dedup** (deferred — GraphQL `reviewDecision` or a richer rule lands with the octocrab client). Confirm this MVP simplification + the `ReviewRequired`-vs-`None` boundary (when do we say "review required"?).
2. **Checks aggregation rule.** My default vote: any `Failing` > any `Pending` > all `Success` > empty `None`. Confirm the `CheckConclusion` input variant set — GitHub check-runs have `conclusion ∈ {success, failure, neutral, cancelled, timed_out, action_required, skipped, stale}` + a `status` (queued/in_progress/completed). My lean: collapse to `{Success, Failure, Pending, Neutral}` for the MVP (failure/timed_out/action_required→Failure; queued/in_progress→Pending; neutral/skipped→Neutral [ignored in the aggregation]; success→Success). Confirm the collapse.
3. **mergeable-state input.** My default vote: a daemon-internal `GitHubMergeableState{Clean,Dirty,Blocked,Behind,Unstable,Unknown}` (mirrors GitHub's `mergeable_state` strings) → mapped to the MVP `Mergeability`. The octocrab client maps GitHub's string → this enum. Confirm (vs. taking the raw string).
4. **Input shape.** My default vote: `from_github(state, draft, merged, mergeable: GitHubMergeableState, reviews: &[ReviewState], checks: &[CheckConclusion])`. Confirm (vs. a single `GitHubPrFacts` struct) — and confirm these daemon-internal input enums live in `pull_request.rs` (the octocrab client populates them later; no octocrab type leaks into this pure fn).

## Dependencies + sequencing
- **Depends on:** edges-004 (`PullRequestSignals` + `derive_pull_request_status`). No Gateway / `shared/` / octocrab dependency.
- **Blocks:** the gated 7.1 octocrab-client slice (fetches PR + reviews + checks → `from_github` → `derive_pull_request_status` → `proj_pull_request` projector via the `PullRequestSynced` event).

## Estimated commit count
**1.** A single cohesive aggregation extension + its input enums + the tests — one concern (GitHub→signals), no safety pin.

## Lessons-logged candidates anticipated
- **Convention candidate** — covered by the edges-004 derivation lesson (the GitHub→§5.1 chain is pure: `from_github` aggregates, `derive_pull_request_status` ranks; both daemon-internal, no octocrab leak) — likely a one-line extension, not a new lesson.
- **Future TODO — belongs-to-a-phase (gated 7.1 octocrab-client slice)** — the octocrab adapter (fetch PR/reviews/checks → the daemon-internal inputs), the richer review-decision rule (required-approvals / GraphQL `reviewDecision`), the `proj_pull_request` projector + the `PullRequestSynced` event.

## How to invoke
1. **Read this brief end-to-end** — Step-2.5 Q1/Q2 (the review + checks aggregation rules) are the design weight.
2. **Run `/tdd github_pr_signals_mapping`.**
3. **Step 0 (Restate)** — confirm: the pure aggregation + input enums only; octocrab client/projector/event deferred.
4. **Step 1 (files)** — confirm; do NOT touch `gateway/`, `shared/`, `eventstore/`, migrations.
5. **Step 2.5** — send the test-design + the 4 design answers; wait for `APPROVED.`
6. **Step 9** — restate the final aggregation rules (for my arch note) + surface anything beyond the anticipated candidates.

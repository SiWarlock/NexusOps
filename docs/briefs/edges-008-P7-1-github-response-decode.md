# /tdd brief — github_response_decode

## Feature
A pure, deterministic decode layer mapping GitHub's **raw REST API string values** for a pull request (top-level `state`, `mergeable_state`, each review's `state`, each check-run's `status`+`conclusion`) into the daemon-internal enums that edges-006's `PullRequestSignals::from_github` already consumes — completing the GitHub-fact→signals path **one layer below** edges-006. No octocrab, no async, no network: the gated octocrab client (next slice) calls these fns on its response strings.

## Use case + traceability
- **Task ID:** P7.1 (in-lane, Approach A — read/decode half; the network adapter + wiring stay deferred)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (GitHub = octocrab typed REST+GraphQL for issues/PRs/checks), `§7.2` (PullRequest SoT = GitHub; `proj_pull_request` is a synced cache w/ `pr_checked_at`; re-fetch before merge/checks decision).
- **Widens phase scope because** the end-to-end chain tests compose with edges-004/006 to produce a **§5.1** `PullRequest` value (the §7.2/§5.1 "PullRequest status" cross-doc invariant Phase-7 task 7.2 extends); this slice **consumes** the frozen §5.1 enum read-only (via `derive_pull_request_status`) and does not modify it. (Same posture + waiver as edges-006.)
- **Related context:** edges-006 (`18ad7f0`) landed `from_github` + the three target enums `GitHubMergeableState` / `ReviewState` / `CheckConclusion` whose **doc comments already pin the intended raw-string mapping** (`daemon/src/integrations/pull_request.rs:124–157`) — this slice is the executable decode those comments describe. edges-004 (`897a9f2`) landed `derive_pull_request_status`. The cross-cycle hand-off arch-note #5 (`docs/sessions/edges-002-*.md`) records the §9/§7.2 aggregation rules this sits below.

## Acceptance criteria (what "done" means)
- [ ] `parse_pr_state` maps `"open"→Open`, `"closed"→Closed`; an unrecognized string → `Open` (never fabricate the terminal Closed).
- [ ] `parse_mergeable_state` maps `"clean"→Clean`, `"dirty"→Dirty`, `"blocked"→Blocked`, `"behind"→Behind`, `"unstable"→Unstable`, `"unknown"→Unknown`; anything else (`"draft"`/`"has_hooks"`/empty/unrecognized) → `Unknown` (the conservative "not a conflict", per the `map_mergeable` doc).
- [ ] `parse_review_state` maps GitHub's UPPERCASE review states `"APPROVED"→Approved`, `"CHANGES_REQUESTED"→ChangesRequested`, `"COMMENTED"→Commented`, `"DISMISSED"→Dismissed`, `"PENDING"→Pending`; an unrecognized string → `Commented` (carries no decision in `aggregate_reviews`).
- [ ] `parse_check_conclusion(status, conclusion)` maps the **two-field** check-run shape: `status ∈ {queued, in_progress}` → `Pending` (conclusion ignored); `status == "completed"` → by `conclusion`: `success→Success`, `failure|timed_out|action_required→Failure`, `neutral|cancelled|skipped|stale→Neutral`, `None`/unrecognized → `Neutral`. (Matches the `CheckConclusion` doc comment exactly.)
- [ ] `signals_from_github_response(...)` composes the four parsers over the raw response (state str, draft bool, merged bool, mergeable str, `&[review str]`, `&[(status str, Option<conclusion str>)]`) and returns a `PullRequestSignals` (via `from_github`) — i.e. the full raw→signals decode in one call.
- [ ] **End-to-end pin:** a realistic raw response (`state="open"`, not draft, not merged, `mergeable_state="dirty"`, reviews `["APPROVED"]`, checks `[("completed","success")]`) → `signals_from_github_response(...)` → `derive_pull_request_status` == `PullRequest::Conflict` (dirty dominates an approving review — the decode→from_github→derive chain holds).
- [ ] Every parser is **total** (no panic / no `unwrap` on input; every `&str` resolves to exactly one variant) and pure (no `Clock`/`IdGen`/IO).
- [ ] All unit tests in `daemon/src/integrations/pull_request.rs` (the `#[cfg(test)]` module) pass.
- [ ] `/preflight` clean (`cargo fmt --check && clippy -D warnings && check && test`).
- [ ] Cross-doc invariant: **none** (daemon-internal pure fns over the already-landed edges-006 enums; no `shared/` model, no contract surface — confirm "none" at Step 9).

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated GitHub read-client + executor slices.** The decode fns are consumed by (a) the **next in-lane slice** — the real `OctocrabGithubReadClient` network adapter (trait + fake + octocrab REST/GraphQL fetch), which calls these parsers on octocrab's response strings — and ultimately (b) the **deferred** `proj_pull_request` projector + the `github` executor (gated on the R1 daemon seam + the `PullRequestSynced` event type). Tested-but-unwired **by design** (Approach A), exactly like edges-004/006: Step 7.5 grep-confirms only the test module references the new symbols. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**Modified:**
- `daemon/src/integrations/pull_request.rs` — append a decode block **below** the edges-006 `from_github` aggregation block: `parse_pr_state` / `parse_mergeable_state` / `parse_review_state` / `parse_check_conclusion` + the `signals_from_github_response` composer, and their `#[cfg(test)]` tests. (Extends the existing file — matches the edges-005/006 "extend, don't spawn" precedent; the enums these produce already live here.)

If the impl judges the file is getting unwieldy and prefers a new `integrations/github.rs` (or `integrations/github/mod.rs`) module home, that's the Step-2.5 Q3 call — flag before GREEN.

## RED test outline (Step 2)
Tests in the `#[cfg(test)]` module of `daemon/src/integrations/pull_request.rs`:

1. **`parse_pr_state_open_closed_unknown`** — Asserts: `"open"→Open`, `"closed"→Closed`, `"weird"→Open`. Why: §7.2 PR top-level state; unknown must not fabricate a terminal.
2. **`parse_mergeable_state_all_known`** — Asserts: each of clean/dirty/blocked/behind/unstable/unknown maps to its variant. Why: §9 mergeable decode; `map_mergeable` consumer contract.
3. **`parse_mergeable_state_unrecognized_is_unknown`** — Asserts: `"draft"`, `"has_hooks"`, `""` → `Unknown`. Why: conservative "not a conflict" (the `map_mergeable` doc).
4. **`parse_review_state_uppercase_set`** — Asserts: the five GitHub UPPERCASE review states map; `"FOO"→Commented`. Why: §9 review decode; unknown carries no decision.
5. **`parse_check_pending_when_not_completed`** — Asserts: `("queued",None)` and `("in_progress",None)` → `Pending`. Why: §9 — a running build is SOFT-pending (the `CheckConclusion` doc).
6. **`parse_check_completed_conclusions`** — Asserts: `success→Success`; `failure`/`timed_out`/`action_required → Failure`; `neutral`/`cancelled`/`skipped`/`stale → Neutral`. Why: §9 check-conclusion collapse (the `CheckConclusion` doc, verbatim).
7. **`parse_check_completed_missing_or_unknown_conclusion_is_neutral`** — Asserts: `("completed",None)` and `("completed",Some("bogus"))` → `Neutral`. Why: total + conservative (ignored in `aggregate_checks`).
8. **`signals_from_github_response_composes_to_from_github`** — Asserts: a fully-specified raw response decodes to the same `PullRequestSignals` as a hand-built `from_github(...)` call with the equivalent enums. Why: the composer is exactly the parser-composition, no extra logic.
9. **`decode_to_derive_dirty_dominates_approved`** — Asserts: the end-to-end pin (open / not-draft / dirty / `["APPROVED"]` / `[("completed","success")]`) → `derive_pull_request_status == Conflict`. Why: the decode→from_github→derive chain holds (edges-004/006 precedence: Conflict > review).
10. **`decode_to_derive_failing_check_dominates_approved`** — Asserts: (open / not-draft / clean / `["APPROVED"]` / `[("completed","failure")]`) → `ChecksFailing` (HARD blocker overrides review). Why: pins the HARD-vs-SOFT distinction survives the raw decode.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none.
- **Orchestrator doc rows to write hot (Step 9 routing):** none new for `daemon/CLAUDE.md` cross-doc / Appendix A. **Anticipated (integration-owned, multi-track — FLAG not edit from `track/edges`):** an `ARCHITECTURE.md §9/§7.2` arch-note candidate (the raw-GitHub-string → daemon-enum **decode table** — the exact string sets per field — extending the hand-off arch-note #5 which captured the *aggregation* rules above it), and a `daemon/LESSONS.md` C-list extension to the edges-006 §9/§7.2 GitHub→§5.1 lesson. I route both at `/orchestrate-end` into the PLAN-DELTA hand-off (not edited in this worktree).
- **§2.5-seam (shared-contract) model touched?** **No** — the decode fns are daemon-internal over the edges-006 daemon-internal enums; no Appendix-A model, no `shared/` surface → **no schema-snapshot test**. (Same posture as edges-004/006.)

## Things to flag at Step 2.5
1. **Unknown-string defaults (the conservative floor).** Each parser must total over arbitrary input. My defaults: `pr_state→Open` · `mergeable_state→Unknown` · `review→Commented` · `check→Neutral`. **Default vote: as listed** — every default is the **least-salient / safest-for-derivation** value, so an unknown API string can never fabricate a *more-blocking* or *terminal* §5.1 status (it degrades toward `Open`, not toward `Conflict`/`Closed`/`ChecksFailing`). Surface if you'd argue a different floor for any field.
2. **Case handling.** GitHub returns **lowercase** for `state`/`mergeable_state`/`conclusion` but **UPPERCASE** for review `state`. Match GitHub's documented exact case, or lowercase-normalize each input before matching (review then matched against lowercased forms)? **Default vote: case-insensitive (normalize before match)** — defensive against API casing drift, costs nothing, and tests pin one case per field with a single mixed-case sanity test. Keep the canonical GitHub case in the doc comments.
3. **Module placement.** Extend `pull_request.rs` (cohesion — the produced enums live there; matches the edges-005/006 extension precedent) vs. a new `integrations/github.rs` module home anticipating the octocrab client. **Default vote: extend `pull_request.rs`** now; spawn `integrations/github/` when the real network adapter lands (next slice) and migrate the decode fns then if warranted. Cheap to move later; premature to split now.
4. **check parser signature.** `parse_check_conclusion(status: &str, conclusion: Option<&str>)` — the two-field GitHub shape (conclusion is only meaningful when `status=="completed"`). Confirm the `Option<&str>` conclusion (a non-completed check has no conclusion) and that `("completed", None)` defends to `Neutral`. **Default vote: as stated.**

## Dependencies + sequencing
- **Depends on:** edges-006 (`18ad7f0`) — `from_github` + `GitHubMergeableState`/`ReviewState`/`CheckConclusion`/`PrState` (landed). edges-004 (`897a9f2`) — `derive_pull_request_status` (the end-to-end pins call it).
- **Blocks:** the real `OctocrabGithubReadClient` network adapter (next in-lane slice — trait + `FakeGithubReadClient` + octocrab REST/GraphQL fetch, which calls these decode fns); and the deferred `proj_pull_request` projector + `github` executor (gated wiring — R1 seam + `PullRequestSynced` event type).

## Estimated commit count
**1.** A focused pure decode layer in one file — no safety invariant, no cross-doc change, ~90–130 lines + tests. Bundling criteria all met for a single slice; nothing here splits.

## Lessons-logged candidates anticipated
- **Convention candidate** — "GitHub raw-string → daemon-enum decode is **total + conservative**: an unrecognized API string maps to the **least-salient** variant so it can never fabricate a more-blocking/terminal §5.1 status; case-normalized for API-drift resilience. Decode (raw strings) and aggregate (`from_github`) are separate, separately-tested layers." (extends the edges-006 §9/§7.2 GitHub→§5.1 two-stage lesson — hand-off C-list #5).
- **Architecture-doc note candidate** — the raw-GitHub-value → daemon-enum **decode table** (the exact string sets per field) under the §9/§7.2 aggregation rules already captured in arch-note #5.
- **Future TODO — next-brief working set** — the `OctocrabGithubReadClient` network adapter (trait + fake + octocrab REST/GraphQL, incl. the `reviewDecision → ReviewRequired` GraphQL layering deferred at edges-006) is the immediate next in-lane slice that consumes this decode layer.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5".
2. **Run `/tdd github_response_decode`** in the implementer session (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (files)** — confirm `daemon/src/integrations/pull_request.rs` (extend).
5. **Step 2.5** — send the test-design write-up + answers to the 4 questions (or take defaults); wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
6. **Step 9** — surface the cross-doc "none" confirmation + the anticipated §9/§7.2 decode-table arch-note + C-list lesson extension (integration-owned — I route, you flag).

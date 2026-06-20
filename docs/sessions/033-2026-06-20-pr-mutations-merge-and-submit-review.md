# Session 033 — PR-mutation cat-1 arc: github.merge_pr (D9) + github.submit_review (D10)

- **Date:** 2026-06-20
- **Phase:** Phase 4 / **P4.7** (the ui-unblock work-order wave-2 PR-Workspace mutation surface; homed under Phase 4 like 4.4/4.5/4.6 — the §4.7 "Future arcs — PR-review mutations").
- **Predecessor:** [032 — PR-card diff-stats (D6) + get_pr_diff read RPC (D7)](032-2026-06-19-pr-card-diff-stats-and-get-pr-diff.md)
- **Successor:** _(next implementer session — team idles after D10 for the user's main→ui merge; resume per the lead)_

## Why this session existed

The §4.7 read surface (D6/D7) was sealed; the two **🔴 cat-1 PR-mutation writes** were the held next slices. The user steered the safety design for both (F1/F2, via the lead) before authoring. D9 (`github.merge_pr`) + D10 (`github.submit_review`) are the **first two GitHub WRITES beyond `create_pr`** — they go live (mechanism-first, fake-tested; the live authenticated call is auth-gated). After D10 the team idles for a main→ui merge (no next slice).

## What was built

### D9 — github.merge_pr (C1 `e544544` contract freeze · C2 `c1cd0be` the cat-1 vertical)

A cat-1, risk-3, NON-standing-grantable Gateway mutation merging a remote PR head→base via octocrab `pulls().merge()`, **SHA-pinned to the approved head**, emitting `PullRequestMerged` and folding `proj_pull_request` → terminal `Merged`. CONTRACT **0.40.0→0.41.0**; MVP 29→30.

**Files modified:** `shared/src/events.rs` (`PullRequestMerged{pr_number, merge_commit_sha?, merged_at}`), `shared/src/catalog.rs` (`github.merge_pr` = `entry_no_standing_grant(L3,Api,FromInputs,Github,refs,params)`), `shared/src/{schema,lib}.rs` (register + CONTRACT 0.41.0), `shared/contracts/schema/…json` (regen), `daemon/src/integrations/github_write.rs` (`MergePrArgs`/`MergedPr`/`map_merge_method`/`merge_pull_request` trait+Octocrab+Fake), `daemon/src/integrations/executor.rs` (`execute_merge_pr` + dispatch), `daemon/src/gateway/policy.rs` (NEW `GITHUB_MUTATION_TYPES` const + the F2 deny-before-risk arm), `daemon/src/projections/pull_request.rs` (`PullRequestMerged`→status=merged fold), `daemon/src/projections/mod.rs` + `daemon/src/gateway/pipeline.rs` (the PullRequest delta nudge). **Tests:** 16 (contract entry+snapshot, policy deny/require-approval/approve-all-exclusion, executor emit/validate/timeout/failure-classes/partial/map_merge_method/requires-ref/unknown-method, projector fold + 0-row no-op, production-path delta nudge).

### D10 — github.submit_review (C1 `de201c0` contract freeze · C2 `cbae5aa` the cat-1 vertical)

A cat-1, risk-3, NON-standing-grantable Gateway mutation submitting a PR review **verdict** (approve/request_changes/comment), **SHA-pinned to the reviewed head (`commit_id`)**, emitting `ReviewSubmitted` and folding `proj_review`. The SECOND GitHub write; a *communication/attestation* write (no branch/code mutation) but gated identically because an `approve` carries merge-gate power. CONTRACT **0.41.0→0.42.0**; MVP 30→31.

**Files created:** `daemon/tests/github_submit_review.rs` (the executor/map/edge test surface). **Files modified:** `shared/src/events.rs` (`ReviewSubmitted{review_id, pr_number, reviewer, state, body?, submitted_at?, commit_id?}`, reuses frozen `ReviewState`), `shared/src/catalog.rs` (`github.submit_review` entry), `shared/src/{schema,lib}.rs` (register + CONTRACT 0.42.0), `shared/contracts/schema/…json` (regen), `daemon/src/integrations/github_write.rs` (`SubmitReviewArgs`/`SubmittedReview`/`map_review_event`/`submit_review` trait+Octocrab+Fake), `daemon/src/integrations/executor.rs` (`execute_submit_review` + dispatch), `daemon/src/gateway/policy.rs` (`GITHUB_MUTATION_TYPES` += `github.submit_review`), `daemon/src/projections/review.rs` (`ReviewSubmitted`→proj_review fold, shared with the `ReviewSynced` fold), `daemon/src/projections/mod.rs` + `daemon/src/gateway/pipeline.rs` (the Review delta nudge). **Tests:** 16 (contract entry+snapshot, policy deny/require-approval/approve-all-exclusion, executor emit/validate/conditional-body/approve-empty+body/timeout/failure-classes/partial/map_review_event+wire-token/requires-ref/unknown-event, projector fold+submit-over-synced, production-path nudge). (`daemon/tests/github_merge_pr.rs` was also created in this session for D9.)

## Decisions made

- **F1 = risk-3 + `entry_no_standing_grant` for BOTH** (user-steered): every merge/submit gets a fresh per-action human approval; a plan-level approve-all can never cover either. The §6.2 floor is blast-radius/authority, NOT risk class (LESSON 32) — risk-3 `create_pr` stays grantable; these do not.
- **F2 = UI/IPC-requester-only**, via a single shared `GITHUB_MUTATION_TYPES` deny-before-risk gate in `CatalogPolicy::decide` (D9 introduced it; D10 extended the const). No agent/Brain merge or review verdict (§15 #8; the PIN-e session-lifecycle precedent generalized).
- **NEW events, not reuse:** `PullRequestMerged` (not `PullRequestSynced`), `ReviewSubmitted` (not `ReviewSynced`) — a write mutation emits its OWN typed audit event ("the user did X" ≠ "we synced X"). The D9 precedent set the pattern D10 followed.
- **SHA-pin both** (anti-race/audit-integrity): merge → `sha`, submit → `commit_id`, both REQUIRED operands (fail-closed on blank).
- **Fail-closed verb maps:** `map_merge_method` / `map_review_event` return octocrab's enum directly (no intermediate domain type — the precedent), unknown → `Err`, never a silent server default.
- **D10 octocrab finding → typed lower-level POST:** octocrab 0.53.1's high-level `create_review` is reachable only via the **deprecated** `pull_number()` (clippy `-D warnings` rejects it; only `pr_review_actions` for existing reviews survives). Used the upgrade-stable `octocrab.post("/repos/{o}/{r}/pulls/{n}/reviews", …)` — byte-equivalent to `create_review`'s internals, same URL interpolation as every github write (no new injection surface). This corrected the brief's "no high-level create_review" premise twice over.
- **D10 conditional-body rule:** body required-non-empty for request_changes/comment, optional for approve (GitHub's own 422 rule); both arms pinned.

## Decisions explicitly NOT made (deferred)

- **Per-hunk inline `comments[]`** (the prototype's per-hunk Accept/Reject/Request-fix) — DEFERRED to a focused §4.7 follow-on (its own security pass). `SubmitReviewArgs` omits `comments`; the POST sends `[]`. Mechanism present in octocrab; UI sends none.
- **The live authenticated merge/submit** — gated on the SHARED deferred per-repo keychain auth + its MANDATORY security-re-review gate (shared with D7/D9/D10; today `Octocrab::default()` unauthenticated → a live call 401→AuthFailed→Failed, fail-closed-correct, mechanism fake-tested).
- **`head_sha` exposure on `PullRequestRow`/`proj_pull_request`** for the live UI Merge/Review-submit buttons (the SHARED D9 follow-on for the SHA-pin/commit_id) — gated on the deferred PR-status-refresh sync.
- **D9 `merged:false` 200-edge classification** — kept ServerError→transient/retry (the create_pr numberless-201 fail-safe precedent); reconsider ClientError→GithubSyncFailed at the auth-lands re-review (live-edge, not unit-tested).

## TDD compliance

**Clean — no violations.** Both slices were strict RED→2.5→GREEN: the RED step was confirmed (compile-error on the missing impl symbols, the canonical Rust RED) before any implementation, for both contract.rs and the daemon test crates. Step-2.5 was reviewed by the orchestrator (APPROVED for both; D9 got 2 ADDs folded; D10 was APPROVED clean). The octocrab live HTTP round-trip is the non-deterministic edge, fake-covered per CLAUDE.md (not unit-tested) — the project's non-deterministic-coverage path.

## Reachability

- **D9 `github.merge_pr`** — reachable from the production UDS gateway: `submit_action`/`approve` → staged pipeline → `CatalogExecutor` dispatch on `ExecutorKind::Github` → `GithubExecutor::execute` → `execute_merge_pr`. `main.rs:273` registers `GithubExecutor` with the real `OctocrabGithubWriteClient`. Production-path delta nudge pinned via the real WriteActor gateway execute (`tests/runtime.rs`).
- **D10 `github.submit_review`** — same path → `execute_submit_review`. Same registration (no new `main.rs` wiring). Production-path nudge pinned in `tests/runtime.rs`.
- No tested-but-unwired gaps. Both auth-gated (mechanism-first); the live authenticated call is the deferred follow-up above.

## Open follow-ups

(Step-9 categorized items — already routed hot to the orchestrator; it wrote the cross-doc rows + LESSON candidate during the session, staged for its `/orchestrate-end` seal.)

- **Cross-doc (orchestrator-written, in tree):** Appendix-A ActionTypeCatalog rows (+github.merge_pr, +github.submit_review, risk-3 non-grant) · EventTypeRegistry (PullRequestMerged, ReviewSubmitted) · MVP-projections notes (the two folds) · CONTRACT 0.41.0→0.42.0 · daemon/CLAUDE.md cross-doc table · MVP 29→31. **Verified present** in the working tree (`git diff` ARCHITECTURE.md / daemon/CLAUDE.md / daemon/LESSONS.md) — the single-track happy path; rides the orch's round seal.
- **Future TODO (belongs-to-a-phase):** per-hunk inline `comments[]` · the live authenticated merge/submit + its MANDATORY security-re-review gate (shared w/ D7) · `head_sha` exposure for the live UI buttons · the D9 `merged:false` classification re-review.
- **FLAG (corrected):** the §6.3 catalog "deferred high-risk … merge" comment = `git.merge` (a deferred WORKTREE merge), UNAFFECTED by `github.merge_pr`/`github.submit_review` — NOT dropped (the brief conflated them; `git.merge` stays None, test-pinned). The github-write tier now has TWO realized writes.

## Preflight

**Daemon trust-core gate GREEN:** lint (`clippy -D warnings`) ✓ · type-check (`check --all-targets`) ✓ · **test 979/0** ✓ · daemon+shared `fmt --check` (`-p` scoped) ✓ · 3-way contract verify @ 0.42.0 ✓.

**Known cross-track note (not a D9/D10 blocker):** the whole-workspace `cargo fmt --check` is RED on **pre-existing ui-track files** (`ui/gateway-uds/src/lib.rs`, `ui/src-tauri/src/commands.rs`) — outside daemon territory, untouched by this session, present before it (also surfaced at the D9 Step-8 gate). Not fixable from the daemon track (the freeze/territory rule forbids editing ui/); flagged for the ui track.

## How to use what was built

A UI (or any getpeereid-trusted IPC peer) submits `github.merge_pr` / `github.submit_review` via the gateway with `{owner, repo, pr_number, sha|commit_id, merge_method|event, body?}` + a Repo `resource_ref`; it routes to a fresh per-action human approval (never approve-all); on approve the gateway executes the SHA-pinned octocrab call and emits `PullRequestMerged`/`ReviewSubmitted`, folding `proj_pull_request`/`proj_review` + nudging the live subscription. The live call needs the deferred per-repo keychain auth (today it fails closed unauthenticated).

# Session 028 — the rich-PR workspace + structured reviews (D5)

- **Date:** 2026-06-16
- **Phase:** Phase 4 / P4.6 (the UI-unblock work order tail — the D5 vertical)
- **Predecessor:** [027-2026-06-16-ui-unblock-d-series.md](027-2026-06-16-ui-unblock-d-series.md)
- **Successor:** _(none yet — session restarting after a scaffolding upgrade; NO respawn)_
- **Round seal:** `3d0cba5` (orchestrator `/orchestrate-end`) · **CONTRACT 0.38.0** · workspace green.

## Why this session existed

The UI-unblock work order's final arc: give the ui PR Review Workspace (§11.2) the rich data it needs —
**mergeability/checks** on the PR row, then a **structured-review** vertical (the per-review list), and finally
the **live producer** that fetches reviews from GitHub. Three ordered slices (D5a → D5b-1 → D5b-2), each an
additive CONTRACT bump, closing the full D5 vertical end-to-end (action → reviews → `proj_review` → typed serve → ui).

## What was built

### D5a — `PullRequestRow` mergeable/checks enrichment (`fa67c11`, CONTRACT 0.35→0.36)

**Files modified:**
- `shared/src/projections.rs` — `PullRequestRow` += `mergeable: Option<bool>` + `checks_summary: Option<String>`.
- `shared/src/lib.rs` — `CONTRACT_VERSION` → 0.36.0.
- `shared/contracts/schema/nexusops-contract.schema.json` — regenerated (`emit_schema`).
- `daemon/src/eventstore/schema.rs` — `MIGRATION_13` (ALTER-only; ADD `mergeable INTEGER`, `checks_summary TEXT`).
- `daemon/src/eventstore/migrations.rs` — register MIGRATION_13 + `SUPPORTED_USER_VERSION` 12→13.
- `daemon/src/projections/pull_request.rs` — fold the 2 fields (INSERT + ON CONFLICT DO UPDATE).
- `daemon/src/ipc/methods.rs` — `read_pull_request_typed` coerces SQLite-INTEGER(0/1)→JSON-bool for `mergeable`.
- tests: `shared/tests/contract.rs`, `daemon/tests/projections.rs`, `daemon/tests/gateway_plan.rs` (+3 daemon tests).

The data was already in the `PullRequestSynced` event; D5a surfaces it. Lockstep slice (the fail-closed typed serve
forces column+fold+row-field+bump atomic). `mergeable` is the **first bool projection column** → the read layer
coerces INTEGER→bool while the `shared/` contract stays a pure `Option<bool>`.

### D5b-1 — the structured-review contract + projection (`90cadd2`, CONTRACT 0.36→0.37)

**Files created:**
- `daemon/src/projections/review.rs` — the `ReviewProjector` (folds `ReviewSynced`→`proj_review`; review_id PK,
  project_id-envelope + repo_id-sibling-read; healthy-skip/fail-closed taxonomy; rebuild-safe).

**Files modified:**
- `shared/src/events.rs` — the `ReviewSynced` event + `EVENT_TYPE`.
- `shared/src/status.rs` — `ReviewState` (a plain VALUE enum, snake_case, reject-unknown — NOT `status_machine!`).
- `shared/src/projections.rs` — `ReviewRow` (the 4th frozen projection-row).
- `shared/src/ipc.rs` — `ProjectionName::Review` (+ `ALL`).
- `shared/src/schema.rs` — `ContractBundle` += review types. `shared/src/lib.rs` — 0.37.0.
- `daemon/src/eventstore/{schema.rs (MIGRATION_14 CREATE), migrations.rs (13→14)}`.
- `daemon/src/projections/{mod.rs (register + `EventDeltaIds.review_id` + the `deltas_for_event` Review arm + the
  §51 guard w/ negative arm), schema.rs (REBUILD_TABLES += proj_review)}`.
- `daemon/src/gateway/pipeline.rs` — `emitted_event_deltas` `ReviewSynced` branch.
- `daemon/src/ipc/{methods.rs (route + `read_review_typed`), mod.rs (export)}`.
- tests: `shared/tests/contract.rs`, `daemon/tests/{projections.rs, session_executor.rs, gateway_plan.rs}` (+12 tests).

Fixture-fed (the live producer = D5b-2). `body` is free-form user text → §15-redacted at the event (security-CLEAR).

### D5b-2 — the `github.sync_reviews` review producer (`76b7f4a`, CONTRACT 0.37→0.38)

**Files modified:**
- `shared/src/catalog.rs` — `github.sync_reviews` (`MVP_ACTION_TYPES` 28→29; `entry(R::Level1, P::Api, I::None, X::Github, true, true)`).
- `shared/src/lib.rs` — 0.38.0. `shared/contracts/schema/...json` — regen (version-field only, no new type).
- `daemon/src/integrations/executor.rs` — `execute_sync_reviews` (validate→`list_reviews`→emit one `ReviewSynced`/review,
  `side_effect_applied=false`) + `classify_failure` generalized to an op-label + `u64_input` helper + dispatch + const.
- `daemon/src/integrations/github_write.rs` — `SyncReviewsArgs` + `ReviewData` + the pure `map_review_state` (unit-tested)
  + the `list_reviews` trait method/octocrab impl/refactored fake (Option-wrapped modes).
- tests: `shared/tests/contract.rs`, `daemon/tests/github_executor.rs` (+6 tests).

Reuses the §46 SYNC-external-network pattern (captured Handle + `block_on` + mandatory timeout; failure→`GithubSyncFailed`
via `FailedWithEvents`). **The full D5 vertical is now live end-to-end.**

## Decisions made

- **D5a — `mergeable` INTEGER→bool coercion in the read layer** (not a custom contract deserializer): the `shared/`
  `PullRequestRow.mergeable` stays a pure `Option<bool>`; the daemon `read_review_typed`/`read_pull_request_typed`
  coerce SQLite's INTEGER representation to the wire bool. Pinned for true/false/None (orchestrator ADD: the `false`/0 edge).
- **D5b-1 — `ReviewState` is a plain VALUE enum, not `status_machine!`** (orchestrator TWEAK): a review *is* a fixed
  verdict (no lifecycle), so a phantom `is_terminal()` would misrepresent the model.
- **D5b-1 — `review_id` is the PK** (globally-unique GitHub id), not a composite; repo_id sibling-read mirrors `PullRequestProjector`.
- **D5b-2 — `github.sync_reviews` = risk-1, `standing_grant_eligible=true`** (orchestrator RULING; the **precedent for
  github network READS**): a network read is NOT risk-0 auto-execute (an untrusted proposer hammering the GitHub API is a
  rate-limit/exposure vector) but below the risk-2 github writes (no mutation/credential). Caught + corrected the brief's
  factual slip (it called `project.rescan` risk-1; it is risk-0, a local FS read).
- **D5b-2 — `I::None` idempotency** (a re-sync re-runs — re-fetching fresh reviews is the point), `side_effect_applied=false`
  (a clean read rollback), and the octocrab `ReviewState`→shared mapping is a pure unit-tested fn (orchestrator ADD).

## Decisions explicitly NOT made (deferred)

- **`review_synced_at` on the `proj_review` row** — intentionally not projected (the event carries it as bookkeeping;
  `submitted_at` is the display timestamp). Additive-later if a "last synced" indicator is needed.
- **`list_reviews` pagination** — first-page-only (`per_page=100`); >100 reviews on one PR is a follow-on.
- **A periodic/triggered review-resync** — today on-demand via the action.
- **3.3c (the CAT-1 Codex interception)** — DESIGN-COMPLETE, held post-cycle (not in this session's scope).

## TDD compliance

**Clean — no violations.** All 3 slices ran strict RED→Step-2.5→GREEN; every non-trivial change had a failing test first
(confirmed RED for the right reason). The octocrab `list_reviews` live HTTP round-trip is the non-deterministic edge,
fake-covered per CLAUDE.md (the project's non-deterministic-coverage path) — and its deterministic core (`map_review_state`)
was pulled out and unit-tested. Reviewer-found code-quality items were fixed in-slice (D5a 1 deferred; D5b-1 2M+1L fixed;
D5b-2 all 5 fixed). security-reviewer CLEAR on the D5b-1 + D5b-2 §15 `body` surface (no Step-9 Findings).

## Cross-doc invariant audit

**Clean.** Every model field change this session was flagged at Step 9 and the orchestrator wrote the doc rows hot —
all sealed in `3d0cba5`: ARCHITECTURE.md Appendix-A (`PullRequestRow` enriched · NEW `ReviewRow` · `ReviewSynced`
EventTypeRegistry · `ProjectionName::Review` · `github.sync_reviews` catalog row) + daemon/CLAUDE.md (MVP-projections,
EventTypeRegistry, §6.3 catalog, cross-doc CONTRACT version) + LESSONS §53/§54. No drift.

## Reachability

- **D5a** — `get_projection(PullRequest)→read_pull_request_typed` (existing RPC) + the existing `PullRequestProjector`. Reachable.
- **D5b-1** — serve (`get_projection(Review)→read_review_typed`) + nudge (gateway `emitted_event_deltas`) reachable; the
  fold was fixture-fed (live producer deferred to D5b-2).
- **D5b-2** — `github.sync_reviews`→`CatalogExecutor`→`GithubExecutor`→`execute_sync_reviews`→`ReviewSynced` in txn-B →
  the D5b-1 `ReviewProjector`→`proj_review` + the Review nudge. **Closes D5b-1's fixture-fed gap — the full D5 vertical
  is live end-to-end.** No tested-but-unwired gaps remain.

## Open follow-ups

- **`list_reviews` pagination** (>100 reviews/PR) — Future TODO (a GitHub-fetch slice).
- **`review_synced_at` on the row** — additive-later if a "last synced" indicator is needed.
- **Periodic/triggered review-resync** — today on-demand.
- **The deferred `auth_expired` `*SyncFailed` variant** still applies to `github.sync_reviews` (the create_pr park).
- **3.3c (CAT-1 Codex interception)** — DESIGN-COMPLETE, held post-cycle.
- **D4b future-CONTRACT flag** — partially closed (`proj_review` now has a subscribe-name); proj_project/proj_repository/
  proj_integration_connection still lack `ProjectionName` variants (a CONTRACT change when a ui consumer needs them).

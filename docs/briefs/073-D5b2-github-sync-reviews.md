# /tdd brief — github_sync_reviews (D5b-2)

## Feature
The **live producer** for D5b-1's review projection: a new **`github.sync_reviews`** catalog action whose
`GithubExecutor` arm fetches a PR's reviews (octocrab `pulls().list_reviews()` via the injected write-client
seam) and emits one **`ReviewSynced`** event per review. Completes the full D5 (the rich PR workspace with
reviews). **Additive CONTRACT 0.37.0 → 0.38.0** (a new catalog action_type — the `integration.connect`
precedent). **NON-cat-1** (a read-that-emits action; fake-client-tested), **security-reviewer opt-in**
(a new external-touching catalog action).

> **Split context:** D5b-1 (`90cadd2`) froze the `ReviewSynced` event + `proj_review` + `ReviewRow` + the
> nudge (fixture-fed). D5b-2 is the producer. Cross-track CLEARED (edges completely finished/closed — zero
> collision risk; lead-relayed 2026-06-16).

## Use case + traceability
- **Task ID:** D5b-2 (the user's UI-unblock work order) — **P4.6** the GitHub review-fetch producer.
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (the GitHub integration executor), **§6.3**
  (the `github.sync_reviews` ActionTypeCatalog entry + risk), **§7.1** (it emits the `ReviewSynced` event),
  **§17** (the integration-failure contract — list_reviews fails → `GithubSyncFailed`).
- **Widens phase scope because** it's a §9/§6.3 GitHub-integration action emitting the §7.1 review event for
  the §11.2 PR Workspace — cross-cutting beyond Phase 4's §8/§17 anchors (the D5a/D5b-1 precedent).
- **Related context:** the `GithubExecutor` (`daemon/src/integrations/executor.rs` — `execute_create_pr`
  pattern: validate inputs → `handle.block_on(timeout(client.<call>))` → classify_failure → emit via
  `ExecutionOutcome::Succeeded{emitted_events}`; `GITHUB_CREATE_PR` const at `:48`); the `GithubWriteClient`
  seam + `OctocrabGithubWriteClient` (`daemon/src/integrations/github_write.rs` — gains `list_reviews`); the
  github catalog entries (`shared/src/catalog.rs:108`/`:310` `github.create_pr*` — the entry shape to
  mirror); `ReviewSynced` (D5b-1, `shared/src/events.rs`) the emit target; `project.rescan` (a read-that-
  emits action — mirror its risk); LESSONS §46 (the SYNC-external-network executor: captured `Handle` +
  `block_on` + mandatory timeout + `*SyncFailed` via `FailedWithEvents`), LESSONS §49 (a catalog-action no-bypass).

## Acceptance criteria (what "done" means)
- [ ] A new **`github.sync_reviews`** catalog action (`shared/src/catalog.rs`: `MVP_ACTION_TYPES` += the
      type + the `entry(...)`: risk per Step-2.5 Q1, `ExecutorKind::Github`, `requires_resource_refs=true`
      [a Repo ref], `standing_grant_eligible` per Q1). **CONTRACT 0.37.0 → 0.38.0.**
- [ ] The `GithubExecutor` handles `github.sync_reviews`: validate inputs (`owner`/`repo`/`pr_number`) →
      `list_reviews` via the injected seam (captured `Handle` + `block_on` + the mandatory `NETWORK_TIMEOUT`,
      LESSONS §46) → emit one `ReviewSynced` per review (review_id/pr_number/reviewer/state/submitted_at?/
      body?/review_synced_at[injected Clock]) via `ExecutionOutcome::Succeeded{emitted_events}`.
- [ ] Zero reviews → zero `ReviewSynced` (clean empty).
- [ ] `list_reviews` failure → `GithubSyncFailed` via `ExecutionOutcome::FailedWithEvents` (reuse
      `classify_sync_failure`, the terminal-non-auth path; LESSONS §46/§49).
- [ ] `GithubWriteClient` (the seam) + `OctocrabGithubWriteClient` (octocrab `pulls().list_reviews()`) +
      the fake test-client gain `list_reviews` (additive; no signature change to the existing methods).
- [ ] The catalog snapshot + the `MVP_ACTION_TYPES` count + the **3-way verify** GREEN @0.38.0.
- [ ] All tests pass; `/preflight` clean; security-reviewer pass.

## Wiring / entry point (Step 7.5)
`github.sync_reviews` is a Gateway action → the `CatalogExecutor` dispatches to the `GithubExecutor` (already
registered, `ExecutorKind::Github`) → `execute_sync_reviews` → `emitted_events` appended in the gateway txn-B
→ the D5b-1 `ReviewProjector` folds them into `proj_review` + the `deltas_for_event` Review nudge fires (the
D4b gateway path). **The full D5 vertical is then live end-to-end** (action → reviews → proj_review → typed
serve → ui). Reachable from the Gateway (a submitted `github.sync_reviews` action).

## Files expected to touch
**Modified:** `shared/src/catalog.rs` (the action + entry) · `shared/src/lib.rs` (0.38.0) ·
`shared/contracts/schema/nexusops-contract.schema.json` (regen) · `shared/tests/contract.rs` (catalog
snapshot + version + 3-way) · `daemon/src/integrations/executor.rs` (the `execute_sync_reviews` arm) ·
`daemon/src/integrations/github_write.rs` (the `list_reviews` seam + octocrab impl + fake) ·
`daemon/tests/github_executor.rs` (the fake-client tests).

## RED test outline (Step 2)
1. **`test_github_sync_reviews_emits_review_synced_per_review`** — a fake `list_reviews` returning N reviews
   → N `ReviewSynced` emitted_events (fields mapped from each review). Why: §9/§7.1 — the producer.
2. **`test_github_sync_reviews_empty_emits_nothing`** — zero reviews → zero events. Why: clean empty.
3. **`test_github_sync_reviews_validates_inputs`** — missing `owner`/`repo`/`pr_number` → a validation
   `ExecutionOutcome::Failed`. Why: §6.3 input contract (the create_pr precedent).
4. **`test_github_sync_reviews_failure_is_github_sync_failed`** — the fake `list_reviews` errs (transient/
   terminal) → `GithubSyncFailed` via `FailedWithEvents` (classify_sync_failure). Why: §17 / LESSONS §46.
5. **the catalog snapshot** — `github.sync_reviews` entry (risk/executor/refs) + `MVP_ACTION_TYPES` count +
   CONTRACT 0.38.0 + the **3-way verify** @0.38.0, tagged `spec(§6.3)`. Why: §5.0/§2.5-seam.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `MVP_ACTION_TYPES` += `github.sync_reviews` + the catalog entry. **CONTRACT
  0.37.0 → 0.38.0.**
- **Orchestrator doc rows to write hot (Step 9):** the §6.3 ActionTypeCatalog cross-doc row
  (daemon/CLAUDE.md) + the ARCHITECTURE.md Appendix-A catalog row — add `github.sync_reviews`. Orchestrator
  writes at the seal.
- **§2.5-seam (shared-contract) model touched?** **YES** — the catalog (§6.3, snapshot-pinned). The catalog
  snapshot test (RED #5) is REQUIRED.

## Things to flag at Step 2.5
1. **Risk + standing_grant — mirror `project.rescan`.** `github.sync_reviews` is a read-that-emits action
   (no mutation). My default vote: **mirror `project.rescan`'s risk** (likely risk-1, approval-gated; a
   network read) + `standing_grant_eligible` per the catalog floor. Confirm against `project.rescan`'s
   actual entry. Flag if you'd make it risk-0 auto-execute (a read — but the network call argues for
   approval-gating).
2. **Inputs + Repo ref.** `owner`/`repo`/`pr_number` to identify the PR + a **Repo resource_ref** (so the
   D5b-1 projector's `repo_id` sibling-read resolves — per the D5b-1 note). Mirror `github.create_pr`'s
   input + ref pattern. Default: yes.
3. **`review_synced_at` = injected Clock** (the daemon's Clock at emit, NOT GitHub's `submitted_at` which is
   the review's own timestamp). Default: yes (the daemon-Clock UTC-Z, LESSON §5 family).
4. **The fake-client seam.** Extend the existing `GithubWriteClient` fake (the `github_executor.rs` test
   double) with `list_reviews` returning canned reviews. Default: yes (the create_pr fake precedent).

## Dependencies + sequencing
- **Depends on:** D5b-1 (✅ `90cadd2` — the `ReviewSynced` event + `proj_review`), the edges GitHub baseline
  (✅ the `GithubExecutor` + `OctocrabGithubWriteClient`, CLEARED). All landed.
- **Blocks:** nothing — **D5b-2 is the LAST D-item**; the full D5 vertical is live after it. Then: seal →
  idle (NO 3.3c) → user remerge → `/team-end`.

## Estimated commit count
**1.** A coherent "new read-action producer" slice (catalog entry + executor arm + the seam method + the
fake + the CONTRACT bump). The `integration.connect`/`github.create_pr` precedent (a new external-touching
catalog action in one slice). **security-reviewer runs (opt-in)** — a new catalog action touching the
network; confirm the risk-classification + the INV-SEC-1 no-bypass (emit via the gateway, not a side path) +
the §17 failure path + that the `ReviewSynced.body` redaction (D5b-1-confirmed) holds at this emit site.

## Lessons-logged candidates anticipated
- **Convention candidate** — a producer for a fixture-fed projection (D5b-1 built the projection; D5b-2 the
  producer) closes the "projection built, producer-next" deferral via a read-that-emits catalog action
  (the project.rescan / LESSONS §46 pattern).
- **Architecture-doc note** — §6.3/§9: `github.sync_reviews` (the review producer); the full review vertical
  is live.
- **Future TODO** — a periodic/triggered review-resync (today on-demand via the action); rich review fields.

## How to invoke
1. **Read this brief end-to-end** (the LESSONS §46 sync-executor pattern + Step-2.5 Q1 the risk are load-bearing).
2. **Run `/tdd github_sync_reviews`**.
3. **Step 0 (Restate)** — confirm the new read-action producer + the full-D5 completion.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — answer the 4 design questions; the catalog snapshot test is in this cycle.
6. **Step 9** — flag the cross-doc catalog rows + the CONTRACT bump + the security-reviewer result.

> **Step-8 reviewer policy:** `code-quality-reviewer` runs (`every-slice`). **`security-reviewer` runs
> (orchestrator opt-in)** — a new external-touching catalog action; confirm risk-classification + INV-SEC-1
> no-bypass + the §17 failure path + the body-redaction-holds-at-emit. NON-cat-1.

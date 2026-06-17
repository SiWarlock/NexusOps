# /tdd brief — review_contract_projection (D5b-1)

## Feature
Freeze the **structured-review CONTRACT + projection** (the daemon-track, deterministic half of D5b): a new
`ReviewSynced` event + a frozen `ReviewState` value enum + the typed `ReviewRow` + a new
`ProjectionName::Review` variant + the `proj_review` projection (MIGRATION_14) + the `ReviewProjector` fold +
the `read_review_typed` fail-closed serve + the `deltas_for_event` Review nudge. Fed by **synthetic
`ReviewSynced` events** in tests (the live GitHub producer = D5b-2). **Additive CONTRACT 0.36.0 → 0.37.0.**
**NON-cat-1** (a new projection vertical; the ②-mini/P7.2/D2 "new projection freeze" pattern), but
**security-reviewer opt-in** (the `body` is free-form user review text — a §15 surface).

> **Split note:** D5b = D5b-1 (this — the contract + projection, daemon-track) + D5b-2 (the GitHub fetch
> that emits `ReviewSynced`, edges-touching). They co-land this round; the cross-track ownership flag (edges
> sealed `fb85938`, additive) is LOW and surfaced lead→user. D5b-1 stands alone (synthetic events test the
> fold/serve/nudge); D5b-2 wires the real producer.

## Use case + traceability
- **Task ID:** D5b-1 (the user's UI-unblock work order) — **P4.6** the structured-review contract+projection.
- **Architecture sections it implements:** `ARCHITECTURE.md §7.1` (the `ReviewSynced` EventTypeRegistry
  entry), **§7.2** (the `proj_review` read model + the typed-serve SoT), **§6.1** (the new
  `ProjectionName::Review` closed-set subscribe variant), **§11.2** (the PR Review Workspace the reviews
  feed), **§5.0** (the contract SoT + the §2.5-seam freeze).
- **Widens phase scope because** it's a new §7.1/§7.2/§11.2 projection vertical + a §6.1 closed-set addition
  + a §5.0 contract bump citing cross-cutting sections beyond Phase 4's §8/§17 anchors (the 4.4/4.5/D5a
  precedent).
- **Related context:** the `PullRequestSynced` event (`shared/src/events.rs:467-488`) the `ReviewSynced`
  mirrors (a `Namespaced` payload event); the §5.1 frozen status enums (`shared/src/status.rs`) the
  `ReviewState` value-enum mirrors (reject-unknown, snake_case wire); the `PullRequestProjector`
  (`daemon/src/projections/pull_request.rs`) the `ReviewProjector` mirrors; the typed rows + `read_*_typed`
  (`shared/src/projections.rs` + `daemon/src/ipc/methods.rs`) the LESSONS §37 pattern; `ProjectionName`
  (`shared/src/ipc.rs:126-153`); the migration list (`migrations.rs:15` `SUPPORTED_USER_VERSION = 13`); the
  `deltas_for_event` shared mapping (`daemon/src/projections/mod.rs`, D4b — ReviewSynced is gateway-emitted,
  so it needs a Review arm there); LESSONS §37 (typed projection-row freeze), §48 (a `proj_*` registry
  projection — fold the coarse event, key by available identity), LESSONS §50 (migration floor test + exact-latest
  pin), LESSONS §51 (the delta-source↔projector agreement — add the Review arm), LESSONS §17 (fold-from-event rebuild-safe).

## Acceptance criteria (what "done" means)
- [ ] `ReviewSynced` event (`shared/src/events.rs`) — payload `{review_id: u64, pr_number: u64, reviewer:
      String, state: ReviewState, submitted_at: Option<Timestamp>, body: Option<String>, review_synced_at:
      Timestamp}` + `EVENT_TYPE = "ReviewSynced"`; registered in the EventTypeRegistry.
- [ ] `ReviewState` value enum (`shared/src/status.rs` or the status home) — `approved`/`changes_requested`/
      `commented`/`dismissed`/`pending` (snake_case wire, reject-unknown — the §5.1 precedent).
- [ ] `ProjectionName::Review` added to the closed enum + `ALL` (declaration-order stable).
- [ ] `proj_review` table (MIGRATION_14: CREATE; `SUPPORTED_USER_VERSION` 13→14; `REBUILD_TABLES` +=
      `proj_review`) — `{review_id PK, pr_number, project_id, repo_id, reviewer, state, submitted_at?, body?,
      updated_at_seq}`.
- [ ] The `ReviewProjector` folds `ReviewSynced` → `proj_review` (INSERT + ON CONFLICT DO UPDATE; keyed by
      `review_id`; `state` binds `ReviewState` reject-unknown; rebuild-equivalent, LESSONS §17/§48).
- [ ] `ReviewRow` (`shared/src/projections.rs`, `deny_unknown_fields`) served typed via
      `get_projection(Review)` → `read_review_typed` (fail-closed; the LESSONS §37 pattern); Some+None round-trip.
- [ ] `deltas_for_event` gains a `ReviewSynced` → `ProjectionName::Review` arm (the D4b shared mapping;
      keyed by `review_id`) so a gateway-emitted `ReviewSynced` nudges a Review subscriber (LESSONS §51).
- [ ] **CONTRACT_VERSION 0.36.0 → 0.37.0**; the `ReviewSynced` + `ReviewRow` + `ProjectionName`
      **schema-snapshots** updated + the **3-way verify** GREEN @0.37.0.
- [ ] All tests pass; `/preflight` clean; security-reviewer pass (the `body` §15 surface).

## Wiring / entry point (Step 7.5)
The fold rides the `ReviewProjector` registered in the projection engine (fed by `ReviewSynced` — synthetic
in D5b-1 tests; the live GitHub producer is **D5b-2**). The serve is `get_projection(Review)` →
`read_review_typed` (the ui PR Review Workspace §11.2 reads it). The nudge rides `deltas_for_event` (the D4b
gateway path — a gateway-emitted `ReviewSynced` publishes a Review delta). **The live producer (the GitHub
fetch) is D5b-2** — D5b-1's projection is mechanism-built + fixture-fed (the "projection built, live producer
next slice" deferral, the proj_session/4.x precedent).

## Files expected to touch
**New:** `daemon/src/projections/review.rs` (the `ReviewProjector`).
**Modified:** `shared/src/events.rs` (`ReviewSynced` + registry) · `shared/src/status.rs` (`ReviewState`) ·
`shared/src/projections.rs` (`ReviewRow`) · `shared/src/ipc.rs` (`ProjectionName::Review` + `ALL`) ·
`shared/src/lib.rs` (0.37.0) · `shared/tests/contract.rs` (snapshots + 3-way verify) ·
`daemon/src/eventstore/schema.rs` (MIGRATION_14 CREATE + `REBUILD_TABLES`) · `daemon/src/eventstore/migrations.rs`
(register + `SUPPORTED_USER_VERSION` 13→14) · `daemon/src/projections/mod.rs` (register the projector + the
`deltas_for_event` Review arm) · `daemon/src/ipc/methods.rs` (`read_review_typed` + the `get_projection(Review)`
route) · `daemon/tests/` (fold/serve/nudge/migration tests) · `daemon/tests/gateway_plan.rs` (exact-latest pin).

## RED test outline (Step 2)
1. **`test_review_synced_projects_to_proj_review`** — a synthetic `ReviewSynced` → the `proj_review` row
   (all fields; `submitted_at=None` for pending; rebuild-equivalent). Why: §7.2 / LESSONS §48 fold.
2. **`test_review_projector_rejects_unknown_state`** — an unbindable `state` wire value → `Decode` (the
   projector degrades + skips; never stored raw — the reject-unknown §5.1 precedent). Why: §5.1 binding.
3. **`test_read_review_typed_serves_review_row`** — the typed serve round-trips a Some-body + a None-body
   row, fail-closed preserved. Why: §7.2/§5.0 / LESSONS §37.
4. **`test_review_synced_publishes_review_delta`** — a gateway-emitted `ReviewSynced` (via the D4b
   fake-executor-through-the-gateway pattern) → a `ProjectionName::Review` `Upsert` delta (keyed by
   review_id). Why: LESSONS §51 (the delta-source↔projector agreement — the new arm).
5. **`test_migration_14_applies`** — `user_version >= 14` + `proj_review` exists (the FLOOR, LESSONS §50) +
   the exact-latest pin (`gateway_plan.rs`) bumps to **14**.
6. **the §2.5-seam snapshots** — `ReviewSynced` + `ReviewRow` + `ProjectionName` field/variant sets ==
   the checked-in snapshot, tagged `spec(§7.1/§7.2/§6.1)` + the **3-way verify** @0.37.0. Why: §5.0/§2.5-seam
   (the implementer authors these in this cycle).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW `ReviewSynced` event + `ReviewState` enum + `ReviewRow` + `ProjectionName::Review`.
  **CONTRACT 0.36.0 → 0.37.0.**
- **Orchestrator doc rows to write hot (Step 9):** the `ARCHITECTURE.md` Appendix-A — a NEW `ReviewRow` row
  + the `ReviewSynced` EventTypeRegistry row + the `ProjectionName` closed-set note (Review added) + the
  daemon/CLAUDE.md MVP-projections row + the EventTypeRegistry row + the cross-doc CONTRACT version. Atomic
  with the round (orchestrator writes; the implementer flags, does NOT touch these). **This also partially
  closes the D4b future-CONTRACT flag** (proj_review now HAS a subscribe-name; project/repository/
  integration_connection still don't).
- **§2.5-seam (shared-contract) model touched?** **YES** — `ReviewSynced`/`ReviewRow`/`ProjectionName`
  (§7.1/§7.2/§6.1, crossed by a §2.5 edge). The schema-snapshot tests (RED #6) are REQUIRED this cycle.

## Things to flag at Step 2.5
1. **`ReviewState` — frozen value enum vs raw String.** GitHub's review states are a known stable set. My
   default vote: **a frozen `ReviewState` value enum (reject-unknown, snake_case wire — the §5.1 precedent)**
   — matches the project's typed-status posture; a new GitHub state (unlikely) degrades the projector (the
   reject-unknown contract). Flag if you'd prefer a forward-compatible String (loses type-safety).
2. **`proj_review` row key — `review_id` alone vs the `{repo_id}#{pr_number}#{review_id}` composite.** GitHub
   review IDs are **globally unique** (a u64), so `review_id` alone is a valid PK. My default vote:
   **`review_id` (globally unique; simpler)** with `pr_number`/`repo_id` as FK columns — NOT the composite
   (the composite is only needed when the natural id isn't globally unique, like `pr_number`). Flag if you
   want the composite for proj_pull_request consistency.
3. **Keying / sibling-read.** `ReviewSynced` carries `review_id` + `pr_number`; the projector keys by
   `review_id`; `project_id` from the envelope; `repo_id` from the action's Repo resource_ref sibling-read
   (the `PullRequestProjector` precedent, LESSON §17) OR the event carries it. My default vote: **fold the
   self-contained event — `project_id` from the envelope; `repo_id` sibling-read like proj_pull_request**
   (LESSONS §48 — no sibling-read when self-contained, but proj_pull_request DOES sibling-read repo_id, so
   mirror it). Flag the exact source if cleaner to put repo_id in the payload.
4. **`body` §15 (the security-reviewer surface).** `ReviewSynced.body` is free-form USER review text → a
   higher-risk §15 surface than D5a's GitHub-generated mergeable/checks. My default vote: **include `body`
   (it's part of "structured reviews"), redacted at the event** — every event payload passes the §15
   Redactor before INSERT (the writer fail-closes on unredacted); the projector reads the PERSISTED
   (redacted) event → the row serves redacted body. **security-reviewer confirms** the `ReviewSynced.body`
   rides the redactor + no un-redacted path reaches the row (no NEW mechanism — the existing §15 gate; the
   review-body is just a free-form payload field). Flag if you'd DEFER `body` (project state/reviewer only)
   to avoid the surface — but the user wants structured reviews, and the redactor already gates it.

## Dependencies + sequencing
- **Depends on:** D5a (✅ `fa67c11` — the latest contract/migration baseline), P7.2 (✅ the typed-row + serve
  pattern), the edges merge (✅ the GitHub-integration baseline), D4b (✅ `deltas_for_event` the Review arm
  extends). All landed.
- **Blocks:** **D5b-2** (the GitHub fetch that emits `ReviewSynced` — the live producer) + the ui PR Review
  Workspace review display (§11.2). D5b-1's projection is fixture-fed until D5b-2.

## Estimated commit count
**1.** A coherent "new projection vertical freeze" (event + enum + row + ProjectionName + projection +
projector + serve + nudge + migration + CONTRACT bump) — the ②-mini/P7.2/D2 precedent (a whole projection
frozen in one slice). The contract surfaces are interdependent (the row needs the event + the enum + the
ProjectionName); splitting them creates a drift window. **Not a §15 safety pin, but security-reviewer runs
(opt-in)** for the `body` free-form-text surface.

## Lessons-logged candidates anticipated
- **Convention candidate** — a NEW projection vertical (event + ProjectionName variant + proj_table + typed
  row + serve + nudge) is the LESSONS §37/§48/§51 patterns composed; the ProjectionName closed-set addition is the
  CONTRACT cost of a subscribe-able projection (the D4b future-CONTRACT flag realized for reviews).
- **Architecture-doc note** — §7.1/§7.2/§6.1: the review vertical (ReviewSynced/proj_review/ReviewRow/
  ProjectionName::Review).
- **Future TODO** — D5b-2 (the GitHub fetch producer).

## How to invoke
1. **Read this brief end-to-end** (the 4 Step-2.5 Qs — esp. the `body` §15 surface — are load-bearing).
2. **Run `/tdd review_contract_projection`**.
3. **Step 0 (Restate)** — confirm the new review vertical + the D5b-1/D5b-2 split (synthetic events here).
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — answer the 4 design questions; the schema-snapshot tests are in this cycle.
6. **Step 9** — flag the cross-doc rows (Appendix-A + CLAUDE.md, the new vertical) + the CONTRACT bump + the
   security-reviewer result.

> **Step-8 reviewer policy:** `code-quality-reviewer` runs (`every-slice`). **`security-reviewer` runs
> (orchestrator opt-in)** — the `ReviewRow.body` is free-form user text; the review confirms `ReviewSynced.body`
> rides the §15 redactor + the row serves redacted (no new bypass). NON-cat-1.

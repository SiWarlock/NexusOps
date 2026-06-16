# /tdd brief — project_registry_projector

## Feature
An event-fed projector that folds the already-emitted `ProjectRescanned` event into a `proj_project` + `proj_repository` read model (MIGRATION_10 lays the two tables), closing the P5.1 read vertical (rescan executor → event → projection). **CONTRACT-neutral** (no new event / catalog / `ProjectionName` / shared-model surface — `ProjectRescanned` is frozen at 0.26).

## Use case + traceability
- **Task ID:** P5.1 (the MIGRATION-deferred registry projector — the executor + emission landed edges-019 `c739278`; the projector was the gated remainder)
- **Architecture sections it implements:** `ARCHITECTURE.md §7.2` (read models / projections) · `§9` (project detection) · `§15` (redaction — `remote_url` userinfo).
- **Related context:**
  - **The mechanism precedent is edges-022** (`daemon/src/projections/worktree.rs`, `WorktreeProjector`) — mirror it: a `Projector` impl, registered in `projections::projectors()`, in-band fold in the event-commit txn, rebuild-equivalent. **This projector is SIMPLER than edges-022** — `ProjectRescanned` carries its detection fields in the payload + identity (`project_id`) on the envelope, so there is **NO LESSON-17 sibling-read** (no `action_requests` resource_ref lookup; `project.rescan` is `requires_resource_refs=false`).
  - **Cross-doc resolution (lead-ruled, frozen in the contract):** `ProjectRescanned`'s doc-comment (`shared/src/events.rs:346`) says *"consumed by `proj_project_activity` + the project graph + a private `projects`/`repositories` registry (the projector splits this ONE coarse event into rows — lead-ruled coarse: a rescan is atomic)."* So for MVP these are **event-fed projections** (`proj_*`, rebuildable), NOT the DATA_MODEL 2.8 / 3 *durable-registry* direct-write rows (those carry `name`/`workspace_id`/`policy_json`/`created_at` that a rescan can't supply, and need a register-project mutator). The fuller durable-registry model is **DEFERRED** (a held arch-note + a carry-forward). This brief implements the event-fed projector path.
  - **Migration number:** **MIGRATION_10** (NOT 11). Wave-C `integration_connections` is HELD (user-gated, shared-contract Finding) and P5.1 ships first → it takes the next contiguous slot. Wave-C → MIGRATION_11 when it unblocks. (`SUPPORTED_USER_VERSION` is 9; main holds MIGRATION_9.)

## Acceptance criteria (what "done" means)
- [ ] **MIGRATION_10** (`schema::MIGRATION_10_PROJECT_REGISTRY`) creates `proj_project` + `proj_repository`; the `migrations::MIGRATIONS` array appends `M::up(MIGRATION_10_…)`; `SUPPORTED_USER_VERSION` 9 → 10. A fresh DB opens at `user_version=10`; an existing v9 DB migrates to v10 (the established backup-on-data path — no new migration logic, just the array entry + constant).
- [ ] A `ProjectRegistryProjector` folds `ProjectRescanned` (and ONLY that `event_type`) → a `proj_project` row keyed by `env.project_id` carrying the **project-identity** fields (`workflow_pack`, `cc_crew`, `plan_file`, `brain`, `scanned_at`) **+** a `proj_repository` row keyed by `env.project_id` (1:1 MVP) carrying the **git-detection** fields (`is_git`, `repo_root`, `remote_url`, `branch`, `detached`, `is_dirty`, `scanned_at`). Both rows carry `updated_at_seq = env.seq`.
- [ ] The projector is registered in `projectors()` (`mod.rs:78`) **and** both tables are in `schema::REBUILD_TABLES` → **rebuild-equivalence**: the incremental in-band fold produces a byte-identical result to a full `rebuild()` of the same log (LESSON 4 / LESSON 17).
- [ ] **Healthy SKIP** when `env.project_id` is `None` → no row written, no error (the `session.rs`/`worktree.rs` precedent — `proj_project.project_id` is `NOT NULL`).
- [ ] **`Decode` → degrade + skip** when the `ProjectRescanned` payload won't bind (reject-unknown); the degrade reason **never echoes payload bytes** (§15).
- [ ] A **re-rescan** (a 2nd `ProjectRescanned` for the same `project_id`) **UPSERTs** (`ON CONFLICT(project_id) DO UPDATE`) — the coarse-atomic rescan REPLACES the detection fields and advances `updated_at_seq`.
- [ ] **§15 (no new secret surface):** the projector reads `remote_url` from the **already-committed, already-redacted** event (userinfo was stripped at source in edges-019 + the redactor backstop) and writes it through — it does NOT re-handle a token and MUST NOT log the value. No new redaction surface.
- [ ] All unit tests in `daemon/tests/project_registry.rs` pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
`projections::projectors()` (`daemon/src/projections/mod.rs:78`) — add `Box::new(project_registry::ProjectRegistryProjector)`. Once registered it is invoked by **`apply_all`** in the in-band event-commit txn (the read-model fan-out) AND by **`rebuild`/`replay`**. The feeding event **`ProjectRescanned` is ALREADY emitted in production** by the `project.rescan` executor (edges-019, registered on the live `CatalogExecutor` in `main.rs`) → so a real `project.rescan` Action reaches this projector end-to-end the moment it is registered (`/wired ProjectRescanned` should show the live path: action → executor → in-txn append → `apply_all` → this projector).

**IPC read = DEFERRED (intentional, to stay CONTRACT-neutral).** Serving `proj_project`/`proj_repository` over IPC needs a new `ProjectionName::Project` variant — a `shared/` change that would bump CONTRACT. This slice does **NOT** add it. The projection is forward-laid + fold/rebuild-reachable; the read RPC is a consumer-gated follow-on (lands when a ui consumer needs it — the `proj_worktree`-before-its-read / Phase-2 `fault.rs` gated-but-fed precedent). At `/phase-exit 5` the reachability-auditor will see `proj_project` reachable-to-the-fold but NOT reachable-to-IPC → declare it **intentionally gated**, not drift.

## Files expected to touch
**New:**
- `daemon/src/projections/project_registry.rs` — the `ProjectRegistryProjector` (folds `ProjectRescanned` → `proj_project` + `proj_repository`).
- `daemon/tests/project_registry.rs` — the RED tests.

**Modified:**
- `daemon/src/eventstore/schema.rs` — `MIGRATION_10_PROJECT_REGISTRY` const (the two `CREATE TABLE`s) + add both names to `REBUILD_TABLES`.
- `daemon/src/eventstore/migrations.rs` — `SUPPORTED_USER_VERSION` 9 → 10 + append `M::up(schema::MIGRATION_10_PROJECT_REGISTRY)` to `MIGRATIONS`.
- `daemon/src/projections/mod.rs` — `mod project_registry;` + register the projector in `projectors()`.
- (If `MIGRATION_10_…` should be re-exported alongside `MIGRATION_1..3` in `eventstore/mod.rs` — match the existing pattern; likely not needed.)

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/project_registry.rs` (the `daemon/tests/projections.rs` / `gateway.rs` append-then-assert harness is the precedent — append a real `ProjectRescanned` envelope through the store, assert the rows):

1. **`test_project_rescanned_folds_proj_project`** — append a `ProjectRescanned` (envelope `project_id = proj_X`) → a `proj_project` row keyed `proj_X` with `workflow_pack`/`cc_crew`/`plan_file`/`brain`/`scanned_at` from the payload, `updated_at_seq = seq`.
   - Asserts: the identity-axis fields fold; `project_id` is the envelope's. Why: §7.2 read-model fold; the edges-022 pattern.
2. **`test_project_rescanned_folds_proj_repository`** — same event → a `proj_repository` row keyed `proj_X` with `is_git`/`repo_root`/`remote_url`/`branch`/`detached`/`is_dirty`/`scanned_at`.
   - Asserts: the git-detection-axis fields fold; `remote_url` is the (already-stripped) committed value. Why: §9 detection → §7.2 read model.
3. **`test_rescan_upserts_and_advances_seq`** — append a 2nd `ProjectRescanned` for `proj_X` with changed detection (e.g. `is_dirty` flips) → the rows are REPLACED (DO UPDATE), `updated_at_seq` advances to the new seq.
   - Asserts: 1 row per table (no dup), new field values, higher seq. Why: the lead-ruled "coarse atomic rescan" — a rescan replaces the detection snapshot.
4. **`test_missing_project_id_is_healthy_skip`** — a `ProjectRescanned` envelope with `project_id = None` → 0 rows in both tables, the append/fold SUCCEEDS (no error).
   - Asserts: healthy skip, not a degrade, not a fail-closed. Why: `session.rs`/`worktree.rs` precedent (`project_id` NOT NULL → non-projectable event skipped).
5. **`test_unbindable_payload_degrades`** — an envelope with `event_type = ProjectRescanned` but a payload that won't bind (e.g. a missing required field) → degrade + skip (no row); the recorded reason contains NO payload bytes.
   - Asserts: `Decode`-class degrade (contained), reason is generic. Why: §15 (no payload echo) + the reject-unknown contained-failure norm (`worktree.rs`).
6. **`test_rebuild_equivalence`** — fold a log incrementally, snapshot `proj_project`+`proj_repository`; `rebuild()`; assert byte-identical.
   - Asserts: incremental == rebuild for BOTH tables. Why: LESSON 4 / LESSON 17 — both tables MUST be in `REBUILD_TABLES`; the projection is event-derived (a registered consumer of the committed event), so a rebuild is deterministic.
7. **`test_migration_10_applies`** (may live in `daemon/tests/` eventstore coverage) — a fresh store opens at `user_version = 10`; `proj_project` + `proj_repository` exist; `SUPPORTED_USER_VERSION == 10`.
   - Asserts: the migration is wired + applied. Why: the forward-only migration discipline (LESSON 8).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **NONE.** No `shared/` contract touched — `ProjectRescanned` is frozen (0.26), no new event / enum / catalog action_type / `ProjectionName` variant / projection-row contract. **CONTRACT stays 0.32 — this slice is CONTRACT-neutral.**
- **Shared-contract seam touched?** **No** → no schema-snapshot test needed (this is a daemon-internal projector + private projection tables; `proj_*` projection tables are not a `shared/` contract — the `proj_worktree`/`proj_pull_request` precedent).
- **Orchestrator doc rows to write hot (HELD-for-merge PLAN-DELTA — edges does NOT edit the shared root docs in-worktree; accumulate in `docs/planning/edges-R5-wiring-plan.md`):**
  - **Arch-note (DATA_MODEL 3 SoT-rule + 2.8 reconciliation):** MVP `projects`/`repositories` are realized as **event-fed projections** (`proj_project`/`proj_repository`, folded from `ProjectRescanned`, in `REBUILD_TABLES`, rebuild-equivalent), NOT the DATA_MODEL 2.8 durable-registry direct-write rows. The full durable-registry model (canonical rows + a `register_project` mutator carrying `name`/`workspace_id`/`policy_json`/`created_at`) is **DEFERRED**. Lead-ruled at the R1b `ProjectRescanned` freeze.
  - **Completed-work tick:** P5.1 = executor (edges-019) + emission + **projector** → the P5.1 read vertical is CLOSED (minus the deferred IPC read RPC).
  - **MIGRATION_10** registered (`proj_project` + `proj_repository`); `SUPPORTED_USER_VERSION` 9→10; Wave-C → MIGRATION_11.
  - **Projections cross-doc row** (`daemon/CLAUDE.md` MVP-projections): `ProjectRescanned` now has its first projector consumer.

## Things to flag at Step 2.5
1. **Two tables (`proj_project` + `proj_repository`) vs one coarse `proj_project`.** A single coarse event with 1:1 project↔repo could fold into ONE table. My default vote: **two tables** — mirrors DATA_MODEL 2.8's `projects`/`repositories` structure + the lead/wiring-plan framing, so the eventual durable-registry migration is a natural evolution. Object if you think the 1:1 cardinality makes one table clearly better.
2. **`proj_repository` key — `project_id` (1:1 MVP) vs a deterministic `repo_` ULID.** `ProjectRescanned` carries no `repo_id`, and minting a fresh ULID per fold breaks rebuild-equivalence. My default vote: **`project_id` PK (1:1 MVP)** — per DATA_MODEL line 683 (`proj_worktree.repo_id` is 1:1 today; multi-repo = a `worktree_repos` join, a marked extension point). A `repo_` ULID + multi-repo is deferred.
3. **IPC read surface — add `ProjectionName::Project` now vs defer.** My default vote: **DEFER** — adding the variant bumps CONTRACT, which breaks the CONTRACT-neutral reorder rationale that put this slice first. The projection is forward-laid + fold/rebuild-reachable; the read RPC lands with its ui consumer. (Declare the IPC-unreachability as intentional gating at `/phase-exit 5`.)
4. **`scanned_at` vs `updated_at_seq`.** Store BOTH? My default vote: **yes** — `scanned_at` = the detection time (staleness UX), `updated_at_seq` = the event watermark (ordering), distinct axes (the `proj_worktree.git_checked_at` vs `updated_at_seq` precedent).

## Dependencies + sequencing
- **Depends on:** edges-019 (`project.rescan` executor + `ProjectRescanned` emission — LANDED, on main via the R8 merge) · the R8 merge (the migration infra + `SUPPORTED_USER_VERSION=9` baseline).
- **Blocks:** the deferred IPC read RPC (`ProjectionName::Project` + `get_projection`) · the deferred durable-registry fuller model (`register_project` mutator) · `/phase-exit 5` (this is the last P5.1 wiring remainder).
- **Sequencing:** ships FIRST in R9 (CONTRACT-neutral); Wave-C follows on the user's build-now-A vs defer-to-daemon ruling.

## Estimated commit count
**1.** The projector + MIGRATION_10 + tests are one logical, bisectable unit. **No safety invariant is touched** — a read-model projector with no mutator, no INV-SEC-1 surface, no new secret handling → **security-reviewer = SKIP** per the `invariant`-only Step-8 policy (the edges-022/edges-025 projectors were security-not-required for the same reason; the `remote_url` fold reads an already-stripped+redacted committed value — confirm at Step 2.5 it's not re-logged). `code-quality-reviewer` runs (every-slice policy).

## Lessons-logged candidates anticipated
- **Convention candidate** — "An event-fed *registry* projection (`proj_project`/`proj_repository`) is a projection, not the DATA_MODEL 2.8 durable registry: fold the coarse event, key by the envelope identity (no sibling-read when the payload is self-contained), put it in `REBUILD_TABLES`, defer the canonical-row/mutator model." (edges lesson candidate — renumber to 44+ at the edges→main merge; edges' 30–33 candidates now collide with daemon's merged-in lessons 30–43.)
- **Architecture-doc note candidate** — the DATA_MODEL 3 SoT-rule reconciliation (above): MVP projects/repositories are event-fed projections; the durable-registry classification is the deferred fuller model.
- **Future TODO — operational** — the IPC read RPC (`ProjectionName::Project`) + the durable-registry `register_project` mutator (CONTRACT-bumping) are consumer-gated follow-ons.

## How to invoke
1. Read this brief end-to-end (don't skip Step 2.5 — answer Q1–Q4 or take defaults).
2. Run `/tdd project_registry_projector`.
3. Step 0 (Restate) → confirm against the Feature line.
4. Step 2.5 → ping back with Q1–Q4 answers (or take defaults).
5. Step 9 → surface anything beyond the anticipated lessons-logged candidates.

# /tdd brief — proj_pull_request_projector

## Feature
The `proj_pull_request` projector (Wave-E) — folds `PullRequestSynced` (the edges-023 `github.create_pr`
emitter) into the `proj_pull_request` §7.2 read cache, **closing the github read vertical**
(mutator → event → projection → IPC). The exact edges-022 `proj_worktree` precedent.

## Use case + traceability
- **Task ID:** P7.1 (Wave-E close-out).
- **Architecture sections it implements:** `ARCHITECTURE.md §7.2` (the GitHub-authoritative `proj_pull_request`
  derived cache), `§7` (the projection fan-out) — within P7's phase scope.
- **Widens phase scope because** `§5.1` (the frozen 11-state `PullRequest` machine — the `status` column binds
  it via `wire_value`, no fork) is a cross-cutting invariant the projector respects; cited for traceability.
- **Related context:**
  - **edges-022 `projections/worktree.rs`** (`c666dc0`) — the EXACT precedent: folds a gateway-emitted event,
    `repo_id` via the LESSON-17 immutable sibling-read of `action_requests.resource_refs`, `status` bound via
    `wire_value` (the layer-correct producer — persistence-core must NOT import an edge module), the three-case
    failure taxonomy (healthy-skip on missing identity / fail-closed `Db` on a missing sibling row / `Decode`-degrade
    on unbindable data), live-read-cache columns left NULL + absent from the DO UPDATE set, rebuild-equivalent.
  - **edges-023** (`498bd21`) — the `PullRequestSynced{pr_number:u64, status: PullRequest, branch, base,
    mergeable, checks_summary, pr_checked_at}` emitter. NOTE: the payload has **no `title`**; `mergeable`/
    `checks_summary` already FED the derived `status` (`derive_pull_request_status`) — they are NOT projected.
  - **The DDL** (`eventstore/schema.rs:198`): `proj_pull_request(pr_id TEXT PK, project_id, repo_id, pr_number
    INTEGER, title, status TEXT NOT NULL, head_branch, base_branch, pr_checked_at, updated_at_seq INTEGER NOT
    NULL)`. In `REBUILD_TABLES` (schema.rs:19) → **rebuild truncates + replays it → the projector MUST be
    rebuild-deterministic** (the `pr_id` key especially — see Q1).

## Acceptance criteria (what "done" means)
- [ ] A `PullRequestProjector` (`projections/pull_request.rs`) folds ONLY `PullRequestSynced` (other event
      types → `Ok(())` no-op), registered in `projections/mod.rs` `projectors()` (after `WorktreeProjector`).
- [ ] **`pr_id` = a deterministic composite of `repo_id` + `pr_number`** (Q1) — rebuild-safe (NOT a minted
      ULID; `proj_pull_request` is in `REBUILD_TABLES` → a minted key would break rebuild-equivalence).
- [ ] Columns: `project_id` ← envelope; `repo_id` ← the LESSON-17 sibling-read of the Repository resource_ref;
      `pr_number`/`head_branch`(←`branch`)/`base_branch`(←`base`)/`pr_checked_at` ← payload; `status` ← payload
      `status` via `wire_value(&PullRequest)` (the §5.1 enum → its snake_case wire string); `title` ← NULL
      (the event has none); `updated_at_seq` ← `env.seq`. `mergeable`/`checks_summary` NOT projected (no column;
      already folded into `status`).
- [ ] **Three-case failure taxonomy (edges-022):** missing `project_id`/`action_request_id`/Repository ref →
      healthy SKIP (no-op); missing `action_requests` sibling row → fail-closed `Db` (the `?` propagates
      `QueryReturnedNoRows`); unbindable `resource_refs_json` or `PullRequestSynced` payload → `Decode`-degrade
      (the reason NEVER echoes payload bytes — §15).
- [ ] `ON CONFLICT(pr_id) DO UPDATE` (re-sync updates the row) — re-fold/rebuild idempotent.
- [ ] **Rebuild-equivalence:** a `rebuild()` reproduces a byte-identical `proj_pull_request` (the composite
      `pr_id` + the deterministic columns guarantee it).
- [ ] All tests pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
**Production entry point:** `projections/mod.rs` `projectors()` — the in-band fan-out applies every registered
projector in the event-commit txn. Adding `PullRequestProjector` there makes it fold on every `PullRequestSynced`
append (the edges-023 emit path → this projector, atomic). Reads served via `get_projection(PullRequest)` (the
IPC read surface, already live). **No main.rs change.** This CLOSES the github read vertical.

## Files expected to touch
**New:**
- `daemon/src/projections/pull_request.rs` — `PullRequestProjector`.
- `daemon/tests/` — projector tests (a new file or extend `tests/projections.rs` — Q2).

**Modified:**
- `daemon/src/projections/mod.rs` — register `PullRequestProjector` in `projectors()` + the `mod pull_request;`.

(No `shared/`/`gateway/`/`main.rs` edit; the DDL already exists.) If implementation needs files beyond this,
flag at Step 2.5.

## RED test outline (Step 2)
1. **`test_pull_request_synced_folds_to_proj`** — Asserts: a `PullRequestSynced` append → one `proj_pull_request`
   row with `pr_number`/`status`/`head_branch`/`base_branch`/`pr_checked_at` from the payload, `project_id` from
   the envelope, `repo_id` from the sibling resource_ref. Why: §7.2/§7 fold.
2. **`test_pr_id_composite_deterministic`** — Asserts: `pr_id` = the `{repo_id}#{pr_number}` composite (Q1);
   two folds of the same (repo, pr_number) hit the SAME row. Why: rebuild-safe key.
3. **`test_status_binds_pull_request_wire_value`** — Asserts: `status` column == `wire_value(&payload.status)`
   (the §5.1 PullRequest snake_case wire string). Why: §5.1 enum binding, layer-correct.
4. **`test_title_null_mergeable_checks_not_projected`** — Asserts: `title` is NULL; no mergeable/checks column
   written (they fed `status`). Why: the event has no title; mergeable/checks → status only.
5. **`test_missing_identity_healthy_skip`** — Asserts: no `project_id`/`action_request_id`/Repository ref →
   no row, no error (healthy skip). Why: the edges-022 taxonomy case 1.
6. **`test_missing_sibling_row_fail_closed`** — Asserts: link set but `action_requests` row gone →
   `Db` error (the `?` propagates). Why: LESSON-17 integrity break, case 2.
7. **`test_unbindable_payload_degrades`** — Asserts: a garbage `PullRequestSynced` payload → `Decode`-degrade
   (skip), reason echoes NO payload bytes (§15). Why: case 3.
8. **`test_on_conflict_updates_row`** — Asserts: a re-fold of the same `pr_id` UPDATEs (status/seq advance),
   one row. Why: re-sync idempotency.
9. **`test_proj_pull_request_rebuild_equivalent`** — Asserts: `rebuild()` reproduces a byte-identical
   `proj_pull_request` (the composite key + deterministic columns). Why: REBUILD_TABLES determinism.
10. **`test_other_event_types_noop`** — Asserts: a non-`PullRequestSynced` event → no `proj_pull_request` write.
    Why: the projector folds only its event.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes (shared/):** NONE — no contract change; the DDL already exists; CONTRACT held 0.26.0.
- **Orchestrator doc rows (held for the final merge — cross-track rule):** an arch note (the `proj_pull_request`
  projector LIVE; the github read vertical CLOSED — mutator→event→projection→IPC); a LESSON-17 generalization
  confirmation (the gateway-event sibling-read pattern reused, 3rd application: worktree → pull_request).
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **`pr_id` derivation (load-bearing — rebuild-safety).** Options: (a) composite `{repo_id}#{pr_number}`
   (deterministic, rebuild-safe, unique per repo+PR); (b) `pr_number` alone (REJECT — collides across repos);
   (c) a minted `pr_` ULID (REJECT — `proj_pull_request` is in `REBUILD_TABLES`; a minted key changes per
   replay → breaks rebuild-equivalence, the usage-ledger composite-key precedent / edges-022's `worktree_id`-
   from-payload reasoning). **My default vote: (a) composite `{repo_id}#{pr_number}`** — pin the exact
   delimiter; pinned by test #2 + #9. Confirm the delimiter (`#` vs `:`; avoid one that can appear in a ULID).
2. **Test file placement.** New `tests/pull_request_projection.rs` vs. extend `tests/projections.rs`. My
   default vote: **mirror edges-022** (wherever the `proj_worktree` projector tests live — co-locate the
   sibling-read + rebuild-equivalence harness). Minor.
3. **`repo_id` sibling-read reuse.** edges-022 has the LESSON-17 sibling-read inline; this is its 2nd use. My
   default vote: **mirror inline** (don't over-extract for 2 uses; extract only if a 3rd identical reader
   appears — though a tiny shared helper is acceptable if clean).

## Dependencies + sequencing
- **Depends on:** edges-023 (`498bd21`, the `PullRequestSynced` emitter); edges-022 (`c666dc0`, the projector
  precedent); the existing `proj_pull_request` DDL + `get_projection` read surface.
- **Blocks:** closes the github read vertical. Remaining phase-exit: the MIGRATION_9-deferred Wave-C
  `integration_connections` + P5.1 registry projector (D8-deferred to the final merge) · the §7.2/subscribe-delta/
  live-read hardening · P5.4 bench · cargo audit · `/phase-exit 5`+`7`.

## Estimated commit count
**1.** A focused read-model projector (the edges-022 precedent). NOT a mutator → `security-reviewer` per the
`invariant` policy is **not required** (no INV-SEC-1 mutation; a read-model fold) — `code-quality-reviewer`
(every-slice) only. (If the impl judges the §15 sibling-read / redaction surface warrants it, run security too.)

## Lessons-logged candidates anticipated
- **Convention candidate** — confirms LESSON 17 generalizes to a 3rd gateway-event projector (worktree →
  pull_request); the composite-rebuild-safe-key pattern (usage-ledger / worktree precedent).
- **Architecture-doc note candidate** — `proj_pull_request` LIVE; the github read vertical CLOSED.

## How to invoke
1. **Read this brief end-to-end** — Q1 (`pr_id` derivation) is the load-bearing call.
2. **Run `/tdd proj_pull_request_projector`.**
3. **Step 2.5** — test-design write-up + coverage map + answers. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 9** — categorized flags: the github-vertical-closed arch note, the LESSON-17 generalization.

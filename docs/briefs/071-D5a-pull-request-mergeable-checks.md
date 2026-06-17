# /tdd brief — pull_request_mergeable_checks (D5a)

## Feature
Enrich `proj_pull_request` + the typed `PullRequestRow` with **`mergeable: Option<bool>`** +
**`checks_summary: Option<String>`** — the **P7.2 "basic-now + SPREAD" enrichment**. The data is ALREADY
emitted in `PullRequestSynced.mergeable?`/`checks_summary?` (the edges GitHub executor); P7.2 froze the row
WITHOUT them (the projector folded neither). This slice adds the 2 columns + folds them + adds the 2 typed
row fields + serves them. **Additive CONTRACT bump 0.35.0 → 0.36.0.** **NON-cat-1.**

> **LOCKSTEP (read first).** `PullRequestRow` has `deny_unknown_fields` and is served via the fail-closed
> `read_pull_request_typed` (deserialize each `proj_pull_request` row → `PullRequestRow`). So the 2 columns,
> the projector fold, the 2 row fields, the typed-serve SELECT, and the CONTRACT bump **must land in ONE
> slice** — adding a column without the row field (or vice-versa) makes the typed serve fail closed on the
> mismatch. This is why it's a single atomic slice, not a column-now + row-later.

## Use case + traceability
- **Task ID:** D5a (the user's UI-unblock work order) — **P4.6** the rich-PR row-extension (the P7.2
  `PullRequestRow` enrichment spread).
- **Architecture sections it implements:** `ARCHITECTURE.md §7.2` (the `proj_pull_request` read model +
  the typed-serve SoT), **§11.2** (the PR Review Workspace the enriched row feeds), **§5.0** (the contract
  SoT + the §2.5-seam freeze of the extended `PullRequestRow`).
- **Widens phase scope because** it's a §7.2/§11.2 PR-vertical row-freeze + a §5.0 contract bump citing
  cross-cutting sections beyond Phase 4's primary §8/§17 anchors — standard for a UI-unblock contract-freeze
  slice (the 4.4/4.5/P7.2 precedent).
- **Related context:** P7.2 (`e748874`, brief 065) — froze `PullRequestRow` (9 BASIC fields) + the typed
  serve; its row doc-comment (`shared/src/projections.rs:46-48`) NAMES this spread ("a later slice adds the
  2 columns + folds them from `PullRequestSynced.mergeable?`/`checks_summary?` + the 2 row fields together").
  · `PullRequestSynced` (`shared/src/events.rs:473`; `mergeable: Option<bool>` `:480`, `checks_summary:
  Option<String>` `:481`). · the projector (`daemon/src/projections/pull_request.rs`; `pr_id =
  {repo_id}#{pr_number}` `:92`; the INSERT `:99`; mergeable/checks NOT projected today `:94-96`). · the
  typed serve `read_pull_request_typed` (`daemon/src/ipc/methods.rs:554`). · the migration list
  (`daemon/src/eventstore/migrations.rs:15` `SUPPORTED_USER_VERSION = 12`; `schema.rs:198` the
  `proj_pull_request` CREATE; `schema.rs:519` `MIGRATION_12`). · the Appendix-A `PullRequestRow` row
  (`ARCHITECTURE.md:613`, CONTRACT 0.34.0). · LESSONS §37 (typed projection-row freeze), §50 (per-migration
  floor test + the single exact-latest pin), §17 (fold-from-event rebuild-safe).

## Acceptance criteria (what "done" means)
- [ ] `proj_pull_request` gains `mergeable` (nullable) + `checks_summary` (nullable TEXT) columns via
      **MIGRATION_13 (ALTER-only; the historical CREATE untouched)**; `SUPPORTED_USER_VERSION` 12 → 13.
- [ ] The `PullRequestProjector` folds `PullRequestSynced.mergeable?`/`checks_summary?` into the 2 columns
      (in the INSERT AND the `ON CONFLICT DO UPDATE` set); `None` → NULL. Rebuild-equivalent (LESSON §17 —
      derive-from-event; `proj_pull_request` stays in `REBUILD_TABLES`).
- [ ] `PullRequestRow` (`shared/src/projections.rs`) gains `mergeable: Option<bool>` + `checks_summary:
      Option<String>` (`deny_unknown_fields`; optionals-as-null; field names match the ui provisional).
- [ ] `read_pull_request_typed` SELECTs + maps the 2 new columns into `PullRequestRow` (Some + None both
      round-trip, fail-closed preserved).
- [ ] **CONTRACT_VERSION 0.35.0 → 0.36.0** (`shared/src/lib.rs`); the `PullRequestRow` **schema-snapshot**
      updated (the new field-name set) + the **3-way verify** GREEN (Rust→schema→{zod,pydantic}).
- [ ] §15: no new secret surface — `mergeable` is a bool; `checks_summary` is GitHub checks text ALREADY
      §15-redacted on the `PullRequestSynced` event (the projector folds the redacted payload). Confirm at
      Step-2.5 (the P7.2 "§15 no-secrets by construction" extends — the redactor backstop holds at the event).
- [ ] All unit/integration tests pass; `/preflight` clean (incl. the 3-way verify).
- [ ] Cross-doc invariant updated atomic (orchestrator writes hot at the seal — see below).

## Wiring / entry point (Step 7.5)
The enriched row is served via the EXISTING `get_projection(PullRequest)` → `read_pull_request_typed`
(`daemon/src/ipc/methods.rs:554`) — the ui PR Review Workspace (§11.2) reads it. The fold rides the EXISTING
`PullRequestProjector` (fed by `PullRequestSynced`, emitted by the live GitHub executor). **Reachable by
construction** — no new entry point; the 2 fields flow through the already-wired read + fold.

## Files expected to touch
**Modified:**
- `shared/src/projections.rs` — `PullRequestRow` += `mergeable: Option<bool>` + `checks_summary: Option<String>`.
- `shared/src/lib.rs` — `CONTRACT_VERSION` 0.35.0 → 0.36.0.
- `shared/tests/contract.rs` — the `PullRequestRow` schema-snapshot (new field set) + the 3-way verify.
- `daemon/src/eventstore/schema.rs` — `MIGRATION_13` (ALTER `proj_pull_request` ADD `mergeable`, ADD
  `checks_summary`); the historical CREATE untouched.
- `daemon/src/eventstore/migrations.rs` — register MIGRATION_13 + `SUPPORTED_USER_VERSION` 12 → 13.
- `daemon/src/projections/pull_request.rs` — fold the 2 fields (INSERT + DO UPDATE).
- `daemon/src/ipc/methods.rs` — `read_pull_request_typed` SELECTs + maps the 2 columns.
- `daemon/tests/` — projector fold + rebuild-equivalence; the typed-serve round-trip; the MIGRATION_13 floor
  test; the exact-latest pin bump (`tests/gateway_plan.rs` → 13).

## RED test outline (Step 2)
1. **`test_pull_request_synced_projects_mergeable_and_checks`** — a `PullRequestSynced` with
   `mergeable=Some(true)` + `checks_summary=Some("3 passing")` → the `proj_pull_request` row carries both;
   `None`/`None` → NULL. Rebuild-equivalent (clock-flip / rebuild assertion).
   - Why: §7.2 — the projector folds the event's enrichment fields (LESSON §17 derive-from-event).
2. **`test_read_pull_request_typed_serves_mergeable_checks`** — the typed serve round-trips the 2 new
   fields (a Some row AND a None row), fail-closed preserved.
   - Why: §7.2/§5.0 — the typed serve includes the enriched fields without breaking the fail-closed contract.
3. **`test_migration_13_applies`** — `store.user_version() >= 13` + the `mergeable`/`checks_summary` columns
   exist (a RUNTIME read; the FLOOR, not exact-latest — LESSONS §50).
   - Why: the migration adds the columns for fresh + existing DBs (ALTER-only, CREATE untouched).
4. **the exact-latest pin** (`tests/gateway_plan.rs`) bumps to **13** (the single double-bump guard, LESSONS §50).
5. **`PullRequestRow` schema-snapshot** (the field-name set == the checked-in snapshot, now with `mergeable`
   + `checks_summary`), tagged **`spec(§7.2)`** + the **3-way verify** (Rust→schema→{zod,pydantic}).
   - Why: §5.0/§2.5-seam — `PullRequestRow` is a frozen shared/ contract; the snapshot + 3-way verify pin
     the additive extension (the implementer authors this in THIS cycle — Step 2.5 reviews it).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `PullRequestRow` += `mergeable: Option<bool>` + `checks_summary: Option<String>`.
  **CONTRACT 0.35.0 → 0.36.0.**
- **Orchestrator doc rows to write hot (Step 9 routing):** the **ARCHITECTURE.md Appendix-A `PullRequestRow`
  row** (`:613`) — update the "**`mergeable`/`checks_summary` NOT frozen** … basic-now + SPREAD" to
  "enriched (D5a, 0.36.0)"; the **daemon/CLAUDE.md MVP-projections [P7.2] note** — same. Atomic with the
  round (orchestrator writes; the implementer flags at Step 9, does NOT touch these files).
- **§2.5-seam (shared-contract) model touched?** **YES** — `PullRequestRow` (§7.2, crossed by a §2.5 edge).
  The schema-snapshot test (RED #5) is REQUIRED in this cycle.

## Things to flag at Step 2.5
1. **§15 — `checks_summary` redaction.** It's a GitHub checks string. My default vote: **no new §15
   surface** — the data is already §15-redacted on the `PullRequestSynced` event (the redactor gates the
   event at persist; the projector folds the redacted payload; the row serves redacted data — the P7.2
   "§15 no-secrets by construction" extends). NOT a security-reviewer trigger. Flag if you see a path where
   un-redacted checks text reaches the row (it shouldn't — the projector reads the persisted event).
2. **Field types — mirror the event exactly?** `mergeable: Option<bool>` + `checks_summary: Option<String>`
   match `PullRequestSynced`. My default vote: **mirror exactly** — keep `checks_summary` the raw string the
   event carries; a structured/enum checks type is a later enrichment (or D5b's review structure), not this
   slice. Flag if you'd normalize.
3. **MIGRATION_13 — ALTER-only, CREATE untouched.** The D2/MIGRATION_12 precedent + LESSONS §50: the migration
   ALTERs (adds the columns for fresh + existing DBs via the migration chain); the historical CREATE TABLE
   stays untouched. My default vote: **ALTER-only**. Confirm the fresh-DB path runs the chain (CREATE + all
   migrations incl. 13's ALTER) — the impl caught exactly this in D2.
4. **`read_pull_request_typed` SELECT.** Confirm the 2 columns are added to the SELECT + the row build
   atomically with the row fields (the LOCKSTEP — the fail-closed serve breaks on a column/field mismatch).
   My default vote: **add to both in the same slice** (already the plan; flag the exact SELECT shape if it's
   a `SELECT *`-style read vs explicit columns).

## Dependencies + sequencing
- **Depends on:** P7.2 (✅ `e748874` — the `PullRequestRow` freeze + `read_pull_request_typed`), the edges
  merge (✅ `proj_pull_request` + `PullRequestSynced.mergeable?`/`checks_summary?`). All landed.
- **Blocks:** the ui PR Review Workspace rich display (§11.2 — mergeable/checks badges). **D5b** (structured
  reviews — the per-review events + `proj_review`) is the next enrichment.

## Estimated commit count
**1.** Additive CONTRACT bump, single logical unit — the column + fold + row field + typed serve + bump are
LOCKSTEP (the fail-closed typed serve forces them atomic). **Not a §15 safety pin** (event-redacted data;
no mutation/auth surface) → security-reviewer not triggered; code-quality-reviewer per policy. The
schema-snapshot + 3-way verify are the contract gate (not a separate commit).

## Lessons-logged candidates anticipated
- **Convention candidate** — the "basic-now + SPREAD" projection-row enrichment lands as a LOCKSTEP slice
  (column + fold + frozen-row field + typed-serve + CONTRACT bump together) because the fail-closed typed
  serve can't tolerate a column/field mismatch (the P7.2→D5a realization of LESSONS §37).
- **Architecture-doc note** — §7.2 Appendix-A: `PullRequestRow` enriched with mergeable/checks (the SPREAD
  consumed).
- **Future TODO** — D5b (structured reviews: per-review events + `proj_review` + the typed row).

## How to invoke
1. **Read this brief end-to-end** (the LOCKSTEP note + Step-2.5 Q3 the migration are load-bearing).
2. **Run `/tdd pull_request_mergeable_checks`**.
3. **Step 0 (Restate)** — confirm the additive enrichment + the LOCKSTEP.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — answer the 4 design questions; the schema-snapshot test is in this cycle.
6. **Step 9** — flag the cross-doc rows (Appendix-A + CLAUDE.md) + the CONTRACT bump for me to write hot.

> **Step-8 reviewer policy:** `code-quality-reviewer` runs (`every-slice`). `security-reviewer` is **not**
> triggered — D5a touches no §15 invariant (event-redacted GitHub metadata; no mutation/auth/redaction-path
> change). NON-cat-1.

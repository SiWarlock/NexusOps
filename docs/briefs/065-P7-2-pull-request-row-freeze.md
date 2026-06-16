# /tdd brief — pull_request_row_freeze

## Feature
Freeze the typed **`PullRequestRow`** in `shared/` + serve it **TYPED** from `proj_pull_request` — the
②-mini/`ApprovalQueueRow` provisional→generated precedent (LESSONS §37) — so the ui **PR Review Workspace**
(§7.2/§11.2) consumes a typed contract, not loose JSON. The **BASIC** row only (the columns the
now-on-main edges-P7.1 `PullRequestProjector` actually folds: `pr_id`/`project_id`/`repo_id`/`pr_number`/
`title`/`status`/`head_branch`/`base_branch`/`pr_checked_at`). **`mergeable`/`checks_summary` are NOT
projected** (no column — they fed the derived `status` in the edges-023 executor) → **freeze basic +
SPREAD the enrichment** (a later slice adds the 2 columns + folds them + adds the 2 row fields together).
**CONTRACT 0.33.0 → 0.34.0.** **NON-cat-1** (a typed row freeze + typed serve — the ②-mini character
MINUS its persistence half, which the edges projector already did; no new mutation/persist/migration).

## Use case + traceability
- **Task ID:** P7.2 (the ui-③ PR-workspace row freeze — the ②-mini/`ApprovalQueueRow` [P4.0b-ui2] lineage;
  the 3rd cross-track ui-unblock projection-row freeze; the user-requested next slice ahead of 3.3c —
  gates the ui PR Review Workspace §11.2).
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (the `proj_pull_request` read
  model + the GitHub-authoritative PR cache), `§5.0` (the contract SoT + the §2.5-seam freeze of
  `PullRequestRow`), `§11.2` (the PR Review Workspace consumer), `§15` (no-unredacted-secret verify on
  the row fields), `§5.1` (the frozen `PullRequest`(11) status machine the row binds).
- **Widens phase scope because** this is a cross-track projection-row freeze citing cross-cutting sections
  (§5.0 the contract SoT, the §2.5 seam, §11.2 the ui consumer, §15) beyond a single phase's primary
  anchors — standard for a contract-freeze slice (the ②-mini/058 precedent declared the same).
- **Related context:** `daemon/src/projections/pull_request.rs` (the edges-025 `PullRequestProjector` —
  the columns it folds; **`mergeable`/`checks_summary` explicitly NOT projected**, lines 17-18 + 94-96);
  the `proj_pull_request` DDL (`daemon/src/eventstore/schema.rs:198` — `pr_id` PK, the nullable columns,
  `status TEXT NOT NULL`, the internal `updated_at_seq`); the FROZEN `ApprovalQueueRow`
  (`shared/src/projections.rs` — the EXACT pattern to mirror: `deny_unknown_fields` + `JsonSchema`,
  `status` binds the §5.1 enum, omit internal bookkeeping, optionals-as-null) + `read_approval_queue_typed`
  / the typed-serve dispatch (`daemon/src/ipc/methods.rs:471-485` + `:494`); the `PullRequest`(11) status
  enum (`shared/src/status.rs:82`); the `PullRequestSynced` event (`shared/src/events.rs:473` — it carries
  `mergeable: Option<bool>` + `checks_summary: Option<String>`, the SPREAD source); the ui provisional PR
  row (`git show track/ui:ui/src/contracts/provisional.ts` — match field names where aligned, the ②-mini
  pin #1); LESSONS §37 (the projection-row-freeze pattern) / §15 (the schemars freeze gotchas) / §17
  (sibling-read rebuild-safety — already satisfied by the shipped projector; nothing new here).

## Acceptance criteria (1 commit — CONTRACT 0.33.0 → 0.34.0)
- [ ] `shared/src/projections.rs` gains a frozen **`PullRequestRow`** (alongside `ApprovalQueueRow`)
  carrying the `proj_pull_request` wire columns: `pr_id`, `project_id`, `repo_id`, `pr_number`, `title`,
  **`status: PullRequest`** (the frozen §5.1 enum — reject-unknown, no loose status string),
  `head_branch`, `base_branch`, `pr_checked_at`. `#[serde(deny_unknown_fields)]`; optionals-as-null (no
  `skip_serializing_if` — LESSON §15 trap 3); **OMIT** the internal `updated_at_seq` (the `ApprovalQueueRow`
  `sort_key`/`updated_at_seq` precedent). **OMIT `mergeable`/`checks_summary`** (no column → SPREAD).
- [ ] Nullability per the resolution (Step-2.5 Q1 — default: match the DDL; `pr_id`/`status` non-Option).
- [ ] Registered in the schema bundle (`shared/src/schema.rs`); schema artifact regenerated;
  `CONTRACT_VERSION` 0.33.0 → 0.34.0.
- [ ] `get_projection(PullRequest)` serves the **typed** `PullRequestRow` via a new `read_pull_request_typed`
  (the `ApprovalQueue` typed-serve branch precedent, `methods.rs:474`): deserialize the DB row →
  `PullRequestRow` STRICTLY (reject-unknown) → serialize; **fails closed** (`InternalError`) on a row that
  doesn't deserialize (no loose JSON; the `read_approval_queue_typed` fail-closed discipline). Drops
  `updated_at_seq`.
- [ ] `shared/tests/contract.rs` snapshot test pins the `PullRequestRow` field-name set + `status:
  PullRequest`; **3-way verify GREEN @0.34.0**.
- [ ] A daemon test: `get_projection(PullRequest)` output deserializes strictly as `Vec<PullRequestRow>`
  + the fail-closed-on-bad-row path; `/preflight` clean.
- [ ] **§15 no-secrets verify:** the `PullRequestRow` fields carry only ULIDs (`pr_id`=`{repo_id}#{pr_number}`,
  `project_id`, `repo_id`), a PR number, branch names, a status enum, and a timestamp — **NO `remote_url` /
  token / auth field** (the `remote_url`-redaction pin is in `ProjectRescanned`/`proj_project`, NOT
  `proj_pull_request`); confirm no unredacted-secret surface. `title` is NULL now (the event carries none).

## Wiring / entry point (Step 7.5)
**Production-reachable.** `get_projection(PullRequest)` is the LIVE §6.1 read RPC the ui PR Review
Workspace calls; the `proj_pull_request` read model is fed by the now-on-main edges-P7.1
`PullRequestSynced`→`PullRequestProjector` path. `/wired` the `PullRequest` projection → `read_pull_request_typed`
→ the served `PullRequestRow`. **No deferred caller** — the projection already populates from the edges
merge; the freeze + typed serve light up the existing read path for the ui.

## Files expected to touch
**Modified:**
- `shared/src/projections.rs` — add the frozen `PullRequestRow`.
- `shared/src/{schema.rs,lib.rs}` — register `PullRequestRow`; `CONTRACT_VERSION` 0.34.0.
- `shared/contracts/schema/*` — regen.
- `shared/tests/contract.rs` — the `PullRequestRow` snapshot + 3-way verify @0.34.0.
- `daemon/src/ipc/methods.rs` — the `PullRequest` typed-serve branch + `read_pull_request_typed`.
- `daemon/tests/` — the typed-serve pin (strict deserialize + fail-closed).

**New:** none (`shared/src/projections.rs` exists from ②-mini).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN. **No migration / no
projector change** is expected (the basic columns are already projected by the edges merge) — if you find
one is needed, flag it (it would mean a data gap I missed).

## RED test outline (Step 2)
1. **`test_pull_request_row_frozen_shape`** (`shared/tests/contract.rs`) — Asserts: the `PullRequestRow`
   field-name set `{pr_id, project_id, repo_id, pr_number, title, status, head_branch, base_branch,
   pr_checked_at}` + `status: PullRequest`; **NO `mergeable`/`checks_summary`/`updated_at_seq`**. Why:
   §2.5-seam freeze (LESSONS §15/§37).
2. **`test_schema_artifact_matches_rust`** stays green + `CONTRACT_VERSION == "0.34.0"`. Why: §5.0 SoT;
   3-way verify @0.34.0.
3. **`test_get_projection_serves_typed_pull_request_row`** (daemon) — Asserts: `get_projection(PullRequest)`
   output deserializes strictly as `Vec<PullRequestRow>`. Why: the typed-serve pin (no loose JSON; the
   `read_approval_queue_typed` precedent).
4. **`test_pull_request_typed_serve_fails_closed`** (daemon) — Asserts: a `proj_pull_request` row that
   doesn't deserialize → `InternalError` (fail-closed, never a silent skip). Why: the typed-serve fail-closed
   discipline (LESSONS §37).
5. **`test_pull_request_row_status_binds_enum`** — Asserts: `status` deserializes as the frozen §5.1
   `PullRequest` enum (an unknown status string is rejected). Why: §5.1 binding (no loose status).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW frozen `PullRequestRow` (`shared/`); `CONTRACT_VERSION` 0.33.0→0.34.0.
  **§2.5-seam touched → YES** (the snapshot test). **NO daemon-internal schema change** (`proj_pull_request`
  already has the columns from the edges merge; no migration, no `SUPPORTED_USER_VERSION` bump).
- **Orchestrator doc rows to write hot (Step 9):** the Appendix-A projection-row entry for `PullRequestRow`
  (the 2nd frozen projection-row after `ApprovalQueueRow`) + the §7/§11.2 note (the PR Review Workspace
  consumes the typed row) + the `daemon/CLAUDE.md` MVP-projections cross-doc row + CONTRACT 0.34.0.
- **Cross-track:** the ui regenerates `PullRequestRow` from 0.34.0 (replaces its provisional) → unblocks
  the ui PR Review Workspace. **The `mergeable`/`checks_summary` enrichment is a SPREAD** (a later slice
  adds the 2 `proj_pull_request` columns + folds them from `PullRequestSynced.mergeable?`/`checks_summary?`
  + adds the 2 `PullRequestRow` fields together) → Carry-forward, `last-consumer-slice: a proj_pull_request
  enrichment slice / when the ui workspace needs mergeable/checks`.

## Things to flag at Step 2.5
1. **Row-field nullability — match the DDL (Option) vs the projector-guarantee (non-Option).** The DDL
   makes `project_id`/`repo_id`/`pr_number`/`title`/`head_branch`/`base_branch`/`pr_checked_at` nullable;
   the projector ALWAYS sets all but `title` (it healthy-skips the event if `project_id`/`repo_id` are
   absent). My default vote: **match the DDL nullability** (`Option` for those; `pr_id`/`status` non-Option)
   — a frozen row should reflect what the column CAN hold, and a NON-safety display read model is more
   robust tolerating a NULL than failing the whole typed serve closed on one unexpected NULL. _(Contrast
   `ApprovalQueueRow`, which chose non-Option for always-present fields BECAUSE it's the safety-critical
   approval path where fail-closed is desirable; `PullRequestRow` is a display read model.)_ Confirm.
2. **`mergeable`/`checks_summary` — confirmed NOT projected → basic freeze + SPREAD.** The edges-025
   projector folds neither (they fed the derived `status` in the edges-023 executor; no column). The
   `PullRequestSynced` EVENT carries both (`mergeable: Option<bool>` + `checks_summary: Option<String>`) —
   so the SOURCE has them, the PROJECTION doesn't. My default: **freeze basic now; SPREAD the enrichment**
   (the lead's "don't freeze a field with no source") — the future enrichment field names match the event
   (`mergeable: Option<bool>`, `checks_summary: Option<String>`). Confirm.
3. **ui provisional name reconcile.** Match the ui's provisional PR-row field names where aligned (the
   ②-mini pin #1). I'm on main; the provisional is on `track/ui` — `git show
   track/ui:ui/src/contracts/provisional.ts | grep -A10 -i pullrequest`. My default: use the
   `proj_pull_request` column names (snake_case) as canonical (the daemon row is the source; the ui
   regenerates from it) + verify against the provisional at authoring. **Watch the head/base naming** — the
   projection uses `head_branch`/`base_branch`; the EVENT uses `branch`/`base`; the row mirrors the PROJECTION
   (`head_branch`/`base_branch`). Confirm.
4. **`title` — keep as a frozen `Option` field (NULL now) vs omit until sourced.** The DDL HAS a `title`
   column (always NULL — the event carries none). My default: **KEEP `title: Option<String>`** (the column
   EXISTS + is served, just NULL — unlike `mergeable`/`checks_summary` which have NO column; freezing it
   now as `Option` avoids a re-freeze when a future slice populates it from the GitHub API, the ②-mini
   freeze-complete posture). Confirm (KEEP vs omit).

## Dependencies + sequencing
- **Depends on:** the edges-P7.1 merge (✅ `proj_pull_request` + `PullRequestSynced` on main @`95df2e0`),
  ②-mini (✅ `shared/src/projections.rs` + the typed-serve pattern @0.30.0), CONTRACT 0.33.0 (✅
  post-edges-merge → bumps to 0.34.0).
- **Blocks:** the ui **PR Review Workspace** (§11.2/§7.2) — the ui regenerates `PullRequestRow` + builds
  the workspace. Then → **3.3c** (the next daemon slice, the cat-1 Codex interception).

## Estimated commit count
**1.** A single typed-row freeze + typed serve — no migration, no projector change (the basic columns are
already projected by the edges merge; this is the C2-equivalent of ②-mini WITHOUT its C1 persistence half).
NON-cat-1; the §15 surface is no-secrets-by-construction (no new payload persisted — a Step-9 confirm only).
If the ui-provisional reconcile or the nullability resolution genuinely grows it, flag — but it's one
cohesive freeze.

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`:** the policy is `invariant`. This slice adds **no** mutation/persist path (the
  `proj_pull_request` data is already persisted + §15-redacted on the `PullRequestSynced` event by the
  edges projector); it freezes a read contract + typed serve. The §15 surface = verifying the row fields
  carry no unredacted secret (ULIDs/branches/status/timestamp; NO `remote_url`/token). My call: **YES, LIGHT**
  — confirm the no-secret-surface + the typed-serve fail-closed (the ②-mini ran security-reviewer on the
  read-contract freeze; this is lighter — no persist, no approval/auth path). **Not cat-1.**
- **`code-quality-reviewer`: YES** (every-slice).

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the 2nd frozen projection-row (`PullRequestRow`) extends the LESSONS §37
  pattern; the `proj_pull_request` read model is now typed-served.
- **Convention candidate** — a projection-row freeze can be **BASIC-now + SPREAD-the-enrichment** when the
  projection folds a SUBSET of the event's fields (Codex `mergeable`/`checks_summary` fed the derived
  `status`, not a column) — freeze what's SERVED, spread what's sourced-in-the-event-but-not-projected;
  don't freeze a field with no projection column.
- **Future TODO** — the `mergeable`/`checks_summary` enrichment SPREAD (the 2 projector columns + fold +
  the 2 row fields); the remaining projection-row freezes (SessionRow/ProjectActivityRow/AuditEventRow).

## How to invoke
1. Read this brief + skim `daemon/src/projections/pull_request.rs` (the folded columns — note
   mergeable/checks NOT projected) + `shared/src/projections.rs` (the `ApprovalQueueRow` pattern to mirror).
2. `git show track/ui:ui/src/contracts/provisional.ts | grep -A10 -i pullrequest` (the provisional names),
   then `/tdd pull_request_row_freeze`.
3. Step 0/1 → confirm the Feature + files (1 commit, NO migration). Step 2.5 → answer the 4 Qs (Q1
   nullability + Q2 the basic-freeze+SPREAD are load-bearing) + the coverage map.
4. Step 8 → `security-reviewer` (LIGHT — no-secrets verify + typed-serve fail-closed); `code-quality-reviewer`.
5. Step 9 → surface the cross-doc (`PullRequestRow` Appendix-A row + CONTRACT 0.34.0 + the mergeable/checks
   SPREAD) for orchestrator hot-routing.

# /tdd brief — session_recovered_fold_and_session_row_freeze (D2)

## Feature
The **survival fold**: fold the already-emitted `SessionRecovered` observation event into `proj_session`
(it is emitted by 4.1b-1's `recover_sessions_on_restart` but **no projector consumes it** — a dead event),
and **freeze the typed `SessionRow`** in `shared/` served typed from `get_projection("Session")` (the
②-mini/`ApprovalQueueRow`→P7.2/`PullRequestRow` precedent — today `SessionRow` is only a `follow`-comment
placeholder). Adds `resume_mode`/`replayed_event_count`/`recovered_at` columns to `proj_session` (the
§11.4 resumed-vs-replayed-vs-reattached recovery display) + the typed row. **CONTRACT 0.34.0 → 0.35.0**
(additive). **NON-cat-1** (event→projection fold + a typed-row freeze; no mutation/Gateway path).
Unblocks the ui per-session recovery indicator + the `RecoveryState` banner (the ui anticipates it —
`provisional.ts:147`).

## Use case + traceability
- **Task ID:** D2 (the user's UI-unblock work order) — **P4.4** the survival-recovery DISPLAY fold (the
  §8.1/§11.4 recovery-display consumer of 4.1b-1's `SessionRecovered`: fold → `proj_session` + the typed
  `SessionRow`).
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (the `proj_session` read model),
  **§8.1**/**§11.4** (the B2-strict survival ladder + the resumed-vs-replayed recovery UX the row feeds),
  **§5.0** (the contract SoT + the §2.5-seam freeze of `SessionRow`), **§5.1** (the frozen `Session` status
  enum the row binds).
- **Widens phase scope because** this is a cross-track projection-row freeze + a §8.1/§11.4 survival-display
  fold citing cross-cutting sections (§5.0 the contract SoT, the §2.5 seam, §11.4 the ui consumer) beyond a
  single phase's primary anchors — standard for a contract-freeze + projection-fold slice (the ②-mini/058 +
  P7.2/065 precedent).
- **Related context:** `daemon/src/projections/session.rs` (the `SessionProjector` — folds `SessionStarted`
  [1.2] + `SessionFailed` [4.2, the UPDATE arm to MIRROR]; **add a `SessionRecovered` arm**); the
  `proj_session` DDL (`daemon/src/eventstore/schema.rs:115` — ~22 columns; `status TEXT NOT NULL` §5.1;
  **add `resume_mode`/`replayed_event_count`/`recovered_at`**); the migration list
  (`daemon/src/eventstore/migrations.rs:15` `SUPPORTED_USER_VERSION = 11` + `MIGRATION_11…` →
  **MIGRATION_12**); the `SessionRecovered` event (`shared/src/events.rs:301` — `{mode: ResumeMode,
  replayed_event_count: u64, execution_profile_id?}`; `EVENT_TYPE = "SessionRecovered"` @ `:313`); the
  `ResumeMode`(4) enum (`shared/src/harness.rs:96`); the frozen `ApprovalQueueRow`/`PullRequestRow` +
  `read_*_typed` (`shared/src/projections.rs` + `daemon/src/ipc/methods.rs:471-485,494` — the EXACT
  freeze + typed-serve pattern to mirror); the ui provisional `SessionRow` (`provisional.ts:141` —
  `{session_id, status, title?, project_id?, resume_mode?}`; NOTE the ui's `title` = the daemon's
  `display_name`); LESSONS §17 (fold MUTABLE-from-event-type, rebuild-safe), §37 (the projection-row-freeze
  pattern).

## Acceptance criteria (CONTRACT 0.34.0 → 0.35.0)

**Commit 1 — the `SessionRecovered` fold + the migration (daemon-internal):**
- [ ] A `SessionRecovered` arm in `SessionProjector::apply` (MIRROR the `SessionFailed` arm): on
  `env.event_type == SessionRecovered::EVENT_TYPE`, parse the payload + `UPDATE proj_session SET
  resume_mode=?, replayed_event_count=?, recovered_at=?, updated_at_seq=? WHERE session_id=?`. The
  recovery fields are **derived from the EVENT TYPE/payload** (rebuild-safe, LESSON §17). A
  `SessionRecovered` for an unknown session = a healthy no-op (UPDATE affects 0 rows — never a degrade,
  the SessionFailed precedent). `resume_mode` binds `ResumeMode` via `wire_value` (the §5.1/§8.1 canonical
  wire string). `replayed_event_count` → `i64`; `recovered_at` = `env.occurred_at`.
- [ ] **MIGRATION_12** adds `resume_mode TEXT`, `replayed_event_count INTEGER`, `recovered_at TEXT` (all
  nullable — NULL for never-recovered sessions) to `proj_session`; `SUPPORTED_USER_VERSION` 11 → 12;
  registered in `migrations.rs`. **`ALTER TABLE ADD COLUMN`** (additive nullable; `proj_session` is in
  `REBUILD_TABLES` so a rebuild re-folds — confirm the mechanism at Step-2.5).
- [ ] Tests (`daemon/tests/projections.rs`): the fold writes `resume_mode`/`replayed_event_count`/
  `recovered_at`; rebuild-equivalence preserved (the fold re-derives identically); the unknown-session
  no-op; `resume_mode` binds the §8.1 `ResumeMode` (an unknown mode wire value → degrade, never raw).

**Commit 2 — freeze the typed `SessionRow` + typed serve (CONTRACT 0.35.0):**
- [ ] `shared/src/projections.rs` gains a frozen **`SessionRow`** (alongside `ApprovalQueueRow`/
  `PullRequestRow`): the user-meaningful `proj_session` wire columns + **`status: Session`** (the frozen
  §5.1 enum, reject-unknown) + **`resume_mode: Option<ResumeMode>`** + `replayed_event_count: Option<u64>`
  + `recovered_at: Option<String>`. `deny_unknown_fields`; optionals-as-null; OMIT internal bookkeeping
  (`updated_at_seq`). The exact field set = Step-2.5 Q1 (the ui-provisional-aligned set + the daemon-
  meaningful fields + the 3 recovery fields).
- [ ] Registered in the schema bundle (`shared/src/schema.rs`); regen; `CONTRACT_VERSION` 0.34.0 → 0.35.0.
- [ ] `get_projection("Session")`/`ProjectionName::Session` serves the **typed** `SessionRow` via a new
  `read_session_typed` (the `read_approval_queue_typed`/`read_pull_request_typed` precedent: deserialize
  each `proj_session` row STRICTLY → `SessionRow` → serialize; **fails closed** `InternalError` on a
  bad/extra-column row; drops `updated_at_seq`).
- [ ] `shared/tests/contract.rs` snapshot pins the `SessionRow` field set + `status: Session` +
  `resume_mode: Option<ResumeMode>`; **3-way verify GREEN @0.35.0**.
- [ ] A daemon test: `get_projection("Session")` deserializes strictly as `Vec<SessionRow>` (incl. a
  recovered row carrying `resume_mode`/`replayed_event_count`/`recovered_at`, + a never-recovered row with
  them None); `/preflight` clean.

## Wiring / entry point (Step 7.5)
**Production-reachable both commits.** C1: the fold runs in-band in the event-commit txn on every
`SessionRecovered` (already emitted by 4.1b-1's `recover_sessions_on_restart` — this lights up the dead
event). C2: `get_projection("Session")` is the LIVE §6.1 read RPC the ui per-session recovery indicator +
`RecoveryState` banner call. `/wired` the `SessionRecovered`→projector→column→typed `SessionRow` path. No
deferred caller.

## Files expected to touch
**Modified:**
- `daemon/src/projections/session.rs` — the `SessionRecovered` fold arm.
- `daemon/src/eventstore/schema.rs` — MIGRATION_12 (the 3 columns) + the `proj_session` CREATE updated.
- `daemon/src/eventstore/migrations.rs` — register MIGRATION_12; `SUPPORTED_USER_VERSION` 12.
- `shared/src/projections.rs` — the frozen `SessionRow`.
- `shared/src/{schema.rs,lib.rs}` — register `SessionRow`; `CONTRACT_VERSION` 0.35.0.
- `shared/contracts/schema/*` — regen.
- `shared/tests/contract.rs` + `daemon/tests/projections.rs` (+ a typed-serve test) — the pins.
- `daemon/src/ipc/methods.rs` — the `Session` typed-serve branch + `read_session_typed`.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
**Commit 1** (`daemon/tests/projections.rs`):
1. **`test_session_recovered_folds_recovery_fields`** — a `SessionRecovered` → `proj_session` carries
   `resume_mode`/`replayed_event_count`/`recovered_at`. Why: §7/§8.1/§11.4 — the recovery display source.
2. **`test_session_recovered_unknown_session_noop`** — a `SessionRecovered` for an absent session → 0-row
   UPDATE, no degrade. Why: the `SessionFailed` healthy-no-op precedent.
3. **`test_session_recovered_rebuild_equivalence`** — rebuild re-derives the recovery fields identically.
   Why: LESSON §17 (mutable-from-event-type, rebuild-safe).
4. **`test_resume_mode_binds_enum`** — `resume_mode` binds the §8.1 `ResumeMode`; an unknown wire value →
   degrade (never raw). Why: §5.1/§8.1 reject-unknown.

**Commit 2** (`shared/tests/contract.rs` + daemon):
5. **`test_session_row_frozen_shape`** — the `SessionRow` field set + `status: Session` + `resume_mode:
   Option<ResumeMode>` + the 2 recovery fields; OMIT `updated_at_seq`. Why: §2.5-seam (LESSONS §15/§37).
6. **`test_contract_version_0_35_0`** + `schema_artifact_matches_rust` green; 3-way verify @0.35.0. Why:
   §5.0 SoT.
7. **`test_get_projection_serves_typed_session_row`** (daemon) — `get_projection("Session")` deserializes
   strictly as `Vec<SessionRow>` (a recovered row + a never-recovered row [recovery fields None]). Why:
   the typed-serve pin (no loose JSON; the `read_approval_queue_typed` precedent).
8. **`test_session_typed_serve_fails_closed`** (daemon) — a corrupt `proj_session` row → `InternalError`.
   Why: the typed-serve fail-closed discipline (LESSONS §37).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW frozen `SessionRow` (`shared/`); new `proj_session` columns (daemon-internal,
  MIGRATION_12); `CONTRACT_VERSION` 0.34.0→0.35.0. **§2.5-seam touched → YES** (the snapshot test).
- **Orchestrator doc rows to write hot (Step 9):** the Appendix-A projection-row entry for `SessionRow` (the
  3rd frozen projection-row) + the EventTypeRegistry note (`SessionRecovered` now CONSUMED by `proj_session`)
  + the §8.1/§11.4 note + the `daemon/CLAUDE.md` MVP-projections row + CONTRACT 0.35.0.
- **Cross-track (→ ui):** the ui regenerates `SessionRow` from 0.35.0 (replaces its 5-field provisional) → the
  `RecoveryState` banner + per-session recovery indicator. **Naming reconcile:** the ui provisional uses
  `title`; the daemon column is `display_name` (Step-2.5 Q2 — flag the ui's `title`→`display_name` regen
  delta, the P7.2 `pr_number` precedent).

## Things to flag at Step 2.5
1. **The `SessionRow` field set — which `proj_session` columns are wire-contract?** Default vote: the
   ui-provisional-aligned core (`session_id`, `project_id?`, `status: Session`, `display_name?`) + the
   daemon-meaningful (`harness?`, `model?`, `execution_profile_id?`) + the 3 recovery fields (`resume_mode?`,
   `replayed_event_count?`, `recovered_at?`); **OMIT** the internal bookkeeping (`updated_at_seq`) + the
   not-yet-consumed columns (`token_usage_json`/`pending_approvals`/`worktree_id`/`linked_*` — freeze when a
   ui consumer needs them, the basic-now+SPREAD posture). Confirm the set.
2. **`display_name` vs the ui's `title`.** The ui provisional names it `title`; the daemon column is
   `display_name`. Default vote: **use the daemon `display_name`** (canonical; the ui maps `title`→
   `display_name` on regen — a cross-track note, the P7.2 `pr_number` precedent). Confirm (or match `title`).
3. **Does `SessionRecovered` change `status`?** Default vote: **NO** — the fold writes ONLY the recovery
   metadata (`resume_mode`/`replayed_event_count`/`recovered_at`); `status` is owned by `SessionStarted`/
   `SessionFailed` (a relaunch re-emits `SessionStarted`; a reattach keeps the live status). The recovery
   fields are the §11.4 banner source, orthogonal to `status`. Confirm.
4. **Migration mechanism.** Default vote: **`ALTER TABLE ADD COLUMN`** (additive nullable; `proj_session`
   re-folds on catch-up; the ②-mini MIGRATION_9 `ALTER` precedent) vs DROP+CREATE+offset-reset. Confirm no
   `REBUILD_TABLES` interaction issue.
5. **Recovery-field nullability.** Default vote: all 3 `Option` (NULL for never-recovered sessions) —
   `resume_mode: Option<ResumeMode>`, `replayed_event_count: Option<u64>`, `recovered_at: Option<String>`.
   Confirm.

## Dependencies + sequencing
- **Depends on:** 4.1b-1 (✅ `SessionRecovered` emitted by `recover_sessions_on_restart`), 4.1a (✅
  `ResumeMode` frozen @0.29.0), ②-mini/P7.2 (✅ `shared/src/projections.rs` + the typed-serve pattern),
  CONTRACT 0.34.0 (✅ post-P7.2 → bumps to 0.35.0).
- **Blocks:** the ui per-session recovery indicator + the `RecoveryState` banner (§11.4). **D3** (the Session
  live-delta — emits a nudge on `SessionRecovered`) consumes this fold. Then → D4 → 3.3c.

## Estimated commit count
**1–2.** Likely **2** (the ②-mini shape): (1) the `SessionRecovered` fold + MIGRATION_12 (daemon-internal
data flow); (2) the typed `SessionRow` freeze + typed serve (CONTRACT 0.35.0). Bundle to 1 if the fold +
freeze stay small + cohesive (NON-cat-1, no safety pin). Confirm at Step-2.5.

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`:** the policy is `invariant`. NON-cat-1; no mutation/§15-payload path (the
  `SessionRecovered` event is already §15-redacted; the fold reads it). The surface = the rebuild-safety of
  the new fold (LESSON §17 — mutable-from-event-type) + the typed-serve fail-closed + no-secrets (the row is
  ULIDs/enums/names/counts/timestamps; no `remote_url`/token). My call: **YES, LIGHT** (confirm rebuild-safety
  + no-secrets + fail-closed; the P7.2 precedent). Not cat-1.
- **`code-quality-reviewer`: YES** (every-slice).

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — `SessionRecovered` is now CONSUMED (the §8.1/§11.4 recovery fold);
  the 3rd frozen projection-row (`SessionRow`).
- **Convention candidate** — folding a previously-dead observation event into its projection + freezing the
  consuming row together (the LESSONS §17 mutable-from-event-type fold + the LESSONS §37 typed-serve), basic-now + SPREAD the
  not-yet-consumed columns.
- **Future TODO** — the remaining `proj_session` columns (token_usage/pending_approvals/worktree/linked_*)
  freeze when a ui consumer needs them; the remaining row freezes (ProjectActivityRow/AuditEventRow).

## How to invoke
1. Read this brief + skim `daemon/src/projections/session.rs` (the SessionFailed arm to mirror) +
   `shared/src/projections.rs` (the `ApprovalQueueRow`/`PullRequestRow` pattern) +
   `git show track/ui:ui/src/contracts/provisional.ts | grep -A8 SessionRow`.
2. **Run `/tdd session_recovered_fold_and_session_row_freeze`**.
3. Step 0/1 → confirm the Feature + files (1-2 commits). Step 2.5 → the 5 Qs (Q1 the field set + Q3 the
   status-unchanged are load-bearing) + the coverage map.
4. Step 8 → `security-reviewer` (LIGHT — rebuild-safety + no-secrets + fail-closed); `code-quality-reviewer`.
5. Step 9 → surface the cross-doc (`SessionRow` Appendix-A + CONTRACT 0.35.0 + the `SessionRecovered`-consumed
   note + the ui `title`/`display_name` reconcile) for orchestrator hot-routing.

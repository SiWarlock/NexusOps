# /tdd brief — session_live_delta (D3)

## Feature
Make a live subscriber re-read `proj_session` after a session **death** or **recovery**: extend
`deltas_for_append` (`daemon/src/runtime/writer.rs:720`) — which today publishes a Session-projection
`Upsert` delta only on `SessionStarted` — to also publish a **row-less Session `Upsert` nudge** on
`SessionFailed` (4.2) and `SessionRecovered` (D2/4.4), the other two events the `SessionProjector` folds
into `proj_session`. Pin the `deltas_for_append`↔`SessionProjector` folded-event-set **agreement** with a
guard test so a future `proj_session`-mutating event can't silently ship without its nudge. **CONTRACT-neutral**
(`ProjectionDelta` is frozen @0.11.0; no `shared/` change; adds NO event/mutation → INV-SEC-1 untouched).
**NON-cat-1.**

## Use case + traceability
- **Task ID:** D3 (the user's UI-unblock work order, slice 2 of 5) — **P4.5** the live-delta nudge
  (the §7/§7.2 `proj_session` read model + the §1.6d/§6.1 subscribe-SERVE push it rides; the consumer of
  4.2's `SessionFailed` fold + D2/4.4's `SessionRecovered` fold).
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (the `proj_session` read model +
  the projection-delta source), **§6.1** (the `GatewayPort` subscribe/`subscription_push` surface the delta
  rides — publish-after-commit), **§11.4** (the resumed-vs-replayed recovery + the Failed-session restart
  affordance UI the nudge keeps live).
- **Widens phase scope because** the delta-source feeds the §7 read model + the §6.1 subscribe push for a
  ui-track live-refresh consumer (§11.4) — cross-cutting beyond Phase 4's primary §8/§17 anchors, standard
  for a UI-unblock read-surface slice (the 4.0b-ui1/4.0b-ui2/4.4 precedent).
- **Related context:** `daemon/src/projections/session.rs` (the `SessionProjector` — its `apply` folds
  EXACTLY three event types into `proj_session`: `SessionStarted` [1.2], `SessionFailed` [4.2], and
  `SessionRecovered` [D2/4.4] — this three-element set is the authority for which events need a nudge);
  the existing delta source `deltas_for_append` (`writer.rs:720`, the `SessionStarted`-only arm) + its
  call site (`writer.rs:500`, inside the write-actor's `Command::Append` arm, **publish-after-commit**);
  the existing test `test_append_publishes_delta_after_commit` (`daemon/tests/runtime.rs:566`) + the
  `session_intent` / `test_intent` helpers (`runtime.rs:59`/`:86`) the new tests mirror; LESSON §9
  (publish-after-commit, never back-pressures the writer), §12 (subscribe-SERVE close-on-lag resync),
  §17 (derive a mutable fold from the EVENT TYPE), LESSONS §50 (a per-set "two lists must agree" guard).

## Acceptance criteria (what "done" means)
- [ ] A committed `SessionFailed` event carrying a `session_id` publishes a `ProjectionDelta{projection:
      Session, kind: Upsert, row: None, id: Some(session_id)}` on the write-actor broadcast.
- [ ] A committed `SessionRecovered` event carrying a `session_id` publishes the same row-less Session
      `Upsert` delta keyed by that `session_id` (the row IS mutated — `resume_mode`/`replayed_event_count`/
      `recovered_at` — even though `status` is unchanged, so the subscriber must re-read).
- [ ] A `SessionFailed` / `SessionRecovered` intent with `session_id == None` publishes **no** delta
      (healthy no-op, parity with the existing `SessionStarted` `if let Some(sid)` guard).
- [ ] The existing `SessionStarted` delta behavior is unchanged (the existing test still passes).
- [ ] Publish-after-commit is preserved: a rolled-back / §15-refused append publishes nothing (no
      regression of the existing test's second half).
- [ ] A guard test pins that **every** event type the `SessionProjector` folds (SessionStarted /
      SessionFailed / SessionRecovered) publishes a Session `Upsert` delta — with a comment binding the
      two lists ("extend BOTH `deltas_for_append` and `SessionProjector::apply` together").
- [ ] All tests in `daemon/tests/runtime.rs` pass.
- [ ] `/preflight` clean.
- [ ] CONTRACT-neutral — no `shared/` change, no `CONTRACT_VERSION` bump, no schema-snapshot churn.

## Wiring / entry point (Step 7.5)
`deltas_for_append` is called at `daemon/src/runtime/writer.rs:500` inside the write-actor's
`Command::Append` arm — the **publish-after-commit** path (`result.is_ok()` → `deltas.send(delta)`).
**Both** append handles route through `Command::Append`: `WriteHandle::append` (async, `writer.rs:167`)
**and** `WriteHandle::try_append_observation` (sync fire-and-forget, `writer.rs:189`). The production
emitters of these events already use them — 4.2's `SessionFailed` driver and 4.1b-1's
`recover_sessions_on_restart` `SessionRecovered` emitter both append via the write-actor — so adding the
two arms makes the nudge fire on the **real** production appends. **Reachable by construction; no separate
wiring slice** (the delta-source is the §7/§6.1 subscribe surface; the ui live-subscribe consumer is the
cross-track UI half).

## Files expected to touch
**Modified:**
- `daemon/src/runtime/writer.rs` — extend `deltas_for_append`: add a `SessionFailed`/`SessionRecovered`
  → Session `Upsert` (`row: None`, `id: Some(session_id)`) arm alongside the `SessionStarted` arm; add a
  cross-reference comment binding the event set to `SessionProjector::apply` (the LESSONS §50 two-lists discipline).
- `daemon/tests/runtime.rs` — new RED tests in the L4 delta-source block (+ small `session_failed_intent`
  / `session_recovered_intent` helpers mirroring `session_intent`).

If implementation needs files beyond this list (e.g. a refactor extracting a shared event-type const —
see Step-2.5 Q1), **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests to write in `daemon/tests/runtime.rs` (the `---- L4 — live subscribe delta-source ----` block,
mirroring `test_append_publishes_delta_after_commit`):

1. **`test_session_failed_publishes_delta_after_commit`** — a committed `SessionFailed` intent with a
   `session_id` publishes a Session `Upsert` delta keyed by that id.
   - Asserts: `delta.projection == Session`, `matches!(delta.kind, Upsert)`, `delta.id == Some(sid)`.
   - Why: §7 — `proj_session` is mutated by `SessionFailed` (4.2 fold, LESSON §17) → the §6.1 subscriber
     must be nudged to re-read or the §11.4 Failed-session card goes stale.

2. **`test_session_recovered_publishes_delta_after_commit`** — same for a committed `SessionRecovered`
   intent (row IS mutated even though `status` is unchanged).
   - Asserts: same three fields, keyed by the recovered session's id.
   - Why: §11.4 — without the nudge the resumed/replayed recovery banner never refreshes after restart
     recovery; the D2/4.4 fold mutates `resume_mode`/`replayed_event_count`/`recovered_at`.

3. **`test_session_event_without_session_id_publishes_no_delta`** — a `SessionFailed` (and/or
   `SessionRecovered`) intent with `session_id == None` publishes no delta.
   - Asserts: `rx.try_recv().is_err()` after a committed append (no delta queued).
   - Why: the delta carries the id (`row: None`); no id → nothing to nudge — parity with the
     `SessionStarted` `if let Some(sid)` guard (`writer.rs:723`).

4. **`test_proj_session_folded_events_each_publish_a_session_delta`** (the set-agreement guard) —
   iterate the three known `proj_session`-mutating event types and assert each, appended with a
   `session_id`, publishes a Session `Upsert` delta.
   - Asserts: for each of `["SessionStarted","SessionFailed","SessionRecovered"]`, a committed append
     publishes a Session `Upsert` delta keyed by the session id.
   - Why: LESSONS §50 (keep-two-lists-honest) generalized to delta-source↔projector — a future
     `proj_session`-folding event (e.g. a clean-terminal `SessionKilled`/`SessionCompleted`) added to the
     projector without a `deltas_for_append` arm is a silent stale-UI bug; the test + a comment ("extend
     BOTH lists together") catch it.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. `ProjectionDelta`/`DeltaKind`/`ProjectionName` are frozen @0.11.0; D3
  emits an existing-shape delta on additional event types — no new field, no wire change.
- **Orchestrator doc rows to write hot (Step 9 routing):** none required (daemon-internal delta-source,
  CONTRACT-neutral). At the seal the orchestrator MAY add a one-line note to the MVP-projections cross-doc
  row prose ("every `proj_session`-mutating event publishes a row-less Upsert nudge") — orchestrator
  territory, not a model row.
- **§2.5-seam (shared-contract) model touched?** No. No schema-snapshot test needed (no `shared/` field
  set changes).

## Things to flag at Step 2.5
1. **Drift-guard shape — a named-list guard test, or a shared event-type const?** The two lists
   (`SessionProjector::apply`'s folded arms + `deltas_for_append`'s arms) must agree. **(A)** a named-list
   guard test (test #4 above: lists the three event types, asserts each emits a delta, + a "extend both"
   comment) — touches only `writer.rs` + `runtime.rs`. **(B)** extract a shared
   `const SESSION_PROJECTED_EVENT_TYPES: &[&str]` consumed by BOTH the projector and the delta-source
   (programmatic single source of truth). My default vote: **(A)** — keeps D3 small + CONTRACT-neutral +
   off the projector; the projector's `if event_type == X` arms don't naturally iterate a slice, and the LESSONS §50
   precedent is a guard *test*, not a refactor. If you see a clean (B) with no projector churn, flag it.
2. **Scope — Session-only now, or fold in the other-4 projections?** D3 is **Session-only**; D4 (the next
   brief) generalizes the row-less nudge to `proj_project_activity`/`proj_pull_request`/`proj_audit_trail`/
   `proj_usage_ledger`. My default vote: **Session-only** — keep the slice atomic; D4 is queued.
3. **`SessionRecovered` with an unbindable payload (bad `ResumeMode`) — does the delta still fire?**
   `deltas_for_append` reads only `intent.event_type` + `intent.session_id`, never the payload, so the
   delta fires on a committed append regardless of payload-bind; the `SessionProjector` independently
   degrades-and-skips a bad payload (the row may not change). My default vote: **delta fires on append
   (payload-agnostic)** — an over-nudge is safe (the lag-resync policy: the subscriber re-reads an
   unchanged row, harmless), an under-nudge is the bug. Do NOT couple the delta-source to the payload
   schema. Flag if you disagree.

## Dependencies + sequencing
- **Depends on:** 4.2 (✅ `SessionFailed` fold, `463abc0`), 4.4/D2 (✅ `SessionRecovered` fold, `7fc8bc7`),
  1.6d (✅ the subscribe-SERVE push + the `deltas_for_append` delta-source). All landed.
- **Blocks:** D4 (the other-4 projection deltas — same delta-source↔projector pattern) + the ui
  live-refresh of the Failed/recovery session cards (the whole-cockpit go-live).

## Estimated commit count
**1.** CONTRACT-neutral, single code area (`writer.rs` + its test), one logical unit. **No safety-critical
pin** — D3 adds no mutation, no event type, no redaction surface; it broadcasts an existing-shape delta
after commit (`broadcast::send` never blocks → forbidden #3 already satisfied). Not a §15 slice.

## Lessons-logged candidates anticipated
- **Convention candidate** — "`deltas_for_append` and the `SessionProjector`'s folded-event set are two
  lists that must agree — a row-mutating event without a delta nudge is a silent stale-UI bug; pin the
  agreement with a guard test (LESSONS §50 keep-two-lists-honest, generalized to delta-source↔projector)."
- **Architecture-doc note candidate** — §7/§6.1: every `proj_session`-mutating event publishes a row-less
  Upsert nudge; the subscriber re-reads the full row via `get_projection` (row enrichment deferred per the
  lag-resync policy).
- **Future TODO — operational** — D4 generalizes the nudge to the other-4 projections; D3 is the
  Session-only precedent.

## How to invoke
1. **Read this brief end-to-end** (don't skip "Things to flag at Step 2.5").
2. **Run `/tdd session_live_delta`** in the implementer session.
3. **Step 0 (Restate)** — confirm against the Feature line.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — answer the 3 design questions (or take defaults) before GREEN.
6. **Step 9** — surface anything outside the anticipated lessons-logged candidates.

> **Step-8 reviewer policy:** `code-quality-reviewer` runs (`every-slice`). `security-reviewer` is
> **not** triggered — D3 touches no §15 invariant (no mutation / event / redaction / auth surface; it
> publishes an existing-shape delta after commit). NON-cat-1.

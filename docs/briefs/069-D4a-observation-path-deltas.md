# /tdd brief — observation_path_deltas (D4a)

## Feature
Generalize the D3 row-less `Upsert` nudge to the two **observation-path** projections so live subscribers
re-read when their rows change: extend `deltas_for_append` (`daemon/src/runtime/writer.rs`) with **(1)** a
`proj_usage_ledger` nudge on `TelemetrySampled` (the only event the `UsageProjector` folds; observation-
appended via `try_append_observation`), and **(2)** a `proj_audit_trail` **blanket** nudge — the
`AuditProjector` folds EVERY event into an audit row, so every `Command::Append` event publishes an
AuditTrail `Upsert`. Both nudges are `id: None` (payload-agnostic — the subscriber re-reads the
aggregate/paged projection via `get_projection`). Pin the delta-source↔projector agreement (LESSONS §51).
**CONTRACT-neutral**, **NON-cat-1**.

> **Scope note (read first):** D4 was split into **D4a (this slice — the observation path,
> `deltas_for_append`)** and **D4b (the gateway-accumulator path, LESSON §17)** because the other-4
> projections fold a MIX of append paths and the bundle/atomize rule says don't bundle cross-area work.
> `proj_pull_request` (PullRequestSynced) + `proj_project_activity` (SessionStarted) are gateway
> `emitted_events` → **D4b**. **`proj_audit_trail` is cross-path** (it folds every event, from both paths):
> this slice wires its **observation half**; **D4b wires its gateway half** (an AuditTrail delta alongside
> the existing ApprovalQueue push). AuditTrail is fully live only after D4b — both co-land this round
> before the ui consumes.

## Use case + traceability
- **Task ID:** D4a (the user's UI-unblock work order, slice 3 of 5+) — **P4.5** the observation-path
  live-delta nudges (the §7/§7.2 read models + the §6.1 subscribe-SERVE push).
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (the `proj_usage_ledger` +
  `proj_audit_trail` read models + the projection-delta source), **§6.1** (the subscribe/`subscription_push`
  surface the delta rides — publish-after-commit), **§11** (the frontend usage + audit views the nudge
  keeps live).
- **Widens phase scope because** the delta-source feeds the §7 read models + the §6.1 subscribe push for
  ui-track live-refresh consumers (§11) — cross-cutting beyond Phase 4's primary §8/§17 anchors, standard
  for a UI-unblock read-surface slice (the D3/4.0b-ui1/4.0b-ui2 precedent).
- **Related context:** D3 (`019a4b1`, brief 068) — the same pattern for `proj_session`; this slice mirrors
  it for the two observation-fed projections. `daemon/src/projections/usage.rs` (the `UsageProjector` folds
  ONLY `TelemetrySampled::EVENT_TYPE`, usage.rs:30) · `daemon/src/projections/audit.rs` (the `AuditProjector`
  folds EVERY event — no early-return filter; audit.rs:26) · the post-D3 `deltas_for_append`
  (`daemon/src/runtime/writer.rs`, the Session arm) + its call site (the write-actor `Command::Append` arm,
  publish-after-commit) · the telemetry sink `WriteActorTelemetrySink::emit_telemetry` →
  `try_append_observation` (`daemon/src/runtime/telemetry_sink.rs`, LESSONS §35) · the existing test
  `test_append_publishes_delta_after_commit` (`daemon/tests/runtime.rs`) — **NOTE it may need updating**
  (see Acceptance + Step-2.5 Q3) · LESSONS §9 (publish-after-commit), §51 (the delta-source↔projector
  two-list agreement, established in D3).

## Acceptance criteria (what "done" means)
- [ ] A committed `TelemetrySampled` event publishes a `ProjectionDelta{projection: UsageLedger, kind:
      Upsert, row: None, id: None}` on the write-actor broadcast.
- [ ] EVERY committed event publishes a `ProjectionDelta{projection: AuditTrail, kind: Upsert, row: None,
      id: None}` (the blanket arm — the audit projector folds every event into a row).
- [ ] A single event that mutates MULTIPLE projections publishes a delta for EACH: e.g. `TelemetrySampled`
      → {UsageLedger, AuditTrail}; `SessionStarted` → {Session, AuditTrail}; `SessionFailed` → {Session,
      AuditTrail} (the `Vec<ProjectionDelta>` carries all of them).
- [ ] Publish-after-commit preserved: a rolled-back / §15-refused append publishes NONE of these deltas
      (the existing `result.is_ok()` guard already gates the whole `Vec`).
- [ ] The D3 Session nudges are unchanged (still fire on SessionStarted/SessionFailed/SessionRecovered).
- [ ] The existing `test_append_publishes_delta_after_commit` stays GREEN — updated if needed to assert the
      Session delta is **among** the published deltas (drain-and-find), since each event now publishes the
      AuditTrail delta too (see Step-2.5 Q3).
- [ ] A LESSONS §51 guard test pins that `proj_usage_ledger`'s folded set (`{TelemetrySampled}`) is covered by the
      delta-source, keyed on `TelemetrySampled::EVENT_TYPE`, with the "extend BOTH lists together" comment.
- [ ] All tests in `daemon/tests/runtime.rs` pass; `/preflight` clean.
- [ ] CONTRACT-neutral — no `shared/` change, no `CONTRACT_VERSION` bump.

## Wiring / entry point (Step 7.5)
`deltas_for_append` at the write-actor's `Command::Append` arm (`daemon/src/runtime/writer.rs`,
publish-after-commit). `TelemetrySampled` reaches it via `WriteActorTelemetrySink::emit_telemetry` →
`WriteHandle::try_append_observation` → `Command::Append` (LESSONS §35); every other observation event
(SessionStarted-direct, SessionFailed, SessionRecovered, AuditIntegrityViolation, SensitiveOutputRedacted,
…) also flows through `Command::Append`, so the AuditTrail blanket arm fires on each. **Reachable by
construction; no separate wiring slice.** (Gateway `emitted_events` — PullRequestSynced, the gateway
Action* family — do NOT pass through `Command::Append`; their nudges are D4b.)

## Files expected to touch
**Modified:**
- `daemon/src/runtime/writer.rs` — extend `deltas_for_append`: a `TelemetrySampled::EVENT_TYPE` →
  UsageLedger `Upsert` (id: None) arm + an unconditional AuditTrail `Upsert` (id: None) push (every event);
  extend the cross-reference comment to name the UsageLedger↔`UsageProjector` agreement + the AuditTrail
  "folds every event" rationale + the D4b gateway-half pointer.
- `daemon/tests/runtime.rs` — new RED tests + update `test_append_publishes_delta_after_commit` to
  drain-and-find if needed.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/runtime.rs` (the L4 delta-source block, mirroring D3):

1. **`test_telemetry_sampled_publishes_usage_ledger_delta`** — a committed `TelemetrySampled` intent
   publishes a UsageLedger `Upsert` delta (`row: None`, `id: None`) among the published deltas.
   - Asserts: a delta with `projection == UsageLedger`, `matches!(kind, Upsert)`, `id.is_none()` is found.
   - Why: §7 — `proj_usage_ledger` is mutated by `TelemetrySampled` → the §6.1/§11 usage view must re-read.

2. **`test_every_event_publishes_an_audit_trail_delta`** — several distinct committed event types
   (e.g. SessionStarted, SessionFailed, TelemetrySampled, AuditIntegrityViolation) EACH publish an
   AuditTrail `Upsert` delta (`row: None`, `id: None`).
   - Asserts: for each, an AuditTrail Upsert delta is among the published deltas.
   - Why: §7 — the `AuditProjector` folds EVERY event into an audit row; the audit view nudges per event.

3. **`test_event_publishes_multiple_projection_deltas`** — a `TelemetrySampled` append publishes BOTH a
   UsageLedger AND an AuditTrail delta; a `SessionFailed` append publishes BOTH a Session AND an AuditTrail
   delta.
   - Asserts: both expected deltas are present in the drained set.
   - Why: a single event mutating N projections must nudge all N (the `Vec<ProjectionDelta>` contract).

4. **`test_proj_usage_ledger_folded_events_match_delta_source`** (the LESSONS §51 guard) — `proj_usage_ledger`
   folds exactly `{TelemetrySampled::EVENT_TYPE}`; assert a committed TelemetrySampled publishes a
   UsageLedger delta (keyed on the const).
   - Asserts: the UsageLedger nudge fires on `TelemetrySampled::EVENT_TYPE`.
   - Why: LESSONS §51 (keep-two-lists-honest) — a future `proj_usage_ledger`-folding event added to the
     projector without a delta-source arm is a silent stale-UI bug (extend BOTH lists; comment binds them).
     _(AuditTrail needs no per-event "set" guard — its arm is unconditional/blanket; the cross-path "every
     event nudges AuditTrail" invariant completes + is pinned in D4b, where its gateway half lands.)_

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. `ProjectionDelta`/`ProjectionName` frozen @0.11.0; `UsageLedger`/`AuditTrail`
  are existing `ProjectionName` variants. No new field, no wire change.
- **Orchestrator doc rows to write hot (Step 9 routing):** none required (daemon-internal delta-source,
  CONTRACT-neutral). The MVP-projections cross-doc row may gain a one-line `[D4a]` AS-BUILT note —
  orchestrator territory, at the seal.
- **§2.5-seam (shared-contract) model touched?** No. No schema-snapshot test.

## Things to flag at Step 2.5
1. **AuditTrail nudge granularity — blanket (every event), `id: None`?** The audit trail is the complete
   event LOG; the user's work order says "nudge when rows change," and an audit row changes on EVERY event.
   My default vote: **blanket arm, `id: None`** — push an AuditTrail Upsert unconditionally; the subscriber
   re-reads the paged audit projection. `broadcast::send` is fire-and-forget and never back-pressures the
   writer (LESSONS §9), so the per-event nudge frequency is safe. (`id: None` because the audit row's key —
   seq/event_id — is assigned at append time, NOT available on the pre-append `AppendIntent`; and a paged
   view re-reads regardless.) Flag if you think the audit view should poll/page instead of live-nudge.
2. **UsageLedger nudge id — `id: None` (payload-agnostic) vs parse for `ledger_id`?** The `ledger_id` lives
   in the `TelemetrySampled` payload, not the `AppendIntent` fields. My default vote: **`id: None`,
   payload-agnostic** — preserve D3's property (`deltas_for_append` reads only envelope/identity fields,
   never the payload); the ui re-reads the whole usage projection (a small aggregate). Parsing the payload
   to key the nudge would couple the delta-source to the `TelemetrySampled` schema for marginal benefit.
3. **The existing `test_append_publishes_delta_after_commit` — drain-and-find vs push-order.** Once the
   AuditTrail blanket arm lands, a `SessionStarted` append publishes TWO deltas (Session + AuditTrail), so
   the existing test's single `rx.recv()` may see either first depending on push order. My default vote:
   **update the existing test (and write the new ones) to assert the expected delta is AMONG the drained
   deltas (drain-and-find), not the first/only one** — robust against multi-projection-per-event ordering;
   don't rely on push order. (Equivalent alternative: order projection-specific pushes before the AuditTrail
   blanket push so the existing recv-once test still gets Session first — but drain-and-find is the durable
   pattern as more projections nudge per event.)

## Dependencies + sequencing
- **Depends on:** D3 (✅ `019a4b1` — the `deltas_for_append` Session pattern + the LESSONS §51 guard pattern this
  mirrors), 1.6d (✅ subscribe-SERVE push), 4.0c (✅ the `TelemetrySampled` production emit path), the audit
  + usage projectors (✅ landed). All landed.
- **Blocks:** the ui live-refresh of the usage + audit views. **D4b** completes AuditTrail (the gateway
  half) + adds PullRequest/ProjectActivity — co-lands this round.

## Estimated commit count
**1.** CONTRACT-neutral, single code area (`writer.rs` + its test), one logical unit (two observation-path
arms sharing the `deltas_for_append` context). **No safety-critical pin** — adds no mutation/event/redaction;
broadcasts existing-shape deltas after commit (`broadcast::send` never blocks → forbidden #3 satisfied).
Not a §15 slice → security-reviewer not triggered; code-quality-reviewer per policy.

## Lessons-logged candidates anticipated
- **Convention candidate (likely already covered by LESSONS §51)** — the blanket-arm pattern for a
  fold-every-event projection (AuditTrail) — an unconditional nudge needs no per-event set guard, unlike a
  selective projection (UsageLedger). If this nuance is worth recording, extend LESSONS §51 rather than a new lesson.
- **Architecture-doc note candidate** — §7/§6.1: the observation-path projections (usage/audit) now
  publish row-less Upsert nudges (`id: None`, subscriber re-reads the aggregate). Orchestrator AS-BUILT note.
- **Future TODO — operational** — D4b (gateway path: PullRequest + ProjectActivity + the AuditTrail gateway
  half).

## How to invoke
1. **Read this brief end-to-end** (don't skip "Things to flag at Step 2.5" — esp. Q3 on the existing test).
2. **Run `/tdd observation_path_deltas`** in the implementer session.
3. **Step 0 (Restate)** — confirm against the Feature line.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — answer the 3 design questions (or take defaults) before GREEN.
6. **Step 9** — surface anything outside the anticipated lessons-logged candidates.

> **Step-8 reviewer policy:** `code-quality-reviewer` runs (`every-slice`). `security-reviewer` is **not**
> triggered — D4a touches no §15 invariant (no mutation/event/redaction/auth; it publishes existing-shape
> deltas after commit). NON-cat-1.

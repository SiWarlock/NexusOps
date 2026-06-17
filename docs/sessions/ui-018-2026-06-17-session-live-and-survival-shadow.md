# ui-018 — Session live (refetch-on-nudge) + SessionRow recovery-shadow drift-pin (ui-062)

- **Date:** 2026-06-17
- **Phase:** Phase 6/7 (whole-cockpit-live + survival) — **P6.8 / §6.1 + §11.4** (the live `ProjectionDelta` transport spread to Session + the survival/recovery display data foundation)
- **Predecessor:** [ui-017](ui-017-2026-06-16-pr-review-consumer-reconcile.md)
- **Successor:** _(none yet)_
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`

## Why this session existed

Two Session-overlapping arcs (whole-cockpit-live + survival) merged into one cohesive slice:
1. **The D3 UI half.** The Session subscribe effect applied row-deltas via `applySessionDelta`, which **no-ops on the daemon's `row:None` nudge** (it was Mock-validated only — LESSON §29). So on the real daemon the Session list only refreshed on reconnect, never on a live SessionStarted/Failed/Recovered. Switching it to the proven ui-059 **refetch-on-nudge** makes Session live on the real daemon.
2. **The survival real-data foundation.** The `SessionRow` provisional shadow was a non-strict 5-field subset (`title`, optional `project_id`). Reconciling it to the frozen 10-field shape `.strict()`-drift-pinned types the recovery fields (`resume_mode`/`replayed_event_count`/`recovered_at`) so the per-session resume-mode indicator (`resumeModesBySessionId`, already reading `SessionRow.resume_mode`) shows real modes.

## What was built (1 atomic commit — `b04feb1`)

### Files deleted

- `ui/src/gateway-client/delta-reducer.ts` + `.test.ts` — `applySessionDelta` was the Mock-only row-apply reducer the refetch-on-nudge supersedes (its sole consumer was the Session effect; LESSON §29 names it the `row:None` footgun). Orchestrator-approved delete (scope #1 (a)) — removes the hazard of someone re-wiring it. `sessionDeltaFixture` kept (the mock subscribe dogfood; the nudge ignores its row).

### Files modified — production

- `ui/src/shell/Shell.tsx` — the Session subscribe effect rewired to the ui-059 refetch-on-nudge (a `createNudgeCoalescer` + `refetchSessions` + `onDelta → coalescer.nudge()` + the refetch-snapshot), mirroring the ApprovalQueue effect below it; `applySessionDelta` import dropped.
- `ui/src/contracts/provisional.ts` — `SessionRow` shadow reconciled **5→10** `.strict()`: `display_name` (was `title`), `project_id` non-Option (was optional), + `harness`/`model`/`execution_profile_id`/`resume_mode` (now `.nullable()` per the frozen `anyOf[ResumeMode,null]`)/`replayed_event_count` (u64)/`recovered_at`. Drift-pinned to `$defs.SessionRow` (the ApprovalQueueRow §37 precedent).
- `ui/src/projections/fixtures/proj_session.ts` — fixture reshaped (title→display_name; recovery fields populated on the recovered rows; `sessionDeltaFixture` row reshaped + comment updated to the nudge model).
- **Forced consumer-reconciles** (title→display_name + project_id non-Option + resume_mode-null): `projections/items.ts` (`toSessionItems`) · `overlays/HumanInputQueue.tsx` · `shell/Sidebar.tsx` · `views/terminal/SessionTerminal.tsx` · `views/command/CommandCenter.tsx` (×2) · `views/command/SessionRowCard.tsx` · `views/sessions/model.ts` (`resume_mode ?? undefined` null-coerce) · `recovery/model.ts` (`resumeModesBySessionId` guard `!== undefined` → `!= null`).

### Files modified — tests

- `ui/src/contracts/provisional.test.ts` — the SessionRow field-set pin (RED-first, 5≠10) + the recovery-fields/uint/strict/state-delegate pin.
- `ui/src/shell/Shell.subscribe.test.tsx` — the old row-apply Session test REPLACED with `session_subscribe_refetches_on_row_none_nudge` (a `GatedSessionRefetchGateway` mirroring the ApprovalQueue gated-refetch pin); the stale file-header comment updated.
- `ui/src/projections/items.test.ts` · `shell/Sidebar.test.tsx` · `recovery/model.test.ts` (+ a null-`resume_mode`-excluded row) · `views/command/CommandCenter.test.tsx` · `views/sessions/model.test.ts` (dropped a now-type-impossible absent-`project_id` row) — title→display_name + project_id reconciles.

## Decisions made

1. **Session subscribe → refetch-on-nudge** (mirror ApprovalQueue exactly) — the daemon's `row:None` nudge demands a coalesced re-read, never a row-apply (LESSON §29). The per-stream `notifyConnectionState("Session", …)` authority was already in place.
2. **Delete `applySessionDelta`/`delta-reducer`** (scope #1 (a), orchestrator-ruled) — dead after the rewire + the LESSON §29 known-wrong-for-real-daemon footgun.
3. **Replace the old Session subscribe test** (scope #2) — it asserted the now-removed row-apply; the replacement pins the corrected refetch-on-nudge behavior (not dropped coverage).
4. **SessionRow full-10 `.strict()` drift-pin** (Q2) — the row now has a real consumer (the resume-mode indicator), so freeze the full served shape; `harness`/`model`/`execution_profile_id` are shadowed-with even though not yet rendered (the drift-pin needs the full set). `.nullable().optional()` per the established ApprovalQueueRow/PullRequestRow/ReviewRow convention (daemon-tolerant read).
5. **`resumeModesBySessionId` guard `!= null`** — the frozen SessionRow serializes an absent `resume_mode` as **explicit null** (not omit), so the old `!== undefined` would wrongly enter a fresh session with a null mode (and break tsc); `!= null` excludes both. **The resume-MAP guard, NOT the deferred banner-derivation.**
6. **`project_id` non-Option** — the frozen row's `project_id` is required; a downstream "absent project_id" test case became type-impossible and was dropped (the `NO_PROJECT="—"` branch stays as defensive code for an undefined-session edge).

## Decisions explicitly NOT made (deferred)

- **The daemon-wide RecoveryState banner real-data derivation** — DEFERRED as a **daemon-gap Finding**: `recovering` is a transient daemon-startup state (not projection-observable); `recovery_failed` has no projection marker; recovery currently always lands `relaunched`. The banner stays fixture-fed; the actionable banner needs a **daemon recovery-status signal** (a daemon ask / work-order). `recovery/model.ts` banner logic + `RecoveryBanner.tsx` were NOT touched (only the resume-MAP guard was).
- **ui-063** — the other-projections refetch-on-nudge spread (ProjectActivity/PullRequest/UsageLedger, the same pattern). Not this slice.

## TDD compliance

**Clean on the shadow; honest note on the subscribe rewire.** The SessionRow field-set pin was authored + confirmed **RED-first** (5≠10) before the shadow reconcile. The `session_subscribe_refetches_on_row_none_nudge` behavioral pin was authored **alongside** the Shell-effect rewire (the GatedSessionRefetchGateway + the rewire landed together) rather than confirmed-RED against the old `applySessionDelta` effect in isolation — a minor deviation from strict red-first (the brief framed it as RED against the old no-op; I verified the corrected behavior green rather than the old behavior red). The forced-consumer reconciles + the reviewer-driven comment fix were mechanical/test-after (tsc-forced shape changes; the deterministic logic — the shadow + the resume-MAP guard + toSessionItems — is pinned). No safety-critical surface (NON-cat-1, read-only).

## Reachability

- **The Session refetch-on-nudge** — reachable from the live production path: `Shell.tsx` mounts the Session subscribe effect (`runSubscriptionSupervisor` → `gateway_subscribe` → the daemon's Session deltas); the rewire corrects its `onDelta`/`refetch` handling (the ApprovalQueue precedent, already reachable + tested).
- **The SessionRow shadow** — consumed on the same `get_projection("Session")` → `boundary.ts` parse path; `resumeModesBySessionId` is wired at `Shell.tsx:381`.
- The deleted `applySessionDelta` removed a dead path. The banner-derivation is NOT added (deferred). No tested-but-unwired gap.

## Open follow-ups

Step-9 categorized list (routed hot to the orchestrator; it writes the doc rows at `/orchestrate-end` — listed for continuity, NOT for me to re-route):

- **[Cross-doc invariant change — orchestrator writes hot]** `ui/CLAUDE.md`: the SessionRow provisional shadow 5→10 `.strict()` drift-pin (recovery fields now consumed) + the Session live-delta is now refetch-on-nudge (the D3 UI half; LESSON §29 generalized to Session). No new generated value-set.
- **[Future TODO — ui-063]** the other-projections refetch-on-nudge spread (ProjectActivity/PullRequest/UsageLedger), same pattern.
- **[Future TODO — daemon ask / Finding]** the RecoveryState banner real-data derivation needs a **daemon recovery-status signal** (recovering/recovery_failed aren't projection-derivable; recovery always lands `relaunched`) — a work-order ask; the banner stays fixture-fed.
- **[Verify-completeness note]** my Step-2.5 forced-consumer sweep under-reported the set (grep too narrow) — tsc surfaced the `views/command/*`, `SessionTerminal`, `views/sessions/model`, and the `recovery/model.ts` resume-MAP-guard consumers. All folded in-scope per the approved scope-(a); a reminder to widen the SessionRow-consumer grep (`.title`/`.resume_mode`/`.project_id`) next time.
- **[Architecture doc note]** none — the ui implements no new §; the daemon owns the contract.

## How to use what was built

The cockpit's Session list is now live on the real daemon — a SessionStarted/Failed/Recovered nudges the stream → a coalesced `get_projection("Session")` re-read → the list + the derived switcher counts update. To consume the recovery surface, read the typed `SessionRow` (`resume_mode`/`replayed_event_count`/`recovered_at` now typed); `resumeModesBySessionId(sessions)` returns the id→ResumeMode indicator map (null/undefined modes excluded). The shadow is drift-pinned to `shared/src/projections.rs` — a daemon field change fails `provisional.test.ts` loudly. The other projections adopt the same refetch-on-nudge in ui-063.

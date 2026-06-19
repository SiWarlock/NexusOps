# /tdd brief — session_live_and_survival_shadow (whole-cockpit-live + survival, the Session slice)

## Feature
Make the cockpit's **Session list live on the REAL daemon** + reconcile the **SessionRow shadow to the frozen 10-field shape** (which lights up the real per-session resume-mode indicators). Two Session-overlapping arcs merged into one cohesive slice: (1) the **D3 UI half** — the Session subscribe currently applies row-deltas via `applySessionDelta`, which **no-ops on the daemon's `row:None` nudge** (Shell.tsx:227-276 + delta-reducer.ts:26-27 — Mock-validated only), so the real Session list only refreshes on reconnect; switch it to the proven ui-059 **refetch-on-nudge** pattern. (2) the **survival real-data foundation** — the SessionRow provisional shadow is a non-strict 5-field subset; reconcile to the frozen 10 fields `.strict()`-drift-pinned, so the recovery fields (resume_mode/replayed_event_count/recovered_at) are typed + the per-session resume-mode indicator (`resumeModesBySessionId`, already reading `SessionRow.resume_mode`) shows real modes. **NON-cat-1, read-only.**

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport — the **live-delta spread to the REST**, now UNGATED by daemon D3/D4; the new tracker checkbox, origin ui-062)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the live `ProjectionDelta` transport — refetch-on-nudge), `§11` (the cockpit display), `§11.4` (the survival/recovery display surface), `§5.1` (SessionRow status binding)
- **Related context:** the ui-059 ApprovalQueue refetch-on-nudge template (Shell.tsx:278-336, `ui/src/gateway-client/refetch-on-nudge.ts`, `recountFrom`; `ui/LESSONS.md §29` — the `row:None` id-nudge → coalesced refetch, NOT a row-apply reducer); the frozen `SessionRow` (10 fields, `shared/src/projections.rs:85-96`; D2 added resume_mode/replayed_event_count/recovered_at @ 0.35); the daemon Session delta emission (D3, `daemon/src/projections/mod.rs` `deltas_for_event` — Session nudges on SessionStarted/Failed/Recovered); the existing recovery model (`ui/src/recovery/model.ts` — `resumeModesBySessionId` reads `SessionRow.resume_mode`; the banner is fixture-fed); the frozen-shadow drift-pin precedent = ApprovalQueueRow/PullRequestRow (`provisional.test.ts`); the ui-061 forced-consumer-reconcile precedent (a shape rename breaks downstream consumers → fold in-scope, tsc-green).

## Acceptance criteria (what "done" means)
- [ ] **Session refetch-on-nudge** — the Session subscribe effect (Shell.tsx:227-276) switches from `applySessionDelta` (the `row:None` no-op) to the ui-059 pattern: `onDelta → coalescer.nudge()`; `refetch → get_projection("Session")` → `recountFrom`. Mirror the ApprovalQueue 2nd-effect exactly (same coalescer, same `notifyConnectionState("Session", …)` per-stream authority — already in place). The Session list now updates on the real daemon's `row:None` nudges (SessionStarted/Failed/Recovered), not just on reconnect.
- [ ] **SessionRow shadow 5→10 `.strict()` drift-pin** — extend `ui/src/contracts/provisional.ts` `SessionRow` to the frozen 10 fields: `session_id`/`status` (non-Option, kept), `project_id` **non-Option** (was optional — fix), `display_name` (**renamed from `title`**), `harness`/`model`/`execution_profile_id` `.nullable().optional()`, `resume_mode` (kept), `replayed_event_count` (NEW, the u64 shadow `.nullable().optional()`), `recovered_at` (NEW, `.string().nullable().optional()`). `.strict()` per the frozen `deny_unknown_fields`; drift-pinned to `$defs.SessionRow` (the ApprovalQueueRow precedent, `LESSONS.md` §37). _(Completes the SessionRow freeze the D2 triage anticipated — the row now has a real consumer.)_
- [ ] **Forced consumer-reconciles (tsc-green, fold in-scope per the ui-061 scope-(a) precedent):** `toSessionItems` (items.ts:34) `s.title` → `s.display_name`; + any other `SessionRow.title`/`project_id`-optionality consumer the rename/non-Option breaks (**verify the full set first** — grep `SessionRow` + `.title` usages; flag at Step 2.5).
- [ ] The per-session **resume-mode indicator shows REAL data** — `resumeModesBySessionId` already reads `SessionRow.resume_mode`; confirm via a test that real-shaped Session rows (resume_mode set) produce the indicator map (currently all `relaunched` from the daemon until the broker 4.1b-2; the wiring is correct + future-proof).
- [ ] The full ui suite stays green (the ~379 + the new tests), `tsc --noEmit` + `oxlint` clean, `/preflight` clean.
- [ ] Cross-doc flagged at Step 9 (orchestrator writes the `ui/CLAUDE.md` row hot: SessionRow shadow 5→10 `.strict()` drift-pin; the Session live-delta now refetch-on-nudge). **Implementer does NOT edit `ui/CLAUDE.md`.**

## Deferred (out of this slice — flagged, not built)
- **The daemon-wide RecoveryState banner real-data derivation** — DEFERRED (a daemon-gap, surfaced as a Finding): `recovering` is a transient daemon-startup state (not projection-observable); `recovery_failed` has no projection marker; recovery currently always lands `relaunched`. The banner stays fixture-fed this slice (the realizable `recovered` is non-intrusive/invisible anyway). The actionable banner needs a **daemon recovery-status signal** (a daemon ask, work-order). Do NOT build the banner derivation here.

## Wiring / entry point (Step 7.5)
The Session subscribe effect is the live production path (Shell.tsx mounts it; `runSubscriptionSupervisor` → `gateway_subscribe` → the daemon's Session deltas). The change rewires `onDelta`/`refetch` (the ApprovalQueue precedent — already reachable + tested). The SessionRow shadow is consumed on the same path (`get_projection("Session")` → `boundary.ts` parse → the refetch). The resume-mode indicator (`resumeModesBySessionId`) is already wired at Shell render (line 377). No NEW reachable symbol beyond the rewired effect; the survival banner-derivation is NOT added (deferred). `/wired` target: the Session subscribe effect (already wired; this corrects its delta handling).

## Files expected to touch
**Modified:** `ui/src/shell/Shell.tsx` (Session effect → refetch-on-nudge) · `ui/src/shell/Shell.subscribe.test.tsx` (the Session `row:None`→refetch pin, mirror the ApprovalQueue pin) · `ui/src/contracts/provisional.ts` (SessionRow 5→10 `.strict()`) · `ui/src/contracts/provisional.test.ts` (the SessionRow drift-pin) · `ui/src/projections/items.ts` + `items.test.ts` (toSessionItems title→display_name) · any other `SessionRow.title`/`project_id` consumer found at verify.
**Not touched:** `recovery/model.ts` + `RecoveryBanner.tsx` (the banner-derivation is deferred — `resumeModesBySessionId` already reads resume_mode, no change) · `generated.ts` (no contract regen — SessionRow is a provisional shadow, not generated) · `ui/CLAUDE.md` (orchestrator territory).

## RED test outline (Step 2)
1. **`session_subscribe_refetches_on_row_none_nudge`** (Shell.subscribe.test.tsx, mirror the ApprovalQueue `row:None` pin #1) — a daemon-shaped `row:None` Session delta → the supervisor refetches `get_projection("Session")` (NOT a row-apply that no-ops). **RED:** current `applySessionDelta` no-ops on `row:None` → no refetch, the new row never appears. [§6.1/`LESSONS.md` §29]
2. **`session_row_field_set_matches_frozen_schema`** (provisional.test.ts, mirror ApprovalQueueRow) — the shadow field-set === `$defs.SessionRow` (10); extra field FAILS `.strict()`; `project_id` required (non-Option); `display_name` present (not `title`). **RED:** current shadow = 5 fields, `project_id` optional, `title` not `display_name`. [§5.1/§11.4]
3. **`session_row_recovery_fields_and_uint`** — `replayed_event_count` numeric (rejects string), `recovered_at` string, `resume_mode` delegates to the generated/shadow ResumeMode. **RED:** fields absent. [§11.4]
4. **`to_session_items_uses_display_name`** (items.test.ts update) — label = `s.display_name ?? s.session_id`. **RED:** current reads `s.title`. [§11.3]
5. **`resume_modes_by_session_id_from_real_shape`** — real-shaped SessionRow[] (resume_mode set) → the indicator map; absent resume_mode → no entry. (May already be green — confirm it holds against the 10-field shape.) [§11.4]

> Confirm the RED is the refetch + shadow/mapper drift; a broader RED (another `SessionRow.title`/`project_id` consumer) is the Step-2.5 forced-consumer set (fold in-scope, ui-061 precedent).

## Cross-doc invariant impact
- **Model field changes:** none in `shared/` (consuming the daemon-frozen 0.35 SessionRow). The ui SessionRow shadow grows 5→10 `.strict()` — a drift-pinned frozen-shadow, not ui-authored contract.
- **Orchestrator doc row (Step 9, I write hot):** `ui/CLAUDE.md` — SessionRow provisional shadow 5→10 `.strict()` drift-pin (the recovery fields now consumed) + the Session live-delta is now refetch-on-nudge (the D3 UI half; `LESSONS.md` §29 generalized to Session). No new generated value-set.
- **2.5-seam:** the SessionRow shadow mirrors a `shared/` 2.5-seam projection row — pinned by the field-set drift test.

## Things to flag at Step 2.5
1. **The full `SessionRow.title`→`display_name` + `project_id`-non-Option forced-consumer set** — verify (grep `SessionRow`/`.title`) + fold in-scope (tsc-green, the ui-061 precedent). My default: toSessionItems is the known one; confirm no others (session card / terminal / view-history).
2. **Full-10 `.strict()` vs minimal shadow** — my default: **full-10 `.strict()` drift-pin** (the row now has a consumer → freeze the full shape, the ApprovalQueueRow/PullRequestRow discipline; harness/model/execution_profile_id are shadowed-with even if not yet rendered — the drift-pin needs the full served set). Flag if you'd rather minimal (non-strict) — but then the served extras would need tolerating.
3. **Banner-derivation deferral** — confirm the RecoveryState banner stays fixture-fed this slice (the daemon-gap; do NOT build the derivation). My default: defer (per the Finding).

## Dependencies + sequencing
- **Depends on:** the 0.38 merge (SessionRow frozen @ 0.35 in the tree) + daemon D3 (Session delta emission — landed on main, now on track/ui).
- **Blocks:** ui-063 (the rest of whole-cockpit-live — ProjectActivity/PullRequest/UsageLedger refetch-on-nudge, the same pattern). The RecoveryState banner real-data = a deferred daemon ask (work-order).

## Estimated commit count
**1** (cohesive Session slice — the refetch-on-nudge rewire + the shadow drift-pin + the forced consumers are one tsc-green unit). If the forced-consumer set is large at verify, a 2-commit split (shadow+consumers / refetch-on-nudge) is acceptable — your call. NON-cat-1.

## Lessons-logged candidates anticipated
- Likely a one-line reinforcement of `LESSONS.md` §29 (the `row:None` refetch-on-nudge generalized from ApprovalQueue to Session — the Mock-vs-real gap the 052 reducer masked) + the SessionRow shadow-on-consume freeze.
- **Future TODO (carry-forward):** ui-063 (the other-projections refetch-on-nudge spread); the RecoveryState banner daemon-status signal (a daemon ask — work-order).

## How to invoke
1. Read this brief end-to-end — especially the deferred banner + the forced-consumer verify.
2. Confirm RED: `pnpm test src/shell/Shell.subscribe.test.tsx src/contracts/provisional.test.ts src/projections/items.test.ts`.
3. `/tdd session_live_and_survival_shadow`.
4. Step 2.5 → the forced-consumer set + the 3 default calls.
5. GREEN → full suite + `/preflight`.
6. Step 9 → the cross-doc row + the ui-063 + daemon-ask carry-forwards.

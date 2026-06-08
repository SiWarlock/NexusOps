# /tdd brief — survival_recovery_display

## Feature
The **Survival/recovery DISPLAY surfaces** (O-2, §11.4): a **post-restart recovery banner** (recovering / recovered / recovery-failed) + a **per-session resumed-(live) vs replayed-(relaunched) indicator**. Fixture-driven (the daemon survival logic = O-2, not built); the **"restart session" affordance is PARKED** (an intent — daemon-1.5). DISPLAY only. The **safety-state display (fencing/hard-conflict + fail-closed/audit-integrity) is a SEPARATE slice** (6.4d-2 — §17, security-reviewer; not here). Daemon-independent (Decision-C track).

## Use case + traceability
- **Task ID:** P6.4d (6.4 decomposition: 6.4a/b/c ✅ → **6.4d Survival/recovery display** → 6.4d-2 safety-state display → accessible-names/checking-banner → 6.5 theme pass).
- **Architecture sections:** `ARCHITECTURE.md §11.4` (Survival/recovery UX O-2: recovery banner; resumed-vs-replayed per session; "restart session" for recovery-failed), `§8`/`§17` (survival/failure-mode contract — the source of the recovery states), `§4.2` (renders from projections). No safety invariant in THIS slice (display only; the §17 conflict/audit-integrity surfaces are 6.4d-2).
- **Related context:** the provisional-shape pattern (Lesson §2 — recovery state + `resume_mode` are not in the frozen contract → provisional, banner-marked); the connection `DegradedBanner` (6.1c — the recovery banner is DISTINCT: transport-degraded vs session-survival); the Sessions table / session VMs (6.3c — the resumed/replayed indicator attaches per session); Lesson §9 (any new controls keyboard-reachable; the reachability audit). Deterministic core = the recovery-state→banner + `resume_mode`→indicator mappings (pure); render-tested.

## Acceptance criteria
- [ ] A provisional **recovery-state** shape + fixture (`recovering | recovered | recovery_failed` + the affected-session set) + a provisional **`resume_mode`** (`resumed | replayed`) field on the session VM (banner-marked, Lesson §2; reconcile at the daemon survival-schema freeze).
- [ ] A pure recovery model (`ui/src/recovery/model.ts`): recovery-state → banner descriptor (kind + message + whether a restart affordance applies); `resume_mode` → indicator descriptor (resumed=live / replayed=relaunched, never-color-alone — glyph+label).
- [ ] `<RecoveryBanner/>` renders the post-restart banner (recovering/recovered/recovery-failed) — **distinct** from the transport `DegradedBanner`; **recovered** auto-dismisses or is non-intrusive; **recovery-failed** surfaces the affected sessions + a **parked "Restart session"** affordance (rendered disabled/parked — gated on `canSubmitIntent`, intent deferred to daemon-1.5; NOT wired).
- [ ] A **resumed/replayed indicator** on each session (e.g. in the Sessions table + sidebar) via the descriptor (glyph+label, never color alone — §11/§11.6). New controls keyboard-reachable + focus ring (Lesson §9); the reachability audit still green.
- [ ] Renders only fixture/projection state (no invented recovery — forbidden #2); `/preflight` clean.
- [ ] **Reachable from** `Shell → <RecoveryBanner/>` (post-restart) + `SessionsTable/Sidebar → resumed/replayed indicator`.

## Wiring / entry point (Step 7.5)
`Shell` renders `<RecoveryBanner/>` (driven by the recovery-state input, fixture) above/with the content; the session VMs carry `resume_mode` → the indicator renders in the Sessions table + sidebar. Confirm the banner is reachable in the shell + the indicator attaches per session. The parked "Restart session" affordance is present-but-disabled (tracked, not a false wire — like 6.1c's `canSubmitIntent`).

## Files expected to touch
**New:** `ui/src/recovery/{model.ts, RecoveryBanner.tsx, model.test.ts, RecoveryBanner.test.tsx}`, fixtures for recovery state.
**Modified:** `ui/src/shell/Shell.tsx` (render `<RecoveryBanner/>`), `ui/src/views/sessions/model.ts`+`SessionsTable.tsx` (resume_mode indicator), `ui/src/projections/items.ts` or the session VM (resume_mode field), `ui/src/contracts/provisional.ts` (provisional recovery + resume_mode), relevant tests + `a11y/reachability.test.tsx` if a new control lands. Flag anything beyond at Step 2.5.

## RED test outline (Step 2)
**`recovery/model.test.ts`:**
1. **`recovery_state_to_banner_descriptor`** — `recovering`/`recovered`/`recovery_failed` → the right banner kind + message; `recovery_failed` flags the restart affordance applies. **[load-bearing]**
2. **`recovered_is_non_intrusive`** — `recovered` (or absent recovery) → no blocking banner (non-intrusive/auto-dismiss semantics).
3. **`resume_mode_to_indicator`** — `resumed`→live descriptor, `replayed`→relaunched descriptor (glyph+label, distinct). **[never-color-alone]**

**`recovery/RecoveryBanner.test.tsx` (jsdom):**
4. **`renders_recovery_failed_with_parked_restart`** — `recovery_failed` renders the affected sessions + a **disabled/parked** "Restart session" control (present, not wired — gated). **[load-bearing — parked-intent discipline]**
5. **`distinct_from_transport_degraded_banner`** — the recovery banner is a distinct surface from the connection `DegradedBanner` (not conflated).

**`views/sessions/SessionsTable.test.tsx` (extend):**
6. **`session_shows_resume_mode_indicator`** — a session row renders its resumed/replayed indicator (glyph+label).

## Cross-doc invariant impact
- **Model field changes:** **none frozen.** Recovery state + `resume_mode` are **provisional** (Lesson §2) — I'll add them to the Carry-forward provisional→generated reconcile spread at Step-9. **Orchestrator rows:** none.

## Things to flag at Step 2.5
1. **Recovery banner vs transport DegradedBanner.** Default vote: a **distinct** `<RecoveryBanner/>` (session-survival, O-2) separate from the transport `DegradedBanner` (6.1c). Confirm.
2. **Resume-mode indicator placement.** Default vote: in the Sessions table (a column/badge) + the sidebar item. Confirm the surfaces.
3. **Parked "Restart session".** Default vote: rendered **disabled/parked** (gated on `canSubmitIntent`; intent deferred to daemon-1.5) — present so the recovery-failed UX is complete, not wired. Confirm vs omitting it entirely.
4. **Recovery-state source.** Default vote: a provisional recovery-state input (fixture) — NOT derived from the connection state (recovery = session survival, distinct from transport). Confirm.

## Dependencies + sequencing
- **Depends on:** 6.1c connection model (distinct-from), 6.3c Sessions table (indicator host), Lesson §2 (provisional) + §9 (a11y). No daemon dependency (fixture-driven; real recovery state integrates at the daemon survival-schema freeze).
- **Blocks:** 6.4d-2 (safety-state display — conflict/audit-integrity); the O-2 survival demo surface.
- **Note:** unstyled until the 6.5 theme pass (accepted).

## Estimated commit count
**1** — a cohesive survival-display slice (recovery model + banner + resume-mode indicator). No safety invariant (display only; the §17 conflict/audit-integrity surfaces + the restart INTENT are separate/parked) → **security-reviewer NOT required**; **code-quality every-slice**. (If the recovery banner + the per-session indicator feel separable, splitting to 2 is fine — flag at Step 2.5; default 1.)

## Lessons-logged candidates anticipated
- **Convention candidate** — survival/recovery is a DISTINCT surface from transport-degraded (don't conflate session-survival with connection state); parked intents render disabled-but-present (recovery-failed UX complete, restart wired at daemon-1.5). Candidate if it recurs.
- **Future TODO — provisional reconcile** — recovery state + `resume_mode` → generated at the daemon survival-schema freeze (added to the spread); the "Restart session" intent wires at daemon-1.5.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd survival_recovery_display`.
1. Read this brief; Q1 (distinct banner) + Q3 (parked restart) are the ones to confirm at Step 2.5.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN.
3. Step 7.5 — name `Shell → RecoveryBanner` + `Sessions → resume-mode indicator`.
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me.

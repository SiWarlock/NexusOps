# /tdd brief — checking_handshake_banner

## Feature
A **"checking / handshaking" degraded-banner variant** that closes the **silent-read-only gap**: today `deriveDegradedState("connected", "unknown")` returns `"ok"` (no banner) while `canSubmitIntent({connection:"connected", version:"unknown"})` is **fail-safe FALSE** — so the post-connect, pre-version-confirm handshake window is **read-only with no explanation**. Add a `"checking"` `DegradedState` so that window renders a **non-intrusive "confirming compatibility… — read-only until the handshake completes"** banner. **Defense-in-depth UX completeness** (read-only is never silently unexplained); the load-bearing gate stays `canSubmitIntent` (unchanged) → **no §15/§17 invariant touched** (security-reviewer NOT required).

## Use case + traceability
- **Task ID:** P6.4 (inlined follow-up; closes the §6.4 checking-banner item — origin 2026-06-07 P6.1c). Last Phase-6 logic item before the 6.5 theme pass.
- **Architecture sections it implements:** `ARCHITECTURE.md §11.4` (the global READ-ONLY degraded mode + its banner — read-only must be explained, never silent), `§6.4` (HelloAck/handshake / `protocol_version` — the version-unknown window is the handshake-in-progress state), `§16` (version-skew/compat — the checking state precedes the compat verdict).
- **Related context:** `ui/src/connection/version.ts` (`deriveDegradedState` + the `DegradedState` union) + `DegradedBanner.tsx` (the existing variants) + `read-only.ts` (`canSubmitIntent` — the fail-safe gate, **unchanged** here, Lesson §4); the existing `version.test.ts` comment already flags this exact follow-up (*"a 'checking' banner variant is a [follow-up]"*). The banner is already Shell-wired (the new variant flows automatically). This pairs conceptually with the 6.4d-2 safety surfaces' "fail-closed display" family — the UI explains its degraded/safety states; never silent.

## Acceptance criteria
- [ ] `DegradedState` gains a `"checking"` member; `deriveDegradedState("connected", "unknown")` → `"checking"` (was `"ok"`).
- [ ] **Precedence preserved:** `update_required` > `disconnected` > `reconnecting`/`connecting` > **`checking` (connected+unknown)** > `ok`. So `connected`+`update_required` → `"update_required"`; `disconnected`+`unknown` → `"disconnected"`; `connecting`+`unknown` → `"reconnecting"` (still establishing transport — NOT checking); `connected`+`compatible` → `"ok"` (handshake resolved).
- [ ] `<DegradedBanner/>` renders the `"checking"` variant: a **non-intrusive** "confirming compatibility… — read-only until the handshake completes" message, `data-degraded="checking"`, **`role="status"`** (polite — transient/informational, distinct from the `role="alert"` failure variants), and **no Retry/Repair buttons** (a handshake self-resolves; Retry/Repair don't apply).
- [ ] `canSubmitIntent` is **unchanged** (still FALSE for connected+unknown — Lesson §4 fail-safe); this slice only *explains* the existing read-only, it does not change the gate.
- [ ] Renders only derived state (no invented state — forbidden #2); `/preflight` clean.
- [ ] **Reachable from** `Shell → deriveDegradedState(connection, version) → <DegradedBanner degraded="checking"/>` during the connected+version-unknown handshake window (the banner is already Shell-wired; the variant flows through).

## Wiring / entry point (Step 7.5)
The Shell already computes `deriveDegradedState(connection, version)` and renders `<DegradedBanner/>`. Confirm the `"checking"` variant is reachable when `connection="connected"` & `version="unknown"` (the handshake window) — no new Shell wiring expected; the derivation returns the new state and the banner renders it. Flag if the Shell needs a change.

## Files expected to touch
**Modified:**
- `ui/src/connection/version.ts` — add `"checking"` to `DegradedState`; the `connected`+`unknown` branch in `deriveDegradedState`
- `ui/src/connection/DegradedBanner.tsx` — render the `"checking"` variant (non-intrusive, `role="status"`, no buttons)
- `ui/src/connection/version.test.ts` — update the connected+unknown assertion (→ `"checking"`); add precedence cases. *(Legitimate contract-evolution edit — the behavior intentionally changes; the old "ok" assertion + its "intentional for now" comment are superseded.)*
- `ui/src/connection/DegradedBanner.test.tsx` — render test for the checking variant + distinct-from-failure-variants

If implementation needs files beyond this list (e.g. a Shell change), **flag at Step 2.5**.

## RED test outline (Step 2)
**`connection/version.test.ts` (extend/update):**
1. **`connected_unknown_is_checking`** — Asserts: `deriveDegradedState("connected","unknown") === "checking"` (was "ok"). Why: §11.4 read-only must be explained; closes the silent-read-only gap.
2. **`checking_precedence`** — Asserts: `connected`+`update_required`→"update_required"; `disconnected`+`unknown`→"disconnected"; `connecting`+`unknown`→"reconnecting"; `connected`+`compatible`→"ok". Why: §16 precedence — checking is the lowest-severity degraded state, only the connected+unknown window.

**`connection/DegradedBanner.test.tsx` (extend):**
3. **`renders_checking_variant`** — Asserts: `degraded="checking"` → a banner with the read-only-during-handshake message, `data-degraded="checking"`, `role="status"`, and **no Retry/Repair button**. Why: §11.4 non-silent read-only; transient/self-resolving.
4. **`checking_distinct_from_failure_variants`** — Asserts: checking has no Retry/Repair (vs `reconnecting`/`disconnected` which do; `update_required` has Repair). Why: handshake isn't a failure — don't offer failure affordances.

## Cross-doc invariant impact
- **Model field changes:** **none.** `DegradedState` is a **UI-local** type (not a frozen contract). `SUPPORTED_PROTOCOL_RANGE` reconcile is already tracked in the Carry-forward `ui ↔ daemon-1.5` spread — unchanged by this slice.
- **Orchestrator doc rows to write hot (Step 9 routing):** none.

## Things to flag at Step 2.5
1. **State name — `"checking"` vs `"handshaking"`.** Default: **`"checking"`** — the existing `version.test.ts` comment already uses it; shorter. My default vote: **`"checking"`**.
2. **Buttons on the checking banner.** Default: **none** — a handshake self-resolves (→ ok / update_required / disconnected, each with its own variant); Retry/Repair don't apply. My default vote: **no action buttons** (non-intrusive informational banner).
3. **`connecting`+`unknown` stays `"reconnecting"`?** Default: **yes** — `connecting` is pre-handshake (transport not yet up) → the reconnecting/connecting banner is right; `"checking"` is specifically `connected` (transport up) + `unknown` (version not yet confirmed). My default vote: **checking = connected+unknown ONLY**.
4. **`role="status"` (polite) vs `role="alert"` (assertive) for checking.** Default: **`role="status"`** — checking is transient/informational, not an error; the failure variants stay `role="alert"`. My default vote: **`role="status"`** (a11y: don't assertively interrupt for a self-resolving handshake).

## Dependencies + sequencing
- **Depends on:** 6.1c connection/version model (`deriveDegradedState`, `DegradedBanner`, `canSubmitIntent`) — all landed.
- **Blocks:** nothing — this is the **last Phase-6 logic item**; next is the **6.5 Graphite Arc theme pass + automated visual gate**.
- **Note:** unstyled until the 6.5 theme pass (accepted).

## Estimated commit count
**1** — a focused connection-module slice (one new degraded state + its banner variant), same code area, < 30 lines. **No safety invariant** (the `canSubmitIntent` gate is unchanged — this only *explains* the existing read-only) → **security-reviewer NOT required**; **code-quality every-slice**.

## Lessons-logged candidates anticipated
- **Convention candidate** — **read-only is never silently unexplained**: every `canSubmitIntent === FALSE` state has a `DegradedState` banner variant (no silent read-only). Pairs with the 6.4d-2 "fail-closed display" family (the UI explains its degraded/safety states). Likely folds into the safety-surface convention lesson at round close.
- **Future TODO — reconcile** — `SUPPORTED_PROTOCOL_RANGE` + the real handshake `protocol_version` reconcile at daemon-1.5 (already in the Carry-forward `ui ↔ daemon-1.5` spread); the checking→compatible/update_required transition exercises the real handshake then.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd checking_handshake_banner`.
1. Read this brief; Q1 (name) + Q4 (role) are the quick confirms.
2. Step 2.5 — test-design write-up (`Asserts:` per test) → wait for the magic-words reply → GREEN.
3. Step 7.5 — name `Shell → deriveDegradedState → DegradedBanner (checking)`.
4. Step 9 — commit-message-first; then `TaskUpdate` the slice task → completed + wake me. (This closes the last Phase-6 logic item → next is the 6.5 theme pass.)

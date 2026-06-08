# /tdd brief — connection_readonly_and_version_skew

## Feature
The daemon-connection state machine + its UI surfaces: a **daemon-connection indicator** (connected / reconnecting / disconnected — distinct from LocalRunner health), a **global READ-ONLY degraded mode** that disables every intent-submitting control + shows a banner with Retry/Repair, and **version-skew → "update required"** handling. This is slice **6.1c** — the deterministic, safety-relevant state logic completing the 6.1 shell (6.1a foundation `fd9738b` + 6.1b chrome `39a87c6` landed). It fills the slots 6.1b reserved (`StatusBar data-slot="connection-indicator"`, the shell load-error placeholder).

## Use case + traceability
- **Task ID:** P6.1c (decomposition of 6.1: 6.1a → 6.1b → **6.1c**)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.4` (net-new surfaces: daemon-connection indicator distinct from LocalRunner health; global READ-ONLY degraded mode disabling Gateway approve/deny, Dispatch, Brain Run-via-Gateway, commit/push, + banner + Retry/Repair; version-skew + update states), `§6.4` (handshake `HelloAck`/`VersionSkewError`, `protocol_version`), `§16` (version-compatibility matrix; UI↔daemon mismatch → refuse+relaunch/update), `§17` (failure-mode contract; degraded/offline states first-class), `§7.2` (degraded SoT), `§4.2` (UI = client).
- **Related context:**
  - 6.1b shell: `ui/src/shell/` (StatusBar with the reserved connection slot; Shell load-error state). 6.1a `gateway-client` (`GatewayPort` interface + `MockGatewayPort` + boundary).
  - **This is the safety-relevant slice → `security-reviewer` runs** (`invariant` policy): the read-only gate is a defense-in-depth guard. NOTE: the load-bearing INV-SEC-1 enforcement is **daemon-side** (the Gateway rejects mutations regardless of UI state); the UI read-only mode is defense-in-depth UX (don't *offer* a mutation when the daemon is unreachable) — verify it's **fail-safe**, not the sole guard.
  - On the `MockGatewayPort` (no real UDS yet): connection transitions + a skewed `get_capabilities` are **simulated** by the mock; the real signal arrives with the `UdsGatewayPort` (daemon 1.5).

## Acceptance criteria (what "done" means)
- [ ] A **connection-state model** (connected / reconnecting / disconnected) with legal transitions, owned by the `gateway-client` transport seam (`getConnectionState()` + `onConnectionChange(cb)`).
- [ ] **`deriveReadOnly(state)`** → true when disconnected/reconnecting (or version-skew); false only when connected+compatible.
- [ ] **`canSubmitIntent`** is the single gate every intent-submitting control will consult — **false in read-only**, and **fail-safe FALSE on unknown/initial state** (before the first successful handshake).
- [ ] **`checkVersionCompat(capabilities, supportedRange)`** → "update required" when `protocol_version` is out of the UI's supported range; **update-required takes precedence over reconnecting** (Retry can't fix a version mismatch).
- [ ] **ConnectionIndicator** renders connected/reconnecting/disconnected in the StatusBar slot, **never color alone** (glyph + text label + intensity — §11.1) .
- [ ] **DegradedBanner** renders in read-only with **Retry/Repair** affordances that stay **enabled** while disconnected (reconnect is a transport action, not a gated intent); an "update required" variant when version-skew.
- [ ] **Reconnect restores live state** — disconnected → reconnecting → connected removes the banner and flips `canSubmitIntent` back to true.
- [ ] `/preflight` clean; `security-reviewer` run (invariant policy).

## Wiring / entry point (Step 7.5)
- **Reachable now:** `Shell` wraps a `ReadOnlyProvider` + mounts `DegradedBanner`; `StatusBar` mounts `ConnectionIndicator`; Retry/Repair → `gateway-client` reconnect. These render on the real path.
- **Reachable-by-6.3+ (tracked):** `canSubmitIntent` is the gate the *future* intent controls (Gateway approve/deny, Dispatch, Brain Run-via-Gateway, commit/push) MUST consult — they don't exist yet (chrome+reads only). Pin `canSubmitIntent` at the predicate level now; add a **forbidden-pattern** ("every intent-submitting control consults `canSubmitIntent` / is disabled in read-only") so the rule is enforced when those controls land. Name this honestly at Step 7.5 — the mechanism is reachable; its future consumers are tracked, not silently unreachable.

## Files expected to touch
**New:**
- `ui/src/connection/state.ts` — connection-state model + transitions.
- `ui/src/connection/read-only.ts` — `deriveReadOnly` + `canSubmitIntent` + `ReadOnlyProvider`/context + `useCanSubmitIntent()`.
- `ui/src/connection/version.ts` — `checkVersionCompat` + the supported `protocol_version` range constant.
- `ui/src/connection/{ConnectionIndicator,DegradedBanner}.tsx`
- `ui/src/connection/{state,read-only,version}.test.ts`, `ui/src/connection/DegradedBanner.test.tsx` (or co-located render tests).

**Modified:**
- `ui/src/gateway-client/types.ts` — add `getConnectionState()` + `onConnectionChange(cb)` to `GatewayPort`.
- `ui/src/gateway-client/mock.ts` — simulate connection transitions (`setConnectionState`) + a skewable `get_capabilities`.
- `ui/src/shell/Shell.tsx` — wrap `ReadOnlyProvider`; mount `DegradedBanner`.
- `ui/src/shell/StatusBar.tsx` — mount `ConnectionIndicator` in the reserved slot.

Flag any file beyond this at Step 2.5.

## RED test outline (Step 2)
**`connection/state.test.ts`:**
1. **`connection_state_legal_transitions`** — connected→disconnected→reconnecting→connected; illegal jumps rejected/marked. Asserts the state machine. Why §11.4.

**`connection/read-only.test.ts`:**
2. **`disconnected_or_reconnecting_is_read_only`** — `deriveReadOnly` true for disconnected + reconnecting, false for connected. Why §11.4 global read-only.
3. **`can_submit_intent_false_in_read_only`** — `canSubmitIntent` false when read-only, true when connected+compatible. Why §11.4 (disables every intent control). **[safety pin]**
4. **`can_submit_intent_fail_safe_false_on_unknown`** — before the first successful handshake (unknown/initial), `canSubmitIntent` is FALSE. Why §15/§17 fail-closed — never offer a mutation until known-connected. **[safety pin — load-bearing]**

**`connection/version.test.ts`:**
5. **`version_skew_out_of_range_requires_update`** — `checkVersionCompat` → update-required when `protocol_version` out of the supported range. Why §6.4 `VersionSkewError`; §16 matrix.
6. **`version_in_range_is_compatible`** — in-range → compatible. Why §16 happy path.
7. **`update_required_precedes_reconnecting`** — when both skew and a dropped connection are present, the derived degraded state is update-required (not reconnecting). Why §16 (must update; Retry won't fix). **[precedence pin]**

**`connection/DegradedBanner.test.tsx` + StatusBar render (jsdom):**
8. **`connection_indicator_never_color_alone`** — indicator shows glyph + text label for each state (not color-only). Why §11.1 never-color-alone; `ui/CLAUDE.md` forbidden-pattern #5.
9. **`degraded_banner_retry_repair_enabled_when_disconnected`** — disconnected → banner with Retry/Repair, and those affordances are enabled (not gated by read-only). Why §11.4 banner + Retry/Repair.
10. **`reconnect_restores_live_state`** — disconnected→connected removes the banner and flips `canSubmitIntent` true (via a test consumer of `useCanSubmitIntent`). Why §11.4 reconnect restores live state.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none frozen. Connection-state + version types are UI-local (transport concerns, not the frozen contract).
- **Orchestrator doc rows to write hot:** flag a **new `ui/CLAUDE.md` forbidden-pattern** — "every intent-submitting control consults `canSubmitIntent` / is disabled in global read-only mode (§11.4); a mutation affordance offered while the daemon is unreachable is a defense-in-depth breach." I write it (orchestrator territory). Likely a `ui/LESSONS.md §4` (fail-safe read-only gate) too.

> Implementer never edits `ui/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, `ui/LESSONS.md`.

## Things to flag at Step 2.5
1. **Connection-state owner.** Default vote: **the `gateway-client`** owns transport connection-state (`getConnectionState()` + `onConnectionChange`); `MockGatewayPort` simulates. It IS the transport seam, so connection liveness belongs there (the real `UdsGatewayPort` will drive it from the socket). Confirm vs a separate connection-manager.
2. **Fail-safe default (safety).** Default vote: **`canSubmitIntent` defaults to FALSE until a successful handshake confirms connected + version-compatible** (§15/§17 fail-closed). Strongly recommended — confirm.
3. **Degraded-state precedence.** Default vote: **update-required > disconnected/reconnecting > connected** (a version mismatch is not Retry-able). Confirm.
4. **Scope of "intent controls" to gate now.** Default vote: **build the `canSubmitIntent` mechanism + indicator + banner (reachable); the actual approve/deny/Dispatch/commit controls are 6.3+ and consult it then** (enforced via the new forbidden-pattern). Don't build a placeholder gated control. Confirm.
5. **Indicator vs LocalRunner health.** Default vote: **6.1c builds ONLY the daemon-connection (transport) indicator** (§11.4 "distinct from LocalRunner health"); LocalRunner health is a projection-driven signal for a later slice. Confirm the separation.

## Dependencies + sequencing
- **Depends on:** 6.1a (`fd9738b`) + 6.1b (`39a87c6`) — gateway-client seam + shell with reserved connection/degraded slots.
- **Blocks:** 6.3+ intent controls (must consult `canSubmitIntent`); the real `UdsGatewayPort` integration (daemon 1.5) drives the connection-state + real handshake.
- **Completes 6.1** — after this, tick 6.1 (`[x]`) at `/orchestrate-end` (all three sub-slices landed).

## Estimated commit count
**1–2.** Cohesive degraded-mode logic (connection-state + read-only + version-skew). The fail-safe `canSubmitIntent` gate (tests 3–4) is safety-relevant — implementer may isolate it in its own commit, else bundle (it's defense-in-depth, not the §15 invariant itself, which is daemon-side). Implementer's call at Step 9. `security-reviewer` runs regardless (invariant policy).

## Lessons-logged candidates anticipated
- **Convention candidate (`ui/LESSONS.md §4`)** — UI read-only/degraded gate is **fail-safe** (`canSubmitIntent` defaults FALSE on unknown; true only on confirmed connected+compatible); it's defense-in-depth, never the sole mutation guard (that's the daemon Gateway, INV-SEC-1).
- **Forbidden-pattern candidate** — every intent-submitting control consults `canSubmitIntent` / is disabled in read-only mode.
- **Architecture-doc note candidate** — if the supported `protocol_version` range needs to be pinned to a value §6.4/§16 doesn't yet state, flag it (don't invent a range silently).

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd`.
1. **Read this brief end-to-end** — the safety pins (tests 3–4 fail-safe) + Step-2.5 Q2 are the load-bearing parts.
2. **Run `/tdd connection_readonly_and_version_skew`.**
3. **Step 0/1** — restate (connection/read-only/version-skew; fills 6.1b's reserved slots); confirm files.
4. **Step 2.5** — test-design write-up + answers to the 5 questions. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
5. **Step 8** — run `security-reviewer` (invariant policy) + code-quality (every-slice).
6. **Step 7.5** — name the reachable surfaces (indicator/banner) + the reachable-by-6.3+ `canSubmitIntent` gate honestly.
7. **Step 9** — flag the new forbidden-pattern + the fail-safe lesson; commit-message-first.

# ui-003 — §17 safety-state display + checking/handshaking banner (Phase-6 logic finish)

- **Date:** 2026-06-08
- **Phase:** Phase 6 (UI track, `track/ui`) — the **last Phase-6 logic items** before the 6.5 theme pass.
- **Predecessor session:** [ui-002](ui-002-2026-06-08-phase6-graph-sessions-a11y-usage-settings-survival-topbar.md)
- **Successor session:** [ui-004](ui-004-2026-06-08-graphite-arc-theme-pass-and-visual-gate.md) (6.5 Graphite Arc theme pass + the full visual gate — closes Phase-6 visual)
- **Round commits:** `ff2f8d6` (P6.4d-2 L1) · `503b6a2` (P6.4d-2 L2) · `5f40149` (P6.4 checking-banner). Suite **116 → 131 green**; tsc + oxlint clean throughout.

## Why this session existed

Finish the remaining Phase-6 logic the round-2 seal carried forward: **6.4d-2** (the §17 safety-state display surfaces — split from 6.4d) and the **checking/handshaking degraded-banner** (the §6.4 silent-read-only follow-up, origin 2026-06-07 P6.1c). Closing these is the Phase-6 **logic** finish line; only the 6.5 Graphite Arc theme/visual pass remains.

## What was built

Built entirely against frozen `shared/` 0.5.0 + `MockGatewayPort` + fixtures + `NexusOps-ui-kit`. No daemon dependency consumed. No frozen contract field changed.

### P6.4d-2 — §17 safety-state display (2-commit safety slice; first §15/§17-touching UI slice)

**L1 — fencing / hard-conflict card (#6 never-auto-resolved) — `ff2f8d6`**

Files created:
- `ui/src/safety/model.ts` — `describeConflict(FencingConflict) → ConflictCardDescriptor` (the load-bearing "never auto-resolved / manual resolution" message; `resolutionParked: true`; glyph/label/severity; no auto-resolve field produced).
- `ui/src/safety/HardConflictCard.tsx` — the fencing/hard-conflict card: affected action/session refs, the never-auto-resolved copy, and a parked (disabled) resolution control gated on `canSubmitIntent`. A distinct surface from DegradedBanner + RecoveryBanner.
- `ui/src/safety/fixtures.ts` — `safetyCleanFixture` (clean default → nothing renders) + `fencingConflictFixture`.
- `ui/src/safety/model.test.ts`, `ui/src/safety/HardConflictCard.test.tsx` — RED-first tests.

Files modified:
- `ui/src/contracts/provisional.ts` — added provisional `ConflictReason` (single-member `fencing_conflict`) + `FencingConflict` + (later) `SafetyState` (Zod schema mirroring the `RecoveryStatus` input-prop shape).
- `ui/src/shell/Shell.tsx` — a fixture-defaulted `safety` prop + a minimal Shell-level `.safety-host` rendering `<HardConflictCard/>`.
- `ui/src/shell/Shell.test.tsx` — Shell-host reachability tests.

**L2 — fail-closed / audit-integrity alert (#5 non-dismissible) — `503b6a2`**

Files created:
- `ui/src/safety/AuditIntegrityAlert.tsx` — the fail-closed audit-integrity alert: the §11.4 named treatments, each on a glyph+label+severity channel (glyph derived from severity); **non-dismissible** (no local dismiss; acknowledge rendered disabled/parked). Rendered prominently near the Shell banner stack.
- `ui/src/safety/AuditIntegrityAlert.test.tsx` — RED-first tests incl. the completeness render test over the full discriminated union.

Files modified:
- `ui/src/contracts/provisional.ts` — `AuditOutcomeStatus = ActionRequest.extract(["partially_succeeded","rollback_failed"])` (REUSES the frozen enum, drift-pinned), provisional `AuditIntegrityKind` (`unknown_outcome`/`audit_write_failed`/`corrupt_payload`), `AuditIntegrityState` (discriminated union on `source`), and `SafetyState.integrity` (required-nullable).
- `ui/src/safety/model.ts` — `describeAuditIntegrity` + `ACTION_OUTCOME`/`INTEGRITY` treatment maps + `SEVERITY_GLYPH` (glyph derived from severity).
- `ui/src/safety/model.test.ts`, `ui/src/safety/fixtures.ts`, `ui/src/shell/Shell.tsx` (wired `<AuditIntegrityAlert/>`), `ui/src/shell/Shell.test.tsx`.

### P6.4 — checking/handshaking degraded-banner variant — `5f40149`

Files modified:
- `ui/src/connection/version.ts` — added `"checking"` to `DegradedState` + the connected+version-unknown branch in `deriveDegradedState` (precedence: `update_required > disconnected > reconnecting/connecting > checking > ok`). Defensive explicit `connection === "connected"` guard.
- `ui/src/connection/DegradedBanner.tsx` — the `checking` variant: non-intrusive `role="status"`, "confirming compatibility… — read-only until the handshake completes", no Retry/Repair.
- `ui/src/connection/version.test.ts` — superseded the old `..._is_ok` assertion with `connected_unknown_is_checking` + `checking_precedence` (incl. a `reconnecting+unknown`→`reconnecting` guard-inversion pin).
- `ui/src/connection/DegradedBanner.test.tsx` — `renders_checking_variant` + `checking_distinct_from_failure_variants`.
- `ui/src/shell/Shell.tsx` — a comment-only trigger-pending note at the `deriveDegradedState` call.

## Decisions made

- **6.4d-2 scoped to a 2-commit safety slice** (each invariant its own bisectable commit; security-reviewer on each layer) — both layers PASS, 0 findings.
- **L1 card scoped to `fencing_conflict` ONLY** (orchestrator TWEAK): `stale_precondition` is a §17/§6.2 **re-approvable** flow (regenerate preview → fresh approval), not a never-auto-resolved hard conflict; rendering it on the card with "never auto-resolved" copy would misrepresent a resolvable state as terminal. Deferred to the approval/preview surface.
- **L2 reuses the frozen `ActionRequest` enum via `.extract`** for `partially_succeeded`/`rollback_failed` (drift-pinned — breaks the build + throws at load if the frozen enum renames them; Lesson §2). Only the net-new states are provisional. Modeled as a discriminated union on `source`.
- **`SafetyState.integrity` is required-nullable** (not optional) — fail-closed-by-construction: a #5 caller must explicitly decide `null` vs an alert, can't silently omit a must-be-seen signal.
- **Audit-integrity glyph derived from severity** (root fix for a review finding): `critical → ⛔`, `warning → ⚠`, so the non-color channel can never drift from the severity it signals (§11.6).
- **Render-completeness forced over the full discriminated union** (`renders_each_audit_integrity_treatment` enumerates from `.options`): a future-added integrity kind is forced to render, never silently dropped (#5).
- **checking-banner: `role="status"` (polite), no buttons, connected+unknown ONLY** (`connecting+unknown` stays `reconnecting`). The `canSubmitIntent` gate is **unchanged** — this only *explains* the existing read-only.
- **checking-banner reachability: accept wired-but-masked-until-daemon-1.5** (lead/orchestrator decision) — a state-coverage gap, not a wiring gap; no Shell behavior change, in-code note added.

## Decisions explicitly NOT made (deferred)

- **stale-precondition re-approval treatment** — deferred to the approval/preview surface + the parked intent seam (6.3d/e / Gateway modal), not this slice.
- **The parked resolution/acknowledge intents** — daemon-1.5; rendered disabled-but-present now.
- **A Shell load-gate / reconnect-re-handshake change** to surface the checking window live — daemon-1.5 integration territory (building a mock approximation now would be speculative + reconcile-later).
- **The full 7-group Human Input Queue host** — Phase 8 (intent seam); the MVP renders these safety surfaces at the Shell level.

## TDD compliance

**Clean — no violations.** All three slices were strictly test-first: tests written at Step 2, reviewed at Step 2.5, RED confirmed for the right reason (missing modules/exports; `"ok"` vs `"checking"`; absent `role="status"`), then GREEN. Review-driven changes (glyph-from-severity, `SafetyState`→Zod-in-provisional, exact-key-set / severity / completeness assertions) refined code that already had tests — not new untested behavior.

## Reachability

- **`<HardConflictCard/>`** — `main.tsx → <Shell/> → .safety-host → <HardConflictCard conflict={safety.conflict}/>`. Reachable; default clean fixture renders nothing (non-intrusive, like 6.4d `recovered`). Parked resolve present-but-disabled.
- **`<AuditIntegrityAlert/>`** — `main.tsx → <Shell/> → <AuditIntegrityAlert integrity={safety.integrity}/>` (prominent, near the banner stack). Reachable; default clean renders nothing. Acknowledge present-but-disabled; no dismiss control.
- **checking-banner** — `main.tsx → <Shell/> → deriveDegradedState(connection, version) → <DegradedBanner degraded="checking"/>`. Wired + unit-reachable; the connected+unknown **trigger** is trigger-pending (masked by the Shell `!data` load gate; `version` set once with `data`) → surfaces at the real daemon-1.5 reconnect re-handshake. Documented in-code at the Shell call site.

No tested-but-unwired gaps. The checking trigger is a state-coverage gap (wired, not yet driven), not a wiring gap.

## Open follow-ups (Step-9 categorized — already routed hot to the orchestrator)

- **Provisional → generated reconcile (cross-track spread):** `ConflictReason`/`FencingConflict`/`SafetyState` + `AuditIntegrityKind`/`AuditIntegrityState` (and `AuditOutcomeStatus`, which already reuses the frozen `ActionRequest`) → reconcile at the daemon §17/survival-schema freeze. Extends the existing provisional→generated spread (alongside the Usage + Recovery shapes).
- **stale-precondition re-approval treatment** — belongs with the approval/preview surface + the intent seam (6.3d/e / Gateway modal).
- **checking-banner trigger-pending** — surfaces at the daemon-1.5 reconnect re-handshake; tracked on the ui↔daemon-1.5 spread.
- **Architecture-doc note (§11.4):** the MVP renders the safety surfaces at the Shell level (alert prominent near the banner stack; card in a Shell-level safety host); the full 7-group HIQ host is Phase 8. _(orchestrator writes)_
- **Lessons to bank** _(orchestrator writes — see the recap's proposed framing)_:
  - the **safety/degraded display** convention (third distinct surface; fail-closed display; parked intents disabled-but-present; glyph tracks severity; read-only never silently unexplained).
- **Deferred code-quality nits (rationale-noted, not blocking):** `acknowledgeParked`/`resolutionParked` literal-type comment (L1+L2 mirror); `toBeDisabled`/`toBeTruthy` matcher idioms (jest-dom not wired; precedent-consistent); the empty `.safety-host` div (intentional host).

## How to use what was built

The Shell now accepts an optional `safety?: SafetyState` prop (default = `safetyCleanFixture`, nothing renders). Drive a conflict via `{ conflict: fencingConflictFixture, integrity: null }`; drive an audit-integrity alert via `{ conflict: null, integrity: { source: "integrity", kind: "audit_write_failed" } }` (or `{ source: "action_status", status: "partially_succeeded" }`). The checking banner surfaces when `deriveDegradedState(connection, version)` returns `"checking"` (connection `"connected"`, version `"unknown"`).

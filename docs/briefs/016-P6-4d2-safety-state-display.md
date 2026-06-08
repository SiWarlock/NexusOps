# /tdd brief — safety_state_display

## Feature
The **§17 safety-state DISPLAY surfaces** (§11.4, §15/§17) — two distinct surfaces:
**(A)** a **fencing / hard-conflict card** (a stale-token / lease-expiry mutation was rejected → `ActionFailed(fencing_conflict)` + conflict event) rendered **never-auto-resolved**, and
**(B)** a **fail-closed / audit-integrity alert** carrying the **"unknown outcome" / "partially succeeded" / "rollback failed"** treatments + the general audit-integrity signal.
**DISPLAY only**, fixture-driven (the daemon §17 enforcement is daemon-side, not built here); every **resolution / acknowledge is an INTENT → PARKED** (daemon-1.5), rendered disabled-but-present. This is the **first §15/§17-touching UI slice → `security-reviewer` runs**; a genuine safety-DESIGN fork at Step 2.5 escalates to the lead **before** sign-off.

## Use case + traceability
- **Task ID:** P6.4d-2 (6.4 decomposition: 6.4a/b/c ✅ → 6.4d survival/recovery display ✅ → **6.4d-2 §17 safety-state display** → checking-banner → 6.5 theme pass).
- **Architecture sections it implements:**
  - `ARCHITECTURE.md §11.4` (net-new surfaces — verbatim: *"Fencing/hard-conflict card in the Human Input Queue (never auto-resolved); fail-closed/audit-integrity alert + 'unknown outcome'/'partially succeeded'/'rollback failed' treatments in Audit + HIQ (§15/§17)."*).
  - `ARCHITECTURE.md §17` (failure-mode contract — the SOURCE of the states): the **fencing row** (*"Stale-token write → `ActionFailed(fencing_conflict)` + conflict event; … **hard-conflict card in Human Input Queue (never auto-resolved)**"*); the **event-write-fails row** (*"**Fail closed** … If a side effect already applied but its terminal event can't be written → emit `ActionPartiallySucceeded` best-effort + **hard audit-integrity alert**"*); the **daemon-crash-mid-action row** (*"un-reconcilable → surface **'unknown outcome'**"*); the **corrupt-payload row** (*"corrupt `payload_json` → quarantine + **audit-integrity event**"*).
  - `ARCHITECTURE.md §15` + root `CLAUDE.md` key safety rules — quote **by name, do not paraphrase**: **#5 Fail-closed on audit-write** (*"An audit-required (risk≥1) action aborts if its authoritative event cannot be written (§15/§17)."*) and **#6 Fencing tokens mandatory** (*"A stale-token mutation is rejected → hard-conflict card, never auto-resolved (§5.1/§17)."*). Also **INV-SEC-1 / Single mutator** — resolution is a daemon-side typed Action; the UI **only displays + parks the intent**.
  - `ARCHITECTURE.md §4.2` (renders from projections — forbidden #2: never invent a safety state).
- **Related context:**
  - **6.4d RecoveryBanner** (`ui/src/recovery/`) — the **exact precedent**: a distinct Shell-rendered display surface, fixture-driven, with a **parked affordance** rendered disabled. Mirror it: `ui/src/safety/` parallels `ui/src/recovery/`; the Shell `safety` prop defaults to a clean fixture (nothing renders), exactly like `recovery = recoveryStatusFixture`.
  - **The DegradedBanner** (`ui/src/connection/`, 6.1c) and **RecoveryBanner** (6.4d) are **both distinct** from these safety surfaces — three distinct concerns: transport-degraded vs session-survival vs **safety-state (conflict / audit-integrity)**. Do not conflate.
  - **The frozen contract** (`ui/src/contracts/generated.ts`): the `ActionRequest` enum **already contains** `partially_succeeded` and `rollback_failed`. **REUSE the frozen enum for those two treatments — do NOT re-declare them in a provisional enum** (that would be a Lesson §2 / drift violation). The genuinely net-new states (`unknown_outcome`, the conflict `reason`, the `audit_write_failed`/`corrupt_payload` integrity signal) are NOT frozen → provisional (Lesson §2), banner-marked, reconcile at the daemon §17/survival-schema freeze.
  - **never-color-alone** (§11.6 / forbidden #5): both surfaces use **glyph + label + severity**, never color alone. **a11y** (Lesson §9): any *enabled* new control is keyboard-reachable + in the §9 reachability audit (the parked controls are disabled → out of focus order).
  - Deterministic core = the safety-state → descriptor mappings (pure); render-tested in jsdom.

## Acceptance criteria
- [ ] **Provisional shapes** (`ui/src/contracts/provisional.ts`, banner-marked, Lesson §2): a `FencingConflict` shape (affected `action_request_id` [frozen IdKind] + optional `session_id` + a provisional `ConflictReason` `fencing_conflict | stale_precondition` + a human-readable `summary`) and an audit-integrity shape that **REUSES the frozen `ActionRequest` enum** for `partially_succeeded`/`rollback_failed` and provides provisional members ONLY for the net-new `unknown_outcome` / `audit_write_failed` / `corrupt_payload`. **No provisional enum re-declares a frozen `ActionRequest` value** (load-bearing — Lesson §2).
- [ ] **Pure safety model** (`ui/src/safety/model.ts`): `describeConflict(conflict) → ConflictCardDescriptor` (kind + summary + an explicit **"requires manual resolution — never auto-resolved"** message + `resolutionParked: true`) and `describeAuditIntegrity(state) → AuditIntegrityDescriptor` (the four treatments — partially-succeeded / unknown-outcome / rollback-failed / audit-write-failed(+corrupt-payload) — each → `{ glyph, label, severity, message }`, never color alone). Reuse-frozen-vs-provisional split honored in the input types.
- [ ] **`<HardConflictCard/>`** — renders the fencing/hard-conflict card: the affected action/session, the reason, the **"never auto-resolved"** message, and a **PARKED resolution affordance rendered disabled** (gated on `canSubmitIntent`; intent deferred to daemon-1.5 — present so the UX is complete, NOT wired). **The card offers NO auto-resolve path** (#6). Distinct from DegradedBanner + RecoveryBanner.
- [ ] **`<AuditIntegrityAlert/>`** — renders the fail-closed / audit-integrity alert with the specific treatment (partially-succeeded / unknown-outcome / rollback-failed / audit-write-failed). **Non-dismissible** — fail-closed (#5) means the signal must be seen; any acknowledge is a PARKED intent (daemon-1.5), not a local dismiss. Prominent (a top-level alert, not a quiet inline note).
- [ ] **Wired in the Shell** (mirroring RecoveryBanner): a `safety` prop (default = a clean fixture → renders nothing/non-intrusive); the `<AuditIntegrityAlert/>` renders prominently near the banner stack; the `<HardConflictCard/>` renders in a minimal Shell-level safety host (the full 7-group HIQ is a separate build — Phase 8 / intent-seam — explicitly NOT built here).
- [ ] Renders **only fixture/projection state** (no invented safety state — forbidden #2); every mutation affordance disabled (parked) per forbidden #6 (`canSubmitIntent`); `/preflight` clean (oxlint + typecheck + test:run).
- [ ] **Reachable from** `Shell → <AuditIntegrityAlert/>` (alert) + `Shell → safety host → <HardConflictCard/>` (card). Any enabled new control is in the §9 reachability audit (default: none enabled).

## Wiring / entry point (Step 7.5)
`Shell` renders `<AuditIntegrityAlert/>` (near the DegradedBanner/RecoveryBanner stack) and `<HardConflictCard/>` (in a Shell-level safety host), both driven by the fixture `safety` prop (default clean → non-intrusive, like 6.4d's `recovered`). Confirm both are reachable in the rendered `<Shell/>`; the parked resolution/acknowledge affordances are present-but-disabled (tracked, not a false wire — like 6.1c's `canSubmitIntent` + 6.4d's parked "Restart session"). The full HIQ host is explicitly deferred.

## Files expected to touch
**New:**
- `ui/src/safety/model.ts` — pure `describeConflict` + `describeAuditIntegrity`
- `ui/src/safety/HardConflictCard.tsx` — the fencing/hard-conflict card (#6, never-auto-resolved, parked resolution)
- `ui/src/safety/AuditIntegrityAlert.tsx` — the fail-closed/audit-integrity alert (#5, non-dismissible)
- `ui/src/safety/fixtures.ts` — fixture safety states (default = clean)
- `ui/src/safety/model.test.ts`, `ui/src/safety/HardConflictCard.test.tsx`, `ui/src/safety/AuditIntegrityAlert.test.tsx`

**Modified:**
- `ui/src/contracts/provisional.ts` — add the provisional `FencingConflict`/`ConflictReason` + audit-integrity shapes (banner-marked; reuse frozen `ActionRequest`)
- `ui/src/shell/Shell.tsx` — render `<AuditIntegrityAlert/>` + `<HardConflictCard/>` via a fixture-defaulted `safety` prop
- `ui/src/a11y/reachability.test.tsx` — extend ONLY if an enabled new control lands (default: not needed — parked controls are disabled)

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
**`safety/model.test.ts`:**
1. **`conflict_to_card_descriptor`** — a `fencing_conflict` → a card descriptor with the affected refs + an explicit **"never auto-resolved"** message + `resolutionParked: true`. **[load-bearing — #6]**
   - Asserts: the descriptor message states manual-resolution / never-auto-resolved; no auto-resolve field is produced. Why: §17 fencing row + safety rule #6.
2. **`audit_integrity_reuses_frozen_action_status`** — `partially_succeeded` + `rollback_failed` inputs map to their treatments **using the frozen `ActionRequest` enum values** (imported from `contracts/index`, not a provisional re-declaration). **[load-bearing — Lesson §2 / no-drift]**
   - Asserts: the model accepts the frozen-enum literals; a test references `ActionRequest.options` to prove reuse. Why: frozen `ActionRequest` already carries these.
3. **`audit_integrity_provisional_only_for_net_new`** — `unknown_outcome` / `audit_write_failed` / `corrupt_payload` → their treatments via the provisional shape; each descriptor carries `glyph + label + severity` (never color alone). **[never-color-alone — forbidden #5]**
   - Asserts: each net-new state → a distinct non-color descriptor. Why: §17 daemon-crash/event-write/corrupt rows + §11.4 named treatments.

**`safety/HardConflictCard.test.tsx` (jsdom):**
4. **`renders_conflict_with_never_auto_resolved_and_parked_resolution`** — the card shows the conflict + the "never auto-resolved" message + a **disabled/parked** resolution control (present, not wired — gated). **[load-bearing — #6 + parked-intent discipline]**
   - Asserts: a disabled resolution affordance exists; **no enabled auto-resolve control** is in the DOM.
5. **`conflict_card_distinct_from_degraded_and_recovery_banners`** — the hard-conflict card is a distinct surface (not the transport DegradedBanner, not the RecoveryBanner). **[don't-conflate]**

**`safety/AuditIntegrityAlert.test.tsx` (jsdom):**
6. **`renders_each_audit_integrity_treatment`** — partially-succeeded / unknown-outcome / rollback-failed / audit-write-failed each render their glyph+label treatment. **[§11.4 named treatments]**
7. **`audit_integrity_alert_is_non_dismissible`** — the alert exposes **no local-dismiss control**; any acknowledge is rendered disabled/parked (intent). **[load-bearing — #5 fail-closed]**
   - Asserts: no enabled dismiss button; the signal persists.

**`shell/Shell.test.tsx` (extend):**
8. **`shell_renders_safety_surfaces_clean_by_default`** — default `safety` fixture → neither surface renders intrusively (non-intrusive, like 6.4d `recovered`); a conflict/integrity fixture → the surfaces appear + are reachable.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none frozen.** `FencingConflict`/`ConflictReason` + the audit-integrity provisional shape are **provisional** (Lesson §2); the two reused treatments delegate to the **frozen `ActionRequest`** enum (no new frozen field). Add them to the **Carry-forward provisional→generated reconcile spread** at Step 9 (alongside the Usage + Recovery shapes — reconcile at the daemon §17/survival-schema freeze).
- **Orchestrator doc rows to write hot (Step 9 routing):** none new (no `shared/` crate / generated-enum / Appendix-A field added or renamed). If the safety-display surfaces warrant an `ARCHITECTURE.md §11.4` clarification (e.g. "the MVP renders these as Shell-level surfaces; the full HIQ host is Phase 8"), flag it as an **architecture-doc note** at Step 9.

> **Implementer never edits `ui/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, or `ui/LESSONS.md`** — flag at Step 9 categorized; orchestrator writes hot.

## Things to flag at Step 2.5
Open design questions — pre-loaded with default votes. **Q3 + Q4 are safety-load-bearing** (determined by invariants #5/#6); a deviation from their defaults is a **lead escalation, not an orchestrator-only call**.

1. **Reuse-frozen-vs-provisional modeling for the audit-integrity input.** Default: a **discriminated union** — `{ source: "action_status", status }` reusing the frozen `ActionRequest` literals (narrowed to `partially_succeeded | rollback_failed`) **+** `{ source: "integrity", kind }` with a provisional enum of ONLY `unknown_outcome | audit_write_failed | corrupt_payload`. My default vote: **the discriminated union** — it keeps the frozen treatments drift-pinned to the frozen enum and provisional only for the net-new, satisfying Lesson §2 with zero duplicated values. (Alternative: one flat provisional enum — **rejected**, it duplicates frozen `partially_succeeded`/`rollback_failed`.)
2. **Render location given the HIQ isn't built.** §11.4 places the card in the HIQ + the alert in Audit + HIQ, but the full 7-group HIQ is a parked/Phase-8 build. Default: render **both as Shell-level display surfaces this slice** (alert near the banner stack; card in a minimal Shell safety host), mirroring the RecoveryBanner precedent; the full HIQ absorbs them later. My default vote: **Shell-level surfaces now** — keeps the slice daemon-independent + display-only, consistent with 6.4d.
3. **[SAFETY] Audit-integrity alert dismissibility (#5 fail-closed).** Default: **non-dismissible** — fail-closed means the safety signal must be SEEN; any acknowledge is a PARKED intent (daemon-1.5), never a local dismiss. My default vote: **non-dismissible / acknowledge-parked**. A local-dismiss would undercut #5 → **if you disagree, escalate to the lead before sign-off.**
4. **[SAFETY] Hard-conflict card resolution affordance (#6 never-auto-resolved).** Default: the card offers **NO auto-resolve path**; any manual-resolution control is rendered **disabled/parked** (intent → daemon-1.5) + an explicit "never auto-resolved" message. My default vote: **parked-disabled resolution + explicit never-auto-resolved copy** — surfacing an auto-resolve affordance would breach #6 → **if you want any enabled resolution here, escalate to the lead before sign-off.**
5. **Commit count — 1 vs 2.** Default: **2** (each safety invariant its own bisectable commit — see "Estimated commit count"). If the two surfaces prove inseparable (shared model dominates), 1 is acceptable — flag it.

## Dependencies + sequencing
- **Depends on:** 6.1c connection model + 6.4d RecoveryBanner (distinct-from precedent + the Shell fixture-prop pattern); the frozen `ActionRequest` enum (`contracts/generated.ts`); Lesson §2 (provisional) + §9 (a11y) + §7 (multi-commit driving). **No daemon dependency** (fixture-driven; real safety state integrates at the daemon §17 freeze).
- **Blocks:** the checking-banner sub-slice (next), then 6.5 theme pass; the full HIQ surface (Phase 8 / intent-seam) absorbs these surfaces.
- **Note:** unstyled until the 6.5 theme pass (accepted).

## Estimated commit count
**2** — a **safety slice**: each of the two distinct safety invariants gets its **own bisectable commit** (root `CLAUDE.md` "every safety-critical slice gets its own commit"; never bundle a safety pin):
- **L1 — fencing / hard-conflict card (#6):** `safety/model.ts` (`describeConflict`) + `FencingConflict`/`ConflictReason` provisional shapes + `<HardConflictCard/>` + its Shell wiring + tests. Self-contained + reachable.
- **L2 — fail-closed / audit-integrity alert (#5):** `describeAuditIntegrity` + the audit-integrity provisional shape (frozen-reuse) + `<AuditIntegrityAlert/>` + its Shell wiring + tests. Self-contained + reachable.

**Multi-commit slice** → the implementer idles after the L1 commit; the **orchestrator drives layer→layer** (one wake per layer at the commit boundary — Lesson §7). **`security-reviewer` runs** (first §15/§17-touching UI slice — `invariant` policy); **`code-quality-reviewer` every-slice**. Run `security-reviewer` on **each** layer's diff (both touch a safety invariant).

## Lessons-logged candidates anticipated
- **Convention candidate** — safety-state display is a THIRD distinct surface (conflict / audit-integrity) beyond transport-degraded + session-survival; safety signals render fail-closed (audit-integrity non-dismissible; conflict never-auto-resolved); all resolution is a parked daemon-1.5 intent rendered disabled-but-present. Candidate if it recurs (likely with the full HIQ).
- **Future TODO — provisional reconcile** — `FencingConflict`/`ConflictReason` + the audit-integrity provisional shape → generated at the daemon §17/survival-schema freeze (added to the spread); the resolution/acknowledge intents wire at daemon-1.5; the full 7-group HIQ host lands Phase 8.
- **Architecture-doc note candidate** — §11.4: the MVP renders the safety surfaces at the Shell level (the full HIQ host is Phase 8) — flag at Step 9 if worth pinning.

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd safety_state_display`.
1. Read this brief; **Q1 (reuse-frozen modeling), Q3 + Q4 (the two safety defaults)** are the ones to confirm at Step 2.5. Q3/Q4 deviations escalate to the lead before sign-off.
2. Step 2.5 — test-design write-up (`Asserts: <invariant> (§anchor)` per test) → wait for the magic-words reply → GREEN. (Heavier safety review this slice.)
3. Step 7.5 — name `Shell → AuditIntegrityAlert` + `Shell → safety host → HardConflictCard`.
4. Step 8 — `security-reviewer` on each layer diff (#5 + #6) + `code-quality-reviewer`.
5. Step 9 — commit-message-first (per layer); then `TaskUpdate` the slice task → completed + wake me.

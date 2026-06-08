# ui-002 — Phase 6: Graph · Sessions · a11y · Usage · Settings · Survival · TopBar-nav

- **Date:** 2026-06-08
- **Phase:** 6 (Frontend shell & projection-driven UI) — round 2 (`track/ui`)
- **Predecessor:** [ui-001](ui-001-2026-06-08-phase6-shell-status-command-center.md) (6.1 + 6.2 + 6.3a)
- **Successor:** [ui-003](ui-003-2026-06-08-safety-state-display-and-checking-banner.md) (6.4d-2 §17 safety-state display + the checking-banner — Phase-6 logic finish)
- **Round commits:** `c420cd7` `885cc0d` (6.3b L1/L2) · `23fbda3` (6.3c) · `f70757e` (6.4a) · `db9b89b` (6.4b) · `765923f` (6.4c) · `290381a` (6.4d) · `823d16e` (6.4e) — **7 slices, 116 tests green**
- **Builds on:** frozen `shared/` 0.5.0 + `MockGatewayPort` + fixtures + `NexusOps-ui-kit`. Real `UdsGatewayPort` integrates at daemon-1.5.

## Why this session existed

Continue Phase 6 from the ui-001 round boundary (6.1/6.2/6.3a sealed). Mid-round, **Decision C** (human, via lead) **parked 6.3d/6.3e** until the daemon freezes the mutation/intent + Terminal-Channel contracts (6.3d's permission card is the UI's first mutation path — build once against the real contract), and **reordered to the daemon-independent 6.4 work**. A theme **Finding** (the Graphite Arc layer was never built — app is functionally-correct but unstyled) became a dedicated **6.5 theme pass** at the end. A nav **Finding** surfaced in 6.4c (duplicate Settings) was human-confirmed and resolved in 6.4e.

## What was built

### 6.3b — Project Graph + a11y list/table fallback (2 commits, layer-driven — Lesson §7)
- **L1 (`c420cd7`):** **New** `projections/items.ts` (`ProjectionItem` + `toSessionItems`/`toPrItems`/`toApprovalItems` mappers) + test. **Mod** Shell + CommandCenter + Sidebar to route through the mappers (no inline re-map) and namespace every `data-item-id` as `<namespace>:<id>` (Lesson §8). `CommandItem`/`SidebarItem` → aliases of `ProjectionItem`.
- **L2 (`885cc0d`):** **New** `views/graph/model.ts` (pure `buildProjectGraph`: project-scoped session/PR nodes via the L1 mappers + `contains` edges; attention/labels from the descriptor table) + `ProjectGraph.tsx` (Graph|List toggle + the §11.6-equivalent semantic table fallback — one row per node, every edge represented, accessible names carry type+status+attention) + tests. **Mod** Shell content-view switch (mounts `<ProjectGraph/>`).

### 6.3c — Sessions dense sortable table (`23fbda3`)
- **New** `views/sessions/model.ts` (`buildSessionRows` join+enrich; `sortSessionRows` pure/stable, attention-desc default, `"en"`-pinned collation, direction-independent id tiebreak) + `SessionsTable.tsx` (dense semantic table, keyboard-operable `<button>`-in-`<th>` sortable headers with state-driven `aria-sort`, `Session:<id>` locators, empty state) + tests. **Mod** Shell (3rd content-view).

### 6.4a — a11y MUSTs: focus-visible ring + reduced-motion (`f70757e`)
- **New** `a11y/focus.css` (one global `:where(...):focus-visible` ring via kit tokens) + `useReducedMotion.ts` (matchMedia, change-subscribed, defaults false) + tests + `reachability.test.tsx` (multi-view keyboard-reachability audit — the §9 merge-gate net). **Mod** `main.tsx` (import focus.css). Kit `motion.css` reduced-motion guard verified wired.

### 6.4b — Usage dashboard (`db9b89b`)
- **New** `views/usage/model.ts` (`buildUsageRows`: Codex/null context → literal `"unknown"` [forbidden #4]; accuracy label in every variant; `"unknown"` not 0 for unavailable; `creditPoolState` thresholds) + `UsageDashboard.tsx` (kit `UsageMeter` wrapped with a **non-color** threshold channel [forbidden #5]) + `fixtures/proj_usage.ts` + tests. **Mod** `provisional.ts` (provisional `UsageRow`/`MetricQuality`/`Harness`/`CreditPool`/`UsageProjectionPage` + registry), `boundary.ts`, `mock.ts`, Shell (interim 4th view).

### 6.4c — Settings tabbed surface + Usage relocation (`765923f`)
- **New** `views/settings/tabs.ts` (pure tab model) + `Settings.tsx` (ARIA tablist; all 5 tabpanels render present-but-hidden so every `aria-controls` resolves; `tabIndex=0` panels; no roving) + tests. Usage tab mounts the relocated `<UsageDashboard/>`; daemon-coupled tabs render honest "pending [Phase X]" stubs; Execution Profiles 0.5b-gated. **Mod** Shell (view-switch interim "Usage" → "Settings").

### 6.4d — Survival/recovery display (`290381a`)
- **New** `recovery/{model.ts,RecoveryBanner.tsx,fixtures.ts}` + tests. `describeRecovery` (recovered = non-intrusive; recovery_failed → affected sessions + a parked-disabled "Restart session"); `describeResumeMode` (glyph+label). `<RecoveryBanner/>` distinct from the transport `DegradedBanner`. **Mod** `provisional.ts` (`RecoveryState`/`ResumeMode`/`RecoveryStatus` + optional `resume_mode` on `SessionRow`), sessions model+table (resume-mode "Recovery" column), `proj_session.ts`, Shell (`recovery` prop + render `<RecoveryBanner/>`).

### 6.4e — TopBar §11.2 nav reconcile + accessible-names (`823d16e`)
- **Mod** `TopBar.tsx` (wired the Settings placeholder → `onOpenSettings`; **real accessible names** on back/forward via a visually-hidden child label inside the closed-prop kit Button — not a wrapper aria-label), `Shell.tsx` (pass `onOpenSettings`; **dropped "Settings" from the view-switch** — content surfaces only), `reachability.test.tsx` (reach Settings via the TopBar). **New** `TopBar.test.tsx`. Resolves the 6.4c nav Finding.

## Decisions made

- **`ProjectionItem` mappers are the single source** for the `{id,label,machine,status}` shape; `data-item-id` namespaced `<namespace>:<id>` on every emitter (Lesson §8).
- **Graph/Sessions/Usage/Recovery models are UI render policy** (like the 6.2a attention table), NOT frozen contracts; edges/joins are pure §4.2 derivations.
- **forbidden #4** (Codex context → `"unknown"`, never a number — even a stray one; a real 0 stays `"0%"`) and **forbidden #5** (credit-pool + resume-mode on a non-color glyph+label channel) pinned by load-bearing tests.
- **a11y foundation** (Lesson §9): one global `:where(...):focus-visible` ring; reduced-motion = the kit's global guard + `useReducedMotion()`; the reachability audit is the comprehensive §11.6 merge-gate net swept across every content view.
- **Settings tabs: no roving tabindex** (all `tabIndex=0`) — keeps tabs keyboard-reachable AND the §9 audit (`tabIndex>=0`) green; arrow-roving deferred. **All 5 tabpanels render** (hidden inactive) so every `aria-controls` resolves (WAI-ARIA).
- **§11.2 nav model** (human-confirmed): view-switch = content surfaces; **Settings reached via the TopBar**. The accessible NAME of a closed-prop kit control comes from a **visually-hidden child inside it**, not a wrapper aria-label (Lesson §6 refinement).
- **Provisional shapes** (Lesson §2, banner-marked): all Usage + Recovery shapes are UI-local, reconcile at the daemon usage/survival/object-schema freeze.

## Decisions explicitly NOT made (deferred)

- **6.3d (Session Terminal) + 6.3e (Code/Diff)** — PARKED (Decision C) until the daemon freezes the mutation/intent + Terminal-Channel contracts (6.3d's permission card = the UI's first mutation path).
- **Intent controls** — all parked for the daemon-1.5 intent seam: the recovery "Restart session" (rendered disabled), the Settings intent controls (notification toggles / save-as-policy / profile changes).
- **ExecutionProfile tab** — 0.5b-gated (rendered "pending", no enum binding).
- **Sidebar resume-mode indicator** — deferred (would thread a session-specific field through the shared `ProjectionItem` §8).
- **Arrow-key roving + the `role=tabpanel`/roving-`tabindex=-1` audit refinement**, the **`textarea` selector** in focus.css, the **TopBar back/forward history nav**, the **Brain TopBar trigger** (Phase 8), and the **global sr-only utility** — all a11y-polish/feature follow-ups.
- **6.5 theme pass** (Graphite Arc) — dedicated end-of-phase slice with an automated browser visual gate vs `NexusOps-ui-kit/ui_kits/control-plane/index.html` (Lesson §10 "green ≠ looks right"). Surfaces unstyled until then (accepted).

## TDD compliance

**Clean.** Every slice ran RED → Step-2.5 (orchestrator-reviewed) → GREEN. No implementation-before-test. Test *updates* this round (6.4d sf1 VM shape gaining `resumeMode`; the 6.4c then 6.4e Shell-test replacements as the Settings entry point moved view-switch → TopBar) were legitimate contract-evolution edits driven by approved design changes, not back-fills. code-quality-reviewer ran every slice; security-reviewer correctly skipped (no §15 invariant touched this round — the §17 safety-state display is 6.4d-2).

## Cross-doc invariant audit

**No frozen contract field changed.** All new shapes are **provisional, banner-marked, UI-local** (`provisional.ts`): `UsageRow`/`MetricQuality`/`Harness`/`CreditPool`/`UsageProjectionPage` (6.4b) and `RecoveryState`/`ResumeMode`/`RecoveryStatus` + the optional `resume_mode` on the provisional `SessionRow` (6.4d). No `shared/` crate / generated-enum / cross-doc Appendix-A field was added/removed/renamed. The orchestrator routed all provisional shapes into the **provisional→generated reconcile spread** (open follow-up below). No `ARCHITECTURE.md` field-level edit was required (the §11.2 nav-note + the credit-pool-thresholds note are orchestrator-owned arch-notes, already routed).

## Reachability (Step 7.5 — carried forward; relocations noted)

- **Project Graph:** `Shell → view-switch (Graph) → <ProjectGraph/> → gateway-client`. Intact.
- **Sessions table:** `Shell → view-switch (Sessions) → <SessionsTable/> → gateway-client`. Intact.
- **Focus ring:** `main.tsx → a11y/focus.css` (global, every render, vite-bundled). The multi-view reachability audit drives the real `<Shell/>` across all content views.
- **Usage dashboard:** **relocated** — final path `Shell → TopBar (Settings) → <Settings/> → Usage tab → <UsageDashboard/>` (6.4c moved it into Settings; 6.4e moved the Settings entry to the TopBar). Confirmed by `settings_still_reachable_and_functional`.
- **Settings:** **relocated** — final path `Shell → TopBar (Settings) → <Settings/>` (6.4e dropped the view-switch Settings). Confirmed by `topbar_settings_opens_settings_view`.
- **Recovery banner:** `Shell → <RecoveryBanner/>` (driven by the `recovery` prop; default fixture `recovered` → non-intrusive). **Resume-mode indicator:** per-row in the Sessions table.
- **Tested-but-not-yet-wired (tracked, not silent gaps):** `useReducedMotion()` (no animated consumer yet); the recovery production-state source (defaults to `recovered` until the daemon-survival integration); the parked-disabled "Restart session"; the TopBar back/forward buttons (accessible-named but no `onClick`).

## Open follow-ups (Step-9 items — routed hot by the orchestrator; captured here)

1. **Provisional→generated reconcile** (at the daemon freeze): Usage shapes (`UsageRow`/`MetricQuality`/`Harness`/`CreditPool`), Recovery shapes (`RecoveryState`/`ResumeMode`/`RecoveryStatus` + `resume_mode`), the earlier `ProjectionItem.machine`/`status` narrow + object-schema/`ProjectionDelta` reconcile.
2. **Arch-doc note** (orchestrator-owned): credit-pool thresholds are provisional (§11.4 pins none); the §11.2 nav-model note (already written).
3. **Decision-C parks:** 6.3d (Terminal) + 6.3e (Code/Diff) + the intent seam — gated on daemon mutation/intent + Terminal-Channel contracts. **Next ui slice = 6.4d-2** (§17 safety-state display: fencing/hard-conflict + fail-closed/audit-integrity — first §15/§17-touching UI slice → security-reviewer + heavier safety Step-2.5), then the checking-banner.
4. **a11y polish:** arrow-key roving + the `role=tabpanel`/roving-`tabindex` audit refinement; `textarea` in the focus.css selector; the sidebar resume-mode indicator (§8).
5. **TopBar follow-ups:** wire back/forward history nav (named-but-inert now); Brain trigger (Phase 8); global sr-only utility (6.5).
6. **6.5 theme pass** (Graphite Arc) + the automated browser visual gate (Lesson §10).
7. **Findings:** nav Finding — **RESOLVED** (6.4e). Theme Finding — tracked as 6.5.

## How to use what was built

The shell's content-view switch (CC/Graph/Sessions) is the `contentView` seam any future content surface extends. Settings (incl. the Usage dashboard) is reached from the **TopBar**. New projection-derived views import the `items.ts` mappers + namespace `data-item-id`; new interactive controls inherit the global focus ring + must stay in the §9 reachability audit's swept set. Provisional shapes live in `contracts/provisional.ts` (banner-marked) until the daemon freezes their schemas.

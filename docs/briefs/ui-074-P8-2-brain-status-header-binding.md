# /tdd brief — brain_status_header_binding

## Feature
Bind the Project Brain drawer + page **header** to a **ProjectBrain §5.1 status** via a new **FakeBrain provider/seam** (the swap-point the live daemon 8.1 source replaces later), plus the **honest-degraded §13.1 states** (Brain absent/stale/error → a distinct degraded surface; the platform NEVER hard-depends on or blocks for Brain). Shell-binding only — the rich answer/evidence/plan CONTENT stays the existing fixture (deferred until `brain/` output is observable); Run-via-Gateway stays disabled.

## Use case + traceability
- **Task ID:** P8.2 (the Brain drawer shell-binding half of 8.2; Phase 8 un-deferred 2026-06-21, user-approved; successor to ui-073)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.5` (Brain UI header — live ProjectBrain status + grounded-at/staleness + privacy/transport), `§13.1` (Brain seam — degrades gracefully when absent/stale via `brain_status_reported_at`; platform never hard-depends), `§5.1` (the frozen `ProjectBrain` 10-state status machine)
- **Related context:**
  - **Verify-before-build findings (already done — do NOT rebuild):** the **TopBar Brain trigger is wired** (`TopBar` Brain button → `onOpenBrain` → `Shell:623` `setOverlay({kind:"brain"})` → `BrainDrawer`); the **EvidenceChip exists** (kit component `@ui-kit/objects/EvidenceChip`, used in `BrainPage:98`); the **ProjectBrain status→descriptor mapping exists** (`status/descriptors.ts:91` — 10 states → `{attentionRank, visualKind}`).
  - The genuine gap: `BrainPage`/`BrainDrawer` are **static kit fixtures** — the header shows a placeholder Badge *"display fixture — sidecar contract Phase 8"* (`BrainPage:182`), **nothing bound to a ProjectBrain status seam**. There is **NO ProjectBrain projection** in the frozen ProjectionName enum (its status is daemon-cached §13.1, served by the not-yet-built 8.1 seam) → **FakeBrain-fed**, exposed-ahead.
  - Patterns to mirror: `shell/active-project.ts` + `connection/read-only.ts` (the LESSON [[13]] pure-model + thin-context/hook provider); `status/StatusPill.tsx` (the descriptor-bound status wrapper, LESSON [[6]]); `connection/DegradedBanner.tsx` (a degraded-surface precedent — but the Brain-degraded surface is DISTINCT, never conflated, LESSON [[11]]).
  - LESSONS [[6]] (descriptor-bound status wrappers), [[11]] (distinct degraded surfaces, fail-closed), [[13]] (UI provider over frozen state — no daemon dep, no canSubmitIntent gate), forbidden #4 (no fabricated metric)/#5 (never color alone).

## Acceptance criteria (what "done" means)
- [ ] A **`BrainStatusProvider`/`useBrainStatus`** (mirror `active-project.ts`) exposes a **FakeBrain** value: a frozen `ProjectBrain` status + `grounded_at` (the `brain_status_reported_at` timestamp) + an `absent` flag. Default-fed by a fixture; the live daemon 8.1 source swaps in later (single swap-point).
- [ ] The **Brain drawer + page headers render the ProjectBrain status** via `StatusPill` + the existing `ProjectBrain` descriptor (glyph+label, **never color alone** — forbidden #5) + a **grounded-at/staleness** indicator, replacing the static *"display fixture"* Badge.
- [ ] **Honest-degraded §13.1:** a degraded ProjectBrain status (`not_configured`/`stale`/`error`/`partial_index`/`graph_degraded`/`reindex_required`) renders a **DISTINCT** Brain-degraded surface (LESSON [[11]] — not the connection `DegradedBanner`, not conflated); a healthy status (`ready`/`indexing`/`transcript_ingestion_active`) renders no degraded surface.
- [ ] **Platform never hard-depends / never blocks (§13.1):** the drawer/page content (the existing fixture thread + modes) **still renders regardless** of Brain status — the degraded surface is ADDITIVE, never replaces/blocks/throws. Pin: an `absent`/`error` Brain → the drawer still renders its content + an honest degraded indicator.
- [ ] **Staleness is daemon-reported, not client-computed** — render the daemon's `stale` status value + the `grounded_at` timestamp; the UI does NOT invent a staleness threshold (forbidden #4 — no fabricated metric; the daemon owns §13.1 status).
- [ ] **No fabrication when status absent** — Brain absent → `not_configured`/honest-absent, never a faked "ready".
- [ ] Run-via-Gateway stays **disabled** (unchanged — the Brain PROPOSES only, INV-SEC-1 #10; A1 does not touch it). EvidenceChip real-freshness binding stays deferred (rich content).
- [ ] Reachable from the TopBar Brain trigger (drawer) + the content-view Brain page; all unit tests pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
`BrainStatusProvider` wraps the Brain surfaces at the **Shell root** (mirror `ActiveProjectProvider`/`ReadOnlyProvider` placement in `Shell.tsx`); `BrainDrawer` + `BrainPage` headers consume `useBrainStatus`. Reachable via the already-wired TopBar trigger (`overlay.kind==="brain"` → `BrainDrawer`) + the content-view (`contentView==="brain"` → `BrainPage`). The FakeBrain default feeds it now; the live daemon 8.1 source replaces the provider's source later (the documented swap-point).

## Files expected to touch
**New:**
- `ui/src/views/brain/brain-status.ts` — the FakeBrain model (`deriveBrainHeader(status, grounded_at, now?)` pure mapping + the degraded-status set) + `BrainStatusProvider`/`useBrainStatus`.
- `ui/src/views/brain/brain-status.test.ts` — the model/degraded/honest-absent pins.
- `ui/src/views/brain/BrainStatusHeader.tsx` (+ `.test.tsx`) — the shared header surface (pill + grounded-at/staleness + degraded indicator) reused by the page + drawer.

**Modified:**
- `ui/src/views/brain/BrainPage.tsx` — header: replace the *"display fixture"* Badge with `<BrainStatusHeader>`.
- `ui/src/overlays/BrainDrawer.tsx` — header: add the status + the degraded surface.
- `ui/src/shell/Shell.tsx` — wrap with `BrainStatusProvider` (root).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
`brain-status.test.ts`:
1. **`derive_header_maps_status_to_descriptor`** — Asserts: `deriveBrainHeader(ready,…)` → the ProjectBrain descriptor's kind/label; never a recomputed value. Why: §5.1/LESSON [[6]].
2. **`degraded_statuses_flag_degraded`** — Asserts: each of `{not_configured,stale,error,partial_index,graph_degraded,reindex_required}` → `degraded:true`; `{ready,indexing,transcript_ingestion_active}` → `degraded:false`. Why: §13.1 (drift-pin the set against the generated `ProjectBrain.options` so a new value is forced to classify).
3. **`absent_brain_is_not_configured_not_faked`** — Asserts: `absent` → `not_configured`/honest, never `ready`. Why: §13.1 / forbidden #4.
4. **`staleness_is_reported_not_computed`** — Asserts: `grounded_at` rendered from the reported timestamp; no client-side threshold invents `stale`. Why: §13.1/forbidden #4.

`BrainStatusHeader.test.tsx`:
5. **`header_renders_status_pill_glyph_and_label`** — Asserts: the pill carries glyph+label (never color alone). Why: forbidden #5 / LESSON [[6]].
6. **`degraded_status_renders_distinct_surface`** — Asserts: a degraded status → a Brain-degraded surface with a distinct testid (NOT the connection `DegradedBanner`). Why: LESSON [[11]] (distinct, never conflated).
7. **`content_still_renders_when_degraded_never_blocks`** — Asserts: with an `error`/`absent` Brain, the header + the host content both render (the degraded surface is additive, no throw/block). Why: §13.1 (platform never hard-depends).
8. **`healthy_status_no_degraded_surface`** — Asserts: `ready` → no degraded surface. Why: §13.1 honest (degraded only when degraded).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** — consumes the FROZEN `ProjectBrain` §5.1 enum + the existing descriptor. No new shared model, no contract bump/regen. The FakeBrain value shape is a **UI-local provisional** (LESSON [[2]]/[[13]]) — when the live daemon source lands it reconciles to whatever §13.1 serves; pin its shape with a local test, not a cross-track-seam snapshot (no frozen contract touched here).
- **Orchestrator doc rows to write hot (Step 9):** a `ui/CLAUDE.md` cross-doc/notes touch (the Brain status-header binding + the FakeBrain seam swap-point) + a LESSON candidate. Implementer FLAGS; orchestrator writes hot + retires the stale plan:707 *"Brain TopBar inert"* note.

## Things to flag at Step 2.5
1. **FakeBrain provider scope.** Just the **status seam** (status + grounded_at + absent) — OR a broader FakeBrain also stubbing thread/evidence. **My default vote: status seam ONLY** — the rich content (thread/evidence) stays the existing fixture (deferred per the lead's "defer content until brain/ output observable"); the provider is the single swap-point for the live 8.1 source.
2. **Degraded-surface reuse.** Reuse `connection/DegradedBanner` — OR a distinct Brain-degraded surface. **My default vote: DISTINCT Brain surface** (LESSON [[11]] — every degraded state renders its own surface, never conflated; connection-degraded ≠ Brain-degraded). Reuse the severity/glyph helpers, not the component.
3. **Staleness source.** Daemon-reported `stale` status + `grounded_at` timestamp (no client threshold) — OR client-computed staleness. **My default vote: daemon-reported** (§13.1 daemon owns status; forbidden #4 — don't invent a metric). The FakeBrain provides both.
4. **Provider placement.** Shell-root (both drawer + page share it) — OR per-surface. **My default vote: Shell-root** (single source; mirror `ActiveProjectProvider`/`ReadOnlyProvider`).
5. **Header in BOTH drawer + page, or shared component?** **My default vote: a shared `BrainStatusHeader`** reused by both (one source for the status surface; the drawer + page headers differ only in chrome).

## Dependencies + sequencing
- **Depends on:** ui-073 (the Phase-8 UI arc head, landed `15fad1f`); the frozen `ProjectBrain` §5.1 enum + descriptor (landed). NON-cat-1 (read/display + a UI provider; no mutation/INV-SEC-1 surface; Run-via-Gateway untouched-disabled).
- **Blocks:** the live daemon 8.1 source swap (replaces the FakeBrain provider's source); the rich Brain content panes (answer/evidence/plan — deferred until `brain/` output observable); the per-answer confidence + the EvidenceChip real-freshness binding.

## Estimated commit count
**1–2.** The FakeBrain provider/model + the shared `BrainStatusHeader` + the header bindings — one logical shell-binding unit; may split into (a) the provider/model + (b) the header binding + degraded surface if sizable (orchestrator drives layer→layer, LESSON [[7]]). **NON-cat-1 → code-quality-reviewer (every-slice); security-reviewer NOT required** (no mutation/INV-SEC-1 invariant touched — read/display only; flag at Step 9 if the degraded logic surfaces anything mutation-adjacent).

## Lessons-logged candidates anticipated
- **Convention candidate** — "A Brain shell-binding is a FakeBrain provider (LESSON [[13]] pattern) exposing the frozen ProjectBrain §5.1 status as the single swap-point for the live daemon 8.1 source; the header renders it via the descriptor-bound StatusPill; §13.1 honest-degraded is a DISTINCT additive surface that never blocks the platform; staleness is daemon-reported, never client-invented."
- **Architecture-doc note** — §11.5/§13.1 as-built: the Brain header status + grounded-at/staleness binds a FakeBrain seam pending the daemon 8.1 ProjectBrain status source.
- **Future TODO (8.1/8.2)** — the live daemon ProjectBrain status source (§13.1 — a projection or a project-row field) replaces the FakeBrain provider; the rich content panes + EvidenceChip real-freshness.

## How to invoke
1. Read this brief end-to-end — especially the verify-before-build findings (don't rebuild the TopBar trigger / EvidenceChip) + the 5 Step-2.5 questions.
2. Run `/tdd brain_status_header_binding`.
3. Step 0 (Restate) — confirm: bind the Brain header to a ProjectBrain §5.1 status via a FakeBrain seam + honest-degraded §13.1, shell-binding only, content deferred.
4. Step 1 (Identify files) — confirm against Files expected to touch.
5. Step 2.5 — ping back with answers (or take defaults). NON-cat-1 → code-quality-reviewer only (no security-reviewer unless Step 9 surfaces an invariant touch).
6. Step 9 — surface anything beyond the anticipated lessons-logged candidates.

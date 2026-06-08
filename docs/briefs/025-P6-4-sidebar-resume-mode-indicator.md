# /tdd brief — sidebar_resume_mode_indicator

## Feature
Surface a session's **resume-mode indicator** (▶ Resumed (live) / ⟳ Replayed (relaunched)) on its **sidebar** item — reusing the existing `describeResumeMode` descriptor — by passing a **side map keyed by session id** to the Sidebar, **without** widening the shared `ProjectionItem` (the Lesson §8 discipline).

## Use case + traceability
- **Task ID:** P6.4 (sidebar resume-mode indicator — §8-respecting follow-up; origin 2026-06-08 P6.4d)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.4` (survival/recovery UX — per-session resumed-vs-replayed indicator), `§8` (survival/failure-mode), `§11.6` (never color alone — glyph + label).
- **Related context:** Polish round (ui-006) slice 4. 6.4d (`290381a`) put the resumed/replayed indicator on the **Sessions table** (`SessionsTable.tsx` → `ResumeModeBadge` via `describeResumeMode`, `recovery/model.ts:56`); the **sidebar** was deferred to avoid threading session-specific `resume_mode` through the **shared `ProjectionItem`** (`projections/items.ts` — Lesson §8: the item mappers are the single source for `{id,label,machine,status}`; no inline re-map; no per-entity fields). The Shell already has the data: `data.sessions` carry `resume_mode?: ResumeMode` (provisional shape), but `toSessionItems` intentionally drops it. The §8-respecting fix is a **side map** (`Record<sessionId, ResumeMode>`) passed alongside the sidebar items — NOT a new field on `ProjectionItem`. The provisional `ResumeMode`/`resume_mode` shapes already reconcile under the existing Carry-forward "survival/recovery provisional shapes → generated" spread (recovery-shapes origin P6.4d) — **this slice adds no new reconcile item** (it's another consumer of the same provisional source).

## Acceptance criteria (what "done" means)
- [ ] A pure `resumeModesBySessionId(sessions)` (in `ui/src/recovery/model.ts`) returns `Record<string, ResumeMode>` containing **only** sessions whose `resume_mode` is defined (fresh-started sessions are absent — no entry, no indicator).
- [ ] The Sidebar accepts an optional `resumeModes?: Record<string, ResumeMode>` prop (default `{}`) and, for each item whose `machine === "Session"` **and** whose `id` is in the map, renders a resume-mode indicator built from `describeResumeMode` (the single descriptor source — no re-derived glyph/label).
- [ ] The indicator is **never color alone** (§11.6): the glyph is a visible non-color channel + an accessible label (see Step-2.5 Q1 for the compact treatment).
- [ ] `ProjectionItem` is **unchanged** — no `resume_mode` field added (Lesson §8); a test pins that the sidebar reads the side map, not a widened item.
- [ ] Items with no map entry (and non-Session items) render **no** indicator.
- [ ] The Shell builds the map from `data.sessions` and passes it to `<Sidebar>`.
- [ ] All unit tests in `ui/src/recovery/model.test.ts` (the map builder) pass.
- [ ] Sidebar behavior tests in `ui/src/shell/Sidebar.test.tsx` pass.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`main.tsx → <Shell/> → resumeModesBySessionId(data.sessions) → <Sidebar resumeModes={…}/> → per Session item (id in map) → describeResumeMode → indicator`. Confirm the Shell passes the real map (not unit-only) and the sidebar renders the indicator on the live path.

## Files expected to touch
**New:** _(none — extends the recovery model + the Sidebar)_

**Modified:**
- `ui/src/recovery/model.ts` — add `resumeModesBySessionId(sessions: SessionRow[]): Record<string, ResumeMode>` (cohesive with `describeResumeMode`).
- `ui/src/recovery/model.test.ts` — the map-builder unit.
- `ui/src/shell/Sidebar.tsx` — accept `resumeModes` prop; render the indicator for mapped Session items (reuse `describeResumeMode`).
- `ui/src/shell/Sidebar.test.tsx` — indicator present for mapped sessions / absent otherwise / never-color-alone (glyph + label) / `ProjectionItem` not widened.
- `ui/src/shell/Shell.tsx` — build the map from `data.sessions`; pass to `<Sidebar>`.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
`ui/src/recovery/model.test.ts` (pure builder):
1. **`resume_map_includes_only_sessions_with_a_mode`** — Asserts: a session with `resume_mode:"replayed"` is in the map (`{[id]:"replayed"}`); a session with `resume_mode` undefined is **absent**. Why: only resumed/replayed sessions get an indicator.
2. **`resume_map_keys_by_session_id`** — Asserts: the map key is `session_id` and the value is the exact `ResumeMode`. Why: id-keyed side map (the §8 mechanism).
3. **`resume_map_empty_when_none`** — Asserts: zero sessions / none-with-a-mode → `{}`. Why: empty is the no-indicator state.

`ui/src/shell/Sidebar.test.tsx`:
4. **`sidebar_shows_resume_indicator_for_mapped_session`** — Asserts: a Session item whose id is in `resumeModes` renders the indicator with the `describeResumeMode` glyph + label. Why: the feature.
5. **`sidebar_no_indicator_when_absent`** — Asserts: a Session item NOT in the map (and any non-Session item) renders no indicator. Why: only mapped sessions.
6. **`sidebar_resume_indicator_never_color_alone`** — Asserts: the indicator carries the glyph + an accessible label (not color alone, §11.6). Why: forbidden #5.
7. **`sidebar_does_not_widen_projection_item`** — Asserts: the indicator derives from the `resumeModes` prop, not an item field — render the SAME items WITHOUT the map → no indicator (proves the data path is the side map, `ProjectionItem` unchanged). Why: Lesson §8.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. Consumes the existing provisional `ResumeMode` / `SessionRow.resume_mode`; adds **no** new provisional shape and **no** field to `ProjectionItem`. The provisional reconcile is already covered by the existing Carry-forward "survival/recovery provisional shapes → generated" spread (recovery-shapes origin P6.4d) — **no new Carry-forward item**.
- **Orchestrator doc rows to write hot (Step 9 routing):** none (likely **extends Lesson §8** — the side-map pattern for session-specific display data; orchestrator decides at Step 9).

## Things to flag at Step 2.5
1. **Compact indicator treatment — glyph + sr-only label, or glyph + short visible label?** Options: (a) the visible glyph (▶/⟳, `aria-hidden`) + a visually-hidden `.sr-only` full label ("Resumed (live)"/"Replayed (relaunched)") — compact; the distinct **shape** is the visible non-color channel, the full meaning reaches AT; (b) glyph + a short visible label inline. My default vote: **(a) glyph + sr-only label** — the sidebar is dense and the glyph is an unambiguous non-color channel (satisfies never-color-alone); the table already carries the full visible label. Flag if the lead wants a visible label in the sidebar too.
2. **Where in the sidebar item does the indicator sit?** Options: after the `StatusPill`, or adjacent to the label. My default vote: **after the `StatusPill`** (trailing the row, consistent with the table's trailing Recovery column) — but it's presentation; implementer's call.
3. **Map builder location — `recovery/model.ts` vs `shell/`?** My default vote: **`recovery/model.ts`** — cohesive with `describeResumeMode` (the survival/recovery view-model home); the Shell just calls it.
4. **Guard on `machine === "Session"` before the map lookup?** My default vote: **yes** — only sessions have resume modes; the sidebar item type is the generic `ProjectionItem` (could be PR/Approval later), so guard the lookup to Session items even though today all sidebar items are sessions. Defensive + future-proof.
5. **Reuse `describeResumeMode` (don't re-derive glyph/label)?** My default vote: **yes, reuse it** — it's the single descriptor source (the table uses it too); never re-declare the glyph/label map.

## Dependencies + sequencing
- **Depends on:** 6.4d survival/recovery display (`290381a`, landed — `describeResumeMode` + the provisional `ResumeMode`). Independent of slices 1–3.
- **Blocks:** nothing.

## Estimated commit count
**1.** Small cohesive slice — a pure id-keyed map builder + the Sidebar consuming it, same survival-display concern, no safety invariant. One Step-10 commit.

## Lessons-logged candidates anticipated
- **Convention candidate** — surface **session-specific display data** (resume_mode) on a shared-shape widget (the sidebar `ProjectionItem`) via an **id-keyed side map prop**, NOT by widening the shared `ProjectionItem` (Lesson §8: the item mappers stay the single source for `{id,label,machine,status}`); reuse the single descriptor (`describeResumeMode`). Likely **extends Lesson §8** — orchestrator decides at Step 9.
- **Architecture-doc note candidate** — §11.4 already names the per-session resumed/replayed indicator; flag only if the sidebar surfacing needs an explicit note.

## How to invoke
1. **Read this brief end-to-end** — don't skip "Things to flag at Step 2.5."
2. **Run `/tdd sidebar_resume_mode_indicator`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** → confirm against the Feature line.
4. **Step 1 (Identify files)** → confirm against "Files expected to touch."
5. **Step 2.5** → tight test-design write-up + answers to the 5 design questions; wait for `APPROVED.` / `TWEAK:` / `ADD:`.
6. **Step 9** → categorized flags + ship-ask.

# /tdd brief — accessible_names_audit

## Feature
Close the **§11.6/§11.7 accessible-names gap** by extending the a11y merge-gate audit
(`ui/src/a11y/reachability.ts`) with a new **`auditAccessibleNames(container)`** classifier
— a standing net that asserts **every interactive control has an accessible name** — and wire
it into the whole-Shell multi-view sweep alongside the existing `auditFocusable`. Label any
control the sweep catches as nameless using the established **visually-hidden-child** pattern
(Lesson 6 — never a wrapper `aria-label`). The reachability audit today checks
keyboard-reachability **only**; a control can be focusable yet nameless (a screen-reader dead
end). This slice makes "every control is named" a mechanically-enforced §11.6 MUST.

> **NOT a cat-1 / safety-invariant slice** — pure deterministic UI a11y logic (the audit
> classifier IS the spec). **`security-reviewer` NOT required** (no INV-SEC-1 / single-mutator surface);
> `code-quality-reviewer` per the every-slice policy. No kit-contract change (the established
> wrapper / `.sr-only` / kit-`IconButton.label` pattern stands — Lesson 6).

## Use case + traceability
- **Task ID:** P6.4 (the inlined "Accessible names on shell controls + kit closed-props (§11.7)"
  item, `IMPLEMENTATION_PLAN.md` line ~556; origin P6.1b, broadened P6.2b).
- **Architecture sections it implements:** `ARCHITECTURE.md §11.6` (Accessibility invariants
  [LOCKED — PRD §14.8 MUST; tested §14] — the accessible-name requirement) + `§11.7` (Component
  contract fixes [LOCKED] — the kit closed-prop boundary). Both are within Phase 6's
  `§11.1–§11.7` scope (no scope widen).
- **Related context:** `ui/LESSONS.md §6` (a closed-prop control's accessible NAME comes from a
  **visually-hidden child INSIDE it**, never a wrapper `aria-label`; route only `data-*`/decorative
  `aria-*` onto wrappers) + Lesson 9 (the a11y foundation + the reachability audit, `roving`-aware,
  vitest-free, throws). The existing `ui/src/a11y/reachability.ts` (`auditFocusable` + the
  `ROVING_CONTAINER` map + `isRovingMember`) is the shape to mirror. The `.sr-only` utility lives
  in `ui/src/theme/global.css`. Kit `IconButton` already takes a required `label` prop (the kit
  wires the name); custom controls (`NavIconButton`, sidebar collapse, editor close-tab) already
  use `.sr-only` children — so the gap is the **missing audit rule**, not missing labels.

## Acceptance criteria (what "done" means)
- [ ] **`auditAccessibleNames(container)`** added to `ui/src/a11y/reachability.ts` — a pure DOM
      classifier (no vitest import; throws a plain `Error` on the first violation, mirroring
      `auditFocusable`). For every interactive control (the existing `INTERACTIVE_SELECTOR`), it
      computes an accessible name and **throws** if none: `name = aria-label || aria-labelledby
      target text || visible text from non-`aria-hidden` descendants (incl. `.sr-only`) || title
      || (input) associated <label>`. **Roving members at `tabIndex=-1` are skipped** (the
      one-tabstop member carries the name — symmetric with `isRovingMember` in `auditFocusable`).
      Decorative non-interactive markers (`AttentionMarker`/`StatusPill`) are NOT in the
      interactive selector → out of scope (correct — they're not controls).
- [ ] The whole-Shell multi-view sweep (`reachability.test.tsx`) calls `auditAccessibleNames`
      at **every view** alongside `auditFocusable` (same gate, same views).
- [ ] **Classifier unit tests** (RED-first): a control with `aria-label` passes; with visible
      text passes; with an `.sr-only` child passes; with `title` passes; a roving `tabIndex=-1`
      member is skipped; a **nameless** icon-only `<button>` **throws** (the named error).
- [ ] Any control the whole-Shell sweep catches as nameless is **labeled** via a visually-hidden
      `.sr-only` child (Lesson 6) — NOT a wrapper `aria-label`. (Expectation from scoping: most/all
      controls are already named; the deliverable is primarily the standing net. If the sweep is
      green on first run, say so — the net is the guard against future nameless controls.)
- [ ] Whole suite green; `/preflight` clean (oxlint + tsc + test:run).
- [ ] No kit-contract change; no new dependency.

## Wiring / entry point (Step 7.5)
**The audit is a merge-gate TEST net, not a production runtime path** — its "entry point" is the
`reachability.test.tsx` whole-Shell sweep (the §11.6 merge-gate, run in CI). `auditAccessibleNames`
is reachable from the test sweep (an uncaught throw fails the gate) + its classifier units. Any
`.sr-only` label added to a real control is rendered in that control's production view (Shell →
that view). Flag at 7.5 as a test-net addition (same class as the existing `auditFocusable`), not
a production wiring concern.

## Files expected to touch
**Modified:**
- `ui/src/a11y/reachability.ts` — add `auditAccessibleNames` (+ a `getAccessibleName(el)` helper if
  the logic warrants; reuse `INTERACTIVE_SELECTOR`, `describeEl`, `isRovingMember`).
- `ui/src/a11y/reachability.test.tsx` — the classifier unit tests + wire `auditAccessibleNames`
  into each view sweep.
- (only if the sweep catches one) the specific control file(s) needing an `.sr-only` label.

If the sweep catches a nameless control in a file not listed, **flag at Step 2.5** with the fix plan.

## RED test outline (Step 2)
1. `accessible_names_throws_on_nameless_control` — a bare `<button><svg/></button>` (icon-only, no
   name) → `auditAccessibleNames` throws the named error. `spec(§11.6)`.
2. `accessible_names_accepts_aria_label` / `_visible_text` / `_sr_only_child` / `_title` — each
   named form passes. `spec(§11.6)`.
3. `accessible_names_skips_roving_member_at_-1` — a `role="tab"` `tabIndex=-1` in a one-tabstop
   tablist is skipped (not required to self-name). `spec(§11.6)`.
4. `accessible_names_excludes_aria_hidden_text` — text inside an `aria-hidden` child does NOT count
   as a name (the glyph is hidden; the name must come from a non-hidden source). `spec(§11.6)`.
5. **Whole-Shell sweep** — `reachability.test.tsx` runs `auditAccessibleNames` across all views;
   green (or fix + green). `spec(§11.6)`.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. This is a UI a11y test net.
- **Orchestrator doc rows to write hot (Step 9):** likely a Lesson 9 extension (the a11y audit now
  asserts name-coverage, not just reachability) — flag it; the orchestrator decides Lesson-9-extend vs a
  new lesson. No `ARCHITECTURE.md` edit (§11.6/§11.7 are the existing locked anchors; the net is the
  enforcement the §11.6 waiver already names).
- **Cross-language contract seam (Appendix-A) touched?** No.

## Things to flag at Step 2.5
1. **The accessible-name algorithm scope.** Full WAI-ARIA accname is large; the pragmatic classifier
   covers `aria-label` / `aria-labelledby` / visible (non-hidden) text incl. `.sr-only` / `title` /
   input-`<label>`. **Default vote:** the pragmatic set (it catches the real gap — icon-only controls
   with no text/label); document the boundary (no full accname recursion — YAGNI, mirror the audit's
   flat-container assumption). Confirm.
2. **Roving + decorative handling.** Skip roving `tabIndex=-1` members (the tabstop names the group);
   exclude decorative markers (not in the interactive selector). **Default vote: as stated** (symmetric
   with `auditFocusable`). Confirm.
3. **If the whole-Shell sweep is already green** (all controls named): the slice ships the standing
   net + the classifier units (the RED is the classifier + the not-defined function). **Default vote:
   ship the net** — it's the §11.6 regression guard for future controls (same value `auditFocusable`
   provides). Note it honestly in Step 9 (no controls needed labeling vs N labeled).

## Dependencies + sequencing
- **Depends on:** the existing `a11y/reachability.ts` audit (landed, P6.4a). Nothing else; fully in-lane.
- **Blocks:** nothing hard; strengthens the §11.6 merge-gate for all future UI controls.

## Estimated commit count
**1.** The audit classifier + test wiring (+ any `.sr-only` labels the sweep forces). Single
deterministic unit; does NOT bundle. `code-quality-reviewer` per the every-slice policy;
`security-reviewer` NOT required (no invariant surface).

## Lessons-logged candidates anticipated
- **Convention candidate (Lesson 9 extension):** the a11y merge-gate audit asserts **accessible-name
  coverage** (not just keyboard-reachability) — every interactive control computes a name
  (`aria-label`/visible-text-incl-`.sr-only`/`title`), roving `-1` members skipped, `aria-hidden`
  text excluded; the name comes from a visually-hidden child (Lesson 6), never a wrapper `aria-label`.

## How to invoke
1. **Read this brief end-to-end** + `ui/LESSONS.md §6` + `§9` + the current `a11y/reachability.ts`.
2. Pre-flight: confirm you're on `track/ui` in the `NexusOps-ui` worktree, `cd ui`.
3. **Run `/tdd accessible_names_audit`.**
4. Step 0 (Restate) + Step 1 (files).
5. **Step 2.5** — answer the 3 design questions (or defaults) + the coverage map (each acceptance
   bullet → its test); send the write-up; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
6. Step 8 — `code-quality-reviewer` (every-slice); **no `security-reviewer`** (not an invariant slice).
7. Step 9 — report whether any control needed labeling (honest count) + the Lesson 9-extension flag.

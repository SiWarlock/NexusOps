# /tdd brief — status_rendering_binding

## Feature
The status **rendering** binding: a `StatusPill` wrapper that renders **every** frozen §5.1 state via the 6.2a descriptor table (four-channel **never-color-alone**), an `AttentionMarker` bound to the attention-rank, **Approval vs ActionRequest as two distinct status surfaces** (R-5), and wiring the attention model into the **6.1b sidebar** (attention-ordered + a needs-attention count). This is the rendering half of 6.2 (6.2a model landed `b32c3c0`); the full Command Center triage view is 6.3.

## Use case + traceability
- **Task ID:** P6.2b (decomposition of 6.2: 6.2a model → **6.2b rendering**)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.3` (StatusPill keys == §5.1 verbatim; render every state of all machines; Approval & ActionRequest as two distinct surfaces; sidebar weight/sort), `§11.1` (never-color-alone — glyph + text + intensity; Attention Ladder), `§11.7` (component contract — render all waiting_* states), `§11` (sidebar).
- **Related context:**
  - 6.2a model (`ui/src/status/`): `describeStatus(machine,status) → {attentionRank, visualKind, label}`, `needsMyAttention`, triage buckets, `compareByAttention`, `deriveWorktreeStatus`.
  - Kit (verified): `StatusPill` takes `status: StatusKind` (the ~20-key visual vocab) + `label` override + `emphasis`/`size`/`beacon`; `STATUS` map carries glyph + label + pulse/beacon per kind. `AttentionMarker` takes `level: 0–5`, `variant: 'rail'|'dot'`. Both are four-channel by design.
  - 6.1b shell: `Sidebar.tsx` is where the attention-ordered project/item weight wires in.
  - **Rendering slice** — the deterministic logic landed in 6.2a; here the test-first surface is the **four-channel coverage** (every state renders glyph+label, not color-only) + the **two-surface** distinction + the **attention-ordered** wiring. Pure pixel fidelity is design-review, not test-first.

## Acceptance criteria (what "done" means)
- [ ] A `StatusPill` wrapper (`ui/src/status/StatusPill.tsx`) takes `(machine, status)`, looks up the 6.2a descriptor, and renders the kit `StatusPill` with the descriptor's `visualKind` + `label`.
- [ ] **Every** frozen §5.1 state renders with **≥3 non-color channels** — glyph + text label + intensity (never-color-alone, §11.1 / forbidden-pattern #5) — including all 17 Session states + `changes_ready` (the kit's ~7 → full coverage, §11.3).
- [ ] An `AttentionMarker` binding maps the descriptor's `attentionRank` → the kit `AttentionMarker` `level` (0–5).
- [ ] **Approval and ActionRequest render as two distinct status surfaces** (R-5) — visually/label-distinguishable, not collapsed into one (Q3).
- [ ] The **6.1b sidebar** orders its items by `compareByAttention` and surfaces a **needs-attention count** (via `needsMyAttention`); attention weight is visible (AttentionMarker rail).
- [ ] **Reachable from** `Shell → Sidebar → StatusPill/AttentionMarker` (real render path); the components are also consumed by 6.3 screens.
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`Shell → Sidebar` renders `StatusPill`/`AttentionMarker` bound to the 6.2a model, attention-ordered. This makes the 6.2a model reachable on the real render path. The StatusPill/AttentionMarker wrappers are also consumed by 6.3 (core screens) — reachable-by-next-slice for the rest. Name the sidebar wiring as the entry point.

## Files expected to touch
**New:**
- `ui/src/status/StatusPill.tsx` — `(machine,status)` → descriptor → kit StatusPill wrapper.
- `ui/src/status/AttentionMarker.tsx` — rank → kit AttentionMarker wrapper.
- `ui/src/status/StatusPill.test.tsx`, `ui/src/status/AttentionMarker.test.tsx`, `ui/src/status/two-surface.test.tsx` (or co-located).

**Modified:**
- `ui/src/shell/Sidebar.tsx` — attention-ordered items (`compareByAttention`) + needs-attention count + AttentionMarker rail.
- `ui/src/shell/Sidebar.test.tsx` (or the shell test) — assert attention ordering + the count.

Flag any file beyond this at Step 2.5.

## RED test outline (Step 2)
**`status/StatusPill.test.tsx`:**
1. **`status_pill_renders_every_frozen_state_four_channel`** — iterate every frozen `(machine, status)` (from the generated `.options`); each renders a glyph + a text label (≥3 non-color channels), never empty/color-only. Asserts §11.1 never-color-alone across full §5.1 coverage. **[load-bearing]**
2. **`status_pill_uses_descriptor_label_and_visualkind`** — the wrapper renders the descriptor's label + maps to the kit visualKind (not the raw snake_case status). Why §11.3 (display labels are a separate copy layer).

**`status/AttentionMarker.test.tsx`:**
3. **`attention_marker_level_matches_rank`** — `AttentionMarker` for a `(machine,status)` renders the kit marker at `level === descriptor.attentionRank`. Why §11.1 ladder.

**`status/two-surface.test.tsx`:**
4. **`approval_and_action_request_render_two_distinct_surfaces`** — the same lifecycle moment (an Approval `awaiting_approval` and an ActionRequest `awaiting_approval`) renders as two visually/label-distinct surfaces, not one. Why §11.3 / R-5. **[load-bearing]**

**`status/` or shell `Sidebar.test.tsx`:**
5. **`sidebar_orders_by_attention`** — sidebar items render in `compareByAttention` order (higher rank first). Why §11.3 (sidebar weight/sort).
6. **`sidebar_needs_attention_count`** — the needs-attention count === number of items with `needsMyAttention` true. Why §11.3 (queue membership surfaced).

## Cross-doc invariant impact
- **Model field changes:** none. Consumes the 6.2a descriptor table + the kit (both already cross-doc-rowed).
- **Orchestrator doc rows to write hot:** none expected. (If the §11.7 kit StatusPill needs a contract change to render a state it can't today — e.g. a missing visual kind — flag it; the kit is canonical per O-5 but §11.7 is the component-contract-fix home.)

## Things to flag at Step 2.5
1. **StatusPill wrapper vs extending the kit.** Default vote: a **thin wrapper** `<StatusPill machine status/>` that looks up the descriptor and renders the kit `StatusPill` with `visualKind`+`label` — don't fork the kit. Confirm. If some §5.1 state has **no clean kit `StatusKind`** (the kit has ~20 visual kinds; some §5.1 states may not map 1:1), flag the mapping gap (a §11.7 component-contract item) rather than inventing a visual silently.
2. **AttentionMarker level = rank directly.** Default vote: `level = descriptor.attentionRank` (both 0–5). Confirm.
3. **What makes Approval vs ActionRequest "two distinct surfaces"?** Default vote: render each from its own machine's descriptor with a **distinct surface treatment + a type-disambiguating label** (e.g. an Approval pill vs an ActionRequest pill are not interchangeable; the user can tell a decision-axis state from an execution-axis state). Confirm the concrete distinction (label prefix? icon? separate component?) — this is the R-5 design nuance.
4. **Sidebar wiring scope.** Default vote: 6.2b wires **attention ordering + needs-attention count + AttentionMarker rail** into the existing 6.1b Sidebar; the full Command Center **needs-attention/working/settled grouped triage view** is **6.3**. Confirm the boundary.
5. **never-color-alone assertion approach (jsdom).** Default vote: assert each surface contains a **glyph element/`::before` content + a text label** (the non-color channels) — color itself isn't reliably testable in jsdom, so pin the presence of the other channels. Confirm.

## Dependencies + sequencing
- **Depends on:** 6.2a model (`b32c3c0`); 6.1b shell sidebar (`39a87c6`); 6.1a generated enums.
- **Blocks:** 6.3 (core screens consume StatusPill/AttentionMarker + the Command Center triage view); completes the 6.2 task (tick `[x]` at `/orchestrate-end` once 6.2b lands).
- **Deferred:** ExecutionProfile pill (its descriptors land at 0.5b — see carry-forward).

## Estimated commit count
**1–2.** Rendering wrappers + sidebar wiring; cohesive. No safety **invariant** → security-reviewer NOT required; code-quality every-slice.

## Lessons-logged candidates anticipated
- **Convention candidate** — StatusPill/AttentionMarker are thin wrappers over the kit, bound to the 6.2a descriptor table; never re-declare a status's visual/label/rank at the call site (single source = the descriptor). Possibly fold into `ui/LESSONS.md §5`.
- **Architecture-doc note candidate** — if a §5.1 state has no clean kit `StatusKind`, that's a §11.7 component-contract gap (flag, don't invent).

## How to invoke
> Session already oriented — **do NOT** run `/session-start`. Jump to `/tdd`.
1. **Read this brief end-to-end** — Q1 (kit mapping gaps) + Q3 (two-surface distinction) are the ones to confirm.
2. **Run `/tdd status_rendering_binding`.**
3. **Step 2.5** — test design + answers to the 5 questions. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
4. **Step 7.5** — name `Shell → Sidebar → StatusPill/AttentionMarker` as the entry point.
5. **Step 9** — flag any kit StatusKind mapping gap (§11.7) + the two-surface treatment; commit-message-first. Completes 6.2.

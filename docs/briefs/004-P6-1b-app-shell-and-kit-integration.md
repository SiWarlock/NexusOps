# /tdd brief — app_shell_and_kit_integration

## Feature
Build the Tauri app **shell chrome** (top bar + project switcher, sidebar, right drawer stack, activity dock, status bar) and establish the **`NexusOps-ui-kit` integration pattern** (link the token `styles.css`; import the kit's component sources via a Vite alias). The shell renders from **fixture projections through the 6.1a `gateway-client` seam** — mounting that seam as the production entry point (closing 6.1a's foundation reachability gap). This is slice **6.1b** (split from 6.1). The **daemon-connection indicator + global read-only degraded mode + version-skew** are the separate **6.1c** slice — 6.1b only reserves their shell slots.

## Use case + traceability
- **Task ID:** P6.1b (decomposition of 6.1: 6.1a foundation [landed `fd9738b`] → **6.1b shell+kit** → 6.1c connection/read-only state)
- **Architecture sections it implements:** `ARCHITECTURE.md §11` (projection-driven reattaching client; right drawer stack; activity dock), `§11.1` (canonical design system — `NexusOps-ui-kit` / Graphite Arc; reference the **semantic** token layer; never-color-alone), `§4.2` (law 2 — UI reads projections, holds no authoritative state), `§11.2` (shell + screen→surface map; project filtering scopes Command Center/Graph/Audit/Activity dock).
- **Related context:**
  - 6.1a foundation: `ui/src/contracts/` (generated Zod + provisional), `ui/src/gateway-client/` (`GatewayPort` interface + boundary + `MockGatewayPort`), `ui/src/projections/fixtures/`. The shell consumes these — never raw payloads.
  - Kit structure (verified): components are **clean self-contained ES modules** (`components/<group>/<Name>.jsx` + `<Name>.d.ts`, styled via CSS custom properties); `styles.css` is an `@import` manifest of `tokens/*.css`; `ui_kits/control-plane/kit-shell.jsx` is the **reference** TopBar+Sidebar design; `_ds_manifest.json` maps name→sourcePath.
  - Kit shell reference (readme §5): top bar = back/forward + **project switcher (repo + live counts: active sessions · open PRs · waiting-on-you)**; Brain + Settings reached from the top bar (not sidebar); collapsible **activity dock** along the bottom (status bar → expandable project-filtered event timeline → "Full audit"); project filtering scopes the main surfaces.
  - **Visual fidelity is NOT test-first** (TDD posture: pure visual is design-review territory). The deterministic, test-first core of this slice is the **projection→shell-chrome derivations** + the **drawer-stack reducer** + **renders-from-projections (no invented state)**.

## Acceptance criteria (what "done" means)
- [ ] `styles.css` tokens linked into the app; a kit component (e.g. `Button`/`StatusPill`) imported via the chosen mechanism (Step-2.5 Q1) renders with the kit's semantic token vars applied.
- [ ] Shell chrome regions exist and compose: **TopBar** (back/forward + project switcher), **Sidebar**, **right DrawerStack**, **ActivityDock** (collapsed status-bar ↔ expanded event timeline), **StatusBar** (reserves a slot for the 6.1c connection indicator).
- [ ] **`deriveProjectSwitcherCounts(projections)`** returns per-project `{ activeSessions, openPRs, waitingOnYou }` derived purely from fixture projections (no invented values).
- [ ] The shell **renders projects from the projection** — exactly the fixture's projects, none invented (pins `ui/CLAUDE.md` forbidden-pattern #2: no authoritative state in the UI).
- [ ] The shell reads **only through the `gateway-client` boundary** (validated payloads) — never a raw/unvalidated payload, never a direct DB/git/GitHub call (forbidden-patterns #2/#3).
- [ ] **DrawerStack** is a LIFO reducer (push / pop / replace / clear; ESC pops top; one drawer visible at a time).
- [ ] **ActivityDock** binds its event timeline to the event/audit projection; collapsed↔expanded is state-driven.
- [ ] **Reachable from** `ui/src/main.tsx` → `<Shell/>` → `gateway-client` — the production entry point; 6.1a's generated layer + boundary + `MockGatewayPort` are now live on the real render path (Step 7.5 closes the 6.1a foundation gap).
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`ui/src/main.tsx` mounts `<Shell/>`; `Shell` instantiates the `gateway-client` (`MockGatewayPort` for now) and reads projections through the boundary validator. **This closes 6.1a's foundation reachability gap** — confirm at Step 7.5 that the generated contract layer + boundary + mock are reached from the real render path, not just tests. (Real `UdsGatewayPort` still deferred to the daemon-1.5 integration; the mock is the live backing until then.)

## Files expected to touch
**New (shell):**
- `ui/src/shell/Shell.tsx` — top-level layout composing the regions.
- `ui/src/shell/{TopBar,ProjectSwitcher,Sidebar,DrawerStack,ActivityDock,StatusBar}.tsx`
- `ui/src/shell/derive.ts` — pure projection→chrome derivations (`deriveProjectSwitcherCounts`, activity-feed selection).
- `ui/src/shell/drawer-stack.ts` — the LIFO drawer-stack reducer.

**New (design-system integration):**
- `ui/src/design-system/kit.ts` — typed re-exports of the kit components the shell uses (single import surface; keeps the kit path in one place).

**New (fixtures + tests):**
- extend `ui/src/projections/fixtures/` — add `proj_project_activity` (projects), `proj_pull_request`, `proj_approval_queue` fixtures for the switcher counts.
- `ui/src/shell/derive.test.ts`, `ui/src/shell/drawer-stack.test.ts`, `ui/src/shell/Shell.test.tsx`.

**Modified:**
- `ui/vite.config.ts` — kit alias + `server.fs.allow` for the sibling `NexusOps-ui-kit/`; ensure the kit `.jsx` is processed by the React plugin (per Q1).
- `ui/tsconfig.json` — `paths` for the kit alias; include the kit `.d.ts`.
- `ui/src/main.tsx` — import the kit `styles.css`; mount `<Shell/>`.
- `ui/src/contracts/provisional.ts` — add provisional projection-row shapes for the new fixtures (banner-marked; enum fields delegate to the generated layer). [flag at Step 9 if added]

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)

**`ui/src/shell/derive.test.ts`:**
1. **`derive_project_switcher_counts_from_projections`** — given fixture session/PR/approval projections, `deriveProjectSwitcherCounts` returns the correct per-project `{activeSessions, openPRs, waitingOnYou}`.
   - Asserts: counts match the fixtures exactly (active = non-terminal sessions; openPRs = `proj_pull_request` open-ish; waitingOnYou = approval-queue entries + `waiting_on_permission`/`waiting_on_human_input` sessions for that project).
   - Why: `§11`/kit shell live-counts; `§4.2` law 2 (derived, not invented).
2. **`derive_counts_empty_projection_is_zeroed_not_absent`** — a project with no activity returns explicit zeros, not `undefined`/missing.
   - Asserts: zero-state is explicit.
   - Why: degraded/empty-state honesty (`§11.4` first-class empty states).

**`ui/src/shell/drawer-stack.test.ts`:**
3. **`drawer_stack_push_pop_lifo`** — push A, push B → top is B; pop → top is A; pop → empty.
   - Asserts: LIFO order.
   - Why: `§11` right drawer stack semantics.
4. **`drawer_stack_replace_and_clear`** — replace swaps the top; clear empties; ESC-equivalent pops top.
   - Asserts: replace/clear/esc behavior.
   - Why: `§11` drawer-stack control.

**`ui/src/shell/Shell.test.tsx` (render over fixtures, jsdom + testing-library):**
5. **`shell_renders_projects_from_projection`** — the project switcher lists exactly the fixture projects, none invented.
   - Asserts: rendered project set === fixture set.
   - Why: `ui/CLAUDE.md` forbidden-pattern #2 (no authoritative state in the UI).
6. **`shell_reads_only_through_gateway_boundary`** — the shell obtains projections via the `gateway-client` boundary (validated); a malformed projection from the mock surfaces as the boundary's reject path, not a raw render.
   - Asserts: reads route through the validated seam.
   - Why: forbidden-patterns #2/#3; closes the 6.1a reachability gap.
7. **`shell_activity_dock_collapsed_and_expanded`** — collapsed shows the status-bar summary; expanded shows the event timeline bound to the event/audit projection.
   - Asserts: state-driven dock; events come from the projection.
   - Why: `§11` activity dock.
8. **`shell_kit_token_layer_applied`** *(light)* — a kit component renders with the kit's semantic token vars / class hooks present (token link works).
   - Asserts: kit integration is live (not a bare unstyled element).
   - Why: `§11.1` canonical design system wired.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none frozen. New provisional projection-row shapes (`proj_project_activity`/`proj_pull_request`/`proj_approval_queue`) stay in `provisional.ts` under the [[2]] banner (enum fields delegate to the generated layer) — **flag at Step 9** so I note them on the carry-forward reconcile list (no `ui/CLAUDE.md`/`ARCHITECTURE.md` row — they're provisional, not frozen).
- **Orchestrator doc rows to write hot:** none new expected (the cross-doc row for the generated layer already landed in 6.1a).

> Implementer never edits `ui/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, `ui/LESSONS.md`.

## Things to flag at Step 2.5
1. **Kit consumption mechanism (load-bearing — recurs every UI slice).** Default vote: **link `styles.css` for tokens + import the kit component `.jsx` sources via a Vite alias `@ui-kit` → `../NexusOps-ui-kit/components`** (add `server.fs.allow` for the sibling dir; `tsconfig` `paths` + the per-component `.d.ts` give types under strict). Verified the kit components are clean React-only ES modules styled via CSS vars, so this should drop in. Alternatives: the runtime global bundle `window.ControlPlaneDesignSystem_a21911` (**rejected** — fights TS-strict ES modules) / vendor-copy into `ui/src` (**rejected** — diverges from the canonical kit + breaks the "re-hue touches primitives only" promise). **Verify at Step 1** the `.jsx` sources have no hidden interdeps and `.d.ts` resolve under strict; if alias-import doesn't cleanly work, flag before GREEN (this is the one decision that could need a re-think).
2. **Shell scope = chrome, not screen contents.** Default vote: **6.1b builds the regions/containers + projection→chrome derivations; screen CONTENTS (Command Center triage, Graph, Sessions, Terminal, Diff) are 6.3** — the shell renders placeholder/empty content panes. Confirm.
3. **Project-switcher count semantics.** Default vote: **activeSessions = sessions in non-terminal states; openPRs = `proj_pull_request` in open/checks/review states; waitingOnYou = approval-queue entries + sessions in `waiting_on_permission`/`waiting_on_human_input`, scoped per project.** The full status→attention-rank table is **6.2** — keep these counts simple/explicit here, don't pre-build 6.2's table. Confirm the semantics.
4. **Render-test stack.** Default vote: **`@testing-library/react` + jsdom under Vitest**; assert structure/derived-content, NOT pixel fidelity (visual is design-review). Confirm adding the testing-library dev dep (via pnpm — now canonical).
5. **DrawerStack model.** Default vote: **LIFO stack (push/pop/replace/clear), one drawer visible, ESC pops top.** Confirm.

## Dependencies + sequencing
- **Depends on:** 6.1a (landed `fd9738b`) — contracts layer + `gateway-client` seam + fixtures.
- **Blocks:** **6.1c** (connection indicator + read-only mode + version-skew — mounts into the StatusBar/global overlay regions this slice creates); **6.2** (status binding uses the sidebar/attention weight); **6.3** (screens fill the content panes).
- **Not blocked by** the flag-1 (object-key strictness) ratification — the shell reads contract-valid fixtures; a strict ruling is absorbed without rework.
- **Later integration:** real `UdsGatewayPort` ← daemon 1.5.

## Estimated commit count
**1–2.** Clean split if it helps bisection: **(1)** kit integration (token link + alias + `design-system/kit.ts` + the light token-applied test); **(2)** shell chrome (regions + `derive.ts` + `drawer-stack.ts` + fixtures + render tests). No safety **invariant** touched (chrome + reads only) → **security-reviewer NOT required** for this slice (that's 6.1c, which gates intent-submitting controls); code-quality-reviewer per the `every-slice` policy.

## Lessons-logged candidates anticipated
- **Convention candidate** — kit consumption pattern: tokens via `styles.css`, components via the `@ui-kit` source-alias (not the global bundle); product code references the **semantic** token layer only (§11.1). Likely `ui/LESSONS.md §3`.
- **Architecture-doc note candidate** — if `kit-shell.jsx`'s region model diverges from `§11`'s described shell (drawer stack / activity dock), flag the reconciliation (the kit is canonical per O-5, but `§11` is the contract).
- **Future TODO** — provisional projection-row shapes added here join the [[2]] reconcile-at-object-schema-freeze carry-forward.

## How to invoke
> Session already oriented (6.1a ran in it) — **do NOT** run `/session-start`. Jump straight to `/tdd`.
1. **Read this brief end-to-end** — Q1 (kit consumption) is the load-bearing one; verify the kit module structure at Step 1.
2. **Run `/tdd app_shell_and_kit_integration`.**
3. **Step 0/1** — restate (shell chrome + kit integration only; connection/read-only is 6.1c); confirm the file list + verify the kit alias resolves.
4. **Step 2.5** — send the test-design write-up + answers to the 5 questions. Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
5. **Step 7.5** — name `main.tsx → <Shell/> → gateway-client` as the entry point; confirm it closes the 6.1a foundation gap.
6. **Step 9** — flag the new provisional shapes + the kit-consumption convention candidate.

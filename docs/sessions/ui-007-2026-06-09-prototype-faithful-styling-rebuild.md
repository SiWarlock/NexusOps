# ui-007 — 2026-06-09 — Prototype-faithful styling/layout rebuild (solo)

**Mode:** solo working session (per handoff 001 — the team is paused; this is the
user-directed rebuild after the 6.5 "Graphite Arc" token-pass was REJECTED as not
matching the prototype).
**Spec:** `NexusOps-ui-kit/ui_kits/control-plane/` (index.html + kit-*.jsx) — the
gold reference, judged by REAL side-by-side rendered comparison (Lesson §10/§12),
prototype served from the ui-kit root + Vite dev server, gstack browser screenshots
every chunk.
**Result:** the running app now matches the prototype's cockpit anatomy across the
shell, all 12 views, and the overlay system. 214/214 tests green · tsc clean ·
oxlint clean. 6 commits on `track/ui`.

## What landed (commit per chunk)

1. **Shell chrome** — prototype grid (top / banner / side+main / EventDock);
   TopBar port (traffic lights, icon nav, dropdown ProjectSwitcher w/ live
   counts, ⌘K field, live runtime pill, Inbox/HIQ/Gateway/Brain/Settings
   cluster); Sidebar workspace tree (per-project groups → attention-ordered
   session rows w/ status dots + Team chips + resume indicators) + view nav +
   PLATFORM section; EventDock (collapsed strip ↔ project-scoped timeline +
   Full audit). lucide-react added; kit re-export surface widened to all 14
   components. **Contract reconcile:** regenerated Zod from shared 0.8.0
   (handoff-prescribed), 4 new enums mirrored, mock version tripwire bumped.
2. **Command Center** — project-scoped cockpit (header + attention-card grid +
   WORKING NOW / RECENTLY SETTLED rich session rows w/ real-Usage ctx rings) +
   the global rail (HIQ cards · Capacity w/ real credit-pool meter · recent
   events). `triageBucket` rank-1 → working (prototype semantics; §5.2 doc
   alignment flagged below).
3. **Project Graph** — dotted canvas, kit GraphNode cards (descriptor-bound
   status, beacons, ctx meta incl. honest "ctx unknown"), curved edges, layered
   layout; keyboard-operable nodes via a DOM-seam focusability patch (kit
   GraphNode hardcodes role="button" w/ closed props — upstream fix flagged);
   List fallback unchanged.
4. **Settings + Usage** — the prototype's 4 tabs (Integrations / Execution
   profiles / Usage / Security & policy) w/ integration + profile cards over
   flagged display fixtures; Usage = stat cards + credit meter + top-context
   consumers + per-subject table over REAL aggregates (no fabricated limits or
   history). Interim Notifications stub tab removed (no prototype equivalent).
5. **Remaining views** — Projects Overview + Audit Trail LIVE off projections;
   Session Terminal (real header/status; honest PTY-pending well; table as
   picker); Code/Diff Review (real PR lanes; diff/worktrees display fixtures);
   Plan / Editor / Agent Team / Workflow Packs / Brain page as faithful
   display-only ports over flagged fixtures. Team sessions route to the team
   view; TopBar Brain wired.
6. **Overlays** — Overlay shell (center/top/right, ESC, focus return) hosting:
   ⌘K Command Palette (live nav routing), ⌘⇧H Human Input Queue (REAL queue;
   Review/Open live; resolutions disabled), Gateway modal (real approval +
   honest ActionPlan-pending consequence note; approve/deny disabled), ⌘⇧P Task
   Inbox (display fixture), Brain drawer (page in drawer mode + Expand),
   Inspector drawer (graph nodes; live Open jump). DrawerStack region removed
   (superseded).

## Method note (Lesson §10/§12 honored)

Every chunk was gated on actual rendered side-by-side comparison (prototype
http-server vs Vite dev server, 1280×800), not token wiring. Two real bugs were
caught only by the visual gate: dual-React invalid-hook crash after the
lucide-react install (stale Vite process) and the unstyled/clipped
ConnectionIndicator + ProjectSwitcher popover (stale compact-row CSS).

## Flags (not faked — need daemon contracts / orchestrator follow-up)

- **Projection enrichment:** session harness/profile/branch/worktree/current/
  activity/team, project repo/workflow, approval risk/actor, PR branch/diff
  stats/age, audit timestamps — all ride PROVISIONAL display side-maps
  (`shell/display-meta.ts`, `views/display-fixtures.ts`, Lesson §8 pattern;
  delete-on-enrichment).
- **Intent seam (daemon-gated):** every mutation affordance (approve/deny/
  defer, dispatch, retry, merge, personalize/upgrade/run, pause, new session/
  worktree/profile/project, Brain composer + Run-via-Gateway, export, sync)
  renders disabled with a gate-naming tooltip (§11.6). DispatchDialog + Toast
  deliberately not built (no honest entry point).
- **triageBucket change:** rank-1 active now WORKING (was settled) — §5.2
  wording in ARCHITECTURE.md should be aligned (orchestrator-territory edit).
- **Deviations from prototype (deliberate):** no "All projects" switcher scope
  (single-project active-project model — a model change, not styling);
  Notifications settings tab removed; DiffHunk per-hunk action bar suppressed
  (kit has no disabled mode → dead clicks); kit GraphNode upstream improvement
  (expose tabIndex/aria-label); kit AttentionMarker level-4 accessible label
  reads "Failed / blocked" for any rank-4 status (kit wording nit).
- **Sessions table** kept reachable as the Session Terminal's no-selection
  picker (prototype has no table view; logic + tests preserved).

## State

- Branch `track/ui`, 6 commits ahead of `ea44705` (not pushed — push at
  close-out per posture).
- Suite: 214 passed / 0 failed; typecheck + oxlint clean.
- The contract reconcile (0.8.0 regen) that handoff 001 parked for "team
  resume" is DONE here (it was blocking a green suite).

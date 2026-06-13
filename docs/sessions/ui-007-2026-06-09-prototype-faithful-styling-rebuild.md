# ui-007 — Prototype-faithful styling/layout rebuild (solo)

- **Date:** 2026-06-09 → 2026-06-10
- **Phase:** post-Phase-6 presentation rebuild (solo working session per handoff 001 — the team is paused; this is the user-directed rebuild after the 6.5 "Graphite Arc" token-pass was REJECTED as not matching the prototype)
- **Track:** `track/ui`
- **Predecessor:** [ui-006 — daemon-independent polish round](ui-006-2026-06-08-daemon-independent-polish-round.md)
- **Successor:** [ui-008](ui-008-2026-06-13-intent-seam-foundation-and-gatewaymodal.md)
- **Round commits:** `8f52a71` · `9d943e3` · `e6b198f` · `856d1d3` · `c566f6a` · `15b0fc4` (+ this doc)
- **Tests:** 193 → **214 green**; tsc + oxlint + prettier clean throughout.

## Why this session existed

The 6.5 theme pass had passed an automated "token-match" visual gate but the
USER REJECTED the rendered result as "way off" the prototype (Lesson §10:
green ≠ looks right). Mission: rebuild the ui's layout + styling so the running
app matches `NexusOps-ui-kit/ui_kits/control-plane/` (index.html + kit-*.jsx)
EXACTLY — aesthetic AND functionality — judged ONLY by real side-by-side
rendered comparison (prototype http-served from the ui-kit root vs the Vite dev
server, gstack browse screenshots, every chunk).

## What was built

**Files created** (all under `ui/src/` unless noted):

- `shell/display-meta.ts` — PROVISIONAL display side-maps (Lesson §8): session
  harness/profile/branch/worktree/current/activity/team, project repo/workflow,
  approval risk/actor, integrations + profiles card sets; plus
  `contextForSession` (real Usage rows → ctx, honest null) and
  `sessionsByProject`. Delete-on-daemon-enrichment.
- `shell/EventDock.tsx` — prototype bottom dock: collapsed status strip (live
  ConnectionIndicator + latest event) ↔ project-scoped audit timeline + "Full
  audit" jump. Replaces ActivityDock + StatusBar.
- `views/cockpit.tsx` — shared cockpit primitives (Eyebrow, card style,
  TermLine).
- `views/display-fixtures.ts` — flagged display fixtures backing the
  daemon-gated views (diff files, worktrees, PR display extras, editor
  tree/files, team, packs, plan, brain thread/memory).
- `views/command/SessionRowCard.tsx` — kit SessionRow anatomy rebuilt from kit
  primitives, descriptor-bound (Lesson §6) + "ctx unknown" support
  (forbidden #4).
- `views/projects/ProjectsOverview.tsx` — LIVE project card grid (real
  projections + counts; workflow chip from side-map).
- `views/audit/AuditTrail.tsx` — LIVE timeline over the AuditTrail projection
  (namespace filter chips, project/all scope, icon bubbles + connectors, seq in
  place of timestamps).
- `views/terminal/SessionTerminal.tsx` — real session header/status + honest
  PTY-pending well (§9.1 invariant #9); permission prompt (disabled) only when
  genuinely waiting; no selection → hosts the Sessions table as picker.
- `views/code/DiffReview.tsx` — Review (kit DiffHunk over fixture; per-hunk
  action bar suppressed) · Worktrees (fixture) · Pull requests (REAL projection
  lanes via the frozen status enum).
- `views/plan/PlanView.tsx`, `views/editor/EditorView.tsx`,
  `views/team/AgentTeamView.tsx`, `views/packs/WorkflowPacksView.tsx`,
  `views/brain/BrainPage.tsx` — faithful display-only ports over flagged
  fixtures (plan parser / worktree FS / AgentTeam projection / pack registry /
  Brain sidecar are daemon-gated).
- `overlays/Overlay.tsx` — overlay shell (center/top/right, scrim+blur, ESC,
  focus-in/focus-return, dialog semantics).
- `overlays/CommandPalette.tsx` (⌘K, live nav routing + filter + Arrow/Enter),
  `overlays/HumanInputQueue.tsx` (⌘⇧H, REAL queue; Review/Open live;
  resolutions disabled), `overlays/GatewayModal.tsx` (real approval + honest
  ActionPlan-pending consequence note; approve/deny disabled),
  `overlays/TaskInbox.tsx` (⌘⇧P, fixture intake), `overlays/BrainDrawer.tsx`
  (BrainPage in drawer mode + Expand), `overlays/InspectorDrawer.tsx` (graph
  node identity + real pill + side-map/usage details + live Open jump).
- `overlays/overlays.test.tsx` — 8 behavior tests (shortcuts, real queue
  contents, fail-safe disabled mutations, live navigation jumps, ESC).

**Files modified:**

- `shell/Shell.tsx` — prototype chrome composition; sidebar-nav view routing
  (12 views); session-selection + team routing; overlay state machine +
  global shortcuts; global waiting count; EventDock wiring.
- `shell/TopBar.tsx` — full prototype port (traffic lights, icon back/forward,
  switcher, repo slug, live counters, ⌘K field, runtime pill, Inbox/HIQ/
  Gateway/Brain/Settings cluster — all overlay triggers live).
- `shell/Sidebar.tsx` — workspace project tree + view nav + Platform section;
  Human Input/Task Inbox live; attention ordering within groups; locators +
  resume indicators preserved.
- `shell/ProjectSwitcher.tsx` — prototype trigger/popover anatomy (roving
  listbox semantics kept; repo slug + waiting dot + workflow square).
- `shell/view-history.ts` — ViewName widened to the prototype's 12-view set.
- `shell/derive.ts` — `pendingApprovals` / `waitingSessions` exported helpers.
- `views/command/CommandCenter.tsx` — the 3-column cockpit (header, attention
  cards, working/settled rows, global rail: HIQ · Capacity · Recent events).
- `views/graph/ProjectGraph.tsx` — dotted canvas + kit GraphNode cards +
  curved edges + layered layout + FocusableNode keyboard patch + onInspect.
- `views/settings/{Settings,tabs}.tsx` — prototype 4-tab Settings (cards over
  flagged fixtures; risk-ladder Security panel; Notifications tab removed).
- `views/usage/UsageDashboard.tsx` — prototype stat-card layout over REAL
  aggregates (no fabricated limits/history).
- `status/attention.ts` — `triageBucket` rank-1 → working (prototype
  semantics).
- `status/StatusPill.tsx` — exported `kitKindFor` (single-source guarded
  kit-kind mapping).
- `design-system/kit.ts` — all 14 kit components re-exported (+ Badge typing
  shim; `DisplayStatusPill` sanctioned exception for non-contract display
  states).
- `contracts/{generated,index}.ts` + `gateway-client/mock.test.ts` — the
  shared-0.8.0 Zod regen/reconcile (handoff-001-prescribed) + version tripwire
  bump.
- `theme/{shell,components}.css` — prototype grid + dock/conn-indicator/
  graph-toggle styles; stale compact-switcher + content-switch + drawer-stack
  styles removed.
- `package.json`/lockfile — `lucide-react` added.
- Tests updated to the new chrome: `Shell.test`, `Sidebar.test`,
  `ProjectSwitcher.test`, `CommandCenter.test`, `Settings.test`, `tabs.test`,
  `view-history.test`, `attention.test`, `a11y/reachability.test` (sweep now
  covers ALL views + the open dock + open dropdown).

**Files deleted:** `shell/ActivityDock.tsx`, `shell/StatusBar.tsx` (merged
into EventDock); `shell/DrawerStack.tsx` + `shell/drawer-stack.{ts,test.ts}`
(superseded by the Overlay system); `views/Placeholder.tsx` (created mid-
session as interim stubs, removed once all views landed).

## Decisions made

- **Port the prototype JSX faithfully as inline-styled TSX** over kit
  components (the prototype is itself inline-styled over the same kit) — exact
  fidelity, minimal indirection; structural grid/scroll stays in `shell.css`.
- **Display side-maps for prototype richness the thin projections lack**
  (Lesson §8 pattern, like `recoveryStatusFixture`): render paths real, data
  source flagged + delete-on-enrichment. Never widened a projection row.
- **Wire-or-disable everywhere** (§11.6): every mutation affordance renders
  disabled with a tooltip naming its gate; nothing fakes a result.
- **`triageBucket` rank-1 = working** — an active session is in-flight
  (prototype semantics); rank 0 only is settled.
- **Sessions table** (no prototype equivalent) kept reachable as the Session
  Terminal's no-selection picker — tested logic preserved, nav matches the
  prototype.
- **Team sessions route to the Agent Team view** on open (prototype behavior),
  driven by the display side-map team flag.
- **Contract reconcile done here** (regen to 0.8.0 + 4 new enums mirrored +
  tripwire bump): handoff 001 parked it for team-resume, but the suite was red
  without it — a green close-out required it.
- **DispatchDialog + Toast deliberately not built:** dispatch is disabled (no
  honest entry point) and no mutation can fire a toast.
- **DiffHunk per-hunk action bar suppressed** (`actions={false}`): the kit bar
  has no disabled mode → rendering it would be dead clicks.
- **Kit GraphNode focusability patched at the DOM seam** (tabIndex +
  aria-label + Enter/Space) — closed props; genuine keyboard operability.

## Decisions explicitly NOT made

- **No "All projects" switcher scope** (prototype has one): the active-project
  model is single-project; an "all" scope is a model change, not styling —
  deferred to a future slice with orchestrator scoping.
- **No real approve/deny/dispatch/merge** — the intent seam is daemon-gated
  (Phase 8 / daemon-1.5+); not faked.
- **No timestamps in audit/event rows** — the projection carries `seq` only;
  rendering invented clock times was rejected (daemon enrichment instead).
- **No kit source edits** (GraphNode tabIndex, AttentionMarker level-4 label
  wording, Button "attention" variant) — flagged upstream instead.

## TDD compliance

This session was a **presentation rebuild** — the TDD-exempt visual/theme
class (root CLAUDE.md TDD posture; Lessons §10/§12), gated by the rendered
visual comparison instead. Within that:

- **Deterministic logic changes** (`triageBucket` re-bucketing, derive
  helpers, `contextForSession`, palette filtering, view-history widening) and
  the new surfaces' behavior were pinned by tests **in the same commits**, but
  mostly **test-with/after, not failing-test-first** — flagged: *TDD violation
  (test-after): `status/attention.ts` re-bucketing, `shell/derive.ts` helpers,
  `shell/display-meta.ts` helpers, overlay behaviors (covered by
  `overlays.test.tsx` written after the components).* None are safety-critical;
  all are now pinned (214 green).
- **Safety-relevant render rules kept test-first-equivalent coverage:** ctx
  "unknown" (forbidden #4), disabled-mutation fail-safety, queue membership,
  reachability sweep — all asserted in the suite.
- The **visual gate was honored every chunk** (side-by-side screenshots before
  each commit); two real bugs only it could catch: the dual-React invalid-hook
  crash (stale Vite process after the lucide-react install) and the unstyled/
  clipped ConnectionIndicator + ProjectSwitcher popover (stale compact-row
  CSS).

## Reachability

All from the production entry (`main.tsx` → `Shell`):

- 12 views reachable from the sidebar nav / TopBar (Settings gear, Brain) /
  Command Center header (Projects chip) / EventDock (Full audit) — pinned by
  the extended `a11y/reachability.test.tsx` sweep over EVERY view + the open
  dock + the open switcher dropdown.
- Agent Team via team-session rows (sidebar tree + inspector Open); Session
  Terminal via non-team session rows + nav; Brain page via drawer Expand +
  palette.
- Overlays reachable from TopBar cluster + sidebar entries + shortcuts
  (⌘K/⌘⇧H/⌘⇧P) + graph node activation (inspector) — pinned by
  `overlays.test.tsx`.
- **No tested-but-unwired features.** The deleted DrawerStack was the only
  wired-but-superseded surface; removed with its tests.

## Open follow-ups

1. **Daemon projection enrichment** (replaces the display side-maps): session
   harness/profile/branch/worktree/current/activity/team · project
   repo/workflow/brain · approval risk/actor/command/worktree + ActionPlan
   consequence preview · PR branch/diff-stats/age/checks detail · audit
   timestamps · usage history (14-day spend) + spend/agent limits · task
   intake projection · plan projection · AgentTeam projection · worktree/diff
   contract · pack registry · Brain sidecar (Phase 8). Each named in-place by
   a banner comment.
2. **Intent seam (daemon-gated):** enable the disabled mutation affordances;
   build DispatchDialog + Toast when dispatch can actually fire.
3. **Orchestrator-territory doc edits (cross-doc audit, §2.5):**
   `triageBucket` rank-1→working needs the §5.2/§11.3 wording aligned in
   `ARCHITECTURE.md` (render-policy change, UI-canonical table itself
   unchanged); the ui/CLAUDE.md cross-doc rows for the generated contract
   layer remain accurate post-regen (0.8.0 — mechanism unchanged).
4. **Kit upstream improvements (flagged):** GraphNode should expose
   tabIndex/aria-label; AttentionMarker level-4 accessible label reads
   "Failed / blocked" for any rank-4 status; Button lacks the prototype's
   "attention" variant (falls back to secondary — prototype renders the same).
5. **"All projects" switcher scope** — model change, future slice.
6. **Lesson candidates for the orchestrator:** (a) the visual gate must be
   RENDERED side-by-side, never token-match (extends §10/§12 — this session is
   the proof); (b) closed-prop kit interactivity gaps are patched at the DOM
   seam + flagged upstream (extends §6/§9).

## How to use what was built

- Dev: `cd ui && npm run dev`; prototype reference:
  `npx http-server NexusOps-ui-kit -p 8081` →
  `/ui_kits/control-plane/index.html`.
- Shortcuts: ⌘K palette · ⌘⇧H Human Input queue · ⌘⇧P Task Inbox; Escape
  closes any overlay.
- When a daemon contract lands, grep the gate name (e.g. "intent seam",
  "projection enrichment") — every placeholder/disabled affordance carries it
  in a comment or tooltip.

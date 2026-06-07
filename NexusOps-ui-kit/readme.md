# NexusOps — Control Plane Design System

> **Product:** NexusOps — a desktop control plane for AI software-engineering agents.
> **Personality:** *Graphite Arc* — dark, precise, premium, technical, quiet, powerful.
> **One line:** Air-traffic control for AI coding agents. The human is the pilot; NexusOps is the instrument panel.

This is the design system project. An automated compiler reads it and ships `styles.css` (tokens + fonts) plus a runtime component bundle (`_ds_bundle.js`, namespace `window.ControlPlaneDesignSystem_a21911`). Link `styles.css`; read components off the namespace.

---

## 1 · What NexusOps is

NexusOps is a **local-first desktop application** that lets one engineer supervise many autonomous coding agents at once. Agents run in real harnesses (Claude Code, Codex CLI/Cloud, custom shells) inside isolated git worktrees; NexusOps is the cockpit that makes their work **observable, governable, and interruptible**. The product's thesis: as agents multiply, the scarce resource is *human attention* — so the entire UI is organized around **surfacing what needs a human, and nothing else.**

Core object model (all recreated as components): **Project → Session → Agent Team (lead + workers) → Worktree → Branch → PR**, grounded by a **Project Brain** (memory/evidence/reasoning) and governed by an **Action Gateway** (risk-rated approvals). **Workflow Packs** (e.g. *cc-crew*) script multi-agent orchestration. An append-only **Event Model / Audit Trail** records everything.

### Primary surfaces (the UI kit recreates these)
- **Command Center** — triage dashboard: *Needs my attention → Working now → Recently settled*, with a human-input queue, capacity meters, and a live event feed.
- **Project Graph** — operational observability map of every object and its status.
- **Session Terminal** — a single agent's live stream, with inline permission prompts.
- **Code / Diff Review** — first-class review surface with per-hunk actions (Accept / Reject / Ask Brain / Request fix).
- **Project Brain drawer** & **Action Gateway modal** & **Command palette (⌘K)** — the cross-cutting overlays.

### Sources provided (stored for reference; reader may not have access)
All under `uploads/`. There is **no Figma or live codebase** — the system was built from these product/architecture specs:
- `PRODUCT_CANON.md`, `PRD.md`, `SHARED_OBJECT_MODEL.md`
- `UX_INFORMATION_ARCHITECTURE.md`, `UI_COMPONENT_INVENTORY.md`
- `ACTION_GATEWAY.md`, `PROJECT_BRAIN_INTERFACE.md`, `WORKFLOW_PACKS.md`, `CC_CREW_WORKFLOW_PACK.md`
- `DESKTOP_FIRST_RUNTIME.md`, `EVENT_MODEL_AND_AUDIT_TRAIL.md`
- `CLAUDE_DESIGN_SYSTEM_PROMPT.md`, `CLAUDE_DESIGN_PROTOTYPE_PROMPT.md`

> **Naming note:** internal docs left the product name open ("AI Engineering Control Plane"); the brand is **NexusOps**. Some sample data still references "Control Plane" as a workspace/repo name — that's deliberate in-world content, not the brand.

---

## 2 · Content fundamentals (voice & copy)

The product talks like a **senior infra/SRE tool**, not a consumer app. Calm, exact, operational.

- **Person & address.** Speak to the operator as **you** ("Needs my attention", "Approve once", "Always allow tests in this project"). The system refers to itself by name (*Project Brain*, *Action Gateway*), never "I". Agents are named by harness + task ("Claude · ENG-221", "Codex · GH-184").
- **Tone.** Declarative and consequence-aware. State *what will happen* before asking for approval ("Sandboxed in worktree `~/wt/eng-221`. No network. Reversible."). Never hype, never apologize, no exclamation marks.
- **Casing.** **Sentence case** everywhere — buttons, titles, menus ("New session", "Run plan via Gateway"). **UPPERCASE micro-eyebrows** label sections ("NEEDS MY ATTENTION", "HUMAN INPUT QUEUE", "CHANGED FILES") — tracked +0.06em, 10px. Object *types* are Title Case nouns (Session, Worktree, Pull Request, Project Brain).
- **Status language is fixed vocabulary.** Use the canonical labels: *Waiting on you · Permission required · Running · Failed · Conflict · Stale · Checks passing · Ready to merge · Archived.* Don't invent synonyms — status strings are part of the taxonomy (see `tokens/status.css`).
- **Numbers & identifiers are monospace and literal.** `128k ctx`, `$2.74`, `+412 −98`, `4f18a70`, `#84`, `ENG-221`, `~/wt/eng-221`. Tabular figures; never round away precision the operator needs.
- **Verbs of governance.** Approve / Deny / Always allow / Run plan / Request fix / Retry checks / Pause / Discard. Each maps to a risk level.
- **Emoji:** none. **Glyphs:** geometric Unicode markers (● ▶ ◆ ⊘ ✓ ✕ △ ◌) used *with* a text label, never instead of one. Iconography is Lucide line icons.
- **Examples to echo:** *"Needs my attention"* · *"Permission required"* · *"Run plan via Gateway"* · *"No writes outside worktree · no push"* · *"grounded @ 4f18a70"* · *"3 evidence"*.

---

## 3 · Visual foundations

### Direction — *Graphite Arc*
A serious desktop **engineering cockpit**. Reference points: Linear (density, keyboard polish), Raycast (command palette/modals), GitHub Primer (status/PR semantics, primitive→semantic token split), VS Code / Warp (terminal-first dark surfaces), Datadog (dense operational timelines), Obsidian/Arc (premium pane discipline). Explicitly **not**: neon cyberpunk, generic purple AI SaaS, colorful Dribbble dashboard.

### Color — neutrals do the work, hue carries meaning
- **Surfaces:** charcoal/graphite ramp with a *subtle* blue undertone (oklch hue ~260, chroma 0.006–0.012 — reads "graphite," never "blue"). Eight steps, `--n-900` window void → `--n-550` active. Default theme is **dark**; a `[data-surface="light"]` scope exists for printed specimens only.
- **Accent (azure, ~235):** the single interactive color — selection, focus, primary actions, active systems.
- **Secondary (violet, ~287):** *Project Brain only* — memory, reasoning, evidence.
- **Tertiary (teal, ~175):** workflow packs, orchestration, agent teams.
- **Status hues, rationed:** live cyan · success muted-green · **attention amber (loudest)** · caution amber-orange · warning ochre (dimmed) · danger coral-red · critical deep-red (+ hazard hatch) · review purple · slate (idle/archived).
- **Tokens are two-tier, GitHub-Primer-style.** Primitives (`--accent`, `--n-700`) → semantic aliases (`--surface-card`, `--text-primary`, `--state-waiting-human`, `--risk-high`, `--diff-add-bg`, `--graph-edge-evidence`). **Always reference the semantic layer in product code.** Full taxonomy in `tokens/color.css`; this is a **first-pass palette, not locked brand color.**
- **Never color alone.** Every status is encoded on four channels — color + **glyph** + **text label** + (optional) **motion** — so it survives grayscale and color-blindness. See the *Attention ladder* and the four-channel `StatusPill`.

### The Attention Ladder (the system's core idea)
States are ranked 0–5 by *how loudly they need a human*, and that rank drives sort order, rail weight, and loudness everywhere (queues, sidebar, graph): **5** waiting-on-human (amber beacon) → **4** failed/blocked/conflict & permission → **3** degraded/high-capacity → **2** running/testing → **1** active/dirty/PR-open → **0** idle/done/archived. See `foundations/status-attention-ladder.html`.

### Type
- **Geist** (UI) + **Geist Mono** (code, IDs, paths, SHAs, numerics — anything alignable or copyable). Two families only. *(Loaded from Google Fonts — see Caveats.)*
- Dense desktop scale: body **13px**, label 12, meta 11, micro 10; titles 16/19/24/32; display 44. Tabular numerics for all data.

### Space, shape, depth
- **4px base grid;** most gaps 4–12px. Compact control heights (22/26/30/36). Minimum desktop hit target 28px.
- **Tight, mechanical radii:** chips/inputs `--r-1` (3px), buttons `--r-2` (5px), cards/panels `--r-3` (7px), modals `--r-4` (10px). This is an instrument panel, not a marketing site.
- **Depth is restrained:** a 1px lighter top key-line + soft contact shadow (`--elev-1..4`); sunken wells (terminals, code, lists) use `--elev-inset`. No glow except the **azure focus/selection** and the **amber attention beacon**.
- **Cards:** `--surface-card` fill, 1px `--border-default`, `--r-3`. Attention/danger cards add a 3px left status rail + tinted surface + matching border. No drop-shadow on resting cards — elevation is reserved for popovers/modals.

### Backgrounds, motion, states
- **Backgrounds:** flat graphite. The only texture is the **Project Graph's** 22px dot-grid (`--graph-grid`) and the **critical hazard hatch** (45° repeating lines). No photographic imagery, no decorative gradients (a gradient appears once, as a scrim fade behind sticky headers).
- **Motion is operational, never delight.** Durations 80–320ms, calm `--ease-standard`. Two signature looping motions only: the **live pulse** (running sessions) and the **attention beacon** (waiting-on-human ring). Overlays use `cp-pop-in` / `cp-slide-in`. Everything is suppressed under `prefers-reduced-motion`.
- **Hover:** surface lifts one step (`--surface-hover`); ghost controls gain a faint fill. **Press:** `scale(0.985)` (`--press-scale`), no color flip. **Focus:** 2px azure ring. **Selected:** azure tint + `inset 0 0 0 1px --accent-line`.
- **Transparency/blur:** rationed — modal scrim (`--scrim`) + a 2px backdrop blur; tinted status surfaces are translucent so they sit on any panel. No frosted-glass everywhere.
- **Imagery vibe:** there is none by design; the "imagery" is data — graphs, diffs, terminals, meters — kept cool and neutral.

---

## 4 · Iconography

- **Lucide** line icons (1.5px stroke, rounded joins) are the icon system — the de-facto match for this Linear/Vercel-class technical UI. Loaded from CDN (`unpkg.com/lucide`); call `lucide.createIcons()` after render. *(Substitution flagged — the source specs named no icon font; see Caveats.)*
- **Usage:** object types map to stable glyphs — `layout-dashboard` Command Center, `workflow` graph/orchestration, `terminal` session, `git-branch`/`git-pull-request`/`git-merge` git, `brain` Project Brain, `users-round` teams, `shield-check` Action Gateway, `folder-git-2` worktree, `file-code`/`file-diff` review. Keep stroke weight consistent; size 13–16px inline, 16–20px in headers.
- **Status markers** are *not* Lucide — they're the geometric **Unicode glyphs** in `tokens/status.css` (● ▶ ◆ ⊘ ✓ ✕ △ ◌ ■ ⨯ ⇡ !), always paired with a label, rendered in Geist Mono so they align in dense rows.
- **No emoji, ever.** Harness identity uses faint mono glyphs (✻ Claude, ⌁ Codex CLI, ☁ Codex Cloud, $ shell) — near-neutral, no official brand color implied.
- See `foundations/iconography.html`. There are no raster/PNG icon assets and no logo file — NexusOps' mark is wordmark-only in these recreations (see Caveats).

---

## 5 · Index / manifest

**Root**
- `styles.css` — the single entry point consumers link (an `@import` manifest only).
- `readme.md` — this guide. · `SKILL.md` — Agent-Skills wrapper.
- `tokens/` — `fonts.css` · `surfaces.css` · `color.css` · `typography.css` · `space.css` · `motion.css` · `status.css`.

**Foundations** (`foundations/*.html`, shown in the Design System tab)
Type (display / body / mono) · Colors (surfaces, text+border, accents, status families, state cheat-sheet, risk+capacity, diff+viz, domains) · Status (attention ladder) · Spacing (scale, radii+elevation, controls, motion) · Brand (iconography).

**Components** (`components/<group>/`, on `window.ControlPlaneDesignSystem_a21911`)
- `controls/` — **Button**, **IconButton**
- `status/` — **StatusPill**, **AttentionMarker**, **RiskBadge**, **UsageMeter**
- `badges/` — **Badge**, **HarnessBadge**, **ProfileBadge**, **MetaChip**
- `objects/` — **SessionRow**, **GraphNode**, **DiffHunk**, **EvidenceChip**
Each dir has `<Name>.{jsx,d.ts,prompt.md}` and one `@dsCard` HTML.

**UI kit** (`ui_kits/control-plane/`)
- `index.html` — the full interactive desktop app (also a Starting Point). Ten surfaces wired with live state.
- Data/shell: `kit-data.js` (sample data) · `kit-shell.jsx` (TopBar + Sidebar, grouped Workspace / Platform nav).
- Views: `kit-views.jsx` (Command Center + Projects overview) · `kit-views2.jsx` (Graph / Session Terminal / Diff Review) · `kit-views3.jsx` (Editor IDE / Agent Team) · `kit-views4.jsx` (Project Brain co-pilot / Audit Trail / Settings: Integrations · Execution Profiles · Security & policy) · `kit-views5.jsx` (Workflow Packs) · `kit-plan.jsx` (Implementation Plan: Phase → Track → PlanTask with dispatch).
- Overlays (drawer-first right stack): `kit-overlays.jsx` — Project Brain drawer, Inspector drawer, Action Gateway modal, Command palette ⌘K · `kit-tasks.jsx` — Task Inbox drawer + Dispatch dialog + **Human Input Queue** (centralized approval triage: permission requests, high-risk actions, failed-check decisions, personalization — opened from the sidebar Human Input row, top-bar bell, or ⌘⇧H).
- **No nav clutter:** the **Worktree / Git / PR control center** is tabs inside Code / Diff Review (*Review · Worktrees · Pull requests*), and the **Usage** dashboard (spend/tokens/context, 14-day trend, spend-by-profile, top context consumers) is a Settings tab — neither adds a sidebar row. Project filtering scopes the Command Center, Project Graph, Audit trail, Activity dock, worktrees, and PR lanes.
- **Shell:** the top bar has back/forward + a working project switcher (repo + live counts: active sessions · open PRs · waiting-on-you); Brain and Settings are reached from the top bar, not the Platform nav. A collapsible **Activity dock** runs along the bottom (status bar → expandable project-filtered event timeline → "Full audit"). **Project filtering** scopes the Command Center, Project Graph, Audit trail, and Activity dock to the selected project. The Editor's file tree + tabs switch real per-file contents; graph nodes open the Inspector drawer; team nodes open the split team-terminals.
- **Task intake (drawer-first):** ⌘⇧P opens the Task Inbox drawer (GitHub / Linear / plan-task chips). Drag a chip onto a session (adds context) or the canvas (new session), or click it to open the **Dispatch dialog** — choose a single session vs a cc-crew `/team-start` agent team, harness, and profile; runs via the Gateway. Connect GitHub/Linear in **Settings → Integrations**.
- **Interactive:** approving a Gateway request resolves the human-input queue, flips session status, toasts; the Brain page is a working chat co-pilot (type → grounded answer + evidence + action plan → Run via Gateway); the graph opens a node modal (team node → all team terminals in a split grid); the Projects overview filters the Command Center; ⌘K navigates; Workflow Packs enforces the pack≠instance lock.

---

## 6 · Caveats & substitutions
- **Fonts** are loaded from **Google Fonts** (Geist + Geist Mono), not vendored binaries — so the compiler reports 0 `@font-face` files. If you want offline/self-hosted fonts, drop `.woff2` files into `assets/fonts/` and swap the `@import` in `tokens/fonts.css` for `@font-face` rules. *Flagging for updated font files if a specific licensed family is intended.*
- **Icons** are **Lucide via CDN** — a substitution, since the source specs prescribed no icon set. Easy to swap if NexusOps adopts a house set.
- **No logo/brand mark** was provided; recreations use a wordmark only. Send a logo to fold in.
- **Color is a first pass.** *Graphite Arc* is a deliberate, documented thesis — not locked brand color. The two-tier token structure means a re-hue touches primitives only.

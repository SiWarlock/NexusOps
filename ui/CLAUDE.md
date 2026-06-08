# NexusOps `ui/` — Build Guide

> **You're in `ui/`.** This file plus root `CLAUDE.md` both load. The root file covers global project conventions + shared comm rules (track-prefix, escalation taxonomy, messaging budget); this file owns code-area conventions for the Tauri desktop UI (TS frontend + thin Rust host).

## Launch protocol

| Working on... | cwd | Loads |
|---|---|---|
| Planning / docs / commits | repo root (`NexusOps/`) | root `CLAUDE.md` only |
| the Tauri desktop UI (TS frontend + thin Rust host) code | `ui/` | this `CLAUDE.md` + root |

<!-- For a multi-area project, add a row per additional code area. -->

If you find yourself fighting the wrong conventions, check your cwd.

## Session start/end protocol

**At session start:**
1. Read `MVP_TASKS.md` (repo root) **by section, not whole** — `grep -n "^##" MVP_TASKS.md` for offsets, then Read with offset/limit just "Currently in progress" + the active phase. (The file grows; never load it whole.)
2. Confirm with the user what feature this session is targeting.
3. Read the relevant section of `ARCHITECTURE.md` from the lookup table below.

**At session end** (only when the user explicitly says we're done):

1. **Implementer runs `/session-end`.** Implementer writes ONLY:
   - `ui/` code files (the slice's implementation)
   - test files (the slice's tests)
   - dependency manifest / lockfile (deps the slice adds)
   - `docs/sessions/<NNN>-<date>-<topic>.md` (session doc, created at `/session-end` Step 5)

   **Implementer must NOT touch (all orchestrator territory):**
   - `MVP_TASKS.md`
   - `ui/LESSONS.md`
   - `ui/CLAUDE.md` (entire file — both the Cross-doc invariants table AND the Lessons logged index)
   - `ARCHITECTURE.md`
   - `docs/orchestrator-briefing.md` / `docs/tdd-brief-template.md` / `docs/briefs/` / `docs/runbooks/`
   - other top-level deliverable / design docs
   - `.gitignore` and root-level dotfiles (unless adding a new artifact to ignore, flagged at Step 9)

   At Step 10: **explicit `git add <path>` per slice file; never `git add -A`/`.`; never stage an orchestrator-territory file.** Changes to any orchestrator-territory file (a new cross-doc model, a lesson, an arch note) are **flagged at Step 9**, not edited here — the orchestrator writes them hot (root `CLAUDE.md` + the Step-9 matrix).

2. **Orchestrator runs `/orchestrate-end`** for round close-out + Carry-forward triage + round terminal commit + push.

## Lookup table — where to find canonical info

Don't paste these sections into the prompt. Grep the file:section, read only what you need. `/check-arch <topic>` dispatches off this table.

| Topic | File (relative to repo root) | Section |
|---|---|---|
| <subsystem A> | `ARCHITECTURE.md` | §X |
| <subsystem B> | `ARCHITECTURE.md` | §Y |
| Lessons logged (full prose) | `ui/LESSONS.md` | by lesson # |

<!-- Starts near-empty. Add a row whenever a topic is looked up twice. -->

**Code intelligence & docs (when available):** prefer a code-intelligence MCP / docs MCP over grep+read loops — see root `CLAUDE.md` "Code intelligence & docs."

## Stack

<!-- ▼ EXAMPLE BLOCK [id=area-stack]: stack quick-reference for implementer sessions. Canonical stack lives in root CLAUDE.md + ARCHITECTURE.md; this is the cheat sheet. ▼ -->

- **Runtime:** Node 22 + pnpm · Rust (Tauri host)
- **Framework:** React 19 + Vite · Tauri 2.x host
- **Validation:** Zod (IPC validation)
- **Lint / types / tests:** oxlint / `tsc --noEmit` / Vitest + Tauri-driver e2e

UI is a **projection-driven reattaching client**: it reads projections + submits intents over the UDS GatewayPort; it **never writes the DB**. The daemon is the single, audited mutator — the UI renders state, it does not own it.

<!-- ▲ END EXAMPLE BLOCK [id=area-stack] ▲ -->

## Standard commands

```bash
# Install deps (run once; re-run when the manifest changes)
pnpm install

# Run the dev server (if applicable)
pnpm tauri dev

# Tests
pnpm test:run

# Quality
pnpm oxlint
pnpm prettier --check .
pnpm typecheck

# Preflight (use before saying "done" with a feature)
pnpm oxlint && pnpm typecheck && pnpm test:run
```

## TDD protocol

**Write the failing test first.** Applies to deterministic code — see the TDD posture in root `CLAUDE.md` for what is test-first vs. exempt.

**Commit per slice when practical.** Never bundle a safety-critical slice with anything else.

## Forbidden patterns

<!-- ▼ EXAMPLE BLOCK [id=forbidden-patterns]: forbidden patterns — 3-5 narrow, enforceable, domain-specific rules. Shape: "Don't <pattern X> because <reason / past incident>; use <alternative Y>." Test-pin them where possible. Starts small; accretes as lessons surface. ▼ -->

Do not:

1. **Write code without a failing test first** (for deterministic code). Even one-line functions.
2. **Hold authoritative state in the UI** — the daemon is source of truth; render from projections. A value the UI invents (or caches as authoritative) drifts from the audited event log the moment the daemon mutates. Subscribe to the projection and re-render; treat the local store as a read cache only.
3. **Submit a mutation as anything but a typed Gateway intent** — never call git, GitHub, or the daemon DB directly from the UI. Every change is a risk-classified, approved Action; bypassing the GatewayPort bypasses the audit + approval path. All daemon access goes through the single `gateway-client`.
4. **Render Codex context-% as a number when it isn't reported** — show `"unknown"` when `supportsContextMetadata === false` (§9.1). Fabricating a percentage for an engine that doesn't expose one is a silent lie in the cockpit.
5. **Communicate status by color alone** — use glyph + label + intensity (§11 never-color-alone). And never ship a graph without its list/table fallback, a control without a focus ring, or a drag without a non-drag equivalent (§11.6 a11y MUSTs).
6. **Offer an intent-submitting control without consulting `canSubmitIntent`** — every mutation affordance (Gateway approve/deny, Dispatch, Brain Run-via-Gateway, commit/push) must be disabled in global READ-ONLY/degraded mode (§11.4); offering a mutation while the daemon is unreachable or version-skewed is a defense-in-depth breach. The gate is **fail-safe** (FALSE on unknown/initial; true only when confirmed connected + version-compatible — `ui/src/connection/read-only.ts`). It is defense-in-depth — the load-bearing INV-SEC-1 enforcement is daemon-side (§15), never the UI gate alone. See [[4]].

<!-- ▲ END EXAMPLE BLOCK [id=forbidden-patterns] ▲ -->

## Cross-doc invariants — schema/docs mirroring

Several typed models in this codebase are **contracts** mirrored in `ARCHITECTURE.md` and indexed in the table below. The architecture doc is the canonical contract; the model is the executable enforcement. Drift produces silent disagreement.

**Authoring discipline (orchestrator owns this table).** The implementer never edits this table or `ARCHITECTURE.md` directly — it flags a field add/remove/rename at Step 9 as a `Cross-doc invariant change`; the orchestrator writes the row + the arch edit hot the same round (see root `CLAUDE.md` + `docs/orchestrator-briefing.md`). Commits stagger; the working tree stays aligned within the round.

| Model | `ARCHITECTURE.md` section | Notes |
|---|---|---|
| Generated Zod contract layer (`ui/src/contracts/generated.ts` + `index.ts`) | §5.0, §5.1 | Generated, drift-caught consumer of the frozen `shared/contracts/schema/nexusops-contract.schema.json` (13 enum value-sets: 9 status machines + `ActorType`/`IdKind`/`DesktopObjectKind`). Never hand-edited; regen via `ui/scripts/gen-contracts.mjs`. Pinned by accept-all / reject-unknown / set+`.options` drift / `CONTRACT_VERSION`===`x-contract-version` tests (P6.1a). 6.2 extends with the status→attention-rank coupling. |
| Status→attention-rank descriptor table (`ui/src/status/descriptors.ts` + `attention.ts`) | §11.3, §11.1, §5.1, §7.2 | UI-canonical (NOT a frozen cross-language contract — UI render policy): every frozen `(machine, status)` → `{attentionRank 0–5, visualKind, label}`. Single source for sidebar weight / queue membership / sort. **Completeness drift-pinned** to the generated `.options` (a daemon-added status fails the test until covered); no fall-through to idle (`waiting_on_permission`/`conflicts`/`blocked`=4, `stale`=3, `changes_ready`=4). Worktree status derived via two-axis precedence. ExecutionProfile deferred (0.5b). (P6.2a) |

<!-- Starts empty (or with the first model if one exists). Populated as contract models land. -->

## Module organization

<!-- ▼ EXAMPLE BLOCK [id=module-layout]: module layout + layer dependency rule. Replace with the project's real directory tree and import-direction DAG. ▼ -->

```
ui/
  src/                      # frontend (TS)
    views/                  # screens / cockpit panels
    components/             # NexusOps-ui-kit components (design-system primitives)
    gateway-client/         # the single client for all daemon access (intents + projection reads)
    projections/            # projection subscriptions (read-only state from the daemon)
    design-system/          # tokens (color/intensity/glyphs, never-color-alone helpers)
  src-tauri/                # thin Rust host (window / channel / UDS client only — no business logic)
```

Layer dependency direction (top depends on bottom, never reverse):

```
views
  ← components
    ← design-system tokens

(all daemon access flows through the single gateway-client; src-tauri host stays logic-free)
```

Cross-cutting layers can be imported from anywhere. Enforce the rule mechanically with a test where possible — the test *is* the spec for the rule.

<!-- ▲ END EXAMPLE BLOCK [id=module-layout] ▲ -->

## Subagents

See `.claude/agents/README.md` for the canonical inventory + integration points.

<!-- ▼ EXAMPLE BLOCK [id=area-subagent-candidates]: area-specific subagent candidates — list candidates that would earn their keep specifically in this area (e.g. an ABI/types syncer for a frontend area, a Pyth/feed verifier for a contracts area). Build only on real friction. ▼ -->

<!-- ▲ END EXAMPLE BLOCK [id=area-subagent-candidates] ▲ -->

## Lessons logged from prior sessions

The full prose for each lesson lives in `ui/LESSONS.md`. This index is the compact orientation surface.

**Lesson numbers are stable IDs** — once assigned, they don't change. New lessons get the next sequential number. `/session-end` proposes additions when it detects them; the user approves before the entry is written and a row is added here.

Lessons start at §1.

| # | Date | Topic | Rule (one-liner) |
|--:|---|---|---|
| 1 | 2026-06-07 | [Generated contract enums](LESSONS.md#1) | UI contract enums are generated from the frozen `shared/` schema (artifact + accept/reject/drift/version pins), never hand-declared; status/ID/actor fields delegate to the generated validators. |
| 2 | 2026-06-07 | [Provisional object shapes](LESSONS.md#2) | Object shapes not yet frozen are provisional UI-local types (banner-marked, enum fields delegated, tolerant of unknown keys on reads); reconcile to generated shapes on the next contract bump. |
| 3 | 2026-06-07 | [ui-kit consumption](LESSONS.md#3) | Consume `NexusOps-ui-kit` as tokens-via-`styles.css` + components-via-`@ui-kit`-source-alias (with `resolve.dedupe(["react","react-dom"])` for the out-of-root peer react), never the global bundle / vendored; reference the semantic token layer only. |
| 4 | 2026-06-07 | [Fail-safe read-only gate](LESSONS.md#4) | The UI read-only/degraded gate is fail-safe (`canSubmitIntent` FALSE on unknown; true only confirmed connected + version-compatible) and defense-in-depth — never the sole mutation guard; the daemon Gateway (INV-SEC-1) is. |
| 5 | 2026-06-07 | [Status→attention-rank table](LESSONS.md#5) | The status→attention-rank descriptor table is keyed by (machine, status), drift-pinned to the generated enums (completeness test), the single source for sidebar/queue/sort; no fall-through to idle (§11.3). |
| 6 | 2026-06-07 (P6.4e refinement 2026-06-08) | [Status rendering wrappers](LESSONS.md#6) | Render status via thin descriptor-bound wrappers over the kit (single source = the descriptor); guard the kit's `\|\|idle` silent fallback (unknown→visible); derive kit-kind validation from the kit STATUS map; route data-*/decorative aria-* onto wrappers (kit props closed) — but a closed-prop control's accessible NAME comes from a visually-hidden child INSIDE it, never a wrapper aria-label. |
| 7 | 2026-06-07 | [Multi-commit slice driving](LESSONS.md#7) | For multi-commit ui slices the implementer idles after each layer commit; the orchestrator drives layer→layer (one wake per layer, at the commit boundary); briefs estimating ≥2 commits enumerate the layers explicitly. |
| 8 | 2026-06-07 (P6.4 side-map extension) | [Projection-item mappers + namespaced locator + entity side-maps](LESSONS.md#8) | Projection→item mappers in `items.ts` are the single source for the `ProjectionItem {id,label,machine,status}` shape (no inline re-map); `data-item-id` is namespaced `<namespace>:<id>` on every emitter (machine for status items, type for graph nodes), one rule with no exceptions. **Entity-specific display data** (e.g. a session's `resume_mode`) rides an **id-keyed side-map prop** (pure builder; machine-guarded lookup; reuse the single descriptor), **never a widened `ProjectionItem`** — pin it with a test that renders the same items WITHOUT the map → no indicator. |
| 9 | 2026-06-07 (P6.4 roving + P7.3 dropdown extensions) | [A11y foundation](LESSONS.md#9) | Focus-visible ring = one global `a11y/focus.css :where(...):focus-visible` (kit tokens, not per-component, extend the selector for new element types); reduced-motion = the kit `motion.css` guard + `useReducedMotion()` for JS motion; every control keyboard-reachable (`tabIndex >= 0`), pinned by the multi-view reachability audit (§11.6 merge-gate net). **Composite widgets** use WAI-ARIA **roving tabindex** (one tabstop; Arrow/Home/End move + auto-activate; focused index read from the DOM at event time — never a stale closure; shared **orientation-aware** `nextTabIndex` in `a11y/roving.ts` — horizontal default for tablists, vertical for listboxes). **Dropdown** = `button[aria-haspopup=listbox]` trigger + roving `role="listbox"` popover (open-focuses-active via `useLayoutEffect` on the open transition; Enter/Escape return focus to trigger, click-outside doesn't; disabled at zero options). The audit (`a11y/reachability.ts`, vitest-free, throws) is **roving-aware** via a `{tab→tablist, option→listbox}` container map (a `tabIndex=-1` member is reachable iff its container has exactly one tabstop) + sweeps `role=option`/covers visible `role="tabpanel"`. Assumes flat containers (nested = YAGNI). |
| 10 | 2026-06-07 | [Green ≠ looks right (visual gate)](LESSONS.md#10) | TDD-exempt visual/theme layers aren't verified by green tests + clean build (jsdom/tsc/oxlint/build don't check appearance); require a rendered-product visual gate (gstack browser: dev server vs the kit Graphite Arc prototype `ui_kits/control-plane/index.html`) + explicit VISUAL acceptance. Never report a UI slice "looks right" from green tests alone. |
| 11 | 2026-06-08 | [Degraded + safety states always explained, fail-closed](LESSONS.md#11) | Every read-only or §15/§17 state renders a distinct display surface (transport-degraded / session-survival / §17 safety — never conflated); fail-closed (conflict never-auto-resolved #6, audit-integrity non-dismissible #5, read-only never silent); parked intents disabled-but-present (gated on `canSubmitIntent`, INV-SEC-1 stays daemon-side); glyph derived from severity (never color alone); reuse frozen enums via `.extract` + enumerate coverage from `.options` so a future state is forced to render. |
| 12 | 2026-06-08 | [Theme/visual layer — apply, ground, gate](LESSONS.md#12) | The theme layer APPLIES the kit SEMANTIC tokens via `ui/src/theme/{global,shell,components}.css` (semantic only, §11.1 re-hue — kit ships tokens-only); GROUND every surface against the prototype's ACTUAL rendering (its chrome is in kit components, not the index.html `<style>` — don't guess); the VISUAL GATE (dev server vs prototype over HTTP) is the acceptance + catches what tsc/oxlint/jsdom can't (the `*/`-in-CSS-comment drop, surface divergences); structural refactors preserve reachability/landmarks; severity color is ADDITIVE to glyph+label. Extends [[10]]. |
| 13 | 2026-06-08 (P6.4 view-history + P6.3 filtering extensions) | [UI selection/scope/nav/filter state over frozen/local state](LESSONS.md#13) | A UI selection/scope (active-project), **navigation** (view-history back/forward), or **filter** (table status/text) is plain UI state over the FROZEN projections / local view selection — NOT a mutation/provisional/Gateway-intent → no `canSubmitIntent` gate, no daemon dep (ships without the parked intent seam). Pure model + thin context/hook wrapper (mirror `ReadOnlyProvider`/`active-project.ts`); scoped views re-root/filter, cross-cutting triage (Command Center) stays GLOBAL; a stale selection re-scopes to a default (no ghost id); view-history uses browser semantics (forward-truncation, collapse-on-same); filtering composes **filter→sort**, derives options from the **unfiltered** set, **ORs independent fields** (never a joined string — cross-boundary false-positive), and renders **filtered-empty distinct from truly-empty**. **Wire-or-disable:** a named control is never a dead click — wire it or `disable` it (§11.6). Distinct from the daemon-1.5 mutation path. |

<!-- Starts empty. Each row links to its `LESSONS.md` anchor. -->

<!-- Slash commands: see root CLAUDE.md "Slash commands available." Implementer pair: /session-start + /session-end. -->

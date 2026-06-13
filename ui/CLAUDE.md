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
1. Read `IMPLEMENTATION_PLAN.md` (repo root) **by section, not whole** — `grep -n "^##" IMPLEMENTATION_PLAN.md` for offsets, then Read with offset/limit just "Currently in progress" + the active phase. (The file grows; never load it whole.)
2. Confirm with the user what feature this session is targeting.
3. Read the relevant section of `ARCHITECTURE.md` from the lookup table below.

**At session end** (only when the user explicitly says we're done):

1. **Implementer runs `/session-end`.** Implementer writes ONLY:
   - `ui/` code files (the slice's implementation)
   - test files (the slice's tests)
   - dependency manifest / lockfile (deps the slice adds)
   - `docs/sessions/<NNN>-<date>-<topic>.md` (session doc, created at `/session-end` Step 5)

   **Implementer must NOT touch (all orchestrator territory).** *This list is the canonical statement
   of the territory rule — `/session-end`, the brief template, and the generated
   `scripts/guards/territory-guard.sh` PreToolUse hook (which mechanically enforces it in team mode)
   all point here.*
   - `IMPLEMENTATION_PLAN.md`
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

# Quality (prettier is NOT a project dep — oxlint + tsc are the gate)
pnpm oxlint
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

**Enforcement patterns (machine-readable — `/preflight` warn-greps the staged diff against these).**
One `grep -E` (or `ast-grep`) expression per line, each tied to a numbered rule above. Rules that can't
be expressed as a pattern carry a `pin:` (test ref) or `accepted:` note on the rule itself instead.

```forbidden-patterns
# <rule 2>: <pattern — e.g.>  datetime\.now\(\)
# <rule 3>: <pattern>
```

<!-- ▲ END EXAMPLE BLOCK [id=forbidden-patterns] ▲ -->

## Cross-doc invariants — schema/docs mirroring

Several typed models in this codebase are **contracts** mirrored in `ARCHITECTURE.md` and indexed in the table below. The architecture doc is the canonical contract; the model is the executable enforcement. Drift produces silent disagreement.

**Authoring discipline (orchestrator owns this table).** The implementer never edits this table or `ARCHITECTURE.md` directly — it flags a field add/remove/rename at Step 9 as a `Cross-doc invariant change`; the orchestrator writes the row + the arch edit hot the same round (see root `CLAUDE.md` + `docs/orchestrator-briefing.md`). Commits stagger; the working tree stays aligned within the round.

| Model | `ARCHITECTURE.md` section | Notes |
|---|---|---|
| Generated Zod contract layer (`ui/src/contracts/generated.ts` + `index.ts`) | §5.0, §5.1, §6.4 | Generated, drift-caught consumer of the frozen `shared/contracts/schema/nexusops-contract.schema.json`. **37 value-sets at `CONTRACT_VERSION 0.28.0`** (the P6.2/P6.3e delta-regens: 20@0.12.0 → 33@0.19.0 [+13 Phase-2 Gateway enums] → 34@0.23.0 [+`TerminalControlKind` pause\|resume; `ExecutorKind`+`adjudication`] → 37@0.28.0 [+`DiffLineKind` context\|added\|removed; +`ExecutionProfile` 9-value runtime-state; +`Provider` github\|linear; `IpcErrorCode`+`not_found` 9→10] — the P6.3e/047 boundary-merge regen). The two Gateway-status `$def` **renames** `ActionRequest`→`ActionRequestStatus` / `Approval`→`ApprovalStatus` are the schema names; the UI status-machine identifiers (`"ActionRequest"`/`"Approval"`) stay **UI-render-policy** (A-lite — decoupled from the `$def` name), bridged by a 2-entry `VALIDATOR_KEY` alias. Never hand-edited; regen via `ui/scripts/gen-contracts.mjs`; **`validators` derives from the generated bundle (`= shape`), never hand-listed** (the hand-list is what drifted at 0.12.0 → [[14]]). Pinned by accept-all / reject-unknown / set+`.options` drift / `CONTRACT_VERSION`===`x-contract-version` tests (P6.1a) + the §6.4 `ServerFrame` frame-mux schema-snapshot (now **3-variant** incl. `terminal_output` — field-name + `seq` field-**type** pins) / `WireError` (P6.2). **Doc-commented enums freeze as `oneOf`-of-`const`** (e.g. `MetricQuality` @ 0.23.0), which `gen-contracts.mjs` (flat-`.enum` only) does **not** emit → they live as **drift-pinned provisional shadows** (`provisional.test.ts`) pending a generator `oneOf`-const extension (carry-forward) (P6.4b). 6.2 extends with the status→attention-rank coupling. |
| Status→attention-rank descriptor table (`ui/src/status/descriptors.ts` + `attention.ts`) | §11.3, §11.1, §5.1, §7.2 | UI-canonical (NOT a frozen cross-language contract — UI render policy): every frozen `(machine, status)` → `{attentionRank 0–5, visualKind, label}`. Single source for sidebar weight / queue membership / sort. **Completeness drift-pinned** to the generated `.options` (a daemon-added status fails the test until covered); no fall-through to idle (`waiting_on_permission`/`conflicts`/`blocked`=4, `stale`=3, `changes_ready`=4). Worktree status derived via two-axis precedence. ExecutionProfile deferred (0.5b). (P6.2a) |
| GatewayPort mutation surface + the intent seam + consumer (`gateway-client/types.ts` + `intent/submit-intent.ts` + `contracts/intent-contracts.ts` + `overlays/GatewayModal.tsx` + `safety/model.ts`) | §6.1, §6.2, §4.2, §15, §11.1, §11.5 | The UI's **FIRST mutation/intent-submission path** (P6.3d, **cat-1**). `GatewayPort` mirrors the §6.1 **id-based wire** (`submit_action(ActionRequest)→ActionAck` / `preview_action(action_request_id)` / `approve(approval_id,step_id?)` / `deny(approval_id,reason)`; daemon source `daemon/src/ipc/methods.rs`). The seam is a **pure intent-submitter** (INV-SEC-1 — submits only, NEVER executes; the **daemon Gateway is the sole mutator + the real chokepoint**, the `canSubmitIntent` gate is **defense-in-depth**, not the sole guard); **no-optimistic-render** (status from the daemon ack only; "done" only on a confirming projection/`ActionResult`); **§6.4 codes verbatim** (never collapse/remap — `fencing_conflict` stays #6); **no intent state** (no cache/auto-retry; Q7-A, B/C parked-for-user). Frozen shapes = provisional Zod shadows (enum fields delegated; `inputs=z.unknown()` opaque passthrough; `WireError` `.strict()` per frozen `additionalProperties:false`), **field-set drift-pinned**. Error classification uses an **`instanceof Error` discriminant** (a daemon frame is plain data, never an `Error` — `.strict()` alone is insufficient: non-enumerable props). **Slice 1 (043):** Scope A — the seam in ISOLATION (7 cat-1 invariants pinned; `security-reviewer` PASS). **Slice 2 (044 — `GatewayModal`-real, LIVE):** the consumer renders the daemon's `PolicyDecision` (new provisional shadow in `intent-contracts.ts`) + `ActionPreview` — **never UI-derived risk / invented preview** (Q4/Q5); daemon-status-**never-"done"** (Q3); `safety/model.ts` `describeRejection` routes each §6.4 code to its **distinct §11.5 treatment** (`fencing_conflict` never re-approvable #6; the unmapped/transport set → an honest generic rejection, never swallowed); the net-new `precondition_stale` re-approvable card; the `policy_grant` "always allow" standing-grant stays **disabled-pinned** (deferred — own cat-1). 10 modal pins; `security-reviewer` PASS. The real `UdsGatewayPort`/`rpc_response` demux + the live daemon projection-enrichment (replacing the fixture side-map) = later slices. **P6.3e read surface (047, NON-safety, exposed-ahead):** `GatewayPort` gains the read-only `get_diff(worktree_id,file)→DiffResult` method (§6.1; 4 frozen-shadow diff shapes `DiffResult`/`Hunk`/`DiffLine`/`GetDiffParams` in `intent-contracts.ts`/`provisional.ts`, `.strict()` field-set + `Hunk`-offset uint32 drift-pins; `DiffLine.kind` delegates to generated `DiffLineKind`) + `PerHunkGitActionType` (the 3 `git.*` ids — a **typing handle only, NO risk/standing-grant** [daemon-authoritative, cat-1 Q4], drift-pinned to `shared/src/catalog.rs`); `describeRejection` routes the new `IpcErrorCode::not_found` (a READ-not-found) to the **honest-generic** treatment, never a fabricated re-approvable/hard-conflict card (forbidden #2). **6.3e proper (048, cat-1, `security-reviewer` PASS):** `DiffReview.tsx`'s Review tab sources from `get_diff` + renders a **dedicated per-hunk git-action bar** (stage/unstage/**discard** — the destructive discard explicitly danger-labeled, NOT the kit's PR-review bar) → each button submits a typed `git.*` `ActionRequest` over the seam → `DiffReview` opens its own `GatewayModal` for the new approval (the daemon's `PolicyDecision`/`ActionPreview`). The resource_ref is formed **verbatim from the displayed `Hunk`** (`{type:"file", id:"{wt}\x1f{file}\x1f{os,ol,ns,nl}"}`, `\x1f`=U+001F, **submitted==displayed** — the security pin [[19]]); the standing-grant stays **disabled** (own cat-1 checkpoint; discard is non-standing-grantable daemon-side). `enrichHunkAction` (`display-meta.ts`) is a **daemon-SHAPED stand-in** extending the 044 `gatewayApprovalEnrichment` [med] carry-forward (risk + approval_id both fixture — **swap for the real daemon projection/policy + preview RPC before any real human approves or any live `UdsGatewayPort` lands**). → [[16]], [[17]], [[19]]. |
| Terminal display surface (`gateway-client/types.ts` `subscribe_terminal` + `contracts/` `TerminalOutputFrame` + `views/terminal/{terminal-stream.ts,TerminalDisplay.tsx,SessionTerminal.tsx,session-lifecycle.ts}`) | §6.4, §9.1, §11 | The 6.3d terminal-well (P6.3d slice 3, **046**) — **DISPLAY-ONLY by safety #9**. `GatewayPort` gains `subscribe_terminal(terminal_id): AsyncIterable<TerminalOutputFrame>` (a display **READ** — **output-only**; the §6.4 terminal channel carries only the `terminal_output` ServerFrame). `TerminalOutputFrame` = a provisional shadow **extracted from the `ServerFrame` union** (one source), field-set drift-pinned. A **pure consumer** (`consumeTerminalFrame`: decode/seq/skip-undecodable/no-gap-fill) feeds an **xterm.js** host (visual-gated). **#9 pins** (`security-reviewer` PASS): no PTY input path, status never from output bytes (reads the Session projection), no invented transcript. **`TerminalProcessExited` is an EVENT→projection, NOT a ServerFrame** → ended-state reads the Session projection (a drift-pinned **ENDED/LIVE** status partition, [[5]]); the `exit_code`/`signal` detail + the live `UdsGatewayPort` terminal demux + per-frame/total buffer bounds + inbound `{pause}`/`{resume}` flow-control = **P4** (the fixture swaps for the live source there). **No `shared/` schema change** (the §6.4 contract is daemon-frozen; the UI consumes it). → [[18]]. |

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
| 9 | 2026-06-07 (P6.4 roving + P7.3 dropdown + P6.4 name-coverage extensions) | [A11y foundation](LESSONS.md#9) | Focus-visible ring = one global `a11y/focus.css :where(...):focus-visible` (kit tokens, not per-component, extend the selector for new element types); reduced-motion = the kit `motion.css` guard + `useReducedMotion()` for JS motion; every control keyboard-reachable (`tabIndex >= 0`), pinned by the multi-view reachability audit (§11.6 merge-gate net). **Composite widgets** use WAI-ARIA **roving tabindex** (one tabstop; Arrow/Home/End move + auto-activate; focused index read from the DOM at event time — never a stale closure; shared **orientation-aware** `nextTabIndex` in `a11y/roving.ts` — horizontal default for tablists, vertical for listboxes). **Dropdown** = `button[aria-haspopup=listbox]` trigger + roving `role="listbox"` popover (open-focuses-active via `useLayoutEffect` on the open transition; Enter/Escape return focus to trigger, click-outside doesn't; disabled at zero options). The audit (`a11y/reachability.ts`, vitest-free, throws) is **roving-aware** via a `{tab→tablist, option→listbox}` container map (a `tabIndex=-1` member is reachable iff its container has exactly one tabstop) + sweeps `role=option`/covers visible `role="tabpanel"`. Assumes flat containers (nested = YAGNI). **+ accessible-NAME coverage (P6.4):** `auditAccessibleNames` (in the combined `auditView`) asserts every interactive control has a non-empty name — `aria-label`/labelledby[aria-hidden-excluded]/non-hidden text incl `.sr-only`/title/input-`<label>`; `aria-hidden` text never counts; roving `-1` members name-skipped (tabstop still checked); name from a visually-hidden child, never a wrapper `aria-label` — the standing §11.6 net (0 caught first-run; no kit-contract change). |
| 10 | 2026-06-07 | [Green ≠ looks right (visual gate)](LESSONS.md#10) | TDD-exempt visual/theme layers aren't verified by green tests + clean build (jsdom/tsc/oxlint/build don't check appearance); require a rendered-product visual gate (gstack browser: dev server vs the kit Graphite Arc prototype `ui_kits/control-plane/index.html`) + explicit VISUAL acceptance. Never report a UI slice "looks right" from green tests alone. |
| 11 | 2026-06-08 | [Degraded + safety states always explained, fail-closed](LESSONS.md#11) | Every read-only or §15/§17 state renders a distinct display surface (transport-degraded / session-survival / §17 safety — never conflated); fail-closed (conflict never-auto-resolved #6, audit-integrity non-dismissible #5, read-only never silent); parked intents disabled-but-present (gated on `canSubmitIntent`, INV-SEC-1 stays daemon-side); glyph derived from severity (never color alone); reuse frozen enums via `.extract` + enumerate coverage from `.options` so a future state is forced to render. |
| 12 | 2026-06-08 | [Theme/visual layer — apply, ground, gate](LESSONS.md#12) | The theme layer APPLIES the kit SEMANTIC tokens via `ui/src/theme/{global,shell,components}.css` (semantic only, §11.1 re-hue — kit ships tokens-only); GROUND every surface against the prototype's ACTUAL rendering (its chrome is in kit components, not the index.html `<style>` — don't guess); the VISUAL GATE (dev server vs prototype over HTTP) is the acceptance + catches what tsc/oxlint/jsdom can't (the `*/`-in-CSS-comment drop, surface divergences); structural refactors preserve reachability/landmarks; severity color is ADDITIVE to glyph+label. Extends [[10]]. |
| 13 | 2026-06-08 (P6.4 view-history + P6.3 filtering extensions) | [UI selection/scope/nav/filter state over frozen/local state](LESSONS.md#13) | A UI selection/scope (active-project), **navigation** (view-history back/forward), or **filter** (table status/text) is plain UI state over the FROZEN projections / local view selection — NOT a mutation/provisional/Gateway-intent → no `canSubmitIntent` gate, no daemon dep (ships without the parked intent seam). Pure model + thin context/hook wrapper (mirror `ReadOnlyProvider`/`active-project.ts`); scoped views re-root/filter, cross-cutting triage (Command Center) stays GLOBAL; a stale selection re-scopes to a default (no ghost id); view-history uses browser semantics (forward-truncation, collapse-on-same); filtering composes **filter→sort**, derives options from the **unfiltered** set, **ORs independent fields** (never a joined string — cross-boundary false-positive), and renders **filtered-empty distinct from truly-empty**. **Wire-or-disable:** a named control is never a dead click — wire it or `disable` it (§11.6). Distinct from the daemon-1.5 mutation path. |
| 14 | 2026-06-12 | [Contract-bump regen discipline](LESSONS.md#14) | On a contract bump: regen `generated.ts` (never hand-edit, [[1]]) + **derive `validators` from the generated bundle** (`= shape`), never hand-list — the hand-list is exactly what drifted 0.12.0→0.19.0. An enum **rename** retargets delegations + `.extract` sources to the new `$def` name (never re-declare, [[2]]); UI status-machine identifiers stay **UI-render-policy** (decoupled from `$def` names via a `VALIDATOR_KEY` alias). A §2.5-seam type adopted **ahead of its consumer** (`ServerFrame`/`WireError`) carries a field-*type* schema-snapshot drift pin the same day (a field-name snapshot alone missed `rpc_response.id` being `integer` not `string`). `pin: generated.test.ts (member-set + validators-keys=$defs, now self-maintaining) + provisional.test.ts (serverframe variant field/id-type snapshot)`. |
| 15 | 2026-06-12 | [Required discriminator for a safety-gating field](LESSONS.md#15) | A discriminator that gates a fail-closed/critical UI state (e.g. `CreditPool.kind` gating `hard_stop` — SDK-only; the auto-resetting interactive pool never hard-stops, §9.1 two-pool) is a **required** field, never optional-with-default: `tsc`-enforced declaration beats a silent default that can render a **false** safety state (a forgotten `kind` silently → `"sdk"` → false `hard_stop` alarm, [[11]]). Model genuine absence as an explicit drift-pinned state. Extends [[11]]/[[13]]. `pin: views/usage/model.test.ts (kind-gated cases) + the required z.enum shape (tsc rejects a kind-less CreditPool)`. |
| 16 | 2026-06-12 | [The UI intent-submission seam (cat-1)](LESSONS.md#16) | The UI's mutation path is a `canSubmitIntent`-gated **PURE SUBMITTER** over the daemon's id-based §6.1 wire — submits only (no execution path, INV-SEC-1), no optimistic render (status from the daemon ack only), §6.4 codes verbatim, no intent state; the **daemon Gateway is the real chokepoint** (the UI gate is defense-in-depth). Frozen shapes = field-set-drift-pinned provisional shadows (`inputs=z.unknown()` passthrough). **Classify caught errors by `instanceof Error`, never by shape** (a frame is plain data; `.strict()` alone is insufficient — non-enumerable `Error` props → a colliding-code Error false-handled as `{error}`). `pin: intent/submit-intent.test.ts (7 cat-1 pins + drift-pin + re-throw pin); security-reviewer every intent slice`. |
| 17 | 2026-06-12 | [The intent-seam consumer — daemon-driven approval card (cat-1)](LESSONS.md#17) | The consumer (`GatewayModal`-real) is a **PURE RENDERER** of the daemon's `PolicyDecision`/`ActionPreview` (never UI-derived risk / invented preview), **daemon-status-never-"done"** (Q3); `safety/model.ts` `describeRejection` routes each §6.4 code to its **distinct §11.5 treatment** (`fencing_conflict` never re-approvable by construction #6; the unmapped/transport set → an honest **generic** rejection, never swallowed — a swallowed reject reads as pending/success, §11.7); a stays-disabled pin locks the deferred `policy_grant` "always allow" standing-grant; key preview fetches on **stable primitives**, not the per-render seam identity. Extends [[16]]. `pin: overlays/GatewayModal.test.tsx (10 modal pins) + the precondition_stale card test; security-reviewer every intent slice`. |
| 18 | 2026-06-13 | [The terminal-well display pattern (#9)](LESSONS.md#18) | A terminal-well is a **PURE frozen-frame consumer** (`consumeTerminalFrame`: base64-decode `terminal_output` → bytes → sink; skip-undecodable; monotonic `seq`, NO client gap-fill — recovery = reconnect) **split from a visual-gated xterm.js host**. **DISPLAY-ONLY #9** (3 pinned axes + `security-reviewer`): no PTY input path (`disableStdin`/no `onData`→gateway/no GatewayPort terminal-input method), status **never** from output bytes (reads the Session projection), frozen-frame fidelity (no invented transcript). **A UI subscription maps 1:1 to ONE daemon channel** — `subscribe_terminal` is **output-only** (the §6.4 channel carries only the `terminal_output` ServerFrame); **`TerminalProcessExited` is an EVENT (→ projection), NOT a ServerFrame** — so a merged output+exit stream invents the delivery topology + reworks at P4. Ended-state from a drift-pinned **ENDED/LIVE session-status partition** ([[5]]); the exit_code/signal detail + the live transport + inbound pause/resume are P4. The **visual gate caught a reachability gap** (fixture `terminalId` on non-sidebar sessions → unreachable despite green tests, [[10]]). `pin: views/terminal/{terminal-stream,TerminalDisplay,SessionTerminal,session-lifecycle}.test + TerminalOutputFrame drift-pin; security-reviewer (#9)`. |
| 19 | 2026-06-13 | [Resource-precise mutation: resource_ref verbatim from the displayed read (submitted == displayed)](LESSONS.md#19) | A per-hunk/per-resource mutation forms its `ResourceRef` **verbatim from the displayed `get_diff` `Hunk`** so submitted-target == displayed-target by construction (a mismatch irreversibly discards the WRONG content); conform to the FROZEN encoding (`{type:"file", id:"{wt}\x1f{file}\x1f{os,ol,ns,nl}"}`, `\x1f`=U+001F — the schema, NOT informal `resource_type:"File"` prose, [[14]]), conformance-pinned both sides; daemon-owned fields are placeholders (`action_request_id` daemon-mints, `risk_level` a never-displayed hint — the card renders the daemon `PolicyDecision`, [[17]]). Extends [[16]]/[[17]]. `pin: intent/hunk-resource-ref.test.ts + DiffReview.test.tsx (submitted==displayed); security-reviewer every mutation slice`. |

<!-- Starts empty. Each row links to its `LESSONS.md` anchor. -->

<!-- Slash commands: see root CLAUDE.md "Slash commands available." Implementer pair: /session-start + /session-end. -->

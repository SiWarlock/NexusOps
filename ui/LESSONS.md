# LESSONS.md — NexusOps (the Tauri desktop UI (TS frontend + thin Rust host))

> Full prose for every lesson logged during work in `ui/`. The compact index lives in `ui/CLAUDE.md` "Lessons logged" table.
>
> **Lesson numbers are stable IDs.** New lessons get the next sequential number. Numbers may be referenced from code comments, commit messages, and cross-references between lessons. **Don't reorder; don't reuse a deleted number's slot.**
>
> **Lessons start at §1.** Each code area has its own lesson sequence — lessons don't carry across code areas.

---

## Lesson format

```markdown
## <a id="N"></a>N. <Short topic> — <one-line rule>

**Date:** YYYY-MM-DD.
**Source slice:** <slice-id or commit hash>.

<2-5 paragraphs explaining: what was discovered, why it matters, how to
apply the rule, what edge cases are still open. Cite file:line references
where applicable.>

**Rule:** <one-sentence summary, same as the heading subtitle>.
```

---

## <a id="1"></a>1. UI contract enums are generated, never hand-declared — checked-in artifact + drift test + version pin

**Date:** 2026-06-07.
**Source slice:** P6.1a (`docs/briefs/003-P6-1a-ui-foundation-contract-layer.md`).

The cross-language contract authority is the Rust `shared` crate; it emits a versioned, CI-diff-gated JSON Schema (`shared/contracts/schema/nexusops-contract.schema.json`, `ARCHITECTURE.md §5.0` Option A). The UI is a **generated, drift-caught consumer** of that schema — it never re-declares status strings, ID kinds, or actor values by hand. `ui/scripts/gen-contracts.mjs` regenerates `ui/src/contracts/generated.ts` (a machine artifact — never hand-edited) from the frozen schema, read-only; `ui/src/contracts/index.ts` derives the individually-exported validators (`Session`, `Task`, …) from the generated bundle.

Three pins keep the generated layer honest and make hand-drift impossible: (1) **accept-all** — every canonical value of all 13 frozen value-sets parses; (2) **reject-unknown** — closed `z.enum`s reject any non-canonical value, preserving the `§5.0` pt4 / `§15`–`§17` fail-closed posture end-to-end; (3) **drift gate** — a test loads the frozen JSON at test time and asserts the exported validator *set* + each `.options` set === the schema `$defs[*].enum` arrays (the TS mirror of the Rust CI schema-diff gate, no `cargo` needed). A fourth pin couples versions: an exported `CONTRACT_VERSION` is asserted === the schema's `x-contract-version`, so the literal `"0.5.0"` used as a handshake tripwire elsewhere can never silently drift.

Apply this whenever a UI surface needs a contract value: import the generated validator, never type a status/ID/actor string union by hand. A status field on any view/projection type delegates to the generated enum. If the value you need isn't in the frozen schema, it isn't frozen yet — see [[2]].

**Rule:** UI contract enums are generated from the frozen `shared/` schema (checked-in artifact + accept/reject/drift/version pins), never hand-declared; status/ID/actor fields delegate to the generated validators.

## <a id="2"></a>2. Unfrozen object shapes are provisional UI-local types — enum fields delegated, reconcile on next contract bump

**Date:** 2026-06-07.
**Source slice:** P6.1a (`docs/briefs/003-P6-1a-ui-foundation-contract-layer.md`).

The 0.5.0 freeze is **enum-only** (9 status machines + `ActorType` + `IdKind` + `DesktopObjectKind`). The object models the UI must render against — projection-row shapes, `GatewayPort` params/results, `ActionRequest`/`ActionPlan` objects — live as `ARCHITECTURE.md` Appendix A *prose*, not yet a generated artifact (the daemon will freeze them as it builds Phase 1/2). The parallel-track plan (`MVP_TASKS.md §parallelization`; `§14` mandates a mock GatewayPort for UI tests) explicitly sanctions building the UI ahead of those frozen object schemas.

The resolution: hand-author **minimal provisional Zod/TS shapes** in `ui/src/contracts/provisional.ts`, each carrying a `// PROVISIONAL — not frozen; reconciles when the daemon freezes object schemas` banner, with **every enum-typed field delegating to the generated validators** from [[1]] (never a re-declared status union). This keeps the only hand-authored surface the *structure*, while the safety-load-bearing *values* stay generated + drift-pinned.

Posture note (object-KEY strictness): the boundary validator rejects unknown enum VALUES (fail-closed, [[1]]), but provisional object shapes are **tolerant of unknown object keys** on read projections (forward-compat / must-ignore-unknown) — `§15`'s reject-unknown invariant is about enum values, which are enforced; strict object-key rejection belongs on intent-submission params (mutation side), not read projections. Harden when object schemas freeze (carry-forward reconcile). When that bump lands, replace the provisional shapes with generated ones and delete the banner — track every provisional type so none survives the reconcile.

**Rule:** object shapes not yet in the frozen contract are provisional UI-local types (clearly banner-marked, enum fields delegated to the generated layer, tolerant of unknown keys on reads); reconcile to generated shapes on the next contract bump.

## <a id="3"></a>3. NexusOps-ui-kit consumption — tokens via styles.css, components via the `@ui-kit` source alias (not the global bundle)

**Date:** 2026-06-07.
**Source slice:** P6.1b (`docs/briefs/004-P6-1b-app-shell-and-kit-integration.md`).

`NexusOps-ui-kit` (the canonical design system, `§11.1` / O-5) ships three delivery forms: a token `styles.css` (`@import` manifest of `tokens/*.css`), per-component `.jsx` sources + `.d.ts` typings under `components/<group>/`, and a runtime bundle on a global namespace (`window.ControlPlaneDesignSystem_a21911`). For a React 19 + Vite + TS-strict app the chosen mechanism is: **(1)** import the token `styles.css` in `main.tsx` (CSS custom properties; product code references the **semantic** token layer only, never primitives); **(2)** import the component `.jsx` sources via a Vite **`@ui-kit` alias** → `../NexusOps-ui-kit/components`, surfaced through one re-export module (`ui/src/design-system/kit.ts`). The runtime global bundle is **not** used (it fights TS-strict ES modules); the kit is **not** vendored/copied (that would diverge from the canonical source and break the "a re-hue touches primitives only" promise).

The one non-obvious snag: the kit `.jsx` components `import React from 'react'` as a peer, but they live **out of `ui/`'s root** where there is no `node_modules` — so Vite couldn't resolve `react`/`react-dom` for them. The fix is `resolve.dedupe: ["react", "react-dom"]` in `vite.config.ts`, which forces those (and the injected `jsx-runtime`s, matched by package name) to resolve from *this* project's root — giving one React instance for app + kit. Supporting config: `server.fs.allow` must include the sibling kit dir; `tsconfig` `paths` mirror the alias; `skipLibCheck` accommodates the kit's hand-authored `.d.ts` (it skips declaration-file checking only — app-code strictness is unaffected); drop the deprecated `baseUrl` (TS6) — `paths` work under `moduleResolution: bundler`. Verified end-to-end by a scratch render + a full `vite build` (kit token CSS bundled). Some kit component contracts are still thin (e.g. `Button.d.ts` lacks `aria-label`) — fold those as they bite (see the 6.4 a11y task).

**Rule:** consume the kit as tokens-via-`styles.css` + components-via-the-`@ui-kit`-source-alias (with `resolve.dedupe(["react","react-dom"])` for the out-of-root peer react), never the global bundle and never vendored; reference the semantic token layer only.

## <a id="4"></a>4. The UI read-only/degraded gate is fail-safe + defense-in-depth — never the sole mutation guard

**Date:** 2026-06-07.
**Source slice:** P6.1c (`docs/briefs/005-P6-1c-connection-readonly-version-skew.md`).

The UI disables every intent-submitting control when the daemon is unreachable or version-skewed (`§11.4` global READ-ONLY degraded mode). Two properties make this safe rather than theater. **(1) Fail-safe by construction:** `canSubmitIntent = connected && version === compatible` (`ui/src/connection/read-only.ts`) — it is a *positive* confirmation, so every unknown/initial state falls out as FALSE, including the subtle `{connected, version=unknown}` pre-handshake window. The UI never offers a mutation until it has positively confirmed the daemon is reachable AND version-compatible; there is no "default to enabled." The `ReadOnlyProvider` with no provider also returns FALSE. **(2) Precedence:** `update_required` (version skew) outranks `disconnected`/`reconnecting` — a version mismatch is not Retry-able (`§16` is update/relaunch), so the degraded derivation checks version first and the banner shows a distinct update-required variant (no misleading transport-Retry).

Crucially this gate is **defense-in-depth, NOT the load-bearing guard**. The real INV-SEC-1 enforcement (no mutation bypasses the Action Gateway) is **daemon-side** (`§15`); the daemon rejects mutations regardless of UI state. The UI gate exists so the cockpit doesn't *offer* an action it knows can't be honored — a UX-integrity property, not the security boundary. Never let UI-side gating substitute for the daemon enforcement, and never weaken the fail-safe default for convenience. Enforced by `ui/CLAUDE.md` forbidden-pattern #6: every intent control consults `canSubmitIntent`. (Connection-state lives on the `gateway-client` transport seam — `getConnectionState`/`onConnectionChange`/`reconnect` — which are UI-client transport concerns, not the frozen `§6.1` RPC surface.)

**Rule:** the UI read-only gate is fail-safe (`canSubmitIntent` FALSE on unknown; true only when confirmed connected + version-compatible) and defense-in-depth — never the sole mutation guard; the daemon Gateway (INV-SEC-1) is.

## <a id="5"></a>5. Status→attention-rank is a (machine,status)-keyed table, drift-pinned to the generated enums, with no fall-through to idle

**Date:** 2026-06-07.
**Source slice:** P6.2a (`docs/briefs/006-P6-2a-status-model-attention-rank.md`).

`§11.3` requires "one canonical `status → attention-rank` table (covering all machines), the single source for sidebar weight, queue membership, and sort order — no silent fall-through to idle." Three properties make the UI implementation (`ui/src/status/descriptors.ts` + `attention.ts`) correct. **(1) Keyed by `(machine, status)`, not status alone** — status strings aren't globally unique (`active` is in Session/AgentTeam/WorkflowInstance; `failed`/`completed`/`archived` are shared), so a flat status→rank map would collide. **(2) Completeness drift-pinned to the generated enums** — a test iterates every frozen enum's `.options` (the [[1]] generated layer is the source of truth) and asserts a real descriptor exists for each `(machine, status)`; a daemon-added status fails the test until the table covers it. This is the same drift-gate philosophy as the contract layer, applied to render policy. **(3) No fall-through to idle** — the known kit bug (`waiting_on_permission`/`conflict`/`stale`/`blocked` flooring to rank 0 and never entering "Needs my attention") is pinned fixed: those map to 4/4/3/4, `changes_ready`=4 (review-needed), `waiting_on_human_input`/`waiting_on_human`=5; an out-of-table status resolves to a *visible* "unknown" descriptor (non-zero rank), never a silent idle.

The attention-rank lives in `ui/src/status/`, **not** `shared/` — it is **UI render policy** (how the cockpit prioritizes display: sidebar weight, the needs-attention/working/settled triage buckets, the `compareByAttention` sort), not a cross-language contract the daemon/Brain consume. If the notifier (`§10`) ever needs a canonical attention-rank it would be promoted to a shared contract via the daemon track — the UI track never writes the frozen `shared/` crate. Worktree status is **derived** from its two frozen axes (git + overlay) via a precedence function (`§5.1`/`§7.2`), not a stored enum.

**Rule:** the status→attention-rank table is `(machine,status)`-keyed, completeness-drift-pinned to the generated enums, the single source for sidebar/queue/sort, with no fall-through to idle; it is UI render policy in `ui/src/status/`, never the frozen `shared/` crate.

## <a id="6"></a>6. Status rendering = thin descriptor-bound wrappers over the kit; guard the kit's silent-idle fallback; kit props are closed

**Date:** 2026-06-07.
**Source slice:** P6.2b (`docs/briefs/007-P6-2b-status-rendering-binding.md`).

`StatusPill`/`AttentionMarker` (`ui/src/status/*.tsx`) are **thin wrappers** over the kit components, bound to the [[5]] descriptor table: `StatusPill(machine, status)` looks up `describeStatus` and renders the kit pill with the descriptor's `visualKind` + humanized `label`; the marker binds `attentionRank` → kit `level`. **Never re-declare a status's visual/label/rank at a call site** — the descriptor is the single source.

Two non-obvious guards. **(1) The kit's silent-idle trap:** the kit `StatusPill` does `STATUS[status] || STATUS.idle`, so an unrecognized kind renders **silently as idle** — exactly the `§11.3` fall-through the project forbids. The wrapper maps the descriptor's `"unknown"` fallback to a **visible `"degraded"`** kit kind, so an unknown status is always visible (pinned by a render test, not only the 6.2a model test). **(2) Drift-free kind validation:** the four-channel coverage test derives `KIT_KINDS` from the kit's **own exported `STATUS` map** (not a hand-maintained list), so the "every descriptor visualKind is a valid kit kind" guard can't rot. (Verified: all 19 descriptor visualKinds are valid kit `StatusKind`s — no `§11.7` mapping gap.)

**Kit components have CLOSED prop types** — no `HTMLAttributes`/`aria-*`/`data-*` passthrough (true for `Button`, `StatusPill`, `AttentionMarker`). So `data-*`, the R-5 surface chip ("Approval"/"Action"), and the `display:contents` marker wrapper all live on **NexusOps wrapper elements**, never on the kit component. Tracked as a `§11.7` component-contract item for the 6.4 a11y pass (HTMLAttributes passthrough) — until then, the wrapper pattern is the workaround.

**Refinement (P6.4e, 2026-06-08) — naming is NOT a wrapper job.** A wrapper carries `data-*` + *decorative* `aria-*` fine, but a wrapper `aria-label` does **NOT** name the inner kit control: an element's accessible **name** comes from its own content/aria, never an ancestor's `aria-label`. To NAME a closed-prop kit control (e.g. an icon-only `Button` with no `aria-label` prop), put a **visually-hidden child label *inside* it** — `<Button><span aria-hidden>glyph</span><span style={srOnly}>Name</span></Button>` → accessible name "Name"; verify with `getByRole({name})`. (My P6.4e brief said "aria-label on the wrapper" — wrong for naming; this is the correction.)

**Rule:** render status via thin descriptor-bound wrappers over the kit (single source = the descriptor); guard the kit's `||idle` silent fallback (unknown→visible); derive kit-kind validation from the kit's own `STATUS` map; route `data-*`/decorative `aria-*` onto wrapper elements (kit props are closed) — but the accessible **NAME** of a closed-prop kit control must come from a **visually-hidden child inside it**, never a wrapper `aria-label`.

## <a id="7"></a>7. Multi-commit ui slices: the implementer idles after each layer commit — the orchestrator drives layer→layer

**Date:** 2026-06-07.
**Source slice:** ui-track Phase 6 round 1 (operational note; carried into round 2).

A ui `/tdd` slice that spans multiple commits (e.g. a refactor/extraction layer followed by the feature layer) does **not** run end-to-end autonomously on the implementer side: after each layer's Step-10 commit the implementer's turn ends and it goes **idle**, waiting for the next instruction. It does not self-advance to the next layer of the same brief.

So the orchestrator must **drive the slice layer-by-layer**. When the implementer reports a layer's commit (a `TaskUpdate` + one-line wake), the orchestrator immediately wakes it onto the next layer of the *same* brief (a terse "proceed to layer 2: <what>" via `SendMessage`) rather than treating the layer commit as the whole slice landing and moving on. Never let a multi-commit slice sit idle mid-slice between its layers. Slice atomicity (root `CLAUDE.md`) still holds — the *current layer* always finishes through its Step-10 commit; the driving happens at the clean commit boundary *between* layers, never as a mid-layer interrupt.

Practical consequence for brief authoring: when a brief estimates ≥2 commits, **enumerate the layers explicitly** (layer 1 = …, layer 2 = …) in the "Estimated commit count" section, so both sides know the layer boundaries up front and the orchestrator knows how many layer→layer wakes to expect.

**Rule:** for multi-commit ui slices the implementer idles after each layer commit; the orchestrator drives it layer→layer (one wake per layer, at the commit boundary), and briefs estimating ≥2 commits enumerate the layers explicitly.

## <a id="8"></a>8. Projection→item mappers are the single source for the `ProjectionItem` shape; `data-item-id` is namespaced `<namespace>:<id>` on every emitter

**Date:** 2026-06-07.
**Source slice:** P6.3b Layer 1 (`docs/briefs/009-P6-3b-project-graph-a11y-fallback.md`).

Two related conventions settled once the Command Center (6.3a), the sidebar (6.2b chrome), and the Project Graph (6.3b) all needed the same `{id, label, machine, status}` item shape derived from projection rows.

**(1) Mapper-as-single-source.** `ui/src/projections/items.ts` owns the projection→item mappers (`toSessionItems`/`toPrItems`/`toApprovalItems`), each producing one generic `ProjectionItem` shape; `CommandItem`/`SidebarItem` are aliases of it. Views and chrome **consume the mappers — they never re-map a projection row → item shape inline at a call site.** Before the extraction the Shell mapped `data.sessions` → items at three sites (sidebar, command items, and the would-be graph nodes) — a silent drift hazard the moment one site's mapping diverged. `ProjectionItem.machine`/`status` stay typed `string` for now (the descriptor table is string-keyed with an unknown→degraded fallback — [[6]]); narrowing them to the generated machine-name/status enum unions folds into the provisional→generated reconcile at the daemon object-schema freeze (tracked in `MVP_TASKS.md` Carry-forward).

**(2) Namespaced locator.** Every `data-item-id` emitter namespaces its locator as `<namespace>:<id>`, **never a bare id** — `${machine}:${id}` for status items (Command Center, sidebar), `${type}:${id}` for graph nodes (the machine-less project root). The namespace is whatever dimension disambiguates a same-id/different-kind collision. **One rule, no special-case emitter:** even where a collision is impossible by construction (the sidebar is sessions-only, so all its ids share `machine="Session"`), the locator is still namespaced — so a test or the PRD §25 demo that locates by `data-item-id` never has to remember an exception. (The React `key` was already collision-safe via `machine:id`; this brings the DOM locator to the same footing.)

Apply both whenever a new view needs projection-derived items: import the mapper (add one to `items.ts` if a new projection appears), and namespace every `data-item-id`. The graph view (6.3b L2) and 6.3c–e are the first consumers.

**Rule:** projection→item mappers in `items.ts` are the single source for the `ProjectionItem {id,label,machine,status}` shape (no inline re-mapping); `data-item-id` is namespaced `<namespace>:<id>` on every emitter (machine for status items, type for graph nodes), one rule with no exceptions.

## <a id="9"></a>9. A11y foundation — global focus-visible ring (one stylesheet, kit tokens), reduced-motion via the kit guard + `useReducedMotion`, every control keyboard-reachable

**Date:** 2026-06-07.
**Source slice:** P6.4a (`docs/briefs/011-P6-4a-a11y-musts-focus-reduced-motion.md`).

The §11.6 a11y MUSTs are merge-gates (PRD §14.8). Three patterns settled when applying the daemon-independent ones (Decision-C reorder).

**(1) Focus-visible ring = ONE global stylesheet, never per-component.** `ui/src/a11y/focus.css` carries a single `:where(button, a[href], [role="button"], [tabindex]:not([tabindex="-1"]), input, select, summary):focus-visible` rule applying the kit ring tokens (`--focus-ring` / `--ring-w` / `--ring-offset`), imported in `main.tsx` after the kit `styles.css`. `:where()` keeps specificity 0 so a component can override if it must; the selector enumerates the interactive element types. **Maintenance:** extend the `:where(...)` selector when a NEW interactive element type lands — `textarea`/`contenteditable` are deliberately omitted today (no such control exists; the PTY terminal + a deny-with-reason `textarea` arrive with the **parked 6.3d**). Never add per-component focus styles; the global rule is the single source.

**(2) Reduced-motion = the kit's global guard + a hook for JS-gated motion.** The kit `tokens/motion.css` already ships a global `@media (prefers-reduced-motion: reduce)` guard that neutralizes ALL animation/transition app-wide — so **CSS** motion is handled for free (just verify `motion.css` is in the `styles.css` import chain — [[3]]). For **JS-driven** motion (conditionally rendering/animating in JS), gate on `useReducedMotion()` (`ui/src/a11y/useReducedMotion.ts`): reads `matchMedia('(prefers-reduced-motion: reduce)')`, change-subscribed, **defaults `false` (motion allowed)** when `matchMedia` is absent (reduced-motion is an explicit OS opt-in; absence ≠ reduce). No animated consumer exists yet — gate the FIRST one (live-pulse / attention-beacon / overlay entrances) through the hook.

**(3) Keyboard-reachability is the substance the ring decorates — pin it with a multi-view audit.** Every interactive control must be keyboard-focusable (a semantic element, or a `role` on a natively-focusable element; never `tabindex="-1"` on an actionable, never a `div`/`span`-onClick). `reachability.test.tsx` is the §11.6 **merge-gate NET**: it sweeps the rendered `<Shell/>` across ALL content views (CC → Graph → Sessions via the view-switch) so a non-focusable control added to ANY view is caught by ONE test, not left to each slice to remember. Unify the focusability check on **`el.tabIndex >= 0`** (one robust predicate that also catches a non-focusable `div[role="button"]`). Keep each view's audit non-vacuous (it must actually find controls).

**Rule:** apply the focus-visible ring globally via one `a11y/focus.css :where(...):focus-visible` (kit tokens, never per-component; extend the selector for new element types); handle reduced-motion via the kit's global `motion.css` guard + `useReducedMotion()` for JS-gated motion; keep every interactive control keyboard-reachable (`tabIndex >= 0`), pinned by the multi-view reachability audit (the §11.6 merge-gate net).

## <a id="10"></a>10. Green tests + clean build ≠ "looks right" — TDD-exempt visual/theme layers need a rendered-product visual gate

**Date:** 2026-06-07.
**Source slice:** USER FINDING (post-6.4b) — the app rendered functionally-correct but **completely unstyled**.

A visual/theme layer is **TDD-exempt** (per the root `CLAUDE.md` TDD posture, visual rendering is covered by fixtures/review, not failing unit tests). The trap: the standard "green" gates verify everything EXCEPT appearance. **jsdom render tests** assert DOM structure/presence (a control renders, a `data-item-id` is right) but **never compute CSS/layout** (jsdom has no layout engine — we already knew this for `focus.css`/`reachability.test.tsx`); **`tsc`/`oxlint`/`vite build`** verify types/lint/bundling, not pixels. So a slice can be **fully green + build-clean while the visual deliverable is absent or wrong**.

This is exactly what happened across 6.1–6.4: the kit `styles.css` only **defines** tokens (`:root { --vars }`) — nothing in the app **applies** them (no `body{}` background/font, no layout grid, no panel chrome). Only the kit's self-theming components (StatusPill/AttentionMarker) rendered themed; the whole app shell + views fell back to browser defaults (serif, white, no layout). Every slice was "complete + verified" for FUNCTIONAL/structural correctness while the **Graphite Arc visual layer was never built or verified.**

**Rule:** for any TDD-exempt visual/theme layer, "green tests + clean build" is **NOT** verification — it must be an **explicit tracked deliverable with VISUAL acceptance**, gated by a **rendered-product check**: run the actual app and compare against the design reference. NexusOps automates this as a **gstack-browser visual gate** — the running dev server vs the runnable Graphite Arc prototype (`NexusOps-ui-kit/ui_kits/control-plane/index.html`) via `/browse` · `/design-review` · `/qa` — run at phase boundaries + when a styling slice lands. A lightweight wiring-guard (assert the theme stylesheet is imported + applies `body` styling) catches gross absence in unit tests but **does not replace** the rendered comparison. Never report a UI slice "done/looks right" from green tests alone.

## <a id="11"></a>11. Degraded + safety states are always EXPLAINED, fail-closed — distinct surfaces, never silent / color-alone / auto-resolving

**Date:** 2026-06-08.
**Source slices:** P6.4d-2 (§17 safety-state display — L1 fencing/hard-conflict card `ff2f8d6`, L2 fail-closed/audit-integrity alert `503b6a2`) + P6.4 checking/handshaking banner (`5f40149`).

Every state where the UI is **read-only** or a **§15/§17 safety invariant is in play** must render an **explicit, distinct display surface** — never silent, never color-alone, never auto-resolving what the daemon must resolve. The UI *displays* the safety state; enforcement stays daemon-side (INV-SEC-1). Five binding parts:

1. **Distinct surfaces, never conflated.** Three+ separate concerns get separate surfaces: **transport-degraded** (`DegradedBanner`, 6.1c) · **session-survival** (`RecoveryBanner`, 6.4d) · **§17 safety-state** (`HardConflictCard` / `AuditIntegrityAlert`, 6.4d-2). The "checking" handshake window is its own `DegradedBanner` variant. Don't fold one into another (a render test asserts each surface is distinct — `data-degraded` ≠ `data-recovery-kind` ≠ `data-conflict-reason` ≠ `data-treatment`).
2. **Fail-closed display.** Conflict cards are **never-auto-resolved** (#6 — the card offers NO auto-resolve path; the "never auto-resolved" copy must be true of every state it shows, so scope it to the genuine hard conflict `fencing_conflict`, NOT a re-approvable `stale_precondition`). Audit-integrity alerts are **non-dismissible** (#5 — no local dismiss; the signal must be seen). Read-only is **never silently unexplained** — every `canSubmitIntent===FALSE` state has a `DegradedState` banner variant (the "checking" connected+version-unknown window was the gap).
3. **Parked intents rendered disabled-but-present.** Resolution / acknowledge / restart are daemon-1.5 **intents** — rendered **disabled**, gated on `canSubmitIntent` (forbidden #6); the UX is complete (the affordance is visible) but no mutation is offered. A present-but-disabled control is tracked, not a false wire.
4. **Non-color channels carry meaning.** glyph + label + severity, never color alone (§11.6 / forbidden #5); **derive the glyph FROM the severity** so the two can't drift.
5. **Drift-pinned reuse + completeness.** Reuse frozen enums via `z.enum().extract([...])` (no re-declared values — throws at load if the frozen enum renames; Lesson [[2]]); model provisional shapes ONLY for genuinely net-new states; **enumerate display coverage from `.options`** (completeness test) so a future-added state is *forced* to render, never silently dropped — a silently-unrendered safety signal is itself a #5 violation.

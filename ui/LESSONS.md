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

**Kit components have CLOSED prop types** — no `HTMLAttributes`/`aria-*`/`data-*` passthrough (true for `Button`, `StatusPill`, `AttentionMarker`). So `data-*`, the R-5 surface chip ("Approval"/"Action"), `aria-label`, and the `display:contents` marker wrapper all live on **NexusOps wrapper elements**, never on the kit component. Tracked as a `§11.7` component-contract item for the 6.4 a11y pass (HTMLAttributes passthrough) — until then, the wrapper pattern is the workaround.

**Rule:** render status via thin descriptor-bound wrappers over the kit (single source = the descriptor); guard the kit's `||idle` silent fallback (unknown→visible); derive kit-kind validation from the kit's own `STATUS` map; route `aria-*`/`data-*` onto wrapper elements (kit props are closed).

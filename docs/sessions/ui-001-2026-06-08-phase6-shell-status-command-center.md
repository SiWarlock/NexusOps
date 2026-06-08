# Session ui-001 — Phase 6 UI track: shell, status model, Command Center

- **Date:** 2026-06-08
- **Track / area:** `ui` (track-namespaced numbering — `ui-NNN` — to avoid colliding with the daemon/shared track's `001`)
- **Phase:** 6 (Frontend shell & projection-driven UI) — slices 6.1a/b/c, 6.2a/b, 6.3a
- **Role:** ui-implementer (team `nexusops-ui`)
- **Predecessor:** `001-2026-06-07-phase0-spikes-and-contract-freeze.md` (the 0.5 contract freeze `06f9576` that unblocked this track) — cross-track dependency, not an in-track predecessor
- **Successor session:** [ui-002](ui-002-2026-06-08-phase6-graph-sessions-a11y-usage-settings-survival-topbar.md) (6.3b/6.3c + 6.4a/b/c/d/e — Graph · Sessions · a11y · Usage · Settings · Survival · TopBar-nav)

## Why this session existed

Phase 0 froze the `shared/` contracts (0.5.0) and the `/arch-finalize` re-validation confirmed no frozen contract moved, opening the `ui` track to build Phase 6 in parallel against the frozen interface + a mock GatewayPort + the existing `NexusOps-ui-kit`. This session stood the track up from a bare area (CLAUDE.md + LESSONS.md only) through the shell, the status model + rendering, and the Command Center landing view.

## What was built

Six slices, each a `/tdd` cycle (RED → Step-2.5 review → GREEN → reviewers → commit):

| Slice | Commit | What landed |
|---|---|---|
| 6.1a | `fd9738b` | Vite+React 19+Vitest+oxlint (strict) scaffold; generated Zod contract layer (checked-in artifact + drift test + CONTRACT_VERSION pin); gateway-client seam (parse-don't-trust boundary + MockGatewayPort read surface) |
| 6.1b | `39a87c6` | App shell chrome (TopBar/ProjectSwitcher/Sidebar/DrawerStack/ActivityDock/StatusBar); `@ui-kit` source-alias integration + `resolve.dedupe`; projection→chrome derivations; LIFO drawer-stack reducer; **closed 6.1a's reachability gap** |
| 6.1c | `402f4c5` | Daemon-connection state machine; **fail-safe** `canSubmitIntent` read-only gate; version-skew (`checkVersionCompat` + update-required precedence); ConnectionIndicator + DegradedBanner; **security-reviewer PASS** |
| 6.2a | `b32c3c0` | `(machine,status)→{attentionRank,visualKind,label}` descriptor table for all 113 frozen states (drift-pinned); `needsMyAttention`/triage buckets/`compareByAttention`; Worktree two-axis precedence |
| 6.2b | `e2cebbc` | StatusPill/AttentionMarker kit wrappers (four-channel coverage; unknown→degraded guard); Approval/ActionRequest two-surface (R-5); sidebar attention-ordering + needs-attention count |
| 6.3a | `144b6b6` | Command Center triage view (Changes-ready extracted + needs/working/settled; default content view) |

**Files created (by area):**
- **Scaffold/config:** `ui/package.json`, `tsconfig.json`, `vite.config.ts`, `vitest.config.ts`, `.oxlintrc.json`, `.npmrc`, `.gitignore`, `index.html`, `pnpm-lock.yaml`, `src/main.tsx`, `src/vite-env.d.ts`, `scripts/gen-contracts.mjs`.
- **Contracts:** `src/contracts/{generated.ts (artifact), provisional.ts, index.ts}` (+ tests).
- **Gateway-client:** `src/gateway-client/{types.ts, boundary.ts, mock.ts}` (+ tests).
- **Connection (6.1c):** `src/connection/{state.ts, version.ts, read-only.ts, ConnectionIndicator.tsx, DegradedBanner.tsx}` (+ tests).
- **Status (6.2):** `src/status/{attention.ts, descriptors.ts, worktree.ts, StatusPill.tsx, AttentionMarker.tsx}` (+ tests).
- **Shell:** `src/shell/{Shell.tsx, TopBar.tsx, ProjectSwitcher.tsx, Sidebar.tsx, DrawerStack.tsx, ActivityDock.tsx, StatusBar.tsx}` (+ derive.ts, drawer-stack.ts + tests).
- **Views:** `src/views/command/{group.ts, CommandCenter.tsx}` (+ tests).
- **Fixtures:** `src/projections/fixtures/{proj_session, proj_project_activity, proj_pull_request, proj_approval_queue, proj_audit_trail}.ts`.
- **Design-system:** `src/design-system/kit.ts`.

**Files modified across slices:** the gateway-client/shell files were extended slice-over-slice (boundary PAGE_SCHEMAS, mock connection sim, Shell content/ReadOnlyProvider/sidebar/CommandCenter wiring, Sidebar attention wiring).

## Decisions made

- **Contract enums GENERATED from the frozen schema** (checked-in artifact + drift test + CONTRACT_VERSION pin), never hand-declared; status fields delegate to the generated validators.
- **kit consumption = `@ui-kit` source alias + `resolve.dedupe(["react","react-dom"])`** (the kit `.jsx` imports react as an out-of-root peer; dedupe resolves it from the app root — Context7-confirmed), NOT the global runtime bundle, NOT vendored. Tokens via the kit `styles.css`.
- **Object reads tolerant of unknown keys** (RATIFIED A-now → strict-at-freeze); enum values stay closed/reject-unknown.
- **Read-only gate is fail-safe** (`canSubmitIntent = connected && version===compatible`; FALSE on any unknown/initial) and **defense-in-depth** — the daemon Gateway remains the load-bearing INV-SEC-1 guard.
- **Attention-rank table is UI render policy → `ui/src/status/`, NOT `shared/`.** Keyed by `(machine,status)`; the four no-fall-through states (`waiting_on_permission`/`conflicts`/`blocked`/`stale`) + `changes_ready` never floor to idle.
- **Command Center: `changes_ready` extracted** (rank-consistent, disjoint, surfaced first); sources = sessions+PRs+approvals; default content view.
- **Toolchain:** pnpm restored via `npm i -g pnpm` (corepack shim broken); `.npmrc verify-deps-before-run=false` (intermittent pre-run crash).

## Decisions explicitly NOT made (deferred)

- **`.strict()` on provisional object reads** — ratified tolerant-now; hardens at object-schema freeze.
- **ExecutionProfile descriptors/pill** — held (0.5b); land at its freeze (6.4 Settings).
- **`SUPPORTED_PROTOCOL_RANGE` doc-anchor** — pinned `{1,1}` UI-side; reconciles against the daemon §6.4 handshake (daemon-1.5), not unilaterally written into the binding doc.
- **Real `UdsGatewayPort`** — interface + mock only; integrates at daemon 1.5.
- **Real Repair/update-relaunch flow** — aliases reconnect for now (daemon-1.5/Phase 10).

## TDD compliance

**Clean.** Every slice was strict test-first — RED confirmed (missing-module / assertion failure for the right reason) before GREEN, at every `/tdd` Step 3. No test written after implementation. No safety-critical TDD skips. 6.1c (the safety slice) ran security-reviewer (PASS) + code-quality; all other slices ran code-quality (every-slice). Reviewer findings folded in-slice or flagged.

## Cross-doc invariant audit

**No violation.** This track introduces **no new frozen contract** — the generated Zod layer is a drift-caught *consumer* of the daemon-authored frozen schema (the drift/completeness tests are the enforcement). Provisional UI-local object shapes + the attention-rank table are explicitly non-frozen (UI render policy). No frozen `shared/` contract field changed this session. The `ui/CLAUDE.md` cross-doc rows (generated layer §5.0/§5.1; attention-rank §11.3) + `ui/LESSONS.md` entries are **orchestrator-owned** and were routed hot (ui/CLAUDE.md + ui/LESSONS.md show as modified in the working tree — the orch's edits, not staged here).

## Reachability (from `/tdd` Step 7.5)

- **6.1a** generated layer + boundary + mock — reachable-by-6.1b (foundation gap) → **CLOSED** by 6.1b.
- **6.1b** `main.tsx → <Shell/> → gateway-client → boundary → generated contracts` — real render path (vite build).
- **6.1c** `Shell → ReadOnlyProvider/DegradedBanner` + `StatusBar → ConnectionIndicator`; Retry/Repair → `client.reconnect()`. `canSubmitIntent` predicate reachable **by-6.3+ intent controls** (tracked via the new forbidden-pattern — the gate exists + is consulted by the read-only context; the intent-submitting controls that must consult it don't exist yet, tracked not silently-unreachable).
- **6.2a** model — consumed-by-6.2b; the completeness test is its production-relevant drift guard.
- **6.2b** `Shell → Sidebar → StatusPill/AttentionMarker` — real path (vite build).
- **6.3a** `Shell content pane → CommandCenter` (default view) → StatusPill/AttentionMarker — real path (vite build).

**No tested-but-unwired gaps.** (The only "reachable-by-next-slice" item is `canSubmitIntent`'s future intent-control consumers — tracked.)

## Open follow-ups (orchestrator-routed hot during the session; captured here)

- **Provisional reconcile @ object-schema freeze (Phase 1/2):** SessionRow, ProjectActivityRow, PullRequestRow, ApprovalQueueRow, AuditEventRow, and `ProjectionDelta.row` (Session-specific → projection-discriminated) + `.strict()` posture.
- **6.4 a11y carry-forward:** closed kit-component props (no aria-*/data-* passthrough — Button history controls, the R-5 surface chip) + focus ring / drag→non-drag / reduced-motion MUSTs.
- **Toolchain runbook:** pnpm corepack restore + the `.npmrc verify-deps-before-run=false` note.
- **6.3 locator convention:** namespace `data-item-id` as `machine:id` (matching the React key) consistently across 6.3 views before the §25 demo.
- **6.3b:** extract a shared `SessionItem` shape/helper (sidebar + command items duplicate the sessions map).
- **Later integration:** real `UdsGatewayPort` (daemon 1.5) → Shell default client + mock dev/test-only; `SUPPORTED_PROTOCOL_RANGE` reconcile; "checking/handshaking" degraded banner variant; `compareByAttention` narrow to `AttentionRank` if synthetic ranks appear; `useMemo` sidebar/command items at scale.
- **ExecutionProfile** descriptors/pill at 0.5b freeze.

## How to use what was built

- Regenerate the contract layer after a schema bump: `cd ui && pnpm gen:contracts` (reads the frozen schema read-only; the drift test fails if `generated.ts` drifts).
- Gate: `cd ui && pnpm test:run && pnpm typecheck && pnpm oxlint` (or the `node_modules/.bin/*` direct binaries). Build: `pnpm build`.
- Build new screens against `MockGatewayPort` + the fixtures; render statuses via `StatusPill`/`AttentionMarker` (never re-declare a status's visual/label/rank — the descriptor table is the single source); gate intent controls on `useCanSubmitIntent()`.

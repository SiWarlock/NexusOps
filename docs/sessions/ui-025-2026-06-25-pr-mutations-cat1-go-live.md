# ui-025 — PR-mutations cat-1 go-live flip (both actions) + dev-shell visual gate

- **Date:** 2026-06-25
- **Phase:** 7 (P7.4 — the §11.2 PR Review Workspace mutation path go-live) · track `ui`
- **Predecessor:** [ui-024](ui-024-2026-06-21-phase8-plan-modal-and-brain-status-binding.md)
- **Successor:** _(next ui session)_
- **Commits:** `caf3d32` (ui-075 C1, dev-shell visual-gate harness, NON-cat-1) · `b0ddc39` (ui-075 C2, the cat-1 go-live flip) — both LOCAL on `track/ui`, push HELD.

## Why this session existed

The whole PR Review Workspace mutation path was already built **guarded-disabled** (ui-070 `github.merge_pr` + ui-071 `github.submit_review`, both cat-1, `enabledPrMutations` EMPTY in production → no live mutation; `head_sha` made real @ui-072). The only remaining gates to going live were USER-side: the cat-1 sign-off + a visual gate. Both daemon prerequisites had landed (the auth-bootstrap 083, CONTRACT 0.45, on `track/ui` post-sync). The USER granted the cat-1 sign-off (both merge + submit-review, both-at-once). This session ran the **visual gate** (against a daemon-free Mock dev shell) and then landed the **production flip**.

Structured as **two commits with a HITL visual gate between them** (the safety-critical cat-1 flip never bundled with anything else):
- Commit 1 (NON-cat-1) = a daemon-free visual-gate harness so the controls could be pixel-checked WITHOUT a live daemon, then PAUSE.
- HITL checkpoint = the user ran the Mock dev shell + signed off ("looks good. pass.").
- Commit 2 (cat-1) = the production flip, landed only after the orchestrator relayed the PASS.

## What was built

### Commit 1 — `caf3d32` — dev-shell Mock visual-gate harness (NON-cat-1)

**Files created:**
- `ui/src/main.mock.test.tsx` — 4 vitest tests pinning the entry seam: `main_default_path_uses_production_uds_no_mock` (env-unset → no Mock), `main_mock_env_injects_mock_gateway` (`VITE_NEXUSOPS_MOCK="1"` → MockGatewayPort), `main_falsy_string_env_fails_closed_to_production` (the `"0"`/`"false"` fail-closed pin), `mock_pr_fixture_enables_pr_controls` (the fixture head_sha → Merge + Approve enabled under the Mock).

**Files modified:**
- `ui/src/main.tsx` — NEW exported `resolveEntryGateway()`: a build-time `import.meta.env.VITE_NEXUSOPS_MOCK`-gated Mock-injection branch wired into the bootstrap (`<Shell gateway={resolveEntryGateway()} />`). Returns a `MockGatewayPort` ONLY when the env is exactly `"1"` (an EXPLICIT allowlist, not truthiness — see Decisions); unset / any other value → `undefined` → the production `UdsGatewayPort`. Vite inlines `import.meta.env.VITE_*` as a build-time literal → a clean production build statically evaluates this to `return undefined` and tree-shakes the Mock branch + its import out of the bundle (empirically verified: 0 `MockGatewayPort`/`VITE_NEXUSOPS_MOCK` in `dist/`).
- `ui/src/projections/fixtures/proj_pull_request.ts` — `head_sha` (40-char hex) added to all 3 fixture PR rows so the Mock-backed PR Review Workspace renders Merge + the 3 verdict controls ENABLED (the gate's pixel-check surface). No contract impact (`head_sha` already on `PullRequestRow` @0.44; `z.string().nullable().optional()`).
- `ui/package.json` — `+dev:mock` script (`VITE_NEXUSOPS_MOCK=1 vite`), the gate-runner convenience.

### Commit 2 — `b0ddc39` — the cat-1 PR-mutations go-live flip

**Files modified:**
- `ui/src/shell/Shell.tsx` — THE FLIP: the production default port now constructs `new UdsGatewayPort({ mutationsEnabled: true, enabledPrMutations: PR_MUTATION_ACTION_TYPES })` (+ the `PR_MUTATION_ACTION_TYPES` import). Lights up BOTH `github.merge_pr` + `github.submit_review` at once. The PrWorkspace Merge + the 3 verdict controls now enable on a connected + version-compatible daemon serving a head_sha'd PR; a click reaches the live `gateway_submit_action` transport.
- `ui/src/shell/Shell.uds-swap.test.tsx` — 4 NEW production-shell pins via a `productionShellAtPrWorkspace()` helper (Merge enabled / Approve enabled / Merge-click → live invoke / Approve-click → live invoke). Written in C1 as a `describe.skip` block (kept C1's suite green); un-skipped in C2 together with the flip (RED-first confirmed). + a `gateway_get_diff` helper branch to silence navigation noise.
- `ui/src/intent/pr-mutation-request.test.ts` — the ui-070/071 held-flip guard UPDATED (not retired): `production_construction_never_enables_pr_mutations` → `pr_mutation_flip_confined_to_the_signed_off_shell_go_live`. The source-grep that asserted ZERO production `enabledPrMutations:` flips now asserts EXACTLY `["src/shell/Shell.tsx"]` (the sole signed-off go-live site); any OTHER production flip still fails it. A `[medium]` code-quality catch was fixed in-slice: the Shell.tsx doc-comment originally repeated the literal `enabledPrMutations: PR_MUTATION_ACTION_TYPES`, which the guard's grep ALSO matched (a comment alone could satisfy the guard) → the colon-value form was stripped from the comment so the guard matches only the real code flip.

## Decisions made

- **Both-at-once, reuse `PR_MUTATION_ACTION_TYPES`** (the full-set constant) — the user ruled both merge + submit-review go live together; reusing the constant means no drift between the gate set and the enabled set.
- **Explicit `=== "1"` allowlist, not a truthiness guard** (security-reviewer [low] fix-in-slice). A string env value like `"0"`/`"false"` is JS-truthy, so a truthiness guard would INVERT a "disable" attempt into a Mock-in-prod leak. The allowlist fails CLOSED to the production port for any value that isn't exactly `"1"`. Pinned by `main_falsy_string_env_fails_closed_to_production`.
- **Build-time env-gated Mock dev entry as the visual-gate mechanism** (brief Q1 (a)) — `resolveEntryGateway()` exported so it's unit-testable without mounting (the bootstrap is `if(rootEl)`-guarded; jsdom has no `#root`). This RETIRES the "no Mock-injection in main.tsx → the visual gate is a manual operator step" caveat ([[22]]/[[23]]) for surfaces that need ENABLED controls without a live daemon.
- **head_sha on all 3 fixture rows** (brief Q2) — any selected PR shows enabled controls for the gate.
- **The held-flip guard is UPDATED, not retired** — a go-live moves the assertion from "zero production flips" to "the Shell, and ONLY the Shell" so the guard stays load-bearing against a rogue/second/accidental flip.
- **`describe.skip` then un-skip** for the C2 pins (brief Q4) — keeps C1's suite fully green while the cat-1 GREEN is held across the HITL checkpoint; C2 un-skips + flips together (genuinely test-driven, RED-first confirmed).

## Decisions explicitly NOT made (deferred)

- **The real-daemon live-write operator walkthrough** (a real Merge against a throwaway repo) — the user's separate live-validation step; the UI flip is necessary-not-sufficient (live writes additionally require the daemon-side per-connection `live_writes_enabled` toggle ON, default OFF, 083 + Connect-via-gh). Out of scope here.
- **Per-hunk inline review `comments[]` (D10 follow-on)** — a §4.7 follow-on, unchanged.
- **A bundle-grep CI gate** asserting `MockGatewayPort` absent from `vite build` output — flagged as a §10.6 hardening Future-TODO (the prod-grep already shows it'd be green today).
- **Helper unification + live `CONTRACT_VERSION` in the production-shell test helpers** — cosmetic; deferred (the helpers reuse `"0.31.0"`, harmless — `checkVersionCompat` keys only on `protocol_version`).
- **The `policy_grant` "always allow" standing-grant** — stays its own cat-1, disabled. Unchanged.

## TDD compliance

**CLEAN.** Every code change was test-first:
- `resolveEntryGateway()` — tests written + RED confirmed (`resolveEntryGateway is not a function`) before the impl.
- fixture `head_sha` — `mock_pr_fixture_enables_pr_controls` RED (`prHeadSha` null) before the fixture edit.
- the `=== "1"` hardening — `main_falsy_string_env_fails_closed_to_production` RED (truthiness guard injects the Mock on `"0"`) before the guard tightening.
- the Shell flip — the 4 production-shell pins RED-first confirmed (Merge `disabled: true` with `enabledPrMutations` empty) before the flip → GREEN.
- the held-flip guard update + the comment-strip — test-file / comment-only changes driven by the flip + a Step-8 review finding (not impl-before-test); the guard verifies a single code match.

## Reachability

- **Commit 1** — `resolveEntryGateway()` reachable from the production entry: `main.tsx` bootstrap `<Shell gateway={resolveEntryGateway()} />` (build-time env-gated; unset → `undefined` → the production `UdsGatewayPort`).
- **Commit 2** — the flip reachable from the production entry: `main.tsx` → `Shell` → `DiffReview`/`PrWorkspaceContainer` → the live `UdsGatewayPort.submit_action`. The 4 production-shell pins ARE the reachability proof (controls enable + clicks reach `invoke("gateway_submit_action")`). No tested-but-unwired gaps.

## Open follow-ups

Step-9 categorized items (routed hot to the orchestrator during the session; it writes the docs at `/orchestrate-end`):
- **Convention candidate / LESSON** — extends [[28]] (the L2-C single-flag go-live) to the per-action PR-mutation set, and adds the **held-flip-guard → single-sanctioned-site** pattern (a go-live updates the zero-flips guard to a single-allowed-site guard, never retires it) + the **build-time env-gated Mock dev-entry visual-gate** pattern (`VITE_NEXUSOPS_MOCK==="1"`, prod tree-shaken). (Orchestrator banks at `/orchestrate-end`.)
- **Architecture-doc note** — §11.2 as-built: the PR Review Workspace mutation controls are LIVE in production (both merge + review), gated by `enabledPrMutations` (UI defense-in-depth) + the daemon `live_writes_enabled` toggle. (Orchestrator.)
- **Future TODO (hardening, §10.6)** — a bundle-grep CI gate asserting `MockGatewayPort` absent from the `vite build` output (would be green today per the prod-grep).
- **Future TODO (cosmetic)** — unify `productionShellAtPrWorkspace`/`productionShellAtCodeView` mock boilerplate + use the live `CONTRACT_VERSION` in both helpers.
- **Cross-doc invariant change** — NONE (no `shared/` model field add/remove/rename; `head_sha` already on `PullRequestRow` @0.44; the fixture is test/dev infra). Confirmed at Step 9.
- **Accepted residual (security [low])** — the held-flip source-grep guard can't catch a dynamically-assembled options object; pre-existing/accepted ([[27]]).
- **The §7.4 "PR-mutations go-live flip" tick** + any plan reconcile ride the orchestrator's `/orchestrate-end`.

## How to use what was built

- **Run the daemon-free visual gate:** `cd ui && pnpm dev:mock` (= `VITE_NEXUSOPS_MOCK=1 vite`) → in the app: sidebar **Code / Diff Review** → **Pull requests** tab → click a PR → the Merge + Approve/Request-changes/Comment controls render enabled (Request-changes/Comment need a non-empty body); clicking opens the GatewayModal approval card. The production build (`pnpm build`) tree-shakes the Mock out entirely.
- **The production cockpit** (`pnpm tauri dev` / a real build) now exposes live Merge + review — gated by a connected daemon + the daemon-side `live_writes_enabled` toggle.

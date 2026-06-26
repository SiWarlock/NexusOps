# /tdd brief — pr_mutations_go_live_flip

## Feature
The **cat-1 PR-mutations go-live flip** (USER-signed-off 2026-06-25): enable **both** PR mutations
(`github.merge_pr` + `github.submit_review`) at once in the production cockpit by populating the
production `UdsGatewayPort`'s per-action `enabledPrMutations` gate — **preceded by a HITL visual gate
run against a Mock-backed DEV SHELL**. The flip lands ONLY after the user passes the visual gate.

## Use case + traceability
- **Task ID:** P7.4 (the "PR-mutations go-live flip (cat-1, HELD)" line, `IMPLEMENTATION_PLAN.md` section 7.4) — completes the `§7.2` PR Review Workspace mutation path.
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (PR Review Workspace), `§7.2` (GitHub PR cache / re-fetch-before-merge), `§6.3` (the `github.*` catalog).
- **Widens phase scope because** the go-live rides the cross-cutting **frozen Gateway/mutation contract** the §11.2 PR Workspace mutation path inherently consumes — `§6.1` (id-based intent seam), `§6.4` (wire codes verbatim), `§11.4`/`§11.5` (READ-ONLY/degraded gating + the Gateway modal), `§15` (INV-SEC-1, the daemon Gateway chokepoint; the UI gate is defense-in-depth). These are Phase-2 frozen contracts the UI consumes — the same widening ui-070/071 used to build the guarded path. No new code in those subsystems; this slice removes a UI-side per-action guard.
- **Related context:** this is the **direct analogue of the L2-C go-live** ([[28]] — "the cat-1 go-live = a single flag flip gated on explicit user sign-off + a live-verification operator gate"). The whole PR-mutation workspace is already built guarded-disabled (ui-070 `github.merge_pr` + ui-071 `github.submit_review`, [[27]]); `head_sha` is real (ui-072); the daemon auth-bootstrap landed (083, CONTRACT 0.45, on track/ui post-sync). The ONLY remaining gates were the user's cat-1 sign-off (GRANTED) + the visual gate (this slice runs it).

**USER rulings (relayed via the lead, 2026-06-25) — already settled, NOT Step-2.5 questions:**
- **Enable BOTH at once** (merge + submit-review) — not staged.
- **Flip NOW** — the cockpit is the live-validation vehicle (use a throwaway repo/PR for the first real write).
- **Visual gate runs BEFORE the flip lands, against the DEV SHELL** (HITL) — Merge + 3 verdict controls + body input + approval modal vs the prototype.

## Acceptance criteria (what "done" means)

**Commit 1 — the dev-shell visual-gate harness (NON-cat-1):**
- [ ] `main.tsx` gains a **build-time env-gated** Mock-injection branch: when `import.meta.env.VITE_NEXUSOPS_MOCK` is set, the entry renders `<Shell gateway={new MockGatewayPort(...)} />`; when unset (the default / every production build) it renders the unchanged production `<Shell/>` (UdsGatewayPort, `mutationsEnabled:true`). The env read is a **build-time literal** (Vite inlines `import.meta.env.VITE_*`) so a production build **tree-shakes the Mock branch to dead code**.
- [ ] The mock PR fixture (`proj_pull_request.ts`) exposes a non-null `head_sha` on its PR row(s) so that, under the Mock (`enabledPrMutations` = full set, `connection: "connected"`), `prHeadSha(pr) != null` ⇒ the PR-workspace **Merge + 3 verdict controls enable** (the gate's pixel-check surface).
- [ ] A test pins: the **default (env-unset) main path constructs no Mock** (production = UdsGatewayPort); the env-set path injects the Mock. The fixture `head_sha` makes `canMerge`/`canReview` true under the Mock-backed workspace.

**→ HITL CHECKPOINT — the visual gate (runs BETWEEN commit 1 and commit 2):** the user runs the Mock dev shell (`VITE_NEXUSOPS_MOCK=1 pnpm dev`), navigates Code/Diff → "Pull requests" → a PR, and pixel-checks **Merge + Approve / Request-changes / Comment + the body `<textarea>` + the GatewayModal** against the Graphite-Arc prototype. **The implementer pauses after commit 1** and pings the orchestrator (Step 7.5) → orchestrator escalates lead→user → the user runs + signs off the gate. **Do NOT land commit 2 until the orchestrator relays the visual-gate PASS.**

**Commit 2 — the production go-live flip (cat-1, security-reviewer):**
- [ ] `Shell.tsx` constructs `new UdsGatewayPort({ mutationsEnabled: true, enabledPrMutations: PR_MUTATION_ACTION_TYPES })` — **both** `github.merge_pr` + `github.submit_review` enabled (reuse the existing `PR_MUTATION_ACTION_TYPES` constant; do not inline a partial set).
- [ ] Production-shell pins (mirror `Shell.uds-swap.test.tsx` [[28]]): with a connected + version-compatible daemon serving a PR row carrying `head_sha`, (a) the **Merge** control enables, (b) a **verdict** control (Approve) enables, (c) a click on Merge reaches the live `invoke("gateway_submit_action", …)`, (d) a click on Approve reaches `invoke("gateway_submit_action", …)`. An enabled control PROVES the production port carries the action_type in `enabledPrMutations` (the only path to enablement is `canSubmit && isPrMutationEnabled(gateway, type) && headSha != null`).
- [ ] The defense-in-depth frame is preserved + un-weakened: the daemon Gateway stays the INV-SEC-1 chokepoint; `canSubmitIntent` fail-safe; no optimistic "merged"/"reviewed"; §6.4 codes verbatim; the `policy_grant` standing-grant stays disabled. The UI flip is **necessary-not-sufficient** — live writes additionally require the daemon-side per-connection `live_writes_enabled` toggle ON (default OFF, 083) + Connect-via-gh (runtime user steps; out of scope here).
- [ ] `/preflight` clean; security-reviewer CLEAR (the `invariant` policy — this is a §15-touching go-live).

## Wiring / entry point (Step 7.5)
- **Commit 1:** `src/main.tsx` (the dev entry) — the Mock branch is reachable via the `VITE_NEXUSOPS_MOCK` build env; the default path is unchanged production `<Shell/>`.
- **Commit 2:** `src/shell/Shell.tsx:133` (the production entry) — the flip lights up the already-wired `PrWorkspaceContainer` controls (`canMerge`/`canReview` in `views/code/DiffReview.tsx`) + the live `UdsGatewayPort.submit_action` transport. No new component wiring — the controls + container already exist (ui-070/071); this slice removes the per-action guard for both types.

## Files expected to touch
**Modified:**
- `src/main.tsx` — the build-time env-gated Mock-injection dev branch (commit 1).
- `src/projections/fixtures/proj_pull_request.ts` — add `head_sha` to the PR fixture row(s) (commit 1).
- `src/shell/Shell.tsx` — the `enabledPrMutations: PR_MUTATION_ACTION_TYPES` flip (commit 2).
- `src/shell/Shell.uds-swap.test.tsx` (or a sibling `Shell.pr-golive.test.tsx`) — the production-shell pins (commit 2 RED).

**New (optional, Step-2.5):**
- a `dev:mock` script in `ui/package.json` (convenience for the gate runner) — flag if added.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)

**Commit 1 — `src/main.tsx` / fixture (a `main.mock.test.tsx` or extend an existing entry test):**
1. **`main_default_path_uses_production_uds_no_mock`** — Asserts: with `VITE_NEXUSOPS_MOCK` unset, the resolved entry gateway is the production `UdsGatewayPort` (no Mock construction). Why: §15 — the production build must never inject a Mock (the dev path is build-time-gated, tree-shaken in prod).
2. **`main_mock_env_injects_mock_gateway`** — Asserts: with the env flag set, the entry resolves a `MockGatewayPort`. Why: the gate harness exists + is env-isolated.
3. **`mock_pr_fixture_enables_pr_controls`** — Asserts: under the Mock (connected, `enabledPrMutations` full) the fixture PR's `prHeadSha != null` ⇒ `canMerge && canReview` true (the controls enable). Why: the visual gate needs the controls rendered enabled.

**Commit 2 — `src/shell/Shell.uds-swap.test.tsx` (production-shell pins):**
4. **`production_shell_enables_pr_merge_when_connected`** — Asserts: `<Shell/>` (no gateway → production port) at the PR workspace, with a connected daemon serving a head_sha'd PR, the **Merge** control is enabled. Why: [[28]] — an enabled control proves `enabledPrMutations` carries `github.merge_pr`.
5. **`production_shell_enables_pr_review_when_connected`** — Asserts: the **Approve** verdict control is enabled. Why: proves `enabledPrMutations` carries `github.submit_review` (both-at-once).
6. **`production_shell_pr_merge_click_reaches_live_transport`** — Asserts: clicking Merge → `invoke("gateway_submit_action", { request: … })`. Why: §6.1 — a real cockpit action reaches the live mutation transport.
7. **`production_shell_pr_review_click_reaches_live_transport`** — Asserts: clicking Approve → `invoke("gateway_submit_action", …)`. Why: the second mutation reaches the live transport.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — consumes the frozen 0.45 surface; no `shared/` change, no CONTRACT bump, no regen.
- **Orchestrator doc rows to write hot (Step 9 routing):** tick the P7.4 "PR-mutations go-live flip" `[ ]`→`[x]` + §7.2 mutation note (go-live LANDED); a LESSON candidate extending [[28]] (the PR-mutation go-live = the per-action `enabledPrMutations` both-flip gated on the visual gate + the daemon-side `live_writes_enabled` toggle; the build-time-gated Mock dev-shell visual-gate harness pattern). The orchestrator updates the `ui/CLAUDE.md` cross-doc "GatewayPort mutation surface" row (the go-live as-built).
- **Shared-contract seam model touched?** No — no Appendix-A model field changes.

## Things to flag at Step 2.5
1. **Dev-mock injection mechanism.** (a) a build-time `import.meta.env.VITE_NEXUSOPS_MOCK`-gated branch in `main.tsx`; (b) a separate dev-only entry (`main.mock.tsx` + a Vite input). My default vote: **(a) env-gated `main.tsx` branch** — simplest, build-time-static so prod tree-shakes the Mock; pin the prod-default-no-Mock test. Confirm the env name (`VITE_NEXUSOPS_MOCK`).
2. **Mock fixture `head_sha` scope.** Add `head_sha` to one PR row or all three? My default vote: **all three fixture PRs** (any selected PR shows enabled controls for the gate), realistic 40-char SHA strings. No contract impact (the field is `z.string().nullable().optional()`).
3. **Both-at-once enablement value.** Pass `PR_MUTATION_ACTION_TYPES` (the existing full-set constant) vs an inline `new Set([...])`. My default vote: **reuse `PR_MUTATION_ACTION_TYPES`** — the user ruled both-at-once; the constant IS both; avoids a drift between the gate set and the enabled set.
4. **Commit structure + the HITL ordering.** My default: **2 commits with the visual gate between** — commit 1 (harness, NON-cat-1) lands; the implementer **pauses + pings the orchestrator**; after the orchestrator relays the visual-gate PASS, commit 2 (the cat-1 flip + pins) lands (its OWN commit per the safety-critical rule). The flip's RED pins (4–7) are written + Step-2.5-reviewed in THIS cycle but GREEN-landed only post-gate. Confirm you're OK holding the cat-1 GREEN across the HITL checkpoint.

## Dependencies + sequencing
- **Depends on:** ui-070/071 (the guarded PR-mutation workspace — built), ui-072 (`prHeadSha` real), the track/ui ← main 0.45 sync (DONE this session), the daemon auth-bootstrap 083 (on track/ui post-sync). The USER cat-1 sign-off (GRANTED).
- **Blocks:** the live PR-mutation operator validation (the user's throwaway-repo first write); the P4.7 per-hunk inline `comments[]` follow-on.

## Estimated commit count
**2.** Commit 1 = the dev-shell visual-gate harness (NON-cat-1: `main.tsx` env-branch + fixture head_sha). Commit 2 = the cat-1 production flip + production-shell pins (its OWN commit — safety-critical; security-reviewer). The HITL visual gate runs between them.

## Lessons-logged candidates anticipated
- **Convention candidate** — the PR-mutation go-live = the per-action `enabledPrMutations` flip (both action types) gated on the HITL visual gate + (runtime) the daemon-side per-connection `live_writes_enabled` toggle; the UI flip is defense-in-depth, necessary-not-sufficient. Extends [[28]] (the L2-C go-live) to the per-action PR-mutation set.
- **Convention candidate** — a daemon-free visual gate uses a **build-time env-gated Mock-injection dev entry** (`import.meta.env.VITE_*`, prod tree-shaken), retiring the "manual operator step, no Mock-injection in main.tsx" caveat ([[22]]/[[23]]) for surfaces that need enabled controls without a live daemon.
- **Architecture-doc note candidate** — §11.2 as-built: the PR Review Workspace mutation controls are LIVE in production (both merge + review), gated by `enabledPrMutations` (UI) + the daemon `live_writes_enabled` toggle.

## How to invoke
1. **Read this brief end-to-end.** The Step-2.5 questions need answers before GREEN; note the HITL visual-gate checkpoint between the two commits.
2. **Run `/tdd pr_mutations_go_live_flip`** in the implementer session.
3. **Step 2.5** — ping the orchestrator with the test-design write-up + answers/defaults to the 4 questions.
4. **After commit 1** — pause + ping the orchestrator (Step 7.5) for the visual gate; do NOT land commit 2 until the PASS is relayed.
5. **Step 9** — surface anything beyond the anticipated lessons-logged candidates; this is a §15 cat-1 slice → the security-reviewer runs on commit 2.

# /tdd brief — get_pr_diff_pr_code_diff_detail_view

## Feature
Wire the **D7 `get_pr_diff`** read RPC end-to-end (the existing `get_diff` transport pattern, mirrored) and **consume it** in the PR Review Workspace — render the real PR code-diff (head-vs-base hunks), retiring the D7 `pr-diff-unavailable` placeholder. READ-ONLY, NON-cat-1 (no mutation; per-hunk actions on a PR diff are a future cat-1, D7-gated).

## Use case + traceability
- **Task ID:** P7.2 (the ui-half Full PR Review Workspace; consumes the daemon P4.7 / D7 work-order item) — see `docs/planning/daemon-unblock-work-order.md` "D7".
- **Architecture sections it implements:** `ARCHITECTURE.md §11.2` (PR Review Workspace — the code-diff panel), `§7.2` (PR is GitHub-authoritative / SoT).
- **Widens phase scope because:** this P7.2 slice extends the **§6.1** GatewayPort read surface + the **§6.4** wire client + the **§5.0** frozen-contract (de)serialization — the established UdsGatewayPort transport-client anchors (the same cross-phase set the Phase-6 generated-Zod / transport rows already carry; "§6.1/§6.4/§5.0 added 2026-06-13 — the live UdsGatewayPort transport consumes them"). No new frozen contract — `get_pr_diff` + `GetPrDiffParams` + `DiffResult` are already frozen @0.40 (landed via the daemon D7).
- **Related context:** the existing `get_diff` plumbing this MIRRORS exactly — `ui/gateway-uds/src/lib.rs:229` (`get_diff` crate helper) · `ui/src-tauri/src/commands.rs:120-176` (`get_diff_params` + `gateway_get_diff`) · `ui/src/gateway-client/types.ts:65` (GatewayPort `get_diff`) · `uds.ts:168` (UdsGatewayPort `get_diff` via `invokeRead(parseDiff,…)`) · `mock.ts:192-197` (Mock `get_diff` fixture) · `boundary.ts:91` (`parseDiff` — REUSED, get_pr_diff returns the same `DiffResult`). The consumer: `DiffReview.tsx` (owns the gateway + the get_diff fetch/loading/ready render at :121-258; renders `<PrWorkspace pr={selectedPr} reviews=… onBack=…/>`) + `PrWorkspace.tsx:91-101` (the `pr-diff-unavailable` placeholder to retire). LESSONS [[20]]/[[21]]/[[22]] (transport-client layering), [[18]]/[[19]] (diff consume), ui-064 (`PrWorkspace` is a PURE display component — NO gateway prop, the no-mutation-reach structural pin).

## The frozen surface this consumes (verified in the merged tree, 2026-06-20)
`shared/src/ipc.rs:255` `GetPrDiffParams { repo_id: String, pr_number: u64, file: Option<String> }` → returns the **REUSED** `DiffResult { hunks: Vec<Hunk> }` (head-vs-base; `file: None` = whole changeset, all files' hunks FLATTENED with NO per-file attribution — a per-file file-tree is a post-D7 follow-on). Daemon RPC: `daemon/src/ipc/methods.rs:749` `get_pr_diff` (resolves `(repo_id, pr_number) → owner/repo` then fetches from GitHub). The UI already shadows `DiffResult`/`Hunk`/`DiffLine` (6.3e) — **no new shadow, no regen, no contract bump**.

## Acceptance criteria (what "done" means)
**Transport (mirror `get_diff`):**
- [ ] `ui/gateway-uds/src/lib.rs` gains `get_pr_diff(stream, repo_id, pr_number, file: Option<&str>, id) → DiffResult` — forms `GetPrDiffParams` + `call` + deserialize `DiffResult` (mirror `get_diff` at :229; import `GetPrDiffParams` from `nexusops-shared`).
- [ ] `ui/src-tauri/src/commands.rs` gains `get_pr_diff_params(repo_id, pr_number, file: Option<String>)` + `#[tauri::command] gateway_get_pr_diff` calling `call_daemon("get_pr_diff", params)`; **registered in the `lib.rs` allowlist** (the registered set IS the allowlist — [[21]]; still NO generic `gateway_call`).
- [ ] `ui/src/gateway-client/types.ts` GatewayPort gains `get_pr_diff(repo_id: string, pr_number: number, file: string | null): Promise<DiffResult>`.
- [ ] `ui/src/gateway-client/uds.ts` UdsGatewayPort implements it via `invokeRead(parseDiff, "gateway_get_pr_diff", {…})` (REUSE `parseDiff`).
- [ ] `ui/src/gateway-client/mock.ts` MockGatewayPort serves a CONTRACT-shaped `get_pr_diff` fixture through the frozen `DiffResult` shadow (mirror the `get_diff` fixture).

**Consumer:**
- [ ] `DiffReview.tsx` fetches `get_pr_diff(pr.repo_id, pr.pr_number)` for the selected PR (mirror its `get_diff` loading/ready/error state machine) and passes the `DiffResult` (+ loading/error states) DOWN to `PrWorkspace` as props — **`PrWorkspace` stays a pure-display component (no gateway prop — the ui-064 no-mutation-reach pin preserved)**.
- [ ] `PrWorkspace.tsx` retires the `pr-diff-unavailable` placeholder (lines 91-101) and renders the passed-in PR code-diff — REUSE the hunk/line render (the kit `DiffHunk` mapping, DiffReview:245-258) but **WITHOUT** the per-hunk git-action bar (`HunkGitActions` is worktree-scoped + cat-1; PR-per-hunk is a future cat-1 — render read-only).
- [ ] **Honest states (§11.7):** a `WireError` (e.g. `not_found`) → an honest "PR diff unavailable" state (code verbatim, never fabricated); a null `repo_id` or `pr_number` → don't fetch, render an honest "no repo link / PR number" state (distinct from a daemon error).
- [ ] **Visual gate** (the standing UI gate [[10]]/[[12]]) — the PR code-diff matches the prototype's Review-tab diff (dev server vs `kit-views2.jsx`); flag for lead/visual sign-off (manual operator step — no headless render of the production cockpit).
- [ ] `/preflight` clean (vitest + the gateway-uds crate tests + the src-tauri tests).

## Wiring / entry point (Step 7.5)
The PR code-diff is reachable from the **Review-tab PR Workspace**: `DiffReview` (the Review tab, already wired) → on a selected PR (`selectedPr` from the live PullRequest projection) fetches `get_pr_diff` → renders via `PrWorkspace`. Confirm at Step 7.5 the production path: a real PR selection in the cockpit triggers the live `gateway_get_pr_diff` invoke through `UdsGatewayPort` (not just Mock/test). The crate `get_pr_diff` + the Tauri command are exposed-ahead in Layer 1; the consumer wiring lands in Layer 2.

## Files expected to touch
**Modified:**
- `ui/gateway-uds/src/lib.rs` — `get_pr_diff` helper + tests (mirror `get_diff_returns_typed_diffresult` / `_malformed_result_is_serde_error`).
- `ui/src-tauri/src/commands.rs` — `get_pr_diff_params` + `gateway_get_pr_diff` + param-match test; `ui/src-tauri/src/lib.rs` — register the command.
- `ui/src/gateway-client/types.ts` — GatewayPort `get_pr_diff` signature.
- `ui/src/gateway-client/uds.ts` — UdsGatewayPort `get_pr_diff` (+ test: invoke + boundary-parse; not-wired methods stay not-wired).
- `ui/src/gateway-client/mock.ts` — Mock `get_pr_diff` fixture.
- `ui/src/views/code/DiffReview.tsx` — fetch `get_pr_diff` for the selected PR + pass diff/loading/error to `PrWorkspace`.
- `ui/src/views/code/PrWorkspace.tsx` — retire `pr-diff-unavailable`; render the passed-in PR code-diff (read-only, no action bar).
- `ui/src/views/code/PrWorkspace.test.tsx` + `DiffReview.test.tsx` — consumer tests.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
**Transport:**
1. `get_pr_diff_returns_typed_diffresult` (`gateway-uds/src/lib.rs`) — Asserts: forms `GetPrDiffParams{repo_id,pr_number,file}`, method == `"get_pr_diff"`, deserializes `DiffResult`. Why: §6.1 (mirror `get_diff`).
2. `get_pr_diff_malformed_result_is_serde_error` (same) — Asserts: a malformed result → serde error (fail-closed). Why: [[20]] boundary discipline.
3. `get_pr_diff_params_match_daemon` (`commands.rs`) — Asserts: `get_pr_diff_params(...)` == the daemon's frozen `GetPrDiffParams`. Why: §6.1 marshal conformance.
4. `uds_get_pr_diff_invokes_and_parses` (`uds.ts`) — Asserts: invokes `gateway_get_pr_diff` + `parseDiff`; a `WireError` → plain (not Error); a transport fault → Error. Why: [[22]] parse-don't-trust / wire-vs-transport.

**Consumer:**
5. `pr_workspace_renders_pr_code_diff` (`PrWorkspace.test.tsx`) — Asserts: given a `DiffResult` prop, renders the hunks/lines AND `pr-diff-unavailable` is gone; NO `HunkGitActions` bar (read-only). Why: §11.2 D7 consumption.
6. `pr_workspace_pr_diff_honest_unavailable_on_error` (same) — Asserts: a WireError state → honest "PR diff unavailable" (code verbatim), never a fabricated diff. Why: §11.7/forbidden #2.
7. `pr_workspace_no_fetch_when_repo_or_pr_number_null` (`DiffReview.test.tsx`) — Asserts: a null `repo_id`/`pr_number` → no `get_pr_diff` call + an honest "no repo link" state. Why: §11.7 honest-degrade.
8. `diff_review_fetches_pr_diff_for_selected_pr` (`DiffReview.test.tsx`) — Asserts: selecting a PR calls `get_pr_diff(repo_id,pr_number)` once; the result flows to `PrWorkspace`; `PrWorkspace` takes NO gateway prop (the no-mutation-reach pin). Why: §11.2 + ui-064 structural pin.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (consumes the already-frozen `GetPrDiffParams`/`DiffResult` @0.40 — no shadow, no regen, no contract bump).
- **§2.5-seam (shared-contract) model touched?** No new/extended shadow — `DiffResult` is already drift-pinned (6.3e). No new schema-snapshot test.
- **Orchestrator doc rows to write hot (Step 9 routing):** the `ui/CLAUDE.md` "Live UdsGatewayPort transport client" + "Tauri host + read-command bridge" rows gain a `get_pr_diff` read-method note (a cross-doc note, NOT a frozen-contract change). **Orchestrator-territory — flag at Step 9; do NOT edit `ui/CLAUDE.md` yourself.**

## Things to flag at Step 2.5
1. **Fetch ownership.** (a) `DiffReview` fetches `get_pr_diff` + passes the diff down to `PrWorkspace` (props) — preserves `PrWorkspace`'s pure-display / no-gateway pin (ui-064); vs (b) `PrWorkspace` self-fetches via a narrowed `Pick<GatewayPort,"get_pr_diff">`. My default vote: **(a)** — `DiffReview` already owns the gateway + the get_diff fetch pattern, and (a) keeps the ui-064 structural guarantee intact.
2. **Per-hunk actions on the PR diff.** The worktree diff has stage/unstage/discard (`HunkGitActions`). My default vote: **render the PR code-diff READ-ONLY (no action bar)** — those ops are worktree-scoped; PR-per-hunk is a future cat-1 (D7-gated per the work-order). Reuse only the hunk/line render.
3. **`file` filter.** `get_pr_diff(file?)` — None = whole changeset (flattened, no per-file attribution). My default vote: **fetch the whole changeset (`file=null`)** for the Review tab; a per-file file-tree is a post-D7 follow-on (flagged in the work-order).
4. **Null `repo_id`/`pr_number`.** My default vote: **don't fetch; render an honest "no repo link / PR number" state** distinct from a daemon error (forbidden #2 no-fabrication).
5. **Commit count.** 2 layers (transport exposed-ahead, then consumer). My default vote: **2** (the implementer may split the transport into Rust-then-TS if it judges it cleaner).

## Dependencies + sequencing
- **Depends on:** ui-068 (the 0.42 boundary regen + the PullRequestRow consume — landed `1eaf496`); the daemon D7 `get_pr_diff` RPC (landed via merge).
- **Blocks:** the **D9/D10** cat-1 PR-mutations arc (the PR-per-hunk Accept/Reject/Request-fix rides this code-diff — but that's the future cat-1, escalated lead→user). The per-file file-tree follow-on.

## Estimated commit count
**2** — a 2-layer slice (LESSON §7; I drive layer→layer):
- **Layer 1 — transport (exposed-ahead):** the `gateway-uds` crate `get_pr_diff` + the Tauri `gateway_get_pr_diff` command + allowlist + the TS GatewayPort/UdsGatewayPort/Mock `get_pr_diff` — all reusing `call`/`invokeRead`/`parseDiff`. No consumer yet.
- **Layer 2 — consumer:** `DiffReview` fetches + passes down; `PrWorkspace` retires `pr-diff-unavailable` + renders the read-only PR code-diff; the honest states; the visual-gate flag.
Read-only, NON-cat-1, same feature → a clean 2-commit slice (not separate briefs). The implementer may split Layer 1 into Rust-then-TS if it prefers (3 commits).

## Lessons-logged candidates anticipated
- **Convention candidate** — "A new GatewayPort READ method mirrors `get_diff` across all 5 layers (crate helper → Tauri command+allowlist → TS port → Mock → boundary-reuse) and is exposed-ahead before the consumer; reuse `parseDiff` when the return shape is an already-frozen `DiffResult`." (likely a refinement of [[20]]/[[21]]/[[22]], not a new lesson.)
- **Architecture-doc note candidate** — the PR code-diff is a flat changeset (no per-file attribution); a per-file file-tree needs a daemon follow-on.
- **Future TODO — next-brief working set** — PR-per-hunk Accept/Reject/Request-fix (rides this code-diff) = part of the D9/D10 cat-1 arc; the per-file file-tree follow-on.

## How to invoke
1. Read this brief end-to-end (don't skip "Things to flag at Step 2.5").
2. Run `/tdd get_pr_diff_pr_code_diff_detail_view`.
3. Step 0 (Restate) — confirm it matches the Feature line.
4. Step 2.5 — ping back the test design (one `Asserts: <invariant> (§anchor)` per test + the coverage map) with answers to the 5 design questions (or take defaults).
5. Step 9 — surface the cross-doc note (the two `ui/CLAUDE.md` transport rows) + anything beyond the anticipated lessons.

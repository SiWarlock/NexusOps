# /tdd brief — per_hunk_git_actions_and_diff_rpc

## Feature
Freeze the cross-track **ui-6.3e unlock**: the 3 per-hunk git mutation action types (`git.stage_hunk`/`git.unstage_hunk` risk-2; `git.discard_hunk` risk-3 + non-standing-grantable + diff-preview) with full §6.3 catalog + policy bindings, the **non-standing-grant policy mechanism** (a risk-3 action that's always per-action-approved), and a **hunk-structured `get_diff` read RPC** (request-on-demand, git2-live). One additive `CONTRACT_VERSION` bump (0.27.0→0.28.0). The git executor BODIES stay stubs (via the R1-A registry seam) until the real git executor lands Phase 5. **USER-ruled design (present, 2026-06-13).**

## Use case + traceability
- **Task ID:** P4.0b-ui1
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (the ActionTypeCatalog — the 3 git.* types + the policy bindings), `§6.1` (the GatewayPort read surface — the `get_diff` RPC), `§6.2` (the policy engine — the non-standing-grant floor). Touches §7.2 (git2 live-read precedent) + §15 (forbidden #6 — git mutations via git-CLI Gateway actions).
- **Phase-scope note — this brief WIDENS phase scope because** §6.3/§6.1 here serve a **ui-track consumer (6.3e per-hunk git + diff review)**, batched into Phase 4 by the cross-track timing (the next freeze task after 4.0b-2, like edges-R1). The git executor bodies are Phase 5; this freezes the contract the ui builds against.
- **Related context:**
  - **USER rulings (present, 2026-06-13):** `git.discard_hunk` = **risk-3 + NON-standing-grantable + `preview_class=diff`** (destructive, irreversible content loss; the policy engine REFUSES to standing-grant it — the §6.3 `workflow.command.invoke` floor mechanism; the preview shows EXACTLY the hunk discarded) · `git.stage_hunk`/`git.unstage_hunk` = **risk-2** (the default, not the risk-1 alternative) · `apply_hunk`/`revert_hunk` = **OUT** · the diff source = **a hunk-structured `get_diff` RPC, NOT a `Diff` projection** (request-on-demand, git2-live).
  - **Precedents:** the §6.3 catalog (`shared/src/catalog.rs`, LESSON 19) + `workflow.command.invoke` risk-4 floor (the non-standing-grant precedent) · the R1-A registration seam (the git executor registers here when real, Phase 5) · the worktree git-axis live-read (§7.2, the `get_diff` reads git2 on demand).
  - **Consumer:** the ui's parked **6.3e** (Code/Diff review + per-hunk actions) resumes against this frozen contract (regen ui-Zod → build the per-hunk action submission + the diff render).

## Acceptance criteria (what "done" means)
- [ ] **3 git.* catalog entries** in `MVP_ACTION_TYPES` (24→27): `git.stage_hunk`(risk-2) · `git.unstage_hunk`(risk-2) · `git.discard_hunk`(risk-3); all `executor_kind=git`, `requires_resource_refs=yes` (the file/hunk), `preview_class` = `diff` for discard (git/diff for stage/unstage — Step-2.5 #2), `idempotency_formula` per Step-2.5 #3.
- [ ] **`git.discard_hunk` is NON-standing-grantable** — the policy engine refuses a standing grant for it (always a per-action human approval, even under an active standing grant), via the **`standing_grant_eligible: bool` catalog field** (Step-2.5 #1). Generalize the existing risk-4 floor: a standing grant is refused for risk-4 OR `!standing_grant_eligible`. `discard_hunk.standing_grant_eligible = false`; `workflow.command.invoke` reconciles to the same flag (or stays risk-4-floored — Step-2.5 #1).
- [ ] **The non-standing-grant is pinned by an adversarial test** — a standing grant for `git.discard_hunk` does NOT auto-approve (it still requires per-action approval), mirroring the `workflow.command.invoke` critical-exclusion test (LESSON 19).
- [ ] **`git.stage_hunk`/`unstage_hunk`** are approval-gated (risk-2, standing-grant-eligible — the normal git.* tier).
- [ ] **The `get_diff` RPC** on the §6.1 GatewayPort: `get_diff(worktree_id, file) -> DiffResult` where `DiffResult` carries structured `Hunk`s (header, old_range, new_range, lines[{kind: context|added|removed, content}]). Reads git2 LIVE on demand (the §7.2 worktree-status precedent; read-only WAL, no mutation). The `Hunk`/`DiffResult` types frozen in `shared/` (reject-unknown).
- [ ] **CONTRACT bump 0.27.0→0.28.0** (additive — the 3 catalog types + the `standing_grant_eligible` field + the `get_diff` RPC + the `Hunk`/`DiffResult` types). Schema snapshots (§2.5-seam) for the new `shared/` types; the 3-way verify GREEN at 0.28.0.
- [ ] **The git executor bodies = stubs** (the CatalogExecutor seam fallback — `executor_kind=git` is unregistered until the real git executor lands Phase 5; the catalog dispatch returns the structured stub). No real git CLI hunk op in this slice.
- [ ] `/preflight` clean. **security-reviewer** (the discard-path destructive action + the non-standing-grant floor — a §6.2 safety mechanism).

## Wiring / entry point (Step 7.5)
- The 3 catalog types are reachable via `submit_action(git.stage_hunk|unstage_hunk|discard_hunk)` over the §6.1 GatewayPort (the ui submits them in 6.3e); the policy engine resolves risk + the non-standing-grant floor; the executor dispatches to the git stub (real = Phase 5).
- The `get_diff` RPC is a new §6.1 read method (`get_diff` alongside `get_projection`), reachable over the UDS; the ui calls it to render the diff. **The CONSUMER (the ui's 6.3e) is the ui track** — this slice freezes the contract; the ui builds against it on resume. (No daemon production caller for the actions until the ui submits them; the `get_diff` RPC is daemon-served + ui-consumed.)

## Files expected to touch
**Modified:**
- `shared/src/catalog.rs` — the 3 git.* entries + the `standing_grant_eligible` field on `ActionTypeCatalogEntry`.
- `shared/src/ipc.rs` (or the GatewayPort module) — the `get_diff` RPC method + the `Hunk`/`DiffResult` types.
- `shared/src/lib.rs` — CONTRACT 0.28.0.
- `shared/contracts/schema/*` — regen.
- `shared/tests/contract.rs` — schema snapshots (the new types + the catalog count 24→27).
- `daemon/src/gateway/policy.rs` — the non-standing-grant floor (`standing_grant_eligible` check, generalizing the risk-4 floor).
- `daemon/src/ipc/` — the `get_diff` handler (git2 live-read; read-only WAL).
- `daemon/tests/` — the catalog/policy/RPC tests (incl. the adversarial non-standing-grant test).

If the git2 diff-read needs a `git/` read module (like the worktree status read), flag at Step 2.5.

## RED test outline (Step 2)
1. **`test_git_hunk_catalog_risks`** — `git.stage_hunk`/`unstage_hunk`=risk-2, `git.discard_hunk`=risk-3; all `executor_kind=git`, `requires_resource_refs=yes`; `MVP_ACTION_TYPES` count 27. Why: §6.3.
2. **`test_discard_hunk_non_standing_grantable`** (adversarial) — with an ACTIVE standing grant covering git.discard_hunk, a submit STILL requires per-action approval (not auto-approved); a standing grant for git.stage_hunk DOES apply. Why: §6.2 — the USER-ruled non-standing-grant floor (the load-bearing safety pin; mirrors LESSON 19's critical-exclusion).
3. **`test_standing_grant_eligible_field`** — the catalog field is `false` for discard_hunk (+ workflow.command.invoke per Step-2.5 #1) and `true` for the normal git.* tier; the policy floor reads it. Why: §6.2/§6.3.
4. **`test_discard_hunk_preview_class_diff`** — discard's `preview_class=diff` (the preview renders the hunk content). Why: §6.3 — the destructive-action preview requirement.
5. **`test_get_diff_returns_structured_hunks`** — `get_diff(worktree, file)` over a fixture git2 repo returns structured `Hunk`s (header + ranges + typed lines); a clean file → empty. Why: §6.1 — the diff-read contract.
6. **`test_get_diff_read_only`** — the `get_diff` path uses a read-only WAL connection (no mutation; forbidden #3). Why: §15/single-writer.
7. **`test_hunk_types_snapshot` + `test_contract_0_28_0`** — the `Hunk`/`DiffResult` field-name snapshots (§2.5-seam) + CONTRACT 0.28.0; 3-way verify GREEN. Why: §5.0/§2.5-seam.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** YES — 3 new catalog action types + the `standing_grant_eligible` catalog field + the `get_diff` RPC + the `Hunk`/`DiffResult` types + CONTRACT 0.28.0. **Orchestrator writes hot:** the §6.3 catalog rows + the §6.1 GatewayPort row + Appendix A + daemon/CLAUDE.md + the bump record.
- **§2.5-seam touched?** YES — the `Hunk`/`DiffResult` types + the catalog cross the seam (ui-consumed) → schema-snapshot tests mandatory.
- **Reviewer policy:** `security-reviewer` = YES (the destructive `discard_hunk` + the non-standing-grant floor are §6.2 safety). `code-quality-reviewer` = every-slice.

## Things to flag at Step 2.5
1. **The non-standing-grant mechanism.** My default vote: **add a `standing_grant_eligible: bool` catalog field** (default `true`; `false` for `git.discard_hunk`); the policy floor refuses a standing grant for risk-4 OR `!standing_grant_eligible`. **Reconcile `workflow.command.invoke`** to set `standing_grant_eligible=false` too (so the floor is one mechanism, not two) — OR keep its risk-4 floor and OR the two conditions. Lean: **the unified field** (one mechanism). Flag if you'd rather a separate `NON_STANDING_GRANTABLE` set.
2. **`preview_class` for stage/unstage.** My default vote: **`git`** (stage/unstage are git index ops) vs `diff`. Discard = `diff` (show the content lost). Flag.
3. **`idempotency_formula` for the hunk actions.** My default vote: **`NaturalResourceRef`** (the hunk/file is the natural key — staging the same hunk twice is idempotent) — or `None` if hunk identity isn't stable across diffs. Flag (the hunk's stability across re-diffs is the question).
4. **The `Hunk` shape.** My default vote: `Hunk{header, old_start, old_lines, new_start, new_lines, lines: Vec<DiffLine{kind: context|added|removed, content}>}` (the standard unified-diff hunk). Confirm the ui's 6.3e render needs (the ui provisional `DiffHunk`, if any, reconciles).

## Dependencies + sequencing
- **Depends on:** R1-A (the registry seam — the git executor registers here when real). **Dispatch AFTER 4.0b-2 L2** (the impl is on the cat-1 live co-land; ui-① is the next freeze task).
- **Blocks:** the **ui-track 6.3e resume** (per-hunk git + diff review). On landing, the orch tells the lead → the lead pings the ui lead to rebase + resume.

## Estimated commit count
**1–2.** The catalog + policy + the RPC + the types are one additive §2.5-seam freeze (the bump is atomic — like R1-B). The non-standing-grant floor (the safety mechanism) could be its own commit if it reads cleaner (it's the §6.2 safety pin), but it's small. Lean **1 commit** (the freeze) or 2 (split the safety floor). No live executor (stubs).

## Lessons-logged candidates anticipated
- **Convention candidate** — "a risk-3 destructive action is made non-standing-grantable via an explicit catalog field (generalizing the risk-4 floor); the §6.2 standing-grant floor reads it." (Extends LESSON 19.)
- **Architecture-doc note candidate** — the §6.1 `get_diff` RPC (the request-on-demand git2-live read; the projection-vs-RPC rationale) + the §6.3 git-hunk catalog rows.

## How to invoke
1. **Read this brief + the USER rulings** (the discard-path risk-3 + non-standing-grant is the load-bearing pin).
2. **Run `/tdd per_hunk_git_actions_and_diff_rpc`** (AFTER 4.0b-2 L2 — the orch dispatches it then).
3. **Step 2.5** — the non-standing-grant mechanism (#1) + the adversarial test (test 2) are the load-bearing safety surface; don't soften. Dispatch `security-reviewer` at Step 8.
4. **Step 9** — the §6.3/§6.1 cross-doc rows for the orch's hot writes + the 3-way verify GREEN at 0.28.0.

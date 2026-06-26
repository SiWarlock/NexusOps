# /tdd brief — git_hunk_staging_activation (W1-git-ui)

## Feature
Activate the ui-6.3e per-hunk git-staging surface now that daemon 095 (`git.stage_hunk`/`git.unstage_hunk`
executor bodies) is LIVE: Stage/Unstage are now functional end-to-end (the UI was already wired). The one
UI change: **hold the destructive Discard control** — it's enabled today but `git.discard_hunk` is still a
daemon STUB (W1-git-discard pending), so a click currently leads to submit→approve→**Failed**
(approve-then-fail). Gate Discard behind a daemon-readiness hold (honest disabled state) until
W1-git-discard lands. Plus an HITL visual-gate that Stage/Unstage work against the live daemon.

## Use case + traceability
- **Task ID:** **W1-git-ui** (the unticked `- [ ] **W1-git-ui**` line under `### WAVE-1`; daemon-orchestrator
  homes it). The daemon halves: W1-git-stage ✅ `b3728c6` (stage/unstage LIVE) · W1-git-discard (pending).
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (the `git.*` per-hunk catalog —
  stage/unstage risk-2, discard risk-3 non-standing-grantable), `§6.1` (`get_diff`), `§6.2` (the approval
  pipeline a risk-2/3 git mutation enters), `§11` (cockpit), `§17` (the exact-hunk resource_ref — read↔mutate
  consistency; the daemon's two-guard race defense executes against the displayed hunk).
- **Phase scope:** this brief **widens phase scope because** it is a UI cockpit-activation slice, not a
  daemon-phase slice — the `§`-references are cross-doc context.
- **Related context (verified against the live surface):**
  - **The surface is ALREADY wired + enabled.** `src/views/code/DiffReview.tsx` `ReviewTab` sources the diff
    from `get_diff`, renders per-hunk Stage/Unstage/Discard buttons (`HunkGitActions`), and on a click →
    `buildHunkActionRequest(actionType, …)` (`intent/hunk-resource-ref.ts` — client-mint + the frozen
    `\x1f`-delimited `File` resource_ref targeting the EXACT displayed hunk) → `seam.submitAction` → the
    daemon adjudicates → `enrichActionApproval` → the `GatewayModal` approval card. All 3 buttons are
    `disabled={!canSubmit}` where `canSubmit = useCanSubmitIntent() && gateway.mutationsEnabled` (TRUE in
    prod since the L2-C go-live). So Stage/Unstage are LIVE end-to-end now (095) — **no transport/wiring
    change for them**.
  - **Discard is the gap:** `git.discard_hunk` (risk-3) has NO daemon executor yet (W1-git-discard pending)
    → submitting it reaches the approval card, the user approves, then the daemon's unregistered-executor
    fail-safe returns **Failed**. An approve-then-fail is a poor, dishonest UX for a DESTRUCTIVE control.
  - **The git-mutation user-gate is the APPROVAL MODAL (lead-confirmed), NOT a default-OFF go-live gate.**
    Git mutations are risk-2/3 → the per-action `GatewayModal` (discard's `preview_class=diff` shows the
    EXACT hunk content being discarded) is the human checkpoint. So Discard does NOT get a default-OFF
    *user-go-live* gate (unlike the session controls); the hold here is purely **daemon-readiness** ("the
    executor isn't built yet"), flipped on when W1-git-discard lands. (The lead's "default-OFF-gate any NEW
    always-on live-write" does not apply — Discard is EXISTING + approval-modal-gated.)
  - **Out of scope (gated):** the changed-files list + worktree/file selection — `diffReviewContext` is a
    hardcoded placeholder ("changed-files list pending worktree projection") gated on `proj_worktree` being
    populated. This slice does NOT wire the file-tree.

## Acceptance criteria (what "done" means)
- [ ] A `DISCARD_AVAILABLE` daemon-readiness flag (default **false**) gates the Discard button: Discard is
      **disabled** while `!DISCARD_AVAILABLE` EVEN WHEN `canSubmit` is true, with an honest disabled
      tooltip/title (e.g. "Discard available when the daemon `git.discard_hunk` executor lands"); a click on
      a disabled Discard never forms/submits an intent.
- [ ] **Stage + Unstage stay enabled** when `canSubmit` (unaffected by `DISCARD_AVAILABLE`); their
      submit→approval flow is unchanged (no regression — the existing DiffReview tests stay green).
- [ ] The flag is a single, clearly-commented constant (flip to `true` in the W1-git-discard-UI follow-on);
      a test pins Discard-disabled-while-Stage/Unstage-enabled at `DISCARD_AVAILABLE=false`, and (if
      cheaply parameterizable) Discard-enabled at `true`.
- [ ] All unit tests pass; `/preflight` clean. **HITL visual-gate** (manual, flagged — not a unit test):
      against a live daemon + a real worktree-with-changes, Stage/Unstage a hunk → the GatewayModal shows
      the daemon's policy/preview → approve → the daemon stages/unstages (jsdom can't exercise the live PTY/
      git path).

## Wiring / entry point (Step 7.5)
Production: the Code/Review tab → `DiffReview`'s `ReviewTab` (reachable in the cockpit; the per-hunk action
bar is already rendered). This slice changes ONLY the Discard button's enablement (the `DISCARD_AVAILABLE`
gate); Stage/Unstage reach the live daemon `git.stage_hunk`/`unstage_hunk` executors unchanged. No new
transport, no new component.

## Files expected to touch
**Modified:**
- `ui/src/views/code/DiffReview.tsx` — the `DISCARD_AVAILABLE` constant + thread it into `HunkGitActions`
  (Discard `disabled = !canSubmit || !DISCARD_AVAILABLE`) + the honest disabled title/tooltip.
- `ui/src/views/code/DiffReview.test.tsx` — the discard-held test (+ Stage/Unstage-still-enabled).

If implementation needs files beyond this list (e.g. a tiny `git-hunk` constants module), **flag at Step 2.5**.

## RED test outline (Step 2)
1. **`discard_held_until_daemon_executor`** — Asserts: at `DISCARD_AVAILABLE=false`, the Discard button is
   disabled even when `canSubmit` (mutationsEnabled + canSubmitIntent both true); clicking it does NOT call
   `seam.submitAction` / form a `git.discard_hunk` request. Why: §6.3 (discard executor pending) + honest-degrade
   (no approve-then-fail).
2. **`stage_unstage_enabled_independent_of_discard_hold`** — Asserts: Stage + Unstage are ENABLED when
   `canSubmit` regardless of `DISCARD_AVAILABLE`; a Stage click submits `git.stage_hunk` for the exact hunk.
   Why: 095 made them live; the hold is discard-scoped (no regression).
3. **`discard_disabled_tooltip_is_honest`** — Asserts: the held Discard's title/aria explains it's pending the
   daemon executor (never a misleading "go-live" framing). Why: honest labeling (lesson 41 spirit).
4. *(if cheaply parameterizable)* **`discard_enabled_when_flag_true`** — Asserts: at `DISCARD_AVAILABLE=true`
   + `canSubmit`, Discard is enabled + a click submits `git.discard_hunk`. Why: pins the flip's effect.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — the `git.*` action types + the resource_ref encoding are frozen daemon
  contract (4.0b-ui1, CONTRACT 0.28). **CONTRACT-neutral; no `shared/` change; no regen.**
- **Orchestrator doc rows to write hot:** none. A likely small Convention candidate (the daemon-readiness
  hold vs the user-go-live gate distinction).

## Things to flag at Step 2.5
1. **Discard hold mechanism.** My default vote: **a module-level `DISCARD_AVAILABLE = false` constant** (a
   daemon-readiness flag, flipped true in the W1-git-discard-UI follow-on) + an honest disabled tooltip —
   keeps the button visible (signals it's coming) + the layout stable. Alternatives: remove the Discard
   button entirely until then; or a capabilities-driven flag (the daemon doesn't report per-action executor
   readiness, so a code constant is the pragmatic hold). Confirm.
2. **NOT a default-OFF user-go-live gate.** Confirm the hold is daemon-readiness only (git mutations'
   user-gate is the approval modal, lead-confirmed) — so when W1-git-discard lands, the flip alone activates
   Discard (no separate USER cat-1 sign-off, unlike the session controls).
3. **The visual-gate is HITL.** My default vote: flag it as a manual gate (live daemon + a real
   worktree-with-changes); jsdom can't exercise the live git path. Confirm.

## Dependencies + sequencing
- **Depends on:** daemon W1-git-stage ✅ `b3728c6` (stage/unstage LIVE) + the live ApprovalQueue + the L2
  seam. Nothing blocking. **Note:** stage/unstage are live but downstream of `proj_worktree` being
  populated — `get_diff` returns `NotFound` until a worktree is registered (the existing honest "diff
  unavailable" state handles it; the HITL visual-gate needs a registered worktree-with-changes).
- **Blocks:** nothing. **Follow-on — the W1-git-discard-UI activation is MORE than a flag flip:** when the
  (cycling) fresh daemon pair lands W1-git-discard, the Discard activation = flip `DISCARD_AVAILABLE=true`
  **AND** add a `displayed_hunk_sha256` input to `buildHunkActionRequest` for `git.discard_hunk` — the lead
  ruled **(A) content-hash**: the UI sends a SHA-256 of the displayed hunk content, the daemon
  verifies-before-destroy (mismatch → Failed "hunk changed, re-examine"). The daemon-orchestrator confirms
  the exact field contract when the fresh pair authors W1-git-discard. (Separately, gated on `proj_worktree`)
  the changed-files/worktree selection.

## Estimated commit count
**1** — a focused honest-degrade change (the Discard daemon-readiness hold + its test). Stage/Unstage need
no code change (already wired + live via 095). `security-reviewer` is light here (no new transport; the UI
remains a pure submitter; the daemon Gateway + the §17 race defense are the chokepoint) — a quick pass on
the no-submit-while-held assertion suffices.

## Lessons-logged candidates anticipated
- **Convention candidate** — a **daemon-readiness hold** (a wired live-write control held by a code flag
  because its daemon executor isn't built yet) is DISTINCT from a **user-go-live gate** (held for a user
  cat-1 sign-off): the readiness hold flips on automatically when the daemon lands; the go-live gate needs
  the user. Both default-held, different release triggers.

## How to invoke
1. Read this brief end-to-end.
2. Run `/tdd git_hunk_staging_activation`.
3. Step 1 → confirm the Discard-hold mechanism + that Stage/Unstage need no change.
4. Step 2.5 → confirm the 3 design Qs.
5. Step 9 → flag the W1-git-discard-UI flip follow-on + the proj_worktree-gated file-tree.

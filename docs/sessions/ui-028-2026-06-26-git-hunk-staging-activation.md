# ui-028 — git-hunk staging activation (W1-git-ui)

**Date:** 2026-06-26
**Role:** ui-orchestrator (converged team `nexusops-daemon`; single working tree on `main`)
**Round:** one standalone slice after the WAVE-1 session-control lane (ui-027) — the git-hunk staging activation.

## What was built

**ui-088 (W1-git-ui)** — committed `6a399fb` (1 commit, CONTRACT-neutral, suite 618/0 + 1 skipped signpost).

Daemon 095 (`b3728c6`) made `git.stage_hunk`/`git.unstage_hunk` functional, so the wired-but-stubbed
ui-6.3e `DiffReview` staging surface is now LIVE for Stage/Unstage end-to-end (the UI was already wired —
no UI transport change). The one UI change: **hold the destructive Discard control** behind a module-level
`DISCARD_AVAILABLE = false` **daemon-readiness flag** — `git.discard_hunk` is still a daemon stub
(W1-git-discard pending), so Discard was an approve-then-fail. Held = disabled even when `canSubmit`, an
honest "...executor lands" tooltip, no-submit-on-click. `HunkGitActions` exported + a `discardAvailable`
prop (default = the const) for the flip-true unit pin.

## Decisions made

- **Daemon-readiness hold ≠ user-go-live gate** (→ LESSON [[42]]): a wired live-write control held by a code
  flag because its daemon executor isn't built yet AUTO-flips when the daemon lands (NO held-flip guard);
  contrast the session controls' user-go-live gates (held for a USER cat-1 sign-off, never auto-flip,
  held-flip-guarded).
- **Git-mutation user-gate = the per-action approval modal** (lead-confirmed): risk-2/3 git mutations are
  gated by the GatewayModal (discard's `preview_class=diff` shows the exact hunk), NOT a default-OFF
  user-go-live gate — so the readiness flip alone activates Discard (no separate sign-off).
- **Test churn (the held control):** 5 action-agnostic flow tests repointed Discard→Stage (now live), 1
  discard-submit integration test `it.skip`'d as a re-enable-at-flip signpost, its action-type coverage
  moved to the `HunkGitActions` unit (both flag states).

## Decisions explicitly NOT made

- **No flag flipped.** Discard stays held (`DISCARD_AVAILABLE=false`) until daemon W1-git-discard lands.
- **No `shared/` change** (CONTRACT-neutral; the `git.*` types + the `\x1f` hunk resource_ref are frozen at
  CONTRACT 0.28 / 4.0b-ui1).
- **Did not wire the changed-files/worktree selection** (the hardcoded `diffReviewContext` placeholder) —
  gated on `proj_worktree` being populated.

## Open follow-ups

- **🔴 The W1-git-discard-UI flip (SECURITY-RELEVANT — NOT a flag-only flip):** when the fresh daemon pair
  lands W1-git-discard (brief 096, in flight), the Discard activation = flip `DISCARD_AVAILABLE=true` **AND**
  add a `displayed_hunk_sha256` content-hash input to `buildHunkActionRequest` for `git.discard_hunk` (the
  lead-ruled (A) §17 verify-before-destroy; daemon re-derives + compares, mismatch → Failed). **Wait for the
  daemon's LOCKED canonicalization spec** (which bytes get hashed — frozen at the daemon Step-2.5; the
  daemon-orchestrator relays it) — do NOT build the hash send against a guess. + re-enable the `it.skip`'d
  discard-submit integration test then. (Recorded in the ui-088 brief + the W1-git-ui plan checkbox.)
- **proj_worktree-gated:** the changed-files list + worktree/file selection (the `diffReviewContext`
  placeholder).
- **HITL visual-gate (pending the user/lead):** live daemon + a registered worktree-with-changes → Stage/
  Unstage a hunk → GatewayModal → approve → daemon stages (jsdom can't; `get_diff` NotFounds until a
  worktree is registered).
- **Shared-doc (via daemon-orchestrator):** tick `W1-git-ui` ✅ (ui-088 landed `6a399fb`).
- **Next dispatchable ui:** the W2 projection-honesty reconciles (AuditTrail `event_type` / UsageLedger
  `creditPool`) when the fresh daemon pair lands the W2-audit half; or the Discard flip when W1-git-discard
  lands (+ its canonicalization spec).

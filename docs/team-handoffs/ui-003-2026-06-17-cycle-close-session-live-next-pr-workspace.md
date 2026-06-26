# Team Handoff ui-003 — full cycle-close (Session live landed; PR Workspace + whole-cockpit-live next)

**Date:** 2026-06-17
**Track:** ui
**Worktree:** `NexusOps-ui` (branch `track/ui`)
**Predecessor handoff:** `docs/team-handoffs/ui-002-2026-06-13-6tail-cross-track-pause.md`
**Successor handoff:** `docs/team-handoffs/ui-004-2026-06-19-phase6-7-complete-integrated-paused-daemon-gated.md`
**Round-seal commit at handoff:** `1b2e3bd` (pushed == origin/track/ui)

## Why this handoff exists
User-directed full cycle-close + shutdown (ui-062 was the explicit last slice). Clean pause, not arc-complete — the ui track has in-lane work remaining + daemon asks to route.

## Team composition at close
- Lead: this session (track `ui`, team `nexusops-ui`).
- Orchestrator: `ui-orchestrator` — `/orchestrate-end`-closed at the round terminal `1b2e3bd`; shut down.
- Implementer (`ui/`): `ui-implementer` — `/session-end`-closed (session doc `ui-018` `c21e69c`); shut down.
- Both closed + spun down at `1b2e3bd`. Registry entries cleaned.

## What landed this session (the arc)
A long, productive arc from the L2 go-live through full-current-with-main:
1. **L2 cockpit go-live COMPLETE** — the cockpit drives real, daemon-classified mutations end-to-end (053b/054/L2-A/B/C; `e66386c`). 🔒 L2-O3 USER-RULED "sign off before go-live" (granted). "Always allow"/`policy_grant` STILL DISABLED (own future cat-1).
2. **main→ui merges to CONTRACT 0.38** — ui-058 (→0.33) + ui-060 (→0.38, daemon D1–D5) + **3 non-contract scaffolding folds** (2ce93f6 / 91c7d59 / 8e6ae03 → main@289a918). track/ui fully current with main.
3. **ApprovalQueue LIVE** — ui-059 (refetch-on-nudge + single-authority connection aggregation; `4a8b003`).
4. **Session LIVE + survival-shadow** — ui-062 (Session refetch-on-nudge — the no-op `applySessionDelta` deleted; SessionRow frozen 5→10 `.strict()` with real per-session resume-mode; `b04feb1`; 376/376 green).

CONTRACT 0.38. main UNTOUCHED by ui (origin/main `2ce93f6`, never pushed by ui). Every slice `security-reviewer`-clean. The **verify-before-build discipline** caught 4 Mock-vs-real / daemon-gap findings before any wasted build (the Phase-7 PR-row, the survival projection, the row:None-nudge no-op, the D7 PR-diff gap).

## In-flight at close
**None — clean close.** Tree clean, all sealed + pushed.

## Carry-forward to next team session (the resume working set)
Per `IMPLEMENTATION_PLAN.md` "Currently in progress" + Carry-forward (orch-staged at the ui-062 seal):
1. **ui-063 whole-cockpit-live** — the other-projections refetch-on-nudge spread (ProjectActivity / PullRequest / UsageLedger; mechanical ui-059/ui-062 replication; daemon D4 deltas emit). Verified-ready, mechanical. *(The 6.8 spread-to-rest; Session ✅ done @ui-062.)*
2. **Phase-7-UI L2 — read-only PR Workspace shell** — build to the **user's prototype**: the **"Pull requests" Kanban tab** (3 columns OPEN/READY-TO-MERGE/MERGED of PR cards, backed by `PullRequestRow`; **D6 diff-stats `+/-·files·commits` = PLACEHOLDER** until the daemon adds them) + the **"Review" tab** (changed-files + per-PR code-diff + per-hunk actions + Approve PR + Ask Brain; the **reviews-list backed by `ReviewRow`** builds now; the **code-diff panel = PLACEHOLDER — D7 CONFIRMED daemon-gap**). **All mutations (Merge / Approve PR / per-hunk accept/reject/request-fix) render DISABLED** = a future cat-1 arc. **Ask Brain / Ask why render DISABLED** = the deferred Brain sibling.
- Sequencing is the next orch's call (verify-before-build each — the discipline that's paid off all session).

## Open decisions / asks for the human
**USER-routed daemon asks** (durable in `docs/planning/daemon-unblock-work-order.md` — route to the daemon track when ready; the UI builds all backed parts + placeholders meanwhile, never blocked):
- **D6** — PR card diff-stats: the edges GitHub-sync must FETCH `additions/deletions/changed_files/commits` (NOT in `PullRequestSynced`) → event → projection → contract bump.
- **D7** — `get_pr_diff(repo_id, pr_number) → DiffResult` RPC: **CONFIRMED gap** (`get_diff` is worktree-scoped; a PR is a remote entity). Reuse the frozen DiffResult/Hunk/DiffLine shapes.
- **D8** — recovery-status signal for the **actionable RecoveryState banner** ("recovery failed" alert): no projection marker today; needs a daemon recovery-status signal (+ the broker 4.1b-2 for richer modes). Optional — the per-session resume indicator already works.

**Future arcs (NOT this read-only phase):** the **PR-review mutations** (Merge/Approve-PR/accept-reject) = a future **cat-1 arc** (own checkpoint, like the L2 go-live). **Brain (8.2-UI)** = deferred sibling `brain/` product (not started).

**Standing rules:** merges USER-GATED on "main idle" (re-verify the sha right before any boundary merge — the stale-target lesson; main moved 5× this session); `track/ui` pushes at round close-outs; **NEVER push main**. cat-1 Q1–Q7 + the L2 cat-1 checkpoint are durable in `docs/planning/` (consume, never re-derive).

## Spawn prompts ready for the next team session

**Orchestrator (`ui-orchestrator`):**
```
You are ui-orchestrator on the NexusOps agent team. Track: ui. Team: nexusops-ui.
Worktree: /Users/dreddy/Documents/Dev/AI-tools/ai-engineering-control-plane/NexusOps-ui (branch track/ui) — operate here; all commits land on track/ui, never the root checkout. Daemon owns shared/ + CONTRACT_VERSION; ui consumes via regen. Ignore non-`ui-` peer DMs.
Activated because: resuming the ui track after a full cycle-close (handoff docs/team-handoffs/ui-003-…). Cursor = track/ui @ 1b2e3bd, CONTRACT 0.38, GREEN, pushed, clean; fully current with main@289a918. Drive the remaining in-lane arcs (verify-before-build each): (1) ui-063 whole-cockpit-live (ProjectActivity/PullRequest/UsageLedger refetch-on-nudge spread — mechanical ui-059/ui-062 replication) (2) Phase-7-UI L2 read-only PR Workspace shell (to the user's prototype: Kanban "Pull requests" tab + "Review" tab reviews-list; D6 diff-stats + D7 code-diff PLACEHOLDERS; mutations + Brain DISABLED). Daemon asks D6/D7/D8 are USER-routed (work-order doc) — build backed parts + placeholders, never block. Escalate to the lead only on a Finding/cat-1/load-bearing design-option.
FIRST ACTION — register: ~/.claude/scripts/team-register.sh "ui-orchestrator" orchestrator "nexusops-ui" "" "ui" "track/ui"
Then run /orchestrate-start. Confirm: start command, registry written, your read of the next slice.
```

**Implementer (`ui-implementer`, area `ui/`):**
```
You are ui-implementer on the NexusOps agent team. Track: ui. Team: nexusops-ui.
Working directory: /Users/dreddy/Documents/Dev/AI-tools/ai-engineering-control-plane/NexusOps-ui/ui/ — the ui area of track/ui. Commits land on track/ui only (explicit git add <path>, never -A). Talk only to ui-orchestrator; ignore non-`ui-` peer DMs.
Activated because: resuming the ui track post cycle-close (handoff ui-003). The orchestrator dispatches the next slice (likely ui-063 whole-cockpit-live, then the Phase-7 PR Workspace shell). Wait for the orch's dispatch + task assignment.
FIRST ACTION — register: ~/.claude/scripts/team-register.sh "ui-implementer" implementer "nexusops-ui" "ui" "ui" "track/ui"
Then run /session-start. Confirm: start command, registry written.
```

## How to resume
Next team session: lead runs `/team-start ui`, reads this handoff + `IMPLEMENTATION_PLAN.md` "Currently in progress" + `docs/planning/daemon-unblock-work-order.md` on demand, spawns the two teammates with the prompts above, verifies read-backs. Re-confirm `main`'s tip before any boundary merge. This doc IS the orient — no re-derivation needed.

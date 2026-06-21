# Team Handoff 008 — §4.7 PR-mutation cat-1 arc COMPLETE (D9 merge_pr + D10 submit_review) · team idles for the main→ui merge

**Date:** 2026-06-20
**Track:** daemon (single-track; critical path P2 → P3 → P4 → §4.7 ui-unblock wave)
**Worktree:** root checkout (single-track, `main`)
**Predecessor handoff:** `docs/team-handoffs/007-2026-06-20-d6d7-sealed-d9d10-cat1-next-tmux-restart.md`
**Successor handoff:** `docs/team-handoffs/009-2026-06-21-083-auth-bootstrap-done-golive-unblocked-pause-for-live-validation.md`
**Round-seal commit at handoff:** `6b847d6`

## Why this handoff exists
**User-directed pause** — the §4.7 PR-mutation cat-1 arc (D9 + D10) is complete and sealed, and the user is idling the daemon team to merge a clean `main` into `track/ui` and resume the ui track. (This session was itself a recovery: it restood-up the team after handoff 007's tmux session was accidentally closed out mid-D9-design — no work was lost; brief 078 was the only uncommitted survivor and is now committed.)

## Team composition at close
- **Lead:** this session (track daemon, label `nexusops-daemon`).
- **Orchestrator:** `daemon-orchestrator` — `/orchestrate-end`-closed at the round seal **`6b847d6`**.
- **Implementer:** `daemon-implementer` — `/session-end`-closed; session doc **033** (`1d2cb87`).
- Both teammates closed cleanly; tree clean (only `.codegraph/` + `scaffold/` untracked). Shut down via `shutdown_request` at this `/team-end`.

## Active arc + where it landed
**The §4.7 ui-unblock work order is COMPLETE.** Read surface (D6+D7) sealed last round; this round sealed the **PR-mutation cat-1 arc**:
- **D9** `github.merge_pr` — `e544544` (contract freeze) + `c1cd0be` (mutation vertical); **CONTRACT 0.40→0.41**; SHA-pinned merge → `PullRequestMerged` folds `proj_pull_request` to terminal `Merged`. Brief 078.
- **D10** `github.submit_review` — `de201c0` (contract freeze) + `cbae5aa` (mutation vertical); **CONTRACT 0.41→0.42**; SHA-pinned review verdict → NEW `ReviewSubmitted` folds `proj_review`. Brief 079.
- Both **F1/F2 user-steered identically**: risk-3 + `entry_no_standing_grant` (per-action approval always) + UI/IPC-requester-only (shared `GITHUB_MUTATION_TYPES` deny-before-risk gate). **security-reviewer CLEAR every layer, 0 findings, both slices.** Suite **979/0**; 3-way verify PASS @ 0.42.0; MVP catalog 29→31. LESSONS **§60 + §61**. Session doc **033**.

## In-flight at close
**None — clean close.** D10 sealed through Step-10; both teammates closed; tree clean.

## ui-track unblock status (cross-track)
**COMPLETE (contract surface).** The ui PR Workspace now has its full daemon-side surface on `main`: read (D6 diff-stats + D7 `get_pr_diff`) + both mutations (D9 merge + D10 review). The read-only PR Workspace shell (ui Opt-A, mutations disabled) was already unblocked by D6/D7; the live Merge/Review-submit buttons additionally need the two prerequisites below.
- **🔴 LIVE-MUTATION GO-LIVE PREREQUISITES** (grouped in §4.7): (1) the per-repo keychain **AUTH-BOOTSTRAP** + its mandatory security-re-review gate (confused-deputy axis; shared D7/D9/D10) — the production GitHub client is unauthenticated, so a live write returns `401 → fail-closed` until this lands; (2) **`head_sha` exposure** on `PullRequestRow`/`proj_pull_request` for the SHA-pin. Both mechanisms fake-tested now.
- **🟡 ui-RESUME PREREQUISITE:** whole-workspace `cargo fmt --check` is red ONLY on pre-existing **ui-track** files (`ui/gateway-uds`, `ui/src-tauri`) — outside daemon territory, untouched by D9/D10. The ui track must run `cargo fmt` on those crates once under the `rust-toolchain.toml` 1.93.0 pin before its next `/preflight`. Annotated in session doc 033.

## Carry-forward to next team session
- **`IMPLEMENTATION_PLAN.md` "Currently in progress":** §4.7 PR-mutation cat-1 arc COMPLETE + sealed `6b847d6`; CONTRACT 0.42.0; suite 979/0; team idles for the main→ui merge.
- **Next daemon-track slice target: USER-DIRECTED** (the user is on ui work next). Candidates when the daemon team resumes:
  - **5.3a** (§15 execution-profiles persistence fork) — **PARKED, needs the user's A/B steer** (`docs/planning/5.3a-execution-profiles-persistence-fork.md`); the lead brings the A/B fork to the user, then the orch authors the brief.
  - **The live-mutation go-live prerequisites** (auth-bootstrap [🔴 mandatory security re-review] + `head_sha` exposure) — if the ui track needs live buttons next.
  - **D8** (recovery-status banner signal) — DEFERRED (lead-ruled out of the critical-unblock set).
- **Open Carry-forward items:** see `IMPLEMENTATION_PLAN.md` "Carry-forward" (triaged @ 2026-06-20); the §4.7 follow-ons (per-hunk inline `comments[]` for D10; `merged:false` 200-edge re-classification for D9) are inlined to §4.7.

## Open decisions / blockers for the human
- **Push decision** — `main` is **+8 ahead** of origin (`origin/main 4a8799f`): briefs 078/079 + D9 (`e544544`/`c1cd0be`) + D10 (`de201c0`/`cbae5aa`) + session doc 033 (`1d2cb87`) + round seal (`6b847d6`). **HELD, user-gated** — the user pushes alongside the main→ui merge. A local main→ui merge does not strictly require a push.
- **5.3a §15 A/B persistence fork** — pending the user's steer (held behind the now-complete cat-1 arc).
- **Live-mutation go-live prerequisites** — the auth-bootstrap's security re-review is a deliberate gate the user will want to schedule before any live authenticated GitHub write.

## Spawn prompts ready for the next team session

**Orchestrator:**
```
You are daemon-orchestrator on the NexusOps agent team.
Track: daemon. Team: nexusops-daemon. Single-track — repo root, commits on `main` (no worktree). You own IMPLEMENTATION_PLAN.md + ARCHITECTURE.md. Ignore non-`daemon-`-prefix peer DMs (channel-bleed).
Activated because: RESUME from handoff 008. The §4.7 ui-unblock work order is COMPLETE — D9+D10 PR-mutation cat-1 arc sealed `6b847d6` (CONTRACT 0.42.0, suite 979/0); the team idled for the user's main→ui merge. Read docs/team-handoffs/008-… + IMPLEMENTATION_PLAN.md "Currently in progress".
NEXT = USER-DIRECTED (confirm with the lead before authoring): 5.3a (§15 A/B persistence fork — PARKED, needs the user's steer; docs/planning/5.3a-…) OR the live-mutation go-live prerequisites (auth-bootstrap [🔴 mandatory security re-review] + head_sha exposure) OR D8 (deferred). Do NOT author a brief until the lead relays the user's pick. For any cat-1 / safety-touching slice, surface the safety-design lead→user BEFORE authoring; security-reviewer `invariant` every layer.
DIAL: production-grade; make production-correct realization calls; surface only a genuinely-NEW safety fork or load-bearing Option to the lead.
FIRST ACTION: ~/.claude/scripts/team-register.sh "daemon-orchestrator" orchestrator "nexusops-daemon" "" "daemon" — then /orchestrate-start. NOT /session-start. Confirm the start command + the registry entry.
```

**Implementer (`daemon`):**
```
You are daemon-implementer on the NexusOps agent team.
Track: daemon. Team: nexusops-daemon. Working dir: `daemon/` in the repo root (single-track, main). Commits on `main`. Talk only to daemon-orchestrator; ignore other prefixes.
Activated because: RESUME from handoff 008. §4.7 PR-mutation cat-1 arc COMPLETE + sealed `6b847d6`; CONTRACT 0.42.0; suite 979/0. Push HELD/user-gated. Stand by for the orch's first dispatch once the user's next-slice direction is set.
FIRST ACTION: ~/.claude/scripts/team-register.sh "daemon-implementer" implementer "nexusops-daemon" "daemon" "daemon" — then /session-start. NOT /orchestrate-start. Confirm the start command + the registry entry.
```

## How to resume
In a fresh session: lead runs `/team-start daemon`, reads this handoff + `IMPLEMENTATION_PLAN.md` "Currently in progress" on demand, spawns the orch + impl with the prompts above, verifies read-backs. **The next daemon slice is user-directed — confirm the pick (5.3a vs the go-live prerequisites vs D8) before the orch authors.** No re-orient overhead — this doc + the tracker IS the orient.

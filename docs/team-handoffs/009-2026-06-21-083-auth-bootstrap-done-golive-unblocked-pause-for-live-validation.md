# Team Handoff 009 — 083 auth-bootstrap DONE · §4.7 live-mutation go-live UNBLOCKED · team paused for live validation

**Date:** 2026-06-21
**Track:** daemon (single-track; critical path P2 → P3 → P4 → §4.7 ui-unblock wave → auth-bootstrap)
**Worktree:** root checkout (single-track, `main`)
**Predecessor handoff:** `docs/team-handoffs/008-2026-06-20-d9d10-cat1-arc-complete-team-idle-for-ui-merge.md`
**Successor handoff:** _(filled in when the next /team-end runs)_
**Round-seal commit at handoff:** `4ee93d3`

## Why this handoff exists
**User-directed pause at a milestone** — the GitHub auth-bootstrap (083) is COMPLETE and the live-mutation go-live is UNBLOCKED. The user chose to pause the daemon team to **validate 083 live end-to-end** (resume the UI track, connect-via-gh, flip the toggle, do a real Merge) before building the 084 fast-follow. Verify-the-real-thing-works-before-building-more.

## Team composition at close
- **Lead:** this session (track daemon, label `nexusops-daemon`).
- **Orchestrator:** `daemon-orchestrator` — `/orchestrate-end`-closed at the round seal **`4ee93d3`**.
- **Implementer:** `daemon-implementer` — `/session-end`-closed; session doc **036** (`71bc5b9`).
- Both teammates closed cleanly; tree clean. **origin/main == local == `4ee93d3` (083 round PUSHED, ahead 0).** Shut down via `shutdown_request` at this `/team-end`.

## Active arc + where it landed
**The GitHub auth-bootstrap arc (083) is COMPLETE — the live authenticated Merge/Review capability ships end-to-end.** This session's path: carry-forward cleanup → P5.3a durable execution-profile registry (Option B) → head_sha (081) → the 4-site confused-deputy closure (082) → §4.7 seal+push → the **083 auth-bootstrap** (the live-mutation go-live).

**083 — 6 commits `335929b`→`b61cad4` (Option-B full vertical, user-ruled):**
- `335929b` keychain-write primitive (`SecretStore` over keyring) — the safety-pinned secret-write, its own commit
- `5572386` gh-reuse acquisition + the single `resolve_authed_token` gate (per-account confused-deputy, no-token fail-closed, toggle-gated, reads+writes both gated)
- `4ca090a` contract freeze — `integration.set_live_writes` + `connect_via_gh` wire (**CONTRACT 0.45.0**)
- `266162e` live-writes toggle vertical (`IntegrationLiveWritesSet` event + projector fold + MIGRATION_18 + policy deny-before-risk; risk-2, UI/IPC-only, non-standing-grantable; default **OFF**)
- `e0830ed` live keychain-backed github clients (per-owner auth via the §63-audited owner)
- `b61cad4` connect_via_gh trigger (getpeereid peer-authed; daemon reads the user's gh token → keychain; no token over IPC)

**Safety:** the in-arc security-reviewer `invariant` re-review came back **CLEAR — core 8/8 + live-path 7/7** (no Finding; the IPC-triggered peer-authed keychain write of the user's own gh token ruled acceptable on the merits per LESSON §49/§58/§62 precedent). Suite **1033/0**, clippy+fmt clean, no ui drift. LESSON **§64** (keychain-write primitive). The seal ticked **§4.7's two go-live prerequisites (AUTH-BOOTSTRAP + head_sha) → go-live UNBLOCKED**.

**Auth design (locked, user-ruled — full rationale in briefs 083/084):** GitHub App + device-flow "Connect GitHub" UX; **NON-EXPIRING L1 token** (the serverless desktop can't safely ship a `client_secret`, so silent-refresh is impossible → non-expiring is the only path that delivers "log in once" without a secret-at-rest; the refresh-scheduler was DROPPED). The **(Y) gh-reuse interim** (083) is the functional unblock for users who have `gh`; **084 device-flow** is the in-app login for users without `gh`.

## In-flight at close
**None — clean close.** 083 sealed+pushed through `/orchestrate-end`; both teammates closed; tree clean; origin synced.

## How the user validates 083 live (their activity during the pause)
The daemon side is ready + gated OFF. The validation chain:
1. Resume the UI track (`/team-start ui`) → wire + enable the live PR Merge/Review buttons against the new surface (currently the read-only shell, mutations disabled).
2. **Connect via gh** (the `connect_via_gh` trigger) → puts the gh token in the keychain.
3. Flip the per-connection **live-writes toggle ON** (`integration.set_live_writes`).
4. Do a real Merge/Review → confirm it reaches GitHub (every write still per-action approved).
Any gap found in that chain comes back to the daemon team as a fix.

## Carry-forward to next team session
- **`IMPLEMENTATION_PLAN.md` "Currently in progress":** 083 auth-bootstrap DONE + sealed/pushed `4ee93d3`; CONTRACT 0.45.0; suite 1033/0; §4.7 go-live UNBLOCKED; team paused for live validation.
- **Next daemon-track slice target: USER-DIRECTED.** Candidates when the daemon team resumes:
  - **084** (device-flow in-app "Connect GitHub" login, non-`gh` users) — **brief 084 authored**; design RULED (L1 non-expiring, refresh dropped). **HITL prereq: the one-time GitHub App registration** — the user registers a GitHub App (deselect user-token-expiration, enable device flow, minimal perms `pull_requests:write`+`contents:write`+`checks:read`) and provides the **public client_id** to ship; flagged at 084's Step-1; the FSM is fake-testable without it.
  - **5.3b** (execution-profile secret half — keychain writes for agent-launch creds + startup self-test + runtime status re-derivation + profile-change-new-approval) — reuses the 083 keychain primitive; the deferred **gh-stdout-zeroize LOW** folds in here.
  - Plus the parked backlog now homed per-phase (3.6/4.8/7.4/10.6) from this session's carry-forward cleanup.
- **Whatever the user found in live validation** — any 083 end-to-end gap takes priority as a fix.

## Open decisions / blockers for the human
- **084 HITL prereq** — the one-time GitHub App registration (public client_id) is the user's to do before device-flow runs end-to-end. Not on the user's own critical path (they have `gh`).
- **Push posture** — origin/main == local == `4ee93d3` (the 083 round is pushed). This `/team-end` handoff commit will leave local **+1 ahead** (docs-only); push is user-gated (rides the next round or a user push).
- **Live validation outcome** — pending the user's hands-on test of the unblock.

## Spawn prompts ready for the next team session

**Orchestrator:**
```
You are daemon-orchestrator on the NexusOps agent team.
Track: daemon. Team: nexusops-daemon. Single-track — repo root, commits on `main` (no worktree). You own IMPLEMENTATION_PLAN.md + ARCHITECTURE.md. Ignore non-`daemon-`-prefix peer DMs (channel-bleed).
Activated because: RESUME from handoff 009. The GitHub auth-bootstrap (083) is COMPLETE + sealed/pushed `4ee93d3` (CONTRACT 0.45.0, suite 1033/0); §4.7 live-mutation go-live UNBLOCKED. The team paused so the user could validate 083 live. Read docs/team-handoffs/009-… + IMPLEMENTATION_PLAN.md "Currently in progress".
NEXT = USER-DIRECTED (confirm with the lead before authoring): 084 (device-flow in-app login — brief 084 authored; HITL GitHub-App-registration prereq the user provides) OR 5.3b (execution-profile secrets — reuses the 083 keychain primitive; folds the deferred gh-stdout-zeroize LOW) OR a fix for anything the user's live validation surfaced. For any cat-1/secret-touching slice, surface the safety-design lead→user BEFORE authoring; security-reviewer `invariant` every layer; secret-write its own commit.
FIRST ACTION: ~/.claude/scripts/team-register.sh "daemon-orchestrator" orchestrator "nexusops-daemon" "" "daemon" — then /orchestrate-start. NOT /session-start. Confirm the start command + the registry entry.
```

**Implementer (`daemon`):**
```
You are daemon-implementer on the NexusOps agent team.
Track: daemon. Team: nexusops-daemon. Working dir: `daemon/` in the repo root (single-track, main). Commits on `main`. Talk only to daemon-orchestrator; ignore other prefixes (channel-bleed).
Activated because: RESUME from handoff 009. 083 auth-bootstrap DONE + sealed/pushed `4ee93d3`; CONTRACT 0.45.0; suite 1033/0; §4.7 go-live UNBLOCKED. Stand by for the orch's first dispatch once the user's next-slice direction is set (084 device-flow / 5.3b / a live-validation fix).
FIRST ACTION: ~/.claude/scripts/team-register.sh "daemon-implementer" implementer "nexusops-daemon" "daemon" "daemon" — then /session-start. NOT /orchestrate-start. Confirm the start command + the registry entry.
```

## How to resume
In a fresh session: lead runs `/team-start daemon`, reads this handoff + `IMPLEMENTATION_PLAN.md` "Currently in progress" on demand, spawns the orch + impl with the prompts above, verifies read-backs. **The next daemon slice is user-directed — confirm the pick (084 vs 5.3b vs a live-validation fix) before the orch authors.** This doc + the tracker IS the orient.

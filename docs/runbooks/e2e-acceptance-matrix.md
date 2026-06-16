# NexusOps — E2E Final-Smoke Acceptance Matrix (LIVING)

> **Status:** living doc — **accretes per phase/slice.** Each slice that ships a route/feature/safety
> surface appends its acceptance line(s) here. The **final release gate** = *run the accumulated matrix*
> end-to-end (manual pass), derived from exactly what was built — **not** a single scripted demo path.
>
> **Replaces** the prior "built backward from the PRD §25 demo" release gate (user-decided 2026-06-14).
> The PRD §25 demo path remains ONE row below (a proof-of-thesis flow), but acceptance is the whole matrix.
>
> **How to use:** at `/phase-exit` and before the final gate, walk every row. ✅ = verified this pass · ⬜ =
> not yet · 🔬 = MANUAL-only (un-unit-testable live property — must be eyeballed, never green-by-test).
> Companion: `docs/runbooks/smoke-harness-live-drive-loop.md` (the live-drive rig the manual pass uses).

---

## How rows are added

Each `/tdd` slice appends, at Step 9 / the round seal, the routes + features it shipped — **especially any
review-verified-but-not-unit-testable SAFETY surface** (those get a 🔬 MANUAL row; an inline TODO rots).
Keep rows grouped by area. Cite the slice that added each row.

---

## A. Daemon — IPC / GatewayPort (§6.1/§6.4)

| # | Route / feature | Check | State | Origin |
|---|---|---|---|---|
| A1 | `get_projection` (all MVP projections) | each returns typed rows; reject-unknown | ⬜ | P1.5/1.6d |
| A2 | `subscribe` live deltas | delta on commit; close-on-lag → reconnect+re-`get_projection` | ⬜ | P1.6d |
| A3 | `get_capabilities` / handshake / version-skew | peer-auth (`getpeereid`); skew refuses | ⬜ | P1.5 |
| A4 | `submit_action`/`preview_action`/`approve`/`deny` | the §6.2 pipeline; audit-before-ack | ⬜ | P2.1b |
| A5 | `submit_action_plan` / per-step approval | whole-plan atomic; critical never approve-all'd | ⬜ | P2.1c |
| A6 | `get_diff(worktree_id,file)` | hunk-structured git2-live read; `NotFound` | ⬜ | P4.0b-ui1 |
| A7 | intercept-wait permit-class split | concurrent 5-min waits never starve the UI approve | ⬜ | P4.0b-2-F2 |

## B. Daemon — Action Gateway + safety invariants (§15/§17)

| # | Surface | Check | State | Origin |
|---|---|---|---|---|
| B1 | INV-SEC-1 live interception | every PreToolUse tool → Gateway adjudication → allow/deny; hook-miss → DENY | 🔬 | P4.0b-2 |
| B2 | risk-class gating (0–4), approve+deny | each class behaves; risk-0 auto-allow narrow; `git.discard_hunk` non-standing-grantable | ⬜ | P2.2/4.0b-ui1 |
| B3 | redaction-before-persist | no secret in any event/row; keychain-refs only | ⬜ | P1.7/2.0-SEC |
| B4 | fencing-conflict | stale-token write → hard-conflict card, never auto-resolved | ⬜ | P2.4 |
| B5 | audit-backbone circuit-breaker | N-consecutive/unrecoverable audit fault → latched quiesce-and-refuse; reads stay live | 🔬 | P4.0b-2c |
| B6 | §15 #8 ExecutionProfile binding | profile recorded-at-start; change = fresh approval; no account-hop | ⬜ | P4.0b-1 |

## C. Daemon — sessions / survival / recovery (§5.1/§8/§8.1/§17)

| # | Surface | Check | State | Origin |
|---|---|---|---|---|
| C1 | session.create/kill live | real `claude` launched, hook-gated; clean kill | 🔬 | P4.0b-2 |
| C2 | daemon-restart recovery | `recover_sessions_on_restart` → `decide_resume` → `SessionRecovered`; profile preserved | ⬜ | P4.1b-1 |
| C3 | **tmux broker B2-strict survival** | kill daemon mid-run → the agent OUTLIVES it (tmux) → restart reattaches the live turn; no orphaned PTY | 🔬 | P4.1b-2 |
| C4 | **#10 / §15 #8 intact THROUGH the tmux spawn** | the launched agent's settings file is **0600** + #10 content (default mode / no `-p` / no bg / generated-not-user settings) intact through `tmux new-session`; `ANTHROPIC_API_KEY` absent in the live tmux-spawned agent's env | 🔬 | P4.1b-2 (lead-directed; un-unit-testable) |
| C5 | tmux graceful-degrade | tmux ABSENT → `PtyLauncher` (B2-achievable: resume/replay, never reattach-live); app works | ⬜ | P4.1b-2 |
| C6 | supervised-child death | child dies (daemon alive) → `SessionFailed` → `proj_session` status=Failed → restart affordance | ⬜ | P4.2 |

## D. UI — screens / features (ui track)

_(accretes as the ui track lands its L2 surfaces — Command Center, Sessions, Approval Queue, Graph, Usage,
Settings, Survival, Diff/PR review, Brain drawer, the §17 safety surfaces. Seed at the ui-L2 go-live.)_

## E. Proof-of-thesis flow (the former PRD §25 demo path)

| # | Flow | Check | State | Origin |
|---|---|---|---|---|
| E1 | PRD §25 path | add project → launch session → permission → approve → review → Brain PR plan → PR created + linked | ⬜ | §19.1 (one flow, not the whole gate) |

## F. Harness coverage

| # | Surface | Check | State | Origin |
|---|---|---|---|---|
| F1 | Claude adapter (PTY-primary) | observe/intercept/telemetry/resume | partial 🔬 | P3.2/4.x |
| F2 | Codex adapter | thread/start/resume; app-server approvals; **defense-in-depth: hook + `--sandbox`**; rollout 0600 (umask 0077) | ⬜ | 3.3 (gated) |

---

_Seeded 2026-06-14 at the 4.2 round seal (daemon track, P4). The 🔬 rows are the
review-verified-but-not-unit-testable safety surfaces this arc surfaced — verified by the manual smoke,
never by a green test._

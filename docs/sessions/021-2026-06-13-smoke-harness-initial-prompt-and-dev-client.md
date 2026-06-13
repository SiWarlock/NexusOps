# Session 021 — P4.0b-2-smoke: the 0.1-HITL smoke harness (initial_prompt thread + dev-client)

- **Date:** 2026-06-13
- **Phase:** 4 (session lifecycle, survival & failure-mode contract) — task **P4.0b-2-smoke**
- **Predecessor:** [020 — P4.0b-2 L2: the live INV-SEC-1 drive loop](020-2026-06-13-live-inv-sec-1-drive-loop-cat1.md)
- **Successor:** _(next session — fresh pair; likely 4.0b-2c the audit-backbone circuit-breaker, then ui-① brief 052, then 4.0b-2-F2)_
- **Commits (on `bd7523b`):** `e79fecb` (C1 — the additive initial_prompt thread, cat-1) · `91fc1fe` (C2 — the thin `nexusopsd smoke` dev-client)
- **Brief:** `docs/briefs/053-P4-0b-2-smoke-harness-live-drive-loop.md` · **Runbook:** `docs/runbooks/smoke-harness-live-drive-loop.md`

## Why this session existed

The live INV-SEC-1 drive loop is *built* (session 020, `bd7523b`) but not yet *demonstrable*: the
as-built session PTY is unreachable through IPC (`session/actor.rs` drops output frames; `SessionCommand`
is `Kill`-only — input/attach is the parked 6.3d), so a launched `claude` has no prompt and does nothing.
This slice makes the loop the user's "see it work" moment — the authorized 0.1-HITL rig — via the lead-ruled
**Option G**: thread a small deterministic `initial_prompt` into the existing `TerminalSession::write` seam
so a real `claude` self-drives a demo prompt (a Read auto-allows, a Bash gates), plus a thin dev-client to
drive `session.create` + observe the approval queue + `approve`/`deny`. **Safety is preserved by construction:**
the prompt only makes claude *act* — every tool call still routes through the unchanged `intercept`
adjudication, the chokepoint, regardless of how claude was prompted.

## What was built

A hybrid slice — **Commit 1 is test-first + security-reviewed (cat-1 session path); Commit 2 is
acceptance-by-review** (a non-deterministic UDS dev tool). Two bisectable commits.

**Files created**
- `daemon/src/smoke.rs` — **C2** the feature-gated (`dev-client`) `nexusopsd smoke <sub>` dev-client: a
  synchronous UDS client (the `hook.rs` precedent — dispatched before the daemon runtime, handshake-first,
  one RPC, print the result; never starts the write-actor/accept-loop). Subcommands `create`/`queue`/
  `approve`/`deny`/`kill`/`audit`. `kill` submits a typed `session.kill` `ActionRequest` via the generic
  `submit_action` (no dedicated `session.kill` IPC method exists; the executor handles the action type).
- `daemon/tests/session_prompt.rs` — **C1** the 3 `#[tokio::test]` integration tests for the prompt thread
  (a `PromptLauncher` test double exposing the launched `FakePty::input_sink` + a write-attempt counter on
  the erroring PTY so the degradation test is a true RED).

**Files modified**
- `daemon/src/gateway/session_executor.rs` — **C1** `write_initial_prompt(&mut launched.terminal, req)`
  called **post-launch, before** `supervisor.spawn_session`: writes `prompt + SUBMIT_TERMINATOR` (`\r`) to
  the launched PTY **exactly once**, **opt-in** (absent/empty → nothing written), **fail-soft** (a write
  error degrades to a recorded `detail` suffix; the session still launches + auto-executes). New
  `SUBMIT_TERMINATOR: u8 = b'\r'` const. `launched` is now `let mut`.
- `daemon/src/ipc/methods.rs` — **C1** extracted the inline `session.create` request-building into a pure
  `build_session_create_request(params) -> Result<ActionRequest, IpcErrorCode>` (testable without a
  `WriteHandle`) + threaded the optional `initial_prompt` param into `ActionRequest.inputs` (alongside the
  existing `execution_profile_id`). +4 in-module `#[cfg(test)]` unit tests.
- `daemon/src/lib.rs` — **C2** `#[cfg(feature = "dev-client")] pub mod smoke;`.
- `daemon/src/main.rs` — **C2** the feature-gated `Some("smoke")` subcommand dispatch (before the daemon runtime).
- `daemon/Cargo.toml` — **C2** the `dev-client` feature (OFF by default; the runbook builds `--features dev-client`).

## Decisions made

- **Prompt-write site = the executor, post-`launch_session`** (Q1). No `SessionLauncher` trait change /
  4.1-broker churn; the prompt is a `session.create` concern, not a launcher concern.
- **Submit terminator = `\r` (CR)** (Q2) — claude's TUI submits on Enter (CR in PTY raw mode). The
  deterministic test asserts the literal bytes; *which* terminator actually submits at the live `claude`
  is the runbook's #1 0.1-HITL watch item (the `\n` fallback is documented).
- **Dev-client = feature-gated `dev-client`** (Q3) — production hygiene; matches the runbook. The CI-rot
  residual (the feature is off in default CI) is owned this slice by running `clippy --all-targets
  --features dev-client -D warnings` + `build --features dev-client` in preflight; a "promote `dev-client`
  to a CI line" Future-TODO is routed.
- **Dev-client surface = create/queue/approve/deny/kill/audit** (Q4) — the full set the runbook drives.
- **`build_session_create_request` extraction** — a behavior-preserving refactor of the inline build,
  done so the param→inputs thread is unit-testable deterministically (orch blessed: not scope creep).
- **No CONTRACT bump** — `initial_prompt` is ad-hoc JSON (no frozen `shared/` type); `CONTRACT_VERSION`
  stays `0.27.0`. The `ActionRequest` struct is unchanged (`inputs` is already a free-form `Value`).

## Decisions explicitly NOT made (deferred)

- **The prompt-feed timing/race** (Q5) — writing immediately post-spawn may race claude's TUI input-handler
  init → early bytes could drop. This is **0.1-HITL / acceptance-by-review** (validated at the user's live
  run; the impl builds without a live run). Buffered immediate write is attempt #1; the settle /
  `SessionStart`-driven-write fallback is the documented operational follow-up (runbook §8) IF the live run
  shows consistent early-input loss. Not over-engineered in this slice.
- **The interactive-terminal UX (Option B)** — the polished session terminal + inline permission card is
  the ui-track **6.3d**, not this dev rig.
- **The deferred dev-client ergonomics** (code-quality, accepted, dev-tool): `flag_value` silently skips a
  trailing optional `--prompt`/`--profile` with no value (runbook gives exact commands); the `cmd_kill`
  placeholder-timestamp readability; the `rest.get(1..)` infallible nit. None are correctness bugs.

## TDD compliance

**Clean.**
- **C1 (the `initial_prompt` thread)** — test-first. The 3 integration tests + 4 in-module unit tests were
  written and watched RED for the right reason (assertion: empty sink / 0 write-attempts; compile: `cannot
  find function build_session_create_request`) before any implementation, then driven GREEN. The
  write-error test was strengthened at Step-3 from a vacuous green to a true RED (asserting the write was
  *attempted*) — a refinement of the same asserted invariant (fail-soft), not a conceptual change.
- **C2 (the dev-client)** — **test-first exempt** (the TDD posture's non-deterministic surface: a UDS dev
  tool with no deterministic failing test to write first). Covered via the project's non-deterministic
  path — acceptance-by-review (code-quality-reviewer) + the operator runbook. No violation.

## Reachability

- **The `initial_prompt` thread** — reachable from the **UDS accept-loop** (`main.rs::spawn_accept_loop`) →
  `dispatch` → `"session.create" => session_create` → `build_session_create_request` (threads
  `initial_prompt`) → `submit_action_blocking` → the registered `SessionExecutor` (`ExecutorKind::Session`,
  `main.rs`) → `execute_create` → `write_initial_prompt`. (Confirmed at `/tdd` Step 7.5.)
- **The dev-client** — reachable from `main.rs::main` (the feature-gated `Some("smoke")` dispatch) →
  `nexusopsd::smoke::run` → the reachable IPC methods over UDS (`session.create`/`approve`/`deny`/
  `submit_action`/`get_projection`). The dev-client is itself the production caller for the smoke run.
- **No tested-but-unwired gaps.**

## Open follow-ups (Step-9 categorized list — routed hot to the orchestrator; its `/orchestrate-end` is the verify pass)

- **Architecture doc note (orch hot-write):** the §9.1/§6.3 AS-BUILT — the live drive loop accepts an
  optional `initial_prompt` fed to the session PTY via the existing `TerminalSession::write` seam (the
  Option-G dev-drive; distinct from the parked 6.3d interactive input). No safety-invariant change
  (security-reviewer PASS confirms `intercept.rs` / the O-13 launch argv / `ClaudeSettings` untouched).
  Folds into the pending §9.1 live-drive-loop AS-BUILT prose.
- **Convention candidate → LESSON (orch confirmed banking as §30):** "a best-effort dev-convenience I/O on
  a safety path degrades soft, never fail-closes the safety action."
- **Future TODO (next working set):** promote `dev-client` to a CI lint/build line — it is off in default
  CI (rot risk); verified `--features dev-client` clippy/build this slice.
- **Future TODO (operational, gated on the live run):** the prompt-feed timing fallback (a settle /
  `SessionStart`-driven write) IF the user's live run shows consistent early-input loss (runbook §8).
- **0.1-HITL items the user's live run validates** (runbook §6, previously flagged pending a real Claude):
  the live loop runs · auto-allow works (Read = `agent.file_read` risk-0) · gating works (Bash =
  `agent.bash` risk-2) · the `PreToolUse` hook allow/deny grammar is honored · hook-miss fails closed · no
  integrity alarm on the happy path.
- **Deferred dev-client ergonomics** (accepted, no action) — see "Decisions NOT made."

## How to use what was built

The runbook `docs/runbooks/smoke-harness-live-drive-loop.md` is the operator guide. In short, from a Max-plan
clean env (`claude setup-token` → `CLAUDE_CODE_OAUTH_TOKEN`, no `ANTHROPIC_API_KEY`): build
`cargo build --release --features dev-client`, start `nexusopsd`, then
`nexusopsd smoke create --project <id> --prompt '<a Read then a Bash, two tool calls>'`, watch the Read
auto-allow + the Bash land in `nexusopsd smoke queue`, and `nexusopsd smoke approve <appr_…>` /
`deny <appr_…> <reason>` over IPC.

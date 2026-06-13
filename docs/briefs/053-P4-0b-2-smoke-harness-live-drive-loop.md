# /tdd brief — smoke_harness_initial_prompt_and_dev_client

## Feature
Make the live INV-SEC-1 drive loop demonstrable end-to-end: add an **additive `initial_prompt`** to
`session.create` (threaded to the launched session's PTY so a real `claude` actually does work), plus a
**thin `nexusopsd` dev-client subcommand** that drives `session.create` + observes the approval queue +
`approve`/`deny`s — the authorized **0.1-HITL smoke rig** (lead-ruled **Option G**, 2026-06-13).

## Use case + traceability
- **Task ID:** P4.0b-2-smoke (the `### 4.0b-2-smoke` Phase-4 row; the user's "see it work" + the 0.1-HITL empirical validation)
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (the live HarnessAdapter drive loop), `§6.3` (the agent-mutation catalog, live), `§0.1` O-4 (the Claude supervision-mode 0.1-HITL validation), `§15` (INV-SEC-1 — the live interception, **unchanged** by this slice)
- **Related context:** brief **051** (the live drive loop it exercises) · session doc **020** · the lead's Option-G ruling (drive via the existing `pty.write` seam, **NOT** the O-13 argv) · the runbook `docs/runbooks/smoke-harness-live-drive-loop.md` (authored alongside this brief — the operator-facing run steps).

> **Why this is a hybrid (TDD + acceptance-by-review), not "thin client only":** the as-built session
> PTY is **not reachable through IPC** (`session/actor.rs:107` DROPS output frames, `SessionCommand` is
> `Kill`-only — input/attach = parked 6.3d), so a launched `claude` has no prompt + no input route and
> does nothing. Option G threads a small, **deterministic** `initial_prompt` into the existing
> `TerminalSession::write` seam (`terminal/mod.rs:258`) so claude self-drives a demo prompt. That thread
> is **test-first** (lead condition); the dev-client is **acceptance-by-review** (a non-deterministic
> UDS-client dev tool, like the 4.0b-2 live-transport layers). **Safety is preserved:** the prompt only
> makes claude *act* — every tool call still routes through the unchanged `intercept` adjudication (the
> interception is the chokepoint regardless of how claude was prompted). The thread does **NOT** touch
> the O-13 `ClaudeLaunchSpec` argv / the #10 enforcement surface.

## Acceptance criteria (what "done" means)

**Commit 1 — the `initial_prompt` thread (TDD, security-reviewed; cat-1 session path):**
- [ ] `session.create` accepts an optional `initial_prompt: string` param (additive, ad-hoc JSON — same handling as the existing `project_id`/`execution_profile_id`; **NO `shared/` contract type, NO CONTRACT bump** — confirm `CONTRACT_VERSION` stays `0.27.0`).
- [ ] The IPC `session_create` (methods.rs) carries `initial_prompt` into the `ActionRequest.inputs` JSON (alongside `execution_profile_id`).
- [ ] `SessionExecutor::execute_create` reads `initial_prompt` from `req.inputs` and, **after** `launch_session()` returns, writes the prompt bytes (+ a submit terminator) to the launched session's PTY via `launched.terminal.write(...)`, **exactly once**, **before** handing the session to `supervisor.spawn_session`.
- [ ] When `initial_prompt` is absent, **nothing** is written (additive/opt-in — back-compat with 4.0b-1/4.0b-2; the existing no-prompt path is byte-unchanged).
- [ ] A PTY write error does **not** crash the executor — it degrades to a recorded launch detail (the session still spawns; the prompt-feed is best-effort dev convenience, not a safety path). Confirm fail-soft, never fail-closed-the-session-on-a-prompt-write (this is NOT a safety I/O).
- [ ] The interception path is **unchanged** — no edit to `harness/claude/intercept.rs`, the O-13 `ClaudeLaunchSpec` argv, or the `ClaudeSettings`. (security-reviewer confirms; see Cross-doc.)
- [ ] All unit tests in `daemon/tests/session_executor.rs` (or a new `daemon/tests/session_prompt.rs`) pass.

**Commit 2 — the thin dev-client subcommand (acceptance-by-review):**
- [ ] A `nexusopsd <devcmd> <sub>` subcommand (dispatched in `main.rs` **before** the daemon runtime, the `hook`-subcommand precedent — a synchronous UDS client, never starts the write-actor/accept-loop) with the minimal surface:
  - `create --project <id> [--prompt "<text>"] [--profile <id>]` → handshake → `session.create` → prints the `ActionAck` (session id / action_request_id / status).
  - `queue` → `get_projection ApprovalQueue` → prints pending approvals (`approval_id` + `action_type` + key fields).
  - `approve <approval_id>` → the `approve` method.
  - `deny <approval_id> <reason>` → the `deny` method.
  - (optional) `kill <session_id>` → `session.kill`; `audit` → `get_projection AuditTrail`.
- [ ] Every call does the IPC GatewayPort handshake-first (`HelloFrame` → read `HelloAck`) then one `RpcRequest`, mirroring `hook.rs` (same socket path, same framing helpers). _(reuses the existing handshake mechanism — not a new anchor.)_
- [ ] Errors print a clear message + non-zero exit (it's a dev tool; fail-closed isn't required — it submits intents, the daemon adjudicates).
- [ ] `/preflight` clean (fmt + clippy `-D warnings` + check + test).

## Wiring / entry point (Step 7.5)
- **The prompt-thread** is reachable on the **already-live** path: IPC `session.create` → `submit_action_blocking` → the registered `SessionExecutor` (`ExecutorKind::Session`, main.rs) → `PtyLauncher` → (new) `launched.terminal.write(prompt)`. No new production wiring — it extends the reachable 4.0b-2 path.
- **The dev-client** is the new entry point: `nexusopsd <devcmd> create ...` (dispatched in `main.rs::main` before `run()`, like `hook`). It is the production caller for the smoke run.

## Files expected to touch
**New:**
- `daemon/src/smoke.rs` (name TBD at Step 2.5 — `smoke`/`devclient`) — the thin UDS dev-client subcommand (acceptance-by-review).
- `daemon/tests/session_prompt.rs` (or extend `daemon/tests/session_executor.rs`) — the prompt-thread RED tests.

**Modified:**
- `daemon/src/ipc/methods.rs` — `session_create` reads `initial_prompt` → `ActionRequest.inputs`.
- `daemon/src/gateway/session_executor.rs` — `execute_create` writes the prompt to `launched.terminal` post-launch.
- `daemon/src/main.rs` — dispatch the dev-client subcommand before the daemon runtime.
- `daemon/src/lib.rs` — `+pub mod smoke` (feature-gated if Step-2.5 Q3 chooses).
- `daemon/src/session/launcher.rs` — IF the test needs `FakeLauncher` to expose the `FakePty::input_sink` (the `input_sink()` seam at `terminal/mod.rs:670`); else a bespoke test launcher in the test file.
- `daemon/Cargo.toml` — IF a `dev-client` cargo feature is added (Step-2.5 Q3).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2) — Commit 1 (the deterministic thread)
Tests in `daemon/tests/session_prompt.rs` (or `session_executor.rs`):

1. **`test_session_create_initial_prompt_written_to_pty`** — a `SessionExecutor` over a test launcher producing a `FakePty` whose `input_sink()` the test captured; submit a `session.create` `ActionRequest` whose `inputs` carry `initial_prompt`.
   - Asserts: the captured `input_sink` contains the prompt bytes (+ the chosen terminator) **exactly once**.
   - Why: the Option-G drive mechanism — the prompt reaches the agent's PTY (`§9.1` live drive loop).

2. **`test_session_create_no_prompt_writes_nothing`** — same setup, `inputs` with **no** `initial_prompt`.
   - Asserts: the `input_sink` is empty.
   - Why: additive/opt-in; the 4.0b-1/4.0b-2 no-prompt path is byte-unchanged (back-compat).

3. **`test_session_create_prompt_threaded_from_ipc_params`** — the IPC `session_create` builds an `ActionRequest` whose `inputs.initial_prompt` == the param.
   - Asserts: the param→inputs thread at the IPC boundary.
   - Why: the thread is complete from the wire, not just the executor.

4. **`test_initial_prompt_write_is_after_launch`** (if cleanly expressible) — the write targets the *launched* terminal (so the PTY exists), not a pre-launch no-op.
   - Asserts: ordering (write follows `launch_session`).
   - Why: a prompt written before spawn would be lost.

5. **`test_prompt_write_error_does_not_fail_the_session`** — a launcher/FakePty whose `write` errors.
   - Asserts: the session still spawns (`Succeeded`/`PartiallySucceeded` per §17, not a hard executor crash); the prompt-feed degrades.
   - Why: the prompt-feed is best-effort dev convenience, NOT a safety I/O path (don't fail-close the session on it).

> **Interception-preservation** is asserted by the EXISTING `tests/claude_intercept.rs` + the inverted guard `tests/session_executor.rs::test_live_session_create_has_interception` (unchanged) + the **security-reviewer's eye** (lead condition) — not a new test (this slice does not touch the adjudication path).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/`. `session.create`'s `initial_prompt` is **ad-hoc JSON** params (no frozen `SessionCreateParams` type) → **NO CONTRACT bump** (`CONTRACT_VERSION` stays `0.27.0`). Confirm at Step 9.
- **Contract schema-snapshot seam (shared-contract) model touched?** No — no Appendix-A model, no schema snapshot (session.create params are ad-hoc JSON, not a frozen type).
- **Orchestrator doc rows to write hot (Step 9 routing):** a `§9.1`/`§6.3` **AS-BUILT note** — the live drive loop accepts an optional `initial_prompt` fed to the session PTY (the Option-G dev-drive mechanism); folds into the pending §9.1 live-drive-loop AS-BUILT prose the orchestrator owes at the seal. No safety-invariant change (escalate-note: confirmed by the security-reviewer that the interception/#10 argv are untouched).

## Things to flag at Step 2.5
1. **Where the prompt write happens — the executor (post-`launch_session`) vs. threaded into the launcher (`launch_session(initial_prompt)`).** My default vote: **the executor, post-launch** — no `SessionLauncher` trait change, no `FakeLauncher`/4.1-broker churn; the prompt is a `session.create` concern, not a launcher concern. (Alternative: launcher-threaded keeps spawn-time I/O cohesive but churns the trait + the 4.1 swap contract.)
2. **Submit terminator — `\r` (CR) vs `\n` (LF).** My default vote: **`\r`** — claude's TUI submits on Enter (CR in PTY raw mode). The deterministic test asserts whatever bytes are written; **which terminator actually submits is a live-run unknown** (documented in the runbook with a fallback). This is the **#1 live-run watch item** (see Q5 / the runbook).
3. **Dev-client packaging — feature-gated (`dev-client` cargo feature) vs. always-compiled subcommand (like `hook`).** My default vote: **feature-gated** — production hygiene (the smoke client isn't needed in prod; the runbook builds `--features dev-client`). (Alternative: always-on is simpler to run from a plain release build; the `hook` precedent is always-on but `hook` IS needed in prod.)
4. **Dev-client subcommand surface.** My default vote: **`create`/`queue`/`approve`/`deny`** as the minimal set, `kill`/`audit` optional. Confirm or trim.
5. **Prompt-feed timing (the #1 live-run risk — NOT a code-decision, a flag).** Writing the prompt immediately post-spawn may race claude's TUI input-handler init → early bytes could be dropped. This is **acceptance-by-review / 0.1-HITL** (validated at the user's live run, since the impl builds without a live run). The runbook documents the fallback: if the prompt doesn't submit, (a) try the other terminator, (b) the implementer may add a brief pre-write settle or drive the write off the `SessionStart` hook signal in a follow-up. Don't over-engineer in this slice — buffered immediate write is attempt #1.

## Dependencies + sequencing
- **Depends on:** 4.0b-2 (the live drive loop: the reachable `session.create` + the live interception + the launcher — all landed `bd7523b`).
- **Blocks:** the user's 0.1-HITL "see it work" run + the empirical permission-grammar / hook-miss validation. The interactive-terminal UX (Option B) is the ui-track **6.3d** follow-on, NOT this slice.

## Estimated commit count
**2.** (1) the `initial_prompt` thread — **its own commit** (cat-1 session path; TDD'd + security-reviewed, never bundled). (2) the dev-client subcommand (acceptance-by-review, non-safety). Bisectable + separable.

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the Option-G dev-drive mechanism (an `initial_prompt` fed to the session PTY via the existing `write` seam, distinct from the parked 6.3d interactive input path).
- **Future TODO — operational** — the prompt-feed timing fallback (settle / SessionStart-driven write) if the live run shows early-input loss → a small hardening follow-up.
- **Convention candidate** (maybe) — "a best-effort dev-convenience I/O on a safety path degrades soft, never fail-closes the safety action."

## How to invoke
1. Read this brief end-to-end + skim the runbook (`docs/runbooks/smoke-harness-live-drive-loop.md`) for the operator-facing shape the dev-client must satisfy.
2. Run `/tdd smoke_harness_initial_prompt_and_dev_client`.
3. Step 0 (Restate) — confirm against the Feature line.
4. Step 2.5 — answer the 5 design questions (or take defaults) + send the test write-up; **don't go GREEN until APPROVED**.
5. Step 8 — the `initial_prompt` thread is a cat-1 session-path change → **security-reviewer** (`invariant` policy: confirm the interception + #10 argv are untouched, the prompt-feed degrades soft). The dev-client is non-safety → code-quality-reviewer per `every-slice`.
6. Step 9 — surface the no-CONTRACT-bump confirmation + the §9.1 AS-BUILT note + the timing fallback.

# Session 040 — WAVE-1 daemon control lane (W1-prof · W1-exec · W1-git-stage)

- **Date:** 2026-06-26
- **Phase:** Phase 4/5 — the WAVE goal ("every daemon feature WIRED into the cockpit, functional + smoke-testable"). WAVE-1 = session-lifecycle control + per-hunk git staging.
- **Predecessor:** [039-2026-06-26-add-project-arc-089-090-091-092.md](039-2026-06-26-add-project-arc-089-090-091-092.md)
- **Successor:** _(next daemon session — TBD; fresh pair queue: git.discard(A content-hash) → W2-audit event_type)_

## Why this session existed

The add-project arc had sealed (HEAD `9ad5461`); the team launched the **WAVE goal** — wire every daemon user-facing feature into the cockpit so it's functional + smoke-testable. WAVE-1 is the #1 gap: the cockpit could VIEW agents but not DRIVE them (session lifecycle controls), and the per-hunk git-staging UI (ui-6.3e) was wired-but-stubbed. This session implemented the three WAVE-1 **daemon** slices that unblock those UI controls.

## What was built

Three test-first `/tdd` slices, all sealed (LOCAL; push user-gated).

### Files modified

**W1-prof — `get_execution_profiles` read RPC + secret-free `ProfileRow` (`1cbb712`, CONTRACT 0.47.0→0.48.0):**
- `shared/src/ipc.rs` — NEW `ProfileRow` (`{execution_profile_id, provider, harness, model?, account_alias?, status, is_default, has_credential}`) + `GetExecutionProfilesResult` (`{profiles}`) — a read-RPC result, NOT a projection row (the `DiffResult` precedent).
- `shared/src/lib.rs` — `CONTRACT_VERSION` → 0.48.0. `shared/src/schema.rs` — the 2 types in `ContractBundle`. `shared/contracts/schema/nexusops-contract.schema.json` — regen.
- `daemon/src/ipc/methods.rs` — `read_execution_profile_rows` (read-only WAL reader, §15 #4 keychain POINTER consumed only to derive `has_credential`, never served) + `get_execution_profiles` dispatch fn + the dispatch arm. `daemon/src/ipc/mod.rs` — `pub use`.
- `shared/tests/contract.rs` (+2), `daemon/tests/get_execution_profiles.rs` (NEW, 5), `daemon/tests/ipc.rs` (+1) — 8 tests.

**W1-exec — session control executor bodies + cross-thread `PtyWriter` seam (`c83765b`, contract-neutral):**
- `daemon/src/session/actor.rs` — `SessionCommand` += `SendMessage(String)`/`Pause`/`Resume` (drops `Copy`); the command-loop arms; a `paused` flag gating the status/telemetry DRIVE arms (soft-pause, NO §5.1 value); status ticker → `MissedTickBehavior::Delay` (no resume catch-up burst); `MESSAGE_SUBMIT_TERMINATOR`.
- `daemon/src/terminal/mod.rs` — a NEW cross-thread `PtyWriter` seam (trait + `Pty::writer()` + `TerminalSession::writer()` + `PortablePtyHost` shared-`Arc<Mutex>` writer + `FakePty` writer/`fail_writes()`). **Write-only by construction** so #9 holds structurally.
- `daemon/src/session/mod.rs` — `SupervisorControl` made `pub` + a test-support `SupervisorHandle::from_sender`.
- `daemon/src/gateway/session_executor.rs` — `execute_send_message` + `execute_route` + `validated_target` + 3 consts + 3 match arms.
- `daemon/tests/{session_executor,session,session_prompt}.rs` — 10 tests (incl. the deterministic `session_pause_gates_the_status_drive`).

**W1-git-stage — `git.stage_hunk`/`unstage_hunk` executor bodies (`b3728c6`, contract-neutral):**
- `daemon/src/git/executor.rs` — `execute_apply_hunk(reverse)` (the 2 bodies differ only by `-R`) + the NEW injected `WorktreePathResolver` seam (trait + `DbWorktreePathResolver` prod, read-only WAL) + `decode_hunk_ref` + the 0600 O_EXCL `TempPatch` + consts/arms + the `with_worktree_resolver` builder.
- `daemon/src/git/mod.rs` — the pure `find_hunk_patch` helper (byte-faithful `@@`-slice).
- `daemon/src/main.rs` — wire the prod `DbWorktreePathResolver`.
- `daemon/tests/git_executor.rs` — 10 tests (incl. the `--check`-fail-closed + unresolvable-worktree pins + a per-command `CheckFailsGitCli` fake).

## Decisions made

- **W1-prof — `is_default` from the first `ExecutionProfileRegistered` event** (no migration; matches `seed_default_profile`/`SqliteProfileLookup::default_id`). The read **soft-degrades** (no default flagged) if the seed event is absent/corrupt — a non-load-bearing UI hint (LESSON §30), distinct from the row-data fail-closed (LESSON §37). Documented + tested. §15 #4: ProfileRow has no field for the keychain pointer (secret-free by construction). [security-reviewer CLEAR]
- **W1-exec — soft-pause:** `Pause`/`Resume` gate the actor's status/telemetry DRIVE loop via a flag (NO new §5.1 `Session` value — the 17-value enum is frozen); the agent PROCESS is NOT OS-suspended in MVP. `SendMessage` feeds the PTY via a cross-thread writer grabbed before the pump (the `killer()` precedent); fail-soft (LESSON §30). The bodies route-only + emit NO event → no new authority. [security-reviewer CLEAR]
- **W1-git-stage — §17 race posture (lead/orch-confirmed):** the patch is re-derived from the LIVE `git diff`; the **position-only resource-ref IS the audited unit**, so executed==audited at that granularity. Two guards: #1 no-hunk-match → Failed (the primary — displayed hunk gone/file shifted); #2 `git apply --check` FIRST (secondary — concurrent index change). The same-position content-drift case is an accepted limitation (recoverable for stage/unstage) — **explicitly NOT carried to the destructive discard slice**. CLI guard family: structural reasons (no stderr), reject-dash (LESSON 45), resolve-from-audited-ref (LESSON 63), 0600 O_EXCL temp patch. [security-reviewer CLEAR]

## Decisions explicitly NOT made (deferred)

- **W1-exec real OS-suspend pause** — the MVP soft-pause gates the drive loop only; a true process suspend is a follow-on (the UI must label "Pause monitoring" honestly — done in ui-087). Lead-routed.
- **W1-exec send_message §13 input-fidelity** — a secret-shaped token in the message over-redacts on the risk-2 approve path (invariant holds; over-redact never leak). Deferred hardening (already lead-routed, the create_worktree precedent).
- **W1-git-discard** — `git.discard_hunk` (risk-3 DESTRUCTIVE) is a SEPARATE slice the orchestrator authors with its own safety scrutiny. Lead ruled **(A) content-hash in the audit** for the destructive race (the position-is-enough posture does NOT carry).
- **stdin-no-disk patch** (W1-git-stage) — pipe the patch to `git apply` via stdin (avoid the temp file entirely); a §15 hygiene nicety, deferred to §10.6.
- **084 device-flow** — still PARKED pre-GREEN (Step-2.5 approved); the next daemon slice candidate per the carry-forward.

## Preflight

**Daemon gate GREEN** — `cargo clippy --all-targets -D warnings` ✓ · `cargo fmt -p nexusopsd --check` ✓ · `cargo check --all-targets` ✓ · `cargo test` **1113 pass, 3 ignored** ✓. The only workspace `cargo fmt --check` failure is `ui/gateway-uds/src/lib.rs` — the **ui pair's** in-progress file (their W1-prof gateway-uds consumer, ui-085/088), NOT daemon territory and not formatted here (shared-tree / cross-track protocol). Daemon `src/` + `shared/` are clean. Not a daemon-side incompleteness.

## TDD compliance

**Clean — all three slices test-first.** Each ran RED (confirmed failing for the right reason: missing symbols) → Step-2.5 orchestrator review → GREEN → reachability → security + code-quality reviewers → scoped commit. No TDD violations. Reviewer findings (3 Step-2.5 ADDs, 1 §17 design Q, 7 code-quality findings) were all addressed in-slice before commit.

## Cross-doc invariant audit

- **W1-prof:** NEW `ProfileRow` + `GetExecutionProfilesResult`; `CONTRACT_VERSION` → 0.48.0. **Flagged at Step-9** (Cross-doc invariant change) — the orchestrator confirmed receipt + routes the `daemon/CLAUDE.md` IPC-GatewayPort row + the §6.1 Appendix-A mirror + LESSON §70 hot at `/orchestrate-end`. The §2.5-seam schema snapshot (`shared/tests/contract.rs`) + the 3-way verify (42 enums @0.48.0) landed in my Step-10 commit. The paired UI regen-to-0.48 = ui-085 (DONE).
- **W1-exec / W1-git-stage:** contract-neutral (no `shared/` model change; `SessionCommand`/`SupervisorControl`/`WorktreePathResolver` are daemon-internal). Architecture doc notes (the §9.1/§6.3 executor rows now LIVE) flagged at Step-9; orchestrator writes them. No drift.

_(Single-track: the orchestrator shares this checkout and committed its hot-routing in round commits — e.g. `7a74d0c` for W1-git-stage. No outstanding doc edit owed.)_

## Reachability

- **W1-prof `get_execution_profiles`** — reachable from `serve_connection`→`dispatch` (`"get_execution_profiles"` arm). Pinned by `test_get_execution_profiles_reachable_through_ipc_dispatch` (tests/ipc.rs).
- **W1-exec `session.send_message`/`pause`/`resume`** — reachable from the gateway execute via the registered `ExecutorKind::Session` `SessionExecutor` (main.rs) → the match arms. Pinned by `session_control_bodies_only_via_gateway_execute` + the existing `test_live_session_create_has_interception` registration pin + a codegraph trace (`execute → execute_send_message → supervisor.route`).
- **W1-git-stage `git.stage_hunk`/`unstage_hunk`** — reachable from the gateway execute via the registered `ExecutorKind::Git` `GitExecutor` (main.rs, now wired with the live `DbWorktreePathResolver`) → the match arms → `execute_apply_hunk`. The `create_worktree` precedent.

No tested-but-unwired gaps.

## Open follow-ups

Step-9 categorized items (orchestrator is routing; recorded here for continuity):
- **Future TODO — W1-exec:** real OS-suspend pause · send_message §13 input-fidelity · the synchronous PTY write in the actor `select!` loop could briefly block on a full child input buffer (a non-blocking send is a refinement).
- **Future TODO — W1-git-stage (§10.6):** stdin-no-disk patch · `TempPatch::path_string` `to_string_lossy` → a clearer diagnostic on a Linux non-UTF-8 `TMPDIR` (darwin-safe today).
- **Convention candidates** — LESSON §70 (secret-free §2.8-registry read RPC + the §30-vs-§37 asymmetry), §71 (session-control body convention + cross-thread PtyWriter #9-structural + soft-pause-gates-drive), §72 (per-hunk git-mutation re-derive-from-live-diff + two-guard §17 race + position-only accepted-limitation-not-for-destructive + the CLI guard family). Orchestrator banks them.
- **Architecture doc notes** — the §6.1 read surface (get_execution_profiles/ProfileRow @0.48.0; execution_profiles registry now UI-readable) · the §9.1/SessionExecutor row (send_message/pause/resume LIVE) · the §6.3/GitExecutor row (stage_hunk/unstage_hunk LIVE). Orchestrator writes.
- **Next daemon work** (orchestrator-sequenced, fresh pair): **git.discard(A content-hash)** → **W2-audit `event_type`** (W2 projection honesty). 084 device-flow still PARKED.

## How to use what was built

- The cockpit profile picker reads `get_execution_profiles` (gateway-client) → `ProfileRow[]`; `is_default` pre-selects, `has_credential` shows credential state (no secret).
- The session cockpit drives agents: "Launch agent" (session.create), Kill, **Send message / Pause / Resume** (W1-exec), Change profile (session.profile_change). Pause is labelled "Pause monitoring" (honest — soft-pause).
- The per-hunk diff view stages/unstages individual hunks (risk-2, approval-gated) — the daemon re-derives the hunk from the live diff + applies to the index.

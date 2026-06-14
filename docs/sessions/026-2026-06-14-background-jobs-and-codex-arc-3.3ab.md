# Session 026 — Phase-4 tail (4.3 background jobs) + the Codex arc head (3.3a/3.3b)

- **Date:** 2026-06-14
- **Phase:** Phase 4 (deterministic core completion) + Phase 3.3 (the Codex adapter arc)
- **Predecessor:** [025-2026-06-14-survival-broker-and-child-death-surface.md](025-2026-06-14-survival-broker-and-child-death-surface.md)
- **Successor:** _(3.3c — the CAT-1 Codex interception; fresh implementer on the user's resume)_
- **Role:** daemon-implementer (single-track, `main`). Cycled proactively at a clean boundary during the ui↔edges merge-hold (before the cat-1 3.3c).

## Why this session existed

Three deterministic, NON-cat-1 slices at a clean phase boundary: finish Phase 4's deterministic core (the background maintenance jobs), then open the 3.3 Codex-adapter arc (the observe core + the launch/auth/profile/umask/resume mechanism) — all mechanism-first, no live spawn, so the CAT-1 interception (3.3c) lands cleanly on a fresh session.

## What was built

### 4.3 — background jobs (commit `81c9242`)
- **Files created:** `daemon/src/runtime/jobs.rs` (the WAL checkpointer + the session-staleness poller + the pure `is_stale`/`recompute_staleness_once`); `daemon/tests/jobs.rs` (7 tests).
- **Files modified:** `daemon/src/eventstore/mod.rs` (`WalCheckpointMode`/`WalCheckpointSummary` + `EventStore::wal_checkpoint`); `daemon/src/runtime/writer.rs` (`Command::WalCheckpoint` + `WriteHandle::wal_checkpoint` + dispatch); `daemon/src/runtime/mod.rs` (re-exports); `daemon/src/main.rs` (production wiring: `spawn_wal_checkpointer` PASSIVE/60s + `spawn_staleness_poller` 120s-threshold/30s, shutdown-awaited).

### 3.3a — CodexAdapter parsing + normalization (commit `c4a4652`)
- **Files created:** `daemon/src/harness/codex/{status,parse,stream,mod}.rs` (the two-parsers→one-`CodexSignal`→one-`derive_status` observe core, classify-by-semantics, `CodexAdapter`+`CODEX_CAPABILITIES`); `daemon/tests/codex_adapter.rs` (10 tests) + `daemon/tests/fixtures/codex/` (2 synthesized secret-free golden fixtures).
- **Files modified:** `daemon/src/harness/mod.rs` (`pub mod codex`).

### 3.3b — CodexAdapter launch + auth/profile + umask + resume-handle (commits `16962c9` C1 §15#8 · `a822015` C2 §15#11 · `a571a62` C3)
- **Files created:** `daemon/src/harness/codex/auth.rs` (§15#8 — `resolve_codex_auth` single-source pin, `auth_env_mutations`, `CodexExecutionProfile`/`bind_codex_profile` keychain-ref/no-Debug-leak); `daemon/src/harness/codex/perms.rs` (§15#11 — `harden_codex_dirs` 0700 fail-closed, `CODEX_CHILD_UMASK=0o077`); `daemon/src/harness/codex/launch.rs` (`CodexLaunchSpec` no-bypass argv + env-hygiene + resume variant, `has_resume_handle`); `daemon/tests/{codex_auth,codex_perms,codex_launch}.rs` (6+3+6 tests).
- **Files modified:** `daemon/src/harness/codex/mod.rs` (module decls + `CodexAdapter::resume_inputs`).

## Decisions made

- **4.3 Q1–Q4 (lead/orch-ruled):** `runtime/jobs.rs` module home; staleness = read-time live recompute (no served `stale` field, no CONTRACT bump); WAL checkpoint PASSIVE on the interval (TRUNCATE available via the enum); missing/unparseable heartbeat → NOT stale.
- **3.3a Q1 (lead-ruled):** the §5.1 status mapping is turn-scoped **non-terminal only** — `task_complete`/`turn_aborted`/`turn.failed` → `Idle`, NEVER terminal `Failed`/`Completed` (a Codex session is multi-turn; a turn boundary ≠ session death; mapping abort→terminal would R-9-sink a recoverable session). Terminal states derive from the process-exit signal (3.3b/c live).
- **3.3a Q2–Q4:** `CodexToolKind{ShellExec,FilePatch,McpTool,Other}` classify-by-semantics; synthesized redaction-safe fixtures; `context_pct=total/model_context_window`→Exact else Unavailable; `tokens_in/out`=input/output (cached/reasoning sub-breakdowns not double-counted).
- **3.3b §15#8 (orch TWEAK):** the env-hygiene **single-source pin** — `resolve_codex_auth` refuses ≥2 distinct methods AND same-method conflicting values (no contestable precedence owned); same-value dup → canonical-pin+strip; the secret VALUE is compared-then-dropped (only the var-NAME + a keychain-ref enter the profile; `AuthSources` has no `Debug`).
- **3.3b Q1 (NON-escalating, orch-confirmed):** the spec makes `--sandbox`/`--ask-for-approval` structurally mandatory (no-bypass surface) but ships NO containment default + NO spawn → no cat-1 decision here; the sandbox value/containment proof + the hook+sandbox=INV-SEC-1 argument are 3.3c (lead→user). Guard: `sandbox`/`approval_policy` are required (no `Default` on the profile).
- **3.3b code-quality bugfix:** `resume_inputs` gates `has_resume_handle` on the **rollout file's existence** (not a non-empty `session_id`) so a fresh session → `Relaunched`, not a spurious `Resumed`.

## Decisions explicitly NOT made (deferred)

- **3.3c (CAT-1):** the live codex spawn + the `PreToolUse`→Gateway interception + the `--sandbox` defense-in-depth containment proof (the binding condition — they land together; design surfaces lead→user).
- **3.3d:** Codex telemetry emission + pricing/cost-derivation (`cost_estimate=0.0` now).
- **HITL:** the live umask-doesn't-chmod-back verify (Open-Q #9); the app-server `thread/resume` + UUID↔`thr_` interconversion (Open-Q #3/#4); the OSS-version fixture/flag-grammar refresh (#5); AgentIdentity auth auto-detection (#8).
- **4.3 follow-ons:** the live `last_heartbeat_at` producer (the poller is correct-but-dormant until it lands); the served `stale` projection field (ui-consuming slice); TRUNCATE-on-size-threshold.

## TDD compliance

**Clean — no violations.** All 3 slices test-first: RED confirmed (compile-failure for the missing symbols/modules) before any GREEN implementation, for every slice. Review-driven fixes (the §15#8 single-source tightening, the `resume_inputs` rollout-existence bugfix, the stream `item.completed` double-count) were applied as test+code together within the slice.

## Cross-doc invariant audit

**Clean — no `shared/` model field changes this session; NO CONTRACT bump on any of the 3 slices.** All daemon-internal: 4.3 (jobs + a maintenance command), 3.3a (CodexAdapter on the frozen §9.1 types), 3.3b (launch/auth/profile on the frozen `ExecutionProfileId`/`ExecutionProfile` @0.24.0 + daemon-internal `ResumeInputs`). Nothing required an `ARCHITECTURE.md` model-row edit; nothing went unflagged at Step 9. (Orch hot-routed AS-BUILT prose + LESSONS §41/§42/§43 — orchestrator territory.)

## Reachability

- **4.3:** `spawn_wal_checkpointer`/`spawn_staleness_poller` WIRED in `main.rs::run` (the `#[tokio::main]` entry) + shutdown-awaited. Checkpoint rides the single write-actor; poller reads read-only WAL.
- **3.3a:** the CodexAdapter observe core has NO production caller by design (the live stream is 3.3c) — reachable via the `HarnessAdapter` trait + the `push_signal` seam; fixture-tested. (Named "observe built, live caller later" deferral — the Claude-042 precedent.)
- **3.3b:** the launch/auth/profile/umask mechanism has NO production spawn caller by design (3.3c) — `resume_inputs()` feeds `ResumeInputs` (consumer = 3.3c's `recover_sessions_on_restart`); the no-spawn binding is structurally pinned by `codex_launch.rs::test_no_codex_spawn_in_slice`.

No tested-but-unwired gaps beyond the named binding-condition deferrals (3.3a/3.3b live callers = 3.3c).

## Open follow-ups

All routed hot by the orchestrator during the session (its `/orchestrate-end` is the verify pass) — captured here:
- **Carry-forward (3.3 arc):** Codex pricing/cost-derivation → 3.3d; OSS-version fixture+flag refresh → HITL (#5); app-server `thread/resume` + UUID↔`thr_` → HITL (#3/#4); the live umask-doesn't-chmod-back verify → HITL (#9); AgentIdentity auto-detection → HITL (#8).
- **Security NIT (3.3c):** confirm `codex_home` is daemon-resolved (from `$HOME`/config), NOT agent-controlled, at the live `harden_codex_dirs` caller.
- **Carry-forward (4.3):** the dormant `last_heartbeat_at` producer; the served `stale` field (ui slice); TRUNCATE-on-size-threshold; the proj_session staleness-scan WHERE-filter at scale (merged into the existing 4.1b-1 recovery-scan park, same table); the §5.1 staleness-candidate semantics (Waiting/Stale states) when a consumer lands.
- **Minor (deferred lows):** the `codex_launch.rs::test_no_codex_spawn_in_slice` grep is non-recursive (module is flat; a future `codex/<subdir>/` must extend the scan — the §28 import-grep precedent).

## How to use what was built

- The background jobs run automatically once the daemon is up (`main.rs::run`).
- The CodexAdapter observe core + launch mechanism are the foundation 3.3c wires into a live drive loop (spawn + interception). 3.3c constructs a `CodexLaunchSpec` from a `bind_codex_profile`'d `CodexExecutionProfile`, applies `harden_codex_dirs` + the umask, and drives the `PreToolUse`→Gateway interception — all the pieces are mechanism-ready.

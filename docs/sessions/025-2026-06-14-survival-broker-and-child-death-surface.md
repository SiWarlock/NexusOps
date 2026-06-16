# Session 025 — the §8 survival arc (restart-recovery + the tmux broker) + the §17 child-death surface

- **Date:** 2026-06-14
- **Phase:** Phase 4 (4.1b-1 · 4.1b-2 · 4.2)
- **Predecessor:** [024-2026-06-13-telemetry-pump-survival-freeze-approval-queue-freeze.md](024-2026-06-13-telemetry-pump-survival-freeze-approval-queue-freeze.md)
- **Successor:** [026-2026-06-14-background-jobs-and-codex-arc-3.3ab.md](026-2026-06-14-background-jobs-and-codex-arc-3.3ab.md)

## Why this session existed
Fresh implementer (the prior hit HARD-STOP at 81%, closed clean at `ea0e93e`/②-mini `657bbd8`). Drove the head of the §8/§8.1 survival design into production + the §17 supervised-child-death surface — three pre-authored briefs (059/060/061), closed at a WARN cycle (clean boundary, nothing in flight).

## What was built

### 4.1b-1 — restart session-recovery orchestration (`475068b` C1 + `1e68f20` C2) — NON-cat-1, security-reviewed
The production caller of 4.1a's `decide_resume` (closed its Step-7.5 reachability gap).
- **New:** `daemon/src/session/broker.rs` (the `Broker` seam + `FakeBroker`), `daemon/src/session/recovery.rs` (`recover_sessions_on_restart`: enumerate non-terminal sessions → `ResumeInputs` → `decide_resume` → dispatch over the seam + emit a System-actor recovery signal; §15 #8 profile preserved), `daemon/src/runtime/recovery.rs` (the production reader + `WriteActorRecoverySink` + `run_restart_recovery` + `NoSurvivorBroker`/`DeferredRecoveryDispatch`), `daemon/tests/recovery_restart.rs` + `recovery_restart_wiring.rs`.
- **Shared:** `SessionRecovered{mode, replayed_event_count, execution_profile_id}` observation event — **CONTRACT 0.30.0→0.31.0**.
- **Q1 ruled (a) System-actor recovery** (lead/user) — my `bootstrap.rs` evidence (cold-start Device/LocalRunner registration is a System-actor event, INV-SEC-1 governs proposer intents not daemon lifecycle) overturned the brief's (b) default.

### 4.1b-2 — the tmux detachable-terminal broker (`0b9f78a`, ONE commit) — INVARIANT-touching, security-CLEAR
The real §8.1 B2-strict survival mechanism behind the frozen `Broker`/`SessionLauncher` seams.
- **New:** `daemon/src/session/tmux.rs` (session-name mapping · command builders · `CommandRunner` seam + `FakeCommandRunner` · `tmux_probe` · `TmuxBroker` survivor-detection · `new_session_argv` · `TmuxLauncher` · `SurvivalBackend` + `select_survival_backend`), `daemon/tests/tmux_broker.rs`.
- **Modified:** `session/broker.rs` (`NoSurvivorBroker` moved here, pub), `session/launcher.rs` (`ROWS`/`COLS`/`terminal_id_for` → pub(crate)), `runtime/recovery.rs` (`run_restart_recovery` +`&dyn Broker`), `main.rs` (probe + select wiring), `tests/session_executor.rs` (co-residency guard marker tracked the launcher-construction refactor).
- **Q1 ruled (b) `env`-wrapper** (lead/user) — I caught that tmux's `-e` can only SET, never UNSET, so it structurally can't strip `ANTHROPIC_API_KEY` (§15 #8); the fix wraps the agent in `tmux new-session … -- env -u ANTHROPIC_API_KEY NEXUSOPS_SESSION_ID=… claude …` (strips at exec time regardless of tmux's server env). NO CONTRACT bump.

### 4.2 — the §17 SessionFailed child-death surface (`463abc0` C1 + `98488f2` C2) — cat-1-boundary pin, security-CLEAR
The §17 cascade HEAD (the 3 cascade arms deferred to their producers).
- **New:** `daemon/src/runtime/session_death.rs` (`WriteActorSessionDeathSink`, deferred-bind), `daemon/tests/session_failed.rs`.
- **Modified:** `session/mod.rs` (`SessionDeathSink` trait + the pure `handle_reaped` router + `NullSessionDeathSink` + `spawn_supervisor_task` +param), `runtime/mod.rs`, `projections/session.rs` (fold `SessionFailed` → status=failed), `main.rs` (deferred-sink wiring), `tests/{session_executor,session_prompt}.rs` (caller fixups).
- **Shared:** `SessionFailed {}` empty-payload observation event — **CONTRACT 0.31.0→0.32.0**.
- **Q1 ruled (a) empty-payload** (orchestrator) — I recommended deferring the `reason` field: at the reap everything is indistinguishably `Session::Failed`, so a reason would be a constant placeholder (known-shape-vs-defer applied to the payload; the forensic "why" lives in the correlated `TerminalProcessExited`).

## Decisions made
- **4.1b-1 Q1 = (a) System-actor recovery** — identity-preserving (a Gateway `session.create` mints a fresh id), no re-approval on restart, the safety guarantee holding via the audited observation event + preserved profile; profile sourced from the COMMITTED `SessionStarted` event (the authoritative store).
- **4.1b-2 Q1 = (b) `env`-wrapper** — the §15 #8 strip is a property of the constructed argv (`env_wrapper_args` derived generically from `spec.env_mutations()`, anti-drift). tmux is an OPTIONAL upgrade → graceful-degrade to `PtyLauncher`+`NoSurvivorBroker` (B2-achievable, never `ReattachedLive`). One commit (tmux.rs is one inseparable subsystem; non-hunk-stageable; security-CLEAR as a whole).
- **4.2 Q1 = (a) empty-payload** — `WorktreeMerged`/`ActionStarted {}` precedent; reason deferred. cat-1 boundary preserved via the injected `SessionDeathSink` trait.

## Decisions explicitly NOT made (deferred → Carry-forward)
- **4.1b-1:** the live recovery-dispatch re-materialization (`DeferredRecoveryDispatch` stays no-op); the resume-affordance population (`supports_resume`/handle/scrollback = false). The latent `proj_session.execution_profile_id`-never-folded projector gap (flagged; I source from the event instead).
- **4.1b-2:** LIVE broker survival (agent outliving the daemon + lossy VT-reattach) = 0.1/0.3-HITL; orphan tmux-session reaping; Option C (zero-dep holder); the `select` Tmux-arm runner-factory; the `cwd.to_string_lossy()` non-UTF8 guard; the settings-write-fail-closed leaf test (`ClaudeLaunchSpec` settings_path not injectable — both launchers).
- **4.2:** the 3 §17 cascade arms (fail-in-flight-action → P5/7 · release-lease → session-leases · Codex distinction → 3.3); the `reason` field (additive-later); the clean-terminal lifecycle events (Killed/Completed → proj_session).

## TDD compliance
**Clean.** Each slice/layer was RED-first (confirmed-RED before GREEN: missing-module/missing-symbol compile-fail for every layer, the contract freezes via the missing type, the behaviors via assertion). One nuance: 4.1b-2 L3 (the `TmuxLauncher`/`select`) tests + impl were authored in the same cycle after the L1+L2 compile-fail RED + the Step-2.5 APPROVED — the tests are RED-capable (they pin exact env-wrapper argv) but were not run against a stub first; not a violation, noted for honesty. No safety-critical TDD skips.

## Cross-doc invariants (single-track — orchestrator hot-writes)
Two new EventTypeRegistry events frozen this session — `SessionRecovered` (CONTRACT 0.31.0) + `SessionFailed` (CONTRACT 0.32.0) — both flagged at Step 9; the orchestrator hot-writes the §7.1/Appendix-A/§17 AS-BUILT rows (present/landing in the working tree's `ARCHITECTURE.md` + `daemon/CLAUDE.md`, committed at its round seals). 4.1b-2 was daemon-internal (NO CONTRACT bump). No drift — every field change was Step-9-flagged + acked.

## Reachability (Step 7.5, carried)
- 4.1b-1: `decide_resume` reachable from `main.rs → run_restart_recovery → recover_sessions_on_restart → decide_resume` (closed its 4.1a gap).
- 4.1b-2: `main.rs:tmux_probe → select_survival_backend → {TmuxLauncher→SessionExecutor, TmuxBroker→run_restart_recovery}` (tmux present → `ReattachedLive` reachable; absent → degrade).
- 4.2: `main.rs deferred sink → spawn_supervisor_task → reap loop → handle_reaped → (Failed) emit_failed → SessionFailed via write-actor → proj_session status=failed`.
No tested-but-unwired gaps. The labelled LIVE survival (broker reattach) + the LIVE child-death (SIGKILL/orphan) integration checks are the §8.1/§17 HITL follow-ons (deterministic surface built; live property HITL), NOT unreachable code.

## Open follow-ups
- **Carry-forward (orchestrator-routed):** all the deferred items above (3 §17 arms · the deferred `reason` · clean-terminal lifecycle events · the live recovery-dispatch drive · LIVE broker/child-death HITL · orphan-reaping · Option C · the settings-path-injection hardening [both launchers] · the runner-factory · the cwd-non-UTF8 guard · the resume-affordance population · the proj_session.execution_profile_id projector fold).
- **LESSON candidates (orchestrator-written):** the injected-sink-over-the-cat-1-seam observation pattern generalized from restart-recovery → in-life child-death (§40); the detachable-terminal-broker-wraps-the-unchanged-spec + probe + consistent-selection + graceful-degrade pattern.
- **Cross-track:** the ui regenerates from 0.31.0 (`SessionRecovered`) + 0.32.0 (`SessionFailed`) + the survival `ResumeMode`/`RecoveryState`.

## How to use what was built
On daemon restart, `main.rs` probes tmux → selects a consistent survival backend → recovers live-at-shutdown sessions (`decide_resume` per session, profile preserved) → emits audited recovery signals. In-life, a supervised child death → `SessionFailed` → `proj_session.status=Failed` → the §11.4 "restart session" affordance. The LIVE survival/reattach is the HITL verify-only follow-on.

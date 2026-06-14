# /tdd brief — detachable_terminal_broker_tmux

## Feature
The real **tmux-backed detachable-terminal broker** — the surviving-PTY holder that realizes the §8.1
B2-strict survival design. It drops into the FROZEN `Broker`/`SessionLauncher` seams (from 4.1b-1 / 4.0a):
new sessions launch **into a detached tmux session** (so the agent can outlive the daemon), and on restart
the broker **detects survivors** via `tmux list-sessions` so `decide_resume` can reach `ReattachedLive`.
**tmux is an OPTIONAL upgrade, not a hard dependency** — absent → graceful-degrade to the non-surviving
`PtyLauncher` (= B2-achievable: resume/replay, never `reattached_live`). The deterministic surface
(command-construction · availability-probe · `list-sessions` parsing · backend-selection · the wiring) is
**test-first**; the LIVE survival (agent actually outliving the daemon + the lossy VT-reattach) is the
labelled **0.1/0.3-HITL verify-only follow-on**, NOT in this slice's deterministic tests.

## Use case + traceability
- **Task ID:** P4.1b-2
- **Architecture sections it implements:** `ARCHITECTURE.md §8` (Recovery: daemon restart), **§8.1** (the
  B2-strict survival EXTENSION — the broker realizes the AS-BUILT design), **§9.1** (harness adapter / the
  launch path — `TmuxLauncher` preserves the O-13 #10 enforcement surface + the Terminal Channel display
  PTY is now a `tmux attach` child), **§5.1** (Session status / `ResumeMode`), **§17** (the "restart
  session" affordance on `Relaunched`), **§15** (#8 ExecutionProfile binding preserved through tmux).
- **USER-RULED (2026-06-13, via lead):** mechanism = **A, tmux** (over abduco/dtach and the custom
  zero-dep holder C). The choice + the six pins below are the binding contract for this brief.
- **Related context:** `daemon/src/session/broker.rs` (the `Broker` seam + `FakeBroker`),
  `daemon/src/session/launcher.rs` (the `SessionLauncher` seam + `PtyLauncher` w/ the `TODO(4.1)`
  swap-point), `daemon/src/session/recovery.rs` (the deterministic orchestration), `daemon/src/runtime/
  recovery.rs` (the production driver + `NoSurvivorBroker`/`DeferredRecoveryDispatch`), brief 059 (4.1b-1),
  LESSONS 28/35/36/38, `docs/planning/P4-deep-dive-live-drive-loop-and-survival.md §7.1/§8.1`,
  `docs/planning/RISKS.md` (the lossy alt-screen VT-reattach = the hardest correctness problem → HITL).

## The six binding pins (USER-ruled — restate at Step 0)
1. **Drop-in behind the FROZEN `Broker`/`SessionLauncher` seams** — daemon-internal, **NO `shared/`
   contract change, NO CONTRACT bump.**
2. **Deterministic surface = test-first:** tmux command-construction + the availability-probe +
   `list-sessions -F` structured-output parsing + the backend-selection + the recovery-dispatch wiring.
3. **Graceful degrade is REQUIRED + test-first:** tmux absent → fall back to the non-surviving
   `PtyLauncher` = B2-achievable (resume/replay, **NEVER `ReattachedLive`**). tmux is an OPTIONAL upgrade;
   the app must work (degraded survival) without it. Pin the probe + the degrade path explicitly.
4. **LIVE survival** (agent outliving the daemon + the lossy VT-reattach + the live recovery
   re-materialization drive) = the labelled **0.1/0.3-HITL verify-only follow-on** — NOT in this slice's
   deterministic tests.
5. **Q1=(a) System-actor recovery UNCHANGED** — the recovery-spawn safety (4.1b-1) is settled; this slice
   does not touch it.
6. **Document the custom zero-dep holder (Option C) as a DEFERRED post-MVP hardening** (orchestrator
   routes it to Carry-forward at Step 9 — the "right" long-term zero-dep answer, deferred, not dropped).

## Acceptance criteria (what "done" means)
**Deterministic (this slice):**
- [ ] **Session-name mapping** — `tmux_session_name(&SessionId) -> String` = `nexusops-<sess_…>`
  (tmux-safe: ULID is alphanumeric, no `:`/`.`) and its inverse parse round-trips; a non-`nexusops-` name
  parses to `None` (foreign tmux sessions ignored).
- [ ] **Command builders** (pure argv, no I/O): `new-session -d -s NAME -c CWD [-e K=V]… -- PROGRAM ARGS…`
  · `list-sessions -F '#{session_name}'` · `attach -t NAME` · `kill-session -t NAME` — each asserted by an
  exact-argv test.
- [ ] **`CommandRunner` seam** — a trait `run(program, args) -> io::Result<Output{status, stdout}>` with a
  production `std::process::Command` impl + a `test-support` fake returning canned `Output` (the
  injectable-seam pattern — `PtySpawner`/`Clock`/`UsageSource` precedent).
- [ ] **Probe** — `tmux_probe(&dyn CommandRunner) -> bool`: `tmux -V` exit-0 → available; ENOENT / non-zero
  → unavailable (deterministic via the fake runner). No panic on a missing binary.
- [ ] **`TmuxBroker` (`Broker` impl)** — `reattach_outcome(session_id)` runs `list-sessions`, parses the
  survivor set, reports `has_live_session = set.contains(session_id)`. "no server running" (non-zero exit /
  empty) → `has_live_session = false` (never errors the recovery path). Deterministic via the fake runner.
- [ ] **Backend selection (pure + consistency-pinned)** — `select_survival_backend(tmux_available) ->
  SurvivalBackend` yields a **consistent pair** (tmux → `TmuxLauncher` + `TmuxBroker`; else → `PtyLauncher`
  + `NoSurvivorBroker`). A test pins **you can never get a `TmuxLauncher` with a `NoSurvivorBroker`** (or
  vice-versa) — launcher and broker always agree on the mechanism.
- [ ] **`TmuxLauncher` (`SessionLauncher` impl)** — `launch_session()` wraps the SAME `ClaudeLaunchSpec`
  (`build`/`program`/`args`/`env_mutations`/`write_settings`) inside `tmux new-session -d -s NAME -- …`,
  then opens the daemon's display PTY via `PtySpawner.spawn("tmux", attach-argv, …)` → a `TerminalSession`;
  constructs the `ClaudeAdapter` (status from hook signals, not PTY); accepts `with_telemetry_sink_factory`
  (same builder as `PtyLauncher`). **Asserted on the COMMAND it constructs + the fail-closed settings
  write** — the live spawn is HITL.
- [ ] **INV-SEC-1 / §15 #8 preserved through the tmux layer** (the safety invariant is deterministically
  pinned): the settings-write **fails closed** (a settings-write `Err` → no session, never a hook-less
  agent — same as `PtyLauncher`); the generated 0600 settings + the `PreToolUse` hook are written
  identically; `env_mutations` (strip `ANTHROPIC_API_KEY` §15 #8 + carry `NEXUSOPS_SESSION_ID` hook
  correlation) are threaded into the tmux `new-session` env so they reach the agent inside tmux.
- [ ] **Wiring** — `main.rs` probes once → builds the selected `SurvivalBackend`; its launcher is consumed
  into `SessionExecutor` (`main.rs:135→144`), its broker into `run_restart_recovery` (`main.rs:202`).
- [ ] All unit tests in `daemon/tests/tmux_broker.rs` (+ `daemon/src/session/tmux.rs` `#[cfg(test)]`) pass.
- [ ] `/wired` confirms the selected `TmuxLauncher`/`TmuxBroker` are reachable from `main.rs` in production
  when tmux is present (and `PtyLauncher`/`NoSurvivorBroker` when absent).
- [ ] `/preflight` clean. **No CONTRACT bump** (pin #1).

**Explicitly NOT in this slice (→ Step-9 Carry-forward / HITL):**
- The LIVE recovery DISPATCH re-materialization (identity-preserving relaunch + the live reattach drive on
  restart) — `DeferredRecoveryDispatch` stays a no-op; the audited `SessionRecovered` signal remains the
  recovery effect (4.1b-1 C2). The real drive = HITL (pin #4) + the launcher-already-consumed wrinkle (see
  Step-2.5 Q3).
- The resume-affordance population (`supports_resume`/`has_resume_handle`/`has_scrollback` stay `false` in
  `enumerate_recoverable_sessions`) — a further follow-on (native `--resume` handle persistence).
- Option C (the custom zero-dep holder) — Carry-forward (pin #6).

## Wiring / entry point (Step 7.5)
Two production swap-points, both already seams (no signature churn beyond `run_restart_recovery`'s broker
arg):
1. **`main.rs:135`** — the launcher construction. Today: `PtyLauncher::new(…).with_telemetry_sink_factory`.
   New: probe → build `SurvivalBackend`; the backend's `Box<dyn SessionLauncher>` (Tmux or Pty) is consumed
   into `SessionExecutor::new(Box::new(launcher), …)` at `main.rs:144` (SessionExecutor unchanged — it
   already takes `Box<dyn SessionLauncher>`).
2. **`main.rs:202` / `run_restart_recovery`** — pass the backend's `Box<dyn Broker>` (Tmux or NoSurvivor)
   into recovery (replace the hardcoded internal `NoSurvivorBroker`). `recover_sessions_on_restart` already
   takes `&dyn Broker` — drop-in. Dispatch stays `DeferredRecoveryDispatch` (pin #4).

## Files expected to touch
**New:**
- `daemon/src/session/tmux.rs` — the tmux subsystem: session-name mapping · command builders ·
  `CommandRunner` trait + prod `SystemCommandRunner` + `test-support` `FakeCommandRunner` · `tmux_probe` ·
  `TmuxBroker` (`Broker`) · `TmuxLauncher` (`SessionLauncher`) · `select_survival_backend` + `SurvivalBackend`.
- `daemon/tests/tmux_broker.rs` — the integration tests (probe / list-parse / reattach_outcome /
  selection-consistency / launcher command-construction + fail-closed settings).

**Modified:**
- `daemon/src/session/mod.rs` — `pub mod tmux;` + re-exports (`TmuxLauncher`/`TmuxBroker`/
  `select_survival_backend`/`SurvivalBackend`).
- `daemon/src/runtime/recovery.rs` — `run_restart_recovery` accepts the selected `&dyn Broker` (param);
  `NoSurvivorBroker` stays the degrade default.
- `daemon/src/main.rs` — the probe + `select_survival_backend` + the consistent launcher/broker threading.

If implementation needs files beyond this list (e.g. a new `EnvMutation` accessor on `ClaudeLaunchSpec`),
**flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/src/session/tmux.rs` (`#[cfg(test)]` for the pure units) + `daemon/tests/tmux_broker.rs`:

1. **`test_session_name_roundtrips`** — `tmux_session_name`/inverse round-trip; foreign name → `None`.
   - Why: §8.1 reattach keys survivors to the ORIGINAL `SessionId` (identity-preserving recovery, Q1=(a)).
2. **`test_new_session_argv`** — exact argv for `new-session -d -s NAME -c CWD -e K=V -- PROGRAM ARGS`.
   - Why: §9.1 — the survivor is launched detached (`-d`); `--` ends options so the spec's args aren't
     mis-parsed.
3. **`test_list_sessions_argv` / `test_attach_argv` / `test_kill_session_argv`** — exact argv each.
4. **`test_probe_present_absent`** — `tmux -V` exit-0 → true; ENOENT/non-zero → false (fake runner).
   - Why: pin #3 — the probe drives graceful-degrade; a missing binary must never panic.
5. **`test_parse_live_sessions`** — `list-sessions -F` stdout → the `nexusops-*` survivor `SessionId` set;
   foreign sessions filtered; empty/"no server" stdout → empty set.
   - Why: §8.1 — only OUR detached sessions are survivors.
6. **`test_tmux_broker_reattach_outcome`** — seeded survivor → `has_live_session=true`; absent → `false`;
   runner error / non-zero exit → `false` (never errors recovery).
   - Why: §8.1 ladder input; condition — the broker must fail-safe toward "no survivor".
7. **`test_select_backend_consistency`** — `tmux_available=true` → Tmux launcher **and** Tmux broker;
   `false` → Pty launcher **and** NoSurvivor broker. Assert you can NEVER mix a Tmux launcher with a
   NoSurvivor broker (or vice-versa).
   - Why: pin #3 — a mixed pair would claim survivors a non-surviving launcher never creates (false
     `ReattachedLive`).
8. **`test_tmux_launcher_builds_wrapped_spec`** — the launcher constructs `tmux new-session … -- <spec.program>
   <spec.args>` wrapping the UNCHANGED `ClaudeLaunchSpec`; the spawned display child is `tmux attach -t NAME`.
   - Why: §9.1 — the agent is still launched via the O-13 spec (default-mode/no-`-p`/no-bg by construction),
     just inside tmux.
9. **`test_tmux_launcher_fail_closed_settings`** — a settings-write `Err` → `launch_session` returns `Err`,
   NO tmux session created, NO display spawn.
   - Why: **INV-SEC-1 / LESSON 25/30** — never a hook-less live agent; the safety invariant is a function
     of the settings write succeeding → fail-closed (NOT soft-degrade).
10. **`test_tmux_launcher_env_hygiene`** — `ANTHROPIC_API_KEY` is NOT propagated into the tmux env and
    `NEXUSOPS_SESSION_ID` IS (the `env_mutations` reach the agent inside tmux).
    - Why: §15 #8 — no silent account-hop through the tmux layer; the hook correlation key must survive.
11. **`test_degrade_no_tmux_never_reattaches`** — with the fallback backend (no tmux), the broker reports
    no survivor for every session → `decide_resume` never yields `ReattachedLive`.
    - Why: pin #3 — graceful-degrade = B2-achievable, never the top rung.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** in `shared/` — daemon-internal subsystem behind frozen seams (pin #1).
  **NO CONTRACT bump.**
- **Shared-contract seam model touched?** **No** — no Appendix-A model, no schema-snapshot test.
- **Orchestrator doc rows to write hot (Step 9 routing):** a §8.1 **AS-BUILT note** (the tmux broker is LIVE;
  the deterministic surface landed; LIVE survival = HITL) + a §9.1 note (the `TmuxLauncher` preserves the
  O-13 #10 surface through the tmux wrapper) + the `daemon/CLAUDE.md` module-org `session/tmux.rs` row + a
  LESSON candidate (see below). Plus the **Option-C deferral → Carry-forward** (pin #6). All orchestrator-written.

## Things to flag at Step 2.5
1. **Env-hygiene through the tmux wrapper (§15 #8).** tmux `new-session` copies a configurable env set;
   `ANTHROPIC_API_KEY` must NOT reach the agent inside tmux and `NEXUSOPS_SESSION_ID` must. Options: (a) pass
   the spec's `env_mutations` via repeated `-e K=V` on `new-session` (tmux ≥3.2) + ensure no leak path; (b)
   set the env on the `CommandRunner` invocation itself. My default vote: **(a) explicit `-e` per mutation +
   a test that the stripped key is absent** — most direct, deterministically testable, no reliance on tmux's
   global `update-environment`. **If the implementer finds tmux structurally cannot prevent the leak, that
   IS a genuinely-new trust-boundary behavior → STOP and route to the orchestrator (→ lead) before GREEN.**
2. **Probe cadence — once at startup, or per-recovery?** Default: **once at `main.rs` startup** (the backend
   is fixed for the daemon's life; tmux won't appear/vanish mid-run). My default vote: **once at startup**
   (simpler, the `SurvivalBackend` is built once). Re-probe only if a Step-9 reason emerges.
3. **Is the LIVE recovery-dispatch drive in THIS slice or deferred?** The production launcher is consumed
   into `SessionExecutor` (Gateway path) before recovery runs (`main.rs:144` vs `:202`), so the recovery
   dispatch can't hold that same instance; an identity-preserving recovery relaunch is also a NEW path
   (≠ `session.create`, which mints a fresh id — Q1=(b) rejected). My default vote: **DEFER the live drive
   (HITL, pin #4)** — keep `DeferredRecoveryDispatch` a no-op; this slice makes the BROKER (detection) +
   the `TmuxLauncher` (forward survivability for NEW sessions) real, so `ReattachedLive` is correctly
   *computed* + *signalled*, and the live re-attach drive is the HITL follow-on. Confirm this boundary.
4. **Orphan-reaping on restart.** Detached tmux sessions for genuinely-terminal/un-recoverable sessions could
   accumulate. Default: **out of scope this slice** (kill-session command builder exists; reaping wires with
   the live recovery-dispatch drive, HITL). My default vote: **defer to the HITL drive** (note it in Step-9
   Carry-forward so "no orphaned PTY" stays tracked). Flag if you disagree.
5. **`SurvivalBackend` shape — owns boxed seams, or an enum the caller matches?** Default: a struct holding
   `Box<dyn SessionLauncher>` + `Box<dyn Broker>` (consistent-by-construction). My default vote: **the struct**
   (the consistency pin #7 is structural). The `with_telemetry_sink_factory` must apply to whichever launcher
   is selected — thread the factory before/into the selection.

## Dependencies + sequencing
- **Depends on:** 4.1b-1 (✅ the `Broker`/`RecoveryDispatch`/`RecoverySignalSink` seams + `recover_sessions_on_restart`
  + `run_restart_recovery`), 4.0a (✅ the `SessionLauncher` seam + `PtyLauncher` + `TODO(4.1)`), 4.0b-2 (✅ the
  live `ClaudeLaunchSpec` + the #10 surface), 3.2 (✅ Claude resume capability), 1.6 (✅ cold-start).
- **Blocks:** the 0.1/0.3-HITL live-survival verification follow-on; the live recovery-dispatch drive; the
  resume-affordance population follow-on.

## Estimated commit count
**2–3.** **L3 (the `TmuxLauncher` + `main.rs` wiring) is INVARIANT-touching → its OWN commit + the
`security-reviewer` pass** (it modifies the live agent launch path = the O-13 #10 / INV-SEC-1 enforcement
surface; a broken env/settings propagation through tmux could create a hook-less agent). L1 (the tmux
primitives: mapping + command builders + `CommandRunner` + probe) and L2 (the `TmuxBroker` + selection) are
non-safety deterministic and MAY bundle if small. Suggested: **L1+L2 (non-safety) → one commit · L3
(invariant) → its own commit.** Do NOT bundle L3 with L1/L2.

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`: YES** — this slice touches INV-SEC-1 / §15 #8 (the launch/interception enforcement
  surface through the new tmux layer; the `invariant` policy fires). Focus: the env-hygiene-through-tmux
  (no `ANTHROPIC_API_KEY` leak), the fail-closed settings write through the wrapper, the
  selection-consistency (no false-survivor pair), the probe never opening an un-degraded path. **Not a new
  architectural fork** (the lead pre-classified 4.1b-2 as the §8.1 realization) — but invariant-PRESERVATION
  through the new layer is the security-reviewer's job. If the review surfaces that tmux structurally breaks
  an invariant, that escalates as a Step-9 Finding.
- **`code-quality-reviewer`: YES** (every-slice policy).

## Lessons-logged candidates anticipated
- **Convention candidate** — "A detachable-terminal broker wraps the UNCHANGED launch spec inside the
  multiplexer (`tmux new-session -- <spec>`) so the O-13 #10 / §15 #8 / fail-closed-settings invariants are
  preserved through the new layer; the broker is an OPTIONAL upgrade behind a pure probe + consistent
  backend-selection (launcher ⇔ broker always agree), graceful-degrading to the non-surviving launcher;
  the survival itself (lossy VT-reattach) is HITL, but the SAFETY invariant is a property of the command/
  settings the launcher constructs → deterministically pinned."
- **Future TODO — operational** — orphan tmux-session reaping on restart; the live recovery-dispatch drive;
  the resume-affordance population; **Option C (custom zero-dep holder) as a post-MVP hardening** (pin #6).
- **Architecture-doc note candidate** — §8.1 AS-BUILT (tmux broker LIVE, deterministic surface; LIVE survival
  = HITL) + §9.1 (the #10 surface preserved through tmux).

## How to invoke
1. **Read this brief end-to-end** — especially the six pins + "Things to flag at Step 2.5".
2. **Run `/tdd detachable_terminal_broker_tmux`** in the implementer session.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line + the six pins.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch".
5. **Step 2.5 (test-design review)** — send the Asserts/coverage write-up + answers to the 5 design
   questions (Q1 env-hygiene + Q3 dispatch-boundary are the load-bearing ones). Don't go GREEN until APPROVED.
6. **Step 8** — dispatch `security-reviewer` (invariant) + `code-quality-reviewer`.
7. **Step 9 (summarize)** — surface anything beyond the anticipated lessons-logged candidates; flag the
   §8.1/§9.1 AS-BUILT + the Option-C deferral for orchestrator hot-routing.

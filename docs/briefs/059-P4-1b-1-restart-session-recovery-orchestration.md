# /tdd brief — restart_session_recovery_orchestration

## Feature
The first half of 4.1b (the §8.1 B2-strict survival): the **deterministic daemon-restart session-recovery orchestration** — on restart, enumerate the sessions that were live at shutdown, and per session populate `ResumeInputs` → `decide_resume` (4.1a) → dispatch the chosen `ResumeMode` strategy through the launcher/**broker SEAM** → emit the resumed-vs-replayed-vs-reattached recovery signal (+ the "restart session" affordance for `relaunched`/recovery-failed). This is the **production caller of `decide_resume`** (closes its Step-7.5 reachability). The **Broker seam** (trait + `FakeBroker`) is introduced here; the **real detachable-terminal broker** `SessionLauncher` impl = **4.1b-2**; LIVE broker survival = the labelled 0.1/0.3-HITL follow-on.

## Use case + traceability
- **Task ID:** P4.1b (sub-slice **4.1b-1** — the restart-recovery orchestration; 4.1b-2 = the real broker subsystem)
- **Architecture sections it implements:** `ARCHITECTURE.md §8`/`§8.1` (the daemon-restart survival ladder — the per-session recovery decision + dispatch), `§17` (the restart recovery / "restart session" affordance; orphaned-`executing` reconcile already built at 2.4), `§5.1` (Session status; lease reclaim), `§9.1` (the `resume()` adapter method the strategy drives), `§16` (the cold-start/restart sequence)
- **Related context:** `daemon/src/bootstrap.rs::cold_start` (the §16 startup — composes pidlock/open/migrate/register/`reconcile_orphans`; returns `DaemonContext`; **pre-runtime** — the recovery orchestration runs in `main.rs` AFTER the supervisor + write-actor are up, NOT inside `cold_start`); `daemon/src/harness/resume.rs` (`decide_resume(&ResumeInputs)` + the `ResumeInputs` 5 fields — 4.1a); `daemon/src/session/{mod.rs,launcher.rs}` (the `SessionSupervisor`/`SupervisorHandle` + the `SessionLauncher` seam with the `TODO(4.1)` broker swap-point; `FakeLauncher`); `daemon/src/gateway/recovery.rs` (`reconcile_orphans` — the 2.4 crash-reconcile, already wired in `cold_start`); the lease reclaim (1.4 `locks/` — reacquire mints a fresh fencing token); LESSONS §9 (write-actor idiom) / §10 (daemon self-recovery = a System-actor event, not a Gateway Action) / §28 (the session spine) / §36 (`decide_resume` is the caller's-loop classifier).
- **Widens phase scope because** this cites cross-cutting sections (§16 cold-start, §5.1 lease, §9.1 adapter) beyond P4's primary anchors — standard for a restart-orchestration slice. The work implements in-scope §8/§8.1/§17.

## Acceptance criteria (what "done" means)
- [ ] A `Broker` **seam** (trait, e.g. `daemon/src/session/broker.rs`) — `reattach_outcome(session_id) -> BrokerReattach` (does a surviving live in-flight PTY exist for this session?) — with a deterministic `FakeBroker` (test-support-gated). The real detachable-terminal impl is 4.1b-2; this slice introduces the seam + the fake.
- [ ] A pure/deterministic **recovery orchestration** (e.g. `session::recovery::recover_sessions_on_restart` or `bootstrap`-adjacent): enumerate the sessions that were live at shutdown (the non-terminal `proj_session` rows, read-only) → per session build `ResumeInputs` (broker reattach outcome + the harness `supports_resume` capability + resume-handle presence + scrollback presence/count) → `decide_resume` → return the per-session `(SessionId, ResumeResult, RecoveryAction)` plan.
- [ ] The chosen `ResumeMode` is dispatched to the right effect over the SEAM (deterministic, no live process): `ReattachedLive` → reattach via the broker; `Resumed` → relaunch + `resume()` (harness `--resume`/`thread/resume`); `Replayed` → relaunch + scrollback replay; `Relaunched` → fresh relaunch + the **"restart session" affordance** (a §17 recovery signal). **The §15 #8 ExecutionProfile binding is PRESERVED on every recovered session** (the recovered session keeps its original profile — no silent account-hop; the no-silent-profile-change pin).
- [ ] The daemon emits a **recovery signal per session** (the resumed-vs-replayed-vs-reattached bit the §11.4 UI renders) — a **non-mutation observation event** via the write-actor (the `SessionStarted`/`TerminalProcessExited` System/adapter-actor precedent, LESSONS §10/§23), NOT a new Gateway mutation path. (Whether a recovery-RELAUNCH spawn is a System-actor recovery event vs a Gateway `session.create` action = Step-2.5 Q1, the trust-boundary design point.)
- [ ] All tests in `daemon/tests/recovery_restart.rs` (NEW) pass — over `FakeBroker` + `FakeLauncher` + the injectable `Clock`/`IdGen` (deterministic; no live process — the live survival is the HITL follow-on).
- [ ] `/preflight` clean; cross-doc invariant updated atomic (if any new event type / `shared/` surface — likely a recovery signal event → flag at Step-9).

## Wiring / entry point (Step 7.5)
The recovery orchestration is called from **`main.rs`**, after `cold_start` returns + the runtime (write-actor) + the `SessionSupervisor` are spawned (1.6c/4.0a) and BEFORE the accept-loop serves clients — the production restart path. It **closes `decide_resume`'s Step-7.5 reachability** (4.1a built the classifier "wired, not driven"; this is its production driver — `/wired decide_resume` should now reach `main.rs` → `recover_sessions_on_restart`). The Broker seam's real impl (the surviving-PTY mechanism) lands 4.1b-2; here the seam is wired with `FakeBroker` (production gets the real broker at 4.1b-2 — note the seam is reachable, the real survival is 4.1b-2/HITL).

## Files expected to touch
**New:**
- `daemon/src/session/broker.rs` — the `Broker` seam (trait) + `FakeBroker` (test-support) + `BrokerReattach`.
- `daemon/src/session/recovery.rs` (or `bootstrap`-adjacent) — `recover_sessions_on_restart` + the per-session recovery plan/dispatch.
- `daemon/tests/recovery_restart.rs` — the deterministic recovery tests.

**Modified:**
- `daemon/src/main.rs` — call the recovery orchestration post-supervisor (the production entry).
- `daemon/src/session/mod.rs` — expose the seam + the recovery entry; the `SupervisorHandle` recovery dispatch.
- (possibly) `shared/src/events.rs` — a recovery-signal event type (if not reusing an existing one) → flag at Step-9 (CONTRACT bump).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/recovery_restart.rs` (FakeBroker/FakeLauncher/injectable Clock+IdGen — deterministic):
1. **`test_reattach_live_when_broker_holds_session`** — Asserts: a live-at-shutdown session + `FakeBroker` reports a surviving PTY → the recovery plan = `ReattachedLive` + a reattach dispatch. Why: §8.1 B2-strict reattach (top of the ladder).
2. **`test_resume_when_supported_no_survivor`** — Asserts: no broker survivor + `supports_resume` + a resume handle → `Resumed` + a relaunch+resume dispatch. Why: §8 `--resume`/`thread/resume`.
3. **`test_replay_when_scrollback_only`** — Asserts: no resume + scrollback → `Replayed` + a relaunch+replay dispatch. Why: §8 scrollback replay.
4. **`test_relaunch_and_restart_affordance`** — Asserts: nothing available → `Relaunched` + the "restart session" §17 recovery signal. Why: §8/§17 affordance tail.
5. **`test_profile_preserved_on_recovery`** — Asserts: a recovered session keeps its original §15 #8 `ExecutionProfile` (no silent account-hop). Why: §15 #8 safety pin (the no-silent-profile-change invariant).
6. **`test_only_live_at_shutdown_sessions_recovered`** — Asserts: terminal/completed sessions are NOT recovered (only non-terminal `proj_session` rows). Why: §5.1 — recover the live set, not history.
7. **`test_recovery_signal_is_observation_not_gateway`** — Asserts: the recovery signal is a write-actor observation event, no Gateway `Action*` mutation (the recovery orchestration proposes/recovers, never a 2nd mutator). Why: INV-SEC-1 / LESSONS §10/§23.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** possibly a new recovery-signal event type (`shared/`) → CONTRACT bump (flag at Step-9; if reusing an existing event, none). The `Broker` seam + recovery orchestration are daemon-internal.
- **Orchestrator doc rows to write hot:** §8.1 AS-BUILT (the restart-recovery orchestration is live; `decide_resume` now driven) + §17 (the restart recovery path) + the EventTypeRegistry/Appendix-A row if a recovery event is added + the `daemon/CLAUDE.md` rows + LESSON candidate (the restart-recovery-over-the-seam pattern).
- **Safety:** **NON-cat-1 IF** recovery composes the EXISTING audited mechanisms (the recovery signal is an observation event; a recovery-relaunch reuses the audited session.create/launcher path; the profile is preserved). **security-reviewer RUNS** (§15 #8 profile-preservation + INV-SEC-1 — the recovery path must not become a 2nd un-audited mutator). **If Step-2.5 Q1 (the recovery-relaunch audit mechanism) reads as a genuinely-NEW trust-boundary behavior** (a recovery spawn that bypasses the Gateway), **escalate to the lead BEFORE sign-off** (the lead's instruction).

## Things to flag at Step 2.5
1. **(TRUST-BOUNDARY — surface to the lead) Recovery-relaunch audit mechanism.** When recovery RELAUNCHES a session (Resumed/Replayed/Relaunched all spawn a fresh process), is that spawn (a) a **System-actor recovery event** (the daemon restoring its OWN prior session — the LESSONS §10 self-recovery precedent: audited observation, not policy-gated, like Device/LocalRunner registration), or (b) a **Gateway `session.create`/`session.recover` action** (risk-0 audited-auto-allow, the 4.0b-1 precedent)? My default vote: **(b) route through the audited session-executor path** (recovery is still a process spawn → keep it on the audited mechanism; INV-SEC-1 — never a direct supervisor re-spawn that bypasses the Gateway; the §15 #8 profile is bound from the original `SessionStarted`). **This is the trust-boundary point the lead asked me to route — I'll surface the resolution to the lead before Step-2.5 sign-off.** If (a) is preferred (self-recovery = System-event), confirm the profile-preservation + the no-new-mutator pins hold.
2. **`ResumeInputs` population — where do the harness capability + resume-handle + scrollback come from at restart?** Default: `supports_resume` from the harness `HarnessCapabilities` (per the session's harness kind, recorded at `SessionStarted`); `has_resume_handle`/`has_scrollback` from the persisted transcript/session state (the `TranscriptRef`/event log). For 4.1b-1 (FakeBroker/FakeLauncher), these are injected fixture inputs; the real wiring rides 4.1b-2 / the live adapters. Flag any input with no deterministic source yet.
3. **Live-at-shutdown enumeration source.** Default: the non-terminal `proj_session` rows (read-only WAL). Confirm vs deriving from the event log (proj_session is the rebuilt read model; it's authoritative post-rebuild).
4. **The recovery signal — new event type or reuse?** Default: a new `SessionResumed`/`SessionRecovered{mode}` observation event (the §11.4 resumed-vs-replayed bit) → CONTRACT bump. Confirm vs reusing `SessionStarted` with a recovery flag.
5. **`Relaunched`'s "restart session" affordance.** Default: a §17 recovery signal the §11.4 UI renders as the "Restart session" affordance (the `RecoveryState=recovery_failed`/`relaunched` surface) — an observation event, not an auto-action.

## Dependencies + sequencing
- **Depends on:** 4.1a (`decide_resume`/`ResumeInputs`/`ResumeResult` ✅), 4.0a (the supervisor/launcher seam ✅), 1.4 (lease reclaim ✅), 1.2 (projection rebuild ✅), 2.4 (`reconcile_orphans` ✅), 1.6a (cold_start ✅).
- **Blocks:** **4.1b-2** (the real detachable-terminal broker swaps into this slice's Broker seam; LIVE survival = HITL). The §11.4 survival-UX (consumes the recovery signal).

## Estimated commit count
**2–3** (implementer's call at Step-2.5): (1) the `Broker` seam + `FakeBroker` + the recovery decision/plan (enumerate → ResumeInputs → decide_resume → plan); (2) the dispatch + the recovery-signal emission + the §15 #8 profile-preservation + the main.rs wiring. A safety pin (profile-preservation / the recovery-relaunch audit mechanism) may warrant its own commit. NOT cat-1 (composes existing audited mechanisms) — but security-reviewer runs (§15 #8 + INV-SEC-1).

## Lessons-logged candidates anticipated
- **Architecture-doc note** — the restart-recovery orchestration drives `decide_resume` in production; the recovery-relaunch audit mechanism (the Step-2.5 Q1 resolution).
- **Convention candidate** — daemon-restart session recovery = enumerate-live → decide_resume → dispatch-over-the-seam, with recovery as a System/audited path (not a 2nd mutator) + profile-preservation; the live survival is the HITL follow-on (the §14 deterministic-logic / non-deterministic-survival split, LESSONS §36 extended).

## How to invoke
1. Read this brief end-to-end (esp. Step-2.5 Q1 — the trust-boundary point).
2. `grep -rn "decide_resume\|cold_start\|SupervisorHandle\|SessionLauncher\|proj_session" daemon/src/` to map the surface, then `/tdd restart_session_recovery_orchestration`.
3. Step 0/1 → confirm Feature + files. Step 2.5 → answer the 5 Qs; **Q1 (recovery-relaunch audit mechanism) surfaces to the orchestrator → lead before sign-off** (the trust-boundary review).
4. Step 8 → **security-reviewer runs** (§15 #8 profile-preservation + INV-SEC-1 no-2nd-mutator); code-quality runs.
5. Step 9 → surface the cross-doc (recovery event / §8.1 AS-BUILT / CONTRACT bump if any).

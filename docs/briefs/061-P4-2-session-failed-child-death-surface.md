# /tdd brief — session_failed_child_death_surface

## Feature
The §17 **supervised-child-death surface** (the cascade HEAD): a new `SessionFailed` System-actor
OBSERVATION event emitted when the supervisor reaps a dead session, folded into `proj_session` as
`status=Failed` so the UI surfaces the "restart session" affordance. The §17 cascade ARMS
(fail-in-flight-action, release-lease, the Codex pipe-drop distinction) are DEFERRED to their
producers (lead-ruled known-shape-vs-defer test) — this slice builds the deterministic head only.

## Use case + traceability
- **Task ID:** P4.2 (narrowed — lead-ruled PROCEED-NARROW 2026-06-13)
- **Architecture sections it implements:** `ARCHITECTURE.md §17` (Failure-mode contract — "Agent / PTY /
  app-server dies (daemon alive) → `SessionFailed`+`TerminalProcessExited` → … → restart-session
  affordance"), **§5.1** (Session status — the `Failed` terminal state), **§7.1** (the EventTypeRegistry —
  the new `SessionFailed` event type).
- **Widens phase scope because** it adds a §7.1 EventTypeRegistry event type (`SessionFailed`) — a P4-native
  contract addition, exactly the `SessionRecovered` (4.1b-1) precedent; additive CONTRACT bump.
- **Related context:** `shared/src/events.rs` (`TerminalProcessExited`/`SessionRecovered` — the
  System-actor observation-event precedent the new event mirrors; the events.rs:268 doc already names this
  "SessionFailed → … cascade is Phase 4 (CONSUMES this event)"), `daemon/src/runtime/recovery.rs` (the
  `WriteActorRecoverySink` driver pattern this mirrors), `daemon/src/session/mod.rs` (the supervisor
  `try_reap`/`reap_next` → `(id, Failed)` seam), `daemon/src/projections/session.rs` (the proj_session
  projector — currently folds ONLY `SessionStarted`), LESSONS §10/§23 (observation-not-Gateway), §17
  (mutable-status-from-event-type rebuild safety), 28 (the cat-1 session boundary), 38 (the
  runtime-driver-over-the-cat-1-seam pattern).

## Scope ruling (lead-ruled 2026-06-13 — restate at Step 0)
- **BUILD (the deterministic cascade HEAD):** `SessionFailed` event + the reap→driver→`proj_session`
  Failed + the restart-session affordance (rendered from `status=Failed`). Claude/PTY side.
- **DEFER (the arms — shapes depend on UNBUILT producers → Carry-forward, NOT built dormant):** (a)
  fail-in-flight-ActionRequest → **Phase 5/7** (agent mutations are adjudication-only — no executing
  target; a session-submitted *executing* action doesn't exist); (b) release-lease → **session-lease
  introduction** (sessions don't acquire leases today); (c) Codex pipe-drop-vs-crash → **3.3** (no Codex).
  The lead's test: build dormant ONLY if the arm's shape is fully §17/contract-defined; DEFER if it
  depends on an unbuilt producer's interface (the Phase-8 lesson — don't guess).

## Acceptance criteria (what "done" means)
- [ ] A NEW `SessionFailed` event payload type in `shared/src/events.rs` + its `EVENT_TYPE` const, a
  System-actor OBSERVATION event (the `TerminalProcessExited`/`SessionRecovered` precedent: written via the
  single write-actor through the §15 redaction gate, **NEVER the Gateway** — a death notification is not a
  state mutation, INV-SEC-1 governs mutations; LESSON §10/§23).
- [ ] **§2.5-seam contract freeze:** `SessionFailed` registered in the `ContractBundle`; schema regen;
  **CONTRACT 0.31.0 → 0.32.0**; a schema-snapshot test (field-name set == checked-in snapshot, tagged
  `spec(§7.1)`) + a `deny_unknown_fields` reject-unknown test; 3-way verify GREEN @ 0.32.0.
- [ ] A runtime/ **death-driver** consumes the supervisor reap (`try_reap`/`reap_next` → `(id, Failed)`)
  and emits `SessionFailed` via the write-actor (System-actor; `actor_type=System`, session identity on
  the envelope; `occurred_at` = daemon-`Clock` UTC-Z; fire-and-forget `try_append_observation` soft-degrade,
  the `WriteActorRecoverySink` precedent).
- [ ] **The cat-1 boundary HOLDS:** the `session/` supervisor emits via an INJECTED sink trait (a
  `SessionDeathSink`-style seam, the `RecoverySignalSink`/`TelemetrySinkFactory` precedent) — `session/`
  holds `Box<dyn _>`, **never a `WriteHandle`**; the import-grep cat-1 guard (LESSON 28) stays green.
- [ ] Only a **`Failed`** reap emits `SessionFailed` — a clean `Killed`/`Completed` reap does NOT (it is
  not a failure; see Step-2.5 Q3).
- [ ] The `proj_session` projector folds `SessionFailed` → `UPDATE proj_session SET status='failed'`
  (+`completed_at`) WHERE the envelope session_id — **mutable status derived from the EVENT TYPE, never a
  registry row's current value** (LESSON §17 rebuild-safety; pin a rebuild-equivalence test).
- [ ] **Wiring (Step 7.5):** `main.rs` constructs the death-driver + the write-actor-backed sink + feeds
  the supervisor's reaps (the `run_restart_recovery`/`spawn_supervisor_task` wiring neighborhood).
- [ ] All unit tests in `daemon/tests/session_failed.rs` (+ `shared/tests/contract.rs` snapshot) pass;
  `/wired` confirms the death-driver is reachable from `main.rs`; `/preflight` clean.

## Wiring / entry point (Step 7.5)
The supervisor (`spawn_supervisor_task`, `main.rs`) already reaps dead actors (`try_reap`/`reap_next`).
This slice routes a `Failed` reap → the injected `SessionDeathSink` → the runtime/ `WriteActor…Sink` →
`SessionFailed` appended → the `proj_session` projector folds it. The driver is wired in `main.rs` near the
supervisor + write-actor construction (the `WriteActorRecoverySink`/`run_restart_recovery` neighborhood).
Confirm the reap→emit path is reachable in production, not just tests.

## Files expected to touch
**New:**
- `daemon/tests/session_failed.rs` — the integration tests (reap→emit; projector fold; cat-1; reject-unknown).

**Modified:**
- `shared/src/events.rs` — `+SessionFailed` payload + `EVENT_TYPE` const.
- `shared/src/lib.rs` / `shared/src/schema.rs` + `shared/contracts/schema/*` — register + regen (CONTRACT 0.32.0).
- `shared/tests/contract.rs` — the `SessionFailed` schema snapshot + reject-unknown test.
- `daemon/src/runtime/` (e.g. `recovery.rs` or a new `session_death.rs`) — the death-driver + the
  write-actor-backed `SessionDeathSink` impl.
- `daemon/src/session/mod.rs` — the supervisor emits a `Failed` reap via the injected sink (cat-1: trait only).
- `daemon/src/projections/session.rs` — fold `SessionFailed` → status=Failed.
- `daemon/src/main.rs` — wire the driver + sink + the reap feed.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`test_session_failed_schema_snapshot`** (`shared/tests/contract.rs`) — the `SessionFailed` field-name
   set == the checked-in snapshot; tagged `spec(§7.1)`. Why: §2.5-seam freeze (LESSON §15 traps).
2. **`test_session_failed_rejects_unknown`** — `deny_unknown_fields` rejects an extra field. Why: §5.0/§15 fail-closed.
3. **`test_failed_reap_emits_session_failed`** — a `(id, Failed)` reap → the injected sink receives one
   `SessionFailed` for that session. Why: §17 child-death → SessionFailed.
4. **`test_clean_reap_does_not_emit`** — a `(id, Killed)` / `(id, Completed)` reap → NO `SessionFailed`
   (not a failure). Why: §17 — only a failure is a failure (Step-2.5 Q3).
5. **`test_driver_emits_via_write_actor`** — the runtime/ driver appends `SessionFailed` as a System-actor
   observation (actor_type=System, session on the envelope, UTC-Z), fire-and-forget. Why: LESSON §10/§23.
6. **`test_session_module_stays_cat1`** — the cat-1 import-grep guard (LESSON 28) still passes: `session/`
   imports no `runtime`/`eventstore`/`gateway`/`WriteHandle` (the sink is a trait). Why: the single-mutator boundary.
7. **`test_projector_folds_session_failed`** — `SessionFailed` → `proj_session.status='failed'`
   (+completed_at). Why: §5.1/§7.2 projection.
8. **`test_session_failed_rebuild_equivalent`** — status derived from the EVENT TYPE (not a row's current
   value) → rebuild-equivalent. Why: LESSON §17 (mutable-from-event-type).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **NEW `SessionFailed` event** (additive). **CONTRACT 0.31.0 → 0.32.0.**
- **§2.5-seam (shared-contract) model touched?** **YES** — `SessionFailed` is a new EventTypeRegistry
  payload. The RED outline includes the schema-snapshot test (tagged `spec(§7.1)`).
- **Orchestrator doc rows to write hot (Step 9 routing):** the `SessionFailed` EventTypeRegistry row
  (`daemon/CLAUDE.md` + `ARCHITECTURE.md §7.1` registry) + the Appendix-A row + a §17 AS-BUILT note (the
  child-death surface LIVE; the cascade arms deferred) + CONTRACT 0.32.0. Orchestrator-written.

## Things to flag at Step 2.5
1. **`SessionFailed` payload shape.** Beyond the envelope's session_id, what does it carry? Options: (a)
   **empty-payload** (the death fact + status=Failed suffices — the `WorktreeMerged` empty-payload
   precedent); (b) a structural **`reason`** (redaction-safe class-name `Option<String>` or a small enum,
   the `GithubSyncFailed.reason` precedent — useful for the §11.4 "why" + distinguishing crash vs PTY-fail).
   My default vote: **(b) a minimal structural `reason`** (a small enum or class-name String) — the supervisor
   reap + the preceding `TerminalProcessExited` give enough to populate a coarse reason; it's cheap + the UI
   benefits. Take (a) if you'd rather not freeze a reason taxonomy yet.
2. **The sink seam name/shape.** A new `SessionDeathSink` trait (`emit_failed(session_id, reason)`) injected
   into the supervisor, mirroring `RecoverySignalSink`. My default vote: **a dedicated `SessionDeathSink`
   trait** (don't overload `RecoverySignalSink` — different event, different lifecycle). Confirm.
3. **Clean-terminal lifecycle events (Killed/Completed) — in scope?** A clean `session.kill` → `Killed`; a
   normal end → `Completed`. Do those also emit lifecycle events (SessionKilled/SessionCompleted) so
   `proj_session` reflects them? My default vote: **OUT of scope** — 4.2 is the FAILURE surface; the
   clean-terminal lifecycle events are a separate concern (flag as a Carry-forward note if proj_session
   should track clean terminals too). This slice emits SessionFailed on `Failed` reaps only.
4. **Reason derivation.** If (b), is the reason derived from the reap's terminal status alone, or correlated
   with the preceding `TerminalProcessExited` (exit_code/signal)? My default vote: **the coarse reap-derived
   reason for MVP** (correlating with TerminalProcessExited is a richer follow-on); keep it simple.

## Dependencies + sequencing
- **Depends on:** 3.2 (✅ Claude children), 3.4 (✅ `TerminalProcessExited` + the observation-event precedent),
  4.0a (✅ the supervisor + the reap seam), 1.2 (✅ the proj_session projector), 4.1b-1 (✅ the
  runtime-driver-over-the-cat-1-seam pattern). **NOT 3.3** (the Codex arm deferred).
- **Blocks:** the deferred cascade arms (Phase 5/7 action-failure, session-lease release, 3.3 Codex
  distinction) — each attaches to this head when its producer lands.

## Estimated commit count
**2.** **C1 (contract-only, non-safety):** the `SessionFailed` §2.5-seam freeze (`shared/`: payload +
EVENT_TYPE + schema snapshot + 3-way verify + CONTRACT 0.32.0). **C2 (the behavior):** the runtime
death-driver + the `SessionDeathSink` + the supervisor emit + the proj_session projector fold + main.rs
wiring. C1 is a clean contract commit; C2 is the daemon behavior. (No cat-1/safety-critical pin — SessionFailed
is an observation event, the cat-1 boundary is PRESERVED via the injected sink. May combine if small.)

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`: YES (invariant)** — focused narrowly on **the cat-1 boundary preservation**: the
  new emit-path adds a sink to the `session/` supervisor; verify `session/` still imports no
  `WriteHandle`/`runtime`/`eventstore`/`gateway` (the sink is an opaque trait), and that `SessionFailed` is
  a write-actor observation (NOT routed through the Gateway). Not a new safety fork — invariant-PRESERVATION.
- **`code-quality-reviewer`: YES** (every-slice).

## Lessons-logged candidates anticipated
- **Convention candidate** — "a supervised-child DEATH is a System-actor observation event (`SessionFailed`,
  the LESSON §10/§23 family) emitted by a runtime/ driver over the cat-1 supervisor's reap seam via an
  injected sink — the 38 runtime-driver-over-the-cat-1-seam pattern, generalized from restart-recovery to
  in-life child-death; the §17 cascade ARMS attach when their producers land (known-shape-vs-defer)."
- **Architecture-doc note candidate** — §17 AS-BUILT (the child-death SessionFailed surface LIVE; the arms deferred).
- **Future TODO** — the deferred arms (Phase 5/7 action-failure, session-lease release, 3.3 Codex distinction);
  the clean-terminal lifecycle events (if proj_session should track Killed/Completed).

## How to invoke
1. **Read this brief end-to-end** — especially the scope ruling + "Things to flag at Step 2.5".
2. **Run `/tdd session_failed_child_death_surface`**.
3. **Step 0 (Restate)** — confirm the narrow scope (the cascade HEAD; arms deferred) + the CONTRACT bump.
4. **Step 2.5** — send the Asserts/coverage write-up + answers to the 4 design questions (Q1 payload shape
   is the main one). Don't go GREEN until APPROVED.
5. **Step 8** — `security-reviewer` (cat-1 boundary) + `code-quality-reviewer`.
6. **Step 9 (summarize)** — surface flags + the §7.1/§17 cross-doc rows for orchestrator hot-routing.

# /tdd brief — background_jobs_wal_checkpoint_and_staleness

## Feature
The deterministic Phase-4 **background jobs**: a periodic **WAL checkpointer** (bounds the SQLite WAL via
the write-actor) and a **session-staleness poller** (derives `stale` from `last_heartbeat_at` age, the live-read recompute) — both following the established `spawn_reaper`/`spawn_drainer` interval pattern
(LESSON 9 / §10). The producer-gated surfaces in the original 4.3 list (the Brain sidecar supervisor; the
integration auth/rate-limit + network-offline failure events) are DEFERRED to their producers.

## Use case + traceability
- **Task ID:** P4.3 (narrowed — the producer-gated scoping test, lead-authorized 2026-06-14)
- **Architecture sections it implements:** `ARCHITECTURE.md §10` (the daemon runtime / long-lived Tokio
  interval tasks — the `spawn_drainer`/`spawn_reaper` family), **§17** (failure-mode contract — the
  staleness signal + the daemon-owned failure surfaces), **§5.1** (Session — the derived `stale` signal).
- **Related context:** `daemon/src/runtime/drainer.rs` (the `spawn_drainer`/`spawn_reaper` interval pattern
  this mirrors — `tokio::time::interval` + `MissedTickBehavior::Delay`, the deterministic `*_once` unit +
  the spawn loop), `daemon/src/eventstore/` (the write-actor + WAL), `proj_session` DDL (has
  `last_heartbeat_at`; NO `stale` column — see Step-2.5 Q2), LESSONS 9 (runtime jobs / bounded commands
  via the write-actor, never a rogue writer), the live-read recompute precedent — worktree-status
  cache), 22 (perf-bench cadence, if a WAL bench is wanted).

## Scope ruling (producer-gated test, lead-authorized 2026-06-14 — restate at Step 0)
- **BUILD (deterministic, daemon-internal shapes):** the **WAL checkpointer** + the **session-staleness
  poller** (the stale-by-age derivation). Both are §10/§17-shape-defined with daemon-internal sources.
- **DEFER (producer-gated → Carry-forward, build-when-the-producer-lands):** (a) the **Brain sidecar
  supervisor** (ping/restart/backoff) — the Brain sidecar is **Phase 8, now DEFERRED** (`last-consumer-slice:
  Phase 8 / Brain start`); (b) the **integration auth/rate-limit + network-offline failure events** — the
  GitHub/Linear executors are **Phase 5/7** (the `*SyncFailed` event types exist in `shared/` but the daemon
  doesn't emit them; the edges/integration executors do) (`last-consumer-slice: Phase 5/7 integration
  executors`). The **daemon-owned §17 failure events already exist + emit** (fail-closed/audit-integrity =
  2.4/1.6c/4.0b-2c · projection-degraded = 1.6c · fencing-conflict = 2.4) — this slice does NOT re-add them.

## Acceptance criteria (what "done" means)
- [ ] A **WAL checkpointer** background job: a `tokio::time::interval` loop (the `spawn_reaper` precedent,
  `MissedTickBehavior::Delay`) that asks the **single write-actor** to run a bounded `wal_checkpoint`
  (PASSIVE/TRUNCATE — Step-2.5 Q3) — **NEVER a rogue writer** (forbidden #3; the checkpoint rides the
  write-actor command path, like `drain_once`/`reap_once`). A deterministic `*_once`-style unit (the
  bounded pass) + the spawn loop (the established split).
- [ ] A **session-staleness poller**: a pure `is_stale(last_heartbeat_at, now, threshold) -> bool` (age >
  threshold; the live-read recompute — **computed live / on rebuild, NOT event-folded/replayed**) + a
  periodic job that recomputes session staleness over `proj_session` (read-only WAL). The derivation is a
  pure, deterministically-testable function (injectable `Clock`).
- [ ] Both jobs are **wired in `main.rs`** alongside `spawn_drainer`/`spawn_reaper` (the runtime
  long-lived-task family); `/wired` confirms they're spawned in production.
- [ ] **No CONTRACT bump** (daemon-internal jobs; no new wire type — UNLESS Q2 adds a `stale` projection
  field, which would be a contract decision → flag at Step 2.5).
- [ ] The §17 acceptance pins: happy (the checkpointer keeps the WAL bounded across N commits); edge
  (heartbeat age > threshold → `stale`; recomputed on rebuild, not replayed); error (the existing
  daemon-owned §17 failure events still emit — a regression-guard, not a re-build).
- [ ] All unit tests in `daemon/tests/jobs.rs` (or `runtime`) pass; `/preflight` clean.

## Wiring / entry point (Step 7.5)
`main.rs` spawns both jobs next to `spawn_drainer`/`spawn_reaper` (the runtime task family, post-write-actor
construction). The WAL checkpointer sends a bounded checkpoint command to the write-actor each interval; the
staleness poller reads `proj_session` (read-only WAL) + recomputes. Confirm both are reachable in production
(not test-only).

## Files expected to touch
**New:**
- `daemon/src/runtime/jobs.rs` (or `daemon/src/jobs/mod.rs`) — the WAL checkpointer + the staleness poller +
  the pure `is_stale` (see Step-2.5 Q1 for the module home).
- `daemon/tests/jobs.rs` — the integration tests.

**Modified:**
- `daemon/src/runtime/mod.rs` (or `lib.rs`) — module decl + re-exports.
- `daemon/src/main.rs` — spawn both jobs.
- possibly `daemon/src/eventstore/` — a `wal_checkpoint` write-actor command (if not already present).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`test_is_stale_by_age`** — `is_stale(heartbeat, now, threshold)`: age>threshold → true; within → false;
   exact-boundary defined; missing heartbeat (`None`) → the chosen default (Step-2.5 Q4). Why: §17 stale-by-age.
2. **`test_staleness_recomputed_not_replayed`** — staleness is derived live (a pure recompute over
   `proj_session`), NOT folded from an event; a `rebuild_projections()` re-derives the same (no replay
   dependence). Why: live-read recompute (the worktree-status precedent).
3. **`test_wal_checkpoint_once_bounds_wal`** — a bounded checkpoint pass via the write-actor reduces/holds
   the WAL (the deterministic `*_once` unit). Why: §10 — bounded WAL.
4. **`test_wal_checkpoint_via_write_actor_only`** — the checkpoint rides the write-actor command path; NO
   second writable connection is opened (forbidden #3 / single-writer). Why: LESSON 9 — never a rogue writer.
5. **`test_jobs_spawn_loop`** — the spawn loop ticks on the injected interval (`MissedTickBehavior::Delay`),
   sends bounded commands, never back-pressures the writer (the `spawn_drainer`/`spawn_reaper` precedent).
6. **`test_daemon_owned_failure_events_still_emit`** (regression-guard) — the existing §17 events
   (fail-closed/audit-integrity, projection-degraded, fencing-conflict) still emit — this slice does NOT
   re-add or break them. Why: §17 — the daemon-owned failure surfaces are already live.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** expected (daemon-internal jobs; no new wire type) — **NO CONTRACT
  bump**. **EXCEPTION:** if Q2 adds a `stale` field to a projection's served shape, that's a contract
  decision → flag at Step 2.5 (I rule it; likely defer the served-`stale` to the ui-consuming slice and keep
  staleness a daemon-internal read-time derivation for now).
- **Shared-contract seam model touched?** **No** (unless Q2 surfaces `stale` on the wire).
- **Orchestrator doc rows to write hot (Step 9):** a §10/§17 AS-BUILT note (the WAL checkpointer + staleness
  poller LIVE; the Brain-sidecar + integration-failure surfaces deferred) + the daemon/CLAUDE.md module-org
  row. Orchestrator-written.

## Things to flag at Step 2.5
1. **Module home — `runtime/jobs.rs` vs a new `jobs/` module?** The tracker named `daemon/src/jobs/` (NEW),
   but the existing periodic jobs live in `runtime/drainer.rs`. My default vote: **`runtime/jobs.rs`** (keep
   the long-lived-task family together; the `spawn_drainer`/`spawn_reaper` neighbors). Take a `jobs/` module
   if you prefer the tracker's name — minor.
2. **Staleness representation — read-time derivation vs a persisted `stale` field?** `proj_session` has NO
   `stale` column. Options: (a) a pure read-time/recompute derivation (no column; the live-read-cache
   pattern — recompute on read/poll, never persisted) — **my default**; (b) a daemon-internal cache flag;
   (c) a served `stale` projection field (a CONTRACT decision — defer to the ui-consuming slice). My default
   vote: **(a) read-time derivation** — staleness is age-derived + time-varying (a stored value goes stale
   itself); recompute it (the worktree-status precedent). Surfacing it on the wire is a ui-slice follow-on.
3. **WAL checkpoint mode — PASSIVE vs TRUNCATE?** PASSIVE (non-blocking, best-effort) vs TRUNCATE (resets
   the WAL, may block on readers). My default vote: **PASSIVE on the interval** (non-blocking, won't contend
   with the read-only WAL readers; TRUNCATE only if a size threshold is crossed) — the single-writer +
   many-readers topology favors PASSIVE. Confirm.
4. **Missing-heartbeat (`last_heartbeat_at IS NULL`) → stale or not?** A session with no heartbeat yet
   (just-started, or the live drive loop hasn't written one). My default vote: **NOT stale** (absence ≠
   stale; only a heartbeat OLDER than the threshold is stale — avoid false-stale on a fresh session). Note:
   the live `last_heartbeat_at` WRITER (the session actor's status/telemetry tick) may be a thin add or a
   follow-on — if no production writer exists yet, the poller is correct-but-dormant until it lands (flag it).

## Dependencies + sequencing
- **Depends on:** 1.6b (✅ the runtime + `spawn_drainer`/`spawn_reaper` task pattern), 1.2 (✅ proj_session),
  the write-actor (✅). **NOT** Phase 8 (Brain sidecar — deferred) or Phase 5/7 (integrations — deferred arms).
- **Blocks:** nothing in Phase 4 (this is the LAST Phase-4 task — after it, Phase 4's deterministic core is
  complete). The deferred surfaces attach to their producers (Phase 8 / Phase 5/7).

## Estimated commit count
**1–2.** Likely **1** (both jobs + the pure `is_stale` + tests are one cohesive `runtime/jobs.rs` add,
non-safety). Split to 2 (checkpointer / staleness) only if each grows large. No safety-critical pin (the
single-writer invariant is PRESERVED via the write-actor command path — verified by test #4 + the reviewer).

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`: YES (invariant)** — narrow focus: the WAL checkpointer rides the **single
  write-actor** command path (forbidden #3 — NO rogue writable connection); the staleness poller reads
  **read-only WAL** only. Confirm no second writer + no back-pressure on the write-actor. Invariant-PRESERVATION.
- **`code-quality-reviewer`: YES** (every-slice).

## Lessons-logged candidates anticipated
- **Convention candidate** — "a periodic maintenance job (WAL checkpoint / staleness recompute) is a
  long-lived Tokio interval task in the `spawn_drainer`/`spawn_reaper` family (LESSON 9): a deterministic
  `*_once` bounded pass + the spawn loop; a WRITE pass rides the single write-actor command (never a rogue
  writer); a derived signal (staleness) is a live-read recompute (computed/rebuilt, never event-folded)."
- **Future TODO** — the deferred producer-gated surfaces (the Brain sidecar supervisor → Phase 8; the
  integration auth/rate-limit + offline failure events → Phase 5/7); the live `last_heartbeat_at` writer (if
  not wired here); surfacing `stale` on the wire (the ui-consuming slice).
- **Architecture-doc note candidate** — §10/§17 AS-BUILT (the WAL checkpointer + staleness poller LIVE).

## How to invoke
1. **Read this brief end-to-end** — especially the scope ruling + "Things to flag at Step 2.5".
2. **Run `/tdd background_jobs_wal_checkpoint_and_staleness`**.
3. **Step 0 (Restate)** — confirm the narrow scope (WAL checkpointer + staleness poller; the Brain-sidecar +
   integration surfaces deferred).
4. **Step 2.5** — send the Asserts/coverage write-up + answers to the 4 design questions (Q2 staleness-rep +
   Q3 checkpoint-mode are the main ones). Don't go GREEN until APPROVED.
5. **Step 8** — `security-reviewer` (single-writer invariant) + `code-quality-reviewer`.
6. **Step 9 (summarize)** — surface flags + the §10/§17 AS-BUILT + the deferred-surface Carry-forwards.

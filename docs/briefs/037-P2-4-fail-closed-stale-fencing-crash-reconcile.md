# /tdd brief — gateway_failclosed_stale_fencing_crash_reconcile

## Feature
The Action Gateway's **§17 failure-mode safety behaviors** — the last Phase-2 slice: **fail-closed on audit-write** (risk≥1 aborts if its terminal event can't be written), **stale-precondition re-check** (re-read the live source after lock + before execute → fresh approval if the previewed diff/resource changed), **fencing-conflict** (stale fencing token → `ActionFailed(fencing_conflict)`, never auto-resolved — via the real 1.4 `validate_held`), and **crash-reconcile** (on restart, reconcile orphaned `executing`/`queued` actions by idempotency key → terminal event or `unknown_outcome`). These are the deterministic safety LOGIC; the real external re-reads (git2/octocrab) land with their adapters (Phase 3/5/7/8) — 2.4 builds the seams + tests them via the §14 fault-injection hook + a fake clock + fake precondition oracle.

## Use case + traceability
- **Task ID:** P2.4
- **Architecture sections it implements:** `ARCHITECTURE.md §17` (the failure-mode contract — fail-closed / daemon-crash-mid-action / fencing), `§6.2` (stale-precondition re-check AG 16.4 + ActionResult), `§7.1` (the `ActionExecution*` event family — adds `ActionPartiallySucceeded` + the structured `ActionFailed` error), `§7.2` (re-read invariant — "re-read the live source before any mutation"), `§15` (INV-SEC-1 / fail-closed-on-audit-write), `§5.1` (the status machine + the rollback edges), `§14` (the fault-injection test hook — non-TDD-exempt: the hook IS the verification vehicle)
- **Related context:** brief `033` (2.1b — the pipeline + the marked 2.4 seams), brief `036` + session `012` (2.3 — the executor framework + idempotency the crash-reconcile keys off), the 2.4 task spec (`IMPLEMENTATION_PLAN.md` task 2.4, incl. the 1.4 fencing-Option-B ruling lines 360-362), the Carry-forward 2.4 working set, `daemon/src/locks/lease.rs:254` (`validate_held`), the 2.4-marked seams (`pipeline.rs:571-577`/`679-683`, `executor.rs:24/45/64`).

## Scope boundary — deterministic safety LOGIC + seams, real external re-reads deferred (read FIRST)
Like 2.3, 2.4 builds the **framework/seams** for behaviors whose real external dependency lands later:
- **Stale-precondition re-read** of the LIVE source (git2 worktree state / octocrab PR state) needs the git/integration adapters (Phase 5/7). 2.4 builds the **re-check seam** + the stale→fresh-approval LOGIC, exercised by a **fake `PreconditionOracle`** (reports changed/unchanged); the real git2/octocrab re-reads swap in at their phases.
- **Crash-reconcile** re-derives real-world state via git2 re-read / octocrab GET / lease check. The git2/octocrab re-reads are later-phase; 2.4 builds the **reconcile scan + the idempotency-key + `validate_held` re-derivation + the transition/terminal-event emission + the `unknown_outcome` fallback**, exercised by the §14 fault-injection hook (SIGKILL the write-actor mid-execute → restart → reconcile) against the stub executor.
- **Fencing** uses the REAL 1.4 `validate_held` (it exists) — 2.4 wires lease-acquire + fencing-token-bind + the check into the execute path for real.
- **Fail-closed** uses fault injection (inject an audit-write failure at the completion txn) — fully real + deterministic.

The acceptance pins are written to be testable **without** a real side effect (fault injection + fake clock + fake oracle).

## Acceptance criteria (what "done" means)

**L1 — §17 contract additions (freeze; CONTRACT 0.18.0→0.19.0):**
- [ ] NEW event type **`ActionPartiallySucceeded`** (the §17 side-effect-applied-but-terminal-event-unwritable record) added to the §7.1 `ActionExecution*` family.
- [ ] A **structured `ActionFailed` error taxonomy** — replace/augment the 2.1b `ActionFailed{error: String}` with a typed `ActionError` (`audit_write_failed` / `stale_precondition` / `fencing_conflict` / `unknown_outcome` / `executor_error{message}`) consumed by L2-L5 (the "typed execute-error taxonomy" 2.4 carry-forward).
- [ ] The §5.1 **rollback edges** wired in the transition guards (`succeeded`/`partially_succeeded → rolled_back`/`rollback_failed`; the statuses are already frozen — this is the `can_transition` edges + the rollback-invocation seam, default fail-closed `rollback_failed`).
- [ ] **§2.5-seam schema-snapshot test** (`spec(§7.1)`-tagged) for the extended event family / error taxonomy + the 3-way verify; CONTRACT 0.18.0→0.19.0; NO behavior (L2-L5 consume these).

**L2 — fail-closed on audit-write (§15/§17 INV-SEC-1):**
- [ ] An audit-required (risk≥1) action whose **terminal event (`ActionSucceeded`/`Failed`) cannot be written** aborts: the completion txn rolls back → the action stays `executing` (reconciled on restart by L5), NEVER acked succeeded (fault-injection: inject an audit-write failure → assert the mutation is not acknowledged + no success event persists).
- [ ] A **side effect applied but its terminal event can't be written** → emit `ActionPartiallySucceeded` best-effort + a hard audit-integrity alert (tested with a fake executor that reports "applied" + injected event-write failure).
- [ ] Risk-0 (no audit-required-terminal? — confirm at Step 2.5) behavior unchanged.

**L3 — fencing-conflict (§17 / 1.4 Option-B):**
- [ ] The execute path **acquires a lease + binds `fencing_token`** on the action, then calls `locks::validate_held(resource_id, lease_kind, owner, token, now)` **after lock + before execute** → `false` → `ActionFailed(fencing_conflict)` + a hard-conflict surface (**never auto-resolved**).
- [ ] "Stale" = NOT a live lease (**expired OR superseded**) per the 1.4 ruling.
- [ ] A long-running action **heartbeats/renews** (or re-acquires) its lease so it doesn't self-fence on its own expiry (renew strictly *before* the `expires_at > now` boundary; fake-clock tested).
- [ ] The **same-owner re-acquire contract** is decided + pinned (idempotent renew-like vs error — 1.4 deliberately left it open).

**L4 — stale-precondition re-check (§6.2 AG 16.4 / §7.2):**
- [ ] After lock + before execute, **re-read the live source** (via the `PreconditionOracle` seam) → on mismatch → `ActionFailed(stale_precondition)`, **regenerate the preview**, and **require fresh approval if the previewed diff/resource changed** (never execute a different mutation than was approved).
- [ ] Unchanged precondition → execute proceeds normally.
- [ ] The re-check ordering is **after lease+fencing acquisition (L3) + before execute** (the §6.2 order).

**L5 — crash-reconcile (§17 daemon-crash-mid-action):**
- [ ] On restart (bootstrap), **scan `action_requests WHERE status IN ('executing','queued')`** (the orphaned set — `executing` = crash mid-execute; `queued` = the two-txn/cascade gap from 2.1b/2.1c/2.2) → re-derive via **idempotency key + `validate_held`** → transition `succeeded`/`failed`/`partially_succeeded` + **emit the missing terminal event**; **un-reconcilable → `unknown_outcome`** (`ActionFailed(unknown_outcome)` + audit-integrity alert).
- [ ] The reconcile covers BOTH the single-action and the **N-orphan cascade** form (2.1c carry-forward).
- [ ] Fault-injection: SIGKILL/abort the write-actor mid-execute (the §14 hook at the named checkpoint) → restart → assert the reconcile drives every orphan to a terminal state + emits the terminal events.

**All layers:**
- [ ] All tests in `daemon/tests/recovery.rs` (NEW) + the relevant `gateway.rs`/`executor.rs` pass; existing suite stays green.
- [ ] `/preflight` clean. **security-reviewer EVERY layer** (INV-SEC-1/§15/§17 — the safety capstone).
- [ ] CONTRACT 0.18.0→**0.19.0** (L1 only; L2-L5 are daemon-internal behavior) + the §2.5-seam snapshot + 3-way verify.

## Wiring / entry point (Step 7.5)
- **L1** — the contract additions ship in `shared/` (the event family + the `ActionError` enum) — reachable via the `ActionExecution*` emission in the pipeline (L2-L5 consume them).
- **L2/L3/L4** — wire into the execute path (`Gateway::execute`, `pipeline.rs:671`) + the approve/auto-execute callers; reachable via `approve`→execute, the plan-approve cascade, and the risk-0 auto-execute path (the 3 gated seams).
- **L5** — wires into **bootstrap/cold-start** (`daemon/src/bootstrap.rs` cold_start, after `EventStore::open`'s catch-up-replay + outbox crash-recovery — the existing restart-recovery composition point); reachable on every daemon start. The §14 fault-injection hook is a `#[cfg(test)]`/feature-gated checkpoint in the write-actor.

## Files expected to touch
**New:**
- `daemon/src/gateway/recovery.rs` — the crash-reconcile scan + re-derivation (L5).
- `daemon/tests/recovery.rs` — the 2.4 RED tests (fail-closed + fencing + stale-precondition + crash-reconcile).
- `shared/` additions: `ActionError` enum + the `ActionPartiallySucceeded` event type (L1).

**Modified:**
- `shared/src/events.rs` / `actions.rs` / `status.rs` — the L1 contract additions (event family + error taxonomy + rollback edges) + the snapshot.
- `daemon/src/gateway/pipeline.rs` — the execute path: lease-acquire + fencing check (L3) + stale-precondition re-check (L4) + fail-closed terminal-event handling (L2); the §17 error mapping.
- `daemon/src/gateway/executor.rs` — the structured `ExecutionOutcome`/`ExecError` → the `ActionError` taxonomy (L1/L2); the `PreconditionOracle` seam (L4); the `rollback` edges (L1).
- `daemon/src/gateway/request.rs` — the `can_transition` rollback edges; the orphan-scan query (L5).
- `daemon/src/bootstrap.rs` — wire `recovery::reconcile_orphans` into cold_start (L5).
- `daemon/src/ipc/methods.rs` — map the new `ActionError` variants → §6.4 IPC codes (`precondition_stale` / a fencing code / `internal_error`? — Step-2.5 Q, ties the 2.1b `IpcErrorCode` carry-forward).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2) — `daemon/tests/recovery.rs` (+ `executor.rs`/`gateway.rs`)
**L1 (contract):** snapshot test for the extended event family + `ActionError`; 3-way verify; the rollback-edge `can_transition` unit.
**L2 (fail-closed):**
1. **`audit_write_failure_aborts_risk1_action`** — inject a completion-txn event-write failure on a risk≥1 action → assert NOT acked succeeded, no `ActionSucceeded` persists, action stays `executing`. Why: §15/§17 fail-closed.
2. **`side_effect_applied_event_unwritable_partially_succeeds`** — fake executor reports "applied" + injected event-write failure → `ActionPartiallySucceeded` + audit-integrity alert. Why: §17 line 444.
**L3 (fencing):**
3. **`stale_fencing_token_fails_closed`** — fake-clock expire the lease → `validate_held` false → `ActionFailed(fencing_conflict)`, never auto-resolved. Why: §17/1.4 Option-B.
4. **`long_action_renews_before_self_fence`** — heartbeat renews before `expires_at` → not self-fenced. Why: 1.4 strict-boundary flag.
**L4 (stale-precondition):**
5. **`precondition_changed_requires_fresh_approval`** — oracle reports changed → `ActionFailed(stale_precondition)` + regenerate preview + fresh-approval required. Why: §6.2 AG 16.4.
6. **`precondition_unchanged_executes`** — oracle unchanged → execute proceeds. Why: the happy path.
**L5 (crash-reconcile):**
7. **`orphaned_executing_reconciled_on_restart`** — SIGKILL mid-execute (the §14 hook) → restart → orphan driven to terminal + terminal event emitted. Why: §17 daemon-crash row.
8. **`cascade_orphans_all_reconciled`** — N-orphan plan cascade → all reconciled. Why: 2.1c carry-forward.
9. **`unreconcilable_orphan_unknown_outcome`** — un-derivable → `unknown_outcome` + audit-integrity. Why: §17 "un-reconcilable → unknown outcome."

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **L1 — YES** (the `ActionExecution*` family gains `ActionPartiallySucceeded`; `ActionFailed` gains a structured `ActionError`). → §2.5-seam (`§7.1 EventTypeRegistry` IS a seam model — `daemon/CLAUDE.md` line 138) → **the schema-snapshot test is MANDATORY**, CONTRACT 0.18.0→0.19.0, 3-way verify.
- **Orchestrator doc rows to write hot (Step 9):** the §7.1 `EventTypeRegistry` Appendix-A row (add `ActionPartiallySucceeded` + the `ActionError` taxonomy, CONTRACT 0.19.0); the §17 rows flip `[IMPLEMENTED 2.4]` (fail-closed / daemon-crash-reconcile / fencing); the §6.2 stale-precondition note `[IMPLEMENTED 2.4]`; the §6.1 GatewayPort CONTRACT → 0.19.0; the `daemon/CLAUDE.md` cross-doc EventTypeRegistry row.
- **§2.5-seam:** YES (§7.1) — the L1 snapshot test is required, authored in the L1 cycle.

## Things to flag at Step 2.5
1. **Layer split / ordering.** 5 layers (L1 contract-freeze → L2 fail-closed → L3 fencing → L4 stale-precondition → L5 crash-reconcile), each its own commit + security-reviewer, mirroring the execute-path order (acquire lease+fencing → re-read/stale-check → execute → [restart-reconcile]). My default vote: **this order** (L3 fencing before L4 stale-precondition because §6.2 puts lease+fencing acquisition before the live-source re-read). Confirm or propose a different grouping (e.g. fold L1 into L2).
2. **`ActionError` shape.** A typed enum (`audit_write_failed`/`stale_precondition`/`fencing_conflict`/`unknown_outcome`/`executor_error{message}`) on `ActionFailed`, replacing the 2.1b `{error: String}`. My default vote: **typed enum** (the 2.4 "typed execute-error taxonomy" carry-forward; drives the §6.4 IPC-code mapping + the UI conflict cards). Confirm the variant set + whether `executor_error` keeps a free `message`.
3. **The §14 fault-injection hook.** A `#[cfg(test)]` (or `cfg(feature="fault-injection")`) checkpoint enum in the write-actor (`mid-apply` / `post-event-pre-projection` / `mid-execute` / `pre-terminal-event`) that a test can arm to abort/panic. My default vote: **a cfg-gated checkpoint enum** (matches the §14 test row's named checkpoints; zero cost in release). Confirm the checkpoint set.
4. **The `PreconditionOracle` seam (L4).** A trait the gateway calls after lock to re-read the live source, faked in 2.4 (real git2/octocrab at Phase 5/7). My default vote: **a `PreconditionOracle` trait** (parallels the 2.3 `ActionExecutor` framework-with-stubs); the stale→fresh-approval logic is real + tested via the fake. Confirm.
5. **Same-owner re-acquire contract (L3).** 1.4 returns `Held{owner:self}` on any live re-acquire incl. the same owner; the gateway decides idempotent-renew vs error. My default vote: **idempotent renew-like** (a long action re-acquiring its own live lease is a heartbeat, not a conflict). Confirm.
6. **Crash-reconcile for STUB executors (L5).** With stub executors (no real side effect), an orphaned `executing` action can't be re-derived via git2/octocrab — so what's its reconcile target? My default vote: **for 2.4, the stub-executor orphan reconciles via the idempotency-key + `validate_held` lease check → if the lease is no longer held → `unknown_outcome`; the real git2/octocrab re-read (definitive succeeded/failed) lands with the adapters**. Confirm — this keeps L5 honest (the framework + the unknown-outcome fallback are real; the definitive external re-read is later-phase).
7. **§6.4 IPC-code mapping for the new errors.** `fencing_conflict` → a new code or reuse? `unknown_outcome`/`audit_write_failed` → `internal_error` (the 2.1b carry-forward flagged `IpcErrorCode` has no `internal_error`). My default vote: **resolve the 2.1b `internal_error` carry-forward here** — add `internal_error` to §6.4 (a §6.4 contract extension — escalate as a load-bearing-contract call if it widens the wire) + map `stale_precondition`→`precondition_stale` (exists). Flag for an escalation if it changes the wire error set.

## Dependencies + sequencing
- **Depends on:** 2.1 (the pipeline + the marked 2.4 seams ✅), 2.2 (the policy + the auto-execute path ✅), 2.3 (the executor framework + idempotency the reconcile keys off ✅), 1.4 (`validate_held` ✅), 1.6a (bootstrap cold_start — the L5 wire point ✅).
- **Blocks:** `/phase-exit 2` (the Phase-2 acceptance criteria: "Fail-closed + fencing-conflict + stale-precondition behaviors tested deterministically (fault-injection + fake clock)"); Phase 3/5/7/8 (the real executors swap the stale-precondition/crash-reconcile external re-reads into the seams this lands).

## Estimated commit count
**5** — one per layer, each its OWN commit (the safety capstone — every layer touches INV-SEC-1/§15/§17; never bundled across the safety seam):
- **L1** §17 contract additions (event family + `ActionError` + rollback edges + the §2.5-seam snapshot; CONTRACT 0.19.0).
- **L2** fail-closed on audit-write (+ ActionPartiallySucceeded).
- **L3** fencing-conflict (lease-acquire + `validate_held` + heartbeat/renew).
- **L4** stale-precondition re-check (the `PreconditionOracle` seam + fresh-approval-on-change).
- **L5** crash-reconcile (the bootstrap orphan scan + re-derivation + unknown-outcome).

Drive **layer→layer**; security-reviewer **every** layer; the fault-injection hook + fake clock + fake oracle keep every behavior deterministic. **If the impl finds L1+L2 or L3+L4 naturally combine, that's a Step-2.5 grouping call.**

## Lessons-logged candidates anticipated
- **Convention candidate** — "The §17 safety behaviors are built as deterministic LOGIC + seams (fault-injection hook + fake clock + fake `PreconditionOracle`); the real external re-reads (git2/octocrab) land with their adapters — like the 2.3 executor framework."
- **Convention candidate** — "Fail-closed extends to the EXECUTE completion txn: a risk≥1 terminal-event-unwritable aborts (stays `executing` → crash-reconcile), never acks succeeded; side-effect-applied-but-unwritable → `ActionPartiallySucceeded` + audit-integrity."
- **Architecture-doc note candidate** — the §17 rows `[IMPLEMENTED 2.4]`; the fencing same-owner re-acquire contract; the structured `ActionError` taxonomy + its §6.4 mapping.
- **Future TODO** — real rollback edges + the rollback executor (the default is fail-closed `rollback_failed`); the definitive git2/octocrab crash-reconcile re-read (Phase 5/7).

## How to invoke
1. **Read this brief end-to-end** — especially the Scope boundary (deterministic-logic-+-seams, real external re-reads deferred) + the 7 Step-2.5 questions (a safety capstone has real design surface).
2. **Run `/tdd gateway_failclosed_stale_fencing_crash_reconcile`**.
3. **Step 0** — confirm the restatement + the 5-layer structure + the framework-not-real-external-reads scope.
4. **Step 1** — confirm files.
5. **Step 2.5** — send the test-design write-up PER LAYER (assert-line + coverage map); answer Q1-Q7. **L1 touches the §7.1 §2.5-seam → the snapshot test is mandatory + CONTRACT 0.19.0**; flag Q7 if the §6.4 wire error set changes (load-bearing-contract escalation). Wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 9** — categorized flags; the orchestrator routes hot + authors the commit message + the §17/§7.1 doc-writes.

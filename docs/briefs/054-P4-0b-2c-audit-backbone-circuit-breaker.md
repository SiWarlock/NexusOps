# /tdd brief — audit_backbone_circuit_breaker

## Feature
A **daemon-wide audit-backbone circuit-breaker**: the single audited mutator **fail-stops when it can no longer audit**. Per-action audit-write faults already deny + raise the durable integrity alarm (4.0b-2 C2); this slice adds the **SYSTEMIC** layer — N-consecutive (or clearly-unrecoverable) audit-write faults trip a latched breaker → the daemon **fail-stops** (halt + durable systemic alarm). Generalizes the C2 *intercept* alarm (`route_intercept_live`) to the **whole audit backbone** (every Gateway audit-write, not just the agent-mutation interception).

## Use case + traceability
- **Task ID:** P4.0b-2c (the `### 4.0b-2c` Phase-4 row — the audit-backbone circuit-breaker; USER-ruled "best practice", REQUIRED-not-deferred)
- **Architecture sections it implements:** `ARCHITECTURE.md §17` (Failure-mode contract — the "Event-write fails (audit-required = all risk≥1)" row), `§15 #5` (Fail-closed on audit-write — *the single audited mutator does not operate when it cannot audit*).
- **Related context:**
  - `daemon/src/integrity.rs` — the C2 durable alarm (`IntegrityAlarm` trait, `FileIntegrityAlarm` prod sink, `IntegrityIncident`/`IntegrityKind::AuditWriteFailed`, `RecordingIntegrityAlarm` test double). Its own module doc (lines 12-14) **explicitly scopes this slice as "part 3"** — the systemic-failure circuit-breaker the per-incident sink builds toward.
  - `daemon/src/harness/claude/intercept.rs` — `route_intercept_live` (C2): raises the alarm on a *single* intercept audit-fault, keyed structurally off `classify_gateway_error(&e) == GatewayDenyKind::AuditWriteFailed`. **Reuse `classify_gateway_error`** so the daemon-wide classification matches C2's exactly.
  - `daemon/src/gateway/mod.rs` — `GatewayError::AuditWriteFailed(#[from] EventStoreError)` is the fail-closed signal (wraps any failed authoritative-event append OR registry-row write in a gateway txn). `db_err`/`lease_err` already funnel DB faults into it.
  - `daemon/src/gateway/pipeline.rs` (~1026-1058) — the 2.4 L2 `execute()` txn-B/txn-C fail-closed path: txn-B fails → stays `executing`; side-effect-applied + txn-C fails → `AuditWriteFailed`. This is the daemon-wide producer of audit-write faults the breaker must observe.
  - `daemon/src/eventstore/mod.rs` — `gateway_txn` (`:232`) is the single chokepoint every Gateway audit-write flows through.
  - **LESSONS §21** (the §17 fail-closed capstone — txn split, record-then-throw, the cfg-gated `fault-injection` Cargo FEATURE = the deterministic test vehicle), **§26** (audit-BEFORE-verdict; an un-auditable path → deny+compensate), **§30** (not every I/O on a safety path is safety I/O — this slice *is* safety I/O: the invariant IS a function of the audit succeeding).

## Acceptance criteria (what "done" means)
- [ ] **One-off fault does NOT trip.** A single `AuditWriteFailed` → the existing per-action Deny + the C2 durable alarm fire, the consecutive-fault counter = 1 (< N), the breaker is **not tripped**, and a subsequent successful audit-commit **resets the counter to 0**.
- [ ] **N consecutive faults trip the breaker.** N `AuditWriteFailed` with no intervening success → the breaker **trips** → a durable alarm with the **new systemic `IntegrityKind`** is raised + the daemon **fail-stops** per the Step-2.5-ruled mechanism (Q1).
- [ ] **Success resets the run.** Fault, fault, success, fault → counter = 1, **not tripped** (the run is consecutive-only — a clean commit between faults clears it).
- [ ] **Clearly-unrecoverable classes fast-trip** (Q3): a disk-full / DB-corruption `EventStoreError` class trips on **first** occurrence (below N); a transient class (`SQLITE_BUSY` / lock-timeout) counts toward N but does **not** fast-trip.
- [ ] **The trip latches — no auto-reset** (Q5): once tripped, the breaker stays tripped across all subsequent calls (the §17 "never auto-resolved" ethos for a safety-state); recovery is an explicit operator restart.
- [ ] **The systemic incident is distinguishable** in the durable alarm log: a new `IntegrityKind` variant (e.g. `AuditBackboneSystemicFailure`), distinct from the per-incident `AuditWriteFailed`, with a content-free factory ctor (the `integrity.rs` by-construction discipline).
- [ ] **[RULED (B) quiesce-and-refuse — lead 2026-06-13]** The latch flips **ATOMICALLY** the instant a SYSTEMIC fault is detected; once latched, **every** mutation path fail-closed-denies with an audit-backbone-down reason **without attempting an audit-write** (spy the write seam — it is NOT called; **no mutation slips the trip window**), while **reads / subscribe / terminals stay live** (not gated). The systemic trip raises the durable systemic alarm **+ a §17 safety-state** raised loudly (daemon-side: the latched breaker state is exposed for the §17 surface to read; the UI display is Phase-6/cross-track).
- [ ] **Deterministic.** Driven by the existing `fault-injection` Cargo feature (`FaultPoint::AuditEventWrite`) + a fake `Clock`; the breaker counter/trip is **pure logic**, unit-tested with no real disk fault.
- [ ] All unit tests in `daemon/tests/circuit_breaker.rs` (or `tests/recovery.rs`) pass.
- [ ] `/preflight` clean (`cargo fmt --check && clippy -D warnings && check && test`).
- [ ] Cross-doc: the §17 AS-BUILT note + Appendix A row written by the orchestrator atomic with the round (the implementer flags at Step 9 — it is orchestrator territory).

## Wiring / entry point (Step 7.5)
The breaker is constructed in **`daemon/src/main.rs`** (production daemon assembly) and injected into the `Gateway`. It is **reachable on the live mutation path**: every IPC `submit_action`/`approve`/`session.create`/`session.kill` handler (`daemon/src/ipc/methods.rs`) → the Gateway → `gateway_txn`, where the breaker **observes** each audit-write outcome and (mechanism B) **gates** the entry. The fault-observation seam sits at the single `gateway_txn` chokepoint so **no mutation path bypasses it** (the INV-SEC-1 one-chokepoint discipline). Confirm the exact placement at Step 2.5 (Q6) — production-reachable, not test-only.

## Files expected to touch
**New:**
- `daemon/src/gateway/circuit_breaker.rs` *(or extend `integrity.rs`)* — the breaker state (consecutive-fault counter + the latched trip flag, thread-safe), the pure `observe(outcome) -> Action` classifier (Reset / Count / Trip), and the recoverable-vs-unrecoverable `EventStoreError` classification. Pure logic, unit-testable in isolation.
- `daemon/tests/circuit_breaker.rs` — the RED tests below *(or fold into `daemon/tests/recovery.rs`, the §17 home — Q at Step 1)*.

**Modified:**
- `daemon/src/integrity.rs` — add the systemic `IntegrityKind` variant + its content-free factory ctor.
- `daemon/src/gateway/pipeline.rs` *(and/or `gateway/mod.rs` / `eventstore/mod.rs`)* — feed the breaker the success/fault outcome at the `gateway_txn` chokepoint; (mechanism B) check `breaker.is_tripped()` at the mutation entry before attempting any audit-write.
- `daemon/src/main.rs` — construct + inject the production breaker (+ the fail-stop handler for the ruled mechanism).
- `daemon/src/harness/claude/intercept.rs` — confirm intercept call-2 audit-faults also feed the daemon-wide counter (Q4) — they are audit-backbone faults.
- `daemon/src/lib.rs` — module declaration if a new `circuit_breaker` module is added.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `daemon/tests/circuit_breaker.rs`:

1. **`test_single_audit_fault_denies_and_alarms_but_does_not_trip`** — one injected `AuditWriteFailed` → per-action Deny + C2 alarm raised, counter = 1, `!breaker.is_tripped()`.
   - Asserts: breaker not tripped after a lone fault; the per-action surface is unchanged.
   - Why: §17 "transient/one-off → per-action deny + durable alarm" (task line 512).
2. **`test_n_consecutive_audit_faults_trip_the_breaker`** — N consecutive faults → `breaker.is_tripped()` + a systemic-kind alarm raised (assert via `RecordingIntegrityAlarm`).
   - Asserts: trip at exactly N; the systemic alarm carries the new `IntegrityKind`.
   - Why: §17 "SYSTEMIC (N-consecutive) → FAIL-STOP" + §15 #5.
3. **`test_success_between_faults_resets_the_counter`** — fault, fault, success, fault → counter = 1, not tripped.
   - Asserts: a clean commit resets the consecutive run.
   - Why: "consecutive" is the systemic signal, not cumulative-ever.
4. **`test_unrecoverable_class_fast_trips_below_n`** *(Q3)* — a disk-full/corruption `EventStoreError` → trips on first; a `SQLITE_BUSY`-class fault does not.
   - Asserts: the recoverable/unrecoverable split; immediate trip on the user-named cases.
   - Why: §17 "clearly-unrecoverable — disk-full, DB corruption" (task line 512).
5. **`test_tripped_breaker_does_not_auto_reset`** *(Q5)* — after a trip, subsequent observe-success calls do NOT clear the latch.
   - Asserts: latched; only a restart clears it.
   - Why: §17 never-auto-resolved safety-state ethos (rule #6 family).
6. **`test_tripped_breaker_fail_closed_denies_mutations_without_audit_write`** *(mechanism B — Q1)* — post-trip, a mutation submit denies immediately; the write seam is never invoked (spy).
   - Asserts: the un-bypassable quiesce gate; no audit-write attempted while tripped.
   - Why: "does not operate when it cannot audit" — refuse before trying.
7. **`test_reads_stay_live_when_breaker_tripped`** *(mechanism B — Q1)* — `get_projection`/`subscribe` succeed while the breaker is tripped.
   - Asserts: only mutations are gated; observability is preserved.
   - Why: the production-grade posture (the operator must SEE the incident).
8. **`test_intercept_audit_fault_counts_toward_systemic`** *(Q4)* — an intercept call-2 `AuditWriteFailed` increments the same daemon-wide counter.
   - Asserts: the intercept path is part of the audit backbone the breaker observes.
   - Why: the slice generalizes the C2 intercept alarm daemon-wide.

> Tests 6/7 (and the exact shape of the fail-stop pin) are **mechanism-dependent** — finalize after Q1 is ruled (see "Things to flag"). Author the mechanism-independent tests (1-5, 8) first.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** the new `IntegrityKind` variant is **daemon-internal** (`integrity.rs`) — NOT a `shared/` contract → **no CONTRACT bump**, no schema-snapshot (no shared-contract seam crossed). (Same posture as the C2 sink: `integrity.rs` has no `shared/` surface.)
- **Orchestrator doc rows to write hot (Step 9 routing):** the **§17 AS-BUILT note** — the "Event-write fails" row gains the systemic fail-stop layer (the circuit-breaker: per-action deny handles the one-off; the breaker declares SYSTEMIC → fail-stop; latched, never auto-reset) → §17 row + **Appendix A**. **escalate-note: a §15/§17 safety mechanism (fail-secure)** — the design (esp. Q1) surfaces to the lead→user **before Step-2.5 sign-off**.
- **Shared-contract (schema-snapshot) model touched?** No — `integrity.rs` is daemon-internal.

## Things to flag at Step 2.5

> **★ THE CRUX (lead-flagged, must be explicit in the Step-2.5 write-up): define precisely what trips the latch — SYSTEMIC vs transient.** 4.0b-2c is the *systemic* breaker, distinct from the per-action fail-closed already shipped. State exactly what trips it (persistent/repeated `AuditWriteFailed` at the `gateway_txn` chokepoint — the N-consecutive run of Q2 + the unrecoverable-class fast-trip of Q3) so a **single transient hiccup does NOT latch the whole daemon** — the existing per-action deny already covers the one-off. Q2+Q3 ARE this definition; surface them explicitly.

1. **[RULED (B) quiesce-and-refuse — lead 2026-06-13; a realization of the user's already-ruled fail-stop posture, FYI-to-user, NOT a gate]** "Fail-stop / halt" = **(B) quiesce-and-refuse**: the daemon stays up; a **latched Gateway breaker** makes every subsequent mutation fail-closed-deny (a new audit-backbone-down deny reason) **without attempting an audit-write**, while **reads/subscribe/terminals stay live** and the durable alarm + a §17 safety-state are loud. (A=process-exit was rejected — restart-thrash on persistent disk-full + kills observability.) **Lead pins to honor:** (i) latch **ATOMICALLY** the instant a systemic fault is detected — **no mutation slips the trip window**; (ii) **no auto-unlatch** — recovery = explicit operator restart only (never flap); (iii) reads/subscribe/terminals stay LIVE + durable alarm + §17 safety-state raised loudly; (iv) the systemic-vs-transient trip condition is THE crux (callout above). The new latched gate is a **new un-bypassable mutation gate = a new INV-SEC-1 surface → its OWN `security-reviewer` pass** (as mandated). A=process-exit is recorded for the audit trail only — **do not build it.**
2. **Threshold N for "consecutive" (part of THE CRUX — Q1 callout).** Default **5** — rides out a brief transient cluster (each fault still per-action-denies + alarms) but trips fast on a sustained fault. Tunable.
3. **Immediate fast-trip on clearly-unrecoverable classes (disk-full / DB-corruption) below N?** Default **yes** — an explicit allow-list of `EventStoreError`/SQLite codes (`SQLITE_FULL`, `SQLITE_CORRUPT`, `SQLITE_IOERR`); a transient `SQLITE_BUSY`/lock-timeout is recoverable (counts toward N, no fast-trip). The user named these exact cases.
4. **Does the breaker observe the intercept call-2 audit-faults (count toward systemic)?** Default **yes** — an intercept audit-write is part of the audit backbone; the per-incident alarm already fires there, the breaker additionally counts it.
5. **Auto-reset?** Default **no** — latched until restart (the §17 fencing-conflict "never auto-resolved" ethos for a safety-state). Recovery = cold-start re-open + reconcile.
6. **Seam for observing success/fault + (B) the gate.** Default: classify at the Gateway `Result` boundary via the **existing `classify_gateway_error`** (reused from `intercept.rs`) so the daemon-wide classification matches C2's; feed + gate the breaker at the single `gateway_txn` chokepoint so **no mutation path bypasses it**. Confirm placement.

## Dependencies + sequencing
- **Depends on:** 4.0b-2 C2 (the durable integrity-incident sink — `integrity.rs` — landed ✅, `a83c498`) + 2.4 §17 audit-fault pipeline (`AuditWriteFailed` + the `fault-injection` feature — landed ✅).
- **Blocks:** nothing hard — a standalone safety hardening. (Lead's order after this: ui-① [brief 052] → 4.0b-2-F2.)

## Estimated commit count
**1 (own commit — safety-critical, NOT bundled).** This is a §15/§17 fail-secure mechanism → it gets its own commit, never bundled (root `CLAUDE.md` "Key safety rules"; template "safety-critical pin → own commit"). Split to **2** only if the mechanism-B Gateway gate + `main.rs` wiring grows large enough to bisect meaningfully from the pure breaker logic + integrity-kind — decide at Step 2.5 once Q1 is ruled. **`security-reviewer` runs** (the `invariant` policy — this slice touches a safety invariant; its OWN pass, per the task).

## Lessons-logged candidates anticipated
- **Convention candidate** — "The audit-backbone circuit-breaker: per-action deny handles the one-off (the invariant is already protected each fault); the breaker declares **SYSTEMIC** (N-consecutive or a clearly-unrecoverable class) → **fail-stop**; the trip **latches** (never auto-resets — the §17 fencing-ethos); under quiesce-and-refuse, **mutations quiesce while reads stay live** so the operator can see the incident."
- **Architecture-doc note candidate** — the §17 "Event-write fails" row gains the systemic fail-stop layer over the per-action fail-closed.
- **Future TODO — operational** — a metric/health surface for the breaker state (tripped/half-open) for the Phase-6 §17 safety-state display + a possible operator-driven reset affordance (not MVP — restart clears it).

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" Q1 (the cat-1 safety fork). The mechanism ruling lands before Step-2.5 sign-off; author the mechanism-independent RED tests (1-5, 8) first.
2. **Run `/tdd audit_backbone_circuit_breaker`** in the implementer session.
3. **Step 0 (Restate)** — confirm against the Feature line.
4. **Step 1** — confirm the file list; decide `circuit_breaker.rs` new-module vs `integrity.rs` extension, and the test home (`tests/circuit_breaker.rs` vs `tests/recovery.rs`).
5. **Step 2.5** — send the test-design write-up + answers to Q1-Q6. **Do NOT go GREEN on the mechanism (Q1) until the orchestrator routes the safety ruling back.**
6. **Step 8** — `security-reviewer` (own pass: the fail-stop threshold, the recovery posture, no-false-halt on a transient, the un-bypassable gate).
7. **Step 9** — surface the §17 cross-doc note + any deviation from the anticipated lessons-logged candidates.

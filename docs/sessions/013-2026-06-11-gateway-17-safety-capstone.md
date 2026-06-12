# Session 013 — Phase 2.4: the Action Gateway's §17 safety capstone (L1–L5)

- **Date:** 2026-06-11
- **Phase:** Phase 2.4 — fail-closed / stale-precondition re-check / fencing-conflict / crash-reconcile (the LAST + most safety-critical Phase-2 slice)
- **Predecessor:** [012 — policy engine + executor framework](012-2026-06-11-policy-engine-and-executor-framework.md)
- **Successor:** [014 — §18 perf benchmark + §14 CI merge gates (Phase-2 close-out)](014-2026-06-12-perf-benchmark-and-ci-gates.md)

## Why this session existed

Phase 2.1–2.3 built the Action Gateway pipeline (chokepoint + plans + policy/risk-0 auto-execute + the executor/preview/idempotency framework). Phase 2.4 closes the pipeline with its **§17 failure-mode behaviors** — the deterministic safety LOGIC + seams (real external re-reads land with the adapters, Phase 3/5/7/8). Brief `037` (spec-lint PASS @45f61d16); 5 layers, each its own commit, security-reviewer every layer.

## What was built (5 layers, 5 commits — all LOCAL, push user-gated)

**L1 — §17 contract freeze (`89bf300`, CONTRACT 0.18.0→0.19.0):**
- `shared/src/events.rs`: NEW `ActionPartiallySucceeded{reason}` event (side-effect-applied-but-terminal-event-unwritable record; structural reason, §15); `ActionFailed.error` `String → ActionError`.
- `shared/src/actions.rs`: NEW `ActionError` taxonomy (internally-tagged on `kind`, the ServerFrame precedent): `audit_write_failed`/`stale_precondition`/`fencing_conflict`/`unknown_outcome`/`executor_error{message}`.
- `shared/src/ipc.rs`: `IpcErrorCode` += `fencing_conflict` + `internal_error` (lead-ruled Option A).
- `shared/src/{schema.rs,lib.rs}` + regen schema; `daemon/src/gateway/{request,pipeline}.rs` (ExecutorError mapping); §5.1 rollback `can_transition` edges. NO behavior.

**L2 — fail-closed on audit-write (`8ea9e20`):**
- `pipeline.rs::execute` SPLIT into txn-A (Queued→Executing+ActionStarted, COMMITS) → executor (off the write-actor) → txn-B (terminal). A terminal-event write failure → stays `executing` (orphan→L5), NEVER acked succeeded. Side-effect-applied + unwritable → txn-C `ActionPartiallySucceeded` best-effort; audit-fully-broken → `AuditWriteFailed` + stays executing.
- `executor.rs`: `ExecutionOutcome::Succeeded += side_effect_applied` + the `side_effect_applied()` method.
- NEW `daemon/src/fault.rs` — the §14 fault-injection (`fault-injection` Cargo feature via the self-dev-dependency idiom; compiled OUT of release). `GatewayTxn::append` consults `TerminalEventWrite`.

**L3 — fencing-conflict via the real 1.4 `validate_held` (`daed666`):**
- `execute` acquires a `resource_mutation` lease over the primary resource (owner = action_request_id), binds the token, validates before execute. `Held{other}`/validate-false → record `ActionFailed{FencingConflict}` (COMMITS) + `Err(GatewayError::FencingConflict)` → §6.4 `fencing_conflict` (never auto-resolved, rule #6). `Held{self}` → idempotent renew (fail-closed on a missing token).
- `eventstore/mod.rs`: `gw_acquire_lease`/`gw_validate_lease` (the 1.4 wrappers over the store's clock). `request.rs`: `bind_fencing_token`.

**L4 — stale-precondition re-check (`d779671`):**
- NEW `daemon/src/gateway/precondition.rs` — `PreconditionOracle` trait + `NullPreconditionOracle` (2.4 production default → Unchanged). `Gateway::with_precondition` (no-churn). `execute` (after L3, before executor): `recheck` → `Changed` → regenerate the preview (through the §15 gate) → `ActionFailed{StalePrecondition}` (COMMITS) + `Err(StalePrecondition)` → `precondition_stale`.

**L5 — crash-reconcile (`37cfa97`, the capstone):**
- NEW `daemon/src/gateway/recovery.rs` — `reconcile_orphans` (wired into `bootstrap::cold_start` step 7): scans `executing`/`queued` (plan_id-agnostic) → `executing` → `ActionFailed{UnknownOutcome}` + KEEP idempotency_key; `queued` → `ActionFailed{ExecutorError, honest "never ran"}` + CLEAR key (the Q6 dedup contract). `fault.rs` += `BeforeExecutingTxn`/`BeforeTerminalTxn`. NEW §5.1 `Queued→Failed` edge. `eventstore::read_conn`; `BootstrapError::Reconcile`.
- Tests: `daemon/tests/recovery.rs` (12 total: 3 L2 + 3 L3 + 2 L4 + 4 L5) + the L1 contract/`can_transition` tests in `shared/tests/contract.rs` + `daemon/tests/gateway.rs`.

## Decisions made

- **L1 `ActionError` = internally-tagged on `kind`** (ServerFrame precedent) → renders as a `oneOf`, invisible to the §5.0 3-way verify's string-enum comparison (verified structurally — the codegen 3-way couldn't run offline; local byte-diff gate passes).
- **§14 fault hook = a `fault-injection` Cargo FEATURE** (not `cfg(test)` — integration tests link the lib without it) enabled via the **self-dev-dependency idiom**; compiled out + un-armable in release (`nm`/`cargo tree` verified).
- **L2 fail-closed is UNIFORM + risk-agnostic** — a deliberate over-satisfaction of §17's "audit-required = risk≥1" (not an INV-SEC-1 mandate; risk-0 is mutation-free).
- **L3/L4 surface = record-then-throw** (Option B): the `ActionFailed{…}` COMMITS before the typed `Err` propagates → §6.4 code (`fencing_conflict` never-auto-resolved vs `precondition_stale` re-approvable).
- **L4 oracle = consulted for EVERY action**; `Null` default; "fresh approval" = re-submission (no Executing→AwaitingApproval back-edge); fail-on-ANY-change MVP posture.
- **L5 Q6 dedup-key:** queued (never ran) CLEARS the key; executing (maybe-applied) KEEPS it (the double-run hole).
- **Away-authority lead rulings this session:** §6.4 `internal_error`+`fencing_conflict` (Option A; frozen in L1).

## Decisions explicitly NOT made (deferred — orchestrator tracks for the seal)

- The real per-namespace executor / preview / live-source re-reads (git2/octocrab/session) → Phase 3/5/7/8 (2.4 is framework + seams).
- A distinct `daemon_crashed`/`abandoned` `ActionError` variant (clean fix for L5's queued-orphan `executor_error` semantic debt; contract bump, before the UI consumes the audit trail).
- `ActionResult.error → ActionError` (frozen-but-unconsumed type; aligns when first emitted).
- `AuditWriteFailed → internal_error` IPC mapping (the Q7 correction; lands with L5's surface in a later pass — currently still `precondition_stale`).
- Multi-resource fencing (primary `resource_refs[0]` only); the heartbeat-renew LOOP (Phase-3+ long executors); reconcile prompt-lease-release (reaper handles).

## TDD compliance

**Clean.** Every layer ran RED→Step-2.5→GREEN; RED confirmed for the right reason each layer (missing-type compile-fails for contract additions; assertion failures for behavior). No test-after-impl, no safety-critical TDD skip.

## Reachability

- L1 contract: schema bundle + `ActionFailed`/`failed_intent` (ExecutorError live); rollback edges in `can_transition`.
- L2/L3/L4: in `execute()` (all 3 paths — risk-0 auto + approve-single + plan-cascade); the §14 hooks cfg-test-only.
- L5: `reconcile_orphans` in `bootstrap::cold_start` (reachable every daemon start). No tested-but-unwired gaps.

## Open follow-ups

- **Orchestrator hot-writes / seal-ledger (deferred to user seal):** the §7.1 EventTypeRegistry rows (`ActionPartiallySucceeded` + `ActionError`), §6.4 IpcErrorCode rows (`fencing_conflict`/`internal_error`), §17 rows `[IMPLEMENTED 2.4]`, §6.2 stale-precondition note, `daemon/CLAUDE.md` cross-doc rows, CONTRACT 0.19.0; the 2.4 LESSONs.
- **cq deferred (polish, all `[polish-defer]`):** reconcile aborts-on-first-orphan (fail-closed-correct; doc-clarify); `action_idempotency_key` test helper masks setup bugs; the cascade test asserts executing==0 but not each final status; the recovery module-doc independence phrasing.
- **Phase 2.5/2.6:** §18 benchmark → CI gates (incl. the cross-language 3-way verify, which needs network) → `/phase-exit 2`.

## How to use what was built

The §17 failure-mode behaviors are end-to-end in the Gateway `execute()` + `bootstrap::cold_start`. The real adapters (Phase 3/5/7/8) swap their stub executor / `NullPreconditionOracle` / the un-reconcilable-executing-orphan path for real git2/octocrab/session re-reads; the seams + the fail-closed contracts are in place.

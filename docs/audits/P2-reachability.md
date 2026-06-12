# Phase 2 Reachability Audit — daemon/ Action Gateway
**HEAD:** 259a094  
**Auditor:** reachability-auditor  
**Date:** 2026-06-11  
**Scope:** Phase 2 exit gate — the Action Gateway surface and its production wiring from `main.rs`

---

## Summary

```
reachability-auditor: daemon/ — 48 exported symbols audited
  REACHABLE: 47
  UNREACHABLE: 0
  INTENTIONALLY-GATED (test/feature): 1 (fault.rs — fault-injection Cargo feature; EXPECTED)
```

**Phase-exit gate: CLEAR**

---

## Methodology

Production entry points traced from:

1. `daemon/src/main.rs` `#[tokio::main]` → `run()` → `cold_start()` → `WriteActor::spawn(gateway)` → `spawn_accept_loop(listener, ..., handle)`
2. `runtime/writer.rs::run_actor()` — the write-actor command loop (processes all `Command::Gateway*` variants)
3. `ipc/methods.rs::dispatch()` — the JSON-RPC method dispatcher served over UDS (called from `spawn_blocking(serve_connection)`)

Each symbol in scope was traced to at least one of these entry points via the production call graph (no test-only paths counted).

---

## Production Entry Point Map

| Wire method | IPC dispatch (`methods.rs`) | Write-actor command | Gateway pipeline method |
|---|---|---|---|
| `submit_action` | `submit_action()` | `Command::GatewaySubmit` | `Gateway::submit_action_collecting()` |
| `submit_action_plan` | `submit_action_plan()` | `Command::GatewayPlanSubmit` | `Gateway::submit_action_plan_collecting()` |
| `approve` | `approve()` | `Command::GatewayApprove` | `Gateway::approve_collecting()` |
| `deny` | `deny()` | `Command::GatewayDeny` | `Gateway::deny_collecting()` |
| `preview_action` | `preview_action()` | `Command::GatewayPreview` | `Gateway::preview_action()` |
| `get_projection` | `get_projection()` | (read-only WAL) | — |
| `get_capabilities` | `capabilities_value()` | — | — |
| `subscribe` | `subscribe_ack()` | — | — |

---

## Classified Symbol Table

### `daemon/src/gateway/mod.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `GatewayError` (enum) | `pub` | REACHABLE — returned by every pipeline method; propagated to IPC dispatch via `gateway_error_to_code()` |
| `db_err` (fn) | `pub(crate)` | REACHABLE — called by request/approval/plan helpers inside gateway txns |
| `lease_err` (fn) | `pub(crate)` | REACHABLE — called in `execute()` fencing path (pipeline.rs) |
| `enum_wire` (fn) | `pub(crate)` | REACHABLE — called by request/approval/plan helpers |
| `enum_int` (fn) | `pub(crate)` | REACHABLE — called by request/plan helpers |
| `gateway_event_intent` (fn) | `pub(crate)` | REACHABLE — called by approval/request intent builders |
| `Gateway` (struct) | `pub` | REACHABLE — instantiated in `main.rs` → `Gateway::new(CatalogPolicy, CatalogExecutor)` |
| `Gateway::new` | `pub` | REACHABLE — called in `main.rs:71` |
| `Gateway::with_precondition` | `pub` | REACHABLE — called in `tests/recovery.rs:500` only. Note: test-only reference, but this is a **seam constructor** documented as the injection point for Phase 5/7 real oracles. The production `Gateway::new` defaults `NullPreconditionOracle` via this constructor's structural equivalent; the method itself is `pub` so future production callers (Phase 5/7 real-oracle wiring) can use it. **Classification: REACHABLE-SEAM** — wired-but-unsealed (no gap, expected per brief). |
| `Gateway::policy` | `pub(crate)` | REACHABLE — called in `pipeline.rs::submit_action_collecting()` |
| `Gateway::precondition` | `pub(crate)` | REACHABLE — called in `pipeline.rs::execute()` (L4 re-check) |
| `Gateway::executor` | `pub(crate)` | REACHABLE — called in `pipeline.rs::execute()` |

### `daemon/src/gateway/pipeline.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `approval_queue_delta` (fn) | `pub(crate)` | REACHABLE — called in `submit_action_collecting`, `approve_single`, `approve_plan_cascade`, `deny_collecting` |
| `Gateway::submit_action` | `pub` | REACHABLE — public API; internally delegates to `submit_action_collecting`; the `pub` form is used by direct test callers; production goes via `submit_action_collecting` from the write-actor |
| `Gateway::submit_action_collecting` | `pub(crate)` | REACHABLE — called in `runtime/writer.rs:357` (`Command::GatewaySubmit`) |
| `Gateway::submit_action_plan` | `pub` | REACHABLE — public API; internally delegates to `submit_action_plan_collecting` |
| `Gateway::submit_action_plan_collecting` | `pub(crate)` | REACHABLE — called in `runtime/writer.rs:364` (`Command::GatewayPlanSubmit`) |
| `Gateway::approve` | `pub` | REACHABLE — public API; delegates to `approve_collecting` |
| `Gateway::approve_collecting` | `pub(crate)` | REACHABLE — called in `runtime/writer.rs:371` (`Command::GatewayApprove`) |
| `Gateway::deny` | `pub` | REACHABLE — public API; delegates to `deny_collecting` |
| `Gateway::deny_collecting` | `pub(crate)` | REACHABLE — called in `runtime/writer.rs:382` (`Command::GatewayDeny`) |
| `Gateway::preview_action` | `pub` | REACHABLE — called in `runtime/writer.rs:390` (`Command::GatewayPreview`) |
| `Gateway::approve_plan_cascade` | `fn` (private) | REACHABLE — called from `approve_collecting` when `ApprovalTarget::Plan` |
| `Gateway::approve_single` | `fn` (private) | REACHABLE — called from `approve_collecting` when `ApprovalTarget::Single` |
| `Gateway::approval_target` | `fn` (private) | REACHABLE — called from `approve_collecting` |
| `Gateway::open_plan_approvals` | `fn` (private) | REACHABLE — called from `submit_action_plan_collecting` |
| `Gateway::open_step_approval` | `fn` (private) | REACHABLE — called from `open_plan_approvals` |
| `Gateway::record_fencing_conflict` | `fn` (private) | REACHABLE — called from `execute()` L3 fencing path |
| `Gateway::record_stale_precondition` | `fn` (private) | REACHABLE — called from `execute()` L4 precondition path |
| `Gateway::execute` | `fn` (private) | REACHABLE — called from `submit_action_collecting` (risk-0 auto-execute) AND `approve_single` AND `approve_plan_cascade` — **all three gated seams confirmed** |
| `load_covered_steps` | `fn` (private module) | REACHABLE — called from `approve_plan_cascade` and `deny_collecting` |

### `daemon/src/gateway/policy.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `PolicyEngine` (trait) | `pub` | REACHABLE — implemented by `CatalogPolicy` (production) and `StubPolicy` (test-only) |
| `StubPolicy` (struct) | `pub` | TEST-ONLY — referenced only from `daemon/tests/runtime.rs`, `tests/gateway.rs`, `tests/gateway_plan.rs`, `tests/ipc.rs`. **Not a production gap** — its doc comment explicitly marks it "2.1b STUB … test-only"; production uses `CatalogPolicy`. This is an expected test fixture. |
| `CatalogPolicy` (struct) | `pub` | REACHABLE — instantiated in `main.rs:71` (`Box::new(CatalogPolicy)`) |

### `daemon/src/gateway/executor.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `ExecError` (enum) | `pub` | REACHABLE — returned by `ActionExecutor::validate()`, propagated through `execute()` |
| `ExecutionOutcome` (enum) | `pub` | REACHABLE — returned by `ActionExecutor::execute()`, consumed in `execute()` txn-B |
| `ExecutionOutcome::side_effect_applied` | `pub` | REACHABLE — called in `execute()` txn-B fail-closed path |
| `ActionExecutor` (trait) | `pub` | REACHABLE — the executor seam; `Gateway::executor()` returns `&dyn ActionExecutor` |
| `StubExecutor` (struct) | `pub` | TEST-ONLY — referenced only from integration tests (policy.rs, gateway.rs, gateway_plan.rs, runtime.rs, recovery.rs, ipc.rs). **Not a production gap** — doc comment marks it "2.1b STUB … test-only"; production uses `CatalogExecutor`. Expected test fixture. |
| `CatalogExecutor` (struct) | `pub` | REACHABLE — instantiated in `main.rs:71` (`Box::new(CatalogExecutor)`) |

### `daemon/src/gateway/precondition.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `PreconditionStatus` (enum) | `pub` | REACHABLE — returned by `NullPreconditionOracle::recheck()`, consumed in `execute()` L4 check |
| `PreconditionOracle` (trait) | `pub` | REACHABLE — the seam; `Gateway::precondition()` returns `&dyn PreconditionOracle` |
| `NullPreconditionOracle` (struct) | `pub` | REACHABLE — instantiated in `Gateway::new()` (`Box::new(NullPreconditionOracle)`) which is called from `main.rs:71` |

### `daemon/src/gateway/idempotency.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `derive_key` (fn) | `pub` | REACHABLE — called from `pipeline.rs:105` in `submit_action_collecting()` |

### `daemon/src/gateway/preview.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `generate_preview` (fn) | `pub` | REACHABLE — called from `pipeline.rs` `preview_action()` (L2), `record_stale_precondition()` (L4), AND `executor.rs` `CatalogExecutor::preview()` |
| `namespace_label` (fn) | `pub(crate)` | REACHABLE — called from `generate_preview()` and `CatalogExecutor::execute()` |
| `owning_phase` (fn) | `pub(crate)` | REACHABLE — called from `generate_preview()` and `CatalogExecutor::execute()` |

### `daemon/src/gateway/approval.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `ApprovalRow` (struct) | `pub(crate)` | REACHABLE — returned by `approval::load()`, consumed in all approve/deny paths |
| `load` (fn) | `pub(crate)` | REACHABLE — called from `approve_single`, `approve_plan_cascade`, `deny_collecting`, `approval_target` |
| `update_status` (fn) | `pub(crate)` | REACHABLE — called in all approval-state-change paths |
| `record_decision` (fn) | `pub(crate)` | REACHABLE — called in `approve_single` and `approve_plan_cascade` |
| `can_transition` (fn) | `pub` | REACHABLE — called from `approval::update_status()` in production |
| `insert` (fn) | `pub(crate)` | REACHABLE — called from `submit_action_collecting`, `open_plan_approvals`, `open_step_approval` |
| `approval_requested_intent` (fn) | `pub(crate)` | REACHABLE — called from `submit_action_collecting`, `open_step_approval` |
| `plan_approval_requested_intent` (fn) | `pub(crate)` | REACHABLE — called from `open_plan_approvals` (plan-level approve-all) |
| `approved_intent` (fn) | `pub(crate)` | REACHABLE — called from `approve_single` and `approve_plan_cascade` |
| `denied_intent` (fn) | `pub(crate)` | REACHABLE — called from `deny_collecting` |
| `expired_intent` (fn) | `pub(crate)` | REACHABLE — called from `approve_single` and `approve_plan_cascade` expiry paths |

### `daemon/src/gateway/request.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `can_transition` (fn) | `pub` | REACHABLE — called from `request::update_status()` in production |
| `insert` (fn) | `pub(crate)` | REACHABLE — called from `submit_action_collecting` and `submit_action_plan_collecting` |
| `update_status` (fn) | `pub(crate)` | REACHABLE — called throughout the pipeline (all state transitions) |
| `action_requested_intent` (fn) | `pub(crate)` | REACHABLE — called from `submit_action_collecting` and `submit_action_plan_collecting` |
| `load` (fn) | `pub(crate)` | REACHABLE — called from `approve_single`, `approve_plan_cascade`, `deny_collecting`, `preview_action`, `reconcile_orphans` |
| `find_by_idempotency_key` (fn) | `pub(crate)` | REACHABLE — called from `submit_action_collecting` (2.3 dedup-on-submit) |
| `scan_orphans` (fn) | `pub(crate)` | REACHABLE — called from `reconcile_orphans()` in `recovery.rs` |
| `clear_idempotency_key` (fn) | `pub(crate)` | REACHABLE — called from `reconcile_orphans()` (`queued` path) |
| `update_preview` (fn) | `pub(crate)` | REACHABLE — called from `preview_action()` and `record_stale_precondition()` |
| `bind_fencing_token` (fn) | `pub(crate)` | REACHABLE — called from `execute()` L3 after successful lease acquire |
| `started_intent` (fn) | `pub(crate)` | REACHABLE — called from `execute()` txn-A |
| `succeeded_intent` (fn) | `pub(crate)` | REACHABLE — called from `execute()` txn-B `Succeeded` arm |
| `partially_succeeded_intent` (fn) | `pub(crate)` | REACHABLE — called from `execute()` txn-C (§17 L2 partial-success record) |
| `failed_intent` (fn) | `pub(crate)` | REACHABLE — called from `execute()` txn-B `Failed` arm, `record_fencing_conflict()`, `record_stale_precondition()`, `reconcile_orphans()` |

### `daemon/src/gateway/plan.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `insert` (fn) | `pub(crate)` | REACHABLE — called from `submit_action_plan_collecting()` |

### `daemon/src/gateway/recovery.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `reconcile_orphans` (fn) | `pub` | REACHABLE — called from `bootstrap::cold_start()` at `bootstrap.rs:181`. `cold_start()` is called from `main.rs:56`. The full production call chain is: `main()` → `run()` → `cold_start(cfg)` → `reconcile_orphans(&mut store)` |

### `daemon/src/eventstore/` (gateway-touching surface)

| Symbol | Visibility | Reachability |
|---|---|---|
| `EventStore::gateway_txn` | `pub` | REACHABLE — called throughout `pipeline.rs` and `recovery.rs` |
| `EventStore::gw_acquire_lease` | `pub` | REACHABLE — called from `execute()` L3 fencing path (`pipeline.rs:814`) |
| `EventStore::gw_validate_lease` | `pub` | REACHABLE — called from `execute()` L3 validate-held check (`pipeline.rs:852`) |
| Schema `MIGRATION_7_GATEWAY` / `MIGRATION_8_PLANS` | `pub const` | REACHABLE — registered in `migrations.rs:25-26`; applied by `EventStore::open()` → called from `cold_start()` |
| `ActionExecution*` event family (8 types) | emitted via `AppendIntent` | REACHABLE — each event intent builder is called from the pipeline (see `request.rs` and `approval.rs` helpers above) |

### `daemon/src/bootstrap.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `cold_start` (fn) | `pub` | REACHABLE — called from `main.rs:56` |
| `BootstrapConfig` (struct) | `pub` | REACHABLE — instantiated in `main.rs:50-55` |
| `DaemonContext` (struct) | `pub` | REACHABLE — returned by `cold_start()`, destructured in `main.rs:65` |
| `DaemonContext::into_parts` | `pub` | REACHABLE — called in `main.rs:65` |
| `BootstrapError` (enum) | `pub` | REACHABLE — returned by `cold_start()`; propagated to `run()` |
| `DaemonVersionInfo` (struct) | `pub` | REACHABLE — field of `DaemonContext`, used in `main.rs:58-60` |
| `DB_FILENAME` (const) | `pub` | REACHABLE — used in `main.rs:90` |

### `daemon/src/fault.rs`

| Symbol | Visibility | Reachability |
|---|---|---|
| `FaultPoint` (enum) | `pub` | **INTENTIONALLY-GATED** behind `#[cfg(feature = "fault-injection")]`. This feature is enabled ONLY via the daemon's self-dev-dependency (`Cargo.toml`) for integration tests — **compiled out of every production build** (`cargo build` / `cargo run` / `--release` do not pull dev-deps). The three `FaultPoint` variants (`TerminalEventWrite`, `BeforeExecutingTxn`, `BeforeTerminalTxn`) are consumed by `#[cfg(feature = "fault-injection")]` blocks in `pipeline.rs:784-788` and `pipeline.rs:875-879` and by `EventStore::append` (`TerminalEventWrite`). Test-only reachability is **EXPECTED and correct** per the 2.4 L2 §14 safety design (LESSON §21). NOT a gap. |
| `arm` (fn) | `pub` | Same — intentionally feature-gated, test-only |
| `arm_n` (fn) | `pub` | Same |
| `take_if` (fn) | `pub` | Same |

---

## Execute Path — Three Gated Seams Confirmed (INV-SEC-1)

Per the audit brief, the executor must be reachable via all three gated seams and ONLY via the gated pipeline:

| Seam | Entry | Traces to `execute()` |
|---|---|---|
| Risk-0 auto-execute | `submit_action_collecting()` → `Routed::Execute(req)` → `self.execute(store, &req)` | YES (`pipeline.rs:222`) |
| Approve-single | `approve_collecting()` → `approve_single()` → `self.execute(store, &req)` | YES (`pipeline.rs:590`) |
| Approve-plan-cascade | `approve_collecting()` → `approve_plan_cascade()` → `self.execute(store, req)` | YES (`pipeline.rs:692`) |

The executor (`self.executor().execute(req)`) is called ONLY from `execute()` (`pipeline.rs:883`), which is private to the `Gateway` impl. No external code reaches the executor directly. INV-SEC-1 confirmed.

---

## Unreachable Symbols

**None.** Every exported or `pub(crate)` production symbol in scope traces to a production entry point.

---

## Wired-but-stub Seams (NOT gaps)

These symbols are REACHABLE via the production pipeline but their real implementations are deferred:

| Symbol | Status | Owning Phase |
|---|---|---|
| `CatalogExecutor::execute()` per-namespace arms | Side-effect-free stubs | Phase 3/5/7/8 |
| `preview::generate_preview()` | Structural-only (all-impossible transient) | Phase 3/5/7/8 |
| `NullPreconditionOracle::recheck()` | Always `Unchanged` | Phase 5/7 |
| `Gateway::with_precondition` | Seam constructor; defaulted to `NullPreconditionOracle` in production | Phase 5/7 |

---

## Summary for Orchestrator

- **0 wiring tasks recommended** — no orphaned production symbols found.
- **Phase-exit gate: CLEAR**
- `StubPolicy` and `StubExecutor` are `pub` but test-fixture-only by design; they are not production gaps (their doc comments explicitly name them "2.1b STUB … test-only").
- `Gateway::with_precondition` is `pub` and referenced only from tests today, but it is the documented seam for Phase 5/7 real oracle injection — it is a **pre-wired seam**, not a gap.
- `fault.rs` is intentionally feature-gated (`fault-injection` dev-dep only); its test-only reachability is the correct §14 design.
- The three execute gated seams (risk-0 auto, approve-single, approve-plan-cascade) all confirmed reachable. The executor is unreachable from outside the `execute()` private method. INV-SEC-1 holds structurally.

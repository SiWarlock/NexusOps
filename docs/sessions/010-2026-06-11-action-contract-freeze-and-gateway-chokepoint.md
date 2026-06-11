# Session 010 — 2.1a action-contract freeze + 2.1b Gateway chokepoint (single-action)

**Date:** 2026-06-11 · **Track:** daemon · **Implementer close-out (LEAN — context HARD-STOP at 96%; orchestrator carries /preflight + close-out verification at /orchestrate-end).**

## Slices landed (all committed, GREEN at each layer)

| Slice | Commit | What |
|---|---|---|
| **2.1a** action-contract freeze | `a45f2e3` | §6.2 Gateway core data model frozen in `shared/` (10 models + 9 enums + `ApprovalId`/`ActionPlanId` + `Timestamp`); CONTRACT **0.14.0→0.15.0** |
| **2.1b L1** rows + R-9 guards | `1907439` | MIGRATION_7 `action_requests`/`approvals` (DATA_MODEL §2.9; SUPPORTED_USER_VERSION 6→7) + ActionRequest(15)/Approval(10) transition guards |
| **2.1b L2** the chokepoint | `9dbb60a` | `append`→`append_in_txn` extraction (byte-identical) + `gateway_txn`/`GatewayTxn` + `submit_action` pipeline + ActionRequested/ActionApprovalRequested + fail-closed/INV-SEC-1 |
| **2.1b L3** closes 2.1b | `2c83b8f` | approve/deny/preview + StubExecutor (off-txn execute seam) + the 6-event completion family + §15 row-redaction + the full IPC wiring + §14 reachability test; CONTRACT **0.15.0→0.16.0** |

**CONTRACT_VERSION = 0.16.0** (3-way verify PASS, 30 enums @ 0.16.0). **184 workspace tests green; clippy `-D warnings` + fmt clean. security-reviewer PASS on all 3 2.1b layers (no escalation).**

## Files touched (this session)

- **`shared/`:** NEW `src/{actions,time,gateway_ids}.rs` (2.1a); `src/events.rs` (+8 ActionExecution* payloads); `src/ipc.rs` (+`ActionAck`); `src/status.rs` (schemars-rename `ActionRequest`/`Approval`→`…Status` $defs + Denied→terminal); `src/{lib,schema}.rs` (CONTRACT 0.15.0/0.16.0 + ContractBundle); `tests/{contract,envelope}.rs`; `contracts/schema/nexusops-contract.schema.json` (regen).
- **`daemon/`:** NEW `src/gateway/{mod,request,approval,policy,pipeline,executor}.rs`; `src/eventstore/{schema,migrations,mod}.rs` (MIGRATION_7 + `append_in_txn`/`GatewayTxn`/`redact_row` + AppendIntent FK columns); `src/ipc/{methods,server}.rs` (mutation dispatch arms + serve threading); `src/runtime/{writer,listener}.rs` (Gateway commands + `*_blocking` WriteHandle + `disconnected()`); `src/main.rs` (Gateway wiring); `src/bootstrap.rs` + `src/lib.rs`; `tests/{gateway,ipc,runtime}.rs` (+ the 8 AppendIntent test constructors).

## Decisions / rulings (5 — orchestrator routes into Decisions-tabled + memory at /orchestrate-end)

1. **2.1a `$def` collision → Option B (lead, authorized):** schemars keys `$defs` by bare type-name → `actions::ActionRequest`/`Approval` collided with the §5.1 status enums (silently dropped 2 of 10 models). Renamed the status enums' schema to `ActionRequestStatus`/`ApprovalStatus` (their existing Rust aliases; value-sets preserved; 3-way verify name-agnostic). Frozen-`$def`-name reconcile, like `prj_→eprj_`.
2. **2.1a Q2 gateway IDs → Option A:** `ApprovalId`(`appr_`)/`ActionPlanId`(`aplan_`) are non-cross-product newtypes off `GatewayObjectKind` (mirrors `DesktopObjectKind`); `IdKind` (frozen-22) untouched.
3. **Denied → terminal (lead, Option A):** added `Denied` to `ActionRequest` `is_terminal()` + `test_terminal_states_marked` — terminal-by-nature (no forward edge, never executes); the 0.5 freeze omitting it was an oversight. Rust-internal, no wire/CONTRACT change.
4. **§15 inputs-at-rest Finding → Option A (lead):** registry rows aren't event-gated, so an untrusted proposer's secret could persist unredacted. Resolved: `GatewayTxn::redact_row` runs the §15 Redactor over **`inputs_json` + `resource_refs_json`** before the `action_requests` INSERT (no-op for clean/keychain-ref data; masks a secret; fail-closed). General principle: **every caller-supplied registry-row payload column passes the §15 row-redaction gate** (preview_json→2.3). Co-landed with the IPC submit exposure (never live without the guard).
5. **R-9 legal-edge sets** pinned (first time edges, not just values+terminals): orchestrator records in Appendix-A/§5.1.

## Step-9 flags (forward — orchestrator routes; carried for the fresh 2.1c/2.2/2.3/2.4 pair)

- **→ 2.4:** crash-reconciliation + fencing — a crash between the decision txn and the execute completion txn strands an action at `queued` (benign with the no-side-effect stub; real executors need idempotency-key reconcile + the fencing guard, Q6 seam). Marked in `pipeline.rs::execute`.
- **→ §6.4 arch note / 2.2 / CI:** the `IpcErrorCode` 7-code set has **no `internal_error`** — `AuditWriteFailed` (fail-closed/infra) + `UnsupportedPolicyDecision` collapse to `precondition_stale`/`policy_denied` (SAFE — `WireError` carries only `{code}`, no leak; reviewer-confirmed) but a caller can't distinguish "stale" from "the daemon couldn't audit." A §6.4 extension candidate.
- **→ 2.1c:** (a) `expires_at` must be `Z`-suffix UTC at write time (lexical compare; 2.1b stub leaves NULL — noted in `pipeline.rs`). (b) gateway events fold into projections in-band but **don't publish subscribe-deltas** — the `proj_approval_queue` projector + its delta-broadcast land with 2.1c (noted in `runtime/writer.rs`). (c) `submit_action_plan` + `action_plans` table + step-approval + the proj_approval_queue projector body are 2.1c's scope.
- **→ 2.3:** `preview_action` uses a write-txn for a read (uniform access; harmless with the stub; optimize when real previews are slow). The **real executor reads off the in-memory validated inputs, NOT a re-read of the redacted row** (lead's directional lean — a redaction FP must not break a legit execution while the row stays redacted at rest; brushes §7.2 "rows canonical for execution" → reconcile §7.2 wording at 2.3). The 6 typed `ActionPreview` per-class previews + the §6.3 `ActionType` catalog enum (action_type is `String` now) also → 2.3 (catalog → 2.2).
- **Carry-forward (deferred this slice):** the envelope `Timestamp` retrofit (occurred_at/recorded_at→Timestamp + seq min:1) → a small follow-up; the L2 code-quality lows (RequireStepApproval comment, the test `count()` helper, the `source_id` literal, typed-vs-String approval_id).

## Cross-doc the orchestrator writes at /orchestrate-end (NOT touched here — orchestrator territory)

- EventTypeRegistry +8 types (ActionRequested/ApprovalRequested/Approved/Denied/Expired/Started/Succeeded/Failed) + CONTRACT 0.16.0; the §6.1 GatewayPort row flips to **[IMPLEMENTED 2.1b]** (submit/preview/approve/deny live over UDS, peer-auth-first); the §15 row-redaction clarification + a forbidden-pattern; the R-9 edge sets + the Denied-terminal reconcile in Appendix-A/§5.1; LESSON candidate (the chokepoint = events-via-`append`-gate + rows-via-`redact_row`, both §15).

## Next (after the fresh-pair spawn)

2.1c (plans + step-approval + the `proj_approval_queue` projector) — or 2.2 (the policy/risk engine swaps `StubPolicy`). Both unblocked by the 2.1b contract + the in-place stub seams (policy→2.2, executor→2.3, fencing→2.4).

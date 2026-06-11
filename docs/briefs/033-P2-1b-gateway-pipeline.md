# /tdd brief — gateway_pipeline_single_action

## Feature
Build the **single-action Action Gateway pipeline** in the daemon — the INV-SEC-1 mutation chokepoint. `submit_action`/`preview_action`/`approve`/`deny` over the GatewayPort drive the staged pipeline (normalize → resolve → policy → preview → approval → execute → audit) with **stub policy (→2.2) + stub executors (→2.3)**; `action_requests`/`approvals` durable rows; the ActionRequest(15)/Approval(10) transition guards; the authoritative `ActionExecution*` events emitted **ONLY** by the Gateway through the write-actor; fail-closed-on-audit-write. This is the first code path that mutates state — every later executor (P3/P5/P7), the Brain proposal path (P8), and the ui intent seam queue behind this contract.

> **Carve-out → 2.1c** (digestibility split within the lead-endorsed 2.1a/2.1b; sequencing, not scope): `submit_action_plan` + the `action_plans` table + plan fan-out + step-by-step approval (O-3) + the **proj_approval_queue projector body**. 2.1b is the single-action core (N=1); 2.1c is the bundled-plan layer + the Human-Input-Queue read model on top. 2.1b stays end-to-end testable on its own.

## Use case + traceability
- **Task ID:** P2.1b (the daemon-pipeline half of plan task 2.1; 2.1a = the contract freeze ✅ `a45f2e3`)
- **Architecture sections it implements:** `ARCHITECTURE.md §6` / `§6.1` (the GatewayPort mutation methods + the staged pipeline), `§6.2` (consumes the frozen ActionRequest/Approval/ActionResult/PolicyDecision types), `§5.1` (the ActionRequest(15)/Approval(10) transition logic — R-9 legal-edge enforcement), `§15` (**INV-SEC-1 no-bypass** + **fail-closed-on-audit-write**), `§17` (the event-write-fails / fail-closed row; `§7.1` EventTypeRegistry for the new event types).
- **BINDING sources:** `ARCHITECTURE.md` Appendix A (the frozen §6.2 types — landed 2.1a) + `DATA_MODEL sec 2.9` (the `action_requests`/`approvals` DDL, binding) + `AG sec 8` (the lifecycle state machine) + `AG sec 17.1` (the event family) / AG sec 17.2 (ActionEvent payload — maps onto the frozen §7.1 envelope columns + a per-event payload, the `events.rs` pattern). AG is origin/rationale; Appendix A + the DDL bind.
- **Related context:** brief 032 (the 2.1a freeze — the types you consume); LESSONS §3 (write-actor), §4 (in-band projections), §9 (runtime), §10 (System-actor events via the write-actor), §15 (schemars/§5.0 freeze gotchas — the new event types are a contract addition).

## Acceptance criteria (what "done" means)
- [ ] **L1** — `action_requests` + `approvals` migrations match `DATA_MODEL sec 2.9` (the `act_`/`appr_` PKs, the columns, the `ux_action_idem` unique partial index on `idempotency_key`); `SUPPORTED_USER_VERSION` bumped; the ActionRequest(15)/Approval(10) **transition guards** accept every legal edge + reject illegal ones (R-9 — illegal → typed `GatewayError`, never silently applied).
- [ ] **L2 (INV-SEC-1 core)** — `submit_action(ActionRequest) → ActionAck{action_request_id,status}` runs the staged pipeline; **every state transition emits its `ActionExecution*` event via the write-actor append path** (through the §15 redaction gate); `ActionRequested` on submit; **fail-closed:** if the authoritative event for a risk≥1 action can't be written, the action aborts with a typed `GatewayError` (no mutation acknowledged). **The §14 architecture-invariant test holds: no executor is reachable except via the Gateway pipeline, and an event row exists for every mutation.**
- [ ] **L3** — `preview_action(action_request_id) → ActionPreview` (stub preview); `approve(approval_id, edits?, step_id?)` / `deny(approval_id, reason)` drive the Approval lifecycle (`ActionApproved`/`ActionDenied`/`ActionExpired`); on approve the action runs the **stub executor** (no real side effect; records "would-execute") → `ActionStarted` → `ActionSucceeded`; an expired approval → `ActionExpired`, action not executed.
- [ ] The new event types are added to `shared/src/events.rs` (EventTypeRegistry) with `EVENT_TYPE` consts + per-event payloads; **CONTRACT_VERSION 0.15.0 → 0.16.0** (additive) + schema regen + 3-way verify green.
- [ ] All mutation methods are reachable from the real IPC dispatch (`daemon/src/ipc/methods.rs`) — Step 7.5.
- [ ] `/preflight` clean; **security-reviewer on EVERY layer** (INV-SEC-1).

## Wiring / entry point (Step 7.5)
The IPC server's method dispatch (`daemon/src/ipc/methods.rs` — today the §6.1 read surface only) gains the mutation arms: `submit_action`/`preview_action`/`approve`/`deny` → the `gateway::Gateway` pipeline → the write-actor (events) + the durable-row store. The entry point is the live UDS `serve_connection` RPC path (`main.rs` → accept-loop → `methods::dispatch`). **Confirm the gateway pipeline is invoked from the real `methods::dispatch` mutation arms — not just tests.** The write-actor append (LESSON 9) is the sole event-emit path; the Gateway never writes the DB directly except via that path (forbidden #2/#3).

## Files expected to touch
**New:**
- `daemon/src/gateway/{mod,pipeline,request,approval}.rs` — the pipeline + the durable-row store + the transition guards.
- `daemon/src/gateway/executor.rs` — the `ActionExecutor` trait + a **stub** executor (real adapters → 2.3).
- `daemon/tests/gateway.rs` — the pipeline/INV-SEC-1/transition tests.

**Modified:**
- `daemon/src/eventstore/{schema,migrations,mod}.rs` — `action_requests`/`approvals` DDL + version bump.
- `daemon/src/ipc/methods.rs` — wire the 4 mutation methods.
- `shared/src/events.rs` + `shared/src/{lib,schema}.rs` — the new `ActionExecution*` event types + CONTRACT 0.16.0 + ContractBundle + 3-way verify.
- `daemon/src/lib.rs` — `pub mod gateway;`.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2) — layered (each layer = a commit; security-reviewer EVERY layer)
**L1 — durable rows + transition guards** (`daemon/tests/gateway.rs` + `eventstore.rs`):
1. **`test_action_requests_approvals_migrations_match_data_model`** — Asserts: the tables/columns/index match DATA_MODEL sec 2.9 (`act_`/`appr_` PKs, `ux_action_idem`). Why: §6.2/DATA_MODEL binding DDL.
2. **`test_action_request_transition_guard_legal_and_illegal`** — Asserts: legal edges (submitted→previewed→policy_decided→awaiting_approval→approved→queued→executing→succeeded) accepted; illegal (e.g. succeeded→executing, submitted→succeeded) → typed error. Why: §5.1 R-9.
3. **`test_approval_transition_guard`** — Asserts: requested→awaiting_approval→approved/denied/expired legal; post-terminal transition rejected. Why: §5.1 R-9.

**L2 — submit_action + pipeline + ActionExecution events + INV-SEC-1** (security-critical):
4. **`test_submit_action_emits_action_requested_and_persists_row`** — Asserts: submit → `action_requests` row (status per policy-stub) + an `ActionRequested` event via the write-actor. Why: §6.1/§6.2/AG 8.2.
5. **`test_every_mutation_has_an_event_row`** (the INV-SEC-1 pin) — Asserts: for any action that reaches a mutating state, a corresponding `ActionExecution*` event row exists; **no executor is invoked except through the pipeline** (a direct executor call path does not exist / is not reachable — §14 invariant). Why: **§15 INV-SEC-1**.
6. **`test_fail_closed_on_audit_write`** — Asserts: a risk≥1 action whose authoritative event cannot be written → aborts with a typed `GatewayError`, the mutation is NOT acknowledged (no side effect, no partial row). Why: **§15/§17 fail-closed**. (Inject a failing write-actor/event sink, like the 1.1 audit-write-fail test.)
7. **`test_action_requested_payload_redacted`** — Asserts: the event payload passes the §15 redaction gate (the Gateway emits through the same `append` path; no `unredacted` persist). Why: §15 redaction-before-persist.

**L3 — approve/deny/preview + execution completion**:
8. **`test_approve_drives_execute_to_succeeded`** — Asserts: approve → queued→executing→succeeded via the stub executor; `ActionApproved`+`ActionStarted`+`ActionSucceeded` events; `ActionResult{succeeded}`. Why: §6.1/AG 8.8-8.11.
9. **`test_deny_with_reason_terminal`** — Asserts: deny → `ActionDenied`, action terminal, executor NOT invoked. Why: §6.1/AG 8.8.
10. **`test_expired_approval_not_executed`** — Asserts: an approval past `expires_at` → `ActionExpired`, executor NOT invoked (fake clock). Why: §17 / AG 8.8.
11. **`test_preview_action_returns_stub_preview`** — Asserts: `preview_action` returns an `ActionPreview` envelope (stub; real preview classes → 2.3). Why: §6.1/§6.2.

**Contract:** **`test_contract_version_bumped_0_16_0`** + the new event types in the 3-way verify (string-enum/payload rules per LESSON §15).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW event types in EventTypeRegistry (`ActionRequested`, `ActionApprovalRequested`, `ActionApproved`, `ActionDenied`, `ActionExpired`, `ActionStarted`, `ActionSucceeded`, `ActionFailed` — the 2.1b milestone set; see Q1) + their payloads. `GatewayError` (daemon-internal, not a `shared/` contract). The `action_requests`/`approvals` durable rows (daemon-internal tables, not new `shared/` types — the §6.2 models ARE the contract; the rows persist them).
- **Orchestrator doc rows to write hot (Step 9):** the EventTypeRegistry Appendix-A row + the `daemon/CLAUDE.md` cross-doc row gain the new event types + CONTRACT 0.16.0; the §6.1 GatewayPort row flips "mutation methods → Phase 2" to "**[IMPLEMENTED 2.1b]** submit_action/preview_action/approve/deny live." **Escalate-if-safety:** the INV-SEC-1 pin is a §15 invariant — if any test reveals a bypass path, that's a **Finding** (→ human via lead), not a silent fix.
- **§2.5-seam:** the new event types are EventTypeRegistry entries (a §2.5-seam) — the brief's RED includes the contract-version + 3-way-verify tests; per-event payloads are snapshot-class (golden-log tests bind). No new §6.2 model shapes (those froze in 2.1a).

## Things to flag at Step 2.5
1. **Q1 — event granularity.** AG sec 17.1 lists 18 lifecycle events; 2.1b has STUB policy/preview/executor. My default vote: **emit the milestone set** that carries real audit/state meaning in 2.1b — `ActionRequested`, `ActionApprovalRequested`, `ActionApproved`/`ActionDenied`/`ActionExpired`, `ActionStarted`, `ActionSucceeded`/`ActionFailed`. **Defer the stage-detail events** (`ActionNormalized`/`ActionValidated`/`ActionRiskClassified`/`ActionPreviewGenerated`/`ActionQueued`) to the slice that owns the real stage (RiskClassified→2.2, PreviewGenerated→2.3) — emitting them now would carry stub data. `ActionPartiallySucceeded`/`ActionRolledBack*` → 2.4 (no rollback in 2.1b). Flag if you'd rather emit all 18 as a complete audit trail now.
2. **Q2 — stub policy contract.** 2.2 owns the risk engine. My default vote: the L2 policy stub returns **`PolicyDecision{require_approval}` for every action** (conservative — nothing auto-executes without approval until 2.2's risk engine classifies risk-0 as `allow`). This keeps the approval gate live + INV-SEC-1-safe; 2.2 swaps in real risk→decision. Flag the alternative (a trivial risk-0=allow heuristic now).
3. **Q3 — stub executor contract.** My default vote: a `StubExecutor` implementing the `ActionExecutor` trait (validate/preview/execute) that performs **no real side effect**, records a "would-execute" marker, and returns success → driving `ActionStarted`→`ActionSucceeded`. The trait shape is real (2.3 fleshes preview classes + real adapters); the stub lets the lifecycle complete end-to-end + the INV-SEC-1 pin assert "executor only via the pipeline." Flag the trait method set.
4. **Q4 — the ActionExecution event payloads.** Each event's identity (action_request_id, actor, project_id, resource_refs) is on the frozen §7.1 **envelope columns** (the `events.rs`/SessionStarted pattern — NOT duplicated in the payload). My default vote: the payloads carry only the event-specific delta — `ActionRequested{action_type, risk_level, requester_type}`, `ActionApproved{approval_id, decided_by?}`, `ActionDenied{approval_id, reason}`, `ActionSucceeded{}`/`ActionFailed{error}`. Confirm the per-event payload fields at Step-2.5 (golden-log-binding).
5. **Q5 — `GatewayError` taxonomy.** My default vote: a typed daemon-internal enum (`IllegalTransition`, `AuditWriteFailed`, `ApprovalExpired`, `NotFound`, `PolicyDenied`) — daemon-internal (the IPC layer maps it to the `IpcErrorCode` set [sec 6.4], incl. `policy_denied`). Not a `shared/` contract. Flag.
6. **Q6 — same-owner re-acquire / fencing.** 2.1b uses **stub executors (no real side effect)**, so the 1.4 fencing oracle (`validate_held`) + the stale-precondition re-check + crash-reconciliation are **2.4's** concern (real execution). My default vote: 2.1b's pipeline does NOT yet call the fencing oracle (no real mutation to fence) — leave a clearly-marked seam (`// 2.4: validate_held before execute`). Flag if you think the fencing gate must land with the chokepoint.

## Dependencies + sequencing
- **Depends on:** 2.1a ✅ (the frozen §6.2 types) · 1.1 (the write-actor + the §15 redaction gate + the audit-write-fail pattern) · 1.5/1.6 (the IPC dispatch + the runtime).
- **Blocks:** **2.1c** (submit_action_plan + action_plans + step approval + proj_approval_queue projector) · **2.2** (the policy engine swaps the policy stub) · **2.3** (real executors + preview classes swap the executor/preview stubs) · **2.4** (fail-closed/stale-precondition/fencing/crash-reconcile harden the pipeline) · the ui mutation/intent seam (the real `submit_*` surface).

## Estimated commit count
**3 commits — a layered slice, driven layer→layer; EVERY layer is safety-critical (INV-SEC-1) → own commit, security-reviewer each, NEVER bundled with non-safety work:**
- **L1** — durable rows + transition guards (RED #1-3).
- **L2** — submit_action + the staged pipeline + the ActionExecution event family + the INV-SEC-1 fail-closed pin (RED #4-7) — the chokepoint.
- **L3** — approve/deny/preview + execution completion (RED #8-11) + the CONTRACT 0.16.0 / 3-way bump.

(The 2.1c carve keeps this at single-action; if L2's surface still proves too large at Step-2.5, L2 can split intake-vs-execute — decide at Step-2.5.)

## Lessons-logged candidates anticipated
- **Convention candidate** — the Gateway emits its authoritative `ActionExecution*` events ONLY through the write-actor append (the §15 gate + the §14 invariant); no executor is reachable except via the pipeline (the INV-SEC-1 enforcement shape — likely a forbidden-pattern pin).
- **Convention candidate** — staged pipeline with injectable stubs (policy→2.2, executor→2.3, fencing→2.4) so the chokepoint + its events are test-first before the real stages exist (determinism-for-testability, the §14 seam pattern).
- **Architecture-doc note** — the §6.1 GatewayPort row flips to [IMPLEMENTED 2.1b]; the event family + CONTRACT 0.16.0.

## How to invoke
1. **Read this brief end-to-end** — Q1 (event granularity) + Q2/Q3 (the stub contracts) shape the whole pipeline; answer them at Step-2.5 before GREEN.
2. **Run `/tdd gateway_pipeline_single_action`.**
3. **Step 0 (Restate)** — confirm: single-action only (plans → 2.1c); stub policy/executor; the INV-SEC-1 fail-closed pin is the core.
4. **Step 2.5** — send the layered test-design write-up + answers to Q1-Q6. Wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN. **This is the INV-SEC-1 chokepoint — expect a careful review.**
5. **Step 8** — security-reviewer EVERY layer (`invariant` policy; this slice IS the invariant).
6. **Step 9** — surface the event-family payloads, the `GatewayError` taxonomy, the CONTRACT 0.16.0 bump, and any INV-SEC-1 finding.

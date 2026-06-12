# Phase 2 Arch-Drift Audit Report

**Phase:** 2 (Action Gateway, tasks 2.0-SEC through 2.4)
**HEAD:** 259a094
**Audited:** 2026-06-11
**CONTRACT_VERSION at HEAD:** 0.19.0
**Test result:** 199 tests passed (18 suites, 1.41s)

---

## Anchor coverage

| Anchor | Statements checked | Method | Verdict |
|---|---|---|---|
| §5.1 ActionRequest machine (15 states + edges + terminal set) | state values, terminal set, `can_transition` legal/illegal edges, rollback edges, `Queued→Failed` 2.4 edge | Green snapshot: `test_every_state_machine_value_present_and_serializes`, `test_terminal_states_marked`, `test_action_request_transition_guard_legal_and_illegal`, `test_rollback_transition_edges` | VERIFIED |
| §5.1 Approval machine (10 states + edges) | state values, terminal set, `can_transition` legal/illegal edges | Green snapshot: `test_every_state_machine_value_present_and_serializes`, `test_terminal_states_marked`, `test_approval_transition_guard` | VERIFIED |
| §6 Module architecture | All stated modules present: gateway/eventstore/projections/harness/terminal/git/integrations/locks/brainclient/workflow/ipc/policy/usage/notifier/model | Code review via codegraph | VERIFIED |
| §6.1 GatewayPort — 8 methods | `submit_action`, `submit_action_plan`, `preview_action`, `approve`, `deny`, `get_projection`, `subscribe`, `get_capabilities` all dispatched | `daemon/src/ipc/methods.rs:52-62` | VERIFIED |
| §6.2 ActionRequest model field set | 13 fields: action_request_id, project_id, action_type, requester_type, requester_id, resource_refs, inputs, risk_level, idempotency_key, fencing_token, status, preview, created_at | Green snapshot: `test_action_model_field_names_snapshot` (`shared/tests/contract.rs:935`) | VERIFIED |
| §6.2 ActionPlan / ActionPlanStep / ActionDependency model field sets | ActionPlan 6 fields, ActionPlanStep 7 fields, ActionDependency 2 fields | Green snapshot: `test_action_model_field_names_snapshot` + `test_plan_ack_field_names_snapshot` | VERIFIED |
| §6.2 ActionPreview model field set | 7 fields | Green snapshot: `test_action_model_field_names_snapshot` | VERIFIED |
| §6.2 Approval model field set | 10 fields | Green snapshot: `test_action_model_field_names_snapshot` | VERIFIED |
| §6.2 ActionResult model field set (error field = Option<String>) | 7 fields including `error: Option<String>` | Green snapshot: `test_action_model_field_names_snapshot` (`error` key present); note: structured ActionError migration deferred to Phase 3+ (when ActionResult is first emitted — code comments confirm intentional; Appendix A "→2.4" refers to ActionError *type creation*, not ActionResult field migration) | VERIFIED (see STALE-DOC note) |
| §6.2 ResourceRef / EvidenceRef / PolicyDecision model field sets | ResourceRef 3 fields, EvidenceRef 4 fields, PolicyDecision 5 fields | Green snapshot: `test_action_model_field_names_snapshot`, `test_catalog_and_policy_decision_field_snapshot` | VERIFIED |
| §6.2 PolicyDecision.status 6 variants | allow, require_approval, require_step_approval, deny, downgrade, needs_more_context | Green snapshot: `test_closed_enum_wire_values` | VERIFIED |
| §6.2 RiskLevel 0–4 integer serialization | 5 variants, integer 0–4, Ord, fail-closed on out-of-range | Green snapshot: `test_risk_level_wire_is_0_to_4` | VERIFIED |
| §6.2 ResourceType 20 values | AG §9.8 verbatim set | Green snapshot: `test_resource_and_evidence_enum_values` | VERIFIED |
| §6.2 stale-precondition re-check (PreconditionOracle seam, §17) | Seam consulted after lease+fencing, before execute; `NullPreconditionOracle` production default; `Changed` → `ActionFailed{StalePrecondition}` | `daemon/src/gateway/precondition.rs:28-40`, `daemon/tests/recovery.rs:494`; green `stale_fencing_token_fails_closed` and precondition tests | VERIFIED |
| §6.3 ActionTypeCatalog — 22-entry closed set | 22 types enumerated in `MVP_ACTION_TYPES`; all have binding catalog entries; `workflow.command.invoke` is risk-4; fail-closed on unknown types | Green snapshot: `test_action_type_catalog_covers_mvp_set`, `test_catalog_lookup_unknown_type_fails_closed`, `test_catalog_and_policy_decision_field_snapshot` (`shared/tests/contract.rs:1405-1501`) | VERIFIED |
| §6.3 catalog risk assignments (AS-BUILT) | risk-0: brain.ask, project.rescan, workflow.detect, git.status, git.diff, code.open_file; risk-1: session.attach_terminal, session.pause; risk-2: 11 types; risk-3: github.create_pr, review.request_agent_fix; risk-4: workflow.command.invoke | `shared/src/catalog.rs:127-221` — matches spec Appendix A AS-BUILT table | VERIFIED |
| §6.4 IpcErrorCode set (9 codes) | version_skew, frame_too_large, unknown_method, unauthorized_peer, policy_denied, precondition_stale, fencing_conflict, protocol_error, internal_error | Green snapshot: `test_ipc_contract_wire_values` (`shared/tests/contract.rs:481-501`) | VERIFIED |
| §6.4 `fencing_conflict` distinct from `precondition_stale` in dispatch | `GatewayError::FencingConflict` → `IpcErrorCode::FencingConflict`; `GatewayError::StalePrecondition` → `IpcErrorCode::PreconditionStale` | `daemon/src/ipc/methods.rs:128-131` | VERIFIED |
| §6.4 `AuditWriteFailed` → `precondition_stale` (not `internal_error`) | Code maps `GatewayError::AuditWriteFailed` to `IpcErrorCode::PreconditionStale` with explicit "Q7 carry-forward" deferred comment | `daemon/src/ipc/methods.rs:127` — **KNOWN-DEFERRED** per dispatch brief | KNOWN-DEFERRED |
| §7.1 Event envelope required fields | event_id, seq, event_type, event_version, occurred_at, recorded_at, workspace_id, actor_type, actor_id, source_type, source_id, correlation_id, sensitivity, redaction_status, payload_json, schema_version | Green snapshot: `test_schema_artifact_matches_rust` (byte-exact CI gate, `shared/tests/contract.rs:462`) | VERIFIED |
| §7.1 ActorType 10 values (R-2 set) | user, project_brain, action_gateway, workflow_runtime, local_runner, session_adapter, integration_syncer, system, remote_client, automation_policy | Green snapshot: `test_actor_enum_matches_r2` | VERIFIED |
| §7.1 source_type 15 values (closed) | all 15 MVP sources | Frozen in schema artifact, tested via schema-snapshot gate | VERIFIED |
| §7.1 redaction_status = unredacted\|redacted only | Two values; writer never persists `unredacted` | Green snapshot + `test_action_requested_payload_redacted` | VERIFIED |
| §7.1 ActionExecution* 8-type family (2.1b) | ActionRequested, ActionApprovalRequested, ActionApproved, ActionDenied, ActionExpired, ActionStarted, ActionSucceeded, ActionFailed | `shared/src/events.rs`; schema-snapshot gate; `test_every_mutation_has_an_event_row_and_no_auto_execute` confirms events emitted correctly | VERIFIED |
| §7.1 ActionPartiallySucceeded{reason} added 2.4 | New event type with `reason` field, `deny_unknown_fields`, round-trips, `EVENT_TYPE` constant | Green snapshot: `test_action_partially_succeeded_wire_contract`, `test_action_failure_family_field_snapshot` | VERIFIED |
| §7.1 ActionFailed.error String → structured ActionError (2.4) | 5 variants: audit_write_failed, stale_precondition, fencing_conflict, unknown_outcome, executor_error{message}; internally-tagged on `kind` | Green snapshot: `test_action_error_taxonomy_wire_contract`, `test_action_failed_carries_structured_error` | VERIFIED |
| §7.1 CONTRACT_VERSION = 0.19.0 | `nexusops_shared::CONTRACT_VERSION` == "0.19.0" | Green: `test_contract_version_bumped_0_19_0` | VERIFIED |
| §7.2 executor read-source split | auto-execute = in-memory reconciled ActionRequest; approve-path = durable row (`request::load`) | `daemon/src/gateway/pipeline.rs`; `daemon/tests/executor.rs` `§7.2` spy test | VERIFIED |
| §7.2 rows canonical for execution | `action_requests`/`approvals` rows = canonical; `proj_approval_queue` = may-lag read-only projection | `daemon/src/gateway/request.rs`, `daemon/src/projections/` | VERIFIED |
| §15 INV-SEC-1 no-bypass | No code path reaches execution without passing policy+approval+audit event | Green: `test_every_mutation_has_an_event_row_and_no_auto_execute`; security-reviewer CLEAN ×5 (per session logs) | VERIFIED |
| §15 redaction-before-persist for inputs_json/resource_refs_json | Both pass the §15 Redactor before INSERT | Green: `test_submit_redacts_inputs_at_rest`, `test_action_requested_payload_redacted` | VERIFIED |
| §15 event never persists redaction_status='unredacted' | fail-closed: NeverRedacts → AuditWriteFailed, no row | Green: `test_fail_closed_on_audit_write` | VERIFIED |
| §15 idempotency_key = one-way SHA-256 of raw inputs | catalog-derived not requester-supplied; dedup correct; recorded-not-trusted | Green: `daemon/tests/executor.rs:115-325` (idem_key_overrides_requester_supplied + fromInputs + naturalResourceRef + dedup tests) | VERIFIED |
| §17 fail-closed on audit-write (split txn-A/B/C) | txn-A commits Executing+ActionStarted; executor off write-actor; txn-B fail → stays executing; side-effect-applied+txn-B-fail → txn-C ActionPartiallySucceeded | Green: `daemon/tests/recovery.rs:129`, `183`, `225` | VERIFIED |
| §17 fencing-conflict never-auto-resolved | `validate_held` rejects stale/expired token → `ActionFailed{FencingConflict}` → `IpcErrorCode::FencingConflict` | Green: `daemon/tests/recovery.rs:334` (`stale_fencing_token_fails_closed`) | VERIFIED |
| §17 crash-reconcile orphaned actions on restart | `reconcile_orphans` wired in `bootstrap::cold_start`; `queued` → Failed + CLEAR idempotency_key; `executing` → Failed{UnknownOutcome} + KEEP key | Green: `daemon/tests/recovery.rs:589`, `630`, `672`, `716`; wiring at `daemon/src/bootstrap.rs:181` | VERIFIED |
| §17 stale precondition surface | `Changed` → `ActionFailed{StalePrecondition}` COMMITS then `Err` returned → `precondition_stale` IPC code | Green: `daemon/tests/recovery.rs:494` | VERIFIED |

---

## DRIFT findings

None.

---

## STALE-DOC notes (code is right, doc is ambiguous)

**Appendix A `ActionResult.error` field wording.** Appendix A line 556 reads `error?(Option<String> — **structured ActionError→2.4**)`. The `→2.4` notation is ambiguous: it could be read as "was migrated to ActionError in 2.4" but the code correctly defers the type change to when `ActionResult` is first emitted (Phase 3+). The `ActionFailed` *event* type (the actual audit record) does correctly use `ActionError` as of 2.4. The Appendix A row should be clarified to read `error?(Option<String> — structured ActionError→Phase 3+ [when first emitted]; ActionError type created 2.4)`. This is a doc-wording ambiguity, not a code drift. Routes as an Architecture-doc note.

---

## Known-deferred items (confirmed, NOT flagged as drift)

Per dispatch brief:
1. Real per-namespace executor/preview/live-source re-reads (git2/octocrab/session) — Phase 3/5/7/8. Code has `NullPreconditionOracle` + side-effect-free stubs.
2. `daemon_crashed`/`abandoned` ActionError variant for queued-orphan semantic — deferred; code uses `ExecutorError{message}` with honest description. (`daemon/src/gateway/recovery.rs:45-52`)
3. `AuditWriteFailed → internal_error` IPC mapping — currently `precondition_stale`, confirmed at `daemon/src/ipc/methods.rs:127` with explicit "Q7 carry-forward" deferred comment.
4. Multi-resource fencing (primary-only) — architectural deferral, not applicable in Phase 2.

---

## Verdict

**CLEAR** — 0 DRIFT / 1 STALE-DOC (doc-wording only) / 0 ambiguous. All 199 tests green. All schema snapshots byte-exact. All §5.1 state machines, §6.2 models, §6.3 catalog, §6.4 IPC codes, §7.1 event types, §7.2 read-source splits, §15 security invariants, and §17 failure-mode behaviors match the spec anchors as stated (modulo the 4 architecturally-ordered known-deferred items).

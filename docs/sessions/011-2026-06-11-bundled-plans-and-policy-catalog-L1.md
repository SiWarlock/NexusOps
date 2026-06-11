# Session 011 — Phase 2.1c (bundled plans + Human-Input-Queue) COMPLETE + Phase 2.2-L1 (ActionTypeCatalog freeze)

- **Date:** 2026-06-11
- **Phase:** 2 (Action Gateway) — 2.1c COMPLETE; 2.2 IN PROGRESS (L1 landed, **resume at L2**)
- **Track:** daemon (single-track, on `main`)
- **Predecessor:** [010 — action-contract freeze + Gateway chokepoint](010-2026-06-11-action-contract-freeze-and-gateway-chokepoint.md)
- **Successor:** _(next session — the fresh daemon-implementer resuming 2.2-L2)_
- **Close-out reason:** IMPL-ONLY context cycle (impl 71% WARN; orchestrator 53% OK continues; no round seal — 2.2 mid-flight). Clean L1 boundary.

## Why this session existed

Continue Phase 2 on the single-action chokepoint sealed at 2.1b (`0578d60`): build the **O-3 bundled-plan layer + the Human Input Queue read model** (2.1c), then begin the **catalog-driven policy engine** (2.2 — the policy half of the chokepoint, the first no-human-approval execution path). Two dispatched tasks: #3 (2.1c, brief 034) and #4 (2.2, brief 035).

## What was built

### Phase 2.1c — bundled action plans + the approval-queue projector (3 commits, COMPLETE)

**L1 — `c681e54`** — the `proj_approval_queue` projector (Human Input Queue read model) + the approval-queue subscribe-delta.
- **NEW** `daemon/src/projections/approval_queue.rs` — `ApprovalQueueProjector`: folds the Gateway's approval events (`ActionApprovalRequested`→queue row @awaiting_approval; `ActionApproved`/`Denied`/`Expired`→status advance) into `proj_approval_queue`. **Mutable `status` derived from the EVENT TYPE; immutable fields (risk/requester/project/expires_at) sibling-read from `action_requests`/`approvals` in-txn** (rebuild-safe — registry rows aren't in `REBUILD_TABLES`). `sort_key = "{4-risk}_{requested_at}"` (risk-DESC then age-ASC).
- **MOD** `projections/mod.rs` (register), `gateway/pipeline.rs` (`approval_queue_delta` + the `_collecting` method variants — public 2.1b sigs kept stable via thin wrappers), `runtime/writer.rs` (`publish_after_commit` — gateway arms publish the ApprovalQueue Upsert post-commit; Q6). Read-model only — no contract change.

**L2 — `b9e00a1`** — `submit_action_plan(ActionPlan)→PlanAck` + MIGRATION_8 + CONTRACT 0.17.0.
- **NEW** `gateway/plan.rs` (the `action_plans` row insert), `tests/gateway_plan.rs`.
- **MOD** `gateway/{mod,pipeline,approval,request}.rs` (the plan pipeline — ONE atomic gateway txn, whole-plan fail-closed; per-mode approval opening), `eventstore/{schema,migrations}.rs` (**MIGRATION_8**: `action_plans` table + `action_requests.plan_id` FK + `approvals`/`proj_approval_queue` generalized to nullable `action_request_id` + `plan_id`; `SUPPORTED_USER_VERSION` 7→8), `projections/approval_queue.rs` (plan-level fold path), `runtime/writer.rs` (`GatewayPlanSubmit`), `ipc/methods.rs` (dispatch arm), `shared/src/{ipc,schema,lib}.rs` (**PlanAck/PlanStepAck** + CONTRACT 0.17.0).
- **Step-2.5 Mod 1:** `Blocked` mode rejected fail-closed (never phantom awaiting_approval). **Mod 2:** plan-level `ActionApprovalRequested` payload stays `{approval_id}` (`correlation_id=plan_id`; no event-contract change).

**L3 — `bb9b4b0`** — `approve` dispatch (per-step vs plan-level **cascade**) + plan deny/expiry.
- **MOD** `gateway/approval.rs` (`ApprovalRow`/`load` → `action_request_id: Option` + `plan_id`), `gateway/pipeline.rs` (`approve_collecting` dispatch → `approve_single`/`approve_plan_cascade`; `deny_collecting` dispatch; `ApprovalTarget`; `load_covered_steps`), `ipc/methods.rs` (`approve` `step_id?` doc).
- **Critical (risk-4) NEVER cascaded** (`load_covered_steps`: `risk_level <> 4 AND status='awaiting_approval'`); each cascaded step emits its own `ActionApproved`/`Started`/`Succeeded` tied to the step + the shared plan approval_id; deny/expiry never execute.

### Phase 2.2 — L1 only (1 commit; L2/L3 remain)

**L1 — `1b45e9d`** — the §6.3 ActionTypeCatalog contract + PolicyDecision extension + CONTRACT 0.18.0.
- **NEW** `shared/src/catalog.rs` — the **22-entry `ActionTypeCatalog`** + `lookup(action_type)→Option<ActionTypeCatalogEntry>` (fail-closed; None for any non-catalog type) + `PreviewClass`/`ExecutorKind`/`IdempotencyFormula` enums + `ActionTypeCatalogEntry{locked_risk, preview_class, idempotency_formula, executor, requires_resource_refs, params_schema_present}`.
- **MOD** `shared/src/actions.rs` (**PolicyDecision +3 fields**: `required_approvals: Vec<RequiredApprover>`, `constraints: Vec<String>`, `safer_alt: Option<String>` — minimal Q3 shapes), `shared/src/{lib,schema}.rs` (**CONTRACT 0.18.0** + ContractBundle), `shared/contracts/schema/*.json` (regen), `shared/tests/{contract,envelope}.rs`, `daemon/src/gateway/policy.rs` (StubPolicy 3-field fill — test-only).

**AS-BUILT risk table (the orchestrator records this in Appendix-A; LOAD-BEARING):**
| risk | action_types |
|---|---|
| **0** (auto-execute eligible — read/inspect/propose-only) | `brain.ask, project.rescan, workflow.detect, git.status, git.diff, code.open_file` |
| **1** | `session.attach_terminal, session.pause` |
| **2** | `session.create, session.resume, session.send_message, plan.link_task, linear.link_issue, linear.create_issue, git.create_worktree, git.create_branch, github.create_pr_draft, brain.sync, brain.summarize_session` |
| **3** | `github.create_pr, review.request_agent_fix` |
| **4** (critical) | `workflow.command.invoke` ← **lead-ruled** (arbitrary pack-command execution; the §6.3 "cannot be standing-granted"/OQ-WP-5 floor is structurally a risk-4 property) |

Per-type `preview_class`/`executor`/`idempotency_formula`/flags are AS-BUILT in `catalog.rs` (NAMED now; realized 2.3). Security-reviewer **verified the risk-0 set is mutation-free** + `workflow.command.invoke` is the sole risk-4.

## Decisions made

- **2.1c Q3 approval-object model:** StepByStep/Mixed → per-step approvals; ApproveAll → ONE plan-level approval over non-critical + a per-step approval per critical (risk-4). Blocked → reject (Mod 1).
- **2.1c Q6 delta seam:** gateway methods accumulate `ProjectionDelta`s; `run_actor` publishes post-commit (publish-after-commit; Err/rollback publishes nothing).
- **2.2 risk-4 anchor (lead-ruled, away-authority 2026-06-11):** `workflow.command.invoke` = risk-4 (not the brief's risk-3 default) — gives the catalog a real risk-4 entry for the critical-exclusion test + the safety pin; the standing-grant floor falls out of risk-4.
- **2.2 Q4/Q5:** risk-0→allow, 1/2/3→require_approval, 4→require_step_approval, unknown→deny; the recorded `risk_level` is OVERWRITTEN with the catalog risk at submit (one authoritative risk). **(L2/L3 work — not yet implemented.)**
- **2.2 Q6 placement:** catalog table+lookup in `shared/src/catalog.rs`; `CatalogPolicy` in `daemon/src/gateway/catalog.rs` (with `policy.rs`); NOT a separate `daemon/src/policy/`.

## Decisions explicitly NOT made (deferred)

- **2.2 L2 + L3 — NOT STARTED** (the resume point; see below). The Step-2.5 design is APPROVED + recorded — the fresh impl goes straight to L2 RED.
- The catalog's `preview_class`/`executor`/`idempotency_formula` are NAMED only — REALIZED at 2.3 (real previews/executors/dedup).
- The risk-range "resolved by resource state" resolver — no MVP range (single `locked_risk`); deferred to 2.4/P5.
- The `downgrade`-path forward note (lead): when the typed-`input_schema` `workflow.command.invoke` case is built, the PolicyDecision `downgrade` path can contextually lower scrutiny for a BOUNDED typed invocation — but the LOCKED catalog risk stays 4. A later policy slice.

## TDD compliance

**Clean — no violations.** Every layer was test-first (RED confirmed before GREEN): 2.1c L1 (5 projector/delta tests), L2 (7 plan tests), L3 (5 cascade tests, incl. the re-approve guard), 2.2 L1 (4 catalog/contract tests). Security-reviewer PASS on every layer (×4); code-quality fixes folded in-slice each layer.

## Reachability

- **2.1c projector + delta:** `methods::dispatch("submit_action")` → `GatewaySubmit` → in-band `apply_all` → `ApprovalQueueProjector` + `publish_after_commit`. Real IPC path.
- **2.1c submit_action_plan:** `methods::dispatch("submit_action_plan")` → `GatewayPlanSubmit` → `submit_action_plan_collecting`.
- **2.1c approve/deny cascade:** `methods::dispatch("approve"|"deny")` → `Gateway{Approve,Deny}` → `*_collecting` dispatch → cascade.
- **2.2-L1 (contract-freeze layer):** the catalog + extended PolicyDecision are in the published schema bundle + pass the 3-way verify; **`catalog::lookup` is the authority the daemon `CatalogPolicy` consumes in L2** (not yet wired — that IS L2's work, not an unwired gap).

## Open follow-ups

### → RESUME HERE: Phase 2.2-L2 + L3 (brief `docs/briefs/035-P2-2-policy-engine-and-action-catalog.md`)
The Step-2.5 design is **APPROVED + recorded** (below) — go straight to L2 RED.
- **L2 (RED #4-7):** `CatalogPolicy` (NEW `daemon/src/gateway/catalog.rs`) — `PolicyEngine::decide` reading `catalog::lookup` for the AUTHORITATIVE risk (NOT `req.risk_level`): risk-0→allow, 1/2/3→require_approval, 4→require_step_approval, unknown-type→deny; `workflow.command.invoke` null-schema → never `allow`. **Wire into the Gateway** (swap `StubPolicy` in `main.rs`/`bootstrap.rs`; StubPolicy stays test-only). **Reconcile** the recorded `req.risk_level` → catalog risk at submit (the audited `ActionRequested.risk_level` + `action_requests.risk_level` carry the authoritative risk). Tests in NEW `daemon/tests/policy.rs`.
- **L3 (RED #8-10, INV-SEC-1-critical):** the **risk-0 `allow` → `policy_decided→queued→executing→succeeded` auto-execute path** (NO approval row, NO `ActionApprovalRequested`) gated STRICTLY on `PolicyDecision==allow` AND catalog-risk-0 — wire the 2.1b `submit_action` `other => UnsupportedPolicyDecision` arm for `allow`. The §14 invariant extends: the ONLY no-approval execution path is `policy_decided→queued` gated on allow+risk-0. **Safety-pin migration:** `submit_action_plan`'s approve-all critical-exclusion (currently `load_covered_steps` keys off the requester `risk_level`) now keys off the **catalog** risk per step — a step claiming risk-0 whose catalog risk is 4 (`workflow.command.invoke`) is excluded from approve-all. RED #9 (no-non-zero/non-allow auto-queue) + #10 (catalog-critical-exclusion) are the load-bearing pins.
- **Brief hash** `@9768c613`; spec-lint PASS. security-reviewer EVERY layer.

### Cross-doc (orchestrator's `/orchestrate-end`, pending the next round seal — deferred this impl-only cycle)
- Appendix-A `ActionTypeCatalog` row → **[IMPLEMENTED 2.2]** with the AS-BUILT risk table above; `PolicyDecision` row drops the 2.2-deferred markers; the §6.1 `GatewayPort` row flips `submit_action_plan`/`PlanAck` → **[IMPLEMENTED 2.1c]**; DATA_MODEL §2.9 `action_plans` addition + the approvals/proj generalization; **CONTRACT 0.18.0** across §6.1/§6.2/§6.3/§7.1 + `daemon/CLAUDE.md`. (All flagged at Step-9 + acknowledged; the orch hot-wrote `daemon/CLAUDE.md` + LESSON §18 already.)

### Future-TODOs (Carry-forward; consumer-marked)
- **→2.4:** the multi-step cascade crash-recovery (N orphaned `queued` on a mid-loop executor failure — documented in `pipeline.rs`; 2.4's orphaned-`queued` reconciliation must cover the cascade); the `approval_target` peek + chosen-path re-load (fold-into-one optimization).
- **→2.2-L3 (this resume):** the §11.5 critical-exclusion migration onto catalog risk (closes the 2.1c L3 follow-on).
- **→2.3:** `load_covered_steps` N+1 (a `load_bulk` single-query); the zero-covered-steps guard (unreachable in 2.1c single-writer); the catalog `preview_class`/`executor`/`idempotency_formula` realization.
- **Cleanup (minor):** `catalog_enum!` duplicates `actions::wire_enum!` — a future `#[macro_export]`/private-`macros` consolidation. The plan-cascade ack reuses `ActionAck.action_request_id` for the plan-level approval_id (field-reuse; a richer `PlanApproveAck` additive-later).

## How to use what was built

- **The catalog** is the authoritative risk source: `nexusops_shared::catalog::lookup("git.create_worktree")` → `Some(entry{locked_risk: Level2, …})`; `lookup("git.force_push")` → `None` (deferred/unknown — fail-closed). L2's `CatalogPolicy::decide` reads it; never trust `req.risk_level`.
- **The Human Input Queue** (`proj_approval_queue`) is fed automatically (in-band) by any Gateway approval event; the open queue is `WHERE status='awaiting_approval' ORDER BY sort_key`. The ui consumes it via `get_projection("ApprovalQueue")` + the subscribe-delta.

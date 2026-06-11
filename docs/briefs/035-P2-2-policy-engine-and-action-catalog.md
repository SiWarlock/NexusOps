# /tdd brief — catalog_driven_policy_engine

## Feature
Replace the 2.1b risk-blind `StubPolicy` with the **catalog-driven policy engine**: a frozen
**`ActionTypeCatalog`** (per-`action_type`: **locked risk 0-4**, required preview class, idempotency-key
formula, executor binding, params-schema presence, resource_refs-required) in `shared/`, and a
`CatalogPolicy` that resolves each `ActionRequest`'s risk from the **catalog (authoritative)** — NOT
the requester-supplied `risk_level` — and returns the §6.2/sec 12 `PolicyDecision`
(`allow`/`require_approval`/`require_step_approval`/`deny`/`downgrade`/`needs_more_context`). This lights
up the **risk-0 `allow` → `policy_decided → queued` auto-execute path** the 2.1b pipeline currently
rejects (`UnsupportedPolicyDecision`), and **migrates the sec 11.5 approve-all critical-exclusion safety
pin onto the authoritative catalog risk** (so a proposer can't slip a true-risk-4 step into approve-all
by under-claiming risk — the 2.1c L3 security-reviewer follow-on).

> **This is the policy half of the chokepoint — INV-SEC-1-critical.** 2.1b proved the chokepoint
> with a require-approval-for-all stub; 2.2 introduces the FIRST path where an action executes WITHOUT
> a human approval (risk-0 `allow` → auto-queue). That path must be gated **strictly** on
> `PolicyDecision=allow` AND catalog-risk-0 — security-reviewer EVERY layer.

## Use case + traceability
- **Task ID:** P2.2 (the policy engine + risk 0-4 + the per-type action catalog).
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (the MVP `ActionTypeCatalog` — the per-type binding contract; the `workflow.command.invoke` null-`input_schema` approval-floor, OQ-WP-5), `§6.2` (consumes/extends the frozen `PolicyDecision` — the 2.2-deferred `required_approvals`/`constraints`/`safer_alt` fields land here per `shared/src/actions.rs`; the risk-range "resolved by resource state" note), `§5.1` (the `policy_decided → queued` allow-edge already pinned in the 2.1b R-9 guard — now reached), `§15` (INV-SEC-1 — the auto-execute path is gated strictly on `allow`; the sec 11.5 critical-exclusion safety pin migrates to authoritative risk), `§6.1` (the catalog/PolicyDecision ride the GatewayPort contract surface).
- **BINDING sources:** `ARCHITECTURE.md` Appendix A row `ActionTypeCatalog` (§6.3 — per action_type: params schema, locked risk, required preview class, idempotency-key formula, executor, resource_refs required) + the §6.3 MVP action-type list (the ~22 types) + `AG sec 7` (the risk-level definitions 0-4 — the binding risk-assignment anchor) + `AG sec 12` (policy inputs + the `PolicyDecision` result shape) + `AG sec 28.2` (the MVP action set).
- **Related context:** brief 032 (2.1a — the contract-freeze pattern: a §2.5-seam shared/ surface + schemars/§5.0 gotchas, LESSON §15); brief 033/034 (the gateway pipeline + the `StubPolicy`/`StubExecutor` seams 2.2 swaps); `daemon/src/gateway/policy.rs` (the `PolicyEngine` trait + `StubPolicy` you replace); `pipeline.rs::submit_action` (the `other => UnsupportedPolicyDecision` arm 2.2 wires for `allow`); LESSON §14 (binding = Appendix A + DDL; AG is origin), LESSON 16 (the chokepoint — the policy stub seam).

## Acceptance criteria (what "done" means)
**L1 — the `ActionTypeCatalog` contract (shared/ freeze) + the `PolicyDecision` extension + CONTRACT bump:**
- [ ] A frozen `ActionTypeCatalog` in `shared/` — for each of the §6.3 MVP `action_type`s a binding entry: **`locked_risk: RiskLevel`**, `preview_class` (the §6.2 preview-class enum — command/diff/git/api/session/workflow/rollback; the typed previews are 2.3, but the catalog names the class now), `idempotency_formula` (how the idempotency key derives — see Q2), `executor` (the executor-kind binding — a name/enum; real executors are 2.3), `requires_resource_refs: bool`, `params_schema_present: bool` (the `workflow.command.invoke` null-schema floor, §6.3/OQ-WP-5). Closed lookup by `action_type` (unknown type → fail-closed, not a default-allow).
- [ ] `PolicyDecision` extended with the 2.2 fields (`required_approvals`, `constraints`, `safer_alt`) per the `shared/src/actions.rs` DEFERRED markers + AG sec 12.2 — additive (optionals as `null`, `deny_unknown_fields`). The unpinned sub-shapes (`ApprovalRequirement`/`ApprovalConstraint`/the safer-alt union) are defined minimally here (see Q3) or kept deferred with a marker.
- [ ] **CONTRACT_VERSION 0.17.0 → 0.18.0** (additive: the catalog + the PolicyDecision fields) + schema regen + 3-way verify green.
- [ ] **§2.5-seam schema-snapshot** for the `ActionTypeCatalog` (+ the `PolicyDecision` extension) — `spec(§6.3)`/`spec(§6.2)`-tagged; the catalog + PolicyDecision are on the §2.5-seam list.

**L2 — the `CatalogPolicy` engine (catalog-authoritative risk → decision) + wire into the Gateway:**
- [ ] `CatalogPolicy` implements `PolicyEngine::decide` reading the **catalog** for the action's true risk (NOT `req.risk_level` — recorded-not-trusted): risk-0 → `allow`; risk 1-3 → `require_approval` (MVP default-confirm posture, AG sec 7); risk-4 → `require_step_approval` (critical, never broad automation); an `action_type` absent from the catalog → `deny` (fail-closed, not allow); `workflow.command.invoke` with `params_schema_present=false` → approval-floored (never `allow`, §6.3/OQ-WP-5). (The exact per-risk decision mapping is Q4.)
- [ ] Wire `CatalogPolicy` into the `Gateway` (swap the `StubPolicy` default in `main.rs`/bootstrap; `StubPolicy` stays test-only). The `submit_action` pipeline still routes `require_approval`/`require_step_approval` as today.
- [ ] The recorded `req.risk_level` is **reconciled to the catalog risk** at submit (the audited `ActionRequested.risk_level` + the `action_requests.risk_level` row reflect the AUTHORITATIVE risk, not the requester's claim — Q5).

**L3 — the risk-0 `allow` auto-execute path + the sec 11.5 critical-exclusion safety-pin migration** (INV-SEC-1-critical):
- [ ] A risk-0 `allow` action drives **`submitted → policy_decided → queued → executing → succeeded`** via the existing R-9 `policy_decided → queued` edge — **NO approval row, NO human gate** — gated STRICTLY on `PolicyDecision.status == allow` AND catalog-risk-0 (a non-zero risk can NEVER reach the auto-queue path; pin it). Emits `ActionRequested` then the execution family (no `ActionApprovalRequested`). The §14 invariant test extends: the ONLY no-approval execution path is `policy_decided → queued` gated on `allow`+risk-0.
- [ ] **Safety-pin migration:** `submit_action_plan`'s approve-all critical-exclusion (2.1c) now keys off the **catalog-authoritative** risk per step (not the requester-supplied `risk_level`) — a step whose CATALOG risk is 4 is excluded from approve-all + gets its own approval, even if the proposer claimed risk-0. Pin: a plan with a risk-claim-0 / catalog-risk-4 step keeps that step out of approve-all.
- [ ] `/preflight` clean; **security-reviewer EVERY layer** (INV-SEC-1 — the auto-execute path + the safety pin).

## Wiring / entry point (Step 7.5)
`CatalogPolicy` is injected into the `Gateway` at construction (`main.rs`/`bootstrap.rs` — swap `StubPolicy`), so the live `submit_action`/`submit_action_plan` pipeline (reachable from `ipc/methods.rs` dispatch) consults the catalog. The catalog lookup is the policy engine's input; the risk-0 `allow` path lands behind the SAME `methods::dispatch("submit_action")` entry (no new IPC method). **Confirm the real Gateway is constructed with `CatalogPolicy` (not `StubPolicy`) on the production path — `StubPolicy` remains only in tests.** No new GatewayPort method (the catalog + PolicyDecision are contract data, not a new RPC).

## Files expected to touch
**New:**
- `shared/src/catalog.rs` (or `actions.rs` extension) — the `ActionTypeCatalog` + the per-type entries + the preview-class/executor-kind enums.
- `daemon/src/gateway/catalog.rs` — the daemon-side catalog lookup + `CatalogPolicy` (the `PolicyEngine` impl), OR `daemon/src/policy/` (the brief's plan names `daemon/src/policy/` NEW + `gateway/catalog.rs` NEW — confirm placement at Step-2.5).
- `daemon/tests/policy.rs` — the catalog/decision/auto-execute/safety-pin tests.

**Modified:**
- `shared/src/actions.rs` — extend `PolicyDecision` (the 2.2 fields); `shared/src/{lib,schema}.rs` — CONTRACT 0.18.0 + ContractBundle + 3-way verify; `shared/tests/contract.rs` — the §2.5-seam catalog + PolicyDecision snapshots.
- `daemon/src/gateway/policy.rs` — keep `StubPolicy` (test-only) + add `CatalogPolicy`; `gateway/pipeline.rs` — wire the `allow` → queue/execute auto-path (the 2.1b `UnsupportedPolicyDecision` arm) + the plan critical-exclusion on catalog risk; `main.rs`/`bootstrap.rs` — inject `CatalogPolicy`.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2) — layered (each layer = a commit; security-reviewer EVERY layer)
**L1 — catalog contract + PolicyDecision extension + CONTRACT** (`shared/tests/contract.rs` + `daemon/tests/policy.rs`):
1. **`test_action_type_catalog_covers_mvp_set`** — Asserts: every §6.3 MVP `action_type` has a catalog entry with a locked risk 0-4 + preview class + executor + the flags. Why: §6.3 binding.
2. **`test_catalog_lookup_unknown_type_fails_closed`** — Asserts: an `action_type` not in the catalog → a typed not-found, NEVER a default entry. Why: §15 fail-closed.
3. **`test_policy_decision_extension_contract_0_18_0`** — Asserts: `PolicyDecision` has the 2.2 fields; the catalog + PolicyDecision in the published schema + 3-way verify; CONTRACT 0.18.0; the `spec(§6.3)`/`spec(§6.2)` snapshots match. Why: §5.0/§2.5-seam.

**L2 — CatalogPolicy (authoritative risk → decision) + wiring**:
4. **`test_catalog_policy_resolves_risk_from_catalog_not_request`** — Asserts: an action claiming `risk_level=0` whose CATALOG risk is 3 → the decision uses risk-3 (require_approval), NOT the claimed 0. Why: **§15 recorded-not-trusted / the safety-pin root**.
5. **`test_catalog_policy_decision_per_risk`** — Asserts: risk-0 → allow; risk 1-3 → require_approval; risk-4 → require_step_approval; unknown type → deny. Why: §6.3/AG sec 7/sec 12.
6. **`test_workflow_command_invoke_null_schema_approval_floored`** — Asserts: `workflow.command.invoke` with `params_schema_present=false` → require_approval (never allow), regardless of catalog risk. Why: §6.3/OQ-WP-5.
7. **`test_recorded_risk_reconciled_to_catalog`** — Asserts: the persisted `action_requests.risk_level` + the `ActionRequested` event carry the AUTHORITATIVE catalog risk (Q5). Why: audit integrity.

**L3 — the allow auto-execute path + the safety-pin migration** (security-critical):
8. **`test_risk0_allow_auto_executes_without_approval`** — Asserts: a risk-0 `allow` action drives submitted→policy_decided→queued→executing→succeeded with NO approval row + NO `ActionApprovalRequested`; the execution family emits. Why: §6.1/§5.1 allow-path.
9. **`test_nonzero_risk_never_auto_queues`** (the INV-SEC-1 pin) — Asserts: NO action with catalog-risk ≥ 1 (or `PolicyDecision != allow`) ever reaches `queued` without an approval — the `policy_decided → queued` edge is gated strictly on `allow`+risk-0. Why: **§15 INV-SEC-1** (the §14 invariant extends).
10. **`test_approve_all_excludes_catalog_critical`** (the safety-pin migration) — Asserts: a plan step claiming `risk_level=0` whose CATALOG risk is 4 is EXCLUDED from approve-all (gets its own approval), even though the proposer under-claimed. Why: **§6.2/sec 11.5 critical-exclusion on authoritative risk** (the 2.1c L3 follow-on).

**Contract:** `test_contract_version_bumped_0_18_0` + the catalog/PolicyDecision 3-way verify (string-enum/bounded-scalar rules per LESSON §15).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW `ActionTypeCatalog` (§6.3, `shared/`) + the preview-class/executor-kind enums; `PolicyDecision` extended (the 2.2 fields). CONTRACT 0.18.0.
- **Orchestrator doc rows to write hot (Step 9):** the Appendix-A `ActionTypeCatalog` row flips to **[IMPLEMENTED 2.2]** with the AS-BUILT per-type table; the `PolicyDecision` Appendix-A row drops its 2.2-deferred markers; CONTRACT 0.18.0 across the §6.2/§6.3/§7.1 mention rows + the `daemon/CLAUDE.md` cross-doc row. **Escalate-if-safety:** the risk-0 auto-execute path is a NEW INV-SEC-1 surface (first no-human-approval execution) — if any test reveals a non-zero-risk or non-`allow` action reaching auto-queue, that's a **Finding** (→ human via lead), not a silent fix.
- **§2.5-seam:** the `ActionTypeCatalog` (§6.3) + `PolicyDecision` (§6.2) are on the §2.5-seam list → the RED includes their schema-snapshots + the CONTRACT/3-way tests.

## Things to flag at Step 2.5
1. **Q1 — the per-type risk assignments (LOAD-BEARING — review carefully).** Proposed (anchored to AG sec 7): **risk-0** `brain.ask, project.rescan, workflow.detect, git.status, git.diff, code.open_file`; **risk-1** `session.attach_terminal, session.pause`; **risk-2** `session.create, session.resume, session.send_message, plan.link_task, linear.link_issue, git.create_worktree, git.create_branch, github.create_pr_draft, linear.create_issue, brain.sync, brain.summarize_session`; **risk-3** `github.create_pr, review.request_agent_fix, workflow.command.invoke`. The MVP catalog tops out at risk-3 (merge/force-push/delete are AG sec 28.3-deferred) — but the **critical-exclusion + the risk-4 decision mapping must still exist** (for when risk-4 actions land). My default vote: this mapping; flag any you'd move (esp. `workflow.command.invoke` 3-vs-4, `session.resume` 1-vs-2, `brain.ask` 0-vs-1).
2. **Q2 — idempotency-key formula representation.** §6.3 names a per-type "idempotency-key formula." My default vote: a small enum/descriptor (e.g. `None` | `FromInputs(field set)` | `Natural(resource_ref)`) per type — NOT executable code in the catalog; the real key derivation is 2.3 (the dedup store). The catalog just NAMES the formula. Flag if you'd model it differently.
3. **Q3 — the PolicyDecision sub-shapes (`ApprovalRequirement`/`ApprovalConstraint`/safer-alt).** AG never fully pins these. My default vote: define `required_approvals` minimally (e.g. `Vec<RequiredApprover>` reusing the frozen §6.2 type) + keep `constraints` + `safer_alt` as DEFERRED markers (additive-later; the real constraint/safer-alt engine is post-MVP) — OR a minimal `constraints: Vec<String>`. Don't invent rich unpinned shapes. Flag.
4. **Q4 — the risk→decision mapping.** My default vote: risk-0 → `allow`; risk 1-3 → `require_approval`; risk-4 → `require_step_approval` (critical). The `downgrade`/`needs_more_context` statuses exist in the enum but have no MVP trigger → unreached in 2.2 (a later policy slice; marked). `deny` only for unknown-type / explicit-deny inputs (no MVP deny-by-default rule beyond fail-closed). Flag if risk-1 should be `allow` (AG sec 7 says level-1 "usually none or lightweight" confirmation — but defaulting to require_approval is the safe MVP posture; `allow` for risk-1 widens the auto-execute surface — I lean conservative).
5. **Q5 — recorded-risk reconciliation.** The `action_requests.risk_level` + `ActionRequested.risk_level` are written at submit (2.1b) from the REQUESTER's claim. My default vote: 2.2 reconciles them to the CATALOG risk at submit (the audited risk is authoritative, not the claim) — so the audit trail + the queue rank reflect true risk. Flag if you'd keep the claimed risk recorded + add a separate `resolved_risk` (I lean overwrite-with-authoritative: one risk, the true one).
6. **Q6 — catalog placement + the policy module.** The plan names `daemon/src/policy/` (NEW) + `gateway/catalog.rs` (NEW). The catalog DATA is a `shared/` contract (§6.3 §2.5-seam); the daemon-side lookup + `CatalogPolicy` are daemon logic. My default vote: the catalog TABLE in `shared/src/catalog.rs` (contract), the lookup + `CatalogPolicy` in `daemon/src/gateway/catalog.rs` (keep policy with the gateway, matching `policy.rs`), NOT a separate `daemon/src/policy/` module (avoid a thin module). Flag if you prefer the plan's `daemon/src/policy/`.
7. **Q7 — risk ranges "resolved by resource state" (§6.2, e.g. `git.delete_worktree` 3-4).** No MVP catalog action has a true range (delete is deferred). My default vote: model `locked_risk` as a single `RiskLevel` for MVP (no range), with a `// range → resolved-by-resource-state when a range action lands (2.4/P5)` marker — don't build the resource-state resolver now (no consumer). Flag.

## Dependencies + sequencing
- **Depends on:** 2.1c ✅ (the full Gateway pipeline + plans + the `StubPolicy`/`StubExecutor` seams) · 2.1a ✅ (the frozen `PolicyDecision` + `RiskLevel`).
- **Blocks:** 2.3 (the catalog's `preview_class` + `executor` bindings + `idempotency_formula` drive the real executors/previews/dedup) · 2.4 (the risk-range resolver + fencing on the now-executing actions) · `/phase-exit 2` (the §6.3 catalog freeze is a phase-exit gate).

## Estimated commit count
**2-3 commits — a layered slice, driven layer→layer; security-reviewer EVERY layer (INV-SEC-1 — the auto-execute path + the safety pin):**
- **L1** — the `ActionTypeCatalog` contract (shared/) + the `PolicyDecision` extension + CONTRACT 0.18.0 (RED #1-3). A §2.5-seam freeze (the 2.1a pattern).
- **L2** — `CatalogPolicy` (authoritative risk → decision) + wire into the Gateway + the recorded-risk reconciliation (RED #4-7).
- **L3** — the risk-0 `allow` auto-execute path + the sec 11.5 critical-exclusion safety-pin migration (RED #8-10) — the INV-SEC-1-critical layer (own commit; if L2's surface is large, L2 can also split contract-vs-engine — decide at Step-2.5).

**Each layer GREEN + independently shippable** (clean seal points if context tightens). **Never bundle a safety layer with non-safety work** — L3 (the auto-execute path) is its own commit.

## Lessons-logged candidates anticipated
- **Convention candidate** — the policy engine resolves risk from the CATALOG (authoritative), never the requester-supplied `risk_level` (recorded-not-trusted); the recorded risk is reconciled to the catalog at submit (closes the 2.1c L3 safety-pin — the sec 11.5 exclusion + the auto-execute gate both key off the TRUE risk).
- **Convention candidate** — the FIRST no-human-approval execution path (risk-0 `allow` → auto-queue) is gated strictly on `PolicyDecision=allow` AND catalog-risk-0; the §14 invariant extends to pin "no non-zero-risk / non-allow action auto-queues" (the INV-SEC-1 auto-execute boundary).
- **Architecture-doc note** — the §6.3 `ActionTypeCatalog` Appendix-A row → [IMPLEMENTED 2.2] with the AS-BUILT per-type table; the `PolicyDecision` 2.2 fields; CONTRACT 0.18.0.
- **Future TODO — 2.3/2.4** — the catalog's `idempotency_formula`/`preview_class`/`executor` bindings are NAMED now, REALIZED at 2.3 (real previews/executors/dedup); the risk-range resolver + fencing at 2.4.

## How to invoke
1. **Read this brief end-to-end** — Q1 (the per-type risks) + Q4 (the risk→decision mapping) + Q5 (recorded-risk reconciliation) shape the safety surface; answer them at Step-2.5 before GREEN.
2. **Run `/tdd catalog_driven_policy_engine`.**
3. **Step 0 (Restate)** — confirm: the catalog is authoritative for risk (not the requester's claim); risk-0 `allow` is the FIRST no-approval execution path (gated strictly); the safety-pin migrates the sec 11.5 exclusion to catalog risk.
4. **Step 2.5** — send the layered test-design write-up + the per-type risk table (Q1) + answers to Q2-Q7. Wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN. **This is the policy half of the INV-SEC-1 chokepoint + the first auto-execute path — expect a careful review, esp. RED #9 (no-non-zero-auto-queue) + #10 (catalog-critical-exclusion).**
5. **Step 8** — security-reviewer EVERY layer (`invariant` policy).
6. **Step 9** — surface the AS-BUILT catalog table, the PolicyDecision extension, the CONTRACT 0.18.0 bump, the auto-execute gate, and any INV-SEC-1 finding.

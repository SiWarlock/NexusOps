# /tdd brief — approval_queue_risk_policy_freeze

## Feature
The cross-track ②-mini: surface the **authoritative risk + policy decision** on the `proj_approval_queue` read model so the ui's live-mutation approval card shows REAL risk/policy (not fixture data) before a human approves — and **freeze the typed `ApprovalQueueRow`** in `shared/` (the long-owed provisional→generated reconcile; the first projection-row freeze). The Gateway already computes the `PolicyDecision` (`pipeline.rs:154`) but currently **drops** it; this persists it (§15-redacted) on the `approvals` row, the projector sibling-reads it, and the frozen row carries `risk_level` (already present) + `policy_decision`. CONTRACT 0.29.0 → 0.30.0. **Fork B, user-ruled** (the safety-critical approval path deserves a typed contract, not loose-JSON parsing).

## Use case + traceability
- **Task ID:** P4.0b-ui2 (the ②-mini; the 2nd cross-track ui-unblock freeze, the 4.0b-ui1 lineage — gates the ui track's live-mutation transport L2)
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (the `proj_approval_queue` read model), `§6.2` (the frozen `PolicyDecision` it surfaces), `§6.3` (risk classification), `§11.5` (the Human Input Queue approval card), `§15` (redaction-before-persist on the new row payload), `§5.0` (the contract SoT + the §2.5-seam freeze of `ApprovalQueueRow`)
- **Related context:** the approval-queue projector `daemon/src/projections/approval_queue.rs` (sibling-read discipline — IMMUTABLE fields from registry rows, MUTABLE `status` from event type; LESSON §17); `daemon/src/gateway/pipeline.rs:154` (the `decision` computed + dropped); the `approvals` table (`eventstore/schema.rs:358`) + `proj_approval_queue` DDL (`schema.rs:142`); the ui provisional `ApprovalQueueRow` (`ui/src/contracts/provisional.ts:172` — `{approval_id, project_id, status, title?}`, the freeze MATCHES its field names where aligned, pin #1); the §15 dual-gate (LESSON §16 — registry-row payload redacted before INSERT, the `action_requests.inputs_json` 2.1b precedent); LESSONS §14/§15 (freeze discipline + schemars gotchas).
- **Widens phase scope because** this enrichment cites cross-cutting sections (§15 redaction, §11.5 the ui consumer, §5.0 the contract SoT, the §2.5 seam) beyond a single phase's primary anchors — standard for a contract-freeze + projection-enrichment slice.

## Acceptance criteria (what "done" means)

**Commit 1 — the §15-safe persistence + projector population (daemon-internal):**
- [ ] A migration adds `policy_decision_json TEXT` to **`approvals`** (nullable — plan-level rows may carry NULL initially) and `policy_decision_json TEXT` to **`proj_approval_queue`** (nullable). `SUPPORTED_USER_VERSION` bumped; the projection is DROP+CREATE+offset-reset rebuildable (the MIGRATION_8 precedent) OR `ALTER ADD COLUMN` (implementer's call — flag at Step-2.5).
- [ ] The Gateway threads the already-computed `decision` (`pipeline.rs:154`) into `approval::insert` → persists `serde_json(PolicyDecision)` on the `approvals` row, **through the §15 Redactor before INSERT** (rule #3 — the `action_requests.inputs_json` dual-gate precedent, LESSON §16; never an un-redacted persist). Single-action path (`pipeline.rs:174`); the plan-level approve-all path persists NULL or the plan-level decision (Step-2.5 Q).
- [ ] The `ApprovalQueueProjector::open_row` **sibling-reads** `policy_decision_json` from `approvals` (alongside `plan_id`/`expires_at`, `approval_queue.rs:75`) and writes it to `proj_approval_queue.policy_decision_json` — IMMUTABLE field, sibling-read (rebuild-safe, the LESSON §17 discipline; NOT derived from a mutable registry value).
- [ ] `risk_level` is **confirmed already present** (sibling-read + served) — no new persistence needed; the freeze (C2) just types it.
- [ ] Tests in `daemon/tests/projections.rs` (or `gateway*.rs`): the fold writes the policy_decision; rebuild-equivalence preserved; the §15 redaction applies to the persisted payload.

**Commit 2 — freeze the typed `ApprovalQueueRow` + typed serve (CONTRACT 0.30.0):**
- [ ] NEW `shared/src/projections.rs` (the first frozen projection-row module) defines **`ApprovalQueueRow`** carrying the wire columns + `policy_decision: Option<PolicyDecision>` (the frozen §6.2 type, deserialized from the json) + `risk_level: RiskLevel`. `deny_unknown_fields`; optionals-as-null; field names MATCH the ui provisional where aligned (`approval_id`/`project_id`/`status`, pin #1). Which of the 14 columns are wire-contract vs internal (`sort_key`/`updated_at_seq`) = Step-2.5 Q.
- [ ] Registered in `ContractBundle` (`schema.rs`); schema bundle regenerated; `CONTRACT_VERSION` 0.29.0 → 0.30.0.
- [ ] `get_projection` serves the **typed** `ApprovalQueueRow` for the `ApprovalQueue` projection (deserialize the DB row → `ApprovalQueueRow` → serialize) so the served shape is provably the frozen contract — pin #2, no loose-JSON on the human-approval path (vs schema-pinned-opaque-serve = Step-2.5 Q).
- [ ] `shared/tests/contract.rs` snapshot test pins the `ApprovalQueueRow` field-name set + `policy_decision`/`risk_level` types; 3-way verify GREEN @0.30.0.
- [ ] `/preflight` clean; cross-doc invariant updated atomic with the freeze (orchestrator hot-writes).

## Wiring / entry point (Step 7.5)
**Production-reachable both commits.** C1: the `approvals` write is on the LIVE Gateway submit→require-approval path (`pipeline.rs:174`, reached by every `submit_action` that needs approval); the projector fold runs in-band in the event-commit txn (reached on every `ActionApprovalRequested`). C2: `get_projection(ApprovalQueue)` is the LIVE §6.1 read RPC the ui calls. `/wired` the approval-open → policy_decision persisted → projector → served typed row. No deferred caller — this lights up the existing approval path.

## Files expected to touch
**New:**
- `shared/src/projections.rs` — the frozen `ApprovalQueueRow` (C2).
- (tests) `daemon/tests/` additions for the projector fold + redaction; `shared/tests/contract.rs` additions.

**Modified:**
- `daemon/src/eventstore/schema.rs` — the new migration (`approvals` + `proj_approval_queue` columns; `SUPPORTED_USER_VERSION`).
- `daemon/src/gateway/pipeline.rs` — thread `decision` into the approval-open (single-action; plan path per Step-2.5).
- `daemon/src/gateway/approval.rs` (the `approval::insert` helper) — accept + §15-redact + persist `policy_decision_json`.
- `daemon/src/projections/approval_queue.rs` — sibling-read `policy_decision_json` → write the column.
- `daemon/src/ipc/methods.rs` — typed serve for `ApprovalQueue` (C2, per Step-2.5).
- `shared/src/{schema.rs,lib.rs}` — register `ApprovalQueueRow`; `CONTRACT_VERSION` 0.30.0.
- `shared/contracts/schema/*` — regen.
- `daemon/src/eventstore/mod.rs` (or migrations list) — register the new migration.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
**Commit 1** (`daemon/tests/projections.rs` / `gateway*.rs`):
1. **`test_approval_open_persists_policy_decision`** — Asserts: an approval-requiring `submit_action` persists `policy_decision_json` on the `approvals` row == the `decide()` output. Why: §6.2/§11.5 — the authoritative decision is captured at approval-open, not recomputed.
2. **`test_proj_approval_queue_carries_policy_decision`** — Asserts: the projector folds `policy_decision_json` into the row (sibling-read). Why: §7 read model.
3. **`test_policy_decision_persist_is_redacted`** — Asserts: the persisted `policy_decision_json` passes the §15 Redactor before INSERT (a planted secret-shaped string in a `reasons[]`/`constraints[]` value is masked). Why: §15 rule #3 (no un-redacted persist; the dual-gate, LESSON §16).
4. **`test_approval_queue_rebuild_equivalence`** — Asserts: rebuild reproduces `policy_decision_json` byte-identically (sibling-read is rebuild-safe). Why: LESSON §17.

**Commit 2** (`shared/tests/contract.rs`):
5. **`test_approval_queue_row_frozen_shape`** — Asserts: `ApprovalQueueRow` field-name set + `policy_decision: Option<PolicyDecision>` + `risk_level: RiskLevel`. Why: §2.5-seam freeze (LESSON §15).
6. **`test_schema_artifact_matches_rust`** stays green + `CONTRACT_VERSION == "0.30.0"`. Why: §5.0 SoT; 3-way verify @0.30.0.
7. **`test_get_projection_serves_typed_approval_row`** (daemon) — Asserts: `get_projection(ApprovalQueue)` output deserializes as `ApprovalQueueRow` (strict). Why: pin #2 — typed contract on the approval path.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW frozen `ApprovalQueueRow` (`shared/`); new `policy_decision_json` columns (`approvals` + `proj_approval_queue`, daemon-internal); CONTRACT 0.29.0→0.30.0. **§2.5-seam touched → YES** (the snapshot test).
- **Orchestrator doc rows to write hot (Step 9):** the Appendix-A **MVP projections** row (proj_approval_queue now carries policy_decision; the `ApprovalQueueRow` is the first frozen projection-row) + a new Appendix-A row for `ApprovalQueueRow` + the §7/§11.5 note (the approval card consumes the typed risk/policy) + the `daemon/CLAUDE.md` cross-doc rows + CONTRACT 0.30.0. **Safety note:** the §15 redaction-before-persist on the new payload is an invariant-touching change → **security-reviewer runs** (NOT cat-1 — no new mutation path; but the §15 invariant applies); Step-9 confirms `policy_decision` carries risk/policy result, NO secret values (lead pin).
- **Cross-track:** the ui regenerates `ApprovalQueueRow` from 0.30.0 (replaces its 4-field provisional) — the consolidated regen (survival types @0.29.0 + this @0.30.0). The ②-mini lands → the lead issues the ui L2-GO.

## Things to flag at Step 2.5
1. **Migration mechanism.** `ALTER TABLE ADD COLUMN` (cheap, both tables) vs DROP+CREATE+offset-reset for `proj_approval_queue` (the MIGRATION_8 rebuildable-projection precedent). Default: **`ALTER ADD COLUMN`** for both (additive nullable column; the projection re-folds on catch-up anyway; simplest). Confirm no rebuild-list interaction.
2. **Plan-level approve-all `policy_decision`.** The single-action path has a clean 1:1 `decision`. The plan-level approval (`action_request_id` NULL) — persist NULL now, or the plan-level decision? Default: **NULL now** (the ui-L2 card targets single-action live mutations first; the plan-level decision sourcing is a follow-on) — `policy_decision: Option<PolicyDecision>` carries it.
3. **Typed serve vs schema-pinned-opaque.** Default: **typed serve** for `ApprovalQueue` (deserialize→`ApprovalQueueRow`→serialize) per pin #2 (no loose JSON on the approval path). If that's too invasive to `get_projection`'s generic path, fall back to schema-pinned-opaque + a strict snapshot — flag it.
4. **Which columns are wire-contract.** `sort_key`/`updated_at_seq` are projector bookkeeping. Default: **include the user-meaningful fields** (approval_id, action_request_id, project_id, session_id?, agent_team_id?, risk_level, status, requester_type, requester_id, preview_summary?, requested_at, expires_at?, policy_decision?) + **omit** `sort_key`/`updated_at_seq` from the wire row (internal). Confirm the ui doesn't rely on `sort_key` (it derives order from risk/requested_at).
5. **§15 redaction of `policy_decision`.** Route the `approvals.policy_decision_json` write through the SAME redactor the `action_requests.inputs_json` uses (LESSON §16 dual-gate). Default: **yes, redact before INSERT** (rule #3 — even though the PolicyDecision is policy-generated, fail-closed; pin #2's no-secrets confirm is the Step-9 check, the redactor is the mechanism).

## Dependencies + sequencing
- **Depends on:** 2.1c (the `proj_approval_queue` projector + `approvals`/`action_plans` ✅), 2.2 (the `PolicyDecision`+3 fields + `CatalogPolicy` ✅), 4.1a (CONTRACT 0.29.0 — this bumps to 0.30.0 ✅).
- **Blocks:** the **ui track L2** (live-mutation approval card consuming the typed risk/policy) — the lead issues L2-GO when this lands. Slotted **before 4.1b** (lead-ruled — consolidates the ui regen + unblocks L2 sooner).

## Estimated commit count
**2.** (1) the §15-safe persistence + projector population (daemon-internal data flow — migration + pipeline + projector + redaction); (2) the typed `ApprovalQueueRow` freeze + typed serve (CONTRACT 0.30.0). The §15 redaction pin (C1) is invariant-touching → its own commit keeps it bisectable + lets security-reviewer scope cleanly. The freeze (C2) is the contract surface. Separable, both reachable.

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the FIRST frozen projection-row (`ApprovalQueueRow`) establishes the projection-row-freeze pattern (`shared/src/projections.rs`; typed serve for the safety-critical projection); future projection-row reconciles follow it.
- **Convention candidate** — persisting an authoritative policy/decision artifact on the registry row at compute-time (so the projector sibling-reads it rebuild-safely) rather than recomputing in the read model — generalizes LESSON §17.
- **Future TODO** — the plan-level approve-all `policy_decision` sourcing (follow-on); the remaining projection-row provisional→generated freezes (SessionRow/ProjectActivityRow/PullRequestRow/AuditEventRow).

## How to invoke
1. Read this brief end-to-end (esp. Step-2.5).
2. `grep -rn "policy_decision\|approval::insert\|proj_approval_queue" daemon/ shared/` to map the surface, then `/tdd approval_queue_risk_policy_freeze`.
3. Step 0/1 → confirm Feature + files. Step 2.5 → answer the 5 Qs + the coverage map.
4. Step 8 → **security-reviewer runs** (§15 redaction-before-persist on the new payload); code-quality runs (every-slice).
5. Step 9 → surface the cross-doc invariants (the Appendix-A `ApprovalQueueRow` + projections rows + CONTRACT 0.30.0) + **confirm `policy_decision` carries no secrets** (lead pin) for the orchestrator hot-write.

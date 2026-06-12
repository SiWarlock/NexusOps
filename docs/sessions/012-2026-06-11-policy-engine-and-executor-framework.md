# Session 012 — Phase 2.2 (catalog-driven policy engine) COMPLETE + Phase 2.3 (executors/preview/idempotency) COMPLETE

- **Date:** 2026-06-11
- **Phase:** 2 (Action Gateway) — **2.2 COMPLETE** (L2+L3 this session; L1 was `1b45e9d`) · **2.3 COMPLETE** (L1+L2+L3)
- **Track:** daemon (single-track, on `main`)
- **Predecessor:** [011 — bundled plans + policy-catalog L1](011-2026-06-11-bundled-plans-and-policy-catalog-L1.md)
- **Successor:** [013 — the Gateway's §17 safety capstone (2.4 L1–L5)](013-2026-06-11-gateway-17-safety-capstone.md)
- **Close-out reason:** IMPL-ONLY context cycle (lead-rung at ACTION 77%); the orchestrator continues. Clean phase boundary (2.3 complete).

## Why this session existed

Resume Phase 2 at the clean 2.2-L1 boundary (the §6.3 `ActionTypeCatalog` freeze) and drive the **policy half of the chokepoint** to completion, then the **executor/preview/idempotency framework** — the two layers that turn the 2.1b require-approval-for-all stub into a real, risk-classified, catalog-driven Gateway. Two dispatched tasks: #4 (2.2, brief 035) and #5 (2.3, brief 036).

## What was built

### Phase 2.2 — the catalog-driven policy engine (2 commits)

**L2 — `4004052`** — `CatalogPolicy` + production wiring + Q5 recorded-risk reconciliation.
- **MOD** `daemon/src/gateway/policy.rs` — `CatalogPolicy` (`decide` resolves risk from the §6.3 catalog, AUTHORITATIVE — never `req.risk_level`; risk-0→allow, 1/2/3→require_approval, 4→require_step_approval, uncatalogued→deny; the null-schema floor for `workflow.command.invoke`). `StubPolicy` stays test-only.
- **MOD** `gateway/pipeline.rs` (`submit_action_collecting`: reconcile `req.risk_level` → catalog risk before persist [Q5 — the audited `ActionRequested` + the row carry the TRUE risk] + route `deny`→`PolicyDenied`), `main.rs` (swap `StubPolicy`→`CatalogPolicy`).

**L3 — `656844e`** — the risk-0 `allow` auto-execute path + the §11.5 critical-exclusion migration.
- **MOD** `gateway/pipeline.rs` — the FIRST no-human-approval execution path: risk-0 `allow` → `submitted→policy_decided→queued→executing→succeeded` (NO approval, NO `ActionApprovalRequested`), gated STRICTLY on `allow` AND catalog-risk-0 with a defense-in-depth **re-gate** (re-verifies risk-0 even if the policy returns `allow` — pinned by an adversarial `AllowAllPolicy`); the two-phase `Routed` enum (executor runs OFF the write-actor txn). The §11.5 approve-all critical-exclusion migrated onto catalog risk in BOTH the opening (`is_critical` → catalog) AND the cascade (`load_covered_steps` SQL reads the reconciled persisted risk); an uncatalogued plan step rejects the WHOLE plan fail-closed (#11).
- **MOD** `tests/gateway_plan.rs` (3 intent-preserving migration swaps: `git.force_push`→`workflow.command.invoke` ×2, `brain.send`→`brain.summarize_session` — forced by catalog-authoritative + reject-uncatalogued).

### Phase 2.3 — executors / preview / idempotency framework (3 commits)

**L1 — `f4ebeaf`** — catalog-derived idempotency keys + dedup-on-submit.
- **NEW** `daemon/src/gateway/idempotency.rs` — `derive_key(req, entry)` resolves the key from the catalog `IdempotencyFormula` (None / FromInputs / NaturalResourceRef), a **one-way SHA-256 of the RAW inputs** (`idem_`+128-bit hex); the requester-supplied key is IGNORED (recorded-not-trusted). The §15 Option-A precedent is encoded in the module doc.
- **MOD** `gateway/request.rs` (`find_by_idempotency_key` dedup-lookup), `gateway/pipeline.rs` (derive + OVERWRITE the key at the reconcile site + dedup-check before insert → idempotent-replay of the original on a hit; **single-action only** — plan steps stay NULL-keyed), `Cargo.toml` (**+`sha2`**).

**L2 — `c244294`** — the typed preview framework.
- **NEW** `daemon/src/gateway/preview.rs` — `generate_preview` dispatches by `preview_class` → a class-specific summary + changed_resources rendered into the FROZEN flat `ActionPreview`. 2.3 has no namespace adapter, so every preview is structural-only (a documented **2.3 transient**): it sets `cannot_preview_reason` (naming the owning phase) + escalates the **preview** risk (envelope-only — never the policy risk).
- **MOD** `gateway/pipeline.rs` (`preview_action`: generate + persist `preview_json` THROUGH the §15 redaction gate), `gateway/request.rs` (`update_preview`), `ipc/methods.rs` (doc), `tests/gateway.rs` (migrated the 2.1b stub-preview test).

**L3 — `957fe65`** — the `ActionExecutor` framework + `CatalogExecutor` + the §7.2 read-source split.
- **MOD** `gateway/executor.rs` — the trait gains `validate(&req)->Result<(),ExecError>` + `rollback` (fail-closed default); `ExecutionOutcome::Succeeded{changed_resources, detail}` (daemon-internal); NEW **`CatalogExecutor`** (production): a `resolve` helper does the catalog lookup + `requires_resource_refs` check once (shared by validate+execute); `execute` validates-first then dispatches by `ExecutorKind` to **side-effect-free per-namespace stubs**. `StubExecutor` stays test-only.
- **MOD** `gateway/preview.rs` (`namespace_label`/`owning_phase`→pub(crate)), `gateway/pipeline.rs` (`Succeeded{..}` match + §7.2 call-site comments), `main.rs` (**StubExecutor→CatalogExecutor production swap**).

## Decisions made

- **2.2 — risk is catalog-authoritative, never the requester's claim** (recorded-not-trusted, §15); the recorded `action_requests.risk_level` + `ActionRequested` are reconciled to the catalog risk at submit.
- **2.2 — the risk-0 `allow` auto-execute path is gated STRICTLY on `allow` + catalog-risk-0** with a defense-in-depth re-gate (adversarially pinned); §14 extends to "no non-zero / non-allow auto-queue."
- **2.2 — `deny`→`GatewayError::PolicyDenied`** (honest `policy_denied` IPC code, not the misleading `UnsupportedPolicyDecision`).
- **2.3 L1 — idempotency key = one-way SHA-256 of RAW inputs (lead-ruled §15 Option A, away-authority).** A fingerprint, not the secret (rule #4-safe); the keychain_ref convention is the load-bearing control; stable-forever dedup is the feature's purpose (HMAC rejected — a key reset → dedup misses → duplicate execution). **Banked in Decisions-tabled by the orchestrator; flagged for the user's return-review.** Added the `sha2` dep.
- **2.3 L1 — keys are catalog-derived, never requester-supplied** (a proposer-chosen key could force a collision to suppress a victim's action). Dedup-on-hit = idempotent-replay reference; permanent-via-`ux_action_idem` window.
- **2.3 L2 — every 2.3 preview is structural-only (all-impossible-in-2.3)** — no real adapter exists; the escalation is on the preview envelope ONLY (a documented transient). Q4 flat-render (no contract change).
- **2.3 L3 — validate@execute (executor-owned, Q5)** · **§7.2 read-source split** (auto=in-memory raw, approve=durable-row canonical — the split already existed structurally; L3 pins+documents it) · **Q7 `ExecutionOutcome` enrichment** (`changed_resources`+`detail`, daemon-internal) · **`rollback` default is fail-closed `Failed`** (never silently claims success).

## Decisions explicitly NOT made (deferred)

- **Plan-step idempotency** — L1 is single-action only; the plan-atomicity × dedup interaction (a plan step colliding with an in-flight action → whole-plan rejection) needs its own reasoning + test → a later 2.3 sub-step or 2.4.
- **The structured execute-error taxonomy** — `ExecutionOutcome::Failed(String)` stays a string; typed taxonomy → 2.4.
- **Real rollback + the rollback edges** — the default is fail-closed `Failed`; real rollback lands with the executors / 2.4.
- **`ExecutionOutcome.{changed_resources, detail}` → a §6.2 `ActionResult`** — daemon-internal now; 2.4 MAY add an additive `detail`/summary to ActionResult (a 2.4 contract call, NOT implied here).
- **The per-namespace real executor BODIES** (git2/octocrab/session/Brain) — Phase 3/5/7/8 (their modules don't exist yet); 2.3 ships the framework + structured stubs.
- **The strict RFC-8785 JCS canonicalizer** for the idempotency hash (vs serde_json's BTreeMap default) — future hardening; **terminal-state-aware dedup window** (re-run after failed/denied) — future.

## TDD compliance

**Clean — no violations.** Every layer (2.2-L2/L3, 2.3-L1/L2/L3) was test-first: RED confirmed before GREEN each time (L1's mechanism-agnostic RED held at the compile-error level pending the §15 lead ruling), Step-2.5 reviewed by the orchestrator per layer, security-reviewer + code-quality-reviewer on every layer (security CLEAR ×5; all medium/low code-quality findings folded in-slice each layer). Final: **229 workspace tests green; clippy `-D warnings` + fmt clean.**

## Cross-doc invariant audit

**CLEAN.** No `shared/` model FIELD change this session — 2.2-L2/L3 + 2.3 added only daemon-internal types (`CatalogPolicy`, `ExecError`, `ExecutionOutcome` enrichment, `CatalogExecutor`); the §6.3 catalog + `PolicyDecision` were frozen at 2.2-L1 (`1b45e9d`, pre-session). **CONTRACT held 0.18.0 throughout.** The 2.2 cross-doc was sealed mid-session in the orchestrator's `5380ad3` round commit. The 2.3 arch-NOTES (§7.2 read-source split; the §15 idempotency-key Option-A precedent; the Appendix-A `executor`/`idempotency_formula`/`preview_class` → `[REALIZED 2.3]` flips) are hot-routed for the orchestrator's next `/orchestrate-end` — notes, not field changes. All flagged at Step-9 + acknowledged.

## Reachability

- **2.2 CatalogPolicy:** `methods::dispatch("submit_action")` → `submit_action_blocking` → `GatewaySubmit` → `submit_action_collecting` → `CatalogPolicy::decide` (main.rs injection). The risk-0 auto-execute path lands behind the same submit entry; the §11.5 migration via `submit_action_plan`.
- **2.3 L1 idempotency:** derive+dedup in `submit_action_collecting` (the live submit path).
- **2.3 L2 preview:** `methods::dispatch("preview_action")` → `preview_action_blocking` → `Gateway::preview_action` → `generate_preview` + persist.
- **2.3 L3 executor:** `CatalogExecutor` in `main.rs`; `execute` invoked ONLY via the 3 gated Gateway seams (post-approval execute, plan-approve cascade, risk-0 auto-execute) — pinned by `executor_only_reachable_via_gateway`.
- **No tested-but-unwired gaps.**

## Open follow-ups (→ the orchestrator's `/orchestrate-end` + Carry-forward)

**→ Phase 2.4 (the carry-forwards):**
- The **pre-execute gate** — when 2.4 builds the stale-precondition/fencing re-check before execute, decide whether pre-execute failures (validate + stale-precondition) skip `ActionStarted` (a cleaner never-started→Failed semantic). Don't restructure the seam now.
- `ExecutionOutcome.{changed_resources, detail}` → a 2.4 additive §6.2 `ActionResult` (contract call).
- `Failed(String)` → a typed execute-error taxonomy.
- Real `rollback` + the rollback edges (the default is fail-closed `Failed`).
- **Crash-recovery / fencing** (2.1b/2.1c/2.2 carry-forwards): orphaned `queued`/`executing` reconciliation by idempotency key (single + the N-orphan cascade form) + the `validate_held` fencing oracle + the same-owner re-acquire contract; the two-txn delta-suppression corollary.
- **Plan-step idempotency** (the plan-atomicity × dedup interaction).

**Future hardening (no consumer yet):** the strict JCS canonicalizer; the terminal-state-aware dedup window; the preview #8 re-target when real previews land (flagged in-code); the §7.2 real-input-fidelity concern (a redaction FP breaking a real execution — owned when real executors land); `owning_phase` indicative phase labels.

**Away-authority decision flagged for the user's return-review:** the §15 idempotency-key = Option A (one-way SHA-256 of raw inputs) — lead-ruled, banked in Decisions-tabled.

## How to use what was built

- **The policy** is catalog-authoritative: `submit_action` resolves risk from `catalog::lookup`, reconciles the recorded risk, and either auto-executes (risk-0) or opens an approval (1-4). Uncatalogued → `PolicyDenied`.
- **Idempotency**: a keyed re-submit (FromInputs/NaturalResourceRef) replays the original action (at-most-one); None-formula actions never dedup. The key is a one-way hash — never reverse it.
- **Preview**: `preview_action` returns + persists a catalog-class `ActionPreview`; in 2.3 it's structural-only (`cannot_preview_reason` set, preview-risk escalated — envelope-only).
- **The executor**: `CatalogExecutor` validates `requires_resource_refs` then dispatches by `ExecutorKind` to side-effect-free stubs. A namespace's real adapter swaps its stub arm in its owning phase; `rollback` defaults fail-closed.

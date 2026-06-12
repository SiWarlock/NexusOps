# /tdd brief — gateway_executors_preview_idempotency

## Feature
Realize the §6.3 catalog's **named-only** `preview_class` / `executor` / `idempotency_formula` bindings as the Gateway's **executor + preview + idempotency framework**: the `ActionExecutor` trait (validate / preview / execute / optional rollback), the 7 typed preview classes + `cannot_preview_reason`, idempotency-key derivation per the catalog formula, and dedup-on-submit. **Scope boundary (load-bearing): the per-namespace executor *bodies* stay structured stubs** — the real side effects (git2, octocrab, session host, Brain) land in their owning phases (3/5/7/8), whose modules (`git/`, `integrations/`, `harness/`, `brainclient/`) do not exist yet. 2.3 builds the *seam every later executor plugs into*, plus everything deterministic that needs no later-phase dependency.

## Use case + traceability
- **Task ID:** P2.3
- **Architecture sections it implements:** `ARCHITECTURE.md §6.2` (ActionPreview / ActionResult / the executor seam), `§6.3` (the per-type catalog: preview_class / executor / idempotency_formula), `§7.2` (source-of-truth / re-read invariant — the executor read-source; "rows canonical for execution"), `§15` (INV-SEC-1 chokepoint; redaction-before-persist), `§17` (referenced only — stale-precondition / fencing / crash-reconcile are **2.4**, not here)
- **Related context:** brief `033` (2.1b — the single-action chokepoint + the `StubExecutor` this swaps), brief `035` + session `011` (2.2 — the catalog-authoritative policy; the risk-0 `allow` auto-execute path 2.3 must also route through the real executor), `shared/src/catalog.rs` (the `PreviewClass`/`ExecutorKind`/`IdempotencyFormula` enums + `lookup`), `daemon/src/gateway/executor.rs` (the 2.1b stub + its doc-comment seam contract), Carry-forward 2.3 items (the catalog realization; `load_covered_steps` N+1; the §7.2 read-source reconcile; the zero-covered-steps guard).

## Scope boundary — framework vs real side effects (read FIRST)
`git/`, `integrations/`, `harness/`, `brainclient/` are **absent** (Phase 3/5/7/8). So 2.3 **cannot** build real git/GitHub/Linear/session/Brain side effects. The `executor.rs` 2.1b doc-comment ("2.3 owns the real per-action-type adapters") is **aspirational** — reconciled here to phase reality. 2.3's *real, test-first* deliverable is:
- **Idempotency-key derivation** (the 3 catalog formulas) + **dedup-on-submit** — fully deterministic, no later-phase dep.
- **The preview framework** — dispatch by catalog `preview_class`; `cannot_preview_reason` + **risk-escalation when a preview is impossible**; render into the **frozen flat `ActionPreview` envelope** (no contract change). For namespaces whose capability is absent, the class emits a structured "preview unavailable — lands Phase N" with `cannot_preview_reason` set; the *dispatch + escalation logic is real and tested*.
- **The `ActionExecutor` trait shape** (validate / execute / optional rollback) + a `CatalogExecutor` that dispatches `execute` by the catalog `ExecutorKind` to **per-namespace structured stubs** (each returns a typed "not-yet-implemented — Phase N" outcome, **no FS/git/network effect**). `validate` enforces the catalog preconditions (`requires_resource_refs`) — that *is* real, tested behavior.

The acceptance pins below are written so each is testable **without** a real side effect.

## Acceptance criteria (what "done" means)

**L1 — idempotency-key derivation + dedup (submit path):**
- [ ] At submit, the idempotency key is **derived from the catalog `idempotency_formula`**, NOT trusted from the requester (mirrors how 2.2 made risk catalog-authoritative): `None` → no key (never deduped); `FromInputs` → a stable hash over `{action_type, project_id, canonical(inputs)}`; `NaturalResourceRef` → a stable key over `{action_type, sorted resource_ref ids}`.
- [ ] A derived key is **deterministic**: the same logical action submitted twice derives the same key.
- [ ] **Dedup-on-submit**: a second submit whose derived key already exists returns a **reference to the original action** (its `action_request_id` + current status) and creates **NO** second `action_requests` row, **NO** second `ActionRequested` event, **NO** second execution (at-most-one — reuses the existing `ux_action_idem` UNIQUE index; **no new migration**).
- [ ] A `None`-formula action (e.g. `project.rescan`, `git.status`) is **never** deduped — repeat submits each create a fresh action.
- [ ] The derived key persists to `action_requests.idempotency_key` and is recorded on the action's audit trail unchanged.

**L2 — preview framework:**
- [ ] `preview_action` dispatches by the catalog `preview_class` (Command / Diff / Git / Api / Session / Workflow / Rollback) — each produces the typed `ActionPreview` rendered into the **frozen flat envelope** (`summary` + `changed_resources` populated per class; **no new model fields**, CONTRACT stays **0.18.0**).
- [ ] **Preview-impossible escalates**: when a class cannot produce a dry-run (capability absent / unpreviewable action), the preview sets `cannot_preview_reason` AND escalates `risk_level` in the returned preview with a `risk_reasons` entry (§6.2: "preview-impossible escalates risk + sets cannotPreviewReason"). The escalation is on the **preview envelope only** — it does not silently lower the catalog-authoritative policy risk.
- [ ] The generated preview persists to `action_requests.preview_json` (was stub-only).
- [ ] An uncatalogued `action_request_id`'s preview fails closed (the type can't reach a row anyway — submit already denies uncatalogued; assert the `NotFound`/load path).

**L3 — executor framework + dispatch:**
- [ ] The `ActionExecutor` trait carries `validate(&req) -> Result<(), ExecError>` + `execute` + an **optional** `rollback` (default no-op) alongside `preview`.
- [ ] `CatalogExecutor::execute` **dispatches by the catalog `ExecutorKind`** to a per-namespace handler; every handler is a **structured stub** returning a typed "would-execute — real adapter lands Phase N" outcome with **no side effect**.
- [ ] `validate` rejects an action missing a `resource_ref` the catalog marks `requires_resource_refs=true` (fail-closed, before any execute); a `validate` failure surfaces as `ActionFailed` (never a silent skip, never a panic in the write-actor).
- [ ] **§7.2 read-source**: the **risk-0 auto-execute** path runs the executor off the **in-memory reconciled `ActionRequest`** (no redacted re-read); the **approve** path runs off the **durable row** (`request::load`) — which §7.2 deems canonical (in-memory inputs no longer exist at approve-time, possibly post-restart). Documented in code; the real-input-fidelity concern is **owned when real executors land** (later phases), not here.
- [ ] Production wiring swaps `StubExecutor` → `CatalogExecutor` in `bootstrap.rs` + `main.rs`; `StubExecutor` stays **test-only**.
- [ ] **INV-SEC-1 preserved**: the executor is still invoked ONLY from the post-approval `execute` + the gated risk-0 auto-execute paths — `CatalogExecutor` introduces no new reach into FS/git/external (asserted: every namespace handler is side-effect-free in 2.3).

**All layers:**
- [ ] All tests in `daemon/tests/executor.rs` pass (+ existing `gateway.rs`/`gateway_plan.rs`/`policy.rs` stay green).
- [ ] `/preflight` clean (clippy `-D warnings`, fmt, full workspace test).
- [ ] **security-reviewer on every layer** (Gateway / INV-SEC-1 / §15 — the `invariant` policy + the lead's "every Gateway slice" standing rule).
- [ ] No cross-doc model field change under the default (see Step-2.5 Q4) → no CONTRACT bump; if a model field set DOES change, the §2.5-seam snapshot + CONTRACT bump + escalation kick in.

## Wiring / entry point (Step 7.5)
All three layers land behind **existing, live production entry points** — no new IPC surface:
- **L1** wires into `Gateway::submit_action_collecting` (`pipeline.rs`) at the catalog-reconcile site (`pipeline.rs:87-94`, beside the risk reconcile) — derive the key + dedup-check **before** `request::insert`. Reachable via `submit_action`/`submit_action_plan` over UDS.
- **L2** wires into `Gateway::preview_action` (`pipeline.rs:797`) — reachable via the `preview_action` IPC method; the persisted `preview_json` is read back on `request::load`.
- **L3** swaps the injected executor in `bootstrap.rs` + `main.rs` (`Gateway::new(policy, executor)` construction site — the same site 2.2 swapped `StubPolicy`); reachable via `Gateway::execute` from the approve path (`pipeline.rs:555`), the plan-approve cascade (`pipeline.rs:658`), and the risk-0 auto-execute path (`pipeline.rs:192`).

## Files expected to touch
**New:**
- `daemon/src/gateway/preview.rs` — the typed preview-class framework: `generate_preview(req, entry, generated_at) -> ActionPreview` dispatched by `PreviewClass`; `cannot_preview_reason` + risk-escalation.
- `daemon/src/gateway/idempotency.rs` — `derive_key(req, entry) -> Option<String>` per `IdempotencyFormula` + the dedup-lookup helper (or fold into `request.rs` — Step-2.5 Q for placement).
- `daemon/tests/executor.rs` — the 2.3 RED tests (idempotency/dedup + preview + executor dispatch + validate + §7.2 read-source).

**Modified:**
- `daemon/src/gateway/executor.rs` — extend the `ActionExecutor` trait (`validate` + optional `rollback`); add `CatalogExecutor` (dispatch by `ExecutorKind`, per-namespace stubs); keep `StubExecutor` test-only; enrich `ExecutionOutcome` minimally if Q7 accepts.
- `daemon/src/gateway/pipeline.rs` — L1 derive+dedup at submit; L2 real preview + persist `preview_json`; L3 `validate` before execute + the §7.2 read-source comments.
- `daemon/src/gateway/request.rs` — persist/read the derived `idempotency_key` + the real `preview_json`; the dedup-lookup query.
- `daemon/src/gateway/mod.rs` — export `preview` + `idempotency` modules; `CatalogExecutor` is the production default.
- `daemon/src/bootstrap.rs` + `daemon/src/main.rs` — swap `StubExecutor` → `CatalogExecutor`.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2) — `daemon/tests/executor.rs`

**L1 — idempotency + dedup (commit 1):**
1. **`idem_key_from_inputs_is_deterministic`** — Asserts: two `FromInputs` requests with identical `{action_type, project_id, inputs}` derive the SAME key. Why: §6.3 `FromInputs` formula.
2. **`idem_key_natural_resource_ref`** — Asserts: a `NaturalResourceRef` action (e.g. `git.create_branch`) derives a key over `{action_type, sorted resource_ref ids}`; differing resource refs ⇒ differing keys. Why: §6.3 `NaturalResourceRef`.
3. **`idem_formula_none_yields_no_key`** — Asserts: a `None`-formula action (`project.rescan`) derives `None` → its row's `idempotency_key` is NULL. Why: §6.3 `None`.
4. **`duplicate_submit_dedups_to_original`** — Asserts: a second submit with the same derived key returns the ORIGINAL `action_request_id` + status, creates no 2nd row, emits no 2nd `ActionRequested`, runs no 2nd execution. Why: §6.3 idempotency / at-most-one; reuses `ux_action_idem`.
5. **`none_formula_resubmit_creates_fresh`** — Asserts: repeat submits of a `None`-formula action each create a distinct action. Why: only keyed actions dedup.

**L2 — preview framework (commit 2):**
6. **`preview_dispatches_by_catalog_class`** — Asserts: `preview_action` on a `Diff`-class action returns a preview whose `summary`/`changed_resources` reflect the diff class (not the 2.1b stub text). Why: §6.2/§6.3 typed preview classes.
7. **`preview_impossible_sets_reason_and_escalates_risk`** — Asserts: an unpreviewable action returns `cannot_preview_reason=Some(..)` AND an escalated `risk_level` + a `risk_reasons` entry on the preview envelope. Why: §6.2 "preview-impossible escalates risk + sets cannotPreviewReason".
8. **`preview_persists_to_row`** — Asserts: after `preview_action`, `action_requests.preview_json` holds the generated preview (round-trips on `request::load`). Why: §7.2 row is the durable preview source.

**L3 — executor framework + §7.2 (commit 3, INV-SEC-1):**
9. **`catalog_executor_dispatches_by_executor_kind`** — Asserts: `CatalogExecutor::execute` routes a `git.*` action to the Git handler, a `github.*` to the Github handler (stub outcomes), distinct per namespace; no side effect. Why: §6.3 `ExecutorKind`.
10. **`validate_rejects_missing_required_resource_ref`** — Asserts: an action whose catalog entry is `requires_resource_refs=true` but carries no `resource_ref` → `validate` Err → `ActionFailed`, no execute. Why: §6.3 `requires_resource_refs`; fail-closed.
11. **`auto_execute_runs_off_in_memory_inputs`** — Asserts: the risk-0 auto-execute path passes the in-memory reconciled `req` to the executor (NOT a redacted re-read). Why: §7.2 read-source reconcile (Carry-forward 2.1b; the lead's pre-flag).
12. **`approve_path_executes_off_durable_row`** — Asserts: the approve→execute path loads the action from the durable row (`request::load`) and executes off it (§7.2 "rows canonical for execution"); the stub outcome drives `ActionStarted→ActionSucceeded`. Why: §7.2 — durable row is the only source at approve-time.
13. **`executor_only_reachable_via_gateway`** (INV-SEC-1 reach pin) — Asserts: no production path invokes `CatalogExecutor::execute` outside the gated post-approval / risk-0 auto paths. Why: §15 INV-SEC-1 (the executor is not a new bypass).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** under the default (Q4) — typed previews render into the frozen flat `ActionPreview`; idempotency reuses the existing column + index; `ExecutionOutcome` is a daemon-internal enum (Q7). → **no `shared/` schema change, CONTRACT stays 0.18.0.**
- **Orchestrator doc rows to write hot (Step 9 routing):** the Appendix-A `ActionTypeCatalog` row note flips `preview_class`/`executor`/`idempotency_formula` from "NAMED, realized 2.3" → "**[REALIZED 2.3]** (framework; per-namespace executor bodies are structured stubs → real adapters land Phase 3/5/7/8)"; the §6.2 `ActionPreview` row note records the realized preview-class semantics; a §7.2 arch-note candidate on the executor read-source split (auto=in-memory, approve=durable-row). No invariant **field** change.
- **§2.5-seam (shared-contract) model touched?** **No** under the default — `ActionPreview` is NOT in the §2.5-seam list (`daemon/CLAUDE.md` line 138: ActionRequest/ActionPlan/Approval/ActionResult), and the catalog enums' field sets are unchanged. **If** Q4 adopts a typed `preview_body` (a model field change), THEN the snapshot test + CONTRACT bump + a **load-bearing-contract escalation** are required — flag before GREEN, do not adopt silently.

## Things to flag at Step 2.5
1. **Idempotency derivation recipe.** For `FromInputs`: `hash(action_type ∥ project_id ∥ canonical-JSON(inputs))` (canonicalized: sorted keys, no whitespace). For `NaturalResourceRef`: `action_type ∥ sorted(resource_ref ids)`. My default vote: **the recipe above, with a stable hash (e.g. SHA-256 hex, truncated) + a short `idem_` prefix** — deterministic, collision-resistant, human-greppable. Confirm the canonicalization (a non-canonical JSON hash would make logically-identical inputs derive different keys).
2. **Dedup behavior on a hit.** (a) Return a reference to the original action (idempotent replay — at-most-one), or (b) reject with a typed `DuplicateAction` error. My default vote: **(a) idempotent-replay reference** — matches the §6.3 "at-most-one execution" intent + the worked-example "same key ⇒ at-most-one charge"; a hard reject would make a legit retry-after-timeout fail.
3. **Dedup window.** The existing `ux_action_idem` UNIQUE index dedups **permanently** (any two actions sharing a key collide forever). My default vote: **permanent-via-existing-index for MVP** — the keyed actions (`FromInputs`/`NaturalResourceRef`) are precisely the mutating ones where re-execution is unsafe; `None`-formula reads/proposals are never keyed and re-run freely. Note a terminal-state-aware refinement (allow re-run after a terminal `failed`/`denied`) as a future TODO if permanent proves too strong.
4. **Preview envelope shape (contract-surface).** (a) Render typed previews into the **frozen flat `ActionPreview`** (`summary` + `changed_resources`; no new fields; no CONTRACT bump), or (b) add a typed `preview_body` discriminated union per class. My default vote: **(a) flat-render** — the model is frozen flat, the ui consumes it stably, and "define minimally / additive-later" matches 2.1c `PolicyDecision` + `ActionPlan.rollback_plan`. **(b) is a §2.5-seam contract change → escalate** (snapshot test + CONTRACT bump + load-bearing-contract call); only take it if a class genuinely can't render into the flat envelope.
5. **`validate` placement.** (a) `validate` runs as the first step of `CatalogExecutor::execute` (executor-owned precondition), or (b) a structural `requires_resource_refs` check moves to submit-time (reject earlier). My default vote: **(a) at execute, executor-owned** — keeps the precondition with the adapter that needs it and the §17 stale-precondition re-check (2.4) co-locates there; a submit-time structural pre-check is an additive hardening, not 2.3-blocking.
6. **§7.2 read-source split (the lead's pre-flagged nuance).** Auto-execute uses the in-memory reconciled `req`; the approve path uses the durable (redacted) row — canonical per §7.2, since in-memory inputs are gone at approve-time. My default vote: **keep the split + document the §7.2 reconcile in code + an arch-note**; the redacted-input-fidelity risk is **theoretical in 2.3** (executors are stubs / no side effect) and is **owned when real executors land** (§15 design already routes secrets to the keychain, not `inputs_json`; 2.0-SEC measured FP-rate 0.0). I assessed this as **non-load-bearing for 2.3** (self-resolves) — raise to the orchestrator only if the impl finds the approve path needs un-redacted inputs for a 2.3 stub (it shouldn't).
7. **`ExecutionOutcome` enrichment.** Keep the 2.1b `Succeeded` / `Failed(String)`, or enrich with `changed_resources` (feeding a future `ActionResult`). My default vote: **enrich minimally** — add `changed_resources: Vec<ResourceRef>` to the `Succeeded` arm (daemon-internal; symmetrical with the preview), but keep the **structured error taxonomy deferred to 2.4** (`Failed(String)` stays). Flag if enrichment balloons the slice.

## Dependencies + sequencing
- **Depends on:** 2.1b (the pipeline + `StubExecutor` seam ✅), 2.1c (the plan-approve cascade execute path ✅), 2.2 (the catalog-authoritative policy + the risk-0 auto-execute path + `catalog::lookup` ✅).
- **Blocks:** 2.4 (stale-precondition re-check + fencing + crash-reconcile build ON the `validate`/execute seam this lands); Phase 3/5/7/8 (each swaps its namespace's structured stub for the real adapter behind this trait); the ui's preview/permission surface (consumes the realized `ActionPreview`).

## Estimated commit count
**3** — one per layer, each its OWN commit (Gateway / INV-SEC-1 surface → never bundled across the safety seam; matches the 2.1c/2.2 layer→layer cadence):
- **L1** idempotency derivation + dedup-on-submit (submit-path; reuses the existing index — no migration).
- **L2** the preview framework (dispatch + cannot_preview_reason + risk-escalation + persist).
- **L3** the executor trait + `CatalogExecutor` dispatch + `validate` + the §7.2 read-source split + production swap (the INV-SEC-1-critical layer).

Drive **layer→layer** (the impl idles after each layer commit; the orchestrator wakes it L1→L2→L3). security-reviewer **every** layer.

## Lessons-logged candidates anticipated
- **Convention candidate** — "Idempotency keys are catalog-derived, never requester-supplied — the same recorded-not-trusted posture as risk (2.2). A `None`-formula action is never deduped."
- **Convention candidate** — "2.3 realizes the catalog's *framework*; per-namespace executor *bodies* are structured stubs until their owning phase — the trait is the seam, not the side effect."
- **Architecture-doc note candidate** — the §7.2 executor read-source split (auto=in-memory reconciled inputs; approve=durable row canonical) + the preview-impossible risk-escalation-on-the-envelope-only rule.
- **Future TODO — operational** — `load_covered_steps` N+1 → `load_bulk` (Carry-forward 2.3); terminal-state-aware dedup window (Q3); typed `preview_body` if a consumer needs class-specific structure (Q4).

## How to invoke
1. **Read this brief end-to-end** — especially the Scope boundary + Step-2.5 questions (7 of them; a framework slice has real design surface — take defaults or ping back per layer).
2. **Run `/tdd gateway_executors_preview_idempotency`** in the implementer session.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line + the framework-not-real-side-effects scope boundary.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — send the test-design write-up per layer (assert-line + coverage map); answer Q1–Q7 (defaults OK). Wait for `APPROVED.`/`TWEAK:`/`ADD:` before Step 4.
6. **Step 9** — surface anything outside the anticipated lessons-logged candidates; the orchestrator routes hot + authors the commit message.

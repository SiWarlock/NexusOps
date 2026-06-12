# Session 017 — Claude `MutationIntercept`→Gateway interception (P3.2 part 2, brief 043)

- **Date:** 2026-06-12
- **Phase:** 3 (Harness adapters & embedded terminal) — task **3.2 part 2** (the INV-SEC-1 interception)
- **Predecessor:** [016 — Terminal Channel + Claude adapter observe path](016-2026-06-12-terminal-channel-and-claude-adapter-observe.md)
- **Successor:** _(next session)_
- **Brief:** `docs/briefs/043-P3-2-claude-mutation-intercept-gateway.md` (spec-lint PASS `@6455090d`)
- **Round commits:** L1 `9f228a6` · L2 `dfbf0aa` · L3 `73d7f79` · L4 `81041c1` · L5 `1a276b7` (L5 amended to fold the intercept.rs comment fix)

## Why this session existed

Phase 3's most INV-SEC-1-load-bearing slice: the **first real harness mutation chokepoint**. A supervised Claude session's `PreToolUse` hook → a daemon hook-receiver → a `MutationIntercept` routed through the **existing Action Gateway** as a typed, risk-classified, **adjudication-only** `ActionRequest` → policy/approval → an **audit-written-before-verdict** `MutationVerdict{Allow|Deny}`. The agent runs the allowed tool; the daemon only **adjudicates + audits** (it never executes the tool). cat-1 safety; the lead is looped as a second gate.

## What was built (5 layers, RED→2.5→GREEN each, security-reviewer every layer)

### Files created
- `daemon/src/harness/claude/intercept.rs` — the hook-receiver + routing: `HookPayload` + `parse_payload` + `map_to_action_type` (matrix-grounded, fail-closed) + `classify_tool` + `disposition`/`Disposition` + `DenyReason` + `route_intercept` + `verdict_for_status` + `InterceptOutcome` + `deny_verdict`.
- `daemon/tests/claude_intercept.rs` — 18 tests across L2–L5 (ingress · routing/adjudication verdict · audit-before-verdict · coverage-matrix disposition · deny-rules · launch compensation · record-then-deny).

### Files modified
- `shared/src/catalog.rs` (L1) — `ExecutorKind::Adjudication` + a separate `AGENT_MUTATION_ACTION_TYPES` const (`agent.bash`/`agent.file_edit`/`agent.file_read`/`agent.mcp_tool`) + 4 `lookup` arms (read-only=risk-0, mutating=risk-2; all adjudication-only). MVP-22 set untouched.
- `shared/src/events.rs` (L5) — `ActionDenied.approval_id` `String`→`Option<String>` (`skip_serializing_if`): a human-deny carries `Some(appr_…)`, a policy-deny `None`.
- `shared/src/lib.rs` — `CONTRACT_VERSION` 0.21.0→**0.22.0** (L1) →**0.23.0** (L5).
- `shared/contracts/schema/nexusops-contract.schema.json` — regenerated twice (0.22.0, 0.23.0).
- `shared/tests/contract.rs` — L1 agent-mutation catalog/adjudication-class/snapshot tests; L5 `ActionDenied` optional test; the version pin.
- `daemon/src/harness/claude/mod.rs` (L2/L4) — `pub mod intercept`; `ClaudeSettings` gains `permissions.deny:[mcp__*,Task]` + empty `allow`/`ask` + a 5s hook timeout (the cheap-closure) + accessors.
- `daemon/src/gateway/policy.rs` (L4) — `AgentMutationPolicy` (wraps `CatalogPolicy`) + the pure `deny_rule_match` (broad-path `rm -rf`, force-push, `curl|sh`, `--dangerously`).
- `daemon/src/gateway/pipeline.rs` (L3/L5) — `is_adjudication`; the risk-0 Allow + `approve_single` rest adjudication at PolicyDecided/Approved (no executor, via `ApproveOutcome`); the `Deny if is_adjudication` **record-then-deny** arm.
- `daemon/src/gateway/executor.rs` (L3) — the defense-in-depth fail-safe `Adjudication` arm (refuses to execute).
- `daemon/src/gateway/approval.rs` (L5) — `denied_intent` (Some) + `policy_denied_intent` (None).
- `daemon/src/fault.rs` + `daemon/src/eventstore/mod.rs` (L3) — `FaultPoint::AuditEventWrite` (cfg-gated; the adjudication audit-gate, for audit-before-verdict testing).

## Decisions made (with rationale)

- **Q1 = Option A (one chokepoint), lead-ratified.** Agent mutations route through the EXISTING `submit_action` pipeline as adjudication-only `ActionRequest`s — not a parallel path (a 2nd path = a 2nd thing to prove INV-SEC-1-safe).
- **Adjudication-only, NO daemon executor.** The `ActionRequest` TERMINATES at the policy/approval verdict — risk-0 read rests at `PolicyDecided`→Allow; a mutating tool → AwaitingApproval → (human) → `Approved`→Allow / `Denied`→Deny. **No new ActionRequest state, no new R-9 edge** — the path stops earlier on existing edges. Code-enforced two ways (the pipeline terminates before execute + the `CatalogExecutor` fail-safe arm).
- **Audit-before-verdict (§15 #5).** An Allow is gated on the authoritative event committing FIRST; any submit/approve error → Deny (falls out of the existing fail-closed txn + `verdict_for_status` defaulting to Deny).
- **Q2 = conservative (lead-ratified).** Read-only auto-allow (read ≠ mutation); every mutating tool require-approval-by-default; no/failed/timed-out → Deny.
- **The hook-miss fail-open Finding (case b) — escalated, not self-resolved → CLOSED.** Verified (claude-code-guide×3 + docs): the `*`-matcher hook fires per-call (good), but a default-mode hook-MISS fails OPEN, and MCP/Task/bg-subagent aren't reliably hook-interceptable. **Resolution (lead-ratified):** the §9.1 matrix disposition DENIES the un-interceptable channels + a two-layer deny (receiver `CoverageGap` + hook-independent `permissions.deny`). **The residual CLOSES** via the cheap-closure (no `permissions.allow` + the non-interactive PTY → a hook-miss has no cached allow → BLOCKS; GH #64271) + a 5s hook timeout — so **no user-escalation** (lead pre-ruled the feasible branch).
- **A1 = record-then-deny (lead-ruled).** A blocked dangerous attempt is AUDITED (`ActionRequested`+`ActionDenied{reason}`), never silently rolled back — the forensic point of A1.
- **The `ActionDenied.approval_id` relax = Option A (orchestrator-ruled).** A policy-deny genuinely has no approval → `None` (honest; rejected B's phantom approval + C's no-event). A 2nd CONTRACT bump (0.23.0), un-consumed by ui → additive-tolerant.

### Human / lead safety gates exercised
- Step-2.5 looped the lead (cat-1 second gate) → `ADD:` (the BestEffort fail-closed assertion + the A1 record-then-deny).
- The case-b Finding → lead ratification of the disposition + the cheap-closure requirement.
- The frozen-field relax + the 2nd bump → flagged to the lead (transparency).

## Decisions explicitly NOT made (deferred)
- **Telemetry emission** (the 3.1 delta/UTC-Z pins + live `TelemetrySampled`) → **brief 044** (the other half of 3.2 part 2).
- **The live drive loop + the production wiring** → **P4**: the runtime Gateway must swap `CatalogPolicy`→`AgentMutationPolicy` AND wire `route_intercept` to the live hook transport (together — dead until then); the per-session `decision_sink` binding + the wall-clock wait + the timeout→Deny.
- **The benign-tool allow-list** (TodoWrite/WebSearch auto-allow) — a P4 **tool-policy** call for the lead (the conservative deny restricts the agent's tool surface to FS/bash; denying TodoWrite would degrade the live agent).
- **The exact Claude permission-rule grammar** for the deny-baseline (`mcp__*`/`Task`) — validate at the P4 live loop (the receiver-side `CoverageGap` deny is the PRIMARY control; `permissions.deny` is defense-in-depth).

## TDD compliance
**Clean.** Every layer ran RED (confirmed failing for the right reason) → Step-2.5 (the whole-slice design write-up looped the lead) → GREEN. `security-reviewer` ran on **every** layer (cat-1); `code-quality-reviewer` every layer. All findings categorized + fixed-in-slice or flagged. No TDD violations; no safety-critical TDD skips. The one stale-context code-quality `[high]` (map mcp→adjudicate) was correctly REJECTED (the lead ratified mcp→DENY; the correctly-briefed security review confirmed L2).

## Reachability
- **Production-live now** (on existing entry points): the §6.3 catalog entries (via `catalog::lookup` in the pipeline); the pipeline adjudication-terminal + the executor fail-safe (the `submit_action`/`execute` path); the record-then-deny (on `submit_action`); the `ClaudeSettings` generation (on `launch()`).
- **Tested-but-unwired (named P4 deferral — Future TODO, belongs to P4):** `route_intercept` + `AgentMutationPolicy` have no production caller — the runtime (`main.rs:71`) builds the Gateway with `CatalogPolicy`. The whole interception is wired at the **P4 drive loop** (the brief's Phase-3-mechanisms / Phase-4-drive-loop split). Fully test-covered via synthetic payloads + the real `submit_action`.

## Open follow-ups (Step-9 routed hot during the session; verify at `/orchestrate-end`)
- **Cross-doc (round seal, orchestrator hot-writes):** the §6.3 ActionTypeCatalog agent-mutation row + `ExecutorKind::Adjudication` + CONTRACT 0.22.0; the `ActionDenied.approval_id`-optional §7.1 EventTypeRegistry row + CONTRACT 0.23.0; the §9.1 interception AS-BUILT (PTY-primary `PreToolUse`→Gateway, adjudication-only, audit-before-verdict, coverage-gap-compensated, the cheap-closure); LESSON §26 candidate. _(Flagged at Step 9; the orchestrator owns these files — see task #3 metadata `round_seal_routes`.)_
- **Decisions-tabled (round seal):** the cat-1 Q1-A/Q2-conservative ruling · the CLOSED fail-open residual (defense-in-depth) · the MCP-fully-denied tradeoff · the A1 record-then-deny + the ActionDenied-relax → all flagged for user return-review.
- **P4 pins (Carry-forward):** the runtime `AgentMutationPolicy` swap + `route_intercept`→live-hook wiring (together) · validate the Claude permission-rule grammar · empirically verify the cheap-closure · the benign-tool allow-list (lead tool-policy) · audit-fault-vs-policy-deny distinct at the decision_sink · the P4 drive-loop's OWN security pass.
- **044:** Claude telemetry emission (the other half of 3.2 part 2).

## How to use what was built
The interception is driven, in production, by a 042-launched Claude session's `PreToolUse` hook → the daemon hook-receiver → `route_intercept(gateway, store, payload)` → `submit_action` → `verdict_for_status`. Until P4 binds the live hook transport + swaps the runtime Gateway policy to `AgentMutationPolicy`, it is exercised only via synthetic `HookPayload`s + the real Gateway (the tests). The generated `ClaudeSettings` (`launch()`) already carry the fail-closed permission baseline.

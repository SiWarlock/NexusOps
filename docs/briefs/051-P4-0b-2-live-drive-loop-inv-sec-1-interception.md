# /tdd brief — live_drive_loop_inv_sec_1_interception

## Feature
The **live drive loop (cat-1)** — wire the first LIVE Claude under the daemon: the reachable IPC `session.create` + the real-`ClaudeAdapter` launch (via the R1-A `register(Session, SessionExecutor)` seam) + the live INV-SEC-1 interception (the 043 mechanism wired to a production caller), **atomic**. This is where the binding "no live agent un-intercepted" condition flips from BY-CONSTRUCTION to **ENFORCED-BY-THE-INTERCEPTION**. Reaches the first live smoke test. **Lead-signed-off (away-authority, 2026-06-13) — 5 calls confirmed + 3 pins.**

## Use case + traceability
- **Task ID:** P4.0b-2
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (the live `HarnessAdapter` drive loop), `§6.2`/`§6.3` (the Gateway session-create + the agent-mutation catalog, live), `§15` (INV-SEC-1 — the live interception). All in Phase 4's Spec-anchor set.
- **Related context:**
  - **The 043 interception mechanism** (LESSON 26; brief 043; the daemon/CLAUDE.md §9.1 row): the `MutationIntercept`→Gateway adjudication-only chokepoint (audit-BEFORE-verdict; the agent executes the Allow'd tool, the daemon ADJUDICATES+AUDITS — no daemon executor) + the O-13 coverage-gap compensation (DirectToolUse-default→Adjudicate; MCP/Task/bg/non-default→DENY). 4.0b-2 wires it to a **production caller** (043 built the mechanism with no live caller).
  - **4.0b-1** (the `SessionExecutor` + the §15 #8 binding + the 0.5b freeze) + **4.0a** (the supervisor spine) + **R1-A** (the registration seam — `register(Session, …)` is the dispatch home).
  - **Deep-dive §8** (`docs/planning/P4-deep-dive-…md`): the finalized slice order + the cat-1 boundary (the live spawn + live interception land together).
  - **LEAD SIGN-OFF (away-authority) — the 5 calls + 3 pins** (Decisions-tabled this round):
    1. `decision_sink` = exactly-once · first-terminal-wins · ambiguity→Deny; the §6.2 wall-clock wait = **5-min default, fail-closed on timeout/cancel/session-death (LOCKED, not a knob)** [realizes d.1]. **The live-wait/cancellation-RACE is the genuinely-new surface → a DEDICATED `security-reviewer` pass is MANDATORY; a real residual there → PARK for the user.**
    2. audit-fault vs policy-deny distinguished — an **audit-write-fault → §17 `AuditWriteFailed`** (loud integrity alert, rule #5); a **policy-deny → routine tool block**.
    3. **split tool-policy [d.2] — PIN:** the benign-internal auto-allow set is an **EXPLICIT ENUMERATED allowlist** (provably no FS/git/external/exfil surface), **NOT a category heuristic**; everything unclassified **fails closed** (deny/approval-gate). Mirror the 4.0b-1 all-risk-0-allowlist + its adversarial test. `WebFetch`/`WebSearch` = approval-gated (exfil dimension); MCP/Task/bg = denied (MVP, #27203).
    4. deterministic-now / HITL-later — the receiver-side `CoverageGap` deny is the **PRIMARY control** (tested deterministically — the test pins the rule STRING, not that Claude honors it); "does Claude honor the grammar" + the hook-miss→block are **0.1-HITL follow-ons** (the authorized CLI smoke harness, user-run). **Flag-for-user (return-review):** the empirical permission-grammar + hook-miss validation PENDS the user's live Claude.
    5. **binding-condition flip — PIN (load-bearing):** the reachable `session.create` + the real-Claude launch + the live interception **CO-LAND in the SAME layer/commit** — never an intermediate shipped state where a live agent is reachable without the interception. The `security-reviewer` verifies this atomicity EVERY layer.

## Acceptance criteria (what "done" means)
- [ ] **Reachable IPC `session.create`** — a new IPC method submits a `session.create` ActionRequest through the Gateway → the R1-A seam dispatches to `SessionExecutor` (`register(Session, …)` in `main.rs`).
- [ ] **Real-Claude launch** — the launcher seam yields the live `ClaudeAdapter` (the `FakeHarness` placeholder in `PtyLauncher` is removed/replaced); the supervisor drives a REAL agent.
- [ ] **Live interception wired:** the `AgentMutationPolicy` runtime swap (`main.rs`) + `route_intercept`→live-hook transport + the per-session `decision_sink`.
- [ ] **`decision_sink` semantics (call 1):** fires **exactly-once**; **first-terminal-wins**; **ambiguity→Deny**; the wall-clock approval-wait = **5-min default, fail-closed on timeout/cancel/session-death (LOCKED)**. Every non-Allow terminal is a Deny.
- [ ] **audit-fault vs policy-deny (call 2):** an audit-write-fault on the adjudication ActionRequest → `AuditWriteFailed` (§17 loud alert) + Deny; a policy-deny → routine Deny. Distinguishable at the sink.
- [ ] **split tool-policy (call 3 + PIN):** an EXPLICIT enumerated benign-internal auto-allow allowlist (TodoWrite-class; no FS/git/external/exfil); `WebFetch`/`WebSearch` approval-gated; MCP/Task/bg denied; **unclassified → fail-closed (deny/approval-gate)**. Adversarial test (an unknown tool is NOT auto-allowed).
- [ ] **fail-closed `ClaudeSettings` (043 posture):** generated 0600, NO `permissions.allow`, a 5s hook timeout, `permissions.deny:["mcp__*","Task"]`; the agent's first instant is fail-closed until the hook is confirmed (a hook-miss → BLOCK).
- [ ] **CO-LAND atomicity (call 5 PIN):** no reachable `session.create` exists in any shipped state without the live interception wired — pinned by a test (the inverted `test_no_reachable_live_caller` → `test_live_session_create_has_interception`).
- [ ] **Live-path kill-path:** on `Kill`/shutdown the SessionActor's read-pump calls `pty.kill()` to unblock the `spawn_blocking` PTY read (a long-running live agent's pump terminates; daemon shutdown stays time-bounded).
- [ ] **`test-support` gating (the R1-A deferral, now unblocked):** `FakeHarness`/`FakePty`/`FakeLauncher` gated `#[cfg(feature = "test-support")]` (now test-only — `PtyLauncher` no longer constructs a placeholder); `cargo build --release` excludes them.
- [ ] `security-reviewer` runs **EVERY layer** (the cat-1 mandate) + a **DEDICATED `decision_sink` concurrency pass** (the live-wait/cancellation-race — MANDATORY; a real residual → escalate/PARK for the user).
- [ ] Cross-doc: §9.1 AS-BUILT (the live interception); CONTRACT impact assessed (likely none new — 043 froze the agent-mutation family at 0.22/0.23). `/preflight` clean.

## Wiring / entry point (Step 7.5)
The IPC `session.create` method is the production entry point — UI/IPC → `submit_action(session.create)` → Gateway → `SessionExecutor` → the real `ClaudeAdapter` launch via the supervisor. The interception is reachable via the live hook path: the Claude `PreToolUse` hook → the daemon receiver → `AgentMutationPolicy` → the Gateway adjudication. **This IS the live wiring** — the first reachable live-agent path; it makes the 4.0a supervisor + the 043 interception + the R1-A seam all production-reachable, atomically (call-5 PIN).

## Files expected to touch
**Modified:**
- `daemon/src/ipc/` — the `session.create` IPC method (+ peer-auth-first, per the existing mutation-method pattern).
- `daemon/src/main.rs` — `register(ExecutorKind::Session, SessionExecutor)` + the `AgentMutationPolicy` runtime swap.
- `daemon/src/harness/claude/` — the live hook transport + the fail-closed `ClaudeSettings` generation.
- `daemon/src/session/` — the launcher swap (`PtyLauncher` → real `ClaudeAdapter`) + the live-path `pty.kill()` kill-path.
- `daemon/src/gateway/` — the `AgentMutationPolicy` (the live route_intercept + the tool-policy allowlist + the decision_sink wait).
- `daemon/src/harness/mod.rs` + `terminal/` + `session/launcher.rs` — `#[cfg(feature="test-support")]` on `FakeHarness`/`FakePty`/`FakeLauncher`.
- `daemon/tests/` — the deterministic decision-logic tests + the co-land atomicity test + the tool-policy adversarial test.

**Layering (Step-2.5 #1):** **L1 = the deterministic decision logic** (the `decision_sink` wait/timeout/cancel/death + the tool-policy allowlist + audit-fault-vs-deny — FakeHarness/fake-sink, test-first, NO live agent) → **L2 = the atomic live wiring** (the IPC `session.create` + the real launch + the live interception, CO-LAND per the call-5 PIN) → **L3 = the non-cat-1 riders** (the `test-support` gating + the kill-path). If implementation needs files beyond this — flag at Step 2.5.

## RED test outline (Step 2)
**L1 — deterministic decision logic (`daemon/tests/`):**
1. **`test_decision_sink_exactly_once_first_terminal_wins`** — concurrent Allow+Deny → first terminal wins; the sink fires once; a 2nd send is ignored. Why: §15 call 1.
2. **`test_approval_wait_timeout_fails_closed`** — the 5-min wall-clock elapses (fake clock) → Deny. Why: §6.2 d.1 fail-closed.
3. **`test_approval_wait_cancel_and_death_fail_closed`** — cancel / session-death mid-wait → Deny. Why: call 1 fail-closed-LOCKED.
4. **`test_audit_fault_routes_to_audit_write_failed`** — an audit-write-fault on the adjudication ActionRequest → `AuditWriteFailed` (§17) + Deny, distinguished from a policy-deny. Why: §15 #5 / call 2.
5. **`test_tool_policy_benign_allowlist_explicit`** — `TodoWrite` (enumerated) auto-allows; an UNKNOWN tool does NOT auto-allow (fails closed); `WebFetch`/`WebSearch` → approval-gated; MCP/Task → denied. Why: call 3 PIN (explicit allowlist, fail-closed; adversarial).
6. **`test_coverage_gap_denies`** — an un-interceptable category (MCP/Task/bg/non-default) → Deny (the receiver-side `CoverageGap`, the PRIMARY control). Why: call 4 deterministic primary control.

**L2 — co-land atomicity:**
7. **`test_live_session_create_has_interception`** (the inverted binding guard) — a reachable `session.create` exists ONLY together with the wired interception; assert no shipped path reaches a live launch without the `AgentMutationPolicy` + the hook transport. Why: call 5 PIN.

**L3:**
8. **`test_fakes_gated_release_excludes`** — `cargo build --release` excludes `FakeHarness`/`FakePty`/`FakeLauncher` (acceptance-by-build). **`test_kill_path_unblocks_pump`** — `pty.kill()` on Kill unblocks the read pump (a non-self-terminating Fake → Kill → pump terminates).

**Live behavior (acceptance-by-review, NOT deterministic):** the live hook transport + the real `ClaudeAdapter` launch are **behavior-pinned + verified by the security-reviewer every layer**; the empirical "does Claude honor the grammar" + the hook-miss→block are the **0.1-HITL follow-ons** (the CLI smoke harness).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** likely **none new** (043 froze the agent-mutation family + `ExecutorKind::Adjudication` at CONTRACT 0.22/0.23; 4.0b-2 wires them live). If the live wiring surfaces a needed contract field (e.g. a session.create input shape) → flag at Step 9 (the orchestrator bumps CONTRACT + mirrors).
- **Orchestrator doc rows to write hot:** the **§9.1 AS-BUILT** (the live interception + the drive-loop production caller) + the §6.3 catalog `session.create` reachable-caller note. **escalate (cat-1) — INV-SEC-1 goes live.**
- **Reviewer policy:** `security-reviewer` = **YES, EVERY LAYER** (cat-1) + the **MANDATORY dedicated `decision_sink` concurrency pass**. `code-quality-reviewer` = every-slice.

## Things to flag at Step 2.5
1. **Layering** (above) — L1 deterministic / L2 atomic-co-land / L3 riders. My default vote: **this 3-layer split** (the deterministic logic test-first; the live wiring atomic per the call-5 PIN; the riders separate). Confirm the L2 co-land is genuinely atomic (one commit).
2. **The `decision_sink` wait mechanism.** My default vote: a **`tokio::sync::oneshot` (or a bounded channel) + `tokio::time::timeout`** for the wall-clock wait, with the first-terminal-wins/exactly-once enforced by the oneshot's single-send semantics; cancel/death drop the sender → the receiver resolves to Deny. The dedicated security pass scrutinizes the race. Flag your design.
3. **The benign-internal allowlist exact membership.** My default vote: start with **`TodoWrite` only** (the one provably-benign internal non-mutation tool); everything else fails closed (read-only tools are risk-0-auto via the existing 043 read-only path; mutating tools require approval). Enumerate explicitly; don't heuristic. Flag if more tools are provably benign.
4. **`ClaudeSettings` generation reuse.** My default vote: **extend the 042/043 `ClaudeSettings` path** (already generates 0600 settings, no `permissions.allow`, the deny-list) — 4.0b-2 wires it to the live launch + confirms the 5s hook timeout + the non-interactive empty-cache posture. Flag if the live path needs a new settings shape.

## Dependencies + sequencing
- **Depends on:** 4.0b-1 (the SessionExecutor + §15 #8 ✅), 4.0a (the supervisor ✅), R1-A (the `register()` seam ✅), 3.2 (the ClaudeAdapter + the 043 interception mechanism ✅).
- **Blocks:** 4.0c (the telemetry pump rides the live session-actor), 4.1 (survival needs the live launch path), and the **CLI smoke harness** (the authorized 0.1-HITL validation rig — a small dev-tool that drives `session.create` over UDS; lands right after 4.0b-2; the user runs it with their account).

## Estimated commit count
**~3.** L1 (the deterministic decision logic — test-first) · **L2 (the atomic live co-land — the cat-1 core, ONE commit per the call-5 PIN — never split the live launch from the interception)** · L3 (the `test-support` gating + the kill-path, non-cat-1). The L2 cat-1 commit is the load-bearing one — it gets the every-layer + the dedicated decision_sink security pass.

## Lessons-logged candidates anticipated
- **Convention candidate** — the live INV-SEC-1 drive loop: the real launch + the interception co-land atomically (never an un-intercepted-live window); the decision_sink fail-closed-on-every-non-Allow-terminal; the explicit-enumerated benign allowlist (fail-closed for unclassified). (Extends LESSON 26.)
- **Architecture-doc note candidate** — the §9.1 live drive-loop AS-BUILT.
- **Future TODO** — the CLI smoke harness (0.1-HITL rig) + the 2 flag-for-user items (empirical-validation-pends-account; the decision_sink security-pass outcome).

## How to invoke
1. **Read this brief end-to-end** + the lead's 5 calls/3 pins + LESSON 26 (the 043 mechanism).
2. **Run `/tdd live_drive_loop_inv_sec_1_interception`**.
3. **Step 0/1** — confirm the Feature + the 3-layer file plan.
4. **Step 2.5** — answer the 4 questions; **the Step-2.5 design comes back to the LEAD (2nd cat-1 gate) before GREEN.** The co-land atomicity (call-5 PIN) + the explicit-allowlist (call-3 PIN) + the decision_sink fail-closed (call-1) are the load-bearing assertions — don't soften them.
5. **Step 8** — `security-reviewer` EVERY layer + the **dedicated decision_sink concurrency pass** (mandatory). A real concurrency residual → escalate (PARK for the user).
6. **Step 9** — surface the §9.1 AS-BUILT + any CONTRACT impact + the security-pass verdicts (incl. the decision_sink pass).

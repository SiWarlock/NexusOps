# /tdd brief — session_create_executor_and_execution_profile_freeze (CAT-1; risk-0; NO live agent)

> **CAT-1 slice — own security-reviewer pass every layer.** Touches §15 #8 (ExecutionProfile binding, a safety
> invariant) + a `shared/` contract freeze (0.5b). Step-2.5 design surfaces to the lead→user before sign-off
> (as 043). **🔴 The binding safety condition (lead-ruled, non-negotiable): NO shipped/reachable state runs a
> real agent un-intercepted** — 4.0b-1 ships **NO production caller for session.create + NO real-Claude
> launch** (the 043 "mechanism built, no live caller" pattern; the reachable IPC session.create + the real
> launch + the live interception land TOGETHER in 4.0b-2).

## Feature
The Gateway `session.create`/`session.kill` action types (**risk-0** — audited auto-allow) + their executors
(drive the 4.0a `SupervisorHandle` to create/Kill a session-actor over the **non-live** launcher) + the
**§15 #8 ExecutionProfile binding** (profile resolved + recorded in `SessionStarted` at start) + the **0.5b
`ExecutionProfile` runtime-state enum freeze** in `shared/` (9 values). **FakeHarness-only; NO real-Claude
launch, NO reachable IPC caller** (the binding guarantee).

## Use case + traceability
- **Task ID:** P4.0b-1
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (catalog risk classification), `§6.2`
  (the Gateway action pipeline), `§15` (#8 execution-profile binding — a safety invariant), `§5.1` (the
  ExecutionProfile state machine — the 0.5b freeze), `§8` ("Start session" flow), `§0.1` (O-1/O-2).
- **Widens phase scope because** the 0.5b `ExecutionProfile` freeze uses the **§5.0** contract-SoT Option-A
  mechanism (Rust authority → schemars → schema → Zod/Pydantic; CONTRACT bump + 3-way verify) — §5.0 is the
  freeze *mechanism*, not a P4 phase anchor.
- **Related context:** the **P4 deep-dive §8** + the **away-authority risk-0 ruling + the 5 protective pins**
  (Decisions-tabled, 2026-06-12 P4.0b); **043** (the no-live-caller pattern this mirrors); **4.0a** (the
  `SupervisorHandle` + the `SessionLauncher` seam this drives); 0.5b (the ExecutionProfile freeze, cat-4 gate
  LIFTED — PTY-primary resolved); LESSON 18/19 (the catalog/policy), LESSON 14/15 (the freeze gotchas).

## Acceptance criteria (what "done" means)
- [ ] **The 0.5b `ExecutionProfile` runtime-state enum frozen in `shared/`** — **9 values**: `available`,
  `active`, `in_use`, `rate_limited`, `auth_expired`, `misconfigured`, `disabled`, `unknown`, **`credit_exhausted`**
  (the §5.1 8 + the SDK-pool hard-stop). A **schema-snapshot test** (`spec(§5.1)`-tagged) pins the value set;
  the schema artifact regenerates (test-9 byte-diff) + 3-way verify extends; Appendix A flips "9 frozen +
  ExecutionProfile held" → "10 frozen". Narrow the deferred `#[allow(unreachable_patterns)]` in `status.rs`.
- [ ] **`session.create`/`session.kill` cataloged at risk-0** (`catalog::lookup` → `Level0`); session.create
  **auto-executes** (no approval) BUT emits the audit trail — **PIN (a): `SessionStarted` is appended via the
  Gateway pipeline** (policy-passed + audited; INV-SEC-1 preserved, no reach-around).
- [ ] **PIN (b): §15 #8 profile recorded-at-start** — the session.create executor resolves the
  `ExecutionProfile` + records `execution_profile_id` in `SessionStarted`.
- [ ] **PIN (c): a session-profile-CHANGE action is approval-gated** (risk ≥ 1, requires approval) — the
  §15 #8 "no silent account-hop" gate lives on the CHANGE, not the routine start. (4.0b-1 introduces the
  change action type + its risk; the executor body may be a later slice — the GATE is pinned here.)
- [ ] **PIN (d): the risk-0 relaxation is NARROW** — only `session.create`/`session.kill` are risk-0
  mutations; the **no-non-zero-risk-auto-execute** safety pin (LESSON 19) stays intact for everything else
  (adversarial test: a non-session-lifecycle mutating type is NOT risk-0-auto-executed).
- [ ] **PIN (e): session.create/kill are UI/IPC-initiated only** — a `RequesterType::AgentSession`/Brain
  session.create is **rejected** (no agent/Brain session-spawn at risk-0; agent paths stay governed by the
  043 `AgentMutationPolicy`).
- [ ] The **session.create executor** drives the 4.0a `SupervisorHandle` to create a session-actor over the
  **`FakeLauncher`/non-live launcher** (NO `ClaudeAdapter` launch); **session.kill** drives a `Kill` → the
  session reaches `Killed` (audited).
- [ ] **Binding condition (structural):** NO reachable IPC `session.create` method + NO real-Claude launch
  path reachable from the executor (the real launch + the IPC method = 4.0b-2). `/preflight` clean.

## Wiring / entry point (Step 7.5)
**`none — the reachable session.create (IPC) + the real-Claude launch land in 4.0b-2`** (the binding
condition). 4.0b-1 builds the catalog entries + the executors + the §15 #8 binding + the 0.5b freeze, all
exercised via `submit_action` in tests with a FakeHarness/non-live launcher; the production daemon gains no
reachable session.create caller + no real-agent launch until 4.0b-2 wires the IPC method + the real launcher +
the interception atomically. This is the 043 pattern (the interception shipped "built but unwired").

## Files expected to touch
**New:** `daemon/tests/session_executor.rs` (or extend `daemon/tests/gateway.rs`).
**Modified:**
- `shared/src/status.rs` + `shared/src/lib.rs` + `shared/contracts/schema/*` — the 0.5b `ExecutionProfile`
  freeze (9 values) + the snapshot + regen.
- `shared/src/catalog.rs` — `session.create`/`session.kill` (+ the profile-change type) catalog entries.
- `daemon/src/gateway/` (catalog wiring / policy if a risk-0-narrow guard is needed) + `daemon/src/session/`
  (the session-create/kill executor that drives the `SupervisorHandle`).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
1. **`test_execution_profile_enum_frozen_9_values`** — Asserts: the snapshot == {9 values}; 3-way verify
   green. Why: §5.1 / §5.0 (the 0.5b §2.5-seam freeze; LESSON 14/15).
2. **`test_session_create_kill_risk_0`** — Asserts: `catalog::lookup("session.create"/"session.kill")` →
   `Level0`. Why: §6.3 (the away-ruled risk-0).
3. **`test_session_create_auto_executes_and_audits`** (PIN a) — Asserts: a UI session.create auto-executes
   (no approval) AND a `SessionStarted` event is appended via the pipeline. Why: §6.2/§15 (INV-SEC-1 — audited
   even when auto-allowed; LESSON 16).
4. **`test_session_started_records_profile`** (PIN b) — Asserts: `SessionStarted.execution_profile_id` ==
   the resolved profile. Why: §15 #8.
5. **`test_profile_change_requires_approval`** (PIN c) — Asserts: the profile-change action's catalog risk ≥ 1
   → `AwaitingApproval` (not auto-executed). Why: §15 #8 (no silent account-hop).
6. **`test_risk0_relaxation_is_narrow`** (PIN d) — Asserts: an adversarial non-session mutating type is NOT
   risk-0-auto-executed (the no-non-zero-auto-execute pin holds). Why: LESSON 19 (the relaxation is scoped).
7. **`test_session_create_rejects_agent_brain_requester`** (PIN e) — Asserts: a `RequesterType::AgentSession`
   /Brain session.create is rejected. Why: §15 #8 / 043 (UI/IPC-only; agents governed by AgentMutationPolicy).
8. **`test_session_create_drives_supervisor_non_live`** — Asserts: the executor creates a session-actor over
   the non-live launcher (FakeHarness; NO ClaudeAdapter launch); session.kill → `Killed`. Why: the binding
   condition + 4.0a integration.
9. **`test_no_reachable_live_caller`** — Asserts (structural): no IPC `session.create` method + no reachable
   real-Claude launch path from the executor. Why: 🔴 the binding condition (043 pattern).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** the **0.5b `ExecutionProfile` runtime-state enum (9 values) frozen in `shared/`** —
  a CONTRACT bump (the 10th status machine; Appendix A "10 frozen"). The §6.3 catalog gains `session.create`/
  `session.kill`/`session.profile_change` rows.
- **Orchestrator doc rows to write hot (Step 9):** Appendix A ExecutionProfile "10 frozen" + the
  `daemon/CLAUDE.md` status-machines row + the §6.3 catalog rows (risk-0 session-lifecycle) + a §15 #8 note
  (profile recorded-at-start; change approval-gated) + the §6.3 risk-0-narrow-relaxation note. **Escalate
  (cat-1) — §15 #8 is a safety invariant + the risk-0 relaxation; ⚠️ return-review.**
- **Shared-contract seam touched? YES** — the 0.5b freeze. The RED outline includes the schema-snapshot test
  (test 1), authored this cycle.

## Things to flag at Step 2.5 (the cat-1 design surface → lead→user)
1. **The binding-condition mechanism (the lead requires this confirmed):** restate exactly how 4.0b-1
   guarantees no reachable live un-intercepted agent — NO IPC session.create + NO real-Claude launch (the
   non-live launcher); the structural test (9). My default: as written (the 043 pattern). Confirm.
2. **Where the risk-0-narrow guard lives** — is "only session.create/kill may be a risk-0 mutation" enforced
   in the catalog (the entries) alone, or with a defense-in-depth policy guard (mirroring LESSON 19's
   `AllowAllPolicy` adversarial re-gate)? My default: **a defense-in-depth guard** — an adversarial test that
   a non-session risk-0 mutation can't auto-execute (the relaxation can't leak). Flag if catalog-only suffices.
3. **The profile-CHANGE action shape** (PIN c) — a distinct `session.profile_change` action type (risk ≥ 1),
   gate pinned here; is the executor body in 4.0b-1 or a later slice? My default: **the type + the risk gate
   here; the executor body can be a later slice** (the SAFETY pin is the approval-gating, testable now).
4. **PIN (e) enforcement** — a requester check (`session.create` requires `RequesterType::Human`/UI; reject
   AgentSession/Brain) vs relying on the architecture (agents only reach the interception). My default:
   **an explicit requester check** (defense-in-depth; the test pins it).
5. **`credit_exhausted` semantics** — confirmed a distinct terminal-ish runtime state (the SDK hard-pool stop)
   vs `rate_limited` (the soft interactive throttle); §11.4 two-pool meter. My default: as the lead ruled (9).

## Dependencies + sequencing
- **Depends on:** 4.0a (the `SupervisorHandle` + the launcher seam ✅), 2.2/2.3 (the catalog + executor
  framework ✅), 0.5b (cat-4 gate LIFTED ✅), 3.1 (the §9.1 types ✅).
- **Blocks:** **4.0b-2** (the live interception + the reachable session.create + the real launch — drives this
  executor + the profile binding). Cross-track: the ui's provisional ExecutionProfile shapes reconcile at the
  ui resume against this 0.5b freeze.

## Estimated commit count
**~3** (cat-1 — each layer its own commit + security pass; drive layer→layer):
- **L1** — the 0.5b `ExecutionProfile` 9-value freeze in `shared/` (snapshot + regen + 3-way verify; the
  `status.rs` `unreachable_patterns` narrow) (tests 1).
- **L2** — the §6.3 catalog risk-0 `session.create`/`session.kill` + the profile-change risk gate + the
  risk-0-narrow guard + PIN (e) requester check (tests 2,5,6,7).
- **L3** — the session.create/kill executor (drives the SupervisorHandle, non-live) + §15 #8 profile
  recorded-at-start + the binding-condition structural test (tests 3,4,8,9).
**Safety-critical → never bundled with 4.0b-2** (the live wiring is a separate cat-1 slice).

## Lessons-logged candidates anticipated
- **Architecture-doc note** — the §6.3 risk-0-narrow session-lifecycle relaxation (+ the 5 pins) + the §15 #8
  recorded-at-start/change-approval-gated split + the 0.5b ExecutionProfile freeze (10th machine).
- **Convention candidate** — "a routine, audited, supervised-lifecycle mutation may be risk-0 (auto-allow) IFF
  its danger is downstream-gated (per-tool interception) + the relaxation is NARROW + the no-non-zero-auto-
  execute pin stays intact + it's UI/IPC-initiated-only."

## How to invoke
1. **Read this brief + the deep-dive §8 + the away-authority risk-0 ruling (Decisions-tabled) end-to-end.**
   This is **cat-1** — Step-2.5 surfaces to the lead→user before GREEN.
2. **Run `/tdd session_create_executor_and_execution_profile_freeze`.**
3. **Step 2.5** — send the test-design write-up (the 5-pin assertions + the binding-condition mechanism + the
   coverage map) + your Q1–Q5 answers; **wait for the lead's cat-1 sign-off** before GREEN.
4. **Step 9** — surface the 0.5b freeze (CONTRACT bump) + the §6.3 risk-0 rows + the §15 #8 note for the
   orchestrator's hot-writes.

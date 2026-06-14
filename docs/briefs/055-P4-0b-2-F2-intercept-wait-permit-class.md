# /tdd brief — intercept_wait_permit_class_split

## Feature
Separate the **intercept-wait** permit class from the general UDS accept pool so a flood of concurrent mutating intercept-waits can never starve the UI `approve`/`deny` (or reads/subscribe) connections. Today the live `intercept` handler holds its single accept-pool permit for the **full 5-min approval-wait**; enough concurrent waits exhaust `MAX_CONNECTIONS` and **refuse** the UI's approve connection → the pending intercepts time out → fail-closed Deny → **degraded availability** (every agent tool denies because the human was starved out). Fix: bound the intercept-waits in their own class, reserving headroom in the general pool for non-wait connections, and **fail-closed-Deny** when the wait class is exhausted (fail-SAFE, never a bypass).

## Use case + traceability
- **Task ID:** P4.0b-2-F2 (the `### 4.0b-2-F2` Phase-4 row — the intercept-wait permit-class split; lead-ruled near-term production hardening, NOT MVP-bounded)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.4` (the UDS GatewayPort transport / connection bounding), `§10` (the live drive-loop availability / concurrent multi-agent supervision). INV-SEC-1-adjacent (the interception transport, cat-1-adjacent — but **fail-safe**, no semantic change to the gate).
- **Phase-scope note — this brief WIDENS phase scope because** §6.4 (the UDS GatewayPort connection-bounding) is the transport the §10 live-drive-loop availability rides; §6.4 isn't in Phase-4's default `Spec anchors`, but the F2 permit-class split is a §6.4 transport mechanism serving the §10 concurrent-multi-agent-supervision goal (the 4.0b-2 interception runs over this transport).
- **Related context:**
  - `daemon/src/runtime/listener.rs:63` — `let permits = Arc::new(Semaphore::new(max_connections))`; each accepted connection `try_acquire_owned()` (`:80`), held for the connection's lifetime (`let _permit = permit`, `:95`); at the cap → **REFUSE/drop** (`:82`, anti-DoS bound on concurrent 8-MiB frame buffers).
  - `daemon/src/ipc/methods.rs:358` — the `intercept` handler's `AwaitingApproval` arm: registers the per-session `decision_sink` + **blocks the serve thread on the `resolve_verdict` bridge up to `APPROVAL_WAIT` (5 min)**; the connection (+ its semaphore permit) is held for the whole wait (the comment at `:356-357` names exactly this bound).
  - `daemon/src/ipc/methods.rs:34` — `APPROVAL_WAIT` (the §6.2 wall-clock; fail-closed on timeout/cancel/death).
  - **Finding origin:** the 4.0b-2 C2 security pass (F2). The user's **multi-Max-plan goal = REAL concurrent multi-agent supervision** → this is a genuine production concern.
  - **LESSONS §9** (the accept loop runs `spawn_blocking(serve_connection)` under a semaphore cap — pin rejection AND permit-release) · **§12** (the dedicated-single-writer connection pattern — the subscribe precedent the intercept-wait reuses) · **§26** (the interception is fail-closed; this slice keeps that — exhaustion → Deny).

## Acceptance criteria (what "done" means)
- [ ] **The general pool keeps reserved headroom for non-wait connections.** With the intercept-wait class capped below the general cap, a flood of intercept-waits holds at most `wait_cap` general permits → **at least `RESERVED` general permits stay free** for UI `approve`/`deny` / reads / subscribe. A UI approve connection submitted while the wait class is saturated **succeeds** (not refused).
- [ ] **Wait-class exhaustion → fail-closed Deny (fail-SAFE), never a bypass.** When the intercept-wait class is full, a new `AwaitingApproval` intercept **denies immediately** (does NOT park, does NOT wait, does NOT bypass the gate) — `MutationVerdict::Deny` with an honest reason; the agent's tool is refused, the gate holds.
- [ ] **Draining works:** once the human approves/denies a pending intercept (via a reserved general slot), that wait **releases its wait-class permit** → the wait class drains → subsequent intercepts can park again. No leak (the permit releases on every terminal path — verdict, timeout, cancel, death, bridge-drop).
- [ ] **The interception semantics are UNCHANGED.** Every tool is still adjudicated; risk-0 auto-allows still resolve immediately (no wait-class permit needed — only `AwaitingApproval` acquires one); a resolved (non-waiting) intercept never touches the wait class. INV-SEC-1 preserved.
- [ ] **No-starvation under concurrency** pinned by a test: saturate the wait class with parked intercepts → assert (a) a UI approve still acquires a general permit, (b) the next intercept fail-closed-denies, (c) approving a parked one frees a wait-class slot.
- [ ] All unit/integration tests in `daemon/tests/<...>` pass.
- [ ] `/preflight` clean.
- [ ] Cross-doc: §6.4/§10 AS-BUILT note (the permit-class split) — orchestrator writes hot at Step 9.

## Wiring / entry point (Step 7.5)
The live UDS accept loop (`runtime/listener.rs::spawn_accept_loop`, wired from `main.rs`) gains the intercept-wait class semaphore; the `intercept` handler (`ipc/methods.rs`, reached on the live `intercept` RPC via `serve_connection`→`dispatch`) acquires a wait-class permit at `AwaitingApproval` before bridging to `resolve_verdict`, and fail-closed-denies if it can't. Production-reachable on the live drive loop (the same path the 4.0b-2 interception runs on).

## Files expected to touch
**Modified:**
- `daemon/src/runtime/listener.rs` — the intercept-wait class semaphore (the `RESERVED`/`wait_cap` split); thread it into the serve path.
- `daemon/src/ipc/methods.rs` — the `intercept` `AwaitingApproval` arm: `try_acquire` a wait-class permit (held across the `resolve_verdict` wait, released on every terminal path) → on `Err` (class full) → fail-closed `Deny`.
- `daemon/src/ipc/server.rs` — thread the wait-class semaphore through `serve_connection` (alongside the `decision_sink` registry it already threads).
- `daemon/src/main.rs` — construct/configure the wait-class cap (the `RESERVED`/`wait_cap` constants).
- `daemon/tests/<intercept|runtime>.rs` — the no-starvation + fail-closed-on-exhaustion + drain tests.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`test_wait_class_reserves_general_headroom`** — saturate the wait class (`wait_cap` parked intercepts) → assert a general-pool acquire (a UI approve/read connection) still succeeds (≥`RESERVED` free).
   - Asserts: the general pool is never fully held by waits.
   - Why: §6.4/§10 no-starvation — the core Finding.
2. **`test_wait_class_exhaustion_denies_fail_closed`** — with the wait class full, a new `AwaitingApproval` intercept → `Deny` (no park, no bypass; spy that `resolve_verdict` is NOT entered).
   - Asserts: fail-SAFE toward Deny on exhaustion.
   - Why: the lead's "err toward Deny, not a bypass."
3. **`test_approve_drains_wait_class`** — park `wait_cap` intercepts → approve one → assert a wait-class slot frees → the next intercept parks (doesn't deny).
   - Asserts: the permit releases on the approve path; the class drains.
   - Why: no leak; the system recovers.
4. **`test_wait_permit_released_on_every_terminal`** — for each terminal (verdict / timeout / cancel / death / bridge-drop), assert the wait-class permit is released.
   - Asserts: no permit leak on any fail-closed path.
   - Why: LESSONS §9 (pin rejection AND permit-release).
5. **`test_resolved_intercept_never_touches_wait_class`** — a risk-0 auto-allow (or any immediately-`Resolved` verdict) → acquires NO wait-class permit.
   - Asserts: only `AwaitingApproval` consumes the class; the gate semantics are unchanged.
   - Why: INV-SEC-1 preserved; the class is scoped to the wait only.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/` — the permit-class split is **daemon-internal** (the accept-loop/IPC runtime; no wire contract) → **no CONTRACT bump**, no schema-snapshot. (The `MAX_CONNECTIONS`/`RESERVED`/`wait_cap` are daemon constants, not a contract.)
- **Orchestrator doc rows to write hot:** the §6.4/§10 AS-BUILT note (the intercept-wait permit-class split — reserved general headroom + fail-closed-on-wait-class-exhaustion) → §6.4/§10 + the relevant Appendix-A/`daemon/CLAUDE.md` GatewayPort row.
- **Shared-contract (schema-snapshot) model touched?** No.

## Things to flag at Step 2.5
1. **The split mechanism — reserved sub-bound vs a fully separate pool with permit-release.**
   - **(A) Reserved sub-bound (nested):** keep the single accept `Semaphore(MAX_CONNECTIONS)`; ADD a second `Semaphore(wait_cap)` where `wait_cap = MAX_CONNECTIONS − RESERVED`. A waiting intercept holds BOTH its general permit AND a wait-class permit; because `wait_cap < MAX`, at most `wait_cap` general permits are ever held by waits → `RESERVED` general permits ALWAYS free for non-waits. No accept-time classification (you can't know the connection type at accept), no mid-connection permit release.
   - **(B) Release-during-wait + separate pool:** the waiting intercept RELEASES its general accept permit and holds only a wait-pool permit during the wait. More total concurrent connections possible, but requires releasing the accept permit mid-connection (thread the owned permit into the serve path + drop it before bridging).
   - **My vote: (A) reserved sub-bound** — it achieves the no-starvation guarantee with the least machinery (no accept-time classification, no mid-connection release), and the waiting connections are idle-blocked (not actively buffering frames) so the anti-DoS frame-buffer bound is still ~`MAX` active connections. (B) is cleaner conceptually but the mid-connection permit release is fiddly + relaxes the total-connection bound.
2. **`RESERVED` / `wait_cap` values.** Default: `RESERVED = MAX_CONNECTIONS / 4` (e.g. 16 of 64 → `wait_cap = 48`). Enough reserved for the UI/reads to never starve; `wait_cap=48` concurrent pending approvals is generous for the multi-agent goal. Tunable (a `main.rs` constant). Confirm the default.
3. **Exhaustion reason string.** The fail-closed Deny on wait-class exhaustion carries an honest, content-free reason (e.g. "approval capacity saturated — fail-closed (try again)"). Confirm it's distinguishable from the timeout Deny (so the UI/operator can tell "saturated" from "timed out") without leaking tool content.
4. **Acquire ordering.** Acquire the wait-class permit BEFORE registering the `decision_sink` + bridging (so an exhausted class denies before any wait state is created). Confirm.

## Dependencies + sequencing
- **Depends on:** 4.0b-2 C2 (the intercept transport + the `decision_sink`/approval-wait — landed ✅) + the live drive loop (`bf0ad74`/the 4.0b-2 set).
- **Blocks:** nothing hard — near-term availability hardening for real concurrent multi-agent supervision.

## Estimated commit count
**1 (own commit).** A focused availability-hardening slice on the interception transport — INV-SEC-1-adjacent → it gets its own commit + the mandated **`security-reviewer` pass** (the `invariant` policy: the no-starvation guarantee + fail-closed-on-exhaustion-per-class + no-bypass). Not bundled.

## Lessons-logged candidates anticipated
- **Convention candidate** — "a long-held wait on a shared accept pool starves the pool → give the wait its own bounded class (a reserved sub-bound below the general cap) so non-wait connections keep guaranteed headroom; exhaustion of the wait class fails CLOSED (toward Deny), never a bypass."
- **Architecture-doc note candidate** — the §6.4/§10 connection-bounding gains the intercept-wait class (reserved general headroom).

## How to invoke
1. **Read this brief end-to-end** — especially the Step-2.5 mechanism question (A vs B).
2. **Run `/tdd intercept_wait_permit_class_split`** in the implementer session.
3. **Step 2.5** — send the test-design write-up + your answers to Q1-Q4. (Fail-safe hardening within the ruled interception design — surface a genuinely-new safety fork ONLY if one appears; the default posture is err-toward-Deny.)
4. **Step 8** — `security-reviewer` (own pass: no-starvation + fail-closed-on-exhaustion + no-bypass).
5. **Step 9** — surface the §6.4/§10 cross-doc note + any deviation from the anticipated lessons-logged candidates.

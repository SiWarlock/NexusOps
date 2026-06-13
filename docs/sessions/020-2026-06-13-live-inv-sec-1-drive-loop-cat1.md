# Session 020 — P4.0b-2 L2: the live INV-SEC-1 drive loop (the cat-1 capstone)

- **Date:** 2026-06-13
- **Phase:** 4 (session lifecycle, survival & failure-mode contract) — task **P4.0b-2 (cat-1)**
- **Predecessor:** [019 — edges-R1 + 4.0b-2 decision logic](019-2026-06-13-edges-r1-and-4-0b-2-decision-logic.md)
- **Successor:** [021 — P4.0b-2-smoke: the 0.1-HITL smoke harness (initial_prompt thread + dev-client)](021-2026-06-13-smoke-harness-initial-prompt-and-dev-client.md)
- **Commits (on the round seal `0650868`):** `7332884` (C1) · `cef85eb` (C1b) · `a83c498` (C2) · `3c772b9` (C3)

## Why this session existed

A fresh full-budget implementer took the **security-critical cat-1 atomic co-land** that the prior impl deferred (it cycled at the first-WARN before the live wiring). The job: flip the binding condition from **BY-CONSTRUCTION** ("mechanism built, no live caller") to **ENFORCED-BY-THE-INTERCEPTION** — make a real Claude reachable under the daemon, with the INV-SEC-1 interception live, **atomically** (never a shipped state with a reachable live agent un-intercepted — the call-5 PIN). Brief 051; lead-signed-off (Option A + 2 hard conditions + the 5 calls + 3 pins + the user-ruled call-2 best-practice audit-fault surface).

## What was built

Decomposed into 4 commits — binding-safe deterministic floors first, then the atomic flip, then the riders.

**Files created**
- `daemon/src/decisions.rs` — **C1** the per-session `DecisionRegistry` (the decision_sink waiter store: `register`/`resolve`/`remove`/`cancel_session`; `Mutex<HashMap<action_request_id, {session_id, oneshot::Sender}>>`).
- `daemon/src/integrity.rs` — **C1b** the durable independent `IntegrityAlarm` (§17/§15 #5 call-2): `FileIntegrityAlarm` (append-only, 0600, fsync'd, **no DB handle**); `IntegrityIncident` content-free **by construction** (private `detail` + an `audit_write_failed(action_type)` factory).
- `daemon/src/hook.rs` — **C2** the `nexusopsd hook` subcommand (the live `PreToolUse` interception ingress; a thin UDS client; **fail-closed on every error** + a 360s client read-timeout).
- `daemon/tests/decision_registry.rs` (C1, 4 tests) · `daemon/tests/integrity_alarm.rs` (C1b, 2 tests).

**Files modified**
- `daemon/src/lib.rs` — `+pub mod decisions/integrity/hook`.
- `daemon/src/harness/claude/intercept.rs` — **C2** `route_intercept_live` (fires the §17 alarm on an audit-WRITE-fault, keyed structurally off `GatewayDenyKind`) + the shared `route_intercept_classified`; `route_intercept` (3-param) UNCHANGED (zero churn to the 043 tests).
- `daemon/src/harness/claude/mod.rs` — **C1b** `ClaudeLaunchSpec::env_mutations()` (strip `ANTHROPIC_API_KEY` → subscription/OAuth auth, §15 #8; set `NEXUSOPS_SESSION_ID` → the hook↔session correlation).
- `daemon/src/terminal/mod.rs` — **C1b** `EnvMutation` + the `PtySpawner::spawn` env param + `PortablePtyHost` env-apply; **C3** the `PtyKiller` trait + `Pty::killer()` + `PortablePtyHost::killer` (portable-pty `clone_killer`) + `FakePty::looping()`/`FakeKiller`; the `test-support` gating.
- `daemon/src/session/launcher.rs` — **C1b** threads `env_mutations()` into the spawn; **C3** `FakeLauncher` gating + import split.
- `daemon/src/session/mod.rs` — **C2** supervisor `cancel_session(id)` on reap (carry-forward a); **C3** `FakeLauncher` re-export split.
- `daemon/src/session/actor.rs` — **C3** the kill-path (extract the `PtyKiller` before the pump, fire `killer.kill()` before `pump.await`).
- `daemon/src/runtime/writer.rs` — **C2** the `GatewayIntercept` command + `intercept_blocking` + `spawn_with_alarm` (the `Option<Box<dyn IntegrityAlarm>>` default-None pattern; no churn to the 15 `spawn` callers).
- `daemon/src/ipc/methods.rs` — **C2** the `session.create` + `intercept` methods (the std-channel async→sync bridge for the decision_sink wait — **no `block_on` on a blocking thread**), `fire_decision_sink` on approve/deny, `verdict_response`.
- `daemon/src/ipc/server.rs` + `daemon/src/runtime/listener.rs` — **C2** thread the `DecisionRegistry` through the accept-loop → `serve_connection` → `dispatch` (`Handle::try_current()` inside the bridge, so the sync serve tests stay sync).
- `daemon/src/main.rs` — **C2** the atomic flip: hook-subcommand dispatch (+ manual tokio runtime); register `SessionExecutor` under `ExecutorKind::Session` over the **live** `PtyLauncher`; swap the production policy to `AgentMutationPolicy`; bind the `FileIntegrityAlarm` (`spawn_with_alarm`); thread the registry.
- `daemon/src/harness/mod.rs` — **C3** `FakeHarness` + `MetricQuality`-import gating.
- `daemon/benches/terminal_attach.rs` — **C1b** spawn-sig `&[]`.
- tests: `claude_intercept.rs` (+`route_intercept_live` alarm test), `claude_adapter.rs` (+env spec test), `session.rs` (+kill-path test), `session_executor.rs` (the **inverted guard** `test_live_session_create_has_interception` + the supervisor-registry param), `ipc.rs`/`runtime.rs` (registry plumbing).

**Test count:** 294 → **299**. fmt/clippy/`cargo build --release` clean.

## Decisions made

- **The async→sync bridge (no `block_on`):** the `intercept` handler runs on a SYNC `spawn_blocking` serve thread; to drive the async `resolve_verdict` wait it spawns a tokio task that bridges the verdict back over a `std::sync::mpsc::sync_channel`, and the serve thread blocks on `recv()`. Chosen over `Handle::block_on` (ambiguous/unsound from a blocking-pool thread per the tokio docs — would risk a panic on the first live tool-call). `Handle::try_current()` (not a threaded `rt` param) so the existing sync `serve_connection` tests stay sync.
- **The hook↔session correlation = `NEXUSOPS_SESSION_ID` env var** (NOT a `ClaudeSettings --session` arg) — unified with the env-hygiene mechanism; the hook subprocess inherits it through claude's env. **Fail-secure:** the env session-id is untrusted, usable ONLY as a drop-only `cancel_session` predicate (the wait keys on the daemon-minted `action_request_id` — a spoof can only Deny, never Allow).
- **Call-2 (user-ruled best practice):** an audit-WRITE-fault → deny + raise a **durable, DB-INDEPENDENT** `IntegrityAlarm` (a separate fsync'd 0600 file) + the structured `GatewayDenyKind::AuditWriteFailed`. The §15 content-free property is **structural** (private `detail` + a factory), since the alarm file is outside the §15 redactor.
- **Commit layering** (refines the brief's L1/L2/L3): binding-safe deterministic floors (C1 registry, C1b alarm + env-hygiene) land FIRST; the atomic cat-1 flip is C2 (tight, focused — the dedicated security pass lands there); the non-cat-1 riders are C3. Lead-approved.
- **`route_intercept_live` (not modifying `route_intercept`)** — keeps the 12 existing 043 call sites churn-free + gives a deterministic call-2 alarm test.
- **`session.create` requester = `User`, server-set** (PIN e) — the daemon never trusts the client's requester; the agent path is denied by `CatalogPolicy`.

## Decisions explicitly NOT made (deferred)

- **F1 — the register-after-commit liveness window** (a human approve in the µs gap between the adjudication commit and `registry.register` → spurious Deny). **Fails SAFE.** Fix direction: register atomically with the commit (the write-actor returns the rx) or post-register status re-check. → fresh pair.
- **F2 — permit-pool sharing** (intercept waits + UI approve/deny share the 64-permit pool). Bounded for MVP (claude tool-calls sequential + subagents denied → pending ≈ live-session count). Mitigation: a separate permit class / non-blocking poll. → fresh pair.
- **4.0b-2c — the systemic audit-backbone circuit-breaker** (N-consecutive/unrecoverable audit-write failure → daemon FAIL-STOPS). Daemon-wide, its own slice + security pass (the user-ruled call-2 part 3). → the orchestrator authors its brief.
- **The live approval-queue delta on intercept** (so the UI sees the pending agent-tool live; today it folds in-band, visible via `get_projection`). → flagged.
- **The 0.1-HITL smoke harness** (the user's "see it work" — a real `claude`: `ls`→auto-allow + a mutation→approval-GATED) — the empirical permission-grammar + hook-miss validation pends the user's live Claude. → fresh pair (lead-prioritized).
- Minor: `session.create` malformed-`project_id` reject-at-boundary-or-test · the `session_id` round-trip explicit test (consistent-by-construction via `SessionId::as_str()` both sides) · the temp-settings-path `O_EXCL`/app-support hardening (self-flagged P4) · `socket_path`/`production_base_dir` DRY · the `CLAUDE_CODE_OAUTH_TOKEN` profile-config (keychain-only + no-Debug-leak).

## TDD compliance

- **Clean (test-first):** C1 (`decision_registry.rs` RED → impl) · C1b (`integrity_alarm.rs` + the env spec test RED → impl) · the C2 `route_intercept_live` alarm test (RED → the refactor).
- **Exempt (non-deterministic — acceptance-by-review per brief 051):** the live transport (the IPC `intercept`/`session.create` methods, the std-bridge, the hook subcommand, the `main.rs` wiring) — these drive a real claude + a hook subprocess; covered by the every-layer security-reviewer pass + the 0.1-HITL smoke harness, not unit tests. The `test-support` gating is acceptance-by-build (`cargo build --release`).
- **Minor deviations (flagged, honest):** (a) the **C3 kill-path** impl (`PtyKiller`/the actor) landed slightly **before** its test `test_kill_path_unblocks_pump` (a non-cat-1 rider; the test genuinely pins it — without the kill-path the looping FakePty hangs/times out). (b) the **inverted binding guard** is a structural source-grep pin written after the `main.rs` wiring it asserts (not a behavior test). Neither is on the cat-1 SAFETY logic (the decision_sink / the alarm / audit-before-verdict were test-first or covered by the existing 043 tests).

## Reachability

- **`session.create`** ← UI/IPC `dispatch("session.create")` → `submit_action` → the registered `SessionExecutor` (`ExecutorKind::Session`) → the live `PtyLauncher` → `ClaudeAdapter` launch. (main.rs wiring; pinned by the inverted guard.)
- **The live interception** ← a live claude `PreToolUse` hook → `nexusopsd hook` → UDS `intercept` → `route_intercept_live` (write-actor) → the Gateway adjudication → the decision_sink wait.
- **The decision_sink** ← `intercept` registers + waits; `approve`/`deny` → `fire_decision_sink`; the supervisor reap → `cancel_session`.
- **The kill-path** ← every `SessionActor` extracts + fires the `PtyKiller` on terminal/Kill.
- **No tested-but-unwired gaps** — C1/C1b were intentional binding-safe floors, all wired by C2's atomic flip.

## Open follow-ups

See "Decisions NOT made" (F1, F2, 4.0b-2c, the live delta, the smoke harness, the minors). All routed hot at Step 9; on the reconciled tracker for the fresh pair. **No cross-doc/CONTRACT debt** (the slice is daemon-internal; the §9.1 AS-BUILT + §15 #8 + the kill-path arch-notes are the orchestrator's `/orchestrate-end` hot-routing). Push stays USER-GATED.

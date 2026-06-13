# Session 019 — 4.0b-T + edges-R1 (R1a/R1b) + 4.0b-2 (L1 + Option-A safe floor)

- **Date:** 2026-06-13
- **Phase:** Phase 4 (the live drive loop) — daemon track, `main` (single-track)
- **Predecessor:** [018](018-2026-06-13-session-create-executor-and-execution-profile-freeze.md)
- **Successor:** [020 — the live INV-SEC-1 drive loop (the cat-1 capstone)](020-2026-06-13-live-inv-sec-1-drive-loop-cat1.md)

## Why this session existed

A fresh implementer pair picked up the post-cycle 4.0b-1 boundary. Order (lead-set):
**4.0b-T** (restore the dark §5.0 verify gate) → **edges-R1** (the cross-track unblock: R1-A the
executor-registration seam + R1-B the Phase-5/7 event contract) → **4.0b-2** (the cat-1 live drive
loop). The session ended mid-4.0b-2 at a deliberate safe-floor cycle (WARN 71%, lead-ruled
impl-only cycle) so the fresh impl does the security-critical atomic co-land from full budget.

## What was built (5 slices, 6 commits)

**4.0b-T — restore the §5.0 3-way verify (`981de9d`).** The cross-language contract verify had sat
silently RED ~7 slices: a per-variant-doc'd enum (`MetricQuality`) emits as a `oneOf`/`const` union,
not a flat `enum`, which `verify.py`'s `from_schema`+`from_zod`+the zod_input filter all missed.
Taught all three to recognize the const-union form (one shared `is_enum_like` predicate; tagged
object-unions excluded) + pinned the codegen tool versions + added a **gate-self-health** assertion
(a green run must surface both arms) + 8 offline `test_verify.py` tests. Realizes LESSON 29.

**4.0b-R1a — executor registration seam + `test-support` feature (`e5c8811` + `c653121`).**
`CatalogExecutor` unit-struct → a per-`ExecutorKind` registration registry (`register()`-or-stub
dispatch); the INV-SEC-1 `Adjudication` guard + the `requires_resource_refs` precondition preserved
BEFORE dispatch (test #4 the pin). `rollback` delegates handler-else-default. The `test-support`
cargo-feature mechanism (gates nothing yet). security-reviewer PASS (0 findings); rollback-Adjudication
symmetry ruled NO-GAP (deferred to the first real rollback caller).

**4.0b-R1b — Phase-5/7 wiring event-type freeze (`150ad83`; CONTRACT 0.26.0).** ~11 new
`EventTypeRegistry` payloads (`ProjectRescanned` · `WorktreeCreated`/`BranchCreated` + 4 empty-payload
overlay transitions · `PullRequestSynced`/`IntegrationConnectionRegistered`/`Github`+`LinearSyncFailed`)
+ the `Provider` enum, one batched additive bump; `shared/`-only, no daemon emission. §15 field
contracts (`remote_url` strip-at-source/backstop · `keychain_ref` pointer · `reason` structural-class).
security-reviewer §15 SOUND; `pr_number: u64` (lead-ruled). **Full edges-R1 now on main.**

**4.0b-2 L1 — the live-interception decision logic (`00d82c0`; CONTRACT 0.27.0).** The 3 lead-ruled
calls: `decision::resolve_verdict` (oneshot + `tokio::time::timeout(5min)`, fail-closed on every
non-Allow path, exactly-once, first-terminal-wins) · `classify_gateway_error`/`GatewayDenyKind` (the
audit-fault vs policy-deny distinction) · the split tool-policy = 3 new `agent.*` catalog types
(`agent.todo_write` the LONE benign auto-allow, on `RISK0_AUTO_EXECUTE_ALLOWLIST`; `agent.web_fetch`/
`agent.web_search` risk-2 egress, approval-gated). The catalog IS the explicit enumerated allowlist;
unclassified → fail-closed. security-reviewer SOUND (0 findings).

**4.0b-2 Option-A foundation — launcher-owns-PTY (`907896d`; safe floor).** Lead-ruled Option A (a
surfaced architectural Finding — two PTY owners for one claude process). `ClaudeAdapter` no longer
spawns/owns a PTY (`new(cwd, session_id)`; `launch()` = the `Creating→Starting` marker; status from
hook signals only, #9); `PtyLauncher` owns the single live-claude spawn site + the **O-13 #10
enforcement surface** (build `ClaudeLaunchSpec` + write 0600 settings fail-closed + spawn). **NO
reachable live caller** (`test_no_reachable_live_caller` still passes) — the #10 relocation is dormant.

### Files
- **NEW:** `shared/contracts/verify/test_verify.py`, `daemon/src/harness/claude/decision.rs`,
  `daemon/tests/claude_decision.rs`, `daemon/tests/session_live.rs`.
- **Modified (4.0b-T):** `shared/contracts/verify/{verify.py,run.sh}`, `.gitignore`.
- **Modified (R1a):** `daemon/src/gateway/executor.rs`, `main.rs`, `gateway/session_executor.rs`,
  `benches/event_write.rs`, `tests/executor.rs`, `daemon/Cargo.toml`.
- **Modified (R1b):** `shared/src/{events,schema,lib}.rs`, `shared/contracts/schema/*`, `tests/contract.rs`.
- **Modified (L1):** `daemon/src/harness/claude/{intercept,mod}.rs`, `gateway/policy.rs`,
  `shared/src/{catalog,lib}.rs`, `tests/{claude_intercept,contract}.rs`, `Cargo.toml`.
- **Modified (foundation):** `daemon/src/harness/claude/mod.rs`, `session/launcher.rs`,
  `tests/{claude_adapter,claude_telemetry,session}.rs`.

## Decisions made
- **4.0b-2 Option A (launcher-owns-PTY), lead-ruled away-authority.** The #10 enforcement surface moves
  `ClaudeAdapter::launch`→`PtyLauncher` (content-preserved, security-verified = a location refinement,
  not a residual). 2 hard conditions for the L2 security pass: (1) #10 intact + single spawn site;
  (2) adapter status = hook signals only. **Surfaced as a Finding rather than guessed** — the right
  cat-1 move; the orchestrator/lead ruled before I wired.
- **CONTRACT 0.27.0** for the benign-allowlist (the brief's "likely none new" was wrong — `agent.todo_write`/
  `agent.web_fetch`/`agent.web_search` need new `agent.*` types to preserve 043 audit-before-verdict).
- **`pr_number: u64`** (R1b) — external natural; positivity structural at the parse boundary.

## Decisions explicitly NOT made (the fresh impl's atomic L2)
- The live co-land — the hook-transport subsystem (the `intercept` IPC method + the per-session
  `decision_sink` registry + the `resolve_verdict` wait) + IPC `session.create` + `main.rs`
  `register(Session, SessionExecutor)` + the `AgentMutationPolicy` swap + the live `PtyLauncher` +
  the inverted atomicity test — **ONE atomic commit** (call-5 PIN; never an un-intercepted-live window).
- Deferred to that L2: the rollback-Adjudication symmetric guard (R1a, first real rollback caller);
  the `*SyncFailed` `auth_expired` variant (R1b, 0.5b gate + §17/INV-SEC re-review).

## TDD compliance
Clean. Every slice ran RED → 2.5 → GREEN. 4.0b-2 L1 decision-wait + tool-policy + audit-fault tests
written first. The Option-A foundation is a behavior-preserving **refactor** (the migrated `session_live.rs`
launcher #10 pins are the test surface; `test_no_reachable_live_caller` is the safe-floor guard) — no
new behavior, so no TDD violation.

## Reachability
- 4.0b-T: the §5.0 verify ← CI `contract-3way` gate (`ci.yml:100 → run.sh`). 3-way GREEN @ 0.27.0.
- R1a: `CatalogExecutor::new()` @ `main.rs`; handler branch = the seam 4.0b-2 registers into.
- R1b: contract surface (EVENT_TYPE consts + schema + snapshots); emitters = edges P5/P7.
- 4.0b-2 L1: `resolve_verdict`/tool-policy reachable via the existing `route_intercept` (the live
  transport that drives them is the L2 wiring).
- 4.0b-2 foundation: **DORMANT by design** — `PtyLauncher` (live) is NOT wired into production
  (`test_no_reachable_live_caller` passes). The fresh impl's L2 wires the reachable caller + the
  interception atomically.

## Open follow-ups (for the fresh impl + the orchestrator)
- **The atomic L2 co-land** (above) + the **dedicated `decision_sink` concurrency security pass**
  (mandatory; carry-forwards: drop-sender-on-death→Deny; no-re-deliver-after-timeout) + the 2
  code-quality flags (structured `GatewayDenyKind` on the outcome; `deliver_once` keep/inline) + the
  **#10-relocation adjudication** at the security pass.
- **Orchestrator hot-writes (already flagged at Step 9):** the 3 `agent.*` catalog rows + CONTRACT
  0.27.0 (daemon/CLAUDE.md + Appendix A); the §6.3 executor-dispatch AS-BUILT (R1a); the ~11
  EventTypeRegistry rows + `Provider` + 0.26.0 (R1b); **the LESSON-25 amendment** (the #10 location
  moves to the launcher); LESSON 29 pin-ref → `test_verify.py`; the `test_no_reachable_live_caller`
  substring-grep tighten (R1a flag 5).

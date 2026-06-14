# Session 024 — telemetry pump + sink-bind · survival-schema freeze + decide_resume · ApprovalQueueRow freeze

- **Date:** 2026-06-13
- **Phase:** Phase 4 (4.0c · 4.1a · 4.0b-ui2/②-mini)
- **Predecessor:** [022-2026-06-13-daemon-circuit-breaker-ui1-f2.md](022-2026-06-13-daemon-circuit-breaker-ui1-f2.md) · 4.1a detail in [023-2026-06-13-survival-schema-freeze-and-decision-logic.md](023-2026-06-13-survival-schema-freeze-and-decision-logic.md)
- **Successor:** [025-2026-06-14-survival-broker-and-child-death-surface.md](025-2026-06-14-survival-broker-and-child-death-surface.md)

## Why this session existed

Fresh implementer (prior hit HARD-STOP at 89%, closed clean at `f3a550d`). Drove three Phase-4 slices off pre-authored briefs (056/057/058): the 044 telemetry P4 deferral, the head of 4.1 survival, and the cross-track ui-L2 unblocker. Closed at the ②-mini boundary on a HARD-STOP cycle (81%) — clean, nothing in flight, all three slices sealed.

## What was built

### 4.0c — live telemetry pump + production sink-bind (`c0a935d`, sealed `a8bfce9`) — NON-safety
- **New:** `daemon/src/runtime/telemetry_sink.rs` — `WriteActorTelemetrySink` (fire-and-forget `try_append_observation`, soft-degrade) + `WriteActorTelemetrySinkFactory` + `TelemetryHandleSlot` (deferred `OnceLock` breaks the launcher↔write-actor bootstrap cycle); `daemon/tests/telemetry_pump.rs`.
- **Modified:** `harness/mod.rs` (`poll_telemetry` trait default), `harness/claude/telemetry.rs` (`UsageSource`+`TelemetrySinkFactory` traits), `harness/claude/mod.rs` (`with_usage_source` + `poll_telemetry` + the cumulative-baseline clamp in `push_usage`), `runtime/writer.rs` (`try_append_observation`), `runtime/mod.rs`, `session/launcher.rs` (opaque factory inject — cat-1 clean), `session/actor.rs` (the telemetry pump tick), `main.rs` (build/inject/bind), `tests/claude_telemetry.rs`. No CONTRACT bump.

### 4.1a — survival-schema freeze + resume decision logic (`60b3919` + `e408024`, sealed `36096c7`) — see 023
- C1: froze `ResumeMode`(4)/`RecoveryState`(3)/`ResumeResult{mode,replayed_event_count}` in `shared/src/harness.rs` (CONTRACT 0.28.0→0.29.0) + migrated the daemon off the internal bool. C2: the pure total `decide_resume(&ResumeInputs)->ResumeResult` §8 survival ladder (`daemon/src/harness/resume.rs`). _(Full detail: session 023.)_

### 4.0b-ui2 / ②-mini — policy-decision persist + §15 redaction + typed ApprovalQueueRow (`4e9579d` + `8c4b948`, sealed `657bbd8`) — NON-cat-1, security-reviewed
- **C1:** `MIGRATION_9` (additive `policy_decision_json` columns on `approvals`+`proj_approval_queue`, SUPPORTED_USER_VERSION 8→9); the dropped `PolicyDecision` now persisted §15-redacted on the approvals row (`gateway/approval.rs::insert` takes `gtx` + redacts via `gtx.redact_row`); the projector sibling-reads it (`projections/approval_queue.rs`). security-reviewer PASS (0 findings).
- **C2:** NEW `shared/src/projections.rs` — `ApprovalQueueRow` (the FIRST frozen projection-row; CONTRACT 0.29.0→0.30.0) + typed serve (`ipc/methods.rs::read_approval_queue_typed`, get_projection branch). Unblocks ui-track L2.

## Decisions made
- **4.0c poll-source pump model** (keeps 044's `push_usage` emit-on-ingest; live `UsageSource` ingress deferred to P4); fire-and-forget sink (soft-degrade, never back-pressures the writer); cumulative-baseline high-water clamp in `push_usage` (fixes dip-then-climb over-count).
- **4.1a:** B2-strict 4-value `ResumeMode`; `decide_resume` is a TOTAL classifier (failed-attempt fallback = the caller's loop, not the function); all resume() stubs default `Replayed` except session.rs's deliberate-live double → `ReattachedLive`.
- **②-mini:** redact inside `approval::insert` via `gtx` (dual-gate co-located); `ALTER ADD COLUMN` (additive, historical NULL); plan-level/per-step `policy_decision`→NULL; typed `status:Approval`/`requester_type:RequesterType` (satisfies both ui-match + reject-unknown pins); typed serve only for the safety-critical ApprovalQueue; fail-closed on a mis-typed `policy_decision_json`.

## Decisions explicitly NOT made (deferred)
- 4.0c: the live `UsageSource` ingress (hook-receiver/statusLine feed) → **P4**; thread real workspace_id/project_id into the sink → **P5**.
- 4.1a: `decide_resume`'s production caller (bootstrap restart) → **4.1b**; the ui `RecoveryStatus` aggregate reconcile → 4.1b/4.3.
- ②-mini: plan-level/per-step `policy_decision` sourcing (NULL now) → follow-on.

## TDD compliance
**Clean.** Every slice RED-first (confirmed-RED before GREEN each time; the freezes via missing-symbol compile-fail, the migration via "no such column", the logic via assertion). No violations.

## Reachability (Step-7.5)
- 4.0c: `poll_telemetry` reachable main→supervisor→`spawn_session_actor`→run→telemetry-tick; main.rs builds/injects/binds the factory; `session/` cat-1 import-grep clean (/wired ×3).
- 4.1a: C1 = the contract surface (ui @0.29.0 + C2); **C2 `decide_resume` test-only — production caller is 4.1b** (Future-TODO, the §28 mechanism-built-test-first precedent).
- ②-mini: C1 on the live `submit_action`→approval-open path; C2 `get_projection(ApprovalQueue)`→`read_approval_queue_typed` (the live ui RPC).

## Cross-doc invariants
Single-track; all field changes flagged at Step-9 + written by the orchestrator hot in the round seals: 4.1a (§8.1 EXTENSION + §9.1/Appendix-A/§11.4 + LESSON §36, in `36096c7`), ②-mini (Appendix-A ApprovalQueueRow + §7/§11.5 + CONTRACT 0.30.0 + projection-row-freeze LESSON, in `657bbd8`), 4.0c (§9.1/§11.4/§18 AS-BUILT, in `a8bfce9`). No drift.

## Open follow-ups
- **P4:** the live `UsageSource` telemetry ingress (4.0c pump emits nothing in prod until then — by design).
- **4.1b:** the bootstrap restart caller of `decide_resume` + the detachable-terminal broker subsystem.
- **P5:** thread the session's real workspace_id/project_id into the telemetry sink.
- **②-mini follow-ons:** plan-level/per-step `policy_decision` sourcing; 2 deferred code-quality notes — the project-wide `to_value(..).unwrap_or(Null)` serialize convention (infallible today) + an end-to-end (socket) test of the get_projection ApprovalQueue dispatch branch (covered today via the direct typed-read call).
- **Cross-track:** ui regenerates from 0.29.0 (survival) + 0.30.0 (ApprovalQueueRow); the lead issues ui-L2-GO.

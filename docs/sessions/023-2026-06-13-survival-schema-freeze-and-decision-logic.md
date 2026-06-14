# Session 023 — 2026-06-13 — daemon (orchestrator): 4.1a survival-schema freeze + decision logic; the §8.1 B2-strict EXTENSION made binding

**Orchestrator-side round close-out.** No implementer `/session-end` this round — the implementer is idle-holding mid-session (the round sealed at a clean 4.1a boundary). This doc is the orchestrator's narrative for the 4.1a round **and** the §8.1 architecture EXTENSION decision, written so the user has a clean artifact for the **return-review of the survival rulings** the lead is relaying.

Predecessor: `022-2026-06-13-daemon-circuit-breaker-ui1-f2.md`.

## Round summary

4.1a = the **head of 4.1** (B2-strict survival). Two commits, NON-cat-1, sealed LOCAL (push USER-GATED — never pushed).

- **C1 freeze `60b3919`** (`feat(shared,harness)`) — `ResumeMode`(4: `resumed|replayed|relaunched|reattached_live`) / `RecoveryState`(3: `recovering|recovered|recovery_failed`) / `ResumeResult{mode, replayed_event_count}` frozen in `shared/` (the §2.5-seam); **CONTRACT 0.28.0→0.29.0**; the daemon migrated off the internal `ResumeResult{resumed_live,…}`.
- **C2 decision logic `e408024`** (`feat(daemon)`) — the pure **TOTAL** `decide_resume(&ResumeInputs)->ResumeResult` classifier (`daemon/src/harness/resume.rs`), strict-precedence ladder, FakeHarness/FakeBroker, harness-agnostic.
- **Round terminal commit** — this doc + the `ARCHITECTURE.md` §8.1 EXTENSION arch-write + LESSON §36 + brief 057 + the tracker reconcile.
- **Gate:** workspace **419 pass / 0 fail** · **3-way verify GREEN @0.29.0** (40 enums, +2) · fmt + clippy(-D) clean.

## The §8.1 B2-strict survival EXTENSION — made binding this round (for the user's return-review)

The binding `ARCHITECTURE.md` §8 previously specified only **B2-achievable** survival (`--resume`/`thread/resume` relaunch-and-resume-from-transcript). The user ruled in the stronger **B2-strict** (away-authority, 2026-06-12): the agent process **outlives the daemon** and the daemon **reattaches to the live in-flight turn** on restart. This round folded that ruling into the contract as a new **`ARCHITECTURE.md` §8.1** — a *realization* of an already-user-ruled decision, **not a fresh fork** (lead-confirmed; no separate `/arch-finalize` pass needed).

What §8.1 now binds:
- **The detachable-terminal broker** (tmux/abduco-class surviving-PTY holder) is the new subsystem B2-strict requires (under PTY-primary, a direct daemon-child agent dies on daemon death — so it must not be a direct child). It drops in behind the 4.0a launcher seam.
- **The survival ladder** (strict precedence, per session on daemon restart): `reattached_live` → `resumed` → `replayed` → `relaunched` (+ "restart session" affordance), carried by the now-frozen 4-value `ResumeMode`.
- **The §14 testability split** (the load-bearing honesty): the resume/replay/reattach **DECISION logic is deterministic + test-first** (landed this round); the **LIVE broker-reattach SURVIVAL** (the agent actually outliving the daemon) is a live-process property → a **labelled 0.1/0.3-HITL verify-only follow-on**, not deterministically unit-testable.

The only fidelity B2-strict gains over B2-achievable is the literal in-flight turn at an abrupt crash (largely reconstructed from the transcript anyway), bought with the broker subsystem + a non-deterministic survival path. The user accepted that cost, eyes open (deep-dive §7.1/§7.2/§8.1). **This remains on the user's return-review list.**

## Decisions made

- **Decomposition (lead-approved):** 4.1 split into **4.1a** (freeze + decision logic, this round) + **4.1b** (the detachable-terminal broker `SessionLauncher` + the bootstrap restart caller; live survival = HITL follow-on). Too big for one brief; the freeze leads (contract-before-consume, unblocks the ui).
- **The resume DECISION = a pure total classifier over AVAILABILITY inputs** — attempt-failure/retry is the **caller's loop** (4.1b re-calls with the failed option removed), **not** a `resume_failed` classifier input (which would make it a state machine). Banked as **LESSON §36**.
- **Harness-agnostic by construction** (reads capability bits, not harness identity) → one function for Claude + Codex, testable for both shapes via fakes now; only the live-Codex `thread/resume` verification defers to 3.3.
- **Migration uniform:** all `resume()` stubs default to `Replayed`; `ReattachedLive` is produced only by `decide_resume`.
- **Step-2.5 ADD (orchestrator):** the Resumed rung is a conjunction (`supports_resume && has_resume_handle`); added a `capability-present, handle-missing` boundary test so a capability-only bug can't pass silently. C2 = 7 tests.
- **NON-cat-1** (a survival data type + a pure classifier; INV-SEC-1 untouched) → security-reviewer correctly skipped (policy); code-quality ran (0 high; 2 med / 1 low fixed in-slice).

## Decisions explicitly NOT made

- **The broker internal-mechanism prose** (the tmux/abduco-class detail) — deferred to **4.1b** when it's actually built (§8.1 records the contract + the subsystem-in-scope; the AS-BUILT mechanism note rides 4.1b).
- **The ui `RecoveryStatus{state,affectedSessions?}` aggregate + the post-restart recovery event/projection shape** — 4.1a froze `RecoveryState` (the enum); the aggregate + the event freeze with their producer at **4.1b / 4.3** (LESSON §14 freeze-load-bearing-defer-the-rest). Cross-track SPREAD.
- **Live-Codex `thread/resume` verification** — defers to a **3.3** follow-on (the decision logic is already both-harness-tested via fakes).

## Open follow-ups

- **4.1b** — the detachable-terminal broker subsystem + the bootstrap restart caller (closes `decide_resume`'s Step-7.5 reachability). Awaiting lead-relayed direction (possibly after the user's survival-ruling return-review).
- **②-mini** `proj_approval_queue` enrich **renumbers to 0.30.0** (4.1a took 0.29.0) — flagged to the lead; the lead folds it into the ②-mini brief.
- **The ui cross-track** is unblocked for the `ResumeMode`/`RecoveryState` provisional→generated reconcile at 0.29.0.
- **User return-review:** the B2-strict survival rulings (the broker cost + the HITL-only survival verification) — now binding in §8.1.

## Audit

- **Hot-routing (orchestrator):** `ARCHITECTURE.md` §8.1 (new) + §9.1 AS-BUILT note + Appendix-A Survival/recovery row + §11.4 ResumeMode-4 + the §8 daemon-restart row pointer; `daemon/CLAUDE.md` §9.1 row flip + the LESSON §36 index row; `daemon/LESSONS.md` §36; the tracker (4.1 split, Log, Carry-forward triage, Currently-in-progress). All orchestrator-territory — the implementer touched only `shared/` + `daemon/` code+tests.
- **Cross-doc:** CONTRACT 0.29.0 mirrored (Appendix A + `daemon/CLAUDE.md` + `shared/` authority). NOT a safety-invariant change.
- **Reachability:** C1 = the contract surface (ui + C2 consume it); `decide_resume` = test-only this round, its production bootstrap caller is 4.1b (the LESSON §28 mechanism-built-test-first precedent) — no silent unreachability.

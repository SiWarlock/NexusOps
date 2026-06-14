# /tdd brief — resume_result_survival_schema_freeze_and_decision_logic

## Feature
The head of 4.1, **2 commits**: **(1)** freeze the survival/recovery contract in `shared/` (the §2.5-seam) — `ResumeMode`(4), `RecoveryState`(3), `ResumeResult{mode, replayed_event_count}` — replacing the daemon-internal `ResumeResult{resumed_live,…}`, CONTRACT 0.28.0→0.29.0; **(2)** the pure, deterministic **resume/replay/reattach DECISION logic** (`harness/resume.rs`) over availability inputs → a `ResumeResult`, FakeHarness/FakeBroker-driven, test-first, harness-agnostic (both Claude + Codex shapes). The §8 EXTENSION arch-write + the broker subsystem follow: the **detachable-terminal broker** itself is **4.1b**; the LIVE broker-reattach survival is the labelled 0.1/0.3-HITL verify-only follow-on.

## Use case + traceability
- **Task ID:** P4.1 (sub-slice **4.1a** — the survival-schema freeze + the decision logic; the first of two: 4.1a = freeze + decision-logic [2 commits] · 4.1b = broker subsystem + bootstrap restart caller [live survival = HITL-verify-only])
- **Architecture sections it implements:** `ARCHITECTURE.md §8` (the daemon-restart recovery flow + the user-ruled **B2-strict survival EXTENSION**: agent-outlives-daemon via a detachable-terminal broker → reattach-to-live-turn; the resume-or-replay-or-relaunch ladder), `§8.1` (the EXTENSION recording), `§9.1` (the `resume()` method's `ResumeResult` — "**deferred** — freezes in `shared/` at P4 §8/§17 survival"), `§5.1` (the §5.1 Session state the survival decision keys off), `§17` (the relaunch/"restart session" affordance tail)
- **Related context:** the P4 deep-dive `docs/planning/P4-deep-dive-live-drive-loop-and-survival.md` deep-dive-§7.1 (the B2-strict FINDING — the broker requirement), deep-dive-§7.2 (the (c) `ResumeResult` freeze proposal — the value set, away-authority-confirmed B2-strict → **4 values**), deep-dive-§8 (the finalized 4.1 slice row), deep-dive-§8.1 (B2-strict = a user-ruled §8 EXTENSION, "forward-note now, full prose with the 4.1 design"). LESSONS §14 (freeze load-bearing, defer the rest) · LESSONS §15 (schemars freeze gotchas) · LESSONS §23 (the contract-seam freeze pattern) · LESSONS §9/§12/§25 (the pure-classifier family the decision logic joins) · LESSONS §28 (the 4.0a "mechanism built test-first, driven next slice" precedent). The ui provisional this reconciles: `ui/src/contracts/provisional.ts` (`ResumeMode = ["resumed","replayed"]` → +`relaunched`+`reattached_live`; `RecoveryState = ["recovering","recovered","recovery_failed"]` frozen as-is).
- **Widens phase scope because** this §2.5-seam contract freeze cites cross-cutting mechanism + consumer sections (§5.0 the contract SoT, §11.4 the ui survival consumer, §7.2 the live-read precedent) beyond P4's primary §8/§8.1/§9.1 anchors — standard for a freeze slice. The work itself implements only in-scope §8/§8.1/§9.1/§5.1/§17.

## Acceptance criteria (what "done" means)

**Commit 1 — the freeze (`shared/`):**
- [ ] `shared/src/harness.rs` defines `ResumeMode` (snake_case wire) with **exactly 4 values**: `resumed`, `replayed`, `relaunched`, `reattached_live` — declaration order — with an `ALL` const (the `MetricQuality::ALL` precedent).
- [ ] `shared/src/harness.rs` defines `RecoveryState` (snake_case wire) with **exactly 3 values**: `recovering`, `recovered`, `recovery_failed` (== the ui provisional, frozen verbatim).
- [ ] `shared/src/harness.rs` defines `ResumeResult { mode: ResumeMode, replayed_event_count: u64 }` — `deny_unknown_fields`, `JsonSchema`, optionals-as-null discipline (the field set is total).
- [ ] The 3 new types are registered in the `ContractBundle` (`shared/src/schema.rs`) and the schema bundle is regenerated (`shared/contracts/schema/*`).
- [ ] `CONTRACT_VERSION` (`shared/src/lib.rs:117`) bumped `0.28.0` → `0.29.0`.
- [ ] `shared/tests/contract.rs` snapshot test pins **the value set + count** for `ResumeMode`(4) and `RecoveryState`(3) (the `test_execution_profile_enum_frozen_9_values:452` precedent) + a reject-unknown test for each; `test_schema_artifact_matches_rust:485` stays green; the `CONTRACT_VERSION` assert (~2407) → 0.29.0.
- [ ] 3-way verify (`shared/contracts/verify/run.sh`) green at 0.29.0 (the string-enum count rises by the two new enums; `replayed_event_count: u64` emits as a **bounded scalar** not an enum, per LESSONS §15 trap 2).
- [ ] **Daemon migrated to the frozen type** (build stays green): the daemon-internal `ResumeResult{resumed_live, replayed_event_count}` (`daemon/src/harness/mod.rs:165`) is **removed** and replaced by a `use` of `nexusops_shared::harness::ResumeResult`; `HarnessAdapter::resume()` returns the shared type; `FakeHarness::resume()` (`mod.rs:286`) returns the new shape; any `resumed_live`-reading site (~`mod.rs:371`) reads `.mode`.

**Commit 2 — the resume/replay/reattach DECISION logic (`daemon/`):**
- [ ] `daemon/src/harness/resume.rs` (NEW): a pure, **total** `decide_resume(&ResumeInputs) -> ResumeResult` implementing the §8 ladder, in strict precedence: `broker_has_live_session` → `ReattachedLive` (count 0); else `supports_resume && has_resume_handle` → `Resumed` (count 0); else `has_scrollback` → `Replayed` (count = `replayed_event_count`); else → `Relaunched` (count 0).
- [ ] `ResumeInputs` carries the availability inputs only (broker reattach outcome, `supports_resume`, resume-handle presence, scrollback presence, replay count) — **harness-agnostic** (Claude + Codex differ only in how the caller populates the inputs; the function is the same).
- [ ] `replayed_event_count` is carried on the result **only** on the `Replayed` path (0 on the others — the §11.4 "resumed-(live) vs replayed-(relaunched)" semantics).
- [ ] All tests in `daemon/tests/resume.rs` pass.
- [ ] `/preflight` clean (fmt + clippy -D + check + test).
- [ ] Cross-doc invariant updated atomic with the freeze (orchestrator writes hot — see below).

## Wiring / entry point (Step 7.5)
- **Commit 1 (freeze):** the frozen `ResumeResult`/`ResumeMode`/`RecoveryState` **IS the contract surface** — consumed by the ui cross-track (the provisional→generated reconcile, unblocked at 0.29.0) and by commit 2. The §2.5-seam-freeze precedent (the 0.5b `ExecutionProfile` freeze, the R1b event-type freeze — `shared/` types frozen ahead of their in-daemon producer); the frozen-enum snapshot test is the standing anti-drift proof.
- **Commit 2 (decision logic):** **none — the production caller (the bootstrap restart path) lands in 4.1b.** This is the established **4.0a precedent** (LESSONS §28): a mechanism built + tested-first one slice, driven by its production caller the next. `decide_resume` is a pure function tested directly in `daemon/tests/resume.rs`; it becomes `/wired`-reachable when 4.1b's `bootstrap.rs` restart path calls it per-session (rebuild → reclaim leases → `decide_resume` → emit the resumed-vs-replayed signal). The Step-7.5 reachability of `decide_resume` is asserted at 4.1b.

## Files expected to touch
**New:**
- `daemon/src/harness/resume.rs` — the `ResumeInputs` + `decide_resume` pure classifier (commit 2).
- `daemon/tests/resume.rs` — the decision-logic tests (commit 2).

**Modified:**
- `shared/src/harness.rs` — add `ResumeMode`, `RecoveryState`, `ResumeResult`; remove the "`ResumeResult` is deliberately ABSENT" module note (~13-16).
- `shared/src/schema.rs` — register the 3 types in `ContractBundle` (the harness block ~163-171); update the "ResumeResult … not registered" comment (~165).
- `shared/src/lib.rs` — `CONTRACT_VERSION` 0.28.0→0.29.0; update the "`ResumeResult`/survival freezes in Phase 4" note (~61-62) → "frozen 4.1a".
- `shared/contracts/schema/*` — regenerated bundle (the byte-diff gate).
- `shared/tests/contract.rs` — the frozen-enum snapshot tests (+ reject-unknown) for `ResumeMode`/`RecoveryState`; CONTRACT assert → 0.29.0.
- `daemon/src/harness/mod.rs` — remove the internal `ResumeResult` (165-168); `use` the shared one; `FakeHarness::resume()` returns the new shape; `mod resume;` declared; `resume()` doc points at the frozen type.
- `daemon/tests/*.rs` — any site asserting the old `resumed_live` field (Step-1 grep `resumed_live`).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)

**Commit 1** — `shared/tests/contract.rs` (the §2.5-seam, the `test_execution_profile_enum_frozen_9_values` precedent):
1. **`test_resume_mode_enum_frozen_4_values`** — Asserts: `ResumeMode::ALL` == `[resumed, replayed, relaunched, reattached_live]` (count == 4, wire values verbatim, declaration order). Why: §9.1/deep-dive §7.2 — the 4-value B2-strict set; the anti-drift gate (LESSONS §14/§15).
2. **`test_recovery_state_enum_frozen_3_values`** — Asserts: serializes to exactly `[recovering, recovered, recovery_failed]`. Why: §11.4 — the ui's existing 3, frozen verbatim.
3. **`test_resume_result_shape`** — Asserts: `ResumeResult{mode, replayed_event_count}` round-trips; field-name set == `{mode, replayed_event_count}`. Why: §9.1 `resume()` return + §2.5-seam field-name stability (LESSONS §15 trap 3).
4. **`test_resume_mode_unknown_value_rejected`** / **`test_recovery_state_unknown_value_rejected`** — Asserts: an out-of-set wire string fails to deserialize. Why: §5.0 reject-unknown (fail-closed).
5. **`test_schema_artifact_matches_rust`** (existing, stays green) + `CONTRACT_VERSION == "0.29.0"`. Why: §5.0 SoT (the authoritative Rust→schema gate).

**Commit 2** — `daemon/tests/resume.rs` (deterministic, FakeHarness/FakeBroker — pure inputs):
6. **`test_broker_live_session_reattaches`** — Asserts: `broker_has_live_session=true` → `ReattachedLive`, count 0 (precedence over every other available option). Why: §8 EXTENSION — B2-strict reattach is the top of the ladder.
7. **`test_native_resume_when_supported_with_handle`** — Asserts: `!broker · supports_resume · has_resume_handle` → `Resumed`. Why: §8 `--resume`/`thread/resume`.
8. **`test_replay_when_no_resume_but_scrollback`** — Asserts: `!resume-able · has_scrollback` → `Replayed`, count == `replayed_event_count`. Why: §8 "else serialized-scrollback replay + relaunch".
9. **`test_relaunch_when_nothing_available`** — Asserts: nothing available → `Relaunched`, count 0. Why: §8/§17 "resume fails → relaunch + 'restart session' affordance".
10. **`test_precedence_resume_over_scrollback`** — Asserts: `supports_resume · has_resume_handle · has_scrollback` → `Resumed` (resume wins). Why: ladder ordering is strict.
11. **`test_harness_agnostic_both_shapes`** — Asserts: a Codex-shaped input (`supports_resume=true`, handle present) and a Claude-shaped input both reach `Resumed`; a no-resume shape (`supports_resume=false`) falls through. Why: deep-dive — the decision logic is FakeHarness-testable for BOTH harnesses (the 3.3 scoping: only live-Codex `thread/resume` verification defers).

The daemon build-green migration (commit 1) is additionally proven by the existing `daemon/` suite compiling + passing against the new `ResumeResult` shape.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** `ResumeResult` (daemon-internal `{resumed_live,replayed_event_count}` → frozen `shared/` `{mode,replayed_event_count}`); **new** frozen `ResumeMode`(4), `RecoveryState`(3). **§2.5-seam (shared-contract) model touched → YES** — the RED outline includes the schema-snapshot tests.
- **Orchestrator doc rows to write hot (Step 9 routing):**
  - **`ARCHITECTURE.md` §8 — the full B2-strict survival EXTENSION prose (written RIGOROUSLY this round, per the lead — not a forward-note):** the daemon-restart recovery EXTENSION — the agent process outlives the daemon via a **detachable-terminal broker** (tmux/abduco-class; the daemon attaches/detaches to a surviving PTY holder) enabling **reattach to the live in-flight turn** (→ `ReattachedLive`); the survival ladder **reattach-live → resume (`--resume`/`thread/resume`) → serialized-scrollback replay+relaunch → relaunch (+ "restart session" affordance)**; the 4-value `ResumeMode` contract; the broker subsystem is **in P4 scope** (user-ruled away-authority 2026-06-12, return-review); the deterministic resume/replay/reattach DECISION logic is test-first, the **LIVE broker-reattach survival is the labelled 0.1/0.3-HITL verify-only follow-on**. (The broker's internal-mechanism AS-BUILT note rides 4.1b when it's built.)
  - **`ARCHITECTURE.md` §8.1** — the EXTENSION recording (user-ruled, Decisions-tabled, return-review) made binding.
  - the `daemon/CLAUDE.md` §9.1 HarnessAdapter cross-doc row — flip `ResumeResult` "**deferred** … freezes at P4 survival" → "**frozen 4.1a, CONTRACT 0.29.0**: `ResumeMode`(4) + `RecoveryState`(3) + `ResumeResult{mode,replayed_event_count}`".
  - `ARCHITECTURE.md` **Appendix A** survival row (the `ResumeResult`/`ResumeMode`/`RecoveryState` mirror) + the **§9.1** note (the `resume()` return is now frozen) + the **§11.4** ResumeMode-4 contract.
  - CONTRACT 0.28.0→0.29.0 row.
  - **NOT a safety-invariant change** (a survival/recovery data type + a pure decision classifier; INV-SEC-1 untouched) — the §8 EXTENSION prose is a *realization of an already-user-ruled decision* (away-authority 2026-06-12, return-review-flagged), not a fresh fork. FYI category, no human gate.

> **Orchestrator territory** — flag at Step 9 categorized; the orchestrator writes hot + commits at `/orchestrate-end`. The implementer touches ONLY `shared/` + `daemon/` code/tests.

## Things to flag at Step 2.5
1. **`ReattachedLive` wire value.** `reattached_live` / `reattached`. Default: **`reattached_live`** — matches deep-dive §7.2 + reads distinctly from `Resumed` (a NEW process from transcript) vs `ReattachedLive` (the SURVIVING same-process in-flight turn). The ui adds this exact string.
2. **Freeze `RecoveryStatus` (`{state, affectedSessions?}`) too, or just `RecoveryState`?** Default: **just `RecoveryState`** (+ `ResumeMode` + `ResumeResult`). `RecoveryStatus` is a ui-side wrapper; the post-restart recovery event/projection shape is 4.1b/4.3 (LESSONS §14 freeze-load-bearing-defer-the-rest). Flag the ui `RecoveryStatus` reconcile as still-pending in the cross-track Carry-forward.
3. **Does `decide_resume` model a resume-ATTEMPT-FAILED transition** (a `resume_failed` input forcing `Relaunched` even when `supports_resume`), or is "resume failed → relaunch" the bootstrap caller's concern (4.1b re-calls with `has_resume_handle=false`)? Default: **the function decides STRATEGY from availability; the failed-attempt fallback is the caller's loop (4.1b)** — keeps `decide_resume` a clean total classifier (the LESSONS §9/§12 pure-classifier discipline). Flag if you see a reason to fold the failure into the function.
4. **`ResumeResult` fields — minimal `{mode, replayed_event_count}`?** Default: **minimal** per deep-dive §7.2 — additive-later is non-breaking; `mode == Relaunched` already implies the "restart session" affordance, no separate flag.
5. **`FakeHarness::resume()` stub return.** `{Replayed,0}` / `{Resumed,0}`. Default: **`{ResumeMode::Replayed, 0}`** — a neutral placeholder the decision logic supersedes; keep `FakeHarness::supports_resume` the input the real decision reads (if a current test asserts `resumed_live==false`, `Replayed` preserves intent).
6. **`ResumeInputs` location — `harness/resume.rs` (daemon-internal)** vs a wider home? Default: **`harness/resume.rs`, daemon-internal** — it's the decision function's input record, not a wire type (only `ResumeResult` is frozen).

## Dependencies + sequencing
- **Depends on:** 3.1 (the §9.1 `HarnessAdapter` trait + `resume()` + the daemon-internal `ResumeResult` this replaces — ✅); the §2.5-seam machinery (`ContractBundle` + `contract.rs` + `verify/run.sh` — ✅, restored green by 4.0b-T).
- **Blocks:** **4.1b** (the bootstrap restart path calls `decide_resume`; the broker subsystem); the **ui cross-track** provisional→generated reconcile (unblocked at 0.29.0). **Note for the lead:** the pending ②-mini `proj_approval_queue` enrich ("→0.29.0") renumbers to **0.30.0**.

## Estimated commit count
**2.** (1) the `shared/` freeze + schema regen + snapshot tests + the mechanical daemon type-migration (atomic — the `shared/` reshape can't land without migrating the daemon consumer in the same commit); (2) the pure `decide_resume` decision logic + its tests. Both NON-safety (a survival data type + a pure classifier; INV-SEC-1 untouched) → no mandatory own-commit split; the two are separable, bisectable units (the freeze is the contract, the logic is the first consumer) so they get their own commits rather than one bundle.

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the B2-strict survival EXTENSION enters §8 as a binding contract (the 4-value `ResumeMode` + the detachable-terminal broker ladder); the broker internal-mechanism note at 4.1b.
- **Convention candidate** — the resume/replay/reattach decision as a pure total classifier over availability inputs (the LESSONS §9/§12/§25 pure-classifier family applied to survival; failed-attempt fallback is the caller's loop, not the function's).
- **Future TODO — cross-track** — the ui `RecoveryStatus` aggregate + the post-restart recovery event/projection shape reconcile at 4.1b/4.3.

## How to invoke
1. Read this brief end-to-end (esp. "Things to flag at Step 2.5").
2. `grep -rn "resumed_live\|ResumeResult" daemon/ shared/` to enumerate the migration surface, then run `/tdd resume_result_survival_schema_freeze_and_decision_logic`.
3. Step 0 (Restate) → confirm against the Feature line (2 commits: freeze + decision logic).
4. Step 1 (Identify files) → confirm against "Files expected to touch" (flag the `resumed_live` call-site set you found).
5. Step 2.5 → ping back with answers to the 6 design questions (or take defaults) + the coverage map (each acceptance bullet → its test or a not-tested-because).
6. Step 9 → surface the cross-doc invariant changes (the §8 EXTENSION full prose + the §9.1 row flip + Appendix A + CONTRACT 0.29.0) for the orchestrator to hot-write.

# Session edges-008 — R4 orchestrator round seal: §D in-lane refinement round (in-lane runway EXHAUSTED → PAUSE)

> **Orchestrator-side Round-4 round doc** (companion to the implementer's `edges-007-2026-06-12-r4-section-d-refinements.md`). Predecessor: `edges-006` (R3 orch) ← `edges-005` (R3 impl) ← … ← `edges-001` (R1 impl). Successor: _(none — R4 is a PAUSE after the seal, not a cycle: in-lane runway exhausted, R1 wiring parked for the user. No successor pair.)_ **Multi-track:** the shared `IMPLEMENTATION_PLAN.md` / `ARCHITECTURE.md` / `daemon/LESSONS.md` / `daemon/CLAUDE.md` are integration-owned and **NOT edited from `track/edges`**; this doc carries the **PLAN-DELTA HAND-OFF** to apply at the P5/P7.1 phase-exit merge (cumulative with `edges-006`'s). Lead decisions: `docs/team-handoffs/edges-lead-decision-log.md` (D6/D7 this round). R4 cross-track planning artifacts: `docs/planning/edges-{R1-routing-packet,copy-detection-finding,phase-exit-readiness}.md`.

## Why this round existed
R3 sealed all three read verticals (GitHub-PR · git diff · Linear) in-lane. The user directed a **FULL §D refinement round** — in-lane hardening only over the `edges-006` §D carry list, no wiring / no eventstore migrations (both gated: R1 seam · D5 migration FINDING). Three cohesive refinement slices landed; the remaining carries were resolved by deferral/fold/finding-doc. The round closed at a clean **arc-complete boundary** (in-lane runway exhausted, sub-ACTION context) on the lead's seal GO — a **PAUSE, not a cycle**.

## What landed (3 in-lane slices · daemon suite 381→392/0 · all LOCAL on `track/edges`, unmerged)
| Slice | Commit | What |
|---|---|---|
| edges-016 P7.1 | `70a7196` | §17 `IntegrationOutcomeClass::NotFound` terminal variant (behavior-preserving vs the synthetic `ClientError{404}`) + pure `parse_rate_limit_reset` (epoch-ms `X-RateLimit-Requests-Reset` → `RetryAfter::Until`) threaded with reset-wins-then-Retry-After precedence; Linear-localized (shared `classify()` untouched). +6 tests. |
| edges-017 P5.2 | `3b6c20b` | `open_diff` DRY refactor — extract the diff-construction block shared by `read_diff` + `read_file_hunks` into `open_diff<'r>(&'r Repository, …) -> Option<git2::Diff<'r>>`. Behavior-preserving (git_diff_log 28/28 byte-identical). +0 tests (refactor; existing suite is the guard). |
| edges-018 P7.1 | `5d31ab0` | richer `LinearIssue` fields (user-directed completeness): +5 `Option` fields (description/priority `Option<u8>`/team `Option<LinearTeam>`/created+updated `Option<Timestamp>`) + tolerant wire + `ISSUE_QUERY` extension + total `extract_issue` mapping; status (edges-013) unchanged. +5 tests (incl. a query↔mapping sync guard). |

**Quality:** 392/0; `/preflight` clean; TDD-clean (016/018 RED-first; 017 behavior-preserving refactor guarded by the green suite); every Step-2.5 orch-reviewed (3 orch **ADD**s landed: 016 precedence-boundary pin · 018 query-sync guard + priority-branch simplification); security-reviewer SKIPPED every slice per `invariant` policy (no safety invariant — §17 resilience / git2 read-only / pure mapping; api_key never enters touched code); code-quality findings fixed-in-slice. **Zero cross-doc invariant change** (`IntegrationOutcomeClass`/`LinearIssue` daemon-internal; `open_diff` private) — **CONTRACT held 0.20.0, `shared/` untouched, NO eventstore migration** (D5).

---

## PLAN-DELTA HAND-OFF (integration owner — apply at the P5/P7.1 phase-exit merge; cumulative with `edges-006`)

### A. Task-tick deltas
- **5.2 / 7.1 remain partial `[ ]`** — R4 added in-lane *hardening* within the already-complete read verticals (not new task completion): the §17 error taxonomy gained a `NotFound` terminal + epoch-ms reset honoring; the Linear read model gained richer secondary signals; the git diff backend's `open_diff` duplication was DRY'd. **Deferred wiring unchanged** (R1-gated): executors + new events + projectors + the `integration_connections` migration (D5).
- **No new `###` task heading added this round** (refinements, not new tasks) → no new-anchor-rule obligation.

### B. Arch notes (→ `ARCHITECTURE.md §9`/§17 — daemon-defined; the architecture leaves read-client granularity unpinned, like R1–R3's notes)
1. **§17 taxonomy (016):** the daemon-internal `IntegrationOutcomeClass` gains a fieldless **`NotFound`** terminal sub-class (→ `Terminal`, behavior-preserving vs the synthetic `ClientError{404}`); the Linear adapter honors the epoch-ms **`X-RateLimit-Requests-Reset`** header as `RetryAfter::Until` (reset-wins-then-`Retry-After`), Linear-localized in `classify_graphql_error_code` (the GitHub-shared `classify()` is untouched — epoch-ms is Linear-specific).
2. **§9 Linear read model (018):** `LinearIssue` carries richer secondary signals — `description` / `priority` (0–4) / `team` / `created_at` + `updated_at` — all `Option`, total-mapped (`map_priority` truncate-to-0..=4; timestamps `Timestamp::parse().ok()`); status stays `state.type`-derived (edges-013); tolerant wire (Option + `serde(default)`) keeps backward-compat + the `Eq` derive.

### C. Lessons (→ `daemon/LESSONS.md` — **renumber to the next free daemon slot at the merge; the daemon track took §26 (043 interception) + §27 (044 telemetry) during its Phase 3, so edges' lessons do NOT reuse those**)
1. **(016) epoch-ms-reset-not-Retry-After** (low-confidence; orch banked it — the trap is non-obvious + bites hard): *"a provider's rate-limit RESET may be epoch-ms (Linear's `X-RateLimit-Requests-Reset`), NOT RFC-7231 `Retry-After`; parse to absolute `RetryAfter::Until`, never `Delta` — an all-digit epoch misread as delta-seconds ≈ infinite backoff. Layer the provider-specific hint in the provider-localized mapper, never in the shared `classify()`."*
> **Lesson-number coordination (cumulative):** `edges-006` proposed 2 lessons (Linear thin-glue mirror · errors-as-400/200 code-override); `edges-008` adds this 1 → **3 edges lessons total** renumber sequentially from the next free daemon slot (≈ §28/§29/§30 at merge time; verify the daemon sequence then). NEVER reuse/collide a daemon slot.

### D. Carry-forward triage (R4 close — Step-5.5)
**Resolved this round (DELETE from the carry set):** §17 NotFound taxonomy ✅ (016) · §17 epoch-ms reset ✅ (016) · `open_diff` DRY refactor ✅ (017) · richer `LinearIssue` fields ✅ (018) · §17 terminal-non-auth/not-found taxonomy variant ✅ (folded into NotFound, 016).
**SPREAD/folded:** `test-support` cargo-feature → **folded into the R1 routing packet** (deliverable #4 + Part 4 — daemon introduces the feature covering all 3 fakes incl. `FakeHarness`; edges gates its 2 once it exists). NOT an edges slice.
**DEFERRED (lead/user-approved — finding-docs / phase-exit):** copy-detection → `docs/planning/edges-copy-detection-finding.md` (git2-0.21 limitation; needs a git2 upgrade or the gated git-CLI) · `reqwest`/octocrab/async-trait `cargo audit` → P7.1 phase-exit · `auth_expired` sync variant + 5.3 ExecutionProfile → H1 (daemon 3.2 enum freeze) · `integration_connections` + registry migrations → D5 (coordinated phase-exit) · the wiring slices (5.1/5.2/7.1) → R1.
**KEEP (small, phase-exit/wiring working set):** octocrab `ReviewState` strict-deserialize hardening · huge-diff perf · GitHub not-found taxonomy adoption (if/when its read path surfaces a not-found) · P5.4 `project.rescan` bench RUN (baseline 1.029 ms, re-author at phase-exit).
*Triage: 5 deleted · 1 spread/folded · 5 deferred · 4 kept. Carry set is the phase-exit working list, not an active next-brief set (no next brief — PAUSE).*

### E. Decisions this round (lead-logged in `docs/team-handoffs/edges-lead-decision-log.md` D6/D7 — referenced, not duplicated)
- **D6 (user, via lead):** richer `LinearIssue` fields → **BUILD** (override of the orch YAGNI-defer rec — user wants completeness); `test-support` → **FOLD INTO R1** (not a separate in-lane slice — cross-track shared-manifest surface + `FakeHarness` daemon-owned).
- **D7 (lead, automated authority):** user → AWAY mode; R1 routing **parks** for user return (cross-track — only the user routes to the daemon track); the in-lane round runs autonomously; lead holds all seal/cycle gates.
- **Seal call:** R4 sealed at the arc-complete clean boundary (in-lane exhausted, sub-ACTION context) — D3/D4 clean-boundary precedent (even cleaner: there literally is no next in-lane slice). PAUSE, not cycle.

---

## Open follow-ups / next round
- **R4 is a PAUSE — no successor pair.** In-lane runway is exhausted; the next real edges work is the **R1-gated phase-exit**, which needs (1) the user to route the R1 packet (`docs/planning/edges-R1-routing-packet.md`, now incl. the test-support deliverable #4) to the daemon track, and (2) the daemon track to deliver the executor-registration seam + Phase-5/7 event types. Until then, edges has no clean in-lane path.
- **The phase-exit is a coordinated edges→main MERGE event** — readiness + the 10-step reconciliation checklist are pre-staged in `docs/planning/edges-phase-exit-readiness.md` (verdict: in-lane COMPLETE, phase-closure BLOCKED on R1; merge target `main` `018479d`/CONTRACT 0.23.0; edges 0.20.0 → absorb 3 daemon bumps, disjoint → low conflict).
- **No merge-to-main / no push-to-main this round** — `track/edges` stays on its branch (based `a40ac00`). The R4 round commit is pushed to **origin/track/edges** (backup, continuing the R4-early backup push); main is untouched. Rebase/merge cadence = the user's call at phase-exit.

## Round seal
Round artifacts committed on `track/edges` (this `/orchestrate-end`): the 3 R4 briefs (`edges-016`/`017`/`018`) + the 3 planning docs (`edges-R1-routing-packet` [+test-support addendum] · `edges-copy-detection-finding` · `edges-phase-exit-readiness`) + this orch round doc (`edges-008`) + the lead's `edges-lead-decision-log.md` D6/D7 update (committed on the lead's behalf — shell git sandbox-blocked). The impl session doc (`edges-007`) rode its own commit `bcbef8c`. Round terminal commit hash recorded in the close-out ack to the lead. **Pushed to origin/track/edges (backup); NOT merged to main** (phase-exit only — P5/P7.1 R1-gated).

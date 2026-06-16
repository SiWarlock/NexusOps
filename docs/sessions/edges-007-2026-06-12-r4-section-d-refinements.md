# edges-007 — R4 §D refinements (implementer close-out)

**Date:** 2026-06-12
**Phase / track:** P5.2 (git diff backend) ∥ P7.1 (Linear read model) — track `edges`, daemon-side, in-lane refinement round (Round 4).
**Worktree / branch:** `../NexusOps-edges` (`track/edges`) — commits land here, never `main`/root.
**Predecessor:** `edges-006-2026-06-12-r3-orchestrator-round-seal.md` (R3 orch round-seal) ← `edges-005` (R3 impl) ← … ← `edges-001` (R1 impl).
**Successor:** _(none — R4 is a PAUSE after the seal, not a cycle; in-lane runway exhausted, R1 wiring parked for the user. No successor pair.)_

> **Multi-track note:** the shared `IMPLEMENTATION_PLAN.md` / `ARCHITECTURE.md` / `daemon/LESSONS.md` / `daemon/CLAUDE.md` are integration-owned and **NOT edited from `track/edges`**. The §B arch-notes + §C lesson candidate this round raised are the **orchestrator's** to route into the R4 round doc's PLAN-DELTA at `/orchestrate-end` (referenced below, not edited here). Lead decisions live in `docs/team-handoffs/edges-lead-decision-log.md`.

## Why this session existed
R3 sealed the Linear read vertical (`1580069`); all three read verticals (GitHub-PR, git diff, Linear) were complete in-lane. The user directed a **FULL §D refinement round** (in-lane hardening only) over the §D carry list from `edges-006` — no wiring, no eventstore migrations (both gated: R1 seam / migration FINDING D5). Three cohesive refinement slices were dispatched and landed.

## What was built (3 slices, 3 commits)

### edges-016 — §17 Linear error-taxonomy refinements (`70a7196`)
Two daemon-internal §17 taxonomy refinements on the in-lane Linear read path:
1. A fieldless `IntegrationOutcomeClass::NotFound` terminal variant (→ `DeliveryOutcome::Terminal`, behavior-preserving) replacing the synthetic `ClientError{404}` `map_linear_response` minted for an absent issue — so the gated `SyncFailed` path can branch not-found from a payload-fix `ClientError`.
2. A pure `parse_rate_limit_reset` (epoch-ms → `RetryAfter::Until`, total/no-panic) threaded through `fetch_issue → map_linear_response → classify_graphql_error_code`'s RATELIMITED arm with **reset-wins-then-Retry-After** precedence (Linear signals rate-limit reset as epoch-ms `X-RateLimit-Requests-Reset`, not RFC-7231 `Retry-After`; the prior path dropped it to dodge the all-digit-epoch → `Delta(~1.7e12 s)` ≈ infinite-backoff trap). Linear-localized — the GitHub-shared `classify()` untouched.

### edges-017 — open_diff DRY refactor (`3b6c20b`)
Behavior-preserving extraction of the diff-construction block duplicated between `read_diff` and `read_file_hunks` into a private `open_diff<'r>(&'r Repository, from, to) -> Option<git2::Diff<'r>>` (matching the in-file `resolve_tree<'r>` lifetime idiom). Each caller keeps its `Repository::discover` + divergent tail and delegates construction. Closed the named follow-up at `reads.rs:368`; consolidated the `find_similar` doc onto the helper.

### edges-018 — richer LinearIssue fields (`5d31ab0`)
User-directed completeness (override of the YAGNI defer): `LinearIssue` gains 5 `Option` fields — `description`, `priority: Option<u8>` (0–4), `team: Option<LinearTeam>` (new nested `pub struct LinearTeam{id,name,key}`), `created_at`/`updated_at: Option<Timestamp>`. Tolerant wire structs (`Option` + `#[serde(default)]` + camelCase rename) so the existing minimal fixtures still deserialize; `priority` wire `Float → Option<u8>` keeps the `LinearIssue: Eq` derive. `ISSUE_QUERY` extended; `extract_issue` maps each total/no-panic (`map_priority` truncates to 0..=4 else None; timestamps via `Timestamp::parse(s).ok()`). Status derivation (edges-013) **UNCHANGED**.

**Files modified (no new files):**
- `daemon/src/integrations/classifier.rs` — `NotFound` variant + its `to_delivery_outcome` arm; `parse_rate_limit_reset` (016).
- `daemon/src/integrations/linear.rs` — issue-absent → `NotFound`; reset threading + precedence; `LinearIssue` +5 fields + `LinearTeam` + `TeamNode`; `ISSUE_QUERY` + `extract_issue` + `map_priority` (016 + 018).
- `daemon/src/git/reads.rs` — `open_diff` helper + both callers rewired (017).
- `daemon/tests/integration_classifier.rs` — `not_found_to_terminal` + 2 `parse_rate_limit_reset` tests (016).
- `daemon/tests/linear_graphql_client.rs` — 11 call-site updates + 3 reset-precedence tests (016) + `build_issue_query_selects_richer_fields` query↔mapping sync guard (018).
- `daemon/tests/linear_read_client.rs` — 4 richer-field tests (018).

## Decisions made
- **016 Q1** — epoch-ms reset reaches the hint via Option-A: `map_linear_response` gains a `rate_limit_reset` param threaded into `classify_graphql_error_code` (precedence in the pure tested core). **Q2** — reset wins over Retry-After (`parse_rate_limit_reset(reset).or_else(|| parse_retry_after(retry_after))`). **Q3** — fieldless `NotFound`. **Q4** — GitHub out of scope (no not-found site). Orch **ADD**: pin the 3-state precedence boundary (reset-wins-over-present-RA · fallback-to-RA · both-absent-None).
- **017 Q1** — `open_diff` signature A (borrow-the-repo; caller keeps `Repository::discover`); the `Diff` borrows the repo's ODB (git2 copies `DiffOptions` in, doesn't retain the trees), so the locals drop at return. **Q2** — construction-only extraction (keep divergent tails). **Q3** — consolidate the richer `find_similar` doc onto the helper.
- **018 Q1** — nested `LinearTeam{id,name,key}` (completeness). **Q2** — numeric `Option<u8>` only (no `priority_label`). **Q3** — tolerant wire + `Option<u8>` keeps `Eq`. **Q4** — all-Option. Orch **ADD**: (1) query↔mapping sync guard; (2) priority = truncating `p as i64` + `0..=4` (dropped the strict-integral branch + EPSILON dance — no untested defensive branch).

## Decisions explicitly NOT made (deferred)
- **R1 wiring slices** (real git/GitHub/Linear executors) — gated on the daemon R1 executor-registration seam + Phase-5/7 event types (cross-track; `docs/planning/edges-R1-routing-packet.md`). Not in-lane.
- **Eventstore migrations** (registry / `integration_connections`) — D5 (global `user_version` collision; defer to the coordinated P5/P7.1 phase-exit). No edges slice adds a migration.
- **`test-support` cargo feature** (gate the fakes out of release) — folded into the R1 packet, not a slice.
- **Copy-detection** (`find_similar` other follow-up) — a git2-0.21 limitation; stays a finding-doc (`docs/planning/edges-copy-detection-finding.md`), not in scope.
- **016 LOW** — `not_found_to_terminal` asserts the variant→Terminal, not the `"not found (404)"` Display string (over-pinning a label; matches the file's `client_to_terminal` convention).
- **018 caveat** — a `team` PRESENT but missing a sub-field nulls the whole issue (consistent with existing required-field behavior; Linear sends `Team!` complete). Accepted as-is.

## TDD compliance — CLEAN
- **016 / 018:** RED tests written first (confirmed failing on the absent new API), then GREEN. No test-after-impl.
- **017:** behavior-preserving refactor → **no new RED test** (per brief + project posture); the guard was the already-green `daemon/tests/git_diff_log.rs` (28 tests, incl. the 3 forbidden-#6 read-only guards) staying **byte-identically green** before→after. Correct non-deterministic-exempt-adjacent path for a pure refactor.
- No safety-critical TDD skips. security-reviewer SKIPPED every slice per the `invariant` policy (no safety invariant touched — §17 resilience / git2 read-only / pure mapping; the api_key never enters any touched code).

## Cross-doc invariant audit — NO drift
Multi-track memory check (the orchestrator's doc edits live in its routing, invisible from `track/edges`): **no model in the `daemon/CLAUDE.md` cross-doc table changed this session.** `IntegrationOutcomeClass` (016) and `LinearIssue` (018) are **daemon-internal** (no Appendix-A row, no `shared/` surface); `open_diff` (017) is a private helper. **No `shared/` change, CONTRACT held 0.20.0.** Each daemon-internal arch-note was flagged at Step 9 (orch confirmed receipt) → see Open follow-ups.

## Reachability (Step 7.5, carried)
- **016:** `fetch_issue → map_linear_response → classify_graphql_error_code → parse_rate_limit_reset` live on the production read path; `NotFound` constructed on it + has its `to_delivery_outcome` arm. Gated downstream (tasks projector / `SyncFailed`/`auth_expired` consumer) stays gated on R1 — standing posture.
- **017:** no new entry point; `open_diff` reachable from both `read_diff` + `read_file_hunks` (the §7.2 git-SoT reads). No new dead code.
- **018:** `fetch_issue → map_linear_response → extract_issue → {map_priority, LinearTeam}` live; extended `ISSUE_QUERY` via `build_issue_query`. Gated Task Inbox consumer (reader of the richer fields) stays gated — standing posture.
- **No tested-but-unwired gaps** introduced: every gated boundary is the established edges read-core/gated-consumer posture, not a new dead-wiring gap.

## Open follow-ups
- **Orchestrator to route at `/orchestrate-end`** (the §B/§C accumulation for the phase-exit PLAN-DELTA — integration-owned, NOT edited from this worktree):
  - **§B arch-note (016):** "§17 daemon-internal taxonomy gains a `NotFound` terminal sub-class; the Linear adapter honors epoch-ms `X-RateLimit-Requests-Reset` as `RetryAfter::Until`."
  - **§B arch-note (018):** "§9 Linear read model gains richer secondary signals — description/priority(0–4)/team/created+updated — all Option, total-mapped; status stays `state.type`-derived (edges-013)."
  - **§C lesson candidate (016, low-confidence — orch's call on generalizability):** "a provider's rate-limit RESET may be epoch-ms (Linear's `X-RateLimit-Requests-Reset`), NOT RFC-7231 `Retry-After`; parse to absolute `RetryAfter::Until`, never `Delta` — an all-digit epoch misread as delta-seconds ≈ infinite backoff."
- **Future TODO — belongs to a phase (gated, not this round):** R1 wiring slices (5.1/5.2/7.1 executors) · eventstore migrations (D5) · `test-support` feature · GitHub not-found taxonomy adoption (if/when its read path surfaces a not-found) · copy-detection (git2 upgrade).

## Suite + posture
Full daemon suite **392/0** (R3 baseline 381 → **+11 this round**: 016 +6 [classifier 21→24, linear_graphql_client 12→15], 017 +0 [behavior-preserving refactor], 018 +5 [linear_read_client 8→12, linear_graphql_client 15→16]). `/preflight` clean (fmt / clippy `-D warnings` / check / test). CONTRACT held **0.20.0**; `shared/` untouched. NOT pushed, NOT merged (phase-exit only).

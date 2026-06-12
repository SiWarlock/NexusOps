# Session edges-002 — Orchestrator round seal: P5/P7.1 in-lane read-vertical foundations

> **Orchestrator-side round doc** (companion to the implementer's `edges-001-2026-06-12-p5-p7.1-read-vertical-foundations.md`). Predecessor: `edges-001`. Multi-track: this doc carries the **PLAN-DELTA HAND-OFF** for the integration owner (the shared `IMPLEMENTATION_PLAN.md` / `ARCHITECTURE.md` / `daemon/LESSONS.md` are integration-owned — NOT edited from `track/edges`; apply at the P5/P7.1 phase-exit merge). Lead decisions **D1/R1/H1** live in `docs/team-handoffs/edges-lead-decision-log.md` (referenced, not duplicated).

## Why this round existed
The `edges` track opened against the frozen P2 Gateway iface under **Approach A (clean ownership separation, lead-decided D1)**: build all in-lane detection/read/logic + private migrations; **defer all wiring** (executor arms + new shared event types); never touch `gateway/` or `shared/`. Six deterministic in-lane slices building the read/logic foundation of **P5** (git/projects) ∥ **P7.1** (GitHub/Linear).

## What landed (6 slices · suite 308/0 · all LOCAL on `track/edges`, unpushed)
| Slice | Commit | What |
|---|---|---|
| edges-001 P5.1 | `d824e42` | project detection engine — `git/detect` (git2 read-only) + `workflow/detect` (pack/cc-crew/plan/Brain signals) |
| edges-002 P5.2 | `b500496` | worktree-status reads + the §5.1-R7 two-axis precedence fn |
| edges-003 P7.1 | `f5d0d6f` | §17 integration-failure classifier → maps into the existing 1.3 `DeliveryOutcome` |
| edges-004 P7.1 | `897a9f2` | §5.1 PullRequest status-derivation fn |
| edges-005 P5.2 | `857694d` | git2 diff + log reads — **completes 5.2's status/diff/log/branch read set** |
| edges-006 P7.1 | `18ad7f0` | GitHub PR-signals aggregation — **completes the raw-GitHub→signals→§5.1 chain** |

**Two in-lane verticals complete:** the worktree read+precedence chain, and the GitHub-PR read chain. Impl session doc commit: `3cdd6a1`.

**Quality:** 308/0; security-reviewer CLEAN on all 4 git2/§17 slices, correctly skipped (invariant policy) on the 2 pure-derivation slices; a real HIGH caught+fixed in edges-005 (untracked-diff line counts). All wiring **deferred-by-design** (named gated follow-ups). **Zero cross-doc invariant change** — every slice consumed frozen `shared/` enums read-only; no CONTRACT_VERSION/schema-snapshot.

**2 brief-vs-spec precedence Findings raised by the impl + resolved** (orchestrator corrected both briefs in place): edges-002 — my Q1 default inverted the LOCKED §5.1-R7 `dirty > ahead/behind`; edges-004 — my Q1 default put `ChecksPending` above the review states, contradicting its own test 9 (resolved: ChecksFailing HARD / ChecksPending SOFT).

---

## PLAN-DELTA HAND-OFF (integration owner — apply at the P5/P7.1 phase-exit merge)

### A. Task-tick deltas (partial — in-lane logic done; wiring deferred)
- **5.1 (partial `[ ]`):** detection engine landed (`d824e42`). Deferred wiring: the `project.rescan` executor arm + `ProjectRescanned` event + `projects`/`repositories` migration + projector.
- **5.2 (partial `[ ]`):** the **status/diff/log/branch READ set COMPLETE** (`b500496`+`857694d`) + the §5.1-R7 precedence fn. Deferred wiring: the `proj_worktree` projector + git-CLI worktree/branch mutations + git watcher.
- **5.3:** untouched — **DEFERRED (H1)**, gated on the 0.5b ExecutionProfile-enum re-freeze (cat-4 HITL).
- **5.4:** **DEFERRED to the P5 phase-exit** — the §18 project-scan bench runs at its own `/phase-exit`+nightly cadence (NOT a per-slice round-ender; timing asserts flaky in RED/GREEN). Brief `edges-007` is written + spec-lint-clean (`@59aa7445`) + bench design APPROVED — re-dispatch at phase-exit (the impl had it written + compiling; ~150 lines). **The bench was measured this round (re-created then discarded at the seal): MEASURED §18 baseline = `min 0.989 ms · median 1.029 ms · p95 1.082 ms`** (300 files, 50 iters; `detect_git`+`detect_workflow` end-to-end) — **median 1.029 ms < 3 s budget → PASS (~2900× margin)**. Record this on the §18 project-scan budget row / the Phase-5 `/phase-exit` perf-budgets row when the bench re-lands.
- **7.1 (partial `[ ]`):** the §17 classifier + the PullRequest derivation + the GitHub-PR-signals aggregation landed (`f5d0d6f`+`897a9f2`+`18ad7f0`) = the full GitHub-PR read chain. Deferred wiring: the octocrab/Linear clients + auth bootstrap + `proj_pull_request` projector + `PullRequestSynced`/`*SyncFailed` events + `integration_connections`.

### B. Arch notes (→ `ARCHITECTURE.md`)
1. **§5.1-R7 worktree-status full precedence total order** (extends the 5-value partial spec at `ARCHITECTURE.md:53`): `deleted > conflicts > locked > merged > pr_open > prunable > dirty > untracked_files > behind_base > ahead_of_base > creating > clean`. Within-git-axis read resolution: `Conflicts > Dirty > UntrackedFiles > BehindBase > AheadOfBase > Clean`.
2. **§9 relative-worktrees reconcile:** the plan's 5.2 "relative-worktrees → CLI-read fallback" text is STALE post-OQ-INT-SPIKE-6 — git2 ≥1.9.4 reads `extensions.relativeWorktrees` repos; the CLI-read fallback is reserved for the sparse-checkout misreport gap only.
3. **§9 detection signal markers:** `workflow_pack=.scaffolding/manifest.json` · `cc_crew=.claude/` · `plan_file = MVP_TASKS.md | IMPLEMENTATION_PLAN.md` (first-match) · `brain=.brain` (provisional MVP placeholder — reconciles to the real project↔Brain marker at Phase 8).
4. **§7.2/§5.1 PullRequest derivation precedence** (daemon-defined — the architecture lists the 11 states but does not pin the mapping): `Merged > Closed > Draft > Conflict > ChecksFailing > {review-block: ChangesRequested | Mergeable | Approved | NeedsReview} > ChecksPending > Open`. ChecksFailing is HARD (overrides review); ChecksPending is SOFT (yields to review). `Mergeable` = review=Approved AND clean AND checks-success.
5. **§9/§7.2 GitHub→signals aggregation rules:** review `ChangesRequested > Approved > None` (ReviewRequired is branch-protection state, NOT from `reviews[]` — layered in by the octocrab client via `reviewDecision`); checks `Failure > Pending > Success > None` (failure/timed_out/action_required→Failure; queued/in_progress→Pending; neutral/skipped/cancelled/stale→Neutral-ignored); mergeable `dirty→Conflicting; clean→Clean; blocked/behind/unstable/unknown→Unknown`.

### C. Lessons (→ `daemon/LESSONS.md` — coordinate the §-numbers with the daemon track; it is at §22/§23, next free §24+)
1. **git2 read-only detection** — forbidden #6 pinned by a before/after HEAD-oid assertion; test via hermetic `git2::Repository::init` fixtures (no shelling to git, no committed fixtures).
2. **§5.1-R7 worktree precedence** — a pure fn (both axes as params, table-tested) with `as_wire_str` PINNED to the frozen serde value (LESSON 2 parity; exhaustive match forces a reconcile on a new variant).
3. **§17 integration-failure classifier** — a pure fn (no `Clock`) → daemon-internal `IntegrationOutcomeClass` preserving the auth-vs-client-vs-transient distinctions § 17 needs, then maps INTO the existing `DeliveryOutcome` (no new outbox type); Retry-After carried as `Delta | Until(Timestamp)` for the caller to resolve.
4. **git2 diff gotcha** — for "what changed vs HEAD" use `diff_tree_to_workdir_with_index` (NOT bare `diff_tree_to_workdir`, which ignores the index); untracked-file line counts need `DiffOptions::show_untracked_content(true)` (else `Patch::from_diff` is `None` → counts read 0). `read_log` bounds the lazy revwalk via `Sort::TIME` + `.take(limit)`.
5. **GitHub→§5.1 two-stage** — `from_github` aggregates raw GitHub facts → daemon-internal signals; `derive_pull_request_status` ranks → frozen §5.1; both daemon-internal, no octocrab leak (the client populates the input enums).

### D. Carry-forward (gated-wiring + deferred — for next briefs / the phase-exit)
- **The wiring slices (5.1/5.2/7.1)** — gated on (i) the daemon executor-registration seam + (ii) new shared `EventTypeRegistry` event types. **Full consumer-driven spec: `docs/planning/edges-R1-wiring-seam-and-event-specs.md`** (R1 — the seam interface + per-namespace event payloads; handed to the lead → daemon track).
- **Security-load-bearing carry:** the §17 auth wiring MUST branch on `IntegrationOutcomeClass::AuthFailed` (NOT the collapsed `DeliveryOutcome::Terminal`) to drive `SyncFailed` + `ExecutionProfile→auth_expired`; warrants a §17/INV-SEC re-review. The `*SyncFailed` non-auth variant lands first; the `auth_expired` variant is gated on the 0.5b ExecutionProfile unfreeze (H1).
- `remote_url` §15-redaction obligation (the `ProjectRescanned` event field); bare-repo `repo_root=None` + missing-vs-non-git caller guards; per-hunk diff + rename detection; the octocrab client's `reviewDecision→ReviewRequired` layering + the richer required-approvals review rule; drainer-honor of the parsed Retry-After; unknown-status→ServerError revisit (only if a consumer special-cases ServerError); the `nightly.yml`/`/phase-exit` bench wiring (integration-owned); **edges-007 §18 bench re-dispatch at the P5 phase-exit**.

### E. Decisions this round (D1/R1/H1 — see `docs/team-handoffs/edges-lead-decision-log.md`, not duplicated)
- **D1** Approach A (clean ownership separation). **R1** the executor-registration seam + wiring event-type specs (consumer-driven, handed up; 3 open choices — async PROVISIONAL + the `block_on`-on-dedicated-blocking-context caveat / `ProjectRescanned` granularity PROVISIONAL / worktree-status-as-live-read-cache DECIDED; `*SyncFailed` split). **H1** 5.3 deferred (0.5b cat-4 HITL).

---

## Open follow-ups / next round
- **The daemon track delivers the executor-registration seam + the Phase-5/7 event types** (per R1) → unblocks all edges wiring slices.
- Next round (fresh pair, from `edges-001` + the decision log + this hand-off): the gated wiring slices once the seam lands, OR more in-lane work — the octocrab/Linear read client (the network adapter behind a trait+fake), private registry migrations, edges-007 §18 bench (at phase-exit).
- **No merge-to-main this round** — `track/edges` stays on its branch; the phase-exit merge (when P5/P7.1 complete) applies this hand-off.

## Round seal
- Round artifacts committed on `track/edges` (this `/orchestrate-end`): the 7 `edges-00N` briefs + `docs/planning/edges-R1-*.md` + this doc + the lead decision log. Round commit hash recorded in the close-out ack to the lead. **NOT pushed** (user-gated). **NO merge/rebase to main** (phase-exit only — P5/P7.1 incomplete).

# Session edges-004 — R2 orchestrator round seal: GitHub-PR read chain + git diff backend + Linear vertical opened

> **Orchestrator-side Round-2 round doc** (companion to the implementer's `edges-003-2026-06-12-r2-github-read-chain-diff-backend-linear-opened.md`). Predecessor: `edges-003` ← `edges-002` (R1 orch) ← `edges-001` (R1 impl). Successor: _(Round-3 — TBD)_. **Multi-track:** this doc carries the **PLAN-DELTA HAND-OFF** for the integration owner — the shared `IMPLEMENTATION_PLAN.md` / `ARCHITECTURE.md` / `daemon/LESSONS.md` / `daemon/CLAUDE.md` are integration-owned and **NOT edited from `track/edges`**; apply this at the P5/P7.1 phase-exit merge. Lead decisions (D1/R1/H1 + the R2 cycle calls D3-reaffirm/D4) live in `docs/team-handoffs/edges-lead-decision-log.md` (referenced, not duplicated).

## Why this round existed
Round 2 continued the `edges` in-lane P5/P7.1 read verticals under **Approach A** (all detection/read/derivation logic; ALL wiring deferred; never touch `gateway/`/`shared/`). Round 1 (`6e36f47`) landed the foundations; Round 2 drove three verticals to milestones: **complete the GitHub-PR read chain**, **complete the git diff read backend**, and **open the Linear vertical**.

## What landed (6 in-lane slices · daemon suite 308→361/0 · all LOCAL on `track/edges`, unpushed/unmerged)
| Slice | Commit | What |
|---|---|---|
| edges-008 P7.1 | `0eb60d4` | GitHub raw-response → signals decode layer (`parse_*` + `signals_from_github_response`) |
| edges-009 P7.1 | `2eec8f2` | GitHub PR read client (octocrab REST; trait + fake + injected-handle live client + `extract_pr_signals`) |
| edges-010 P7.1 | `5283245` | GraphQL `reviewDecision` layering — **completes the GitHub-PR read vertical** (closes the `NeedsReview` gap) |
| edges-011 P5.2 | `fcf3ba9` | git2 rename detection in `read_diff` (`find_similar`; `ChangeKind::Renamed` + `FileChange.old_path`) |
| edges-012 P5.2 | `59392d5` | git2 per-hunk diff read — **completes the P5.2 diff backend** (`read_file_hunks` + `DiffHunk`/`DiffLine`) |
| edges-013 P7.1 | `d7a9458` | Linear issue-state derivation — **opens the Linear vertical** (`WorkflowState.type` → §5.1 `Task`) |

**Two verticals completed + one opened:** the GitHub-PR read chain (live GitHub → signals → §5.1 `PullRequest`), the git diff read backend (status/diff/log/branch + rename + per-hunk), the Linear external-task derivation. Impl session-doc commit: `793c924` (`edges-003`).

**Quality:** 361/0; TDD clean (test-first, every Step-2.5 orch-reviewed); **zero cross-doc invariant change** (`shared/` untouched — verified `git diff --stat 6e36f47..d7a9458 -- ../shared/` empty); security-reviewer ran + CLEAN on the auth/§17/forbidden-#6 slices (009/010/011/012), correctly skipped on the pure-derivation slices (008/013, `invariant` policy). Code-quality findings folded in-slice each round (all test-strengthening / forward-compat). **Deps added: `octocrab 0.53.1` + `async-trait 0.1.89`** (the only deps this round → `cargo audit` at the P7.1 phase-exit).

**edges-014 (the Linear read client core+seam) was dispatched then ABANDONED cleanly at the lead's R2 cut** (no commit; brief `@6646a508` re-opens Round 3). See "Decisions this round" for the cycle-gate process note.

---

## PLAN-DELTA HAND-OFF (integration owner — apply at the P5/P7.1 phase-exit merge)

### A. Task-tick deltas (partial — in-lane logic; wiring deferred)
- **5.2 (still partial `[ ]`):** the git diff read backend is now **COMPLETE in-lane** — `read_diff` (status/diff/log/branch, R1) + **rename detection** (`find_similar`, edges-011) + **per-hunk** (`read_file_hunks`/`DiffHunk`/`DiffLine`, edges-012). Deferred wiring (unchanged): the `proj_worktree` projector + git-CLI worktree/branch mutations + the git watcher.
- **7.1 (still partial `[ ]`):** the **GitHub-PR read chain is COMPLETE in-lane** — decode (`0eb60d4`) → octocrab REST client (`2eec8f2`) → GraphQL `reviewDecision` (`5283245`): live GitHub → signals → §5.1 `PullRequest`. The **Linear vertical is OPENED** — the issue-state derivation (`d7a9458`). Deferred wiring (unchanged): the octocrab/Linear `Destination` adapters + auth bootstrap + `proj_pull_request`/`tasks` projectors + the `PullRequestSynced`/`*SyncFailed`/`IntegrationConnectionRegistered` events + `integration_connections`.
- **5.3 / 5.4 / the `auth_expired` sync variant:** untouched — DEFERRED (H1 0.5b ExecutionProfile gate · phase-exit bench cadence · H1-linked). _(The §18 project-scan baseline measured in R1 — median 1.029 ms PASS — still stands; re-land edges-007's bench at the P5 phase-exit.)_

### B. Arch notes (→ `ARCHITECTURE.md` §9/§7.2/§5.1 — daemon-defined, the architecture leaves these unpinned, like R1's §7.2 PR-precedence note)
1. **§9/§7.2 GitHub raw-string → daemon-enum decode tables** (edges-008): the exact GitHub string sets per field (`mergeable_state` clean/dirty/blocked/behind/unstable→…; review APPROVED/CHANGES_REQUESTED/…; check `status`+`conclusion`) → the edges-006 input enums; **total + conservative** (unknown → least-salient floor; case-insensitive). Extends R1 arch-note #5 (aggregation) with the decode layer below it.
2. **§9/§7.2 GitHub read-client boundary** (edges-009): the client takes an **injected `octocrab::Octocrab`** (auth bootstrap deferred); **octocrab REST `CheckRun` has no `status` field** in 0.53.1 → completed-ness from `conclusion.is_some()`; **octocrab REST cannot see `ReviewRequired`** (branch-protection state — GraphQL-only).
3. **§9/§7.2 GraphQL `reviewDecision` layering** (edges-010): `ReviewRequired`/`NeedsReview` is **GraphQL-only**; the layering is **best-effort** (a GraphQL failure degrades to REST-only signals, never fails the read); octocrab's high-level `graphql::<R>()` folds `GraphqlResponse::Err`→`Err`; GraphQL coords go through **typed variables** (injection-safe).
4. **§9 git2 diff** (edges-011/012): **rename detection is ON** in `read_diff` (`find_similar`, matches the user's terminal `git diff -M`; default ~50% threshold); **copy detection is unavailable in git2 0.21**; the **per-hunk read shape** (`DiffHunk{old_start,old_lines,new_start,new_lines,header,lines}` / `DiffLine{kind,content,old_lineno,new_lineno}`) feeds the O-6 §7.2 PR Review Workspace; binary → `Patch::from_diff` is `Some`-with-0-hunks (not `None`).
5. **§5.1/§9 Linear `WorkflowState.type` → §5.1 `Task` mapping table** (edges-013, daemon-defined): `triage→NeedsClarification · backlog→Queued · unstarted→Ready · started→InProgress · completed→Done · canceled→Abandoned`; unknown→Backlog (conservative floor). `Done` is non-terminal (a completed issue can reopen); `Canceled→Abandoned` is terminal. Review-flavored `Task` states are unreachable from a Linear state-type (they come from a PR, not an issue). A Linear issue is an **external_task** (§5.1 R-8); status derives from the **`WorkflowState.type`** (the closed 6-value set), NOT the team's custom state name.

### C. Lessons (→ `daemon/LESSONS.md` C-list — coordinate §-numbers with the daemon track, now at §25, next free **§26+**)
1. **GitHub two-stage decode/aggregate** (008): decode (raw strings → enums, total+conservative) and aggregate (`from_github`) are separate, separately-tested layers.
2. **Thin-glue read client** (009): octocrab fields → edges-008 composer; `octocrab::Error` → edges-003 `classify`; live fetch fake-covered (octocrab models deserialize from recorded public JSON); injected handle keeps auth deferred; the §17 error carries `IntegrationOutcomeClass` (NOT the collapsed `DeliveryOutcome`).
3. **REST/GraphQL two-source review state** (010): REST `reviews[]` + GraphQL `reviewDecision` (the only source of `ReviewRequired`); GraphQL authoritative when present, REST the fallback; best-effort enrichment; typed GraphQL variables (injection-safe).
4. **git2 rename + per-hunk** (011/012): `find_similar(renames.for_untracked)` post-diff (read-only, forbidden #6; the workdir rename pairing needs find_similar's OWN `for_untracked`); the ~50% threshold gates over-eager pairing; copy unavailable in git2 0.21; the per-hunk read iterates `Patch::hunk`/`line_in_hunk` (binary → `Some`-0-hunks; `&[u8]` lossy; EOFNL→Context; rename-aware, new-path matched).
5. **Linear external-task derivation** (013): `WorkflowState.type`→`Task` (conservative floor + exhaustive match; daemon-defined mapping). Mirrors the GitHub two-stage pattern.

### D. Carry-forward (gated-wiring + deferred — for next briefs / the phase-exit)
- **edges-014/015 the real Linear `LinearGraphqlReadClient`** — the reqwest GraphQL fetch + auth + the **GraphQL-errors-as-200 mapping** (edges-010 lesson) + the HTTP dep. **edges-014 (the core+seam — `LinearIssue` + `extract_issue(&str)->Option` w/ private wire structs + `LinearReadClient` trait + `FakeLinearReadClient` + `LinearReadError`) brief is written + spec-lint-clean (`@6646a508`)** → Round-3 opener; edges-015 = the network adapter.
- **The wiring slices (5.1/5.2/7.1)** — gated on the daemon R1 executor-registration seam + the new shared `EventTypeRegistry` event types (`docs/planning/edges-R1-wiring-seam-and-event-specs.md`).
- **Security-load-bearing carries:** the §17 auth wiring MUST branch on `IntegrationOutcomeClass::AuthFailed` (not the collapsed `DeliveryOutcome::Terminal`); the gated `*SyncFailed` **persist path MUST run `GithubReadError.message` (and the Linear equivalent) through the §15 Redactor** before any sink (carries the GitHub JSON error body + URLs — NOT the token, but redact on persist); the `octocrab`/`async-trait` **`cargo audit` at the P7.1 phase-exit** + a TLS-backend / no-credential-auto-discovery-feature review at the auth-bootstrap slice.
- **§17 taxonomy refinement** — a terminal-non-auth `IntegrationOutcomeClass` variant (decode/protocol errors currently fold to retryable `ServerError`; semantically terminal) — an edges-003-classifier enrichment for the gated wiring.
- **Engine refinements:** the `open_diff` DRY refactor (extract the shared `read_diff`+`read_file_hunks` DiffOptions+target-selection+`find_similar` construction); **copy detection** (git2 0.21 can't — revisit on a newer libgit2 + a real consumer); huge-diff `find_similar`/per-hunk perf; **secondary signals** (Linear assignee-id/timestamps + richer `LinearIssue` fields); octocrab `ReviewState` strict-deserialize limitation (a `_get`+tolerant-deserialize hardening if GitHub adds a review state).
- **Tooling note:** the `rtk` full-suite *summary* mis-aggregates (it reported a spurious "393/31 suites") — use the **per-binary** breakdown for the round suite total (daemon 361; workspace incl. `shared/` 415).

### E. Decisions this round (lead-logged in `docs/team-handoffs/edges-lead-decision-log.md` — referenced, not duplicated)
- **Daemon-defined derivations/decode tables** (orch-reviewed at each Step-2.5; routed as the §B arch-notes): the GitHub decode tables + the Linear `WorkflowState.type`→`Task` table.
- **R2 cycle-gate call:** lead reaffirmed D3 (cycle at a clean arc boundary before a big greenfield slice); **sealed Round 2 at edges-013** (6 slices) — the GitHub-PR read + git diff backends complete + the Linear vertical opened, RIGHT BEFORE the big real Linear network client (deferred to R3 with full runway, avoiding a ~78% heavy close-out). Clean ~65% close-out.
- **PROCESS NOTE (orchestrator self-correction — the gate↔dispatch race):** on the lead's "seal here" GO, the orch had **already dispatched edges-014**; the orch then sent a premature "abandon" (assuming pre-RED on "no Step-2.5 yet"), which crossed the impl's in-flight Step-2.5, then a contradicting "finish" (UNDERWAY branch), which crossed the impl's clean abandon. Net resolved cleanly (6-slice seal, no work lost — the edges-014 design is in its brief + Step-2.5). **Root cause (R1 + R2):** the cycle-gate↔dispatch race. **Fix (lead-ratified, baked into the R3 spawn prompt):** when surfacing a CYCLE-GATE decision, **HOLD all dispatch until the lead responds**; and **send conditional-on-state (not absolute) when impl-state is uncertain** (the orch's version of the lead's D3 over-steering lesson).

---

## Open follow-ups / next round (Round 3)
- **Round-3 opener = edges-014** (the Linear read client core+seam — brief `@6646a508` ready) → **edges-015** (the real `LinearGraphqlReadClient` network adapter, the big greenfield slice with full runway). Then the gated `tasks`(external_task) projector + §7.3 Task Inbox.
- **The daemon track delivers the R1 executor-registration seam + the Phase-5/7 event types** → unblocks ALL edges wiring slices (still the cross-track gate; not yet landed/merged — no wiring-readiness trigger this round).
- **No merge-to-main this round** — `track/edges` stays on its branch (based `a40ac00`; main since advanced to the daemon Phase-3 seals — reconcile at the P5/P7.1 phase-exit). Rebase cadence = the user's call.

## Round seal
- Round artifacts committed on `track/edges` (this `/orchestrate-end`): the 7 `edges-008..014` briefs + this orch round doc (`edges-004`) + the lead decision log. Round commit hash recorded in the close-out ack to the lead. **NOT pushed** (user-gated). **NO merge/rebase to main** (phase-exit only — P5/P7.1 incomplete). The impl session doc (`edges-003`) rode its own commit `793c924`.

# Session edges-003 — R2 implementer: GitHub-PR read chain + git diff backend + Linear vertical opened

> **Implementer-side Round-2 session doc.** Predecessor: `edges-002` (R1 orchestrator round-seal) ← `edges-001` (R1 implementer). Successor: _(Round-3 implementer session — TBD)_. Multi-track (`track/edges`): the shared `IMPLEMENTATION_PLAN.md` / `ARCHITECTURE.md` / `daemon/LESSONS.md` / `daemon/CLAUDE.md` are integration-owned — NOT edited from this worktree; the cross-doc arch-notes + C-list lessons surfaced below are **flagged** for the orchestrator's PLAN-DELTA hand-off (applied at the P5/P7.1 phase-exit merge).

## Why this session existed
Round 2 of the `edges` track: continue the in-lane P5/P7.1 read verticals under **Approach A** (clean ownership separation — all detection/read/derivation logic + private migrations; ALL wiring deferred; never touch `gateway/` or `shared/`). Round 1 (`6e36f47`) landed the read/logic foundations; Round 2 drove three verticals to milestones: complete the **GitHub-PR read chain**, complete the **git diff read backend**, and **open the Linear vertical**.

## What landed (6 in-lane slices · daemon suite 308→361/0 · all LOCAL on `track/edges`, unpushed/unmerged)
| Slice | Commit | What |
|---|---|---|
| edges-008 P7.1 | `0eb60d4` | GitHub raw-response → signals decode layer (`parse_*` + `signals_from_github_response`) |
| edges-009 P7.1 | `2eec8f2` | GitHub PR read client (octocrab REST; trait + fake + injected-handle live client + `extract_pr_signals`) |
| edges-010 P7.1 | `5283245` | GraphQL `reviewDecision` layering — **completes the GitHub-PR read vertical** (closes the `NeedsReview` gap) |
| edges-011 P5.2 | `fcf3ba9` | git2 rename detection in `read_diff` (`find_similar`; `ChangeKind::Renamed` + `FileChange.old_path`) |
| edges-012 P5.2 | `59392d5` | git2 per-hunk diff read — **completes the P5.2 diff backend** (`read_file_hunks` + `DiffHunk`/`DiffLine`) |
| edges-013 P7.1 | `d7a9458` | Linear issue-state derivation — **opens the Linear vertical** (`WorkflowState.type` → §5.1 `Task`) |

### Files created
- `daemon/tests/github_response_decode.rs` — edges-008 decode-layer tests (13).
- `daemon/src/integrations/github.rs` — edges-009/010 GitHub read client (trait/fake/octocrab client + `extract_pr_signals` + the GraphQL `reviewDecision` layering).
- `daemon/tests/github_read_client.rs` — edges-009/010 tests (18).
- `daemon/src/integrations/linear.rs` — edges-013 Linear derivation (`LinearStateType` + `parse_linear_state_type` + `derive_task_status_from_linear`).
- `daemon/tests/linear_issue_state.rs` — edges-013 tests (6).

### Files modified
- `daemon/src/integrations/pull_request.rs` — edges-008 decode block + `PartialEq/Eq` on `PullRequestSignals`.
- `daemon/src/integrations/mod.rs` — `pub mod github;` (009) + `pub mod linear;` (013) + doc updates.
- `daemon/Cargo.toml` + `Cargo.lock` — edges-009 added `octocrab = "0.53"` (resolved 0.53.1) + `async-trait = "0.1"` (resolved 0.1.89). **The only deps added this round.**
- `daemon/src/git/reads.rs` — edges-011 rename detection + edges-012 per-hunk read (`read_file_hunks` + the new types).
- `daemon/tests/git_diff_log.rs` — edges-011 rename tests + edges-012 hunk tests (18→28).

## Decisions made (with rationale)
- **Pinned-source spikes corrected 4 brief assumptions** (each before GREEN, ratified at Step-2.5): (008) emit canonical SCREAMING_SNAKE review-state strings; (009) octocrab `CheckRun` has **no `status` field** → completed-ness from `conclusion.is_some()`; octocrab `state`/`mergeable_state`/review-`state` are **typed `#[non_exhaustive]` enums** (stringified → edges-008 decode); `octocrab::Error` is `#[non_exhaustive]`/non-constructible → the error-map is `not-tested-because` + the §17 carry pinned via the fake. (010) octocrab's high-level `graphql::<R>()` **already folds** `GraphqlResponse::Err` → `Err(Error::Graphql)` → the degrade is one `Err(_)=>None` arm; **GraphQL variables** (not string interpolation) for injection-safety. (011) git2 0.21 detects **no copies** of unmodified files in either diff mode → **copy deferred** (no never-produced variant); `find_similar` needs its **own `for_untracked`** flag for a workdir rename. (012) git2 binary → `Patch::from_diff` is **`Some`-with-0-hunks** (not `None`) → handled both ways; `read_file_hunks` is **rename-aware** (sibling-read consistency with `read_diff`, file-matched on the NEW path).
- **Daemon-defined mappings** (the architecture leaves these unpinned — flagged for the orch's arch-notes): the §7.2/§5.1 PR-derivation precedence (consumed); the §9/§7.2 GitHub raw-string → daemon-enum decode tables; the **Linear `WorkflowState.type` → §5.1 `Task`** table (`triage→NeedsClarification · backlog→Queued · unstarted→Ready · started→InProgress · completed→Done · canceled→Abandoned`; unknown→Backlog).
- **`LinearStateType` derives `Default` (`#[default] Backlog`)** — matches the GitHub intermediate-enum convention + pre-empts a next-slice (Linear read client) `#[derive(Default)]` build break (code-quality MED, fixed in edges-013).

## Decisions explicitly NOT made (deferred)
- **All wiring** (Approach A): the executor arms (`github`/`linear.*`/`project.rescan`), the new `EventTypeRegistry` event types + their projectors (`proj_pull_request`/`proj_worktree`/`tasks`), and the registry migrations — gated on the daemon track's R1 seam + event types.
- **copy detection** (edges-011) — git2 0.21 can't; revisit on a newer libgit2 + a real consumer.
- **The real `LinearGraphqlReadClient`** (edges-014/015) — the reqwest GraphQL fetch + auth + the GraphQL-errors-as-200 mapping + the HTTP dep; **edges-014 was dispatched then ABANDONED at the Round-2 lead cut** (no commit; the brief `@6646a508` re-opens in Round 3).
- **§17-taxonomy refinement** (edges-009 LOW) — octocrab decode/protocol errors fold to retryable `ServerError`; a terminal-non-auth `IntegrationOutcomeClass` variant is an edges-003-classifier enrichment for the gated wiring.
- **Secondary signals** — assignee-id/timestamps (Linear), richer `LinearIssue` fields, per-hunk perf, the `open_diff` DRY refactor.

## TDD compliance
**Clean — no violations.** All 6 slices ran RED→Step-2.5→GREEN test-first; each Step-2.5 test-design write-up was orchestrator-reviewed (`APPROVED.`/`ADD:`) before GREEN. Spikes were throwaway (created → measured → removed before commit; never staged). Code-quality findings folded in-slice (008: 3-med; 009: 2-med; 011: 2-med; 012: 3-med+2-low; 013: 1-med+2-low) — all test-strengthening or forward-compat, no behavior regressions.

## Cross-doc invariant audit
**NONE — clean.** Multi-track memory check: `shared/` was **not touched** this round (verified `git diff --stat 6e36f47..d7a9458 -- ../shared/` is empty); every new model (`GithubReadError`, `ChangeKind::Renamed`/`FileChange.old_path`, `DiffHunk`/`DiffLine`/`DiffLineKind`, `LinearStateType`) is **daemon-internal** (no `shared/` surface, no CONTRACT bump, no schema-snapshot). Every "cross-doc: none" was confirmed at the slice's Step 9. The daemon-defined mapping tables + decode tables are **integration-owned arch-notes** flagged at Step 9 for the orchestrator's PLAN-DELTA hand-off (not edited from `track/edges`).

## Reachability
**Every slice is tested-but-unwired BY DESIGN (Approach A)** — each grep-confirmed at Step 7.5 that the new symbols are referenced only by their module + test (no production entry point):
- `parse_*`/`signals_from_github_response`, `extract_pr_signals`/`GithubReadClient`/`OctocrabGithubReadClient`, `parse_review_decision`/`layer_review_decision` → consumer = the gated `github` executor + `proj_pull_request` projector.
- `read_diff` (rename-aware) + `read_file_hunks`/`DiffHunk` → consumer = the gated `proj_worktree` projector + the §7.2 PR Review Workspace + the 6.7 diff-open bench.
- `parse_linear_state_type`/`derive_task_status_from_linear` → consumer = the gated Linear read client (edges-014/015) + the `tasks`(external_task) projector + §7.3 Task Inbox.
No wiring was removed by a later slice; no silent unreachable gap (the unwired state is the deliberate Approach-A posture).

## Open follow-ups (Step-9 categorized — already routed hot to the orch; for the phase-exit hand-off)
- **Arch-notes (integration-owned, orch routes):** §9/§7.2 GitHub decode tables + the injected-handle/REST-can't-see-ReviewRequired/CheckRun-no-status notes; §9 rename-detection-ON + per-hunk-read-shape; §5.1/§9 Linear `WorkflowState.type`→`Task` mapping table.
- **C-list lessons (orch routes):** GitHub two-stage decode/aggregate; thin-glue read client (octocrab + classify, fake-covered fetch, error carries `IntegrationOutcomeClass`); REST/GraphQL two-source review state (GraphQL authoritative, best-effort degrade, injection-safe variables); git2 rename (`for_untracked` gotcha, copy-unavailable) + per-hunk (binary `Some`-0-hunks, EOFNL→Context, rename-aware new-path match); Linear external-task derivation.
- **Carry-forwards:** edges-014/015 the real Linear `LinearGraphqlReadClient`; the wiring slices (R1 seam + event types); the §17 terminal-non-auth class; the `open_diff` DRY refactor; copy detection; huge-diff perf; secondary signals (assignee-id/timestamps/richer fields); the `octocrab` dep → `cargo audit` at the P7.1 phase-exit (+ TLS-backend / no-credential-auto-discovery review at the auth-bootstrap slice); the gated `SyncFailed` persist path MUST Redact `GithubReadError.message` (§15).
- **Round-2 lead cut:** edges-014 abandoned cleanly (no commit) at the post-edges-013 seal; re-opens Round 3.

## How to use what was built
The GitHub-PR read chain (`detect_git`→…→`extract_pr_signals`+`layer_review_decision`→`derive_pull_request_status`) and the git diff backend (`read_diff`/`read_file_hunks`) and the Linear derivation (`parse_linear_state_type`→`derive_task_status_from_linear`) are pure, deterministic, fixture-tested cores ready for the gated wiring to consume once the daemon track delivers the R1 executor-registration seam + the Phase-5/7 event types.

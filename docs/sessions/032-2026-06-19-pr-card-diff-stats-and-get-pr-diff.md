# Session 032 — PR-card diff-stats (D6) + get_pr_diff read RPC (D7)

- **Date:** 2026-06-19
- **Phase:** Phase 4 / **P4.7** (the ui-unblock work-order wave-2 — read-surface enrichments for the §11.2 PR Review Workspace; homed under Phase 4 like 4.4/4.5/4.6).
- **Predecessor:** [031 — VT-arc tie-off + build-hygiene (075e)](031-2026-06-18-vt-arc-tieoff-and-build-hygiene.md)
- **Successor:** _(next implementer session — D9 cat-1 PR-mutation safety-design, post-cycle)_

## Why this session existed

The `docs/planning/daemon-unblock-work-order.md` wave-2 (D6, D7) unblocks the ui PR Review Workspace. Both are read-surface, NON-cat-1, additive-contract slices: D6 surfaces PR diff-stats on the card; D7 adds the remote-PR code-diff the Review tab renders. Two `/tdd` slices, sealed for a **wholesale team cycle** (lead-approved) before the cat-1 D9/D10 PR-mutation arc.

## What was built

### D6 — PR-card diff-stats (commit `ca50ca1`)
The D5a / LESSON-53 **LOCKSTEP** enrichment: `additions`/`deletions`/`changed_files`/`commits` (`Option<u64>`) end-to-end. CONTRACT **0.38.0→0.39.0**.

**Files modified:** `shared/src/events.rs` (PullRequestSynced +4), `shared/src/projections.rs` (PullRequestRow +4), `shared/src/lib.rs` (CONTRACT 0.39.0), `daemon/src/eventstore/schema.rs` (MIGRATION_15 ALTER) + `migrations.rs` (SUPPORTED_USER_VERSION 14→15 + register), `daemon/src/projections/pull_request.rs` (fold INSERT + ON CONFLICT), `daemon/src/integrations/{pull_request.rs (PullRequestSignals +4), github.rs (extract_pr_signals captures from `pr.additions`…), executor.rs (threads `created.signals.*` into the create_pr emit)}`, `daemon/src/ipc/methods.rs` (clarifying comment — the generic `read_table_as_json` auto-serves the 4 INTEGER cols as `Option<u64>`, no coercion), `shared/contracts/schema/…json` (regen). **Tests:** 5 new (fold + typed-serve + MIGRATION_15 + extract-capture + create_pr-emit-threading) + the frozen-shape field-sets / sample builders / version pin / gateway_plan migration pin.

### D7 — get_pr_diff read RPC (commit `924ebcc`)
A NEW `get_pr_diff(repo_id, pr_number, file?) → DiffResult` §6.1 read RPC — the remote-PR head-vs-base code-diff (distinct from the worktree-scoped `get_diff`; a PR has no worktree FK). **The first network read in the IPC layer.** CONTRACT **0.39.0→0.40.0**.

**Files created:** `daemon/tests/get_pr_diff.rs` (parser ×3 + adversarial-garbage + CRLF + handler ×3).
**Files modified:** `shared/src/ipc.rs` (NEW `GetPrDiffParams{repo_id, pr_number, file: Option<String>}`; `DiffResult`/`Hunk`/`DiffLine` REUSED), `shared/src/lib.rs` (CONTRACT 0.40.0), `shared/src/schema.rs` (register `GetPrDiffParams`), `daemon/src/git/mod.rs` (`parse_unified_diff` + helpers — CRLF-tolerant, panic-free), `daemon/src/integrations/github.rs` (`fetch_pr_diff` on the trait + Fake `with_diff` + Octocrab `get_diff`), `daemon/src/ipc/methods.rs` (`get_pr_diff` arm + `read_pr_diff` + `resolve_pr_owner_repo` + `parse_owner_repo` + a `parse_owner_repo` unit battery), `daemon/src/ipc/server.rs` + `daemon/src/runtime/listener.rs` (thread `Arc<dyn GithubReadClient>` through `serve_connection`/`spawn_accept_loop`), `daemon/src/main.rs` (construct the **unauthenticated** `OctocrabGithubReadClient`), `daemon/src/ipc/mod.rs` (export `read_pr_diff`), `shared/contracts/schema/…json` (regen). **Tests:** contract `GetPrDiffParams` snapshot + version 0.40.0; runtime.rs/ipc.rs `fake_github` wiring fixups.

## Decisions made
- **D6:** Q1 `Option<u64>` (mirrors D5a; octocrab `Option<u64>`; rebuild-safe None→NULL). Q2 producer = extract + `create_pr` emit only (the PR-status-refresh sync stays deferred, same gating as D5a). Q3 octocrab fields confirmed vs the **vendored 0.53.1 source**. Q4 +4 fields on both frozen shapes; collapse to 1 commit.
- **D7 (Q4 rulings, orchestrator-decided, read-surface internal):** (a) wiring = `Arc<dyn GithubReadClient>` + a captured `Handle` threaded through `serve_connection`→`dispatch`, LESSON-46 `block_on`+mandatory timeout, test-seamed (`FakeGithubReadClient`); (b) resolution = the EXACT `(repo_id, pr_number)` PR row → `project_id` → `proj_repository.remote_url` → parse owner/repo, unresolvable→NotFound; (c) auth = thread the seam + handler NOW (fake-tested), DEFER the production per-repo keychain auth → the production client is constructed **unauthenticated** (a public-repo fetch works; private → a typed error until auth lands); (d) `file:None` = flattened whole-changeset (the flat `DiffResult` carries no per-file attribution).
- Q1 (octocrab) confirmed: `pulls(owner,repo).get_diff(pr) → String` (the `vnd.github.diff` unified-diff string), vendored 0.53.1 `pulls.rs:137`.

## Decisions explicitly NOT made (deferred)
- The per-repo keychain **auth bootstrap** (the production GitHub read client is unauthenticated) — its own slice (LESSON 28/43/49 token→keychain), **carrying a mandatory security re-review gate** (the cross-project `resolve_pr_owner_repo` confused-deputy concern must be re-reviewed BEFORE private fetch goes live).
- The **PR-status-refresh sync** (D6: `fetch_pr_signals` exists but isn't emit-wired) — populates diff-stats (+ D5a mergeable/checks) for EXISTING synced PRs; today they populate only on `create_pr`.
- The Review-tab **file-tree** (D7: the flat `DiffResult` has no per-file attribution; `file:None` flattens) — a changed-files list + per-file grouping is a post-D7 follow-on (the work-order honored "reuse DiffResult").
- Deferred code-quality lows: a `repo_id→owner/repo` projection link (repo_id isn't first-class in `proj_repository`) · the `dispatch` arg-count smell · a parser rename-edge test.
- **D8** deferred; **5.3a** stays parked.

## TDD compliance
**Clean.** D6: the 5 new tests written FIRST, RED-confirmed (missing struct fields), then GREEN. D7: the parser + handler tests written FIRST, RED-confirmed (missing `parse_unified_diff`/`read_pr_diff`/`GetPrDiffParams`/`with_diff`); the handler tests were finalized against the Step-2.5-ruled resolution/wiring before GREEN. The Step-8 review-response additions (CRLF test, adversarial-garbage test, `parse_owner_repo` battery) are hardening/characterization tests over already-test-first code, not violations. Nothing safety-critical skipped.

## Cross-doc invariant audit
**Both contract changes have paired doc edits in the working tree** (single-track — the orchestrator wrote the rows hot, uncommitted, for its `/orchestrate-end` seal; `ARCHITECTURE.md` + `daemon/CLAUDE.md` show modified in `git status`):
- D6: `PullRequestSynced` + `PullRequestRow` each +4 `Option<u64>` → Appendix-A `PullRequestRow` row + the EventTypeRegistry `PullRequestSynced` note + the MVP-projections [4.6/4.7] note + CONTRACT 0.39.0. Flagged at Step 9; orchestrator confirmed.
- D7: NEW `GetPrDiffParams` + the `get_pr_diff` §6.1 method → the IPC `GatewayPort` row + the §6.1/Appendix-A IPC-method note + CONTRACT 0.40.0 + LESSON §59. Flagged at Step 9; orchestrator confirmed.
No drift; no violation.

## Reachability
- **D6** — no new entry; the 4 diff-stats ride existing wired paths: read `get_projection(PullRequest)`→`read_pull_request_typed`; producer `PullRequestSynced` via the `github.create_pr` executor; fold in the event-commit txn; MIGRATION_15 at `EventStore::open`. Populated on `create_pr` now; existing synced PRs at the deferred refresh-sync.
- **D7** — `get_pr_diff` reachable from the UDS gateway: `serve_connection` → `dispatch("get_pr_diff")` → `read_pr_diff` → resolve + `client.fetch_pr_diff` + `parse_unified_diff`. The production client is WIRED in `main.rs` (unauthenticated); the live private-repo fetch is gated on the deferred per-repo keychain auth.
No tested-but-unwired gaps.

## Open follow-ups (Step-9 categorized; routed hot — orchestrator owns)
- **Cross-doc invariant changes** (D6 + D7): the contract rows + CONTRACT 0.39.0/0.40.0 (orchestrator's `/orchestrate-end`).
- **Architecture doc note** (D7): the IPC read layer gains NETWORK-CLIENT access — record the wiring seam + the unauthenticated-client / auth-deferral posture.
- **LESSON §59** (D7, orchestrator-logging): the initiator-based rule — UI-peer network read = read RPC (LESSON 33) + mandatory timeout (LESSON 46); agent-proposer = risk-1 Gateway action (LESSON 55).
- **Future TODO — belongs-to-a-phase:** the per-repo keychain auth-bootstrap (with the mandatory security re-review gate) · the PR-status-refresh sync (D6) · the Review-tab file-tree (D7).
- **Future TODO — security re-review:** `resolve_pr_owner_repo` cross-project confused-deputy — acceptable at MVP (getpeereid-trusted local peer + read-only public), re-review when auth lands.
- **Deferred lows:** repo_id-not-first-class projection link · dispatch arg-count · parser rename-edge test.

## Gate
- D6: suite 937/0; clippy clean; **3-way verify PASS** (CONTRACT 0.39.0, 41 enums, self-health green). security-reviewer SKIPPED per policy (NON-cat-1, no invariant/secret).
- D7: `cargo test --workspace` **EXIT=0 (947/0)**; clippy `-D` clean; **3-way verify PASS** (CONTRACT 0.40.0, 41 enums, self-health green). **security-reviewer CLEAR every invariant dimension** (§15 no-leak structural, userinfo-strip layered, INV-SEC-1 read-only, panic-free parser, mandatory-timeout, honest auth-deferral) — no Step-9 Finding.
- `/preflight` (this session): see the handoff (run at close-out).

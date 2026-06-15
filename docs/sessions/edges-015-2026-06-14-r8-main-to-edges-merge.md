# edges-015 — R8: main→edges merge re-sync (CONTRACT 0.26→0.32)

**Date:** 2026-06-14
**Role:** edges-daemon-implementer (R8 — the phase-exit/merge round; single merge actor)
**Predecessor:** [edges-014-2026-06-13-r7-orchestrator-round-seal.md](edges-014-2026-06-13-r7-orchestrator-round-seal.md) (R7 orch round-seal, `7b4fd37`)
**Successor:** [edges-016-2026-06-14-r8-orchestrator-merge-seal.md](edges-016-2026-06-14-r8-orchestrator-merge-seal.md) (R8 orch merge-round seal)
**Merge commit:** `536ac04` (2 parents: `1f1f14f` track/edges + `df19f89` main) · branch `track/edges` · **NOT pushed, NOT merged to main** (the edges→main merge is the user's later coordination)

> **Companion (the authoritative accumulated cross-track ledger):** `docs/planning/edges-R5-wiring-plan.md`. This doc is the merge-side narrative.

## Why this session existed
Re-sync the edges track with the daemon track (`main`) before the user-gated edges→main phase-exit. `main` had advanced its whole **Phase-4 + Codex-3.3 arc** (CONTRACT **0.26.0 → 0.32.0**); merging edges→main cleanly later requires edges to first absorb main's changes. The orchestrator started a `git merge --no-commit main` (7 conflicts) and handed the resolution to a single merge actor (no race). NOT a `/tdd` slice — a merge reconciliation (resolve → verify → security-review → commit → HOLD).

## What was built
**No new feature code** — this is a merge integration. Absorbed main's arc: 4.0b-2 live interception · 4.0c telemetry pump · 4.1a/b survival + tmux broker · 4.2 SessionFailed · 4.3 background jobs (WAL checkpointer + staleness poller) · 3.3a/b Codex adapter. The merge committed 122 files (auto-merged from main + the 7 resolved conflicts).

**Files conflicted + resolved (7):**
- `daemon/src/lib.rs` — additive union: `pub mod integrations;` (edges) + `pub mod integrity;` (main).
- `daemon/src/runtime/mod.rs` — additive union: `mod git_watcher;` + `pub mod jobs;`; re-exports unioned (`spawn_git_watcher` + `compute_worktree_cache` kept in the writer export, alongside main's `jobs`/`telemetry_sink`/`wait_class`).
- `daemon/Cargo.toml` — dev-deps union: main's `tokio` superset (incl. `test-util`) + edges' `reqwest` dev-dep + main's `git2` dev-dep. **Plus a beyond-plan dedup** (see Decisions).
- `daemon/src/git/mod.rs` (add/add) — edges' submodule index (`cli`/`detect`/`executor`/`precedence`/`reads`) + main's `read_diff`/`GitReadError` (the ui get_diff per-hunk backend). No collision: `git::read_diff(repo_path,file)→DiffResult` (main) vs `git::reads::read_diff(path,from,to)→Vec<FileChange>` (edges) — different paths/sigs.
- `daemon/src/runtime/writer.rs` — union all 4 regions: imports (edges' `git::precedence`/`reads` + main's `intercept`/`integrity`); the `Command` enum (`RefreshWorktreeStatus` + `WalCheckpoint`); the `WriteHandle` methods; the handler arms.
- `daemon/src/main.rs` (CAT-1) — folded edges' Project/Git/Github/Linear executor registrations into main's **live INV-SEC-1 drive loop** (under `AgentMutationPolicy` + the registered `SessionExecutor` + `spawn_with_alarm_and_breaker`, replacing edges' old `CatalogPolicy` + plain `WriteActor::spawn`); unioned the spawns (git-watcher + staleness + checkpointer + restart-recovery) + the shutdown awaits.
- `Cargo.lock` — regenerated (see Decisions).

## Decisions made
- **main.rs cat-1 integration:** take main's live drive loop as the base; register edges' 4 executors into the SAME `catalog_exec` so they run under the LIVE `AgentMutationPolicy` (catalog-authoritative — risk-2/3 edges mutators still approval-gated) + the interception + the audit-backbone breaker. This is the correct INV-SEC-1-preserving integration (confirmed by the security review — see TDD/Reachability).
- **`git/mod.rs` add/add:** combine rather than pick a side — edges' submodules and main's `read_diff` are both git read-only concerns that coexist (distinct symbol paths).
- **Beyond-plan #1 — `[dependencies]` git2 TOML duplicate:** the auto-merge left TWO `git2` keys in `[dependencies]` (edges' `{vendored-libgit2, default-features=false}` + main's plain `git2 = "0.21"` from P4.0b-ui1) — a duplicate-key error NOT surfaced as a conflict (the two entries sat at different line positions). Deduped: kept edges' vendored entry (the forbidden-#6 read-only / OQ-INT-SPIKE-6 posture, a strict feature subset), removed main's plain one. main's `read_diff` uses only core libgit2 diff APIs → satisfied by the vendored build. Verified (get_diff tests pass).
- **Beyond-plan #2 — Cargo.lock:** resolved via `git checkout --theirs Cargo.lock` (main's lock, the larger arc) + `cargo check` regenerated it (added edges' octocrab/reqwest/async-trait), rather than `cargo generate-lockfile` — preserves main's pins + adds edges' (minimal, consistent).

## Decisions explicitly NOT made (deferred)
- **No push, no merge to main, no `/phase-exit`** — held per the lead/orch; the edges→main direction + the phase-exit are the user's later coordination.
- **Wave-C `integration_connections` (MIGRATION_10)** — D8 resolved (main holds MIGRATION_9 as a POLICY_DECISION → edges' Wave-C = MIGRATION_10), but Wave-C is unbuilt; the next round (R9) builds it. No edges migration to renumber now.
- **A fresh full cargo audit on the merged lock** — flagged to the orch; the orch ran it → the same rsa-only finding (RUSTSEC-2023-0071, no new findings from main's Phase-4/Codex deps).

## TDD compliance
**N/A — merge reconciliation, not a feature slice** (the orch sanctioned no RED→GREEN). No behavior authored from scratch: main's Phase-4/Codex code was TDD'd on main; edges' code was TDD'd in prior rounds; this session only integrates them. The integration is verified by the full suite (760/0/0) + `cargo check` + `clippy -D` + `fmt --check` + the **cat-1 security review** (the load-bearing gate). No violation.

## Cross-doc invariant audit
**No edges-authored model field change this session.** The CONTRACT bump (0.26→0.32) is main's, absorbed wholesale via the merge (main's `shared/` + `ARCHITECTURE.md` came in with the merge commit). Multi-track memory check: nothing edges-side to flag — no paired `ARCHITECTURE.md` edit owed by edges. The merged tree is internally consistent (CONTRACT_VERSION = 0.32.0; the schema-snapshot + migration tests are among the 760 passing).

## Reachability
- **edges' 4 mutators (Project/Git/Github/Linear):** reachable from the accept-loop client path (`submit_action`/`approve` → write-actor → Gateway pipeline → `CatalogExecutor` dispatch → the edges executor) — and the security review confirmed they are reachable **ONLY** via that pipeline (INV-SEC-1 no-bypass; the sole executor invocation is `pipeline.rs:976`, downstream of policy+approval+audit). No second dispatch path.
- **git-watcher (edges-026):** reachable from `main.rs run()` → `spawn_git_watcher` (preserved through the merge).
- main's Phase-4/Codex features are reachable per main's own wiring (absorbed). No tested-but-unwired gaps.

## Open follow-ups (already routed hot to the orch / the wiring-plan ledger)
- **Wave-C (R9):** `integration_connections` MIGRATION_10 + `IntegrationConnectionRegistered` + the P5.1 registry projector → then `/phase-exit 5`+`7` → seal → HOLD for the user's edges→main.
- **cargo audit:** RUSTSEC-2023-0071 (rsa Marvin-Attack, MEDIUM, no fix) — same finding on the merged lock; exposure LOW (no GitHub-App RS256-JWT); accept+document; octocrab feature-prune follow-up.
- **Carried from R5–R7** (the wiring-plan ledger): the overlay-source MIGRATION_9-deferred follow-on, the unified write-actor-I/O-offload SPREAD, the §15 over-redaction return-review item, the subscribe-delta gap, the `/tdd`-Step-8-add-`cargo fmt --check` convention, the held-for-merge arch-notes + LESSON candidates.

## Security (cat-1) — the load-bearing gate
**`INV-SEC-1: PASS (no-bypass confirmed)`** (security-reviewer, 6 criteria, file:line evidence):
1. No bypass — edges' mutators reachable only via the Gateway pipeline; they hold no WriteHandle/eventstore/submit, run no SQL, emit only via `emitted_events`; `CatalogExecutor` dispatch runs AFTER `requires_resource_refs` + the Adjudication guard.
2. `AgentMutationPolicy` catalog-authoritative — only raises `agent.*` to Deny, else falls through to `CatalogPolicy` → edges' risk-2/3 still require approval (not auto-allowed); risk-0 `project.rescan` auto-execute is intentional (read-only).
3. `FailedWithEvents` audited atomically (ActionFailed + `*SyncFailed` in txn-B, §15 structural reason).
4. Fail-closed on audit-write — edges events through the breaker-gated write-actor.
5. No un-intercepted-live window — `test_live_session_create_has_interception` still meaningful (in the 760).
6. §15 secrets clean — unauthenticated injected clients, no secret constructed/logged, fail-closed deferred-auth.

## Test count
edges 620 → **760 passed / 0 failed / 0 ignored** workspace (absorbed main's Phase-4 + Codex arc). `cargo check` + `clippy -D warnings` + `cargo fmt --check` clean. CONTRACT_VERSION 0.32.0.

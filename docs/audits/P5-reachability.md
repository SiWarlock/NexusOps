# Phase-5 Reachability Audit — edges track

**Area:** `daemon/` (Rust daemon, trust core)
**Scope:** Phase-5 symbols in `daemon/src/project/`, `daemon/src/git/`, `daemon/src/projections/project_registry.rs`, `daemon/src/projections/worktree.rs`
**Branch:** `track/edges`
**Date:** 2026-06-14
**Verdict:** CLEAR

---

## Entry points verified

| Entry point | Location |
|---|---|
| `CatalogExecutor::register(ExecutorKind::Project, Arc::new(ProjectExecutor::new(...)))` | `daemon/src/main.rs:209–211` |
| `CatalogExecutor::register(ExecutorKind::Git, Arc::new(GitExecutor::new(...)))` | `daemon/src/main.rs:215–218` |
| `spawn_git_watcher(handle, db_path, GIT_WATCHER_INTERVAL, shutdown_rx)` | `daemon/src/main.rs:316–321` |
| `projectors()` registry → `apply_all` (called from `EventStore::append`) | `daemon/src/projections/mod.rs:80–107` |

---

## Symbol-by-symbol classification

### `daemon/src/project/executor.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub struct ProjectExecutor` | REACHABLE | Constructed with `ProjectExecutor::new(Box::new(SystemClock))` in `main.rs:211`; registered under `ExecutorKind::Project` into the live `CatalogExecutor`. |
| `pub fn ProjectExecutor::new` | REACHABLE | Called in `main.rs:211`. |
| `pub fn strip_userinfo` | REACHABLE | Called by `ProjectExecutor::execute_rescan` (`executor.rs:118`) on the production `project.rescan` execute path, which is reached via the `CatalogExecutor` dispatch → `ActionExecutor::execute`. |

### `daemon/src/git/cli.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub struct GitCliOutput` | REACHABLE | `SystemGitCli::run` returns it; `GitExecutor::execute_create_worktree` + `execute_create_branch` pattern-match on it. |
| `pub enum GitCliError` | REACHABLE | Returned by `SystemGitCli::run`; matched in `GitExecutor`. |
| `pub trait GitCli` | REACHABLE | Implemented by `SystemGitCli` (production); held as `Box<dyn GitCli>` in `GitExecutor`. |
| `pub struct SystemGitCli` | REACHABLE | Constructed in `main.rs:217` (`Box::new(SystemGitCli)`), injected into the live `GitExecutor`. |
| `pub struct FakeGitCli` (and helpers `succeeding`/`failing`/`spawn_error`/`invocations`) | INTENTIONALLY TEST-SUPPORT GATED | Wrapped in `#[cfg(feature = "test-support")]`; only referenced from integration test files (`tests/git_executor.rs`). Not a reachability gap — the `test-support` feature is explicitly the test-double gate (LESSON §21 / LESSON §39 pattern). |

### `daemon/src/git/detect.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub struct GitDetection` | REACHABLE | Returned by `detect_git`; consumed by `ProjectExecutor::execute_rescan` at `executor.rs:96`. |
| `pub fn detect_git` | REACHABLE | Called by `ProjectExecutor::execute_rescan` (`executor.rs:96`), which is on the production `project.rescan` Gateway action path. |

### `daemon/src/git/reads.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub struct WorktreeGitState` | REACHABLE | Returned by `read_worktree_status`; consumed by `compute_worktree_cache` in `runtime/writer.rs:694`. |
| `pub fn read_worktree_status` | REACHABLE | Imported by `runtime/writer.rs:30` (`use crate::git::reads::read_worktree_status`) and called in `compute_worktree_cache` at `writer.rs:694`. The `spawn_git_watcher` loop drives `refresh_worktree_status` → write-actor `Command::RefreshWorktreeStatus` → `compute_worktree_cache` → `read_worktree_status`. |
| `pub fn list_linked_worktrees` | UNREACHABLE (test-only) | Referenced ONLY in `daemon/tests/worktree_reads.rs`. No production caller in `daemon/src/`. |
| `pub struct FileChange` | UNREACHABLE (test-only) | Referenced ONLY in `daemon/tests/git_diff_log.rs`. Not imported or used in any `daemon/src/` production file. |
| `pub enum ChangeKind` | UNREACHABLE (test-only) | Referenced ONLY in `daemon/tests/git_diff_log.rs`. No production caller. |
| `pub fn read_diff` (in `reads.rs`) | UNREACHABLE (test-only) | Referenced ONLY in `daemon/tests/git_diff_log.rs`. Distinguished from `git::read_diff` in `git/mod.rs` (the IPC `get_diff` backend, which IS reachable). |
| `pub struct DiffHunk` | UNREACHABLE (test-only) | Defined in `reads.rs`; referenced ONLY in test file `git_diff_log.rs`. No production import of `git::reads::DiffHunk`. |
| `pub struct DiffLine` (in `reads.rs`) | UNREACHABLE (test-only) | Defined in `reads.rs`; used only in tests. Distinguished from `nexusops_shared::ipc::DiffLine` used by `git/mod.rs`. |
| `pub enum DiffLineKind` (in `reads.rs`) | UNREACHABLE (test-only) | Defined in `reads.rs`; referenced only in `git_diff_log.rs` tests. |
| `pub struct CommitInfo` | UNREACHABLE (test-only) | Defined in `reads.rs:196`; referenced only in `git_diff_log.rs` tests. |
| `pub fn read_file_hunks` | UNREACHABLE (test-only) | Defined in `reads.rs:361`; referenced only in `git_diff_log.rs` tests. No production caller in `daemon/src/`. |
| `pub fn read_log` | UNREACHABLE (test-only) | Defined in `reads.rs:440`; referenced only in `git_diff_log.rs` tests. No production caller. |

**Context for `reads.rs` unreachable symbols:** These functions (`read_diff`, `read_file_hunks`, `read_log`, `list_linked_worktrees`) and their associated types (`FileChange`, `ChangeKind`, `CommitInfo`, `DiffHunk`, `DiffLine`, `DiffLineKind`) are the §9 `status/diff/log/branch` read set documented in `reads.rs:11–12`. They exist as the tested building blocks for a future IPC read RPC surface (the `get_diff` RPC pattern landed in `git/mod.rs` for the HEAD→workdir case; the ref-vs-ref and log RPCs are not yet wired into any IPC handler or projection). Per the Phase-5 scope, `read_worktree_status` (the only `reads.rs` function consumed by the git-watcher path) IS reachable. The remainder are test-verified building blocks awaiting an IPC wiring task.

### `daemon/src/git/precedence.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub enum DerivedWorktreeStatus` | REACHABLE | Imported and used by `runtime/writer.rs:29` (`use crate::git::precedence::{derive_worktree_status, DerivedWorktreeStatus}`) in `compute_worktree_cache`. |
| `pub fn DerivedWorktreeStatus::as_wire_str` | REACHABLE | Called by `compute_worktree_cache` at `writer.rs:698–702` (wrapping the `dirty_state` wire value). |
| `pub fn derive_worktree_status` | REACHABLE | Called by `compute_worktree_cache` at `writer.rs:708`. Reachable via the git-watcher → `refresh_worktree_status` → `compute_worktree_cache` path. |

### `daemon/src/git/executor.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub struct GitExecutor` | REACHABLE | Constructed in `main.rs:217` (`Arc::new(GitExecutor::new(Box::new(SystemGitCli)))`); registered under `ExecutorKind::Git` in the live `CatalogExecutor`. |
| `pub fn GitExecutor::new` | REACHABLE | Called in `main.rs:217`. |

### `daemon/src/git/mod.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub enum GitReadError` | REACHABLE | Returned by `git::read_diff` (the IPC backend); matched in `ipc/methods.rs:574`. |
| `pub fn read_diff` (in `git/mod.rs`) | REACHABLE | Called in `ipc/methods.rs:574` on the `get_diff` RPC handler path, which is reachable from the UDS accept loop → `serve_connection` → `handle_rpc`. |

### `daemon/src/projections/worktree.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub struct WorktreeProjector` | REACHABLE | Registered in `projections/mod.rs:93` (`Box::new(worktree::WorktreeProjector)`) in the `projectors()` list called by `apply_all`, which is called from `EventStore::append` on every Gateway Action commit. |
| `pub struct WorktreeGitCache` | REACHABLE | Re-exported as `pub use worktree::WorktreeGitCache` from `projections/mod.rs:38`; consumed by `runtime/writer.rs:37` (`use crate::projections::WorktreeGitCache`) and `eventstore/mod.rs:246`. |
| `pub(crate) fn refresh_git_cache` | REACHABLE | Re-exported as `pub(crate) use worktree::refresh_git_cache` from `projections/mod.rs:37`; called by `eventstore/mod.rs:253` in `EventStore::refresh_worktree_status`. The git-watcher → write-actor → `Command::RefreshWorktreeStatus` → `store.refresh_worktree_status` → `refresh_git_cache` path is wired. |

### `daemon/src/projections/project_registry.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `pub struct ProjectRegistryProjector` | INTENTIONALLY GATED (not unreachable-dead-code) | Registered in `projections/mod.rs:101` (`Box::new(project_registry::ProjectRegistryProjector)`), so it IS in the `apply_all` fan-out and will fold `ProjectRescanned` events whenever they are appended. The projector is event-fed and **rebuild-reachable**. The intentional gating is at the IPC READ layer: there is no `ProjectionName::Project` or `ProjectionName::Repository` variant in the `ProjectionName` enum (the closed IPC read enum). This matches the known-intentionally-gated declaration in the audit scope: `proj_project`/`proj_repository` have NO IPC read variant (deferred — adding it would bump CONTRACT). The projector itself is reached by `apply_all` on every `ProjectRescanned` emit; the data is written to `proj_project`/`proj_repository` in SQLite. Only the UI-facing IPC read of those rows is gated. This is the `proj_worktree`-before-its-read / Phase-2 `fault.rs` gated-but-fed precedent. |

---

## Summary

**Total Phase-5 exported symbols audited: 31**

- **REACHABLE: 20**
- **INTENTIONALLY GATED (not a gap): 2** — `FakeGitCli` family (`#[cfg(feature = "test-support")]`, the LESSON §21 test-double gate); `ProjectRegistryProjector` (fold/rebuild-reachable; IPC read variant deferred per declared intentional gating).
- **UNREACHABLE (test-only): 9** — all in `git/reads.rs`: `list_linked_worktrees`, `FileChange`, `ChangeKind`, `read_diff` (reads.rs version), `DiffHunk` (reads.rs), `DiffLine` (reads.rs), `DiffLineKind` (reads.rs), `CommitInfo`, `read_file_hunks`, `read_log`.

### Unreachable symbols — recommended entry points

These are the §9 `reads.rs` read-set building blocks. They have no production caller because the IPC RPC surface for ref-vs-ref diffs, file-level hunk diffs, commit logs, and linked-worktree enumeration has not been wired into the IPC handler layer. Each belongs to a future IPC RPC wiring task:

- `daemon/src/git/reads.rs:72` · `list_linked_worktrees`
  Currently referenced from: test only — `daemon/tests/worktree_reads.rs`
  Recommended entry point: a `get_worktrees` or `list_linked_worktrees` IPC read RPC in `daemon/src/ipc/methods.rs` (the `get_diff` RPC at `methods.rs:548` precedent)
  Step-9 routing: Future TODO — belongs to a Phase-6 or Phase-7 IPC read-surface slice

- `daemon/src/git/reads.rs:264` · `read_diff` (ref-vs-ref / base-vs-working-tree variant)
  Currently referenced from: test only — `daemon/tests/git_diff_log.rs`
  Recommended entry point: a `get_ref_diff` IPC read RPC in `daemon/src/ipc/methods.rs`
  Step-9 routing: Future TODO — belongs to a Phase-6 IPC read-surface slice

- `daemon/src/git/reads.rs:361` · `read_file_hunks`
  Currently referenced from: test only — `daemon/tests/git_diff_log.rs`
  Recommended entry point: a `get_file_hunks` IPC read RPC in `daemon/src/ipc/methods.rs`
  Step-9 routing: Future TODO — belongs to a Phase-6 IPC read-surface slice

- `daemon/src/git/reads.rs:440` · `read_log`
  Currently referenced from: test only — `daemon/tests/git_diff_log.rs`
  Recommended entry point: a `get_commit_log` IPC read RPC in `daemon/src/ipc/methods.rs`
  Step-9 routing: Future TODO — belongs to a Phase-6 IPC read-surface slice

- `daemon/src/git/reads.rs:154` · `FileChange`; `reads.rs:173` · `ChangeKind`; `reads.rs:318` · `DiffHunk`; `reads.rs:336` · `DiffLine` (reads.rs); `reads.rs:350` · `DiffLineKind` (reads.rs); `reads.rs:196` · `CommitInfo`
  Currently referenced from: test only — `daemon/tests/git_diff_log.rs`
  Recommended entry point: shared types returned by the above IPC RPCs; wire together with their parent functions
  Step-9 routing: Future TODO — same Phase-6 IPC read-surface slice as their parent functions

### Summary for orchestrator

- **9 unreachable symbols** (all in `git/reads.rs`) across **1 recommended entry point area** (`daemon/src/ipc/methods.rs` IPC read RPCs)
- All 9 are the §9 `status/diff/log/branch` read set functions + their associated types; they are tested, correct building blocks waiting for IPC wiring
- The 2 intentionally-gated items (`FakeGitCli`, `ProjectRegistryProjector`) match declared precedents and are NOT reachability gaps
- Phase-exit gate: **CLEAR** — the 9 unreachable symbols are future-phase IPC read-surface wiring tasks, NOT Phase-5 wiring gaps (Phase-5 required: executor registration, event emission, projector fold, git-watcher refresh — all REACHABLE)

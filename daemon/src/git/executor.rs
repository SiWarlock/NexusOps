//! The P5.2 edges git mutators (`ExecutorKind::Git`) — the FIRST real edges FS/git MUTATIONS.
//!
//! `GitExecutor` handles `git.create_worktree` (`git worktree add`, edges-020) and `git.create_branch`
//! (`git branch`, edges-021) by shelling to the git CLI via the injected [`GitCli`] seam —
//! **forbidden #6: NEVER git2 for mutations** (git2 stays read-only in `git::{detect,reads,
//! precedence}`). Each mutator validates the catalog `requires_resource_refs` precondition, guards its
//! operands against argument-injection ([`reject_dash_operands`]), runs the CLI, and on success emits
//! its lifecycle event (`WorktreeCreated` / `BranchCreated`) through the in-txn §15 gate via the
//! edges-019 `EmittedEvent::Namespaced` bridge. `side_effect_applied: true` — a real FS/git change → a
//! txn-B append fault yields the honest `ActionPartiallySucceeded` (LESSON 21), not a clean rollback.
//! `git.status`/`git.diff` (also `ExecutorKind::Git`) delegate to the inner stub (served via the read
//! path).

use std::path::Path;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::events::{BranchCreated, WorktreeCreated};
use nexusops_shared::ids::WorktreeId;
use nexusops_shared::time::Timestamp;

use crate::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecError, ExecutionOutcome,
};
use crate::git::cli::GitCli;

/// The git action types this executor handles directly (`ExecutorKind::Git`); the rest delegate.
const GIT_CREATE_WORKTREE: &str = "git.create_worktree";
const GIT_CREATE_BRANCH: &str = "git.create_branch";

/// Runs git mutations via the injected CLI seam (forbidden #6). Holds an inner [`CatalogExecutor`] for
/// the catalog precondition check + delegation of the non-`create_worktree` `git.*` actions.
pub struct GitExecutor {
    cli: Box<dyn GitCli>,
    inner: CatalogExecutor,
}

impl GitExecutor {
    pub fn new(cli: Box<dyn GitCli>) -> Self {
        Self {
            cli,
            inner: CatalogExecutor::new(),
        }
    }

    fn execute_create_worktree(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition (the repo IDENTITY) FIRST — this
        // path runs its own side effect, never reaching `inner.execute`'s validation (§6.3, the
        // SessionExecutor precedent).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }

        // Operational params come from `req.inputs` (the resource_ref is the repo IDENTITY for
        // audit/policy; inputs carry the operation — the session.create precedent).
        //
        // §7.2/§15 NOTE — real-input-fidelity (MVP-ACCEPT; edges-020 finding): on the APPROVE path
        // these inputs are read from the durable row, which is §15-REDACTED before INSERT
        // (`pipeline.rs:671-675` — the deferred "real-input-fidelity concern"). A HIGH-entropy path
        // component would be masked to `[REDACTED]` → a broken op. Real production worktree/repo paths
        // are LOW-entropy → survive; the high-entropy edge is the LESSON §13 recall-envelope FP
        // (OVER-redaction, never a leak — the §15 invariant HOLDS). The proper fix (a non-redacted
        // operational-input channel for approve-gated executors) is deferred hardening (routed to the
        // lead). Pinned by `tests/git_executor.rs` 9a (real CLI, raw inputs) + 9b (approve path,
        // low-entropy paths).
        let Some(repo_path) = string_input(req, "repo_path") else {
            return ExecutionOutcome::Failed(
                "git.create_worktree requires a non-empty inputs[\"repo_path\"]".to_string(),
            );
        };
        let Some(worktree_path) = string_input(req, "worktree_path") else {
            return ExecutionOutcome::Failed(
                "git.create_worktree requires a non-empty inputs[\"worktree_path\"]".to_string(),
            );
        };
        let Some(branch_name) = string_input(req, "branch_name") else {
            return ExecutionOutcome::Failed(
                "git.create_worktree requires a non-empty inputs[\"branch_name\"]".to_string(),
            );
        };
        let base_branch = string_input(req, "base_branch");

        // ARGUMENT-INJECTION guard (shared across both git mutators — see `reject_dash_operands`).
        let mut operands = vec![worktree_path.as_str(), branch_name.as_str()];
        operands.extend(base_branch.as_deref());
        if let Err(failed) = reject_dash_operands(&operands) {
            return failed;
        }

        // forbidden #6 — the mutation runs via the git CLI, NEVER a git2 mutating API. Canonical
        // option-before-operand order (POSIX-portable):
        //   git worktree add -b <branch_name> <worktree_path> [<base_branch>]
        let mut args = vec![
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            branch_name.clone(),
            worktree_path.clone(),
        ];
        if let Some(base) = &base_branch {
            args.push(base.clone());
        }
        match self.cli.run(&args, Path::new(&repo_path)) {
            Ok(out) if out.success => {}
            Ok(_) => {
                // a non-zero git exit — fail BEFORE any event (no phantom WorktreeCreated). The reason
                // is a STRUCTURAL class ONLY — raw git stderr can carry paths (§15), so it is never
                // surfaced into the (persisted, redaction-gated) `ActionFailed`.
                return ExecutionOutcome::Failed(
                    "git worktree add failed (non-zero exit)".to_string(),
                );
            }
            Err(e) => {
                // a spawn failure (git not runnable) — also fail before any event; `GitCliError`'s
                // Display is structural (no path content).
                return ExecutionOutcome::Failed(format!("git worktree add: {e}"));
            }
        }

        // success → mint a fresh `wt_` id (the worktree is created by THIS action; the
        // `ExecutionProfileId::new()` precedent — domain ids mint via `::new()`, persisted-once +
        // rebuild reads the persisted event so it is replay-safe) and emit `WorktreeCreated`.
        let payload = WorktreeCreated {
            worktree_id: WorktreeId::new(),
            path: worktree_path,
            branch_name,
            base_branch,
        };
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => return ExecutionOutcome::Failed(format!("serialize WorktreeCreated: {e}")),
        };

        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "git.create_worktree — created a worktree via the git CLI".to_string(),
            // a real on-disk worktree was created BEFORE txn-B → a lost terminal write yields the
            // honest `ActionPartiallySucceeded` (the worktree exists; the event didn't commit), NOT a
            // clean rollback (LESSON 21).
            side_effect_applied: true,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: WorktreeCreated::EVENT_TYPE,
                payload_json,
            }],
        }
    }

    fn execute_create_branch(&self, req: &ActionRequest) -> ExecutionOutcome {
        // validate the catalog `requires_resource_refs` precondition (the repo IDENTITY) FIRST.
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // operational params from `req.inputs` (the §7.2/§15 real-input-fidelity NOTE on
        // `execute_create_worktree` applies equally — production branch names/paths are low-entropy;
        // MVP-accept).
        let Some(repo_path) = string_input(req, "repo_path") else {
            return ExecutionOutcome::Failed(
                "git.create_branch requires a non-empty inputs[\"repo_path\"]".to_string(),
            );
        };
        let Some(branch_name) = string_input(req, "branch_name") else {
            return ExecutionOutcome::Failed(
                "git.create_branch requires a non-empty inputs[\"branch_name\"]".to_string(),
            );
        };
        // the optional start-point (`base`); `git branch <name>` with no start-point uses current HEAD.
        let base = string_input(req, "base");

        // ARGUMENT-INJECTION guard (shared across both git mutators — see `reject_dash_operands`).
        let mut operands = vec![branch_name.as_str()];
        operands.extend(base.as_deref());
        if let Err(failed) = reject_dash_operands(&operands) {
            return failed;
        }

        // forbidden #6 — via the git CLI, NEVER a git2 mutating API: git branch <name> [<start-point>].
        let mut args = vec!["branch".to_string(), branch_name.clone()];
        if let Some(start) = &base {
            args.push(start.clone());
        }
        match self.cli.run(&args, Path::new(&repo_path)) {
            Ok(out) if out.success => {}
            Ok(_) => {
                // a non-zero git exit — fail BEFORE any event; STRUCTURAL reason only (raw git stderr
                // can carry paths, §15).
                return ExecutionOutcome::Failed("git branch failed (non-zero exit)".to_string());
            }
            Err(e) => return ExecutionOutcome::Failed(format!("git branch: {e}")),
        }

        let payload = BranchCreated { branch_name, base };
        let payload_json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => return ExecutionOutcome::Failed(format!("serialize BranchCreated: {e}")),
        };
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "git.create_branch — created a branch via the git CLI".to_string(),
            // a real git branch was created BEFORE txn-B → honest ActionPartiallySucceeded on a lost
            // terminal write (LESSON 21).
            side_effect_applied: true,
            emitted_events: vec![EmittedEvent::Namespaced {
                event_type: BranchCreated::EVENT_TYPE,
                payload_json,
            }],
        }
    }
}

impl ActionExecutor for GitExecutor {
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError> {
        self.inner.validate(req)
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            GIT_CREATE_WORKTREE => self.execute_create_worktree(req),
            GIT_CREATE_BRANCH => self.execute_create_branch(req),
            // git.status/git.diff (also ExecutorKind::Git) are not handled → the inner side-effect-free
            // stub (no-op success, no event); served via the read path (get_projection(Worktree) + the
            // diff backend).
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}

/// ARGUMENT-INJECTION guard (defense-in-depth, INV-SEC-1; the edges-020 security HIGH, now SHARED
/// across both git mutators). `Command::args` is shell-free, but git parses a leading-`-` OPERAND as
/// an OPTION (e.g. `--no-checkout`/`--force`/`--orphan` would SILENTLY change the mutation → the
/// on-disk result diverges from the approved+audited Action). Reject any git-ARG operand starting with
/// `-`, fail-closed. `operands` are the values passed to git's arg parser ONLY — the cwd (`repo_path`,
/// passed via `Command::current_dir`) is NOT a git arg → not an injection vector, so callers exclude
/// it. Generalizes to every git/external mutator.
fn reject_dash_operands(operands: &[&str]) -> Result<(), ExecutionOutcome> {
    if operands.iter().any(|op| op.starts_with('-')) {
        return Err(ExecutionOutcome::Failed(
            "git operand must not start with '-' (argument-injection guard)".to_string(),
        ));
    }
    Ok(())
}

/// a non-blank string input — `None` if absent or whitespace-only (fail-closed for a required input;
/// the natural optionality for `base_branch`/`base`).
fn string_input(req: &ActionRequest, key: &str) -> Option<String> {
    req.inputs
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

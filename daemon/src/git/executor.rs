//! The `git.create_worktree` executor (P5.2, edges-020) — `ExecutorKind::Git`. The FIRST real edges
//! FS/git MUTATION.
//!
//! Handles `git.create_worktree` by shelling to the git CLI (`git worktree add`) via the injected
//! [`GitCli`] seam — **forbidden #6: NEVER git2 for mutations** (git2 stays read-only in
//! `git::{detect,reads,precedence}`). On success it mints a `WorktreeId` and emits `WorktreeCreated`
//! through the in-txn §15 gate via the edges-019 `EmittedEvent::Namespaced` bridge.
//! `side_effect_applied: true` — a real on-disk worktree → a txn-B append fault yields the honest
//! `ActionPartiallySucceeded` (LESSON 21), not a clean rollback. `git.status`/`git.diff`/
//! `git.create_branch` (also `ExecutorKind::Git`) delegate to the inner stub (no consumer this slice;
//! reads via the read path; `create_branch` → edges-021).

use std::path::Path;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::events::WorktreeCreated;
use nexusops_shared::ids::WorktreeId;
use nexusops_shared::time::Timestamp;

use crate::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecError, ExecutionOutcome,
};
use crate::git::cli::GitCli;

/// The git action type this executor handles directly (`ExecutorKind::Git`); the rest delegate.
const GIT_CREATE_WORKTREE: &str = "git.create_worktree";

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

        // ARGUMENT-INJECTION guard (defense-in-depth, INV-SEC-1): `Command::args` is shell-free, but
        // git parses a leading-`-` OPERAND as an OPTION — e.g. `base_branch = "--no-checkout"` would
        // SILENTLY change the create, so the on-disk worktree would diverge from the approved+audited
        // Action (an audit-integrity gap). Reject any git-arg operand that starts with `-`, fail-closed
        // BEFORE the CLI runs (mirrors the blank-input guard). `repo_path` is the cwd (not a git arg) →
        // exempt. Generalizes to every git/external mutator (edges-021+).
        for operand in [&worktree_path, &branch_name]
            .into_iter()
            .chain(base_branch.as_ref())
        {
            if operand.starts_with('-') {
                return ExecutionOutcome::Failed(
                    "git.create_worktree operand must not start with '-' (argument-injection guard)"
                        .to_string(),
                );
            }
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
}

impl ActionExecutor for GitExecutor {
    fn validate(&self, req: &ActionRequest) -> Result<(), ExecError> {
        self.inner.validate(req)
    }

    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        match req.action_type.as_str() {
            GIT_CREATE_WORKTREE => self.execute_create_worktree(req),
            // git.status/git.diff/git.create_branch (also ExecutorKind::Git) are not handled this slice
            // → the inner side-effect-free stub (no-op success, no event). status/diff are served via
            // the read path (get_projection(Worktree) + the diff backend); create_branch → edges-021.
            _ => self.inner.execute(req),
        }
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        self.inner.preview(req, generated_at)
    }
}

/// a non-blank string input — `None` if absent or whitespace-only (fail-closed for a required input;
/// the natural optionality for `base_branch`).
fn string_input(req: &ActionRequest, key: &str) -> Option<String> {
    req.inputs
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

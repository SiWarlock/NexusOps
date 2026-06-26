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

use std::path::{Path, PathBuf};

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
/// W1-git-stage/095 — the per-hunk INDEX mutations (risk-2, recoverable). `git apply --cached [-R]`.
const GIT_STAGE_HUNK: &str = "git.stage_hunk";
const GIT_UNSTAGE_HUNK: &str = "git.unstage_hunk";
/// W1-git-discard/096 — the per-hunk DESTRUCTIVE WORKING-TREE mutation (risk-3, IRREVERSIBLE). `git apply
/// -R` (NO --cached) + the (A) content-hash verify-before-destroy guard.
const GIT_DISCARD_HUNK: &str = "git.discard_hunk";

/// The §6.3 frozen hunk resource-ref separator (4.0b-ui1) — the `\x1f` unit separator.
const HUNK_REF_SEPARATOR: char = '\u{1f}';

/// (W1-git-stage/095) Resolve a `worktree_id` → its filesystem path — the seam the per-hunk git
/// mutators use to locate the worktree (the AUDITED resource-ref carries the id, the daemon resolves the
/// path — LESSON 63; NEVER `inputs`). Injected so the apply bodies are testable over a real repo without
/// a populated DB (the `ProfileLookup` precedent). `Send + Sync` for the `ActionExecutor` bound.
pub trait WorktreePathResolver: Send + Sync {
    /// `worktree_id → path`; `None` = unknown (NotFound-class — the `get_diff` posture until proj_worktree populates).
    fn resolve(&self, worktree_id: &str) -> Option<PathBuf>;
}

/// The default resolver — resolves NOTHING (every per-hunk action fails NotFound). `main.rs` swaps in
/// the real [`DbWorktreePathResolver`]; a `GitExecutor::new(cli)` without it is fail-closed for hunks.
struct UnresolvedWorktreePaths;
impl WorktreePathResolver for UnresolvedWorktreePaths {
    fn resolve(&self, _worktree_id: &str) -> Option<PathBuf> {
        None
    }
}

/// Production — resolves `worktree_id → proj_worktree.path` over a READ-ONLY WAL conn (the `get_diff`
/// resolution; the executor runs on the write-actor, so a 2nd READ conn is single-writer-safe — forbidden
/// #3 governs WRITES). `None` on any read fault / unknown id (fail-closed); NotFound until proj_worktree populates.
pub struct DbWorktreePathResolver {
    db_path: PathBuf,
}

impl DbWorktreePathResolver {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

impl WorktreePathResolver for DbWorktreePathResolver {
    fn resolve(&self, worktree_id: &str) -> Option<PathBuf> {
        use rusqlite::OptionalExtension as _;
        // read-only WAL — never a writable Connection (single-writer; Forbidden #3 / LESSON §3).
        let conn = crate::eventstore::open_read_only(&self.db_path).ok()?;
        let path: Option<String> = conn
            .query_row(
                "SELECT path FROM proj_worktree WHERE worktree_id = ?1",
                [worktree_id],
                |r| r.get(0),
            )
            .optional()
            .ok()?;
        path.map(PathBuf::from)
    }
}

/// Runs git mutations via the injected CLI seam (forbidden #6). Holds an inner [`CatalogExecutor`] for
/// the catalog precondition check + delegation of the non-handled `git.*` actions, and a
/// [`WorktreePathResolver`] for the per-hunk mutators (W1-git-stage/095).
pub struct GitExecutor {
    cli: Box<dyn GitCli>,
    resolver: Box<dyn WorktreePathResolver>,
    inner: CatalogExecutor,
}

impl GitExecutor {
    pub fn new(cli: Box<dyn GitCli>) -> Self {
        Self {
            cli,
            resolver: Box::new(UnresolvedWorktreePaths),
            inner: CatalogExecutor::new(),
        }
    }

    /// (W1-git-stage/095) Inject the worktree-path resolver (the per-hunk mutators need it). A builder so
    /// `GitExecutor::new(cli)` stays backward-compatible (create_worktree/create_branch don't use it).
    pub fn with_worktree_resolver(mut self, resolver: Box<dyn WorktreePathResolver>) -> Self {
        self.resolver = resolver;
        self
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

    /// (W1-git-stage/095) The shared body for `git.stage_hunk` (`reverse=false`) / `git.unstage_hunk`
    /// (`reverse=true`) — they differ ONLY by the `-R` reverse flag. Re-derive the targeted hunk from the
    /// LIVE diff (the frozen position-only resource-ref carries LOCATION, not content — 4.0b-ui1), build a
    /// one-hunk patch, and apply it to the INDEX via `git apply --cached [-R]`, with `git apply --check`
    /// FIRST as the §17 read↔mutate race guard. NO domain event (the worktree git-axis is a live-read
    /// cache — LESSON 47; the UI re-reads `get_diff`). Contract-neutral.
    fn execute_apply_hunk(&self, req: &ActionRequest, reverse: bool) -> ExecutionOutcome {
        let label = if reverse {
            GIT_UNSTAGE_HUNK
        } else {
            GIT_STAGE_HUNK
        };
        // validate the catalog `requires_resource_refs` precondition FIRST (this path runs its own side
        // effect, never reaching `inner.execute`'s validation) — the create_worktree precedent.
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // decode the frozen position-only hunk resource-ref (LESSON 63 — resolve from the AUDITED ref,
        // never `inputs`). A malformed id → Failed BEFORE any git call.
        let Some(rref) = req.resource_refs.first() else {
            return ExecutionOutcome::Failed(format!(
                "{label} requires the target hunk resource_ref"
            ));
        };
        let Some(hunk) = decode_hunk_ref(&rref.id) else {
            return ExecutionOutcome::Failed(format!("{label}: malformed hunk resource-ref"));
        };
        // the `file` is a git operand (`git diff -- <file>`) → reject a leading-`-` fail-closed (LESSON
        // 45 — defense-in-depth even though `--` disambiguates; the standing external-mutator guard).
        if let Err(failed) = reject_dash_operands(&[hunk.file.as_str()]) {
            return failed;
        }
        // resolve `worktree_id → path` over read-only WAL (the get_diff resolution). Unknown → NotFound-class Failed.
        let Some(repo_path) = self.resolver.resolve(&hunk.worktree_id) else {
            return ExecutionOutcome::Failed(format!("{label}: unknown worktree (not found)"));
        };

        // read the LIVE diff (stage = workdir-vs-index `git diff`; unstage = staged index-vs-HEAD `git
        // diff --cached`). forbidden #6: the diff READ is the CLI here (the patch source of truth, Q1) —
        // git2 stays read-only elsewhere; this is a CLI read feeding the CLI mutation.
        let mut diff_args = vec!["diff".to_string()];
        if reverse {
            diff_args.push("--cached".to_string());
        }
        diff_args.push("--".to_string());
        diff_args.push(hunk.file.clone());
        let diff_text = match self.cli.run(&diff_args, &repo_path) {
            Ok(out) if out.success => out.stdout,
            // a non-zero git exit / spawn failure → STRUCTURAL reason ONLY (raw git stderr can carry paths, §15).
            Ok(_) => {
                return ExecutionOutcome::Failed(format!(
                    "{label}: git diff failed (non-zero exit)"
                ))
            }
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: git diff: {e}")),
        };

        // §17 GUARD #1 (the PRIMARY race guard): re-derive the hunk at the audited positions from the LIVE
        // diff. No match → the displayed hunk is gone / the file changed since the UI read it → fail-closed,
        // NO apply. ACCEPTED LIMITATION (the frozen position-only ref): same-position content-drift (the
        // workdir hunk at this position now has DIFFERENT content than the UI displayed) is unguardable —
        // the POSITION is the audited unit, so executed == audited at the contract's granularity, and it is
        // RECOVERABLE for stage/unstage (re-read get_diff → reverse). A content-hash would be a future
        // CONTRACT change (out of scope). NB: this posture does NOT carry to the DESTRUCTIVE discard slice.
        let Some(patch) = crate::git::find_hunk_patch(
            &diff_text,
            hunk.old_start,
            hunk.old_lines,
            hunk.new_start,
            hunk.new_lines,
        ) else {
            return ExecutionOutcome::Failed(format!(
                "{label}: the hunk no longer applies (re-approve)"
            ));
        };

        // write the one-hunk patch to a 0600 temp file (auto-cleaned via Drop). §15: the patch content IS
        // the working-tree content already on disk in the repo → a transient duplicate, not a new exposure.
        let patch_file = match TempPatch::write(req, &patch) {
            Ok(p) => p,
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: stage the patch: {e}")),
        };

        // apply to the INDEX: `git apply --cached [-R]`. §17 GUARD #2 (the secondary integrity gate): run
        // `--check` FIRST — a non-clean check (e.g. a concurrent index change between the diff-read and the
        // apply) → Failed, NO apply. Then the real apply.
        let mut base = vec!["apply".to_string(), "--cached".to_string()];
        if reverse {
            base.push("-R".to_string());
        }
        let mut check_args = base.clone();
        check_args.push("--check".to_string());
        check_args.push(patch_file.path_string());
        match self.cli.run(&check_args, &repo_path) {
            Ok(out) if out.success => {}
            Ok(_) => {
                return ExecutionOutcome::Failed(format!(
                    "{label}: the hunk no longer applies cleanly (re-approve)"
                ))
            }
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: git apply --check: {e}")),
        }
        let mut apply_args = base;
        apply_args.push(patch_file.path_string());
        match self.cli.run(&apply_args, &repo_path) {
            Ok(out) if out.success => {}
            Ok(_) => {
                return ExecutionOutcome::Failed(format!(
                    "{label}: git apply failed (non-zero exit)"
                ))
            }
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: git apply: {e}")),
        }

        // success — NO domain event (the worktree git-axis is a live-read cache, LESSON 47; the UI re-reads
        // get_diff to see the new staged/unstaged state). `side_effect_applied: true` (a real INDEX change →
        // a txn-B fault yields the honest ActionPartiallySucceeded, LESSON 21; recoverable: re-stage/unstage).
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: format!("{label} — applied the hunk to the index via the git CLI"),
            side_effect_applied: true,
            emitted_events: vec![],
        }
    }

    /// (W1-git-discard/096) The `git.discard_hunk` body — risk-3 **DESTRUCTIVE / IRREVERSIBLE**. Its OWN
    /// body (NOT a `reverse` flag on [`Self::execute_apply_hunk`] — discard targets the WORKING TREE not the
    /// index, adds the content-hash guard, and is irreversible; conflating it with the recoverable index path
    /// is a safety smell). Order: validate → decode the position-only ref → reject-dash → resolve → require
    /// the hash present → §17 GUARD #1 (position, re-derive the patch from the LIVE workdir diff) → §17 GUARD
    /// #2 (the LEAD-ruled (A) content-hash verify-before-destroy) → §17 GUARD #3 (`apply -R --check`) → the
    /// destructive `git apply -R` (NO `--cached`). NO domain event (LESSON 47 live-read cache). Contract-neutral
    /// (the hash rides `inputs`; the catalog + the resource-ref froze at 4.0b-ui1). The position-only accepted
    /// limitation that stage/unstage carry (LESSON 72) does NOT carry here — the content-hash closes the
    /// same-position content-drift race for the irreversible op.
    fn execute_discard_hunk(&self, req: &ActionRequest) -> ExecutionOutcome {
        let label = GIT_DISCARD_HUNK;
        // validate the catalog `requires_resource_refs` precondition FIRST (the create_worktree precedent).
        if let Err(e) = self.inner.validate(req) {
            return ExecutionOutcome::Failed(e.to_string());
        }
        // decode the frozen position-only hunk resource-ref (LESSON 63 — resolve the TARGET from the AUDITED
        // ref, never `inputs`; the hash is a verification TOKEN, not a target). Malformed → Failed BEFORE any git call.
        let Some(rref) = req.resource_refs.first() else {
            return ExecutionOutcome::Failed(format!(
                "{label} requires the target hunk resource_ref"
            ));
        };
        let Some(hunk_ref) = decode_hunk_ref(&rref.id) else {
            return ExecutionOutcome::Failed(format!("{label}: malformed hunk resource-ref"));
        };
        // the `file` is a git operand (`git diff -- <file>`) → reject a leading-`-` fail-closed (LESSON 45).
        if let Err(failed) = reject_dash_operands(&[hunk_ref.file.as_str()]) {
            return failed;
        }
        // resolve `worktree_id → path` over read-only WAL (the get_diff resolution). Unknown → NotFound-class.
        let Some(repo_path) = self.resolver.resolve(&hunk_ref.worktree_id) else {
            return ExecutionOutcome::Failed(format!("{label}: unknown worktree (not found)"));
        };

        // Early hash-presence gate (a pre-GUARD #1 fail-fast — NOT the GUARD #2 comparison, which is below
        // after the hunk is re-derived): a discard with NO `displayed_hunk_sha256` is REFUSED before any git
        // call → never falls through to a position-only destructive apply (the (A) ruling).
        let Some(displayed_hash) = string_input(req, "displayed_hunk_sha256") else {
            return ExecutionOutcome::Failed(format!(
                "{label}: discard requires the displayed-hunk hash"
            ));
        };

        // §17 GUARD #1 (position): read the LIVE WORKDIR diff (workdir-vs-index `git diff`, NO `--cached` — the
        // change the UI displays by default) + re-derive the byte-faithful one-hunk patch (095's proven path).
        // No match → the displayed hunk is gone / the file changed → fail-closed, NO mutation.
        let diff_args = vec!["diff".to_string(), "--".to_string(), hunk_ref.file.clone()];
        let diff_text = match self.cli.run(&diff_args, &repo_path) {
            Ok(out) if out.success => out.stdout,
            // a non-zero git exit / spawn failure → STRUCTURAL reason ONLY (raw git stderr can carry paths, §15).
            Ok(_) => {
                return ExecutionOutcome::Failed(format!(
                    "{label}: git diff failed (non-zero exit)"
                ))
            }
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: git diff: {e}")),
        };
        let Some(patch) = crate::git::find_hunk_patch(
            &diff_text,
            hunk_ref.old_start,
            hunk_ref.old_lines,
            hunk_ref.new_start,
            hunk_ref.new_lines,
        ) else {
            return ExecutionOutcome::Failed(format!(
                "{label}: the hunk no longer applies (re-examine)"
            ));
        };

        // §17 GUARD #2 (the content-hash — the LEAD-ruled (A) verify-before-destroy, the load-bearing
        // destructive guard): hash EXACTLY the bytes about to be reverse-applied (the one-hunk `patch` from
        // GUARD #1, parsed back to a structured Hunk) + require the FROZEN canonical sha256 to equal the
        // UI-supplied `displayed_hunk_sha256`. Hashing the APPLIED hunk (NOT a second `read_diff`) makes
        // verified-bytes == destroyed-bytes BY CONSTRUCTION (audited == executed, §17). On a PARTIALLY-STAGED
        // file the applied workdir-vs-index hunk diverges from the UI's HEAD→workdir `get_diff` hunk even when
        // the position quad coincides → the hashes differ → fail-closed (a staged-hunk discard is OUT of MVP —
        // the user unstages first). In the unstaged-only case the two bases coincide → the hash matches the
        // UI's. Mismatch → Failed, NO mutation: irreversible loss must never touch un-reviewed bytes (LESSON
        // 72's position-only accepted limitation does NOT carry to the destructive op). `parse_unified_diff`
        // strips only the 1-byte origin marker + retains content verbatim (same shape as `read_diff` — the
        // canonical form is diff-source-agnostic).
        let Some(applied_hunk) = crate::git::parse_unified_diff(&patch, None)
            .hunks
            .into_iter()
            .next()
        else {
            // defensive — `patch` is a valid one-hunk patch from find_hunk_patch (unreachable in practice).
            return ExecutionOutcome::Failed(format!(
                "{label}: could not re-derive the hunk to verify"
            ));
        };
        // case-insensitive (hex case carries no meaning) — defensive against a non-lowercase UI send.
        if !crate::git::canonical_hunk_sha256(&applied_hunk).eq_ignore_ascii_case(&displayed_hash) {
            return ExecutionOutcome::Failed(format!(
                "{label}: hunk changed since you reviewed it, re-examine"
            ));
        }

        // write the one-hunk patch to a 0600 temp file (auto-cleaned via Drop on EVERY return path).
        let patch_file = match TempPatch::write(req, &patch) {
            Ok(p) => p,
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: write the patch: {e}")),
        };

        // §17 GUARD #3 + the destructive apply: REVERSE-apply to the WORKING TREE (`git apply -R`, NO
        // `--cached`). `--check` FIRST (a concurrent workdir change between the read and the apply → Failed, NO
        // apply); then the real reverse-apply discards exactly that one hunk (other hunks / the index untouched).
        let base = vec!["apply".to_string(), "-R".to_string()];
        let mut check_args = base.clone();
        check_args.push("--check".to_string());
        check_args.push(patch_file.path_string());
        match self.cli.run(&check_args, &repo_path) {
            Ok(out) if out.success => {}
            Ok(_) => {
                return ExecutionOutcome::Failed(format!(
                    "{label}: the hunk no longer applies cleanly (re-examine)"
                ))
            }
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: git apply --check: {e}")),
        }
        let mut apply_args = base;
        apply_args.push(patch_file.path_string());
        match self.cli.run(&apply_args, &repo_path) {
            Ok(out) if out.success => {}
            Ok(_) => {
                return ExecutionOutcome::Failed(format!(
                    "{label}: git apply failed (non-zero exit)"
                ))
            }
            Err(e) => return ExecutionOutcome::Failed(format!("{label}: git apply: {e}")),
        }

        // success — NO domain event (the worktree git-axis is a live-read cache, LESSON 47; the UI re-reads
        // get_diff). `side_effect_applied: true` (a real WORKING-TREE change → a txn-B fault yields the honest
        // ActionPartiallySucceeded, LESSON 21; discard is IRREVERSIBLE — there is no clean rollback).
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: format!("{label} — discarded the hunk from the working tree via the git CLI"),
            side_effect_applied: true,
            emitted_events: vec![],
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
            // W1-git-stage/095 — the per-hunk INDEX mutations (stage/unstage differ only by `-R`).
            GIT_STAGE_HUNK => self.execute_apply_hunk(req, false),
            GIT_UNSTAGE_HUNK => self.execute_apply_hunk(req, true),
            // W1-git-discard/096 — the per-hunk DESTRUCTIVE WORKING-TREE mutation (its OWN body, NOT a
            // `reverse` flag on execute_apply_hunk — working-tree not index, adds the hash guard, IRREVERSIBLE).
            GIT_DISCARD_HUNK => self.execute_discard_hunk(req),
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

/// (W1-git-stage/095) The decoded frozen position-only hunk resource-ref (4.0b-ui1): the worktree id, the
/// target file, and the `@@` position quad the hunk is re-derived against.
struct HunkRef {
    worktree_id: String,
    file: String,
    old_start: u32,
    old_lines: u32,
    new_start: u32,
    new_lines: u32,
}

/// Decode the frozen `{worktree_id}\x1f{file}\x1f{old_start},{old_lines},{new_start},{new_lines}` id
/// (4.0b-ui1). `None` on ANY malformation (wrong field count, empty id/file, non-numeric positions) —
/// fail-closed BEFORE any git call.
fn decode_hunk_ref(id: &str) -> Option<HunkRef> {
    let parts: Vec<&str> = id.split(HUNK_REF_SEPARATOR).collect();
    if parts.len() != 3 {
        return None;
    }
    let (worktree_id, file, positions) = (parts[0], parts[1], parts[2]);
    if worktree_id.is_empty() || file.is_empty() {
        return None;
    }
    let nums: Vec<&str> = positions.split(',').collect();
    if nums.len() != 4 {
        return None;
    }
    Some(HunkRef {
        worktree_id: worktree_id.to_string(),
        file: file.to_string(),
        old_start: nums[0].parse().ok()?,
        old_lines: nums[1].parse().ok()?,
        new_start: nums[2].parse().ok()?,
        new_lines: nums[3].parse().ok()?,
    })
}

/// (W1-git-stage/095) A 0600 temp patch file the per-hunk mutator feeds to `git apply`, auto-removed on
/// Drop (every return path — including the early fail-closed ones). §15: the patch content is the
/// working-tree content already on disk in the repo (a transient duplicate, not a new secret exposure);
/// 0600 keeps it owner-only regardless.
struct TempPatch(PathBuf);

impl TempPatch {
    fn write(req: &ActionRequest, patch: &str) -> std::io::Result<Self> {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        // unique per-action filename (the audited id is alnum/underscore — a safe path component).
        let path = std::env::temp_dir().join(format!(
            "nexusops-hunk-{}.patch",
            req.action_request_id.as_str()
        ));
        // `create_new` (O_CREAT|O_EXCL) — the 0o600 mode is then UNCONDITIONAL (it applies on the
        // guaranteed-fresh create; 0o600 carries no group/other bits → owner-only on every umask) AND
        // it refuses to follow a pre-existing file/symlink at that path (fails loud on the ~impossible
        // 80-bit-ULID collision rather than silently writing through stale perms — CWE-377 hardening).
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        f.write_all(patch.as_bytes())?;
        f.flush()?;
        Ok(Self(path))
    }

    fn path_string(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for TempPatch {
    fn drop(&mut self) {
        // best-effort cleanup (the patch is a transient; a leftover on a crash is harmless 0600 data).
        let _ = std::fs::remove_file(&self.0);
    }
}

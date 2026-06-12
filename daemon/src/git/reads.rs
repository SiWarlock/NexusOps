//! git2 READ-ONLY worktree-status backend (forbidden #6).
//!
//! `read_worktree_status` resolves the §5.1 git-sync axis (`WorktreeGit`), the ahead/behind
//! divergence, the branch, and the HEAD sha for a worktree path — via git2 read APIs only, never a
//! mutating API (worktree/branch/commit/merge go through the git CLI as Gateway actions). The
//! within-git-axis resolution honors §5.1-R7 (`conflicts > dirty > untracked > behind > ahead >
//! clean` — dirty masks divergence). A non-git / inaccessible path yields `None` (typed absence — a
//! worktree has no git state). The `proj_worktree` projector (the gated 5.2-remainder) consumes
//! these plus `precedence::derive_worktree_status`.

use std::path::Path;

use git2::{Oid, Repository, StatusOptions};
use nexusops_shared::status::WorktreeGit;

/// The daemon-internal git-truth read for a worktree (feeds `proj_worktree`: `dirty_state` = the
/// `git_axis` wire value, plus `branch_name`/`last_commit_sha`/`ahead_count`/`behind_count`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeGitState {
    /// The within-axis-resolved git-sync status (§5.1-R7).
    pub git_axis: WorktreeGit,
    /// Current branch shorthand; `None` when detached or unborn.
    pub branch: Option<String>,
    /// HEAD commit sha (40-hex); `None` for an unborn HEAD.
    pub last_commit_sha: Option<String>,
    /// Commits ahead of `base`; `None` when no base given or it can't be resolved.
    pub ahead_count: Option<usize>,
    /// Commits behind `base`; `None` when no base given or it can't be resolved.
    pub behind_count: Option<usize>,
}

/// Read the worktree's git state at `path` (READ-ONLY). `base` (e.g. `proj_worktree.base_branch`),
/// when given and resolvable, drives ahead/behind. A non-git / inaccessible path → `None`.
pub fn read_worktree_status(path: &Path, base: Option<&str>) -> Option<WorktreeGitState> {
    let repo = Repository::discover(path).ok()?;

    let detached = repo.head_detached().unwrap_or(false);
    let head_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let branch = if detached {
        None
    } else {
        // git2 0.21: `Reference::shorthand` returns `Result<&str, Error>` — `.ok()` folds the
        // non-UTF-8 / unborn case into the absent branch.
        repo.head()
            .ok()
            .and_then(|h| h.shorthand().ok().map(str::to_owned))
    };
    let last_commit_sha = head_commit.as_ref().map(|c| c.id().to_string());

    let (ahead_count, behind_count) = match (base, head_commit.as_ref()) {
        (Some(base), Some(head)) => divergence(&repo, head.id(), base),
        _ => (None, None),
    };

    let git_axis = resolve_git_axis(&repo, behind_count, ahead_count);

    Some(WorktreeGitState {
        git_axis,
        branch,
        last_commit_sha,
        ahead_count,
        behind_count,
    })
}

/// Linked worktree names (git2 `worktrees()`); empty for a non-git path or none linked.
pub fn list_linked_worktrees(path: &Path) -> Vec<String> {
    let Ok(repo) = Repository::discover(path) else {
        return Vec::new();
    };
    match repo.worktrees() {
        // git2 0.21 `StringArray::iter` yields `Result<Option<&str>, Error>`; keep the UTF-8 names,
        // drop read errors / non-UTF-8 entries.
        Ok(names) => names
            .iter()
            .filter_map(|name| name.ok().flatten().map(str::to_owned))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Ahead/behind `local` vs `base` (resolved as a revspec, e.g. a branch name). `(None, None)` if the
/// base won't resolve or the graph walk fails.
fn divergence(repo: &Repository, local: Oid, base: &str) -> (Option<usize>, Option<usize>) {
    let base_oid = repo
        .revparse_single(base)
        .ok()
        .and_then(|obj| obj.peel_to_commit().ok())
        .map(|commit| commit.id());
    match base_oid {
        Some(base_oid) => match repo.graph_ahead_behind(local, base_oid) {
            Ok((ahead, behind)) => (Some(ahead), Some(behind)),
            Err(_) => (None, None),
        },
        None => (None, None),
    }
}

/// Within-git-axis resolution (§5.1-R7): `conflicts > dirty > untracked > behind > ahead > clean`
/// (dirty masks divergence). `behind`/`ahead` surface only on an otherwise-clean tree. Read-only:
/// `index().has_conflicts()` + default `StatusOptions` (no `GIT_STATUS_OPT_UPDATE_INDEX`).
fn resolve_git_axis(
    repo: &Repository,
    behind_count: Option<usize>,
    ahead_count: Option<usize>,
) -> WorktreeGit {
    if repo
        .index()
        .map(|index| index.has_conflicts())
        .unwrap_or(false)
    {
        return WorktreeGit::Conflicts;
    }

    let (mut tracked_changes, mut untracked) = (false, false);
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).include_ignored(false);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        for entry in statuses.iter() {
            let status = entry.status();
            if status.is_wt_new() {
                untracked = true;
            } else {
                // `include_ignored(false)` ⇒ no ignored entry is yielded, so every non-WT_NEW entry
                // here is a real tracked change (staged or worktree-side).
                tracked_changes = true;
            }
        }
    }

    if tracked_changes {
        WorktreeGit::Dirty
    } else if untracked {
        WorktreeGit::UntrackedFiles
    } else if behind_count.unwrap_or(0) > 0 {
        WorktreeGit::BehindBase
    } else if ahead_count.unwrap_or(0) > 0 {
        WorktreeGit::AheadOfBase
    } else {
        WorktreeGit::Clean
    }
}

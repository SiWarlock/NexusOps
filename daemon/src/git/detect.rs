//! Git repo introspection — READ-ONLY (forbidden #6).
//!
//! `detect_git` is the pure, deterministic git-state read the `project.rescan` executor will call
//! (wired in edges-002). It NEVER mutates the repo — git2 read APIs only; worktree/branch/commit/
//! merge mutations go through the git CLI as Gateway actions. A non-git or missing path → a valid
//! degraded `GitDetection { is_git: false, .. }`, never an `Err`/panic ("basic project w/ no git
//! still works"). Worktree-list, diff, and log reads belong to the 5.2 dual-git read backend.

use std::path::{Path, PathBuf};

use git2::{Repository, StatusOptions};

/// The minimal load-bearing git state for project detection (§7.2 git-state SoT).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitDetection {
    /// Whether `path` resolves to a git repository.
    pub is_git: bool,
    /// The repo's working-directory root (canonicalized). `None` for a **bare** repo even when
    /// `is_git` is true — callers must NOT assume `is_git == repo_root.is_some()` (the edges-002
    /// `project.rescan` wiring guards the bare-repo / no-workdir case).
    pub repo_root: Option<PathBuf>,
    /// The `origin` remote URL, when set (feeds P7.1 GitHub linking).
    pub remote_url: Option<String>,
    /// The current branch shorthand (e.g. `main`); `None` when detached or unborn.
    pub branch: Option<String>,
    /// Whether HEAD is detached.
    pub detached: bool,
    /// Whether the working tree has uncommitted changes (including untracked files).
    pub is_dirty: bool,
}

/// Read-only git introspection at `path`. Resolves the enclosing repo via `Repository::discover`
/// (a project root OR a subdir of one both resolve to that repo — matches git's own UX). A non-git
/// or missing path → a valid degraded result (`GitDetection::default()`), never an `Err`/panic.
pub fn detect_git(path: &Path) -> GitDetection {
    let repo = match Repository::discover(path) {
        Ok(repo) => repo,
        Err(_) => return GitDetection::default(),
    };

    let repo_root = repo.workdir().and_then(|w| w.canonicalize().ok());
    // git2 0.21: `Remote::url` / `Reference::shorthand` return `Result<&str, Error>` (Err on
    // non-UTF-8) — `.ok()` folds that into the absent case alongside a missing remote/branch.
    let remote_url = repo
        .find_remote("origin")
        .ok()
        .and_then(|remote| remote.url().ok().map(str::to_owned));
    // `head_detached` is `Result<bool, Error>`; an unreadable HEAD (e.g. a damaged `.git/HEAD`)
    // degrades to attached-with-no-branch — self-consistent here: `detached=false` AND `branch=None`
    // below, because the same error/unborn HEAD makes `repo.head().ok()` yield `None` too.
    let detached = repo.head_detached().unwrap_or(false);
    let branch = if detached {
        None
    } else {
        repo.head()
            .ok()
            .and_then(|head| head.shorthand().ok().map(str::to_owned))
    };
    let is_dirty = worktree_dirty(&repo);

    GitDetection {
        is_git: true,
        repo_root,
        remote_url,
        branch,
        detached,
        is_dirty,
    }
}

/// True when the working tree has any uncommitted change (including untracked files). Read-only:
/// the default `StatusOptions` do NOT set `GIT_STATUS_OPT_UPDATE_INDEX`, so status never writes.
fn worktree_dirty(repo: &Repository) -> bool {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).include_ignored(false);
    match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => !statuses.is_empty(),
        Err(_) => false,
    }
}

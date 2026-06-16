//! git2 READ-ONLY worktree-status backend (forbidden #6).
//!
//! `read_worktree_status` resolves the §5.1 git-sync axis (`WorktreeGit`), the ahead/behind
//! divergence, the branch, and the HEAD sha for a worktree path — via git2 read APIs only, never a
//! mutating API (worktree/branch/commit/merge go through the git CLI as Gateway actions). The
//! within-git-axis resolution honors §5.1-R7 (`conflicts > dirty > untracked > behind > ahead >
//! clean` — dirty masks divergence). A non-git / inaccessible path yields `None` (typed absence — a
//! worktree has no git state). The `proj_worktree` projector (the gated 5.2-remainder) consumes
//! these plus `precedence::derive_worktree_status`.
//!
//! `read_diff` (file-level `FileChange`, working-tree-vs-ref or ref-vs-ref) and `read_log` (a bounded
//! newest-first `CommitInfo` list) complete the §9 `status/diff/log/branch` read set; both are git2
//! read-only and degrade to an empty list on a non-git path.

use std::path::Path;

use git2::{Delta, DiffFindOptions, DiffOptions, Oid, Patch, Repository, Sort, StatusOptions};
use nexusops_shared::status::WorktreeGit;
use nexusops_shared::time::Timestamp;

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

// ---- diff + log (§9 read backend) ------------------------------------------

/// A file-level change in a diff. File-level granularity; the per-hunk detail (ranges + per-line
/// content) is `read_file_hunks` / `DiffHunk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// The changed file's path, relative to the repo root (the rename DESTINATION for a `Renamed`).
    pub path: String,
    /// The kind of change.
    pub change_kind: ChangeKind,
    /// The rename SOURCE path (the `old_file` path) for a `Renamed` change; `None` for
    /// Added/Modified/Deleted/Other.
    pub old_path: Option<String>,
    /// Lines added in this file.
    pub additions: usize,
    /// Lines deleted in this file.
    pub deletions: usize,
}

/// The kind of file change. Rename detection is ON (git2 `find_similar`) → a rename reads as ONE
/// `Renamed` change carrying its `old_path`, not a Delete + Add pair. Copy detection is OFF (git2 0.21
/// does not detect copies of unmodified files in either diff mode — a named follow-up; no `Copied`
/// variant ships unproduced). Any non-core git2 delta collapses to `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Other,
}

impl ChangeKind {
    fn from_delta(status: Delta) -> Self {
        match status {
            // an untracked working-tree file is an addition relative to the base tree
            Delta::Added | Delta::Untracked => ChangeKind::Added,
            Delta::Modified | Delta::Typechange => ChangeKind::Modified,
            Delta::Deleted => ChangeKind::Deleted,
            Delta::Renamed => ChangeKind::Renamed,
            _ => ChangeKind::Other,
        }
    }
}

/// One commit in a `read_log` walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// The commit's 40-hex sha.
    pub sha: String,
    /// The commit message's first line.
    pub summary: String,
    /// The author as `name <email>`.
    pub author: String,
    /// The commit time as an RFC3339-Z `Timestamp`; `None` only for an out-of-range epoch.
    pub timestamp: Option<Timestamp>,
}

/// Build the rename-aware `git2::Diff` for `(from, to)` over an already-discovered `repo` (READ-ONLY).
/// `(from, to)` selects the target exactly as the public reads: `(None, None)` = working-tree-vs-HEAD;
/// `(Some(base), Some(head))` = ref-vs-ref; `(Some(base), None)` = base-vs-working-tree. Returns `None`
/// on any early-out — an unresolvable ref, an unborn HEAD with no base, or a diff-construction failure
/// — which the callers map to an empty result.
///
/// The returned `Diff` borrows `repo`: git2 ties the `Diff` to the repository's ODB and copies the
/// `DiffOptions` into it (it does NOT retain the locally-resolved trees), so `opts`/the resolved trees
/// are free to drop at return while the `Diff<'r>` stays valid. The caller therefore keeps its own
/// `Repository::discover`, and `repo` must outlive the returned `Diff` — the `'r` lifetime, matching
/// the `resolve_tree<'r>` idiom below.
fn open_diff<'r>(
    repo: &'r Repository,
    from: Option<&str>,
    to: Option<&str>,
) -> Option<git2::Diff<'r>> {
    let mut opts = DiffOptions::new();
    // `show_untracked_content` so a new working-tree file gets real line counts (without it,
    // `Patch::from_diff` is `None` for an untracked delta → counts silently read 0).
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);

    let from_tree = resolve_tree(repo, from);
    let diff = match to {
        // ref-vs-ref
        Some(to_ref) => match (from_tree, resolve_tree(repo, Some(to_ref))) {
            (Some(f), Some(t)) => repo.diff_tree_to_tree(Some(&f), Some(&t), Some(&mut opts)),
            _ => return None,
        },
        // base (or HEAD) -vs- working tree — `_with_index` = `git diff <tree>` semantics (accounts
        // for the index, so staged-only changes show too), not bare tree-vs-workdir.
        None => match from_tree {
            Some(f) => repo.diff_tree_to_workdir_with_index(Some(&f), Some(&mut opts)),
            None => return None, // unborn HEAD + no base → nothing to diff
        },
    };
    let mut diff = diff.ok()?;
    // Rename detection (forbidden #6: `find_similar` mutates only the in-memory `Diff`, never the
    // repo — the `diff_log_do_not_mutate`/`rename_detection_read_only`/`read_file_hunks_read_only`
    // guards hold). `for_untracked` is REQUIRED to pair a rename whose NEW side is an untracked
    // working-tree file (the common `(None,None)` case) — find_similar needs its OWN flag, distinct
    // from the `DiffOptions` untracked flags above; it's a no-op when there is no untracked side (the
    // ref-vs-ref / base-vs-tree modes). git's DEFAULT similarity threshold (≈50%, matching the user's
    // terminal `git diff -M`) — a low-similarity pair stays Add + Delete. Copy detection stays OFF
    // (git2 0.21 detects none). A `find_similar` failure is best-effort: the diff keeps its
    // un-collapsed Add + Delete deltas.
    let mut find_opts = DiffFindOptions::new();
    find_opts.renames(true).for_untracked(true);
    let _ = diff.find_similar(Some(&mut find_opts));
    Some(diff)
}

/// Read a file-level diff at `path` (READ-ONLY). `(from, to)` selects the target: `(None, None)` =
/// working-tree-vs-HEAD; `(Some(base), Some(head))` = ref-vs-ref; `(Some(base), None)` = base-vs-
/// working-tree. A non-git path, a clean tree, or an unresolvable ref → an empty list (the §7.2
/// consumer establishes git-ness first via `detect_git`/`read_worktree_status`).
pub fn read_diff(path: &Path, from: Option<&str>, to: Option<&str>) -> Vec<FileChange> {
    let Ok(repo) = Repository::discover(path) else {
        return Vec::new();
    };
    let Some(diff) = open_diff(&repo, from, to) else {
        return Vec::new();
    };

    diff.deltas()
        .enumerate()
        .map(|(i, delta)| {
            let (additions, deletions) = line_counts(&diff, i);
            let change_kind = ChangeKind::from_delta(delta.status());
            // `old_path` = the rename SOURCE (`old_file` path), only for a `Renamed` delta.
            let old_path = (change_kind == ChangeKind::Renamed)
                .then(|| {
                    delta
                        .old_file()
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                })
                .flatten();
            FileChange {
                path: delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                change_kind,
                old_path,
                additions,
                deletions,
            }
        })
        .collect()
}

/// Per-file `(additions, deletions)` for delta `idx`; `(0, 0)` for a binary file or a stats failure.
fn line_counts(diff: &git2::Diff, idx: usize) -> (usize, usize) {
    match Patch::from_diff(diff, idx) {
        Ok(Some(patch)) => match patch.line_stats() {
            Ok((_, additions, deletions)) => (additions, deletions),
            Err(_) => (0, 0),
        },
        _ => (0, 0),
    }
}

// ---- per-hunk diff (the §7.2 O-6 PR Review Workspace data) ------------------

/// A single hunk of a file's diff — geometry + header + per-line content. The daemon-side data for the
/// O-6 §7.2 PR Review Workspace per-hunk actions + diff render. git2 read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// First old-file line the hunk covers.
    pub old_start: u32,
    /// Number of old-file lines in the hunk.
    pub old_lines: u32,
    /// First new-file line the hunk covers.
    pub new_start: u32,
    /// Number of new-file lines in the hunk.
    pub new_lines: u32,
    /// The hunk header (e.g. `@@ -1,5 +1,5 @@`), UTF-8-lossy.
    pub header: String,
    /// The hunk's lines, in order.
    pub lines: Vec<DiffLine>,
}

/// One line within a hunk. `old_lineno`/`new_lineno`: an Addition has `new_lineno` Some + `old_lineno`
/// None; a Deletion the reverse; a Context line both Some.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Addition (`+`) / Deletion (`-`) / Context (` ` + the EOFNL markers).
    pub kind: DiffLineKind,
    /// The raw line text (UTF-8-lossy), kept incl. the trailing newline — the UI renders it.
    pub content: String,
    /// The old-file line number; `None` for an Addition.
    pub old_lineno: Option<u32>,
    /// The new-file line number; `None` for a Deletion.
    pub new_lineno: Option<u32>,
}

/// The kind of diff line. git2's EOFNL / context origins map to `Context` (a display line, not a
/// `+`/`-` change).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Addition,
    Deletion,
    Context,
}

/// Read the per-hunk diff for a single `file` (READ-ONLY). `(from, to)` selects the target exactly as
/// `read_diff`. **Rename-aware** (`find_similar`, like `read_diff`/edges-011) so a renamed-with-edit
/// file matches on its NEW path and its hunks reflect the EDIT (the two sibling reads of the same diff
/// agree). A non-git path, a clean tree, an unresolvable ref, a file with no change, or a binary file
/// (`Patch::from_diff` `None` OR a patch with 0 hunks) → an empty list (degrade-not-panic).
pub fn read_file_hunks(
    path: &Path,
    file: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Vec<DiffHunk> {
    let Ok(repo) = Repository::discover(path) else {
        return Vec::new();
    };
    let Some(diff) = open_diff(&repo, from, to) else {
        return Vec::new();
    };

    // Find the delta for `file` by its NEW path (= the FileChange.path read_diff reports — for a
    // Renamed delta that is the new name, so the OLD name does NOT resolve, matching read_diff's
    // single-path-per-rename). A pure Deleted delta may carry the path only on its old side → match
    // old_file for Deleted ONLY (never for a rename's source).
    let target = diff.deltas().position(|d| {
        d.new_file()
            .path()
            .is_some_and(|p| p.to_string_lossy() == file)
            || (d.status() == Delta::Deleted
                && d.old_file()
                    .path()
                    .is_some_and(|p| p.to_string_lossy() == file))
    });
    let Some(idx) = target else {
        return Vec::new();
    };
    // Binary file → `Patch::from_diff` `None` OR a patch with 0 hunks; both → an empty list. Bind the
    // result so the `patch` temporary (which borrows `diff`) drops before the block's locals.
    let hunks = match Patch::from_diff(&diff, idx) {
        Ok(Some(patch)) => extract_hunks(&patch),
        _ => Vec::new(),
    };
    hunks
}

/// Iterate a per-file `Patch`'s hunks + lines into `DiffHunk`s (READ-ONLY). A hunk/line read error
/// degrades that hunk/line (never panics).
fn extract_hunks(patch: &Patch) -> Vec<DiffHunk> {
    (0..patch.num_hunks())
        .filter_map(|h| {
            let (hunk, _) = patch.hunk(h).ok()?;
            let num_lines = patch.num_lines_in_hunk(h).unwrap_or(0);
            let lines = (0..num_lines)
                .filter_map(|l| patch.line_in_hunk(h, l).ok().map(diff_line))
                .collect();
            Some(DiffHunk {
                old_start: hunk.old_start(),
                old_lines: hunk.old_lines(),
                new_start: hunk.new_start(),
                new_lines: hunk.new_lines(),
                header: String::from_utf8_lossy(hunk.header()).into_owned(),
                lines,
            })
        })
        .collect()
}

/// Map a git2 `DiffLine` → the daemon `DiffLine`. Origin `+`→Addition, `-`→Deletion, everything else
/// (context + the EOFNL markers) → Context. Content UTF-8-lossy, kept raw.
fn diff_line(line: git2::DiffLine<'_>) -> DiffLine {
    let kind = match line.origin_value() {
        git2::DiffLineType::Addition => DiffLineKind::Addition,
        git2::DiffLineType::Deletion => DiffLineKind::Deletion,
        _ => DiffLineKind::Context,
    };
    DiffLine {
        kind,
        content: String::from_utf8_lossy(line.content()).into_owned(),
        old_lineno: line.old_lineno(),
        new_lineno: line.new_lineno(),
    }
}

/// Read a bounded, newest-first commit log from `start` (a ref; `None` = HEAD), capped at `limit`.
/// READ-ONLY. A non-git path, an unborn HEAD, or an unresolvable ref → an empty list. `limit` bounds
/// the lazy revwalk — no full-history walk.
pub fn read_log(path: &Path, start: Option<&str>, limit: usize) -> Vec<CommitInfo> {
    let Ok(repo) = Repository::discover(path) else {
        return Vec::new();
    };
    let start_oid = match start {
        Some(r) => repo
            .revparse_single(r)
            .ok()
            .and_then(|obj| obj.peel_to_commit().ok())
            .map(|commit| commit.id()),
        None => repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .map(|commit| commit.id()),
    };
    let Some(start_oid) = start_oid else {
        return Vec::new(); // unborn HEAD / unresolvable ref
    };
    let Ok(mut walk) = repo.revwalk() else {
        return Vec::new();
    };
    if walk.set_sorting(Sort::TIME).is_err() || walk.push(start_oid).is_err() {
        return Vec::new();
    }
    walk.filter_map(Result::ok)
        .take(limit)
        .filter_map(|oid| repo.find_commit(oid).ok())
        .map(|commit| {
            let author = commit.author();
            CommitInfo {
                sha: commit.id().to_string(),
                // git2 0.21: `Commit::summary` is `Result<Option<&str>, Error>` (Err on non-UTF-8).
                summary: commit
                    .summary()
                    .ok()
                    .flatten()
                    .map(str::to_owned)
                    .unwrap_or_default(),
                author: format!(
                    "{} <{}>",
                    author.name().unwrap_or_default(),
                    author.email().unwrap_or_default()
                ),
                timestamp: commit_timestamp(&commit),
            }
        })
        .collect()
}

/// The commit time as an RFC3339-Z `Timestamp`; `None` only for an out-of-range epoch
/// (`from_unix_timestamp` is theoretically fallible — keeps the read no-unwrap).
fn commit_timestamp(commit: &git2::Commit) -> Option<Timestamp> {
    let dt = time::OffsetDateTime::from_unix_timestamp(commit.time().seconds()).ok()?;
    let rfc3339 = dt
        .format(&time::format_description::well_known::Rfc3339)
        .ok()?;
    Timestamp::parse(&rfc3339).ok()
}

/// Resolve `refspec` (or HEAD when `None`) to its tree; `None` if it won't resolve (e.g. unborn HEAD).
fn resolve_tree<'r>(repo: &'r Repository, refspec: Option<&str>) -> Option<git2::Tree<'r>> {
    match refspec {
        Some(r) => repo.revparse_single(r).ok()?.peel_to_tree().ok(),
        None => repo.head().ok()?.peel_to_tree().ok(),
    }
}

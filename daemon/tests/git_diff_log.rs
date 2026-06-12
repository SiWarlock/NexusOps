//! P5.2 — git2 READ-ONLY diff + log reads (`git/reads.rs` extension), completing the named
//! `status/diff/log/branch` read backend (status/branch/worktree-list landed in edges-002).
//!
//! `read_diff(path, from, to)` → file-level `FileChange` list: `(None,None)` = working-tree-vs-HEAD,
//! `(Some(base),Some(head))` = ref-vs-ref, `(Some(base),None)` = base-vs-working-tree. `read_log(path,
//! start, limit)` → a bounded newest-first `CommitInfo` list. Both git2 read-only (forbidden #6 —
//! pinned by `diff_log_do_not_mutate`) and degraded-not-panic on a non-git path. Hermetic `git2::init`
//! fixtures, per edges-002.

use std::path::Path;

use git2::{Oid, Repository, RepositoryInitOptions, Signature};
use tempfile::tempdir;

use nexusopsd::git::reads::{read_diff, read_log, ChangeKind};

// ---- hermetic git fixtures -------------------------------------------------

fn init_repo(path: &Path) -> Repository {
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    Repository::init_opts(path, &opts).expect("init repo")
}

/// Commit a file into `repo`'s workdir; chains onto the current HEAD. Returns the new commit oid.
fn commit_file(repo: &Repository, name: &str, content: &str) -> Oid {
    let workdir = repo.workdir().expect("non-bare repo");
    std::fs::write(workdir.join(name), content).expect("write file");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(name)).expect("add path");
    index.write().expect("write index");
    let tree_oid = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");
    let sig = Signature::now("Test", "test@example.com").expect("sig");
    let parents: Vec<git2::Commit> = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok())
        .into_iter()
        .collect();
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parent_refs)
        .expect("commit")
}

// ---- diff ------------------------------------------------------------------

#[test]
fn diff_modified_file() {
    // spec(§9): a modified tracked file → FileChange{Modified} with line counts.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "line1\nline2\n");
    std::fs::write(dir.path().join("a.txt"), "line1\nCHANGED\n").unwrap();
    let changes = read_diff(dir.path(), None, None);
    let a = changes
        .iter()
        .find(|c| c.path == "a.txt")
        .expect("a.txt change");
    assert_eq!(a.change_kind, ChangeKind::Modified);
    assert_eq!(a.additions, 1);
    assert_eq!(a.deletions, 1);
}

#[test]
fn diff_added_file() {
    // spec(§9): a new working-tree file → FileChange{Added}.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "x\n");
    std::fs::write(dir.path().join("new.txt"), "hello\n").unwrap();
    let changes = read_diff(dir.path(), None, None);
    let n = changes
        .iter()
        .find(|c| c.path == "new.txt")
        .expect("new.txt change");
    assert_eq!(n.change_kind, ChangeKind::Added);
    assert_eq!(n.additions, 1); // an untracked file's lines count (needs show_untracked_content)
}

#[test]
fn diff_base_vs_worktree() {
    // spec(§7.2): (Some(base), None) → base-tree-vs-working-tree — sees BOTH a committed-after-base
    // file and an uncommitted working-tree add as changes vs the named base ref.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let c1 = commit_file(&repo, "a.txt", "x\n");
    repo.branch("base", &repo.find_commit(c1).unwrap(), false)
        .unwrap();
    commit_file(&repo, "b.txt", "y\n"); // committed after base
    std::fs::write(dir.path().join("c.txt"), "z\n").unwrap(); // uncommitted working-tree add
    let changes = read_diff(dir.path(), Some("base"), None);
    assert!(changes
        .iter()
        .any(|c| c.path == "b.txt" && c.change_kind == ChangeKind::Added));
    assert!(changes
        .iter()
        .any(|c| c.path == "c.txt" && c.change_kind == ChangeKind::Added));
}

#[test]
fn diff_deleted_file() {
    // spec(§9): a deleted tracked file → FileChange{Deleted}.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "x\n");
    std::fs::remove_file(dir.path().join("a.txt")).unwrap();
    let changes = read_diff(dir.path(), None, None);
    let a = changes
        .iter()
        .find(|c| c.path == "a.txt")
        .expect("a.txt change");
    assert_eq!(a.change_kind, ChangeKind::Deleted);
}

#[test]
fn diff_clean_empty() {
    // spec(§9): a clean tree → an empty diff (not an error).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "x\n");
    assert!(read_diff(dir.path(), None, None).is_empty());
}

#[test]
fn diff_ref_vs_ref() {
    // spec(§7.2): diff two refs → the changed files between them (PR/branch-diff use).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let c1 = commit_file(&repo, "a.txt", "x\n");
    repo.branch("base", &repo.find_commit(c1).unwrap(), false)
        .unwrap();
    commit_file(&repo, "b.txt", "y\n"); // main now ahead of base with b.txt added
    let changes = read_diff(dir.path(), Some("base"), Some("main"));
    assert!(changes
        .iter()
        .any(|c| c.path == "b.txt" && c.change_kind == ChangeKind::Added));
}

// ---- log -------------------------------------------------------------------

#[test]
fn log_recent_commits() {
    // spec(§9): a commit log → CommitInfo list (sha/summary/author/timestamp), newest-first.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "1\n");
    commit_file(&repo, "b.txt", "2\n");
    let last = commit_file(&repo, "c.txt", "3\n");
    let log = read_log(dir.path(), None, 50);
    assert_eq!(log.len(), 3);
    assert_eq!(log[0].sha, last.to_string()); // newest-first
    assert!(!log[0].summary.is_empty());
    assert!(log[0].author.contains("Test"));
    assert!(log[0].timestamp.is_some());
}

#[test]
fn log_limit_caps() {
    // spec(§9): `limit` caps the walk (no unbounded revwalk).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    for i in 0..5 {
        commit_file(&repo, &format!("f{i}.txt"), "x\n");
    }
    assert_eq!(read_log(dir.path(), None, 2).len(), 2);
}

#[test]
fn log_empty_repo() {
    // spec(§9): an empty repo (unborn HEAD) → an empty log, never a panic.
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    assert!(read_log(dir.path(), None, 50).is_empty());
}

#[test]
fn log_unresolvable_ref() {
    // spec(§9): an unresolvable start ref → an empty log, never a panic.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "x\n");
    assert!(read_log(dir.path(), Some("no-such-ref"), 50).is_empty());
}

// ---- general ---------------------------------------------------------------

#[test]
fn diff_log_do_not_mutate() {
    // spec(§9): forbidden #6 — diff/log reads do not mutate the repo (HEAD oid unchanged).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let oid = commit_file(&repo, "a.txt", "x\n");
    std::fs::write(dir.path().join("a.txt"), "y\n").unwrap();
    let before = repo.head().unwrap().target().unwrap();
    let _ = read_diff(dir.path(), None, None);
    let _ = read_log(dir.path(), None, 50);
    let after = repo.head().unwrap().target().unwrap();
    assert_eq!(before, after);
    assert_eq!(after, oid);
}

#[test]
fn diff_log_non_git_degraded() {
    // spec(§9): a non-git / missing path → degraded (empty), never a panic.
    let dir = tempdir().unwrap();
    assert!(read_diff(dir.path(), None, None).is_empty());
    assert!(read_log(dir.path(), None, 50).is_empty());
    let missing = dir.path().join("nope");
    assert!(read_diff(&missing, None, None).is_empty());
    assert!(read_log(&missing, None, 50).is_empty());
}

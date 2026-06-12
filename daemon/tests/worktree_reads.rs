//! P5.2 — git2 read-only worktree-status backend (`git/reads.rs`) + the §5.1-R7 two-axis
//! worktree-status precedence fn (`git/precedence.rs`).
//!
//! `read_worktree_status` resolves the git-sync axis (`WorktreeGit`), ahead/behind counts, branch,
//! and HEAD sha via git2 READ-only APIs (forbidden #6 — pinned by `worktree_read_does_not_mutate`).
//! `derive_worktree_status` collapses the git axis and the lifecycle-overlay axis (`WorktreeOverlay`)
//! into the single most-salient §5.1 status per the LOCKED §5.1-R7 order
//! (`locked/conflicts > dirty > ahead/behind > clean`, ARCHITECTURE.md:53).
//!
//! Fixtures are hermetic (`tempfile` + `git2::Repository::init`), per edges-001 — except the
//! relative-worktrees fixture, which uses the git CLI to create a genuinely relative linked worktree
//! (a git-CLI-era feature); the IMPL under test stays git2-read-only.

use std::path::Path;
use std::process::Command;

use git2::{IndexEntry, IndexTime, Oid, Repository, RepositoryInitOptions, Signature};
use tempfile::tempdir;

use nexusops_shared::status::{WorktreeGit, WorktreeOverlay};

use nexusopsd::git::precedence::{derive_worktree_status, DerivedWorktreeStatus};
use nexusopsd::git::reads::{list_linked_worktrees, read_worktree_status};

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

/// Craft an unmerged index entry pair (stages 2 + 3) for `name`, so `index.has_conflicts()` is true —
/// a hermetic conflict without driving a real merge.
fn add_conflict(repo: &Repository, name: &str) {
    let mut index = repo.index().expect("index");
    let ours = repo.blob(b"ours\n").expect("blob ours");
    let theirs = repo.blob(b"theirs\n").expect("blob theirs");
    for (stage, oid) in [(2u16, ours), (3u16, theirs)] {
        let entry = IndexEntry {
            ctime: IndexTime::new(0, 0),
            mtime: IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: oid,
            flags: (stage << 12) | (name.len() as u16 & 0x0fff),
            flags_extended: 0,
            path: name.as_bytes().to_vec(),
        };
        index.add(&entry).expect("add conflict entry");
    }
    index.write().expect("write index");
}

/// Diverge HEAD (`main`) from a `base` branch: both fork at C1, each gains one extra commit.
/// After this: ahead(main, base) == 1 and behind(main, base) == 1; the worktree stays clean.
fn make_divergence(repo: &Repository) {
    let c1 = commit_file(repo, "f.txt", "1");
    let c1_commit = repo.find_commit(c1).expect("c1");
    repo.branch("base", &c1_commit, false).expect("base branch");
    commit_file(repo, "f.txt", "2"); // C2 on main (HEAD advances)
                                     // base gains B2 (child of C1) without moving HEAD/worktree (reuse C1's tree → empty-delta commit).
    let tree = c1_commit.tree().expect("c1 tree");
    let sig = Signature::now("Test", "test@example.com").expect("sig");
    repo.commit(
        Some("refs/heads/base"),
        &sig,
        &sig,
        "B2",
        &tree,
        &[&c1_commit],
    )
    .expect("base commit");
}

// ---- reads -----------------------------------------------------------------

#[test]
fn worktree_read_clean() {
    // spec(§5.1): a fresh commit + clean tree → WorktreeGit::Clean.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    let st = read_worktree_status(dir.path(), None).expect("some");
    assert_eq!(st.git_axis, WorktreeGit::Clean);
}

#[test]
fn worktree_read_dirty() {
    // spec(§7.2): a modified tracked file → Dirty.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    std::fs::write(dir.path().join("a.txt"), "changed").unwrap();
    assert_eq!(
        read_worktree_status(dir.path(), None).unwrap().git_axis,
        WorktreeGit::Dirty
    );
}

#[test]
fn worktree_read_untracked() {
    // spec(§5.1): untracked-only (no tracked changes) → UntrackedFiles (distinct from Dirty).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    std::fs::write(dir.path().join("new.txt"), "untracked").unwrap();
    assert_eq!(
        read_worktree_status(dir.path(), None).unwrap().git_axis,
        WorktreeGit::UntrackedFiles
    );
}

#[test]
fn worktree_read_conflicts() {
    // spec(§5.1): an unmerged index → Conflicts (the precedence-critical state).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    add_conflict(&repo, "a.txt");
    assert_eq!(
        read_worktree_status(dir.path(), None).unwrap().git_axis,
        WorktreeGit::Conflicts
    );
}

#[test]
fn worktree_read_ahead_behind() {
    // spec(§5.1): commits ahead + behind a base → ahead_count/behind_count + the BehindBase signal
    // (clean tree → the divergence surfaces on the git axis).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    make_divergence(&repo);
    let st = read_worktree_status(dir.path(), Some("base")).expect("some");
    assert_eq!(st.ahead_count, Some(1));
    assert_eq!(st.behind_count, Some(1));
    assert_eq!(st.git_axis, WorktreeGit::BehindBase);
}

#[test]
fn worktree_read_dirty_masks_divergence() {
    // spec(§5.1): §5.1-R7 within-git-axis order — dirty > ahead/behind. A worktree that is BOTH
    // dirty AND behind reports git_axis=Dirty (NOT BehindBase), with the divergence counts still
    // populated. (Pins the §5.1 ordering the brief's Q1 default inverted.)
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    make_divergence(&repo);
    std::fs::write(dir.path().join("f.txt"), "dirty-now").unwrap();
    let st = read_worktree_status(dir.path(), Some("base")).expect("some");
    assert_eq!(st.git_axis, WorktreeGit::Dirty);
    assert_eq!(st.behind_count, Some(1));
}

#[test]
fn worktree_read_branch_and_head() {
    // spec(§7.2): current branch name + HEAD commit sha.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let oid = commit_file(&repo, "a.txt", "hello");
    let st = read_worktree_status(dir.path(), None).expect("some");
    assert_eq!(st.branch.as_deref(), Some("main"));
    assert_eq!(
        st.last_commit_sha.as_deref(),
        Some(oid.to_string().as_str())
    );
}

#[test]
fn worktree_read_ahead_only() {
    // spec(§5.1): strictly ahead of base (behind==0) → ahead_count + git_axis=AheadOfBase (the
    // least-reachable git-axis branch — only when behind==0).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let c1 = commit_file(&repo, "f.txt", "1");
    repo.branch("base", &repo.find_commit(c1).unwrap(), false)
        .unwrap();
    commit_file(&repo, "f.txt", "2"); // main ahead of base by 1; base not ahead
    let st = read_worktree_status(dir.path(), Some("base")).expect("some");
    assert_eq!(st.ahead_count, Some(1));
    assert_eq!(st.behind_count, Some(0));
    assert_eq!(st.git_axis, WorktreeGit::AheadOfBase);
}

#[test]
fn worktree_read_detached_head() {
    // spec(§7.2): detached HEAD → branch=None, last_commit_sha still the HEAD oid.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let oid = commit_file(&repo, "a.txt", "hello");
    repo.set_head_detached(oid).unwrap();
    let st = read_worktree_status(dir.path(), None).expect("some");
    assert!(st.branch.is_none());
    assert_eq!(
        st.last_commit_sha.as_deref(),
        Some(oid.to_string().as_str())
    );
}

#[test]
fn worktree_read_unresolvable_base() {
    // spec(§9): a base ref that won't resolve → ahead/behind None (degraded); git_axis falls to Clean.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    let st = read_worktree_status(dir.path(), Some("no-such-branch")).expect("some");
    assert_eq!(st.ahead_count, None);
    assert_eq!(st.behind_count, None);
    assert_eq!(st.git_axis, WorktreeGit::Clean);
}

#[test]
fn worktree_read_worktree_list() {
    // spec(§9): linked worktrees are enumerated (git2 worktrees()).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    let wt_path = dir.path().join("wt1");
    repo.worktree("wt1", &wt_path, None)
        .expect("create worktree");
    assert!(list_linked_worktrees(dir.path()).iter().any(|n| n == "wt1"));
}

#[test]
fn worktree_read_does_not_mutate() {
    // spec(§9): forbidden #6 — the reads do not mutate the repo (HEAD oid unchanged).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let oid = commit_file(&repo, "a.txt", "hello");
    let before = repo.head().unwrap().target().unwrap();
    let _ = read_worktree_status(dir.path(), None);
    let _ = list_linked_worktrees(dir.path());
    let after = repo.head().unwrap().target().unwrap();
    assert_eq!(before, after);
    assert_eq!(after, oid);
}

#[test]
fn worktree_read_relative_worktrees() {
    // spec(§9 / OQ-INT-SPIKE-6): a repo with `extensions.relativeWorktrees` reads via git2 (no CLI
    // fallback) — libgit2 ≥1.9.4 tolerates the extension. The worktree is created via the git CLI
    // (a CLI-era feature → genuinely relative); the IMPL under test is git2-read-only.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_i32("core.repositoryformatversion", 1).unwrap();
        cfg.set_bool("extensions.relativeWorktrees", true).unwrap();
    }
    let wt_path = dir.path().join("rel-wt");
    let out = Command::new("git")
        .args([
            "-C",
            dir.path().to_str().unwrap(),
            "worktree",
            "add",
            wt_path.to_str().unwrap(),
        ])
        .output()
        .expect("run git worktree add");
    assert!(
        out.status.success(),
        "git worktree add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(list_linked_worktrees(dir.path())
        .iter()
        .any(|n| n == "rel-wt"));
    assert!(read_worktree_status(&wt_path, None).is_some());
}

#[test]
fn worktree_read_non_git_degraded() {
    // spec(§9): a non-git / missing path → None (degraded), never a panic (edges-001 posture).
    let dir = tempdir().unwrap();
    assert!(read_worktree_status(dir.path(), None).is_none());
    assert!(read_worktree_status(&dir.path().join("nope"), None).is_none());
    assert!(list_linked_worktrees(dir.path()).is_empty());
}

// ---- precedence ------------------------------------------------------------

#[test]
fn precedence_conflicts_over_dirty() {
    // spec(§5.1): Conflicts is the top non-terminal — it outranks every overlay but Deleted, and
    // (transitively, via locked > dirty) outranks Dirty. (§5.1: locked/conflicts > dirty.)
    use WorktreeGit::*;
    use WorktreeOverlay::*;
    assert_eq!(
        derive_worktree_status(Conflicts, Some(Locked)),
        DerivedWorktreeStatus::Git(Conflicts)
    );
    assert_eq!(
        derive_worktree_status(Dirty, Some(Locked)),
        DerivedWorktreeStatus::Overlay(Locked)
    );
}

#[test]
fn precedence_locked_over_dirty() {
    // spec(§5.1): a Locked overlay outranks a Dirty git axis (§5.1: locked > dirty).
    assert_eq!(
        derive_worktree_status(WorktreeGit::Dirty, Some(WorktreeOverlay::Locked)),
        DerivedWorktreeStatus::Overlay(WorktreeOverlay::Locked)
    );
}

#[test]
fn precedence_deleted_dominates() {
    // spec(§5.1): the terminal Deleted overlay dominates ALL — even Conflicts.
    use WorktreeGit::*;
    for git in [
        Clean,
        Dirty,
        Conflicts,
        BehindBase,
        AheadOfBase,
        UntrackedFiles,
    ] {
        assert_eq!(
            derive_worktree_status(git, Some(WorktreeOverlay::Deleted)),
            DerivedWorktreeStatus::Overlay(WorktreeOverlay::Deleted)
        );
    }
}

#[test]
fn precedence_clean_baseline() {
    // spec(§5.1): baseline — Clean + no overlay → Clean; Clean + Creating → Creating (creating > clean).
    assert_eq!(
        derive_worktree_status(WorktreeGit::Clean, None),
        DerivedWorktreeStatus::Git(WorktreeGit::Clean)
    );
    assert_eq!(
        derive_worktree_status(WorktreeGit::Clean, Some(WorktreeOverlay::Creating)),
        DerivedWorktreeStatus::Overlay(WorktreeOverlay::Creating)
    );
}

#[test]
fn precedence_full_order() {
    // spec(§5.1 R-7): the §5.1-faithful total precedence (locked/conflicts > dirty > ahead/behind >
    // clean), filled with the overlay-lifecycle + terminal tiers. Each row = (git, overlay) → winner.
    use DerivedWorktreeStatus::{Git, Overlay};
    use WorktreeGit as G;
    use WorktreeOverlay as O;
    let cases: &[(G, Option<O>, DerivedWorktreeStatus)] = &[
        // deleted dominates all
        (G::Conflicts, Some(O::Deleted), Overlay(O::Deleted)),
        (G::Clean, Some(O::Deleted), Overlay(O::Deleted)),
        // conflicts is the top non-terminal
        (G::Conflicts, Some(O::Locked), Git(G::Conflicts)),
        (G::Conflicts, Some(O::Merged), Git(G::Conflicts)),
        // locked > dirty/divergence
        (G::Dirty, Some(O::Locked), Overlay(O::Locked)),
        (G::BehindBase, Some(O::Locked), Overlay(O::Locked)),
        // merged / pr_open / prunable outrank the git axis below locked
        (G::Dirty, Some(O::Merged), Overlay(O::Merged)),
        (G::Dirty, Some(O::PrOpen), Overlay(O::PrOpen)),
        (G::BehindBase, Some(O::Prunable), Overlay(O::Prunable)),
        // dirty > ahead/behind > creating > clean (§5.1 locked order)
        (G::Dirty, Some(O::Creating), Git(G::Dirty)),
        (G::BehindBase, Some(O::Creating), Git(G::BehindBase)),
        (G::Dirty, None, Git(G::Dirty)),
        (G::BehindBase, None, Git(G::BehindBase)),
        (G::Clean, Some(O::Creating), Overlay(O::Creating)),
        (G::Clean, None, Git(G::Clean)),
    ];
    for (git, overlay, expected) in cases {
        assert_eq!(
            derive_worktree_status(*git, *overlay),
            *expected,
            "derive({git:?}, {overlay:?})"
        );
    }
}

#[test]
fn derived_wire_str_matches_serde() {
    // The hand-mapped `as_wire_str` must equal the FROZEN serde snake_case wire value for every
    // variant of both axes (LESSON §2 — the wire value is the contract; pin the parity, don't trust).
    for &g in WorktreeGit::ALL {
        let wire = serde_json::to_value(g)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(DerivedWorktreeStatus::Git(g).as_wire_str(), wire);
    }
    for &o in WorktreeOverlay::ALL {
        let wire = serde_json::to_value(o)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(DerivedWorktreeStatus::Overlay(o).as_wire_str(), wire);
    }
}

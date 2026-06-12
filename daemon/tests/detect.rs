//! P5.1 — the project-detection engine (the pure, deterministic logic the `project.rescan`
//! Gateway executor will call; the executor wiring + event emission + registry/projector land in
//! `edges-002`, cross-track-gated).
//!
//! Two independent detectors:
//!   * `git::detect::detect_git`      — git2 READ-ONLY repo introspection (§9 / §7.2 git-state SoT).
//!   * `workflow::detect::detect_workflow` — file-signal detection (§7.2 WorkflowInstance/PlanTask SoT).
//!
//! **Forbidden #6** is the governing safety rule: git2 is read-only here — `git_detect_does_not_mutate_repo`
//! pins it with a before/after HEAD assertion. Fixtures are hermetic (`tempfile` + `git2::Repository::init`
//! + programmatic commits/remotes) — no shelling to `git`, no committed fixture repos.

use std::fs;
use std::path::Path;

use git2::{Repository, RepositoryInitOptions, Signature};
use tempfile::tempdir;

use nexusopsd::git::detect::detect_git;
use nexusopsd::workflow::detect::detect_workflow;

// ---- hermetic git fixtures -------------------------------------------------

/// Init a fresh repo at `path` with `main` as the initial branch (deterministic branch name,
/// independent of the host's `init.defaultBranch`).
fn init_repo(path: &Path) -> Repository {
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    Repository::init_opts(path, &opts).expect("init repo")
}

/// Commit a single file into `repo`'s workdir; returns the new commit oid. Parents are read from the
/// current HEAD (so the first commit is a root commit, later commits chain).
fn commit_file(repo: &Repository, name: &str, content: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("non-bare repo");
    fs::write(workdir.join(name), content).expect("write file");
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

// ---- git detection ---------------------------------------------------------

#[test]
fn git_detect_repo_root() {
    // spec(§9): a git repo at a path → is_git + the resolved repo root.
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    let d = detect_git(dir.path());
    assert!(d.is_git);
    assert_eq!(d.repo_root, Some(dir.path().canonicalize().unwrap()));
}

#[test]
fn git_detect_origin_remote_url() {
    // spec(§9): the `origin` remote URL is extracted (feeds P7.1 GitHub linking).
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    repo.remote("origin", "https://github.com/example/repo.git")
        .unwrap();
    let d = detect_git(dir.path());
    assert_eq!(
        d.remote_url.as_deref(),
        Some("https://github.com/example/repo.git")
    );
}

#[test]
fn git_detect_no_remote() {
    // spec(§9): no remote → remote_url is None, not an error.
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    assert_eq!(detect_git(dir.path()).remote_url, None);
}

#[test]
fn git_detect_current_branch() {
    // spec(§7.2): current branch is reported; a normal HEAD is not detached.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    let d = detect_git(dir.path());
    assert_eq!(d.branch.as_deref(), Some("main"));
    assert!(!d.detached);
}

#[test]
fn git_detect_detached_head() {
    // spec(§7.2): detached HEAD → detached=true AND no branch shorthand, never a panic.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let oid = commit_file(&repo, "a.txt", "hello");
    repo.set_head_detached(oid).unwrap();
    let d = detect_git(dir.path());
    assert!(d.detached);
    assert!(d.branch.is_none());
}

#[test]
fn git_detect_unborn_head() {
    // spec(§7.2): a fresh repo with no commits (unborn HEAD) → is_git, not detached, no branch — no panic.
    let dir = tempdir().unwrap();
    init_repo(dir.path());
    let d = detect_git(dir.path());
    assert!(d.is_git);
    assert!(!d.detached);
    assert!(d.branch.is_none());
}

#[test]
fn git_detect_bare_repo() {
    // spec(§9): a bare repo → is_git but no workdir → repo_root=None — pins the documented
    // GitDetection.repo_root contract (callers must not assume is_git ⇒ repo_root.is_some()).
    let dir = tempdir().unwrap();
    git2::Repository::init_bare(dir.path()).unwrap();
    let d = detect_git(dir.path());
    assert!(d.is_git);
    assert_eq!(d.repo_root, None);
}

#[test]
fn git_detect_dirty_clean() {
    // spec(§7.2): the minimal git-state set — clean tree is clean; an untracked file makes it dirty.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "a.txt", "hello");
    assert!(!detect_git(dir.path()).is_dirty);
    fs::write(dir.path().join("untracked.txt"), "new").unwrap();
    assert!(detect_git(dir.path()).is_dirty);
}

#[test]
fn git_detect_non_git_path() {
    // spec(§9): a non-git path → is_git=false, a valid degraded result (no Err/panic).
    let dir = tempdir().unwrap();
    let d = detect_git(dir.path());
    assert!(!d.is_git);
    assert_eq!(d.repo_root, None);
    assert_eq!(d.branch, None);
}

#[test]
fn git_detect_missing_path() {
    // spec(§9): a nonexistent path → typed degraded result, never a panic/unwrap (typing posture).
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does-not-exist");
    assert!(!detect_git(&missing).is_git);
}

#[test]
fn git_detect_does_not_mutate_repo() {
    // spec(§9): forbidden #6 — git2 detection is read-only; repo HEAD is unchanged after detect.
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let oid = commit_file(&repo, "a.txt", "hello");
    let head_before = repo.head().unwrap().target().unwrap();
    let _ = detect_git(dir.path());
    let head_after = repo.head().unwrap().target().unwrap();
    assert_eq!(head_before, head_after);
    assert_eq!(head_after, oid);
}

// ---- workflow / file-signal detection --------------------------------------

#[test]
fn workflow_detect_pack_manifest() {
    // spec(§7.2): WorkflowInstance SoT — `.scaffolding/manifest.json` presence.
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".scaffolding")).unwrap();
    fs::write(dir.path().join(".scaffolding").join("manifest.json"), "{}").unwrap();
    assert!(detect_workflow(dir.path()).workflow_pack);
}

#[test]
fn workflow_detect_cc_crew() {
    // spec(§9): cc-crew scaffolding — a `.claude/` directory.
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    assert!(detect_workflow(dir.path()).cc_crew);
}

#[test]
fn workflow_detect_plan_file() {
    // spec(§7.2): PlanTask SoT — the plan file + its path (MVP_TASKS.md canonical | IMPLEMENTATION_PLAN.md).
    let mvp = tempdir().unwrap();
    fs::write(mvp.path().join("MVP_TASKS.md"), "# tasks").unwrap();
    assert_eq!(
        detect_workflow(mvp.path()).plan_file,
        Some(mvp.path().join("MVP_TASKS.md"))
    );

    let plan = tempdir().unwrap();
    fs::write(plan.path().join("IMPLEMENTATION_PLAN.md"), "# plan").unwrap();
    assert_eq!(
        detect_workflow(plan.path()).plan_file,
        Some(plan.path().join("IMPLEMENTATION_PLAN.md"))
    );
}

#[test]
fn workflow_detect_plan_file_precedence() {
    // spec(§7.2): both plan files present → the §7.2-canonical MVP_TASKS.md wins (first-match order).
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("MVP_TASKS.md"), "# tasks").unwrap();
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), "# plan").unwrap();
    assert_eq!(
        detect_workflow(dir.path()).plan_file,
        Some(dir.path().join("MVP_TASKS.md"))
    );
}

#[test]
fn workflow_detect_brain_presence() {
    // spec(§9): Brain is presence-only here (full Brain status = Phase 8 brainclient).
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".brain")).unwrap();
    assert!(detect_workflow(dir.path()).brain);
}

#[test]
fn workflow_detect_none() {
    // spec(§9): a bare directory → all signals absent, a valid result ("basic project w/ no pack works").
    let dir = tempdir().unwrap();
    let w = detect_workflow(dir.path());
    assert!(!w.workflow_pack);
    assert!(!w.cc_crew);
    assert_eq!(w.plan_file, None);
    assert!(!w.brain);
}

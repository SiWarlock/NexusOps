//! edges-026 (P5.2 follow-on) — the §7.2 worktree-status LIVE-READ cache refresh.
//!
//! Reads a worktree's live git truth (`read_worktree_status`, git2 read-only, forbidden #6) and
//! writes the git-axis cache columns of `proj_worktree` (`dirty_state`/`ahead_count`/`behind_count`/
//! `last_commit_sha`/`git_checked_at` + the recomputed `status`) through a NEW **non-Gateway,
//! non-event** write-actor command (`RefreshWorktreeStatus`), triggered by a git-watcher interval
//! task (the drainer/reaper precedent). NO `WorktreeStatusRefreshed` event — the git-axis is a
//! live-read projection cache, not event-sourced (§7.1).
//!
//! Fixtures are hermetic (`tempfile` + `git2::Repository::init`, per edges-001/worktree_reads.rs);
//! the proj_worktree row is seeded Gateway-end-to-end (the edges-022 precedent) so a real
//! WorktreeCreated event + action_requests sibling exist (rebuild-meaningful).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use git2::{Oid, Repository, RepositoryInitOptions, Signature};
use tempfile::tempdir;
use tokio::sync::watch;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::ids::{ActionRequestId, ProjectId};
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::{CatalogExecutor, StubExecutor};
use nexusopsd::gateway::policy::{CatalogPolicy, StubPolicy};
use nexusopsd::gateway::Gateway;
use nexusopsd::git::cli::FakeGitCli;
use nexusopsd::git::executor::GitExecutor;
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::{compute_worktree_cache, spawn_git_watcher, WriteActor};

/// the write-actor's injected clock — `git_checked_at` is stamped from THIS (UTC-Z, LESSON §5).
const CHECKED_AT: &str = "2026-06-13T12:00:00Z";

// ---- fixtures --------------------------------------------------------------

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

fn init_repo(path: &Path) -> Repository {
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    Repository::init_opts(path, &opts).expect("init repo")
}

/// Commit `name`=`content` onto HEAD; returns the new commit oid (worktree_reads.rs precedent).
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

/// Diverge `main` from a `base` branch: both fork at C1, each gains one commit → ahead==1 && behind==1,
/// worktree stays clean (so resolve_git_axis surfaces the divergence, not Dirty). worktree_reads.rs precedent.
fn make_divergence(repo: &Repository) {
    let c1 = commit_file(repo, "f.txt", "1");
    let c1_commit = repo.find_commit(c1).expect("c1");
    repo.branch("base", &c1_commit, false).expect("base branch");
    commit_file(repo, "f.txt", "2"); // C2 on main (HEAD advances)
    let tree = c1_commit.tree().expect("c1 tree");
    let sig = Signature::now("Test", "test@example.com").expect("sig");
    repo.commit(Some("refs/heads/base"), &sig, &sig, "B2", &tree, &[&c1_commit])
        .expect("base commit");
}

// ---- gateway-seed of a proj_worktree row (edges-022 precedent) -------------

fn gw_with_git() -> Gateway {
    let mut cat = CatalogExecutor::new();
    cat.register(
        ExecutorKind::Git,
        Arc::new(GitExecutor::new(Box::new(FakeGitCli::succeeding()))),
    );
    Gateway::new(Box::new(CatalogPolicy), Box::new(cat))
}

/// A stub gateway for the write-actor (the refresh path never touches it — it is a non-Gateway command).
fn stub_gateway() -> Gateway {
    Gateway::new(Box::new(StubPolicy), Box::new(StubExecutor))
}

/// A `git.create_worktree` request. LOW-ENTROPY operational inputs (the §7.2 approve-path redaction —
/// a high-entropy tempdir path would be masked, edges-020 FINDING); the refresh's real git path is passed
/// to the command separately, decoupled from this seeded `proj_worktree.path`.
fn create_worktree_req(worktree_path: &str, base_branch: &str) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: Some(ProjectId::new()),
        action_type: "git.create_worktree".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: "repo_alpha".to_string(),
            uri: None,
        }],
        inputs: serde_json::json!({
            "repo_path": "/repo", "worktree_path": worktree_path,
            "branch_name": "feature", "base_branch": base_branch
        }),
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-08T00:00:00Z").unwrap(),
    }
}

fn approval_id_of(path: &Path) -> String {
    let conn = open_read_only(path).expect("ro conn");
    conn.query_row(
        "SELECT approval_id FROM approvals ORDER BY approval_id LIMIT 1",
        [],
        |r| r.get(0),
    )
    .expect("an approval")
}

/// submit + approve a git.create_worktree → drives WorktreeCreated + the in-band proj_worktree fold.
/// Returns the minted `worktree_id`.
fn seed_worktree(store: &mut EventStore, path: &Path, worktree_path: &str, base_branch: &str) -> String {
    let gw = gw_with_git();
    gw.submit_action(store, create_worktree_req(worktree_path, base_branch))
        .expect("submit");
    gw.approve(store, &approval_id_of(path)).expect("approve");
    let conn = open_read_only(path).expect("ro conn");
    conn.query_row("SELECT worktree_id FROM proj_worktree LIMIT 1", [], |r| {
        r.get(0)
    })
    .expect("a seeded worktree row")
}

// ---- proj_worktree row reader ----------------------------------------------

#[derive(Debug)]
struct WtRow {
    path: String,
    branch_name: Option<String>,
    base_branch: Option<String>,
    repo_id: String,
    status: String,
    dirty_state: Option<String>,
    ahead_count: Option<i64>,
    behind_count: Option<i64>,
    last_commit_sha: Option<String>,
    git_checked_at: Option<String>,
}

fn read_row(path: &Path, wt_id: &str) -> WtRow {
    let conn = open_read_only(path).expect("ro conn");
    conn.query_row(
        "SELECT path, branch_name, base_branch, repo_id, status, dirty_state, ahead_count, \
         behind_count, last_commit_sha, git_checked_at FROM proj_worktree WHERE worktree_id = ?1",
        [wt_id],
        |r| {
            Ok(WtRow {
                path: r.get(0)?,
                branch_name: r.get(1)?,
                base_branch: r.get(2)?,
                repo_id: r.get(3)?,
                status: r.get(4)?,
                dirty_state: r.get(5)?,
                ahead_count: r.get(6)?,
                behind_count: r.get(7)?,
                last_commit_sha: r.get(8)?,
                git_checked_at: r.get(9)?,
            })
        },
    )
    .expect("the worktree row")
}

fn count_events(path: &Path) -> i64 {
    let conn = open_read_only(path).expect("ro conn");
    conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap()
}

// =============================================================================
// 1. fills the git-axis live-read cache; event-sourced cols untouched
// =============================================================================

#[tokio::test]
async fn test_refresh_fills_git_axis_cache() {
    // spec(§7.2): a refresh reads live git truth → fills dirty_state/ahead/behind/last_commit_sha/
    // git_checked_at; the event-sourced cols (path/branch/base/repo_id) are NOT touched.
    let (_d, dbpath) = temp_db();
    let mut store = open(&dbpath);
    let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");
    let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());
    let handle = actor.handle();

    // a REAL diverged repo (ahead==1, behind==1, clean tree → BehindBase) the refresh reads.
    let repo_dir = tempdir().unwrap();
    make_divergence(&init_repo(repo_dir.path()));

    let rows = handle
        .refresh_worktree_status(
            wt_id.clone(),
            repo_dir.path().to_string_lossy().into_owned(),
            Some("base".to_string()),
        )
        .await
        .expect("refresh via the write-actor");
    assert_eq!(rows, 1, "the keyed row was updated");
    actor.shutdown().await;

    let r = read_row(&dbpath, &wt_id);
    assert_eq!(r.dirty_state.as_deref(), Some("behind_base"), "git_axis wire value");
    assert_eq!(r.ahead_count, Some(1));
    assert_eq!(r.behind_count, Some(1));
    assert!(r.last_commit_sha.is_some(), "HEAD sha cached");
    assert_eq!(r.git_checked_at.as_deref(), Some(CHECKED_AT));
    // event-sourced cols untouched (the refresh writes ONLY the live-read cache):
    assert_eq!(r.path, "/repo/wt");
    assert_eq!(r.branch_name.as_deref(), Some("feature"));
    assert_eq!(r.base_branch.as_deref(), Some("base"));
    assert_eq!(r.repo_id, "repo_alpha");
}

// =============================================================================
// 2. status is recomputed from the live git axis (§5.1 precedence)
// =============================================================================

#[tokio::test]
async fn test_refresh_recomputes_status_from_live_git_axis() {
    // spec(§5.1): the recomputed status = derive_worktree_status(live git_axis, Creating overlay) —
    // a DIRTY tree (rank 6) outranks the Creating overlay (rank 10) → "dirty"; a CLEAN tree (rank 11)
    // is outranked BY Creating → stays "creating" (the only emitted overlay, Q2).

    // (a) dirty → "dirty"
    {
        let (_d, dbpath) = temp_db();
        let mut store = open(&dbpath);
        let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");
        let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());
        let repo_dir = tempdir().unwrap();
        let repo = init_repo(repo_dir.path());
        commit_file(&repo, "a.txt", "hello");
        std::fs::write(repo_dir.path().join("a.txt"), "changed").unwrap(); // dirty (tracked change)
        actor
            .handle()
            .refresh_worktree_status(wt_id.clone(), repo_dir.path().to_string_lossy().into_owned(), None)
            .await
            .unwrap();
        actor.shutdown().await;
        assert_eq!(read_row(&dbpath, &wt_id).status, "dirty");
    }

    // (b) clean → "creating" (the Creating overlay wins over a clean git axis)
    {
        let (_d, dbpath) = temp_db();
        let mut store = open(&dbpath);
        let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");
        let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());
        let repo_dir = tempdir().unwrap();
        let repo = init_repo(repo_dir.path());
        commit_file(&repo, "a.txt", "hello"); // clean tree
        actor
            .handle()
            .refresh_worktree_status(wt_id.clone(), repo_dir.path().to_string_lossy().into_owned(), None)
            .await
            .unwrap();
        actor.shutdown().await;
        assert_eq!(read_row(&dbpath, &wt_id).status, "creating");
    }
}

// =============================================================================
// 3. git_checked_at is the injected daemon Clock, UTC-Z
// =============================================================================

#[tokio::test]
async fn test_refresh_stamps_git_checked_at_utc_z() {
    // spec(§7.2 / LESSON §5): the freshness stamp is the write-actor's injected Clock, UTC-Z (a `Z`
    // suffix — the projector's bucket-day / staleness derivations rely on the UTC-Z invariant).
    let (_d, dbpath) = temp_db();
    let mut store = open(&dbpath);
    let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");
    let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());
    let repo_dir = tempdir().unwrap();
    commit_file(&init_repo(repo_dir.path()), "a.txt", "hi");
    actor
        .handle()
        .refresh_worktree_status(wt_id.clone(), repo_dir.path().to_string_lossy().into_owned(), None)
        .await
        .unwrap();
    actor.shutdown().await;

    let stamp = read_row(&dbpath, &wt_id).git_checked_at.expect("stamped");
    assert_eq!(stamp, CHECKED_AT);
    assert!(stamp.ends_with('Z'), "UTC-Z freshness stamp");
}

// =============================================================================
// 4. a non-git path stamps git_checked_at only (typed absence, Q3)
// =============================================================================

#[tokio::test]
async fn test_refresh_non_git_path_stamps_checked_only() {
    // spec(Q3): a None read (non-git / inaccessible path) → stamp git_checked_at (we checked; no git
    // truth), leave the git-axis cols + status unchanged. No panic.
    let (_d, dbpath) = temp_db();
    let mut store = open(&dbpath);
    let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");
    let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());
    let non_git = tempdir().unwrap(); // a real dir that is NOT a git repo

    let rows = actor
        .handle()
        .refresh_worktree_status(wt_id.clone(), non_git.path().to_string_lossy().into_owned(), None)
        .await
        .expect("refresh does not panic on a non-git path");
    assert_eq!(rows, 1, "the row was touched (git_checked_at stamped)");
    actor.shutdown().await;

    let r = read_row(&dbpath, &wt_id);
    assert_eq!(r.git_checked_at.as_deref(), Some(CHECKED_AT), "we checked");
    assert_eq!(r.dirty_state, None, "no git truth → git-axis cols left as-is (NULL)");
    assert_eq!(r.ahead_count, None);
    assert_eq!(r.behind_count, None);
    assert_eq!(r.last_commit_sha, None);
    assert_eq!(r.status, "creating", "status unchanged on a None read");
}

// =============================================================================
// 5. the refresh is a write-actor command — NO event (forbidden #3 / §7.1)
// =============================================================================

#[tokio::test]
async fn test_refresh_is_write_actor_not_gateway_no_event() {
    // spec(§7.1 / forbidden #3): the live-read cache is NOT event-sourced — a refresh appends NO event
    // (no WorktreeStatusRefreshed, no change to the events count) and writes via the single write-actor.
    let (_d, dbpath) = temp_db();
    let mut store = open(&dbpath);
    let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");
    let before = count_events(&dbpath);
    let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());
    let repo_dir = tempdir().unwrap();
    commit_file(&init_repo(repo_dir.path()), "a.txt", "hi");
    actor
        .handle()
        .refresh_worktree_status(wt_id, repo_dir.path().to_string_lossy().into_owned(), None)
        .await
        .unwrap();
    actor.shutdown().await;

    assert_eq!(count_events(&dbpath), before, "a refresh appends NO event");
    let conn = open_read_only(&dbpath).unwrap();
    let refreshed_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_type = 'WorktreeStatusRefreshed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(refreshed_events, 0, "there is no WorktreeStatusRefreshed event type");
}

// =============================================================================
// 6. a refresh for an unknown worktree_id is a no-op (fail-safe)
// =============================================================================

#[tokio::test]
async fn test_refresh_unknown_worktree_id_noop() {
    // spec(fail-safe): a refresh for a worktree_id with no proj_worktree row → 0 rows updated, no error.
    let (_d, dbpath) = temp_db();
    let store = open(&dbpath); // no seeded row
    let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());
    let non_git = tempdir().unwrap();
    let rows = actor
        .handle()
        .refresh_worktree_status(
            "wt_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            non_git.path().to_string_lossy().into_owned(),
            None,
        )
        .await
        .expect("no error");
    assert_eq!(rows, 0, "unknown worktree_id → no-op (0 rows)");
    actor.shutdown().await;
}

// =============================================================================
// 7. the git-watcher refreshes known worktrees (reachability — Step 7.5)
// =============================================================================

#[tokio::test]
async fn test_git_watcher_refreshes_known_worktrees() {
    // spec(Step 7.5 reachability): the git-watcher interval task enumerates proj_worktree + issues a
    // refresh per row through the write-actor — the PRODUCTION trigger (the drainer/reaper precedent).
    // A previously-unstamped row gets git_checked_at stamped after a watcher tick. NOTE: the watcher's
    // refresh-with-a-REAL-git-path → live cache values is covered by the COMPOSITION of this test (the
    // watcher enumerates + dispatches `refresh_worktree_status` per row) and tests #1-#3 (that same
    // command, given a real path, fills the cache / recomputes status / stamps UTC-Z). A single
    // watcher→real-repo e2e is not pinned here because the seeded `proj_worktree.path` is §15-redaction-
    // masked for a high-entropy tempdir (edges-020 FINDING), so the real path is passed to the command
    // directly in #1-#3 instead.
    let (_d, dbpath) = temp_db();
    let mut store = open(&dbpath);
    let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");
    assert!(
        read_row(&dbpath, &wt_id).git_checked_at.is_none(),
        "seed leaves git_checked_at NULL"
    );
    let actor = WriteActor::spawn(store, Box::new(FixedClock::new(CHECKED_AT)), stub_gateway());

    let (sd_tx, sd_rx) = watch::channel(false);
    let watcher = spawn_git_watcher(actor.handle(), dbpath.clone(), Duration::from_millis(2), sd_rx);
    tokio::time::sleep(Duration::from_millis(40)).await; // let the watcher tick
    sd_tx.send(true).unwrap();
    watcher.await.expect("watcher joins cleanly on shutdown");

    assert_eq!(
        read_row(&dbpath, &wt_id).git_checked_at.as_deref(),
        Some(CHECKED_AT),
        "the watcher issued a refresh for the known worktree"
    );
    actor.shutdown().await;
}

// =============================================================================
// 8. a rebuild RESETS the live-read cache (REBUILD_TABLES — not event-sourced)
// =============================================================================

#[test]
fn test_proj_worktree_rebuild_resets_live_read_cache() {
    // spec(§7.2 REBUILD_TABLES): the live-read cache is NOT event-sourced — a full rebuild truncates +
    // replays from events, resetting the git-axis cols to NULL + status to the event-derived "creating"
    // (the watcher repopulates post-rebuild); the edges-022 event-sourced cols still hold.
    let (_d, dbpath) = temp_db();
    let mut store = open(&dbpath);
    let wt_id = seed_worktree(&mut store, &dbpath, "/repo/wt", "base");

    // a real dirty repo → a populated cache (status "dirty").
    let repo_dir = tempdir().unwrap();
    let repo = init_repo(repo_dir.path());
    commit_file(&repo, "a.txt", "hello");
    std::fs::write(repo_dir.path().join("a.txt"), "changed").unwrap();
    let cache = compute_worktree_cache(repo_dir.path(), None);
    store
        .refresh_worktree_status(&FixedClock::new(CHECKED_AT), &wt_id, cache.as_ref())
        .expect("refresh");
    let before = read_row(&dbpath, &wt_id);
    assert_eq!(before.status, "dirty");
    assert_eq!(before.dirty_state.as_deref(), Some("dirty"));
    assert!(before.git_checked_at.is_some());

    // a full rebuild → the cache resets (the live-read cache is not in the event log).
    store.rebuild_projections().expect("rebuild");
    let after = read_row(&dbpath, &wt_id);
    assert_eq!(after.status, "creating", "status resets to the event-derived overlay");
    assert_eq!(after.dirty_state, None);
    assert_eq!(after.ahead_count, None);
    assert_eq!(after.behind_count, None);
    assert_eq!(after.last_commit_sha, None);
    assert_eq!(after.git_checked_at, None);
    // the event-sourced cols survive the rebuild (edges-022 rebuild-equivalence):
    assert_eq!(after.path, "/repo/wt");
    assert_eq!(after.repo_id, "repo_alpha");
}

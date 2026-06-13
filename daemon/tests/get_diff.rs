//! Brief 052 — P4.0b-ui1: the hunk-structured `get_diff` read RPC (§6.1; the ui-6.3e diff source).
//!
//! `get_diff(worktree_id, file)` resolves `worktree_id → proj_worktree.path` over a READ-ONLY WAL
//! conn, then reads git2 LIVE (read-only) for the file's HEAD→workdir diff → structured `Hunk`s. NO
//! mutation, NO write-actor (the §7.2 worktree-live-read precedent). MVP: an unpopulated worktree_id
//! → typed NotFound (proj_worktree fills at P5.2/edges); the real read is fixture-tested + P5-ready.

use std::path::Path;

use nexusops_shared::ipc::{DiffLineKind, DiffResult, Hunk};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;

// ---- git2 fixture helpers -----------------------------------------------------------------------

/// init a git repo at `dir`, write `name`=`content`, stage + commit it (so HEAD exists). Returns once
/// the file is committed clean (no workdir diff yet).
fn init_repo_with_committed_file(dir: &Path, name: &str, content: &str) {
    let repo = git2::Repository::init(dir).expect("git init");
    std::fs::write(dir.join(name), content).expect("write file");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(name)).expect("add_path");
    index.write().expect("index write");
    let tree_oid = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_oid).expect("find_tree");
    let sig = git2::Signature::now("Test", "test@example.com").expect("sig");
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .expect("commit");
}

fn temp_db_with_worktree(worktree_id: &str, path: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("nexusops.db");
    // open the store once (runs migrations → proj_worktree exists), then insert a proj_worktree row
    // directly (a test fixture for the wt_id→path resolution; the real projector folds it at P5).
    {
        let _store = EventStore::open(
            &db_path,
            Box::new(UlidGen),
            Box::new(FixedClock::new("2026-06-13T00:00:00Z")),
            Box::new(PrefixRedactor),
        )
        .expect("open store");
    }
    // TEST-FIXTURE-ONLY raw writable conn — bypasses the single-writer actor to seed a proj_worktree
    // row (production never does this; the projector folds WorktreeCreated at P5). The EventStore above
    // is dropped (closed) before this opens, so WAL locking is clean. NOT a pattern for production code.
    let conn = rusqlite::Connection::open(&db_path).expect("writable conn");
    conn.execute(
        "INSERT INTO proj_worktree
           (worktree_id, project_id, repo_id, path, status, updated_at_seq)
         VALUES (?1, 'proj_x', 'repo_x', ?2, 'active', 1)",
        rusqlite::params![worktree_id, path],
    )
    .expect("insert proj_worktree fixture row");
    (dir, db_path)
}

// ---- 052 #5 — read_diff over a git2 fixture returns structured hunks (§6.1) ----------------------

#[test]
fn test_get_diff_returns_structured_hunks() {
    // spec(§6.1) — git::read_diff reads the file's HEAD→workdir diff LIVE via git2 and returns
    // structured Hunks (header + old/new ranges + typed lines context|added|removed). A modified line
    // → a hunk with the removed old line + the added new line; a CLEAN (unmodified) file → no hunks.
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    init_repo_with_committed_file(repo, "f.txt", "alpha\nbeta\ngamma\n");

    // clean (just committed, unmodified) → no hunks.
    let clean = nexusopsd::git::read_diff(repo, "f.txt").expect("read_diff clean");
    assert!(
        clean.hunks.is_empty(),
        "a clean file has no workdir diff hunks"
    );

    // modify the middle line → one hunk with the removed + added line.
    std::fs::write(repo.join("f.txt"), "alpha\nBETA\ngamma\n").unwrap();
    let diff = nexusopsd::git::read_diff(repo, "f.txt").expect("read_diff modified");
    assert_eq!(diff.hunks.len(), 1, "one modified region → one hunk");
    let h: &Hunk = &diff.hunks[0];
    assert!(!h.header.is_empty(), "the hunk carries its @@ header");
    let kinds: Vec<DiffLineKind> = h.lines.iter().map(|l| l.kind).collect();
    assert!(
        kinds.contains(&DiffLineKind::Removed) && kinds.contains(&DiffLineKind::Added),
        "the modified hunk has a removed (beta) + an added (BETA) line"
    );
    assert!(
        h.lines
            .iter()
            .any(|l| l.kind == DiffLineKind::Removed && l.content.contains("beta")),
        "the removed line is the old content"
    );
    assert!(
        h.lines
            .iter()
            .any(|l| l.kind == DiffLineKind::Added && l.content.contains("BETA")),
        "the added line is the new content"
    );
    // the POSITION fields are the load-bearing hunk-identity (the ui packs old_start/new_start into
    // the git.* resource_ref id — §17 read↔mutate consistency). Pin them to concrete values so a
    // regression that zeroes them is caught: the edit is at line 2 of a 3-line file.
    assert_eq!(h.old_start, 1, "the hunk starts at the file head (1-based)");
    assert_eq!(h.new_start, 1);
    assert_eq!(h.old_lines, 3, "the hunk spans the 3 old lines");
    assert_eq!(h.new_lines, 3, "the hunk spans the 3 new lines");
}

// ---- 052 #6 — get_diff resolves wt_id→path read-only; unpopulated → NotFound; no mutation --------

#[test]
fn test_get_diff_read_only() {
    // spec(§6.1 / §15 single-writer) — read_worktree_diff resolves worktree_id → proj_worktree.path
    // over a READ-ONLY WAL conn + reads git2 read-only: NO mutation (the event log is unchanged), NO
    // write-actor. An unpopulated worktree_id → typed NotFound (proj_worktree empty in MVP).
    use nexusops_shared::ipc::IpcErrorCode;

    let wt = "wt_01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let repo_dir = tempfile::tempdir().unwrap();
    init_repo_with_committed_file(repo_dir.path(), "f.txt", "one\ntwo\n");
    std::fs::write(repo_dir.path().join("f.txt"), "one\nTWO\n").unwrap();
    let (_d, db_path) = temp_db_with_worktree(wt, repo_dir.path().to_str().unwrap());

    // event count before (proves the read appends nothing).
    let before = {
        let conn = nexusopsd::eventstore::open_read_only(&db_path).unwrap();
        conn.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap()
    };

    // a populated worktree_id → Ok(structured diff).
    let got: DiffResult = nexusopsd::ipc::read_worktree_diff(&db_path, wt, "f.txt")
        .expect("a populated worktree resolves + reads");
    assert_eq!(got.hunks.len(), 1, "the modified file yields one hunk");

    // read-only: the event log is UNCHANGED (no mutation, no write-actor).
    let after = {
        let conn = nexusopsd::eventstore::open_read_only(&db_path).unwrap();
        conn.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap()
    };
    assert_eq!(before, after, "get_diff is a pure read — no event appended");

    // an unpopulated worktree_id → typed NotFound (proj_worktree empty until P5).
    let err = nexusopsd::ipc::read_worktree_diff(&db_path, "wt_unknown000000000000000000", "f.txt")
        .expect_err("an unknown worktree_id is NotFound");
    assert_eq!(
        err,
        IpcErrorCode::NotFound,
        "an unpopulated worktree_id → typed NotFound (not precondition_stale / internal_error)"
    );
}

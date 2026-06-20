//! D7 (P4.7) — `get_pr_diff` §6.1 read RPC: the remote-PR code-diff (head-vs-base) the Review tab
//! renders. The DETERMINISTIC core is `parse_unified_diff` (a GitHub PR unified-diff STRING — octocrab
//! 0.53.1 `pulls(owner,repo).get_diff(pr)→String`, verified against the vendored source — → the frozen
//! `DiffResult`/`Hunk`/`DiffLine` shapes, reused for ui consistency). The live octocrab fetch + the
//! repo_id→owner/repo resolution + the client-wiring are the seam/handler (Step-2.5 design surfaces).
//!
//! The parser tests pin the design-stable core; the handler tests (`read_pr_diff`: resolution → seamed
//! fetch → parse) drive the `FakeGithubReadClient` over a seeded proj_pull_request/proj_repository (the
//! repo_id+pr_number→owner/repo resolution) + a built Runtime handle (LESSON-46 block_on, plain `#[test]`).

use std::time::Duration;

use nexusops_shared::ipc::{DiffLineKind, DiffResult, IpcErrorCode};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::git::parse_unified_diff;
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrations::classifier::IntegrationOutcomeClass;
use nexusopsd::integrations::github::{FakeGithubReadClient, GithubReadError};
use nexusopsd::ipc::read_pr_diff;

/// A 2-file GitHub PR unified diff (the `pulls().get_diff()` shape): per-file `diff --git`/`---`/`+++`
/// headers + `@@ -old,len +new,len @@` hunks + ` `/`-`/`+` lines.
const PR_DIFF: &str = "diff --git a/src/foo.rs b/src/foo.rs
index 1111111..2222222 100644
--- a/src/foo.rs
+++ b/src/foo.rs
@@ -1,3 +1,4 @@ fn foo()
 ctx line
-old line
+new line
+extra new
 tail ctx
diff --git a/README.md b/README.md
index 3333333..4444444 100644
--- a/README.md
+++ b/README.md
@@ -10,2 +10,2 @@
-old readme
+new readme
";

#[test]
fn test_get_pr_diff_parses_unified_diff() {
    // spec(§6.1/§11.2 / LESSON 33): a GitHub PR unified-diff string → the correct DiffResult — hunk
    // header + old_start/old_lines/new_start/new_lines + the per-line DiffLineKind (Context/Added/Removed),
    // content with the trailing newline retained.
    let diff: DiffResult = parse_unified_diff(PR_DIFF, Some("src/foo.rs"));
    assert_eq!(diff.hunks.len(), 1, "one hunk for src/foo.rs");
    let h = &diff.hunks[0];
    assert_eq!(h.header, "@@ -1,3 +1,4 @@ fn foo()");
    assert_eq!((h.old_start, h.old_lines), (1, 3));
    assert_eq!((h.new_start, h.new_lines), (1, 4));
    let kinds: Vec<DiffLineKind> = h.lines.iter().map(|l| l.kind).collect();
    assert_eq!(
        kinds,
        vec![
            DiffLineKind::Context,
            DiffLineKind::Removed,
            DiffLineKind::Added,
            DiffLineKind::Added,
            DiffLineKind::Context,
        ],
        "the +/-/space markers classify each line"
    );
    assert_eq!(
        h.lines[0].content, "ctx line\n",
        "content strips the marker, retains the trailing newline"
    );
    assert_eq!(h.lines[2].content, "new line\n");
}

#[test]
fn test_get_pr_diff_file_filter() {
    // spec(the GetPrDiffParams.file contract): file=Some(path) returns ONLY that file's hunks; file=None
    // returns the whole changeset (all files' hunks, flattened — DiffResult carries no file grouping).
    let only_readme = parse_unified_diff(PR_DIFF, Some("README.md"));
    assert_eq!(
        only_readme.hunks.len(),
        1,
        "Some(README.md) → only its hunk"
    );
    assert_eq!(
        (
            only_readme.hunks[0].new_start,
            only_readme.hunks[0].new_lines
        ),
        (10, 2)
    );

    let all = parse_unified_diff(PR_DIFF, None);
    assert_eq!(
        all.hunks.len(),
        2,
        "None → both files' hunks (whole changeset)"
    );

    let miss = parse_unified_diff(PR_DIFF, Some("does/not/exist.rs"));
    assert!(
        miss.hunks.is_empty(),
        "an unmatched file → no hunks (empty, not an error)"
    );
}

#[test]
fn test_get_pr_diff_hunk_default_lengths() {
    // spec(unified-diff grammar): a hunk header may OMIT the length (`@@ -L +L @@` == 1 line). The parser
    // defaults the omitted count to 1 (a single-line change), not 0.
    let single = "diff --git a/x b/x
--- a/x
+++ b/x
@@ -5 +5 @@
-a
+b
";
    let d = parse_unified_diff(single, Some("x"));
    assert_eq!(d.hunks.len(), 1);
    assert_eq!(
        (d.hunks[0].old_start, d.hunks[0].old_lines),
        (5, 1),
        "omitted old_lines → 1"
    );
    assert_eq!(
        (d.hunks[0].new_start, d.hunks[0].new_lines),
        (5, 1),
        "omitted new_lines → 1"
    );
}

#[test]
fn test_get_pr_diff_parser_tolerates_garbage() {
    // spec(§42 no-panic): the GitHub diff response is UNTRUSTED → the parser must never panic on
    // garbage / truncated / empty / NUL-laced input (it degrades to whatever it can parse). No assertion
    // on the result beyond "did not panic" + a couple of don't-crash shapes.
    let _ = parse_unified_diff("", None);
    let _ = parse_unified_diff("not a diff at all\n\x00\x01\x02 garbage", None);
    // truncated mid-hunk (a header with no body, no trailing newline).
    let _ = parse_unified_diff("diff --git a/x b/x\n@@ -1,2 +1", None);
    // a `+`/`-` line OUTSIDE any hunk (no preceding `@@`) → ignored, not a crash.
    let _ = parse_unified_diff("+orphan added line\n-orphan removed", None);
    // a hunk header with non-numeric ranges → tolerant (no parse panic).
    let d = parse_unified_diff("diff --git a/x b/x\n@@ -foo +bar @@\n+a\n", Some("x"));
    assert_eq!(
        d.hunks.len(),
        1,
        "a junk-range hunk still parses (degraded counts)"
    );
}

#[test]
fn test_get_pr_diff_crlf_file_filter() {
    // spec(CRLF tolerance): a CRLF-terminated diff still matches the file filter (the `\r` must not leak
    // into the `b/<path>` match). GitHub returns LF, but a stray `\r` must not silently drop all hunks.
    let crlf = "diff --git a/src/x.rs b/src/x.rs\r\n--- a/src/x.rs\r\n+++ b/src/x.rs\r\n@@ -1 +1 @@\r\n-a\r\n+b\r\n";
    let d = parse_unified_diff(crlf, Some("src/x.rs"));
    assert_eq!(d.hunks.len(), 1, "CRLF diff still matches the file filter");
}

// ---- the handler core: resolution (repo_id+pr_number → owner/repo) + the seamed fetch --------------

/// Open a store (runs migrations), then seed a proj_pull_request + proj_repository row so
/// `read_pr_diff` can resolve `repo_id`+`pr_number` → project_id → remote_url → owner/repo. TEST-FIXTURE
/// raw writable conn (the get_diff.rs precedent; production folds these via the projectors).
fn temp_db_with_pr(
    repo_id: &str,
    pr_number: u64,
    remote_url: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("nexusops.db");
    {
        let _store = EventStore::open(
            &db_path,
            Box::new(UlidGen),
            Box::new(FixedClock::new("2026-06-13T00:00:00Z")),
            Box::new(PrefixRedactor),
        )
        .expect("open store");
    }
    let conn = rusqlite::Connection::open(&db_path).expect("writable conn");
    conn.execute(
        "INSERT INTO proj_pull_request
           (pr_id, project_id, repo_id, pr_number, status, updated_at_seq)
         VALUES (?1, 'prj_d7', ?2, ?3, 'open', 1)",
        rusqlite::params![format!("{repo_id}#{pr_number}"), repo_id, pr_number as i64],
    )
    .expect("seed proj_pull_request");
    conn.execute(
        "INSERT INTO proj_repository
           (project_id, is_git, remote_url, detached, is_dirty, scanned_at, updated_at_seq)
         VALUES ('prj_d7', 1, ?1, 0, 0, '2026-06-13T00:00:00Z', 1)",
        rusqlite::params![remote_url],
    )
    .expect("seed proj_repository");
    (dir, db_path)
}

/// A current-thread Runtime for the LESSON-46 captured-Handle + block_on (plain `#[test]`, NEVER
/// `#[tokio::test]` — the handler's block_on must run off an entered runtime).
fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime")
}

#[test]
fn test_get_pr_diff_not_found() {
    // spec(§6.1 / LESSON 33): an unresolvable (repo_id, pr_number) — no proj_pull_request row → no
    // project_id → no owner/repo — returns typed NotFound (the get_diff-unpopulated precedent), BEFORE any
    // network call. The fake (with a valid diff) is never reached.
    let rt = rt();
    let (_d, db_path) = temp_db_with_pr("repo_present", 1, "https://github.com/acme/widget.git");
    let client = FakeGithubReadClient::new(Err(GithubReadError {
        class: IntegrationOutcomeClass::ServerError,
        message: "unused".into(),
    }))
    .with_diff(Ok(
        "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n".into(),
    ));

    // a DIFFERENT repo_id (no matching row) → NotFound.
    let err = read_pr_diff(
        &db_path,
        &client,
        rt.handle(),
        Duration::from_secs(5),
        "repo_absent",
        1,
        None,
    )
    .expect_err("an unresolvable repo_id is NotFound");
    assert_eq!(err, IpcErrorCode::NotFound);

    // the right repo_id but the WRONG pr_number → also NotFound (the EXACT PR row must exist).
    let err2 = read_pr_diff(
        &db_path,
        &client,
        rt.handle(),
        Duration::from_secs(5),
        "repo_present",
        999,
        None,
    )
    .expect_err("a wrong pr_number is NotFound");
    assert_eq!(err2, IpcErrorCode::NotFound);
}

#[test]
fn test_get_pr_diff_network_failure_is_typed() {
    // spec(LESSON 46 / §15): a resolvable PR whose GitHub fetch FAILS (the fake returns Err) → a TYPED
    // error (InternalError), never a panic, never raw API text. Bounded by the mandatory timeout.
    let rt = rt();
    let (_d, db_path) = temp_db_with_pr("repo_present", 7, "https://github.com/acme/widget.git");
    let client = FakeGithubReadClient::new(Err(GithubReadError {
        class: IntegrationOutcomeClass::ServerError,
        message: "boom secret=ghp_LEAK".into(), // the raw message must NOT surface
    }))
    .with_diff(Err(GithubReadError {
        class: IntegrationOutcomeClass::ServerError,
        message: "boom secret=ghp_LEAK".into(),
    }));

    let err = read_pr_diff(
        &db_path,
        &client,
        rt.handle(),
        Duration::from_secs(5),
        "repo_present",
        7,
        Some("any.rs"),
    )
    .expect_err("a GitHub fetch failure is a typed error");
    assert_eq!(
        err,
        IpcErrorCode::InternalError,
        "a network/auth failure → typed InternalError (not a panic, not NotFound)"
    );
}

#[test]
fn test_get_pr_diff_resolves_and_parses() {
    // spec(§6.1/§11.2 end-to-end): a resolvable PR + a successful fetch → the parsed DiffResult. Proves
    // the resolution (repo_id+pr_number → owner/repo) + the seamed fetch + parse compose. file=Some
    // filters to the one file.
    let rt = rt();
    let (_d, db_path) = temp_db_with_pr("repo_present", 7, "https://github.com/acme/widget.git");
    let client = FakeGithubReadClient::new(Err(GithubReadError {
        class: IntegrationOutcomeClass::ServerError,
        message: "unused".into(),
    }))
    .with_diff(Ok(PR_DIFF.into()));

    let diff = read_pr_diff(
        &db_path,
        &client,
        rt.handle(),
        Duration::from_secs(5),
        "repo_present",
        7,
        Some("src/foo.rs"),
    )
    .expect("a resolvable PR + successful fetch → DiffResult");
    assert_eq!(
        diff.hunks.len(),
        1,
        "Some(src/foo.rs) → that file's one hunk"
    );
    assert_eq!(diff.hunks[0].header, "@@ -1,3 +1,4 @@ fn foo()");
}

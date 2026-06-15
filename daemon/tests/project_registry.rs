//! P5.1 (edges-028) — the `proj_project` + `proj_repository` registry projector.
//!
//! Folds the already-emitted `ProjectRescanned` event (edges-019 `project.rescan` executor) into the
//! event-fed project registry read model: a `proj_project` row (the project-IDENTITY axis) + a
//! `proj_repository` row (the git-DETECTION axis), both keyed by `env.project_id` (1:1 MVP). MIGRATION_10
//! lays the two tables. **CONTRACT-neutral** — `ProjectRescanned` is frozen at 0.26; no new `shared/`
//! surface (the IPC read is the deferred follow-on).
//!
//! The projector is SIMPLER than the edges-022 `WorktreeProjector` — `ProjectRescanned` carries its
//! detection fields in the payload + identity (`project_id`) on the envelope, so there is NO LESSON-17
//! sibling-read (no `action_requests` resource_ref lookup; `project.rescan` is
//! `requires_resource_refs=false`). The assertions append a real `ProjectRescanned` envelope through the
//! store and read the persisted rows over a read-only WAL connection (the projections.rs precedent).

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::events::ProjectRescanned;
use nexusops_shared::ids::{ProjectId, WorkspaceId};
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, AppendIntent, EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;

const FIXED_TS: &str = "2026-06-11T00:00:00Z";

// ---- harness helpers -------------------------------------------------------

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(FIXED_TS)),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// A minimal append intent with the identity/edge fields defaulted to absent.
fn intent(payload: &str) -> AppendIntent {
    AppendIntent {
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: FIXED_TS.to_string(),
        workspace_id: WorkspaceId::new(),
        actor_type: ActorType::User,
        actor_id: "u_1".to_string(),
        source_type: SourceType::DesktopUi,
        source_id: "src_1".to_string(),
        correlation_id: "corr_1".to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json: payload.to_string(),
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: None,
        action_request_id: None,
        approval_id: None,
        causation_id: None,
    }
}

/// A sample `ProjectRescanned` payload — a fully-populated detection snapshot.
fn rescan_payload(is_dirty: bool) -> ProjectRescanned {
    ProjectRescanned {
        is_git: true,
        repo_root: Some("/home/dev/acme".to_string()),
        remote_url: Some("https://github.com/acme/acme.git".to_string()),
        branch: Some("main".to_string()),
        detached: false,
        is_dirty,
        workflow_pack: true,
        cc_crew: false,
        plan_file: Some("IMPLEMENTATION_PLAN.md".to_string()),
        brain: true,
        scanned_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// Append a `ProjectRescanned` event carrying `project_id` on the envelope.
fn append_rescan(
    store: &mut EventStore,
    project_id: Option<&ProjectId>,
    payload: &ProjectRescanned,
) {
    let mut i = intent(&serde_json::to_string(payload).unwrap());
    i.event_type = ProjectRescanned::EVENT_TYPE.to_string();
    i.project_id = project_id.cloned();
    store.append(i).unwrap();
}

// ---- row readers -----------------------------------------------------------

#[derive(Debug, PartialEq)]
struct ProjectRow {
    project_id: String,
    workflow_pack: bool,
    cc_crew: bool,
    plan_file: Option<String>,
    brain: bool,
    scanned_at: String,
    updated_at_seq: i64,
}

fn proj_project_rows(path: &std::path::Path) -> Vec<ProjectRow> {
    let conn = open_read_only(path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT project_id, workflow_pack, cc_crew, plan_file, brain, scanned_at, updated_at_seq \
             FROM proj_project ORDER BY project_id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok(ProjectRow {
                project_id: r.get(0)?,
                workflow_pack: r.get(1)?,
                cc_crew: r.get(2)?,
                plan_file: r.get(3)?,
                brain: r.get(4)?,
                scanned_at: r.get(5)?,
                updated_at_seq: r.get(6)?,
            })
        })
        .unwrap()
        .map(|r| r.unwrap());
    rows.collect()
}

#[derive(Debug, PartialEq)]
struct RepositoryRow {
    project_id: String,
    is_git: bool,
    repo_root: Option<String>,
    remote_url: Option<String>,
    branch: Option<String>,
    detached: bool,
    is_dirty: bool,
    scanned_at: String,
    updated_at_seq: i64,
}

fn proj_repository_rows(path: &std::path::Path) -> Vec<RepositoryRow> {
    let conn = open_read_only(path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT project_id, is_git, repo_root, remote_url, branch, detached, is_dirty, \
             scanned_at, updated_at_seq FROM proj_repository ORDER BY project_id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok(RepositoryRow {
                project_id: r.get(0)?,
                is_git: r.get(1)?,
                repo_root: r.get(2)?,
                remote_url: r.get(3)?,
                branch: r.get(4)?,
                detached: r.get(5)?,
                is_dirty: r.get(6)?,
                scanned_at: r.get(7)?,
                updated_at_seq: r.get(8)?,
            })
        })
        .unwrap()
        .map(|r| r.unwrap());
    rows.collect()
}

/// (last_seq, state) for the project_registry projector's offset row.
fn offset(path: &std::path::Path, name: &str) -> (i64, String) {
    open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT last_seq, state FROM projection_offsets WHERE projection_name=?1",
            [name],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
}

// ---- Test 1 — the identity axis folds into proj_project ---------------------

#[test]
fn test_project_rescanned_folds_proj_project() {
    // spec(§7.2): the project-IDENTITY fields fold into proj_project, keyed by the envelope project_id.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let pid = ProjectId::new();
    append_rescan(&mut store, Some(&pid), &rescan_payload(false));

    let rows = proj_project_rows(&path);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.project_id, pid.as_str());
    assert!(r.workflow_pack);
    assert!(!r.cc_crew);
    assert_eq!(r.plan_file.as_deref(), Some("IMPLEMENTATION_PLAN.md"));
    assert!(r.brain);
    assert_eq!(r.scanned_at, FIXED_TS);
    assert!(r.updated_at_seq > 0);
}

// ---- Test 2 — the git-detection axis folds into proj_repository ------------

#[test]
fn test_project_rescanned_folds_proj_repository() {
    // spec(§9 → §7.2): the git-DETECTION fields fold into proj_repository, keyed 1:1 by project_id;
    // remote_url is the (already-stripped-at-source) committed value, written through unchanged.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let pid = ProjectId::new();
    append_rescan(&mut store, Some(&pid), &rescan_payload(true));

    let rows = proj_repository_rows(&path);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.project_id, pid.as_str());
    assert!(r.is_git);
    assert_eq!(r.repo_root.as_deref(), Some("/home/dev/acme"));
    assert_eq!(
        r.remote_url.as_deref(),
        Some("https://github.com/acme/acme.git")
    );
    assert_eq!(r.branch.as_deref(), Some("main"));
    assert!(!r.detached);
    assert!(r.is_dirty);
    assert_eq!(r.scanned_at, FIXED_TS);
    assert!(r.updated_at_seq > 0);
}

// ---- Test 2b — a non-git / no-remote rescan folds Optionals as NULL -------

#[test]
fn test_non_git_project_folds_nulls() {
    // spec(§7.2): the most common real input is a non-git dir — None→NULL is a distinct binding path
    // from Some→value. A ProjectRescanned with is_git=false + all Optionals None folds both rows with
    // NULL Optionals + is_git=false, no rejection.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let pid = ProjectId::new();
    let payload = ProjectRescanned {
        is_git: false,
        repo_root: None,
        remote_url: None,
        branch: None,
        detached: false,
        is_dirty: false,
        workflow_pack: false,
        cc_crew: false,
        plan_file: None,
        brain: false,
        scanned_at: Timestamp::parse(FIXED_TS).unwrap(),
    };
    append_rescan(&mut store, Some(&pid), &payload);

    let proj = proj_project_rows(&path);
    assert_eq!(proj.len(), 1);
    assert_eq!(proj[0].plan_file, None, "plan_file None → NULL");
    assert!(!proj[0].workflow_pack);
    assert_eq!(proj[0].scanned_at, FIXED_TS);

    let repo = proj_repository_rows(&path);
    assert_eq!(repo.len(), 1);
    let r = &repo[0];
    assert!(!r.is_git, "is_git=false folds, no rejection");
    assert_eq!(r.repo_root, None, "repo_root None → NULL");
    assert_eq!(r.remote_url, None, "remote_url None → NULL");
    assert_eq!(r.branch, None, "branch None → NULL");
    assert!(!r.detached);
    assert!(!r.is_dirty);
}

// ---- Test 3 — a re-rescan UPSERTs both rows + advances the seq -------------

#[test]
fn test_rescan_upserts_and_advances_seq() {
    // spec(lead-ruled coarse-atomic rescan): a 2nd ProjectRescanned for the same project_id REPLACES
    // the detection snapshot (ON CONFLICT DO UPDATE) — one row per table, new values, higher seq.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let pid = ProjectId::new();
    append_rescan(&mut store, Some(&pid), &rescan_payload(false));
    let seq1 = proj_repository_rows(&path)[0].updated_at_seq;

    // second rescan flips is_dirty
    append_rescan(&mut store, Some(&pid), &rescan_payload(true));

    let proj = proj_project_rows(&path);
    let repo = proj_repository_rows(&path);
    assert_eq!(proj.len(), 1, "no dup proj_project row");
    assert_eq!(repo.len(), 1, "no dup proj_repository row");
    assert!(repo[0].is_dirty, "detection field replaced");
    assert!(repo[0].updated_at_seq > seq1, "seq advanced");
    assert_eq!(
        proj[0].updated_at_seq, repo[0].updated_at_seq,
        "both rows advance to the same new seq"
    );
}

// ---- Test 4 — a missing project_id is a healthy skip ----------------------

#[test]
fn test_missing_project_id_is_healthy_skip() {
    // spec(healthy skip): a ProjectRescanned envelope with project_id=None → 0 rows in both tables,
    // the append/fold SUCCEEDS (proj_project.project_id is NOT NULL; the session.rs/worktree.rs skip).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_rescan(&mut store, None, &rescan_payload(false));

    assert_eq!(proj_project_rows(&path).len(), 0, "no project_id → skip");
    assert_eq!(proj_repository_rows(&path).len(), 0, "no project_id → skip");
    // healthy (not degraded) AND the offset ADVANCED past the skipped event — a healthy skip is a
    // no-op fold that still advances last_seq (contrast Test 5's degrade, which holds at 0).
    let (last_seq, state) = offset(&path, "project_registry");
    assert_eq!(state, "healthy");
    assert!(last_seq > 0, "offset advanced past the healthy-skip");
    assert_eq!(store.read_all().unwrap().len(), 1, "raw event intact");
}

// ---- Test 5 — an unbindable payload degrades (no payload echo, §15) --------

#[test]
fn test_unbindable_payload_degrades() {
    // spec(§15 / reject-unknown): an envelope typed ProjectRescanned but whose payload won't bind
    // (missing required fields) → the projector Decode-degrades + skips (no row); the generic reason
    // never echoes payload bytes (the map_err discards the serde error by construction).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let mut i = intent("{}"); // empty object: every required ProjectRescanned field is missing
    i.event_type = ProjectRescanned::EVENT_TYPE.to_string();
    i.project_id = Some(ProjectId::new());
    store.append(i).unwrap();

    assert_eq!(
        proj_project_rows(&path).len(),
        0,
        "no row from an unbinding payload"
    );
    assert_eq!(proj_repository_rows(&path).len(), 0);
    assert_eq!(
        offset(&path, "project_registry"),
        (0, "degraded".to_string()),
        "degraded, offset not advanced (savepoint rolled row+offset back together)"
    );
    assert_eq!(
        store.read_all().unwrap().len(),
        1,
        "raw event intact (§7.2)"
    );
}

// ---- Test 6 — rebuild-equivalence for BOTH tables -------------------------

#[test]
fn test_rebuild_equivalence() {
    // spec(LESSON 4/17): the incremental in-band fold == a full rebuild() of the same log, for BOTH
    // proj_project and proj_repository (both MUST be in REBUILD_TABLES; the fold is event-derived).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let pid_a = ProjectId::new();
    let pid_b = ProjectId::new();
    append_rescan(&mut store, Some(&pid_a), &rescan_payload(false));
    append_rescan(&mut store, Some(&pid_b), &rescan_payload(true));
    append_rescan(&mut store, Some(&pid_a), &rescan_payload(true)); // re-rescan a

    let proj_before = proj_project_rows(&path);
    let repo_before = proj_repository_rows(&path);
    assert_eq!(proj_before.len(), 2);
    assert_eq!(repo_before.len(), 2);

    store.rebuild_projections().unwrap();

    assert_eq!(
        proj_before,
        proj_project_rows(&path),
        "rebuild reproduces proj_project byte-identically"
    );
    assert_eq!(
        repo_before,
        proj_repository_rows(&path),
        "rebuild reproduces proj_repository byte-identically"
    );
}

// ---- Test 7 — MIGRATION_10 is wired + applied -----------------------------

#[test]
fn test_migration_10_applies() {
    // spec(LESSON 8 forward-only migration): a fresh store opens at user_version=10 with both new tables.
    let (_d, path) = temp_db();
    let store = open(&path);
    assert_eq!(store.user_version().unwrap(), 10, "fresh DB at v10");

    let conn = open_read_only(&path).unwrap();
    for t in ["proj_project", "proj_repository"] {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [t],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "MIGRATION_10 must create `{t}`");
    }
    assert_eq!(
        nexusopsd::eventstore::SUPPORTED_USER_VERSION,
        10,
        "SUPPORTED_USER_VERSION bumped to 10"
    );
}

// ---- Test 8 — the projector folds ONLY ProjectRescanned -------------------

#[test]
fn test_ignores_other_event_types() {
    // spec(§7.2): the projector folds ONLY ProjectRescanned — a different event writes no registry rows.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let mut i = intent("{\"status\":\"active\"}");
    i.event_type = "SessionStarted".to_string();
    i.project_id = Some(ProjectId::new());
    store.append(i).unwrap();

    assert_eq!(proj_project_rows(&path).len(), 0);
    assert_eq!(proj_repository_rows(&path).len(), 0);
}

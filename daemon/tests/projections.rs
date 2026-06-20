//! Phase 1.2 — projection engine (RED first). ARCHITECTURE §7 (in-band fan-out:
//! "a single event may update multiple projections within the one event-commit
//! transaction"), §7.2 (rebuildable `proj_*`, tracked by `projection_offsets`),
//! DATA_MODEL §2.2 (object_refs) / §2.3 (the 10 proj_* tables) / §2.4 (offsets
//! advance in the SAME txn) / §2.11 (FTS over the redaction-safe audit projection).
//!
//! Layered to mirror the 3-commit slice:
//!   L1 (tests 1–3)  — migration 3 schema + AppendIntent identity fields.
//!   L2 (tests 4–9, 14) — in-band apply + the feedable projectors.   [added at L2]
//!   L3 (tests 10–13) — catch-up replay / rebuild / degraded-skip.    [added at L3]

use std::sync::Arc;

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::actor::ActorType;
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::event_envelope::{RedactionStatus, Sensitivity, SourceType, Visibility};
use nexusops_shared::events::{
    PullRequestMerged, PullRequestSynced, ReviewSynced, SessionRecovered, WorktreeCreated,
};
use nexusops_shared::harness::ResumeMode;
use nexusops_shared::ids::{ActionRequestId, ProjectId, SessionId, WorkspaceId, WorktreeId};
use nexusops_shared::status::{ActionRequestStatus, PullRequest, ReviewState, Session};
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    AppendIntent, EventStore, EventStoreError, PrefixRedactor, RedactionOutcome, Redactor,
};
use nexusopsd::gateway::executor::CatalogExecutor;
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::git::cli::FakeGitCli;
use nexusopsd::git::executor::GitExecutor;

// ---- fixtures ---------------------------------------------------------------

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// A minimal append intent with the 1.2 identity/edge fields defaulted to absent.
fn intent(payload: &str) -> AppendIntent {
    AppendIntent {
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: "2026-06-08T00:00:00Z".to_string(),
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
        // --- 1.2 identity/edge fields (the AppendIntent extension) ---
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: None,
        action_request_id: None,
        approval_id: None,
        causation_id: None,
    }
}

/// a SessionStarted intent carrying identity + a type-specific payload.
fn session_intent(session_id: &SessionId, project_id: &ProjectId, payload: &str) -> AppendIntent {
    let mut i = intent(payload);
    i.session_id = Some(session_id.clone());
    i.project_id = Some(project_id.clone());
    i
}

/// count rows in `table` via a read-only connection.
fn count(path: &std::path::Path, sql: &str) -> i64 {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

/// (last_seq, state) for a projector's offset row.
fn offset(path: &std::path::Path, name: &str) -> (i64, String) {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT last_seq, state FROM projection_offsets WHERE projection_name=?1",
            [name],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
}

/// test Redactor that refuses to redact — exercises the §15 fail-closed gate.
struct NeverRedacts;
impl Redactor for NeverRedacts {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        RedactionOutcome {
            status: RedactionStatus::Unredacted,
            payload_json: payload_json.to_string(),
            engine_version: "never".to_string(),
            quarantine: None,
        }
    }
}

/// the set of table names present in the db
fn table_names(path: &std::path::Path) -> std::collections::BTreeSet<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','virtual') OR type='table'")
        .unwrap();
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap());
    rows.collect()
}

// ---- Test 1 — migration 3 creates every projection table (§2.2/§2.3/§2.4) ----

#[test]
fn test_migration_3_creates_projection_tables() {
    let (_d, path) = temp_db();
    let store = open(&path);
    // projections were introduced at migration 3; later migrations raise the version
    // further (the exact-version pin lives in each migration's own test).
    assert!(
        store.user_version().unwrap() >= 3,
        "open migrates at/above the projections migration (3)"
    );

    let tables = table_names(&path);
    // object_refs (§2.2) + projection_offsets (§2.4) + the 10 MVP projections (§2.3;
    // ProjectGraph is two physical tables: node + edge).
    for t in [
        "object_refs",
        "projection_offsets",
        "proj_project_activity",
        "proj_session",
        "proj_approval_queue",
        "proj_worktree",
        "proj_pull_request",
        "proj_plan_progress",
        "proj_graph_node",
        "proj_graph_edge",
        "proj_audit_trail",
        "proj_agent_team",
        "proj_usage_ledger",
    ] {
        assert!(tables.contains(t), "migration 3 must create `{t}`");
    }
    // the 1.1 events spine is untouched
    assert!(tables.contains("events"), "events table preserved");
}

// ---- Test 2 — migration 3 over a 1.1-era (v2) db backs up first (§16) --------

#[test]
fn test_migration_3_over_existing_events_backs_up() {
    let (_d, path) = temp_db();
    // simulate a real 1.1 (user_version 2) db that already holds an event: apply the
    // frozen v1+v2 migration DDL directly, insert one row, stamp user_version=2.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(nexusopsd::eventstore::MIGRATION_1_EVENTS)
            .unwrap();
        conn.execute_batch(nexusopsd::eventstore::MIGRATION_2_REDACTION)
            .unwrap();
        // valid minted IDs so the startup catch-up replay can reconstruct + fold it
        conn.execute(
            "INSERT INTO events (event_id, seq, event_type, event_version, occurred_at, \
             recorded_at, workspace_id, actor_type, actor_id, source_type, source_id, \
             correlation_id, sensitivity, payload_json, schema_version, redaction_status) \
             VALUES ('evt_01ARZ3NDEKTSV4RRFFQ69G5FAV',1,'SessionStarted',1,\
             '2026-06-07T00:00:00Z','2026-06-07T00:00:00Z','ws_01ARZ3NDEKTSV4RRFFQ69G5FAV',\
             'user','u','desktop_ui','s','c','internal','{}','event-envelope-v1','redacted')",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
    }
    // opening raises 2→3→… on a NON-EMPTY db → backup `.bak-2` first (§16, one
    // pre-run snapshot keyed by the starting version, restored if any step fails)
    let store = open(&path);
    assert!(
        store.user_version().unwrap() >= 3,
        "migrated through 3 (and any later migrations)"
    );
    let bak = std::path::PathBuf::from(format!("{}.bak-2", path.display()));
    assert!(
        bak.exists(),
        "auto-backup wrote .bak-2 before the 2→4 raise"
    );
    // the irreplaceable raw event survived the migration
    let n: i64 = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1, "raw events intact across the migration");
}

// ---- Test 3 — AppendIntent identity fields round-trip to the events row -------

#[test]
fn test_append_intent_persists_identity_fields() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let project_id = ProjectId::new();
    let session_id = SessionId::new();
    let mut i = intent("{}");
    i.project_id = Some(project_id.clone());
    i.session_id = Some(session_id.clone());
    i.visibility = Some(Visibility::System);
    store.append(i).unwrap();

    let e = &store.read_all().unwrap()[0];
    assert_eq!(
        e.project_id.as_ref(),
        Some(&project_id),
        "project_id persisted"
    );
    assert_eq!(
        e.session_id.as_ref(),
        Some(&session_id),
        "session_id persisted"
    );
    assert_eq!(
        e.visibility,
        Some(Visibility::System),
        "visibility persisted (not the DDL default 'project')"
    );
}

// ======================= L2 — in-band apply (tests 4–9, 14) ==================

// ---- Test 4 — append folds the session projection in-txn (§7) ---------------

#[test]
fn test_append_folds_session_projection_in_txn() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"starting\"}"))
        .unwrap();

    // one proj_session row, status bound to the §5.1 Session enum wire value
    let status: String = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row(
            "SELECT status FROM proj_session WHERE session_id=?1",
            [sid.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "starting", "status ∈ Session enum");
    // offset advanced to this event's seq, atomically (§2.4)
    assert_eq!(offset(&path, "session"), (1, "healthy".to_string()));
}

// ---- Test 5 — object_refs derived from typed identity fields (§2.2) ----------

#[test]
fn test_append_derives_object_refs() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"starting\"}"))
        .unwrap();

    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM object_refs"),
        2,
        "session + project refs"
    );
    let kinds: std::collections::BTreeSet<String> = {
        let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
        let mut stmt = conn.prepare("SELECT object_type FROM object_refs").unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    assert!(kinds.contains("session") && kinds.contains("project"));
}

// ---- Test 6 — a projector failure never leaves the offset ahead (§2.4/§7.2) --

#[test]
fn test_offset_never_ahead_of_rows() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    // an unbindable status fails the session projector's §5.1 enum bind → degrade.
    store
        .append(session_intent(
            &sid,
            &pid,
            "{\"status\":\"not_a_real_status\"}",
        ))
        .unwrap();

    // the failing projector wrote NO row and its offset did NOT advance (savepoint
    // rolled rows + offset back together) — never ahead of applied rows.
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_session"),
        0,
        "no row"
    );
    assert_eq!(
        offset(&path, "session"),
        (0, "degraded".to_string()),
        "degraded, not ahead"
    );
    // the raw event still persisted (a projector must never corrupt the spine §7.2)
    assert_eq!(store.read_all().unwrap().len(), 1, "event intact");
    // a sibling projector (object_refs, status-agnostic) was unaffected — isolation
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM object_refs"),
        2,
        "siblings applied"
    );
}

// ---- Test 7 — §15 redaction gate still fail-closed with apply wired ----------

#[test]
fn test_redaction_gate_still_fail_closed() {
    let (_d, path) = temp_db();
    let mut store = EventStore::open(
        &path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(NeverRedacts),
    )
    .unwrap();
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let res = store.append(session_intent(&sid, &pid, "{\"status\":\"starting\"}"));
    assert!(
        matches!(res, Err(EventStoreError::RedactionRequired)),
        "the §15 gate refuses before any projector runs"
    );
    // nothing persisted: not the event, not any projection
    assert_eq!(count(&path, "SELECT COUNT(*) FROM events"), 0);
    assert_eq!(count(&path, "SELECT COUNT(*) FROM proj_session"), 0);
    assert_eq!(count(&path, "SELECT COUNT(*) FROM object_refs"), 0);
}

// ---- Test 8 — audit projection + FTS over the redaction-safe headline (§2.11) -

#[test]
fn test_audit_projection_populates_fts() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"starting\"}"))
        .unwrap();

    // a rendered audit row exists
    assert_eq!(count(&path, "SELECT COUNT(*) FROM proj_audit_trail"), 1);
    // and is searchable by its headline text via FTS (not the raw payload)
    let hits = count(
        &path,
        "SELECT COUNT(*) FROM fts_events WHERE fts_events MATCH 'started'",
    );
    assert_eq!(hits, 1, "headline indexed for the audit search box");
}

// ---- Test 9 — one SessionStarted fans out, one txn (demo step 7) -------------

#[test]
fn test_session_started_fans_out() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"starting\"}"))
        .unwrap();

    // a single append populated proj_session AND the graph AND object_refs together
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_session"),
        1,
        "session"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_graph_node"),
        2,
        "session + project nodes"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_graph_edge"),
        1,
        "project owns session"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM object_refs"),
        2,
        "normalized edges"
    );
}

// ---- Test 14 — ProjectActivity counter rollup (increment-only until Phase 3) -

#[test]
fn test_session_started_increments_activity_counter() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let pid = ProjectId::new();
    store
        .append(session_intent(
            &SessionId::new(),
            &pid,
            "{\"status\":\"starting\"}",
        ))
        .unwrap();
    store
        .append(session_intent(
            &SessionId::new(),
            &pid,
            "{\"status\":\"starting\"}",
        ))
        .unwrap();

    let active: i64 = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row(
            "SELECT active_sessions FROM proj_project_activity WHERE project_id=?1",
            [pid.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(active, 2, "two SessionStarted → active_sessions counts 2");
}

// ======================= L3 — recovery (tests 10–13, 15) ====================

/// a stable fingerprint of the whole projection surface (for rebuild-equivalence).
fn projection_fingerprint(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).unwrap();
    let sessions: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT session_id, status FROM proj_session ORDER BY session_id")
            .unwrap();
        stmt.query_map([], |r| {
            Ok(format!(
                "{}={}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
    };
    let one = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
    format!(
        "sessions=[{}] refs={} nodes={} edges={} audit={} activity={}",
        sessions.join(","),
        one("SELECT COUNT(*) FROM object_refs"),
        one("SELECT COUNT(*) FROM proj_graph_node"),
        one("SELECT COUNT(*) FROM proj_graph_edge"),
        one("SELECT COUNT(*) FROM proj_audit_trail"),
        one("SELECT COALESCE(SUM(active_sessions),0) FROM proj_project_activity"),
    )
}

// ---- Test 10 — startup catch-up replays pending events (§7.2) ----------------

#[test]
fn test_startup_catch_up_replays_pending() {
    let (_d, path) = temp_db();
    let pid = ProjectId::new();
    {
        let mut store = open(&path);
        store
            .append(session_intent(
                &SessionId::new(),
                &pid,
                "{\"status\":\"starting\"}",
            ))
            .unwrap();
        store
            .append(session_intent(
                &SessionId::new(),
                &pid,
                "{\"status\":\"starting\"}",
            ))
            .unwrap();
    }
    // simulate a projector that lagged behind the log (crash mid-write): rewind the
    // session offset + drop its rows, leaving the raw events ahead.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE projection_offsets SET last_seq=0, state='healthy' WHERE projection_name='session'", []).unwrap();
        conn.execute("DELETE FROM proj_session", []).unwrap();
    }
    // reopening runs catch_up_replay → folds events WHERE seq > last_seq
    let _store = open(&path);
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_session"),
        2,
        "catch-up re-folded the 2 pending events"
    );
    assert_eq!(offset(&path, "session"), (2, "healthy".to_string()));
}

// ---- Test 11 — rebuild == incremental fold (§7.2 fully-rebuildable) ----------

#[test]
fn test_rebuild_equivalence() {
    let (_d, path) = temp_db();
    let pid = ProjectId::new();
    let mut store = open(&path);
    for _ in 0..3 {
        store
            .append(session_intent(
                &SessionId::new(),
                &pid,
                "{\"status\":\"active\"}",
            ))
            .unwrap();
    }
    let incremental = projection_fingerprint(&path);
    store.rebuild_projections().unwrap();
    let rebuilt = projection_fingerprint(&path);
    assert_eq!(
        incremental, rebuilt,
        "full rebuild reproduces the incremental-fold state"
    );
}

// ---- Test 12 — rebuild truncates proj_* but never the raw events (§7.2) ------

#[test]
fn test_rebuild_preserves_raw_events() {
    let (_d, path) = temp_db();
    let pid = ProjectId::new();
    let mut store = open(&path);
    for _ in 0..3 {
        store
            .append(session_intent(
                &SessionId::new(),
                &pid,
                "{\"status\":\"active\"}",
            ))
            .unwrap();
    }
    let before = store.read_all().unwrap();
    store.rebuild_projections().unwrap();
    let after = store.read_all().unwrap();
    assert_eq!(
        before, after,
        "rebuild leaves the raw events byte-identical"
    );
    assert_eq!(after.len(), 3);
}

// ---- Test 13 — unknown event_version degrades, skips, continues (§7.2/§17) ---

#[test]
fn test_unknown_event_version_degrades_skips() {
    let (_d, path) = temp_db();
    let pid = ProjectId::new();
    let sid_b = SessionId::new();
    let mut store = open(&path);
    store
        .append(session_intent(
            &SessionId::new(),
            &pid,
            "{\"status\":\"starting\"}",
        ))
        .unwrap(); // seq 1
    store
        .append(session_intent(&sid_b, &pid, "{\"status\":\"active\"}"))
        .unwrap(); // seq 2
                   // make seq 1 unfoldable by this binary (a future event_version)
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE events SET event_version = 9999 WHERE seq = 1", [])
            .unwrap();
    }
    store.rebuild_projections().unwrap();

    // seq 1 skipped (degraded), seq 2 still folded (skip + continue), events intact
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_session"),
        1,
        "only the foldable event projected"
    );
    let only: String = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row("SELECT session_id FROM proj_session", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        only,
        sid_b.as_str(),
        "the survivor is seq 2, not the degraded seq 1"
    );
    // the offset advanced PAST the degraded seq 1 to the last folded seq 2 — a
    // degraded event never strands last_seq, so a reopen re-folds nothing (no
    // double-count); a future binary that understands the version re-attempts seq 1.
    assert_eq!(
        offset(&path, "session"),
        (2, "degraded".to_string()),
        "advanced past the skip, sticky-degraded"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM events"),
        2,
        "raw events never mutated"
    );
}

// ---- Test 15 — catch-up is a strict no-op when offsets are current (§7.2) ----

#[test]
fn test_catch_up_replay_noop_when_offsets_current() {
    let (_d, path) = temp_db();
    let pid = ProjectId::new();
    {
        let mut store = open(&path);
        store
            .append(session_intent(
                &SessionId::new(),
                &pid,
                "{\"status\":\"starting\"}",
            ))
            .unwrap();
        store
            .append(session_intent(
                &SessionId::new(),
                &pid,
                "{\"status\":\"starting\"}",
            ))
            .unwrap();
    }
    let before = projection_fingerprint(&path);
    // reopen → catch_up_replay runs with offsets already current → strict `seq > last_seq`
    // means it folds NOTHING: the increment-only activity counter is NOT doubled and
    // object_refs hits no PK conflict (idempotent replay on restart).
    let _store = open(&path);
    let after = projection_fingerprint(&path);
    assert_eq!(
        before, after,
        "catch-up on a current log is a no-op (no double-count)"
    );
    // explicit: activity stayed at 2, not 4
    assert!(
        after.contains("activity=2"),
        "counter not re-incremented: {after}"
    );
}

// ======================= P3.1 — UsageLedger projector (tests 17–23) ==================
//
// The `proj_usage_ledger` projector folds `TelemetrySampled` events into per-day usage
// rollups (§7/§7.2/§18). Driven here through the REAL append path (registered in
// `projectors()` → runs in the in-band fan-out), so these also prove reachability.
// rollup key `ledger_id` = (project, session, profile, model, bucket_day);
// tokens/cost SUM; context_pct_max = MAX; metric_quality = worst-wins.

use nexusops_shared::events::TelemetrySampled;
use nexusops_shared::harness::{MetricQuality, TelemetrySample};

/// a TelemetrySampled intent: identity (session/project/occurred_at) on the envelope; the rollup
/// dims the envelope lacks (model/execution_profile_id) + the sample in the payload. source = UsageMeter.
fn telemetry_intent(
    session_id: &SessionId,
    project_id: &ProjectId,
    occurred_at: &str,
    payload: &TelemetrySampled,
) -> AppendIntent {
    let mut i = intent(&serde_json::to_string(payload).unwrap());
    i.event_type = "TelemetrySampled".to_string();
    i.occurred_at = occurred_at.to_string();
    i.session_id = Some(session_id.clone());
    i.project_id = Some(project_id.clone());
    i.source_type = SourceType::UsageMeter;
    i
}

fn sampled(
    tokens_in: u64,
    tokens_out: u64,
    context_pct: Option<f32>,
    cost_estimate: f64,
    metric_quality: MetricQuality,
    model: &str,
    profile: &str,
) -> TelemetrySampled {
    TelemetrySampled {
        sample: TelemetrySample {
            tokens_in,
            tokens_out,
            context_pct,
            cost_estimate,
            metric_quality,
        },
        model: Some(model.to_string()),
        execution_profile_id: Some(profile.to_string()),
    }
}

/// the one usage row for a session (panics if != 1 row) — (tokens_in, tokens_out, ctx_max, cost, quality, bucket_day).
fn usage_row(
    path: &std::path::Path,
    session_id: &str,
) -> (i64, i64, Option<f64>, f64, String, String) {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT tokens_in, tokens_out, context_pct_max, cost_estimate, metric_quality, bucket_day \
             FROM proj_usage_ledger WHERE session_id=?1",
            [session_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .unwrap()
}

/// a stable fingerprint of the whole usage-ledger surface (for rebuild-equivalence).
fn usage_fingerprint(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT ledger_id, tokens_in, tokens_out, context_pct_max, cost_estimate, metric_quality \
             FROM proj_usage_ledger ORDER BY ledger_id",
        )
        .unwrap();
    let rows: Vec<String> = stmt
        .query_map([], |r| {
            Ok(format!(
                "{}|in={}|out={}|ctx={:?}|cost={}|q={}",
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<f64>>(3)?,
                r.get::<_, f64>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    rows.join(",")
}

// ---- Test 17 — one TelemetrySampled folds one usage row with the right dims+values (§7/§18) ----

#[test]
fn test_usage_projector_folds_single_sample() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let s = sampled(
        1200,
        340,
        Some(42.5),
        0.5,
        MetricQuality::Exact,
        "claude-opus-4-8",
        "prof_01ARZ3NDEKTSV4RRFFQ69G5FAV",
    );
    store
        .append(telemetry_intent(&sid, &pid, "2026-06-08T12:00:00Z", &s))
        .unwrap();

    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_usage_ledger"),
        1,
        "one sample → one rollup row"
    );
    let (ti, to, ctx, cost, q, day) = usage_row(&path, sid.as_str());
    assert_eq!((ti, to), (1200, 340), "tokens recorded");
    assert_eq!(ctx, Some(42.5), "context_pct_max gauge");
    assert_eq!(cost, 0.5, "cost recorded");
    assert_eq!(q, "exact", "metric_quality wire string");
    assert_eq!(
        day, "2026-06-08",
        "bucket_day = the UTC date of occurred_at"
    );
    // wired + reachable: the projector advanced its offset in the same txn (§2.4)
    assert_eq!(offset(&path, "usage_ledger"), (1, "healthy".to_string()));
}

// ---- Test 18 — same (project,session,profile,model,day) accumulates; ctx=MAX; quality=worst ----

#[test]
fn test_usage_projector_accumulates_tokens_and_cost() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let profile = "prof_01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let a = sampled(
        100,
        50,
        Some(30.0),
        0.5,
        MetricQuality::Exact,
        "claude-opus-4-8",
        profile,
    );
    let b = sampled(
        200,
        70,
        Some(45.0),
        0.25,
        MetricQuality::Estimated,
        "claude-opus-4-8",
        profile,
    );
    store
        .append(telemetry_intent(&sid, &pid, "2026-06-08T08:00:00Z", &a))
        .unwrap();
    store
        .append(telemetry_intent(&sid, &pid, "2026-06-08T09:00:00Z", &b))
        .unwrap();

    // same rollup key → ONE row that accumulates
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_usage_ledger"),
        1,
        "same key → one accumulating row"
    );
    let (ti, to, ctx, cost, q, _day) = usage_row(&path, sid.as_str());
    assert_eq!((ti, to), (300, 120), "tokens_in/out SUM across the bucket");
    assert_eq!(cost, 0.75, "cost_estimate SUMs");
    assert_eq!(
        ctx,
        Some(45.0),
        "context_pct_max takes the MAX (a gauge, not a sum)"
    );
    assert_eq!(
        q, "estimated",
        "metric_quality is worst-wins (any estimated → estimated; §11.7)"
    );
}

// ---- Test 18b — context_pct_max MAX is NULL-safe across None/Some orderings (the hot CASE) ----

#[test]
fn test_usage_projector_context_pct_max_null_orderings() {
    // the `context_pct_max` upsert CASE must survive every None/Some ordering in one bucket —
    // SQLite `MAX(a, b)` returns NULL if EITHER arg is NULL, so the CASE guards the NULL arms.
    // Sequence: None (INSERT stores NULL) → Some(40) (None→Some: takes the new) → None (Some→None:
    // KEEPS the existing max, must NOT be wiped to NULL).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let profile = "prof_01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let none_sample = sampled(
        10,
        5,
        None,
        0.0,
        MetricQuality::Unavailable,
        "claude-opus-4-8",
        profile,
    );
    let some_sample = sampled(
        10,
        5,
        Some(40.0),
        0.0,
        MetricQuality::Exact,
        "claude-opus-4-8",
        profile,
    );
    for (occurred, s) in [
        ("2026-06-08T08:00:00Z", &none_sample), // INSERT with NULL context
        ("2026-06-08T09:00:00Z", &some_sample), // None→Some: stores 40
        ("2026-06-08T10:00:00Z", &none_sample), // Some→None: keeps 40 (not wiped to NULL)
    ] {
        store
            .append(telemetry_intent(&sid, &pid, occurred, s))
            .unwrap();
    }

    let (_ti, _to, ctx, _cost, _q, _day) = usage_row(&path, sid.as_str());
    assert_eq!(
        ctx,
        Some(40.0),
        "context_pct_max = MAX over present gauges; a later None sample never wipes it to NULL"
    );
}

// ---- Test 19 — different occurred_at UTC dates bucket into distinct rows ----

#[test]
fn test_usage_projector_buckets_by_day() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let profile = "prof_01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let s = sampled(
        10,
        10,
        Some(5.0),
        0.5,
        MetricQuality::Exact,
        "claude-opus-4-8",
        profile,
    );
    store
        .append(telemetry_intent(&sid, &pid, "2026-06-08T23:00:00Z", &s))
        .unwrap();
    store
        .append(telemetry_intent(&sid, &pid, "2026-06-09T01:00:00Z", &s))
        .unwrap();

    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_usage_ledger"),
        2,
        "two UTC dates → two day-buckets"
    );
    let days: std::collections::BTreeSet<String> = {
        let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
        let mut stmt = conn
            .prepare("SELECT bucket_day FROM proj_usage_ledger")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    assert!(days.contains("2026-06-08") && days.contains("2026-06-09"));
}

// ---- Test 20 — same session, different model → distinct ledger rows ----

#[test]
fn test_usage_projector_distinct_model_distinct_row() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let profile = "prof_01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let claude = sampled(
        10,
        5,
        Some(5.0),
        0.5,
        MetricQuality::Exact,
        "claude-opus-4-8",
        profile,
    );
    let codex = sampled(
        20,
        8,
        None,
        0.25,
        MetricQuality::Estimated,
        "gpt-5.5-codex",
        profile,
    );
    store
        .append(telemetry_intent(
            &sid,
            &pid,
            "2026-06-08T08:00:00Z",
            &claude,
        ))
        .unwrap();
    store
        .append(telemetry_intent(&sid, &pid, "2026-06-08T09:00:00Z", &codex))
        .unwrap();

    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_usage_ledger"),
        2,
        "distinct model → distinct ledger_id row (model is a rollup dim)"
    );
}

// ---- Test 21 — rebuild reproduces identical rollups (§7.2 rebuild-equivalence) ----

#[test]
fn test_usage_projector_rebuild_idempotent() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    let profile = "prof_01ARZ3NDEKTSV4RRFFQ69G5FAV";
    // two into one bucket (accumulate) + one into another (distinct model)
    store
        .append(telemetry_intent(
            &sid,
            &pid,
            "2026-06-08T08:00:00Z",
            &sampled(
                100,
                40,
                Some(20.0),
                0.5,
                MetricQuality::Exact,
                "claude-opus-4-8",
                profile,
            ),
        ))
        .unwrap();
    store
        .append(telemetry_intent(
            &sid,
            &pid,
            "2026-06-08T10:00:00Z",
            &sampled(
                150,
                60,
                Some(35.0),
                0.25,
                MetricQuality::Estimated,
                "claude-opus-4-8",
                profile,
            ),
        ))
        .unwrap();
    store
        .append(telemetry_intent(
            &sid,
            &pid,
            "2026-06-08T11:00:00Z",
            &sampled(
                20,
                8,
                None,
                0.125,
                MetricQuality::Unavailable,
                "gpt-5.5-codex",
                profile,
            ),
        ))
        .unwrap();

    let incremental = usage_fingerprint(&path);
    store.rebuild_projections().unwrap();
    let rebuilt = usage_fingerprint(&path);
    assert_eq!(
        incremental, rebuilt,
        "a full rebuild reproduces the incremental SUM/MAX/worst-wins rollups (each event folded once)"
    );
    // sanity: the accumulating bucket really did SUM (not a no-op fingerprint)
    assert!(
        incremental.contains("in=250|out=100"),
        "claude bucket SUMmed: {incremental}"
    );
}

// ---- Test 22 — a non-TelemetrySampled event is a healthy no-op (no row, no degrade) ----

#[test]
fn test_usage_projector_ignores_other_event_types() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent(
            &SessionId::new(),
            &ProjectId::new(),
            "{\"status\":\"starting\"}",
        ))
        .unwrap();

    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_usage_ledger"),
        0,
        "SessionStarted writes no usage row"
    );
    // healthy no-op: the projector advanced past the event WITHOUT degrading (the session.rs precedent)
    assert_eq!(
        offset(&path, "usage_ledger"),
        (1, "healthy".to_string()),
        "advanced, healthy (a foreign event is a no-op, not a degrade)"
    );
}

// ---- Test 23 — a malformed TelemetrySampled payload degrades-skips (Decode); no raw bytes leaked ----

#[test]
fn test_usage_projector_rejects_unbinding_payload() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    // a TelemetrySampled event whose payload does NOT bind (missing `sample`, unknown keys) — the
    // §15 reject-unknown / Decode path. The payload still passes the (structure-agnostic) redactor.
    let mut i = intent("{\"not_a_sample\":true}");
    i.event_type = "TelemetrySampled".to_string();
    i.occurred_at = "2026-06-08T12:00:00Z".to_string();
    i.session_id = Some(sid.clone());
    i.project_id = Some(pid.clone());
    i.source_type = SourceType::UsageMeter;
    store.append(i).unwrap();

    // the usage projector Decode-failed → degraded + skipped (no row); the append still succeeded
    // and the raw event persisted (a projector never corrupts the spine, §7.2).
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_usage_ledger"),
        0,
        "no row from an unbinding payload"
    );
    assert_eq!(
        offset(&path, "usage_ledger"),
        (0, "degraded".to_string()),
        "degraded, offset not advanced (savepoint rolled row+offset back together)"
    );
    assert_eq!(store.read_all().unwrap().len(), 1, "raw event intact");
}

// ---- Test 24 (brief 044) — the adapter's DELTAS, SUMmed by the REAL projector, == the cumulative;
//      bucket_day == the UTC date. Proves delta-not-cumulative AND UTC-Z bucketing end-to-end. ----

#[test]
fn test_usage_ledger_sums_deltas_utc_bucketed() {
    // spec(§18) + LESSON §5 — append the two DELTA TelemetrySampled events the adapter's pure
    // `telemetry_sample` produces (from cumulative readings 100/20/0.01 then 250/60/0.03) through the
    // REAL write-actor → the proj_usage_ledger row SUMs to the cumulative total (250/60/0.03) and
    // buckets by the UTC-Z occurred_at. If the adapter emitted cumulative (not deltas), the SUM would
    // double-count to 350/80. If context_pct were deltaed (not a gauge), the MAX would be 30 not 55.
    use nexusopsd::harness::claude::telemetry::{telemetry_sample, UsageReading};

    let ur = |tokens_in, tokens_out, context_pct, cost| UsageReading {
        tokens_in,
        tokens_out,
        context_pct,
        cost,
        model: Some("claude-opus-4-8".to_string()),
    };

    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();

    let r1 = ur(100, 20, Some(30.0), 0.01);
    let r2 = ur(250, 60, Some(55.0), 0.03);
    let deltas = [
        telemetry_sample(None, &r1),
        telemetry_sample(Some(&r1), &r2),
    ];

    for sample in deltas {
        let ev = TelemetrySampled {
            sample,
            model: Some("claude-opus-4-8".to_string()),
            execution_profile_id: Some("prof_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string()),
        };
        store
            .append(telemetry_intent(&sid, &pid, "2026-06-08T12:00:00Z", &ev))
            .unwrap();
    }

    // same dims (session/project/profile/model/day) → ONE rollup row, SUM-of-deltas.
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM proj_usage_ledger"),
        1,
        "two deltas, same dims → one rollup row"
    );
    let (ti, to, ctx, cost, _q, day) = usage_row(&path, sid.as_str());
    assert_eq!(
        (ti, to),
        (250, 60),
        "SUM-of-deltas == cumulative total (delta-not-cumulative; cumulative would be 350/80)"
    );
    assert!(
        (cost - 0.03).abs() < 1e-9,
        "cost SUM-of-deltas == cumulative 0.03"
    );
    assert_eq!(
        ctx,
        Some(55.0),
        "context_pct_max = MAX gauge 55 (a deltaed context would be 25 → MAX 30)"
    );
    assert_eq!(
        day, "2026-06-08",
        "bucket_day == the UTC date of occurred_at"
    );
}

// =============================================================================
// edges-022 — the proj_worktree projector (the gated 5.2-remainder)
// =============================================================================
//
// Driven Gateway-end-to-end (the proj_approval_queue sibling-read precedent): a real
// git.create_worktree (submit → approve → execute) creates the action_requests sibling
// row AND emits WorktreeCreated, which the projector folds in-band → proj_worktree.

/// A Gateway with the real GitExecutor over a FakeGitCli (emits WorktreeCreated on approve).
fn gw_with_git() -> Gateway {
    let mut cat = CatalogExecutor::new();
    cat.register(
        ExecutorKind::Git,
        Arc::new(GitExecutor::new(Box::new(FakeGitCli::succeeding()))),
    );
    Gateway::new(Box::new(CatalogPolicy), Box::new(cat))
}

/// A `git.create_worktree` request. `repo_id`: Some → a Repository resource_ref carrying it (the repo
/// identity the projector sibling-reads); None → a non-Repo ref (satisfies requires_resource_refs but
/// has no repo identity → the projector skips). LOW-ENTROPY inputs (the §7.2 approve-path redaction).
fn create_worktree_req(project_id: Option<ProjectId>, repo_id: Option<&str>) -> ActionRequest {
    let resource_refs = match repo_id {
        Some(rid) => vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: rid.to_string(),
            uri: None,
        }],
        None => vec![ResourceRef {
            resource_type: ResourceType::Worktree,
            id: "wt_other".to_string(),
            uri: None,
        }],
    };
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id,
        action_type: "git.create_worktree".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs,
        inputs: serde_json::json!({
            "repo_path": "/repo", "worktree_path": "/repo/wt", "branch_name": "feature",
            "base_branch": "main"
        }),
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-08T00:00:00Z").unwrap(),
    }
}

fn approval_id_of(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("ro conn");
    // deterministic single-approval lookup (each test uses one worktree per db → one approval).
    conn.query_row(
        "SELECT approval_id FROM approvals ORDER BY approval_id LIMIT 1",
        [],
        |r| r.get(0),
    )
    .expect("an approval")
}

/// submit + approve a git.create_worktree → drives WorktreeCreated + the in-band proj_worktree fold.
fn create_worktree(
    store: &mut EventStore,
    gw: &Gateway,
    path: &std::path::Path,
    req: ActionRequest,
) {
    gw.submit_action(store, req).expect("submit");
    gw.approve(store, &approval_id_of(path)).expect("approve");
}

/// the proj_worktree rows (the asserted columns), ordered by worktree_id, for byte-identical compare.
#[derive(Debug, PartialEq)]
struct WtRow {
    worktree_id: String,
    project_id: String,
    repo_id: String,
    path: String,
    branch_name: Option<String>,
    base_branch: Option<String>,
    status: String,
    dirty_state: Option<String>,
    ahead_count: Option<i64>,
    behind_count: Option<i64>,
    git_checked_at: Option<String>,
    updated_at_seq: i64,
}

fn proj_worktree_rows(path: &std::path::Path) -> Vec<WtRow> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("ro conn");
    let mut stmt = conn
        .prepare(
            "SELECT worktree_id, project_id, repo_id, path, branch_name, base_branch, status, \
             dirty_state, ahead_count, behind_count, git_checked_at, updated_at_seq \
             FROM proj_worktree ORDER BY worktree_id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok(WtRow {
                worktree_id: r.get(0)?,
                project_id: r.get(1)?,
                repo_id: r.get(2)?,
                path: r.get(3)?,
                branch_name: r.get(4)?,
                base_branch: r.get(5)?,
                status: r.get(6)?,
                dirty_state: r.get(7)?,
                ahead_count: r.get(8)?,
                behind_count: r.get(9)?,
                git_checked_at: r.get(10)?,
                updated_at_seq: r.get(11)?,
            })
        })
        .unwrap();
    rows.map(|r| r.unwrap()).collect()
}

#[test]
fn test_worktree_created_inserts_proj_worktree_row() {
    // spec(§7.2): git.create_worktree (submit→approve→execute) → a proj_worktree row with the payload +
    // sibling-sourced project_id/repo_id + initial status + updated_at_seq.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    create_worktree(
        &mut store,
        &gw,
        &path,
        create_worktree_req(Some(pid.clone()), Some("repo_alpha")),
    );

    let rows = proj_worktree_rows(&path);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert!(r.worktree_id.starts_with("wt_"));
    assert_eq!(r.project_id, pid.as_str());
    assert_eq!(r.repo_id, "repo_alpha");
    assert_eq!(r.path, "/repo/wt");
    assert_eq!(r.branch_name.as_deref(), Some("feature"));
    assert_eq!(
        r.base_branch.as_deref(),
        Some("main"),
        "base_branch round-trips from the payload"
    );
    assert_eq!(r.status, "creating");
    assert!(r.updated_at_seq > 0);
}

#[test]
fn test_worktree_projector_repo_id_from_sibling() {
    // spec(LESSON 17): repo_id is the immutable sibling read of the action's Repository resource_ref.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    create_worktree(
        &mut store,
        &gw,
        &path,
        create_worktree_req(Some(ProjectId::new()), Some("repo_beta")),
    );
    assert_eq!(proj_worktree_rows(&path)[0].repo_id, "repo_beta");
}

#[test]
fn test_worktree_projector_live_read_columns_null() {
    // spec(§7.2 split): the live-read cache columns are inserted NULL (a separate refresh populates them).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    create_worktree(
        &mut store,
        &gw,
        &path,
        create_worktree_req(Some(ProjectId::new()), Some("repo_x")),
    );
    let r = &proj_worktree_rows(&path)[0];
    assert_eq!(r.dirty_state, None);
    assert_eq!(r.ahead_count, None);
    assert_eq!(r.behind_count, None);
    assert_eq!(r.git_checked_at, None);
}

#[test]
fn test_worktree_projector_skips_identity_less() {
    // spec(healthy skip): a WorktreeCreated with no project_id OR no repo ref → no row, no error
    // (proj_worktree.project_id/repo_id are NOT NULL; the session.rs skip precedent).
    // (a) no project_id:
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    create_worktree(
        &mut store,
        &gw,
        &path,
        create_worktree_req(None, Some("repo_x")),
    );
    assert_eq!(proj_worktree_rows(&path).len(), 0, "no project_id → skip");

    // (b) no repo ref (a non-Repo resource_ref):
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    let gw2 = gw_with_git();
    create_worktree(
        &mut store2,
        &gw2,
        &path2,
        create_worktree_req(Some(ProjectId::new()), None),
    );
    assert_eq!(proj_worktree_rows(&path2).len(), 0, "no repo ref → skip");
}

#[test]
fn test_worktree_projector_skips_no_action_request_id() {
    // spec(healthy skip): a WorktreeCreated whose envelope carries project_id but NO action_request_id
    // (structurally possible — it's Option on the envelope) → no sibling to resolve repo_id → skip,
    // no row, no error. Direct append (the Gateway always sets action_request_id, so this exercises the
    // other half of the identity guard).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let payload = serde_json::to_string(&WorktreeCreated {
        worktree_id: WorktreeId::new(),
        path: "/repo/wt".to_string(),
        branch_name: "feature".to_string(),
        base_branch: None,
    })
    .unwrap();
    let mut i = intent(&payload);
    i.event_type = "WorktreeCreated".to_string();
    i.project_id = Some(ProjectId::new()); // project_id present, action_request_id stays None
    store.append(i).unwrap();
    assert_eq!(
        proj_worktree_rows(&path).len(),
        0,
        "no action_request_id → no sibling → skip"
    );
}

#[test]
fn test_worktree_projector_status_binds_5_1() {
    // spec(§5.1): status is the canonical §5.1 Worktree wire value (overlay lifecycle "creating"), not raw.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    create_worktree(
        &mut store,
        &gw,
        &path,
        create_worktree_req(Some(ProjectId::new()), Some("repo_x")),
    );
    assert_eq!(proj_worktree_rows(&path)[0].status, "creating");
}

#[test]
fn test_worktree_projector_rebuild_equivalent() {
    // spec(LESSON 4/17): rebuild() reproduces byte-identical proj_worktree rows — the immutable
    // sibling-read (action_requests read at final state) is deterministic.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    create_worktree(
        &mut store,
        &gw,
        &path,
        create_worktree_req(Some(ProjectId::new()), Some("repo_r")),
    );
    let before = proj_worktree_rows(&path);
    assert_eq!(before.len(), 1);
    store.rebuild_projections().unwrap();
    let after = proj_worktree_rows(&path);
    assert_eq!(
        before, after,
        "rebuild reproduces the incremental proj_worktree state"
    );
}

#[test]
fn test_worktree_projector_ignores_other_events() {
    // spec: the projector folds ONLY WorktreeCreated — a non-WorktreeCreated event writes no proj_worktree.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent(
            &SessionId::new(),
            &ProjectId::new(),
            "{\"status\":\"active\"}",
        ))
        .unwrap();
    assert_eq!(proj_worktree_rows(&path).len(), 0);
}

// =============================================================================
// edges-025 — the proj_pull_request projector (Wave-E; closes the github read vertical)
// =============================================================================
//
// Decoupled from the github write-client + runtime: `submit_action` is executor-agnostic (it persists
// the action_requests sibling row at AwaitingApproval without invoking the executor), so a submit seeds
// the sibling row and a DIRECT PullRequestSynced append drives the in-band proj_pull_request fold. The
// PROJECTOR is what's under test here (edges-023's e2e already proved the github executor emits the
// event). The exact edges-022 proj_worktree precedent.

/// A github.create_pr request carrying a Repo resource_ref (the repo identity the projector
/// sibling-reads). Some(repo_id) → a Repo ref; None → a non-Repo ref (no repo identity → projector skips).
fn github_pr_req(project_id: Option<ProjectId>, repo_id: Option<&str>) -> ActionRequest {
    let resource_refs = match repo_id {
        Some(rid) => vec![ResourceRef {
            resource_type: ResourceType::Repo,
            id: rid.to_string(),
            uri: None,
        }],
        None => vec![ResourceRef {
            resource_type: ResourceType::Worktree,
            id: "wt_other".to_string(),
            uri: None,
        }],
    };
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id,
        action_type: "github.create_pr".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs,
        inputs: serde_json::json!({
            "owner": "acme", "repo": "widget", "head": "feature", "base": "main", "title": "T"
        }),
        risk_level: RiskLevel::Level3,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-08T00:00:00Z").unwrap(),
    }
}

/// submit a github.create_pr (executor-agnostic → persists the action_requests sibling row at
/// AwaitingApproval) → returns its action_request_id (the projector's LESSON-17 sibling-read key).
fn seed_pr_action(
    store: &mut EventStore,
    gw: &Gateway,
    project_id: Option<ProjectId>,
    repo_id: Option<&str>,
) -> ActionRequestId {
    let req = github_pr_req(project_id, repo_id);
    let arid = req.action_request_id.clone();
    gw.submit_action(store, req)
        .expect("submit seeds the action_requests sibling row");
    arid
}

/// a PullRequestSynced append intent linked to `action_request_id` (the sibling-read key) + `project_id`.
fn pr_synced_intent(
    project_id: Option<ProjectId>,
    action_request_id: Option<ActionRequestId>,
    payload: &str,
) -> AppendIntent {
    let mut i = intent(payload);
    i.event_type = "PullRequestSynced".to_string();
    i.project_id = project_id;
    i.action_request_id = action_request_id;
    i
}

/// D9 — a PullRequestMerged append intent linked to `action_request_id` (the repo_id sibling-read key) +
/// `project_id` (same envelope as the synced event → folds the SAME `{repo_id}#{pr_number}` row).
fn pr_merged_intent(
    project_id: Option<ProjectId>,
    action_request_id: Option<ActionRequestId>,
    payload: &str,
) -> AppendIntent {
    let mut i = intent(payload);
    i.event_type = "PullRequestMerged".to_string();
    i.project_id = project_id;
    i.action_request_id = action_request_id;
    i
}

/// D9 — a PullRequestMerged payload (pr_number + the merge-commit SHA + the merged-at stamp).
fn pr_merged_payload(pr_number: u64, merge_commit_sha: Option<&str>) -> String {
    serde_json::to_string(&PullRequestMerged {
        pr_number,
        merge_commit_sha: merge_commit_sha.map(|s| s.to_string()),
        merged_at: Timestamp::parse("2026-06-20T00:00:00Z").unwrap(),
    })
    .unwrap()
}

fn pr_payload(pr_number: u64, status: PullRequest, branch: &str, base: &str) -> String {
    serde_json::to_string(&PullRequestSynced {
        pr_number,
        status,
        branch: branch.to_string(),
        base: base.to_string(),
        mergeable: None,
        checks_summary: None,
        additions: None,
        deletions: None,
        changed_files: None,
        commits: None,
        pr_checked_at: Timestamp::parse("2026-06-08T00:00:00Z").unwrap(),
    })
    .unwrap()
}

/// like `pr_payload` but sets the D5a enrichment fields (mergeable/checks_summary) — the data the event
/// has always carried (P7.1), now surfaced on the row.
fn pr_payload_rich(
    pr_number: u64,
    status: PullRequest,
    branch: &str,
    base: &str,
    mergeable: Option<bool>,
    checks_summary: Option<&str>,
) -> String {
    serde_json::to_string(&PullRequestSynced {
        pr_number,
        status,
        branch: branch.to_string(),
        base: base.to_string(),
        mergeable,
        checks_summary: checks_summary.map(|s| s.to_string()),
        additions: None,
        deletions: None,
        changed_files: None,
        commits: None,
        pr_checked_at: Timestamp::parse("2026-06-08T00:00:00Z").unwrap(),
    })
    .unwrap()
}

/// D6 — like `pr_payload` but sets the diff-stats enrichment (additions/deletions/changed_files/commits),
/// the data the octocrab GET PR carries; the §11.2 PR card renders them.
#[allow(clippy::too_many_arguments)]
fn pr_payload_diff_stats(
    pr_number: u64,
    status: PullRequest,
    branch: &str,
    base: &str,
    additions: Option<u64>,
    deletions: Option<u64>,
    changed_files: Option<u64>,
    commits: Option<u64>,
) -> String {
    serde_json::to_string(&PullRequestSynced {
        pr_number,
        status,
        branch: branch.to_string(),
        base: base.to_string(),
        mergeable: None,
        checks_summary: None,
        additions,
        deletions,
        changed_files,
        commits,
        pr_checked_at: Timestamp::parse("2026-06-08T00:00:00Z").unwrap(),
    })
    .unwrap()
}

/// the proj_pull_request rows (the asserted columns), ordered by pr_id, for byte-identical compare.
#[derive(Debug, PartialEq)]
struct PrRow {
    pr_id: String,
    project_id: Option<String>,
    repo_id: Option<String>,
    pr_number: Option<i64>,
    title: Option<String>,
    status: String,
    head_branch: Option<String>,
    base_branch: Option<String>,
    pr_checked_at: Option<String>,
    // D5a — the mergeable/checks_summary enrichment. `mergeable` is a SQLite INTEGER (0/1); rusqlite
    // coerces INTEGER → Option<bool> on the typed `get` (distinct from the JSON-read serve path).
    mergeable: Option<bool>,
    checks_summary: Option<String>,
    // D6 — the diff-stats enrichment (INTEGER columns; u64 stored as i64, lossless for real PR stats).
    additions: Option<i64>,
    deletions: Option<i64>,
    changed_files: Option<i64>,
    commits: Option<i64>,
    updated_at_seq: i64,
}

fn proj_pull_request_rows(path: &std::path::Path) -> Vec<PrRow> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("ro conn");
    let mut stmt = conn
        .prepare(
            "SELECT pr_id, project_id, repo_id, pr_number, title, status, head_branch, base_branch, \
             pr_checked_at, mergeable, checks_summary, additions, deletions, changed_files, commits, \
             updated_at_seq FROM proj_pull_request \
             ORDER BY pr_id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok(PrRow {
                pr_id: r.get(0)?,
                project_id: r.get(1)?,
                repo_id: r.get(2)?,
                pr_number: r.get(3)?,
                title: r.get(4)?,
                status: r.get(5)?,
                head_branch: r.get(6)?,
                base_branch: r.get(7)?,
                pr_checked_at: r.get(8)?,
                mergeable: r.get(9)?,
                checks_summary: r.get(10)?,
                additions: r.get(11)?,
                deletions: r.get(12)?,
                changed_files: r.get(13)?,
                commits: r.get(14)?,
                updated_at_seq: r.get(15)?,
            })
        })
        .unwrap();
    rows.map(|r| r.unwrap()).collect()
}

#[test]
fn test_pull_request_synced_folds_to_proj() {
    // spec(§7.2/§7): a PullRequestSynced append → one proj_pull_request row with pr_number/head_branch/
    // base_branch/pr_checked_at from the payload, project_id from the envelope, repo_id from the sibling ref.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid),
            &pr_payload(42, PullRequest::Open, "feature", "main"),
        ))
        .expect("append folds in-band");
    let rows = proj_pull_request_rows(&path);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.project_id.as_deref(), Some(pid.as_str()));
    assert_eq!(r.repo_id.as_deref(), Some("repo_alpha"));
    assert_eq!(r.pr_number, Some(42));
    assert_eq!(r.head_branch.as_deref(), Some("feature"));
    assert_eq!(r.base_branch.as_deref(), Some("main"));
    assert_eq!(r.pr_checked_at.as_deref(), Some("2026-06-08T00:00:00Z"));
    assert!(r.updated_at_seq > 0);
}

#[test]
fn test_pr_id_composite_deterministic() {
    // spec(Q1 — rebuild-safe key): pr_id = the {repo_id}#{pr_number} composite; two folds of the same
    // (repo, pr_number) hit the SAME row (NOT a minted ULID — proj_pull_request is in REBUILD_TABLES).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid.clone()),
            &pr_payload(7, PullRequest::Open, "f", "m"),
        ))
        .unwrap();
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid),
            &pr_payload(7, PullRequest::Merged, "f", "m"),
        ))
        .unwrap();
    let rows = proj_pull_request_rows(&path);
    assert_eq!(
        rows.len(),
        1,
        "same (repo, pr_number) → one row (composite key)"
    );
    assert_eq!(rows[0].pr_id, "repo_alpha#7");
}

#[test]
fn test_status_binds_pull_request_wire_value() {
    // spec(§5.1): the status column is the canonical PullRequest snake_case wire value (not raw / hardcoded).
    for (status, wire) in [
        (PullRequest::Open, "open"),
        (PullRequest::Merged, "merged"),
        (PullRequest::ChecksFailing, "checks_failing"),
    ] {
        let (_d, path) = temp_db();
        let mut store = open(&path);
        let gw = gw_with_git();
        let pid = ProjectId::new();
        let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_x"));
        store
            .append(pr_synced_intent(
                Some(pid),
                Some(arid),
                &pr_payload(1, status, "f", "m"),
            ))
            .unwrap();
        assert_eq!(proj_pull_request_rows(&path)[0].status, wire);
    }
}

#[test]
fn test_title_null_mergeable_checks_null_when_absent() {
    // spec: title is NULL (the PullRequestSynced event carries no title); after D5a mergeable/checks_summary
    // HAVE columns (the SPREAD), but `pr_payload` carries None for both → they fold to NULL.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_x"));
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid),
            &pr_payload(1, PullRequest::Open, "f", "m"),
        ))
        .unwrap();
    let r = &proj_pull_request_rows(&path)[0];
    assert_eq!(r.title, None, "the event has no title → NULL");
    assert_eq!(r.mergeable, None, "absent mergeable → NULL");
    assert_eq!(r.checks_summary, None, "absent checks_summary → NULL");
}

#[test]
fn test_pull_request_synced_projects_mergeable_and_checks() {
    // spec(§7.2): the projector folds PullRequestSynced.mergeable?/checks_summary? into the 2 D5a columns
    // (Some → value, None → NULL); rebuild-equivalent (derive-from-event, LESSON §17). The data was always
    // in the event (P7.1) — D5a surfaces it on the row.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid.clone()),
            &pr_payload_rich(
                42,
                PullRequest::Open,
                "feature",
                "main",
                Some(true),
                Some("3 passing"),
            ),
        ))
        .expect("append folds in-band");
    let r = &proj_pull_request_rows(&path)[0];
    assert_eq!(r.mergeable, Some(true), "mergeable folded from the event");
    assert_eq!(
        r.checks_summary.as_deref(),
        Some("3 passing"),
        "checks_summary folded from the event"
    );

    // ON CONFLICT DO UPDATE folds the enriched columns too: re-sync the SAME pr_id (reuse the sibling
    // action, the test_on_conflict_updates_row pattern) with CHANGED mergeable/checks → the row reflects
    // the new values (not the stale INSERT values).
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid),
            &pr_payload_rich(
                42,
                PullRequest::ChecksFailing,
                "feature",
                "main",
                Some(false),
                Some("1 failing"),
            ),
        ))
        .unwrap();
    let rows = proj_pull_request_rows(&path);
    assert_eq!(rows.len(), 1, "re-sync of the same pr_id → one row");
    assert_eq!(rows[0].mergeable, Some(false), "DO UPDATE folds mergeable");
    assert_eq!(
        rows[0].checks_summary.as_deref(),
        Some("1 failing"),
        "DO UPDATE folds checks_summary"
    );

    // rebuild-equivalent: the enriched columns survive a rebuild (derive-from-event, REBUILD_TABLES).
    let before = proj_pull_request_rows(&path);
    store.rebuild_projections().unwrap();
    assert_eq!(
        before,
        proj_pull_request_rows(&path),
        "the enriched row is reproduced byte-identically on rebuild"
    );

    // None/None → NULL (a distinct repo, so a distinct pr_id row).
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    let gw2 = gw_with_git();
    let pid2 = ProjectId::new();
    let arid2 = seed_pr_action(&mut store2, &gw2, Some(pid2.clone()), Some("repo_beta"));
    store2
        .append(pr_synced_intent(
            Some(pid2),
            Some(arid2),
            &pr_payload_rich(7, PullRequest::Open, "f", "m", None, None),
        ))
        .unwrap();
    let n = &proj_pull_request_rows(&path2)[0];
    assert_eq!(n.mergeable, None, "absent mergeable → NULL");
    assert_eq!(n.checks_summary, None, "absent checks_summary → NULL");
}

#[test]
fn test_read_pull_request_typed_serves_mergeable_checks() {
    // spec(§7.2/§5.0): the typed serve round-trips the 2 D5a fields — a Some(true)/Some(text) row AND a
    // None/None row both deserialize STRICTLY into the frozen PullRequestRow, fail-closed preserved. Pins
    // the INTEGER(0/1)→bool coercion in read_pull_request_typed: `mergeable` is the FIRST bool projection
    // column → SQLite stores it as INTEGER → the read layer coerces it to the contract's JSON bool (without
    // the coercion the strict deserialize of `Option<bool>` over a JSON number fails closed).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid),
            &pr_payload_rich(
                42,
                PullRequest::Open,
                "feature",
                "main",
                Some(true),
                Some("3 passing"),
            ),
        ))
        .unwrap();
    // a 2nd, distinct PR with mergeable=Some(false) — the FALSY edge (INTEGER 0 → JSON false), DISTINCT
    // from Some(true) (INTEGER 1): a naive truthiness slip or treating 0 as absent only surfaces here.
    let arid_false = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_gamma"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid_false),
            &pr_payload_rich(
                9,
                PullRequest::ChecksFailing,
                "f",
                "m",
                Some(false),
                Some("2 failing"),
            ),
        ))
        .unwrap();
    // a 3rd, distinct PR with None/None.
    let arid2 = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_beta"));
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid2),
            &pr_payload_rich(7, PullRequest::Merged, "f", "m", None, None),
        ))
        .unwrap();

    let rows = nexusopsd::ipc::read_pull_request_typed(&path).expect("typed pull-request read");
    let some = rows
        .iter()
        .find(|r| r.pr_id == "repo_alpha#42")
        .expect("the Some(true) row");
    assert_eq!(
        some.mergeable,
        Some(true),
        "mergeable=true served as a typed bool"
    );
    assert_eq!(some.checks_summary.as_deref(), Some("3 passing"));
    let f = rows
        .iter()
        .find(|r| r.pr_id == "repo_gamma#9")
        .expect("the Some(false) row");
    assert_eq!(
        f.mergeable,
        Some(false),
        "mergeable=false (INTEGER 0 → JSON false) — the falsy coercion edge, not None/true"
    );
    assert_eq!(f.checks_summary.as_deref(), Some("2 failing"));
    let none = rows
        .iter()
        .find(|r| r.pr_id == "repo_beta#7")
        .expect("the None row");
    assert_eq!(none.mergeable, None, "absent mergeable → None on the wire");
    assert_eq!(none.checks_summary, None);
}

#[test]
fn test_migration_13_applies() {
    // spec(MIGRATION_13 floor, LESSON §50): a fresh DB opens at/above user_version 13 and proj_pull_request
    // gained the mergeable + checks_summary columns (ALTER-only; the historical CREATE untouched). FLOOR
    // (>= 13), not exact-latest — the single exact-latest pin lives in gateway_plan.rs.
    let (_d, path) = temp_db();
    let store = open(&path);
    assert!(
        store.user_version().unwrap() >= 13,
        "open migrates at/above MIGRATION_13"
    );
    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let cols: std::collections::BTreeSet<String> = conn
        .prepare("SELECT name FROM pragma_table_info('proj_pull_request')")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(cols.contains("mergeable"), "MIGRATION_13 adds mergeable");
    assert!(
        cols.contains("checks_summary"),
        "MIGRATION_13 adds checks_summary"
    );
}

#[test]
fn test_migration_15_applies() {
    // spec(MIGRATION_15 floor, LESSON §50): a fresh DB opens at/above user_version 15 and
    // proj_pull_request gained the 4 diff-stats columns (ALTER-only, the MIGRATION_13 precedent; the
    // historical CREATE untouched). FLOOR (>= 15), not exact-latest — the single exact-latest pin lives
    // in gateway_plan.rs.
    let (_d, path) = temp_db();
    let store = open(&path);
    assert!(
        store.user_version().unwrap() >= 15,
        "open migrates at/above MIGRATION_15"
    );
    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let cols: std::collections::BTreeSet<String> = conn
        .prepare("SELECT name FROM pragma_table_info('proj_pull_request')")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    for c in ["additions", "deletions", "changed_files", "commits"] {
        assert!(cols.contains(c), "MIGRATION_15 adds proj_pull_request.{c}");
    }
}

#[test]
fn test_pr_diff_stats_folded() {
    // spec(§7.2 / LESSON §53): the projector folds PullRequestSynced.additions?/deletions?/changed_files?/
    // commits? into the 4 D6 columns (Some → value, None → NULL); rebuild-equivalent (derive-from-event,
    // LESSON §17). A populated event → the 4 values on the row; an absent-stats event → NULL (the
    // rebuild-safe None arm).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    // a fully-populated diff-stats event.
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid.clone()),
            &pr_payload_diff_stats(
                42,
                PullRequest::Open,
                "feature",
                "main",
                Some(120),
                Some(7),
                Some(4),
                Some(3),
            ),
        ))
        .expect("append folds in-band");
    // a 2nd PR with NO diff-stats (all None) → the NULL arm.
    let arid_none = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_beta"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid_none),
            &pr_payload_diff_stats(7, PullRequest::Merged, "f", "m", None, None, None, None),
        ))
        .unwrap();

    let rows = proj_pull_request_rows(&path);
    let some = rows.iter().find(|r| r.pr_id == "repo_alpha#42").unwrap();
    assert_eq!(some.additions, Some(120), "additions folded from the event");
    assert_eq!(some.deletions, Some(7), "deletions folded from the event");
    assert_eq!(some.changed_files, Some(4), "changed_files folded");
    assert_eq!(some.commits, Some(3), "commits folded");
    let none = rows.iter().find(|r| r.pr_id == "repo_beta#7").unwrap();
    assert_eq!(none.additions, None, "absent additions → NULL");
    assert_eq!(none.deletions, None, "absent deletions → NULL");
    assert_eq!(none.changed_files, None, "absent changed_files → NULL");
    assert_eq!(none.commits, None, "absent commits → NULL");

    // ON CONFLICT DO UPDATE folds the diff-stats too (the D5a mergeable/checks precedent): re-sync the
    // SAME pr_id with CHANGED stats → the row reflects the new values, not the stale INSERT values.
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid),
            &pr_payload_diff_stats(
                42,
                PullRequest::Open,
                "feature",
                "main",
                Some(200),
                Some(50),
                Some(9),
                Some(5),
            ),
        ))
        .unwrap();
    let resynced = proj_pull_request_rows(&path);
    let some = resynced
        .iter()
        .find(|r| r.pr_id == "repo_alpha#42")
        .unwrap();
    assert_eq!(some.additions, Some(200), "DO UPDATE folds additions");
    assert_eq!(some.deletions, Some(50), "DO UPDATE folds deletions");
    assert_eq!(some.changed_files, Some(9), "DO UPDATE folds changed_files");
    assert_eq!(some.commits, Some(5), "DO UPDATE folds commits");

    // rebuild-equivalent: the diff-stat columns survive a rebuild (derive-from-event, REBUILD_TABLES).
    let before = proj_pull_request_rows(&path);
    store.rebuild_projections().unwrap();
    assert_eq!(
        before,
        proj_pull_request_rows(&path),
        "the diff-stats row is reproduced byte-identically on rebuild (LESSON §17)"
    );
}

#[test]
fn test_pull_request_merged_folds_to_terminal() {
    // spec(§5.1/§7.2 / D9 / LESSON 17): a PullRequestMerged append folds proj_pull_request.status →
    // terminal `merged` for the SAME `{repo_id}#{pr_number}` row a prior PullRequestSynced created. The
    // status is derived from the EVENT TYPE (not the row's current value) → rebuild-safe. Other columns
    // (branch/base/pr_number) are untouched; updated_at_seq advances. rebuild-equivalent.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    // 1. a synced PR → the row at Open. (The synced + merged events share ONE action_requests sibling row
    // here — the projector only needs it to resolve repo_id via the LESSON-17 sibling-read; in production
    // the merge is a separate github.merge_pr action carrying its own Repo ref to the SAME repo_id. A 2nd
    // seed_pr_action would DEDUP on the NaturalResourceRef idempotency key [same repo] → no new row.)
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid.clone()),
            &pr_payload(42, PullRequest::Open, "feature", "main"),
        ))
        .expect("synced fold");
    assert_eq!(
        proj_pull_request_rows(&path)[0].status,
        "open",
        "the PR starts Open"
    );

    // 2. a merge of the SAME PR (same repo_id sibling → same `{repo_id}#{pr_number}` pr_id) → `merged`.
    store
        .append(pr_merged_intent(
            Some(pid),
            Some(arid),
            &pr_merged_payload(42, Some("9fceb02")),
        ))
        .expect("merged fold");
    let rows = proj_pull_request_rows(&path);
    assert_eq!(
        rows.len(),
        1,
        "the merge UPDATES the existing row (no new row)"
    );
    let r = &rows[0];
    assert_eq!(r.pr_id, "repo_alpha#42");
    assert_eq!(
        r.status, "merged",
        "PullRequestMerged folds status → terminal merged (derived from the event type)"
    );
    assert_eq!(
        r.pr_number,
        Some(42),
        "pr_number untouched by the merge fold"
    );
    assert_eq!(
        r.head_branch.as_deref(),
        Some("feature"),
        "branch untouched"
    );

    // 3. rebuild-equivalent: the merged status survives a full rebuild (derive-from-event, REBUILD_TABLES).
    let before = proj_pull_request_rows(&path);
    store.rebuild_projections().unwrap();
    assert_eq!(
        before,
        proj_pull_request_rows(&path),
        "the merged row is reproduced byte-identically on rebuild (LESSON 17)"
    );
}

#[test]
fn test_pull_request_merged_no_prior_row_is_healthy_noop() {
    // spec(D9): a PullRequestMerged with NO prior PullRequestSynced row → the projector's UPDATE hits 0
    // rows → a HEALTHY no-op (NOT an error, NOT a fabricated INSERT). The row materializes only when the
    // PR is synced; on a full event log the synced event always precedes the merge, so this guards the
    // partial/out-of-order edge (an unintended INSERT or a 0-rows-error regression would fail here).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_merged_intent(
            Some(pid),
            Some(arid),
            &pr_merged_payload(99, Some("deadbeef")),
        ))
        .expect("a merge with no prior synced row is a healthy no-op (not an error)");
    assert!(
        proj_pull_request_rows(&path).is_empty(),
        "no row is created by a bare merge (the UPDATE hit 0 rows — no fabricated INSERT)"
    );
}

#[test]
fn test_read_pull_request_typed_diff_stats() {
    // spec(§7.2/§5.0 / LESSON §53): the fail-closed typed serve round-trips the 4 D6 fields — a populated
    // row AND a NULL row both deserialize STRICTLY into the frozen PullRequestRow. The diff-stats are
    // INTEGER columns surfacing as JSON numbers → bind directly into Option<u64> (NO bool-coercion, unlike
    // D5a's mergeable). The column+row-field+serve must land together or the serve fails closed (LESSON §53).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid),
            &pr_payload_diff_stats(
                42,
                PullRequest::Open,
                "feature",
                "main",
                Some(120),
                Some(7),
                Some(4),
                Some(3),
            ),
        ))
        .unwrap();
    let arid2 = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_beta"));
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid2),
            &pr_payload_diff_stats(7, PullRequest::Merged, "f", "m", None, None, None, None),
        ))
        .unwrap();

    let rows = nexusopsd::ipc::read_pull_request_typed(&path).expect("typed pull-request read");
    let some = rows.iter().find(|r| r.pr_id == "repo_alpha#42").unwrap();
    assert_eq!(some.additions, Some(120), "additions served as typed u64");
    assert_eq!(some.deletions, Some(7));
    assert_eq!(some.changed_files, Some(4));
    assert_eq!(some.commits, Some(3));
    let none = rows.iter().find(|r| r.pr_id == "repo_beta#7").unwrap();
    assert_eq!(none.additions, None, "absent additions → None on the wire");
    assert_eq!(none.deletions, None);
    assert_eq!(none.changed_files, None);
    assert_eq!(none.commits, None);
}

#[test]
fn test_missing_identity_healthy_skip() {
    // spec(edges-022 case 1): no project_id / no action_request_id / no Repository ref → no row, no error.
    // (a) no project_id:
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let arid = seed_pr_action(&mut store, &gw, Some(ProjectId::new()), Some("repo_x"));
    store
        .append(pr_synced_intent(
            None,
            Some(arid),
            &pr_payload(1, PullRequest::Open, "f", "m"),
        ))
        .expect("append (skip, no error)");
    assert_eq!(
        proj_pull_request_rows(&path).len(),
        0,
        "no project_id → skip"
    );

    // (b) no action_request_id (no sibling to resolve repo_id):
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    store2
        .append(pr_synced_intent(
            Some(ProjectId::new()),
            None,
            &pr_payload(1, PullRequest::Open, "f", "m"),
        ))
        .expect("append (skip, no error)");
    assert_eq!(
        proj_pull_request_rows(&path2).len(),
        0,
        "no action_request_id → skip"
    );

    // (c) a sibling row with NO Repository ref (a non-Repo resource_ref). Reuse the SAME project_id for
    // the seed + the synced-intent, so the ONLY reason for the skip is the absent Repo ref (the projector
    // doesn't compare project_ids — a distinct pid would be a false diagnostic; the worktree precedent).
    let (_d3, path3) = temp_db();
    let mut store3 = open(&path3);
    let gw3 = gw_with_git();
    let pid3 = ProjectId::new();
    let arid3 = seed_pr_action(&mut store3, &gw3, Some(pid3.clone()), None);
    store3
        .append(pr_synced_intent(
            Some(pid3),
            Some(arid3),
            &pr_payload(1, PullRequest::Open, "f", "m"),
        ))
        .expect("append (skip, no error)");
    assert_eq!(
        proj_pull_request_rows(&path3).len(),
        0,
        "no Repository ref → skip"
    );
}

#[test]
fn test_missing_sibling_row_fail_closed() {
    // spec(edges-022 case 2 / LESSON 17): link set but the action_requests sibling row is GONE (a dangling
    // action_request_id never submitted) → fail-closed Db (the ? propagates QueryReturnedNoRows; the
    // append/replay txn aborts). NOT a silent default.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let dangling = ActionRequestId::new(); // never submitted → no sibling row
    let result = store.append(pr_synced_intent(
        Some(ProjectId::new()),
        Some(dangling),
        &pr_payload(1, PullRequest::Open, "f", "m"),
    ));
    assert!(
        result.is_err(),
        "a missing sibling action_requests row is an integrity break → fail-closed (append aborts)"
    );
    assert_eq!(
        proj_pull_request_rows(&path).len(),
        0,
        "the aborted append wrote no proj row"
    );
}

#[test]
fn test_unbindable_payload_degrades() {
    // spec(edges-022 case 3): a valid sibling row + an UNBINDABLE PullRequestSynced payload → Decode-degrade
    // (skip, no row); the append succeeds (a degrade is contained, not propagated) and the reason echoes NO
    // payload bytes (§15). Distinct from the missing-sibling Db break. NOTE: the payload must be VALID JSON
    // (the events table's `CHECK json_valid(payload_json)` rejects non-JSON at INSERT, before the projector)
    // but the WRONG SHAPE — `deny_unknown_fields` + missing required fields → it won't bind PullRequestSynced.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_x"));
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid),
            r#"{"not_a_pull_request_field":true}"#,
        ))
        .expect("append succeeds — a decode-degrade is contained, not propagated");
    assert_eq!(
        proj_pull_request_rows(&path).len(),
        0,
        "an unbindable payload → degrade, no row"
    );
}

#[test]
fn test_on_conflict_updates_row() {
    // spec: a re-fold of the same pr_id UPDATEs the row (status + seq advance), still one row (re-sync idempotent).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_x"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid.clone()),
            &pr_payload(5, PullRequest::Open, "f", "m"),
        ))
        .unwrap();
    let seq1 = proj_pull_request_rows(&path)[0].updated_at_seq;
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid),
            &pr_payload(5, PullRequest::Merged, "f", "m"),
        ))
        .unwrap();
    let rows = proj_pull_request_rows(&path);
    assert_eq!(rows.len(), 1, "re-sync of the same pr_id → one row");
    assert_eq!(rows[0].status, "merged", "status updated on re-sync");
    assert!(
        rows[0].updated_at_seq > seq1,
        "updated_at_seq advanced on re-sync"
    );
}

#[test]
fn test_proj_pull_request_rebuild_equivalent() {
    // spec(REBUILD_TABLES determinism): rebuild() reproduces byte-identical proj_pull_request rows — the
    // composite {repo_id}#{pr_number} key + the deterministic columns + the immutable sibling-read guarantee it.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_r"));
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid),
            &pr_payload(9, PullRequest::Open, "feature", "main"),
        ))
        .unwrap();
    let before = proj_pull_request_rows(&path);
    assert_eq!(before.len(), 1);
    store.rebuild_projections().unwrap();
    let after = proj_pull_request_rows(&path);
    assert_eq!(
        before, after,
        "rebuild reproduces the incremental proj_pull_request state"
    );
}

#[test]
fn test_pull_request_projector_ignores_other_events() {
    // spec: the projector folds ONLY PullRequestSynced — a non-PullRequestSynced event writes no proj row.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent(
            &SessionId::new(),
            &ProjectId::new(),
            "{\"status\":\"active\"}",
        ))
        .unwrap();
    assert_eq!(proj_pull_request_rows(&path).len(), 0);
}

// ---- P7.2 — the PullRequest projection is served TYPED (the ApprovalQueue typed-serve precedent) ----

#[test]
fn test_get_projection_serves_typed_pull_request_row() {
    // spec(§6.1 / §7.2 — P7.2): the PullRequest projection is served TYPED via `read_pull_request_typed`
    // (the ApprovalQueue typed-serve precedent, pin #2 / LESSON §37) — the REAL projector-folded row
    // deserializes STRICTLY into the frozen `PullRequestRow` (no loose JSON; the internal `updated_at_seq`
    // dropped; `status` the typed §5.1 enum). Drives the in-band projector (a PullRequestSynced append
    // over a seeded sibling row), then reads via the typed serve.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid.clone()),
            Some(arid),
            &pr_payload(42, PullRequest::Open, "feature", "main"),
        ))
        .expect("append folds in-band");

    let rows = nexusopsd::ipc::read_pull_request_typed(&path).expect("typed pull-request read");
    assert_eq!(rows.len(), 1, "the folded PR row is served");
    let r = &rows[0];
    assert_eq!(r.pr_id, "repo_alpha#42");
    assert_eq!(r.project_id.as_deref(), Some(pid.as_str()));
    assert_eq!(r.repo_id.as_deref(), Some("repo_alpha"));
    assert_eq!(r.pr_number, Some(42));
    assert_eq!(r.status, PullRequest::Open, "status is the typed enum");
    assert_eq!(r.head_branch.as_deref(), Some("feature"));
    assert_eq!(r.base_branch.as_deref(), Some("main"));
    assert_eq!(r.pr_checked_at.as_deref(), Some("2026-06-08T00:00:00Z"));
    assert_eq!(r.title, None, "title is NULL (the event carries none)");
}

#[test]
fn test_pull_request_typed_serve_fails_closed() {
    // spec(§7.2 / LESSON §37): the typed serve FAILS CLOSED on a proj_pull_request row that no longer
    // deserializes into the frozen `PullRequestRow` (a corrupt/contract-broken row) → `InternalError`,
    // never a silent skip or a loose-JSON fallback. Forces it by corrupting `status` to a value the
    // frozen §5.1 PullRequest machine rejects (a direct writable conn — test-only fixture corruption,
    // the gateway.rs precedent; production never writes this).
    use nexusops_shared::ipc::IpcErrorCode;
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(pr_synced_intent(
            Some(pid),
            Some(arid),
            &pr_payload(7, PullRequest::Open, "f", "m"),
        ))
        .expect("append folds in-band");
    {
        let c = rusqlite::Connection::open(&path).expect("fixture conn");
        c.execute(
            "UPDATE proj_pull_request SET status = 'not_a_real_status'",
            [],
        )
        .expect("corrupt the status");
    }
    let err = nexusopsd::ipc::read_pull_request_typed(&path)
        .expect_err("a row that doesn't deserialize fails closed");
    assert_eq!(
        err,
        IpcErrorCode::InternalError,
        "a corrupt/unbindable row → InternalError (fail-closed, never a silent skip)"
    );
}

// =============================================================================
// D2 (P4.4) — the SessionRecovered fold (§8.1/§11.4 survival display) + the typed SessionRow serve
// =============================================================================

/// a SessionRecovered intent linked to `session_id` (the recovery-fold UPDATE key).
fn session_recovered_intent(session_id: &SessionId, payload: &str) -> AppendIntent {
    let mut i = intent(payload);
    i.event_type = SessionRecovered::EVENT_TYPE.to_string();
    i.session_id = Some(session_id.clone());
    i
}

fn session_recovered_payload(mode: ResumeMode, replayed: u64) -> String {
    serde_json::to_string(&SessionRecovered {
        mode,
        replayed_event_count: replayed,
        execution_profile_id: None,
    })
    .unwrap()
}

/// (resume_mode, replayed_event_count, recovered_at) for a proj_session row.
fn proj_session_recovery(
    path: &std::path::Path,
    session_id: &str,
) -> (Option<String>, Option<i64>, Option<String>) {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT resume_mode, replayed_event_count, recovered_at FROM proj_session \
             WHERE session_id=?1",
            [session_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
}

#[test]
fn test_session_recovered_folds_recovery_fields() {
    // spec(§7/§8.1/§11.4) — a SessionRecovered folds the recovery OUTCOME onto proj_session (the
    // resumed-vs-replayed-vs-reattached banner source). resume_mode binds the §8.1 ResumeMode wire value;
    // replayed_event_count + recovered_at (= the event occurred_at) populate.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"active\"}"))
        .expect("SessionStarted folds the base row");
    store
        .append(session_recovered_intent(
            &sid,
            &session_recovered_payload(ResumeMode::Replayed, 7),
        ))
        .expect("SessionRecovered folds in-band");
    let (mode, replayed, recovered_at) = proj_session_recovery(&path, sid.as_str());
    assert_eq!(mode.as_deref(), Some("replayed"));
    assert_eq!(replayed, Some(7));
    assert_eq!(recovered_at.as_deref(), Some("2026-06-08T00:00:00Z"));
}

#[test]
fn test_session_recovered_unknown_session_noop() {
    // spec — a SessionRecovered for an absent session = a healthy no-op (UPDATE affects 0 rows, no
    // degrade; the SessionFailed precedent). No proj_session row materializes.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    store
        .append(session_recovered_intent(
            &sid,
            &session_recovered_payload(ResumeMode::Resumed, 0),
        ))
        .expect("a SessionRecovered for an unknown session is a healthy no-op");
    assert_eq!(count(&path, "SELECT COUNT(*) FROM proj_session"), 0);
}

#[test]
fn test_session_recovered_rebuild_equivalence() {
    // spec(LESSON §17) — the recovery fields derive from the EVENT (type/payload), so a full rebuild
    // re-derives them identically (mutable-from-event-type, rebuild-safe).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"active\"}"))
        .unwrap();
    store
        .append(session_recovered_intent(
            &sid,
            &session_recovered_payload(ResumeMode::ReattachedLive, 0),
        ))
        .unwrap();
    let before = proj_session_recovery(&path, sid.as_str());
    store.rebuild_projections().unwrap();
    let after = proj_session_recovery(&path, sid.as_str());
    assert_eq!(
        before, after,
        "rebuild re-derives the recovery fields identically"
    );
    assert_eq!(before.0.as_deref(), Some("reattached_live"));
}

#[test]
fn test_resume_mode_binds_enum() {
    // spec(§5.1/§8.1) — resume_mode binds the §8.1 ResumeMode (reject-unknown): a SessionRecovered whose
    // payload carries an UNKNOWN mode wire value fails to bind → the projector degrades + skips (never
    // stores raw); the recovery columns stay NULL.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"active\"}"))
        .unwrap();
    // a bogus mode → the SessionRecovered payload doesn't bind → Decode-degrade (skip), not raw-store.
    store
        .append(session_recovered_intent(
            &sid,
            "{\"mode\":\"not_a_mode\",\"replayed_event_count\":0,\"execution_profile_id\":null}",
        ))
        .expect("a degrade-skip commits the append (logic error, not Db error)");
    let (mode, _replayed, _recovered_at) = proj_session_recovery(&path, sid.as_str());
    assert_eq!(
        mode, None,
        "an unknown ResumeMode degrades+skips — never stored raw"
    );
}

// ---- C2 — the Session projection is served TYPED (the ApprovalQueue/PullRequest precedent) ----

#[test]
fn test_get_projection_serves_typed_session_row() {
    // spec(§6.1/§7.2/§11.4) — get_projection("Session") is served TYPED via read_session_typed (no loose
    // JSON). A recovered row carries resume_mode/replayed_event_count/recovered_at; a never-recovered row
    // carries them None. Both deserialize strictly into the frozen SessionRow.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let pid = ProjectId::new();
    let recovered = SessionId::new();
    let fresh = SessionId::new();
    store
        .append(session_intent(&recovered, &pid, "{\"status\":\"active\"}"))
        .unwrap();
    store
        .append(session_recovered_intent(
            &recovered,
            &session_recovered_payload(ResumeMode::Resumed, 0),
        ))
        .unwrap();
    store
        .append(session_intent(&fresh, &pid, "{\"status\":\"active\"}"))
        .unwrap();

    let rows = nexusopsd::ipc::read_session_typed(&path).expect("typed session read");
    assert_eq!(rows.len(), 2, "both sessions served");
    let rec = rows
        .iter()
        .find(|r| r.session_id == recovered.as_str())
        .unwrap();
    assert_eq!(rec.status, Session::Active, "status is the typed §5.1 enum");
    assert_eq!(rec.resume_mode, Some(ResumeMode::Resumed));
    assert_eq!(rec.recovered_at.as_deref(), Some("2026-06-08T00:00:00Z"));
    let new = rows
        .iter()
        .find(|r| r.session_id == fresh.as_str())
        .unwrap();
    assert_eq!(new.resume_mode, None, "a never-recovered row carries None");
    assert_eq!(new.replayed_event_count, None);
}

#[test]
fn test_session_typed_serve_fails_closed() {
    // spec(LESSON §37) — the typed serve FAILS CLOSED on a proj_session row that no longer deserializes
    // (a corrupt status) → InternalError, never a silent skip. (Direct writable conn = test-only fixture
    // corruption; production never writes this.)
    use nexusops_shared::ipc::IpcErrorCode;
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let sid = SessionId::new();
    let pid = ProjectId::new();
    store
        .append(session_intent(&sid, &pid, "{\"status\":\"active\"}"))
        .unwrap();
    {
        let c = rusqlite::Connection::open(&path).expect("fixture conn");
        c.execute("UPDATE proj_session SET status = 'not_a_real_status'", [])
            .expect("corrupt the status");
    }
    let err = nexusopsd::ipc::read_session_typed(&path)
        .expect_err("a row that doesn't deserialize fails closed");
    assert_eq!(err, IpcErrorCode::InternalError);
}

// =============================================================================
// D5b-1 (P4.6) — the structured-review vertical: ReviewSynced fold + proj_review + typed ReviewRow serve
// =============================================================================

/// a ReviewSynced append intent linked to `action_request_id` (the sibling-read key) + `project_id`.
fn review_synced_intent(
    project_id: Option<ProjectId>,
    action_request_id: Option<ActionRequestId>,
    payload: &str,
) -> AppendIntent {
    let mut i = intent(payload);
    i.event_type = ReviewSynced::EVENT_TYPE.to_string();
    i.project_id = project_id;
    i.action_request_id = action_request_id;
    i
}

fn review_payload(
    review_id: u64,
    pr_number: u64,
    reviewer: &str,
    state: ReviewState,
    submitted_at: Option<&str>,
    body: Option<&str>,
) -> String {
    serde_json::to_string(&ReviewSynced {
        review_id,
        pr_number,
        reviewer: reviewer.to_string(),
        state,
        submitted_at: submitted_at.map(|s| Timestamp::parse(s).unwrap()),
        body: body.map(|s| s.to_string()),
        review_synced_at: Timestamp::parse("2026-06-16T00:00:00Z").unwrap(),
    })
    .unwrap()
}

/// the proj_review rows (the asserted columns), ordered by review_id, for byte-identical compare.
#[derive(Debug, PartialEq)]
struct ReviewRowT {
    review_id: u64,
    pr_number: Option<u64>,
    project_id: Option<String>,
    repo_id: Option<String>,
    reviewer: Option<String>,
    state: String,
    submitted_at: Option<String>,
    body: Option<String>,
    updated_at_seq: i64,
}

fn proj_review_rows(path: &std::path::Path) -> Vec<ReviewRowT> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("ro conn");
    let mut stmt = conn
        .prepare(
            "SELECT review_id, pr_number, project_id, repo_id, reviewer, state, submitted_at, body, \
             updated_at_seq FROM proj_review ORDER BY review_id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok(ReviewRowT {
                review_id: r.get(0)?,
                pr_number: r.get(1)?,
                project_id: r.get(2)?,
                repo_id: r.get(3)?,
                reviewer: r.get(4)?,
                state: r.get(5)?,
                submitted_at: r.get(6)?,
                body: r.get(7)?,
                updated_at_seq: r.get(8)?,
            })
        })
        .unwrap();
    rows.map(|r| r.unwrap()).collect()
}

#[test]
fn test_review_synced_projects_to_proj_review() {
    // spec(§7.2 / LESSONS §17/§48): a synthetic ReviewSynced folds into one proj_review row — all fields
    // from the payload (review_id PK, pr_number, reviewer, state, submitted_at, body) + project_id from the
    // envelope + repo_id sibling-read (the PullRequestProjector precedent). submitted_at=None for a pending
    // review. Rebuild-equivalent (derive-from-event, REBUILD_TABLES).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(review_synced_intent(
            Some(pid.clone()),
            Some(arid),
            &review_payload(
                9001,
                42,
                "octocat",
                ReviewState::Approved,
                Some("2026-06-15T00:00:00Z"),
                Some("LGTM"),
            ),
        ))
        .expect("append folds in-band");
    let rows = proj_review_rows(&path);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.review_id, 9001);
    assert_eq!(r.pr_number, Some(42));
    assert_eq!(r.project_id.as_deref(), Some(pid.as_str()));
    assert_eq!(r.repo_id.as_deref(), Some("repo_alpha"));
    assert_eq!(r.reviewer.as_deref(), Some("octocat"));
    assert_eq!(r.state, "approved");
    assert_eq!(r.submitted_at.as_deref(), Some("2026-06-15T00:00:00Z"));
    assert_eq!(r.body.as_deref(), Some("LGTM"));
    assert!(r.updated_at_seq > 0);

    // rebuild-equivalent.
    let before = proj_review_rows(&path);
    store.rebuild_projections().unwrap();
    assert_eq!(
        before,
        proj_review_rows(&path),
        "rebuild reproduces the proj_review row"
    );

    // a pending review on a 2nd repo → submitted_at/body NULL.
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    let gw2 = gw_with_git();
    let pid2 = ProjectId::new();
    let arid2 = seed_pr_action(&mut store2, &gw2, Some(pid2.clone()), Some("repo_beta"));
    store2
        .append(review_synced_intent(
            Some(pid2),
            Some(arid2),
            &review_payload(9002, 7, "hubot", ReviewState::Pending, None, None),
        ))
        .unwrap();
    let p = &proj_review_rows(&path2)[0];
    assert_eq!(p.state, "pending");
    assert_eq!(p.submitted_at, None, "pending → NULL submitted_at");
    assert_eq!(p.body, None, "no body → NULL");
}

#[test]
fn test_review_synced_on_conflict_updates_row() {
    // spec(§7.2): a re-sync of the same review_id UPDATEs the row (state + body + seq advance), still one row.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_x"));
    store
        .append(review_synced_intent(
            Some(pid.clone()),
            Some(arid.clone()),
            &review_payload(5, 1, "octocat", ReviewState::Commented, None, Some("wip")),
        ))
        .unwrap();
    let seq1 = proj_review_rows(&path)[0].updated_at_seq;
    store
        .append(review_synced_intent(
            Some(pid),
            Some(arid),
            &review_payload(
                5,
                1,
                "octocat",
                ReviewState::ChangesRequested,
                None,
                Some("nit"),
            ),
        ))
        .unwrap();
    let rows = proj_review_rows(&path);
    assert_eq!(rows.len(), 1, "re-sync of the same review_id → one row");
    assert_eq!(rows[0].state, "changes_requested", "DO UPDATE folds state");
    assert_eq!(rows[0].body.as_deref(), Some("nit"), "DO UPDATE folds body");
    assert!(rows[0].updated_at_seq > seq1, "seq advanced on re-sync");
}

#[test]
fn test_review_projector_rejects_unknown_state() {
    // spec(§5.1): a ReviewSynced payload with an unbindable `state` wire value → Decode-degrade (skip, no
    // row); the append succeeds (a degrade is contained), and the reason echoes NO payload bytes (§15). The
    // valid JSON / wrong shape case (the proj_pull_request test_unbindable_payload_degrades precedent).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_x"));
    store
        .append(review_synced_intent(
            Some(pid),
            Some(arid),
            r#"{"review_id":1,"pr_number":1,"reviewer":"x","state":"not_a_real_state","review_synced_at":"2026-06-16T00:00:00Z"}"#,
        ))
        .expect("append succeeds — a decode-degrade is contained, not propagated");
    assert_eq!(
        proj_review_rows(&path).len(),
        0,
        "an unbindable state → degrade, no row"
    );
}

#[test]
fn test_read_review_typed_serves_review_row() {
    // spec(§7.2/§5.0 / LESSONS §37): the Review projection is served TYPED via read_review_typed — the REAL
    // folded row deserializes STRICTLY into the frozen ReviewRow (no loose JSON; updated_at_seq dropped;
    // state the typed ReviewState enum). A Some-body row AND a None-body row both round-trip, fail-closed.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(review_synced_intent(
            Some(pid.clone()),
            Some(arid),
            &review_payload(
                9001,
                42,
                "octocat",
                ReviewState::Approved,
                Some("2026-06-15T00:00:00Z"),
                Some("LGTM"),
            ),
        ))
        .unwrap();
    let arid2 = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_beta"));
    store
        .append(review_synced_intent(
            Some(pid),
            Some(arid2),
            &review_payload(9002, 7, "hubot", ReviewState::Pending, None, None),
        ))
        .unwrap();

    let rows = nexusopsd::ipc::read_review_typed(&path).expect("typed review read");
    let approved = rows
        .iter()
        .find(|r| r.review_id == 9001)
        .expect("the approved row");
    assert_eq!(
        approved.state,
        ReviewState::Approved,
        "state is the typed enum"
    );
    assert_eq!(approved.reviewer.as_deref(), Some("octocat"));
    assert_eq!(approved.body.as_deref(), Some("LGTM"));
    let pending = rows
        .iter()
        .find(|r| r.review_id == 9002)
        .expect("the pending row");
    assert_eq!(pending.state, ReviewState::Pending);
    assert_eq!(pending.body, None, "no body → None on the wire");
    assert_eq!(pending.submitted_at, None);
}

#[test]
fn test_review_typed_serve_fails_closed() {
    // spec(§7.2 / LESSONS §37): the typed serve FAILS CLOSED on a proj_review row that no longer deserializes
    // (a corrupt state) → InternalError, never a silent skip. (Direct writable conn = test-only fixture.)
    use nexusops_shared::ipc::IpcErrorCode;
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gw_with_git();
    let pid = ProjectId::new();
    let arid = seed_pr_action(&mut store, &gw, Some(pid.clone()), Some("repo_alpha"));
    store
        .append(review_synced_intent(
            Some(pid),
            Some(arid),
            &review_payload(7, 1, "octocat", ReviewState::Approved, None, None),
        ))
        .unwrap();
    {
        let c = rusqlite::Connection::open(&path).expect("fixture conn");
        c.execute("UPDATE proj_review SET state = 'not_a_real_state'", [])
            .expect("corrupt the state");
    }
    let err = nexusopsd::ipc::read_review_typed(&path)
        .expect_err("a row that doesn't deserialize fails closed");
    assert_eq!(err, IpcErrorCode::InternalError);
}

#[test]
fn test_migration_14_applies() {
    // spec(MIGRATION_14 floor, LESSONS §50): a fresh DB opens at/above user_version 14 and proj_review exists
    // (the FLOOR, >= 14 — the single exact-latest pin lives in gateway_plan.rs).
    let (_d, path) = temp_db();
    let store = open(&path);
    assert!(
        store.user_version().unwrap() >= 14,
        "open migrates at/above MIGRATION_14"
    );
    assert!(
        table_names(&path).contains("proj_review"),
        "MIGRATION_14 creates proj_review"
    );
}

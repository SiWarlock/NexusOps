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
use nexusops_shared::events::WorktreeCreated;
use nexusops_shared::ids::{ActionRequestId, ProjectId, SessionId, WorkspaceId, WorktreeId};
use nexusops_shared::status::ActionRequestStatus;
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

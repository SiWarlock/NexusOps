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

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{RedactionStatus, Sensitivity, SourceType, Visibility};
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    AppendIntent, EventStore, EventStoreError, PrefixRedactor, RedactionOutcome, Redactor,
};

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

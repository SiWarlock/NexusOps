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
use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{AppendIntent, EventStore, PrefixRedactor};

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
    assert_eq!(store.user_version(), 3, "open migrates to user_version 3");

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
        conn.execute(
            "INSERT INTO events (event_id, seq, event_type, event_version, occurred_at, \
             recorded_at, workspace_id, actor_type, actor_id, source_type, source_id, \
             correlation_id, sensitivity, payload_json, redaction_status) \
             VALUES ('evt_x',1,'SessionStarted',1,'t','t','ws_x','user','u','desktop_ui','s',\
             'c','internal','{}','redacted')",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
    }
    // opening raises 2→3 on a NON-EMPTY db → backup `.bak-2` first (§16)
    let store = open(&path);
    assert_eq!(store.user_version(), 3, "migrated to 3");
    let bak = std::path::PathBuf::from(format!("{}.bak-2", path.display()));
    assert!(
        bak.exists(),
        "auto-backup wrote .bak-2 before the 2→3 raise"
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

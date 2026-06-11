//! Phase 1.1 (L2) — event-store mechanics (RED first). ARCHITECTURE §7/§7.1/§16/§17,
//! DATA_MODEL §2.1. Single-writer WAL append/read-by-seq, migrations + backup/rollback,
//! degrade/quarantine-on-read, deterministic golden-log. (Redaction gate tests 4/5
//! land with the L3 redaction sub-feature.)

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{EventEnvelope, RedactionStatus, Sensitivity, SourceType};
use nexusops_shared::ids::WorkspaceId;
use nexusopsd::clock::{Clock, FixedClock};
use nexusopsd::eventstore::{
    AppendIntent, EventStore, EventStoreError, PrefixRedactor, RedactionOutcome, Redactor,
    SUPPORTED_USER_VERSION,
};
use nexusopsd::idgen::{FixedIdGen, UlidGen};

/// test Redactor that refuses to redact — exercises the fail-closed gate (test 4).
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

/// A minimal valid append intent (the store assigns event_id, seq, recorded_at).
fn intent(occurred_at: &str, payload: &str) -> AppendIntent {
    AppendIntent {
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: occurred_at.to_string(),
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
    }
}

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-07T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

// ---- Test 1 — append then read back by seq (§7.1/§7) -------------------------

#[test]
fn test_append_then_read_by_seq() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    for i in 0..5 {
        store
            .append(intent("2026-06-07T00:00:00Z", &format!("{{\"i\":{i}}}")))
            .unwrap();
    }
    let events = store.read_all().unwrap();
    assert_eq!(events.len(), 5);
    // monotonic, gapless seq in read order
    for (idx, e) in events.iter().enumerate() {
        assert_eq!(e.seq, (idx as i64) + 1, "seq monotonic from 1");
    }
}

// ---- Test 2 — seq is canonical order, not occurred_at (§7.1, EM §23) ---------

#[test]
fn test_seq_is_canonical_not_occurred_at() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    // append in DESCENDING occurred_at; read order must still follow seq (append order)
    store.append(intent("2026-06-07T03:00:00Z", "{}")).unwrap();
    store.append(intent("2026-06-07T01:00:00Z", "{}")).unwrap();
    store.append(intent("2026-06-07T02:00:00Z", "{}")).unwrap();
    let events = store.read_all().unwrap();
    let occ: Vec<&str> = events.iter().map(|e| e.occurred_at.as_str()).collect();
    assert_eq!(
        occ,
        vec![
            "2026-06-07T03:00:00Z",
            "2026-06-07T01:00:00Z",
            "2026-06-07T02:00:00Z"
        ]
    );
    // both timestamps kept; recorded_at comes from the injected FixedClock
    assert!(events
        .iter()
        .all(|e| e.recorded_at == "2026-06-07T00:00:00Z"));
}

// ---- Test 3 — single writer; readers are read-only (forbidden #3 / §15) ------

#[test]
fn test_single_writer_readers_are_readonly() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store.append(intent("2026-06-07T00:00:00Z", "{}")).unwrap();
    // a read-only connection cannot mutate the log
    let reader = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let write = reader.execute("DELETE FROM events", []);
    assert!(
        write.is_err(),
        "read-only connection must reject writes (single-writer §15)"
    );
    drop(store);
}

// ---- Test 6 — audit-write failure fails closed (§15/§17) ---------------------

#[test]
fn test_audit_write_failure_fails_closed() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    // a malformed payload (invalid JSON) violates the events CHECK(json_valid) →
    // the INSERT fails → typed error, nothing persisted (no silent success).
    let bad = intent("2026-06-07T00:00:00Z", "{not valid json");
    let res = store.append(bad);
    assert!(
        matches!(res, Err(EventStoreError::Write(_))),
        "INSERT failure → typed error"
    );
    assert_eq!(
        store.read_all().unwrap().len(),
        0,
        "nothing persisted on failure"
    );
}

// ---- Test 7 — idempotency_key dedup (§7.1 / AG §16.1) ------------------------

#[test]
fn test_idempotency_key_dedup() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let mut a = intent("2026-06-07T00:00:00Z", "{}");
    a.idempotency_key = Some("idem_dup".to_string());
    store.append(a).unwrap();
    let mut b = intent("2026-06-07T00:00:01Z", "{}");
    b.idempotency_key = Some("idem_dup".to_string());
    let res = store.append(b);
    assert!(
        matches!(res, Err(EventStoreError::DuplicateIdempotencyKey)),
        "dup key rejected"
    );
    assert_eq!(store.read_all().unwrap().len(), 1);
}

// ---- Test 8 — migrations forward-only + backup/rollback (§16) ----------------

#[test]
fn test_migration_forward_only_with_backup_rollback() {
    let (_d, path) = temp_db();
    {
        let mut store = open(&path);
        store.append(intent("2026-06-07T00:00:00Z", "{}")).unwrap();
        assert!(
            store.user_version().unwrap() >= 1,
            "migrations ran to >=1 on open"
        );
    }
    // §16 backup/restore round-trip (the rollback primitive, WAL-safe)
    EventStore::backup_db(&path, 1).unwrap();
    {
        // simulate raw data loss on a throwaway conn; FK off because migration 3 gave
        // events child rows (object_refs/proj_audit_trail) — production never DELETEs
        // from the append-only log, this is only to verify the restore primitive.
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute("DELETE FROM events", []).unwrap();
    }
    EventStore::restore_db(&path, 1).unwrap();
    {
        let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "restore brought the event back");
    }
    // §16 version floor — normal db accepted; a too-new db refused
    EventStore::refuses_db_newer_than_supported(&path).unwrap();
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
    }
    assert!(
        matches!(
            EventStore::refuses_db_newer_than_supported(&path),
            Err(EventStoreError::DbNewerThanSupported { .. })
        ),
        "db newer than supported is refused"
    );
}

// ---- Test 9 — unknown event_version degrades, no crash (§17) -----------------

#[test]
fn test_unknown_event_version_degrades() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store.append(intent("2026-06-07T00:00:00Z", "{}")).unwrap();
    // inject a future event_version directly, then read: degraded marker, no panic
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE events SET event_version = 9999 WHERE seq = 1", [])
            .unwrap();
    }
    let events = store.read_all_degradable().unwrap();
    assert_eq!(events.len(), 1);
    assert!(
        events[0].is_degraded(),
        "unknown event_version → degraded marker"
    );
}

// ---- Test 10 — corrupt payload quarantined on read, no crash (§17) ----------

#[test]
fn test_corrupt_payload_quarantined() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store.append(intent("2026-06-07T00:00:00Z", "{}")).unwrap();
    // inject a corrupt payload, bypassing the json_valid CHECK, then read
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        conn.execute(
            "UPDATE events SET payload_json = '{not valid json' WHERE seq = 1",
            [],
        )
        .unwrap();
    }
    let events = store.read_all_degradable().unwrap();
    assert_eq!(events.len(), 1);
    assert!(
        events[0].is_quarantined(),
        "corrupt payload → quarantined, no crash"
    );
}

// ---- Test 11 — deterministic golden-log replay (§14) -------------------------

#[test]
fn test_golden_log_deterministic_replay() {
    // FIXED inputs (incl. workspace_id) so the WHOLE envelope is comparable —
    // the store-assigned fields (event_id/seq/recorded_at) must be deterministic.
    fn fixed_intent(n: usize) -> AppendIntent {
        AppendIntent {
            event_type: "SessionStarted".to_string(),
            event_version: 1,
            occurred_at: "2026-01-01T00:00:00Z".to_string(),
            workspace_id: WorkspaceId::parse("ws_01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
            actor_type: ActorType::User,
            actor_id: "u_1".to_string(),
            source_type: SourceType::DesktopUi,
            source_id: "src_1".to_string(),
            correlation_id: "corr_1".to_string(),
            sensitivity: Sensitivity::Internal,
            payload_json: format!("{{\"n\":{n}}}"),
            schema_version: "event-envelope-v1".to_string(),
            idempotency_key: None,
            project_id: None,
            session_id: None,
            agent_team_id: None,
            visibility: None,
        }
    }
    let golden = |path: &std::path::Path| -> Vec<EventEnvelope> {
        let mut store = EventStore::open(
            path,
            Box::new(FixedIdGen::new()),
            Box::new(FixedClock::new("2026-01-01T00:00:00Z")),
            Box::new(PrefixRedactor),
        )
        .unwrap();
        for i in 0..3 {
            store.append(fixed_intent(i)).unwrap();
        }
        store.read_all().unwrap()
    };
    let (_d1, p1) = temp_db();
    let (_d2, p2) = temp_db();
    // full-envelope equality: injected Clock+IdGen + fixed inputs → identical log
    assert_eq!(golden(&p1), golden(&p2), "byte-identical golden log");
}

// ---- Test (ADD) — WAL pragmas applied (ADR-003 / §18) -----------------------

#[test]
fn test_wal_pragmas_applied() {
    // per-connection pragmas (synchronous/foreign_keys/busy_timeout) must be on
    // the WRITER's connection (ADR-003); journal_mode is file-level.
    let (_d, path) = temp_db();
    let store = open(&path);
    assert_eq!(
        store.pragma_string("journal_mode").unwrap().to_lowercase(),
        "wal"
    );
    assert_eq!(
        store.pragma_i64("synchronous").unwrap(),
        1,
        "synchronous=NORMAL"
    );
    assert_eq!(
        store.pragma_i64("foreign_keys").unwrap(),
        1,
        "foreign_keys=ON"
    );
    assert_eq!(
        store.pragma_i64("busy_timeout").unwrap(),
        5000,
        "busy_timeout=5000"
    );
    let _ = Clock::now_rfc3339(&FixedClock::new("x")); // touch the trait import
}

// ---- Test 4 (L3, LOAD-BEARING) — writer refuses unredacted (§15) -------------

#[test]
fn test_redaction_gate_fail_closed() {
    // THE §15 redaction-before-persist invariant: the writer REFUSES to persist
    // any event that is not `redacted`. A Redactor yielding `unredacted` →
    // RedactionRequired error, nothing persisted.
    let (_d, path) = temp_db();
    let mut store = EventStore::open(
        &path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-07T00:00:00Z")),
        Box::new(NeverRedacts),
    )
    .unwrap();
    let res = store.append(intent("2026-06-07T00:00:00Z", "{}"));
    assert!(
        matches!(res, Err(EventStoreError::RedactionRequired)),
        "writer must refuse to persist a non-redacted event (§15)"
    );
    let reader = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let n: i64 = reader
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0, "nothing persisted when redaction didn't run");
}

// ---- Test 5 (L3) — secret token redacted before persist (§15) ----------------

#[test]
fn test_secret_token_redacted_before_persist() {
    let (_d, path) = temp_db();
    let mut store = open(&path); // PrefixRedactor
    store
        .append(intent(
            "2026-06-07T00:00:00Z",
            "{\"token\":\"ghp_SECRETSECRETSECRETSECRET\",\"key\":\"sk-abc123def456\"}",
        ))
        .unwrap();
    let events = store.read_all().unwrap();
    assert_eq!(events.len(), 1);
    let p = &events[0].payload_json;
    assert!(
        !p.contains("ghp_SECRETSECRETSECRETSECRET"),
        "ghp_ secret masked"
    );
    assert!(!p.contains("sk-abc123def456"), "sk- secret masked");
    assert!(p.contains("[REDACTED]"), "redaction marker present");
    assert_eq!(
        events[0].redaction_engine_version.as_deref(),
        Some("prefix-entropy-v3") // 2.0-SEC L3 — JSON-value detection bumped the engine (was v2 @ 1.7)
    );
}

// ---- Test (L3, must-close §16) — auto-backup before a raise on a non-empty db -

#[test]
fn test_auto_backup_before_migration_on_nonempty_db() {
    let (_d, path) = temp_db();
    // simulate a real v1 db that already holds an event (valid IDs so the startup
    // catch-up replay on open can reconstruct + fold it).
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(nexusopsd::eventstore::MIGRATION_1_EVENTS)
            .unwrap();
        conn.execute(
            "INSERT INTO events (event_id, seq, event_type, event_version, occurred_at, \
             recorded_at, workspace_id, actor_type, actor_id, source_type, source_id, \
             correlation_id, sensitivity, payload_json, schema_version) \
             VALUES ('evt_01ARZ3NDEKTSV4RRFFQ69G5FAV',1,'SessionStarted',1,\
             '2026-06-07T00:00:00Z','2026-06-07T00:00:00Z','ws_01ARZ3NDEKTSV4RRFFQ69G5FAV',\
             'user','u','desktop_ui','s','c','internal','{}','event-envelope-v1')",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
    }
    // opening raises 1→2→3 on a NON-EMPTY db → backup .bak-1 first (§16)
    let _store = open(&path);
    let bak = std::path::PathBuf::from(format!("{}.bak-1", path.display()));
    assert!(
        bak.exists(),
        "auto-backup wrote .bak-1 before the user_version raise"
    );
}

// ---- Test (1.6a L1) — user_version() is a typed Result, never a -1 sentinel ---

#[test]
fn test_user_version_returns_typed_result() {
    // 1.1 L2 carry-forward (§16 version-compat): the applied schema version must not be
    // read as a silent -1 sentinel. `Result<u32, _>` makes -1 impossible BY CONSTRUCTION
    // (a real read error is `Err`, never a magic integer the caller might branch on).
    let (_d, path) = temp_db();
    let store = open(&path);
    let v: Result<u32, EventStoreError> = store.user_version();
    assert_eq!(
        v.unwrap(),
        SUPPORTED_USER_VERSION as u32,
        "a migrated DB reports the supported user_version as a typed Ok(u32)"
    );
}

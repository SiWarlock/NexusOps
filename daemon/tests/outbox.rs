//! Phase 1.3 — transactional outbox (RED first). ARCHITECTURE §7 (event+projection+
//! **outbox** one txn), §12 (drainer), §15 (sync sink — outbox payload derives from
//! the already-redacted event), §17 (integration-failure contract), DATA_MODEL §2.5.
//!
//! Integration tests (public surface) per the 1.1/1.2 convention; FakeDestination /
//! StepClock implement the pub Destination / Clock traits. Layered:
//!   L1 (1–3) — outbox table (migration 4) + in-txn write + §15 sync gate.
//!   L2 (4–9) — drainer (drain_once / classify / backoff / dead-letter) + crash redeliver. [L2]

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{RedactionStatus, Sensitivity, SourceType};
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    AppendIntent, EventStore, EventStoreError, PrefixRedactor, RedactionOutcome, Redactor,
};

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

/// a SessionStarted intent carrying identity + a type-specific payload.
fn session_intent(payload: &str) -> AppendIntent {
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
        project_id: Some(ProjectId::new()),
        session_id: Some(SessionId::new()),
        agent_team_id: None,
        visibility: None,
    }
}

fn count(path: &std::path::Path, sql: &str) -> i64 {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

fn tables(path: &std::path::Path) -> std::collections::BTreeSet<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','index')")
        .unwrap();
    let out = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    out
}

/// a Redactor that refuses to redact — drives the §15 fail-closed gate abort.
struct NeverRedacts;
impl Redactor for NeverRedacts {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        RedactionOutcome {
            status: RedactionStatus::Unredacted,
            payload_json: payload_json.to_string(),
            engine_version: "never".to_string(),
        }
    }
}

// ---- Test 1 — migration 4 creates the outbox (§2.5/§16) ---------------------

#[test]
fn test_migration_4_creates_outbox() {
    let (_d, path) = temp_db();
    let store = open(&path);
    assert_eq!(store.user_version(), 4, "open migrates to user_version 4");
    let t = tables(&path);
    assert!(t.contains("outbox"), "migration 4 creates outbox");
    assert!(t.contains("ix_outbox_due"), "due-index created");
    assert!(
        t.contains("events") && t.contains("proj_session"),
        "spine + projections intact"
    );
}

// ---- Test 2 — outbox rows written in the append txn (§7 transactional) -------

#[test]
fn test_append_writes_outbox_rows_in_txn() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent("{\"status\":\"starting\"}"))
        .unwrap();

    // one pending row per subscribed destination (jsonl_mirror in Phase 1)
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM outbox"),
        1,
        "one pending row"
    );
    assert_eq!(
        count(
            &path,
            "SELECT COUNT(*) FROM outbox WHERE status='pending' AND destination='jsonl_mirror'"
        ),
        1,
        "pending jsonl_mirror row"
    );

    // boundary (§7.2): a full rebuild reconstructs read models only — it must NOT
    // re-emit / resurrect outbox delivery intents.
    store.rebuild_projections().unwrap();
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM outbox"),
        1,
        "rebuild leaves the outbox untouched (no re-delivery of history)"
    );

    // atomicity: a redaction-gate abort persists NOTHING — not the event, not the
    // projections, not the outbox (they commit / roll back together).
    let mut gated = EventStore::open(
        &path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(NeverRedacts),
    )
    .unwrap();
    let before_events = count(&path, "SELECT COUNT(*) FROM events");
    let res = gated.append(session_intent("{\"status\":\"active\"}"));
    assert!(
        matches!(res, Err(EventStoreError::RedactionRequired)),
        "the §15 gate refuses (fail-closed), proving the abort actually fired"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM events"),
        before_events,
        "no event"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM outbox"),
        1,
        "no new outbox row"
    );
}

// ---- Test 3 — outbox payload carries no secret (§15 sync sink) ---------------

#[test]
fn test_outbox_payload_has_no_secret() {
    let (_d, path) = temp_db();
    let mut store = open(&path); // PrefixRedactor redacts at the 1.1 gate
    store
        .append(session_intent(
            "{\"status\":\"starting\",\"token\":\"ghp_SECRETSECRETSECRETSECRET\"}",
        ))
        .unwrap();
    let payload: String = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row("SELECT payload_json FROM outbox", [], |r| r.get(0))
        .unwrap();
    assert!(
        !payload.contains("ghp_SECRETSECRETSECRETSECRET"),
        "the secret never reaches the outbox payload"
    );
    assert!(
        payload.contains("[REDACTED]"),
        "the outbox mirrors the redacted event"
    );
}

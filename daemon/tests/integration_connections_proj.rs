//! P7.1 Wave-C (edges-030) — the `proj_integration_connection` projector.
//!
//! Folds the edges-029 `IntegrationConnectionRegistered` event into a `proj_integration_connection`
//! read model (MIGRATION_11 lays the table), closing the Wave-C connection vertical (mutator → event →
//! projection). Like edges-028 (and UNLIKE edges-022) there is NO LESSON-17 sibling-read — the event is
//! self-contained. **Key difference from edges-028:** the identity `connection_id` is on the PAYLOAD
//! (the mutator minted it), NOT the envelope → the projector keys by `payload.connection_id`.
//!
//! **CONTRACT-neutral** — `IntegrationConnectionRegistered` is frozen at 0.26; no new `shared/` surface
//! (the CONTRACT 0.33 bump is edges-029's catalog add, not this slice). The IPC read is the deferred
//! follow-on. The assertions append a real event through the store and read the persisted rows.

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::events::{IntegrationConnectionRegistered, Provider};
use nexusops_shared::ids::WorkspaceId;
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

/// A clean `IntegrationConnectionRegistered` — `keychain_ref` is a low-entropy pointer (so the §15
/// redaction gate leaves it intact in the committed event; the edges-029 mutator guarantees this).
fn conn_payload(
    connection_id: &str,
    provider: Provider,
    account: Option<&str>,
) -> IntegrationConnectionRegistered {
    IntegrationConnectionRegistered {
        connection_id: connection_id.to_string(),
        provider,
        keychain_ref: "nexusops/github/ref".to_string(),
        account: account.map(|s| s.to_string()),
    }
}

fn append_connection(store: &mut EventStore, payload: &IntegrationConnectionRegistered) {
    let mut i = intent(&serde_json::to_string(payload).unwrap());
    i.event_type = IntegrationConnectionRegistered::EVENT_TYPE.to_string();
    store.append(i).unwrap();
}

#[derive(Debug, PartialEq)]
struct ConnRow {
    connection_id: String,
    provider: String,
    keychain_ref: String,
    account: Option<String>,
    status: String,
    updated_at_seq: i64,
}

fn conn_rows(path: &std::path::Path) -> Vec<ConnRow> {
    let conn = open_read_only(path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT connection_id, provider, keychain_ref, account, status, updated_at_seq \
             FROM proj_integration_connection ORDER BY connection_id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok(ConnRow {
                connection_id: r.get(0)?,
                provider: r.get(1)?,
                keychain_ref: r.get(2)?,
                account: r.get(3)?,
                status: r.get(4)?,
                updated_at_seq: r.get(5)?,
            })
        })
        .unwrap()
        .map(|r| r.unwrap());
    rows.collect()
}

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

// ---- Test 1 — the event folds, keyed by the PAYLOAD connection_id ----------

#[test]
fn test_registered_folds_proj_integration_connection() {
    // spec(§7.2): the event folds into a row keyed by payload.connection_id (NOT the envelope);
    // status='connected' is the resting state for a register; updated_at_seq = env.seq.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(
        &mut store,
        &conn_payload("conn_X", Provider::Github, Some("me")),
    );

    let rows = conn_rows(&path);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.connection_id, "conn_X");
    assert_eq!(r.provider, "github");
    assert_eq!(r.keychain_ref, "nexusops/github/ref");
    assert_eq!(r.account.as_deref(), Some("me"));
    assert_eq!(r.status, "connected");
    assert!(r.updated_at_seq > 0);
}

// ---- Test 2 — provider binds via the frozen-enum wire value -----------------

#[test]
fn test_provider_bound_via_wire_value() {
    // spec(LESSON 2): provider stores the canonical snake_case wire value via `wire_value`, not Debug.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(&mut store, &conn_payload("conn_L", Provider::Linear, None));
    assert_eq!(conn_rows(&path)[0].provider, "linear");
}

// ---- Test 3 — account None folds NULL --------------------------------------

#[test]
fn test_account_none_folds_null() {
    // spec(optional-as-null): account=None → the column is NULL; no row rejected.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(&mut store, &conn_payload("conn_N", Provider::Github, None));
    assert_eq!(conn_rows(&path)[0].account, None);
}

// ---- Test 4 — an unbindable / empty-id payload degrades (no payload echo) ---

#[test]
fn test_unbindable_payload_degrades() {
    // spec(§15 / reject-unknown): a typed-IntegrationConnectionRegistered event whose payload won't
    // bind (missing required fields) OR carries an empty connection_id → Decode-degrade + skip (no row,
    // offset degraded, not advanced); the generic reason never echoes payload bytes.
    // (a) won't bind:
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let mut i = intent("{}");
    i.event_type = IntegrationConnectionRegistered::EVENT_TYPE.to_string();
    store.append(i).unwrap();
    assert_eq!(
        conn_rows(&path).len(),
        0,
        "no row from an unbinding payload"
    );
    assert_eq!(
        offset(&path, "integration_connection"),
        (0, "degraded".to_string())
    );
    assert_eq!(
        store.read_all().unwrap().len(),
        1,
        "raw event intact (§7.2)"
    );

    // (b) binds but carries an empty connection_id → also Decode-degrade (a connection has no identity).
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    let mut i2 = intent("{\"connection_id\":\"\",\"provider\":\"github\",\"keychain_ref\":\"ref\",\"account\":null}");
    i2.event_type = IntegrationConnectionRegistered::EVENT_TYPE.to_string();
    store2.append(i2).unwrap();
    assert_eq!(
        conn_rows(&path2).len(),
        0,
        "no row from an empty connection_id"
    );
    assert_eq!(
        offset(&path2, "integration_connection"),
        (0, "degraded".to_string()),
        "an empty connection_id degrades AND does not advance the offset (no valid identity; LESSON 11)"
    );
}

// ---- Test 5 — a re-register UPSERTs + advances the seq ----------------------

#[test]
fn test_reregister_upserts_and_advances_seq() {
    // spec(idempotent re-fold): a 2nd event for the same connection_id REPLACES the row (DO UPDATE) +
    // advances updated_at_seq — 1 row, no dup.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(
        &mut store,
        &conn_payload("conn_X", Provider::Github, Some("me")),
    );
    let seq1 = conn_rows(&path)[0].updated_at_seq;
    // re-register with a changed account
    append_connection(
        &mut store,
        &conn_payload("conn_X", Provider::Github, Some("you")),
    );
    let rows = conn_rows(&path);
    assert_eq!(rows.len(), 1, "no dup row");
    assert_eq!(rows[0].account.as_deref(), Some("you"), "row replaced");
    assert!(rows[0].updated_at_seq > seq1, "seq advanced");
}

// ---- Test 6 — rebuild-equivalence ------------------------------------------

#[test]
fn test_rebuild_equivalence() {
    // spec(LESSON 4/17): the incremental in-band fold == a full rebuild() of the same log, byte-identical
    // (the table is in REBUILD_TABLES; the projection is event-derived).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(
        &mut store,
        &conn_payload("conn_A", Provider::Github, Some("a")),
    );
    append_connection(&mut store, &conn_payload("conn_B", Provider::Linear, None));
    append_connection(
        &mut store,
        &conn_payload("conn_A", Provider::Github, Some("a2")),
    ); // re-register A

    let before = conn_rows(&path);
    assert_eq!(before.len(), 2);
    store.rebuild_projections().unwrap();
    assert_eq!(
        before,
        conn_rows(&path),
        "rebuild reproduces proj_integration_connection byte-identically"
    );
}

// ---- Test 7 — MIGRATION_11 is wired + applied ------------------------------

#[test]
fn test_migration_11_applies() {
    // spec(LESSON 8 forward-only migration): once MIGRATION_11 is applied the table exists. Asserted as
    // a FLOOR (>= 11), not the exact latest — a later MIGRATION_12 would raise the version while this
    // table persists (cumulative; the projections.rs `>= 3` convention). gateway_plan pins the exact
    // latest for the double-bump guard.
    let (_d, path) = temp_db();
    let store = open(&path);
    assert!(
        store.user_version().unwrap() >= 11,
        "MIGRATION_11 applied (v11+)"
    );

    let conn = open_read_only(&path).unwrap();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            ["proj_integration_connection"],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1, "MIGRATION_11 must create proj_integration_connection");
    // (the runtime user_version >= 11 above + the table existence prove MIGRATION_11 applied; the
    // const SUPPORTED_USER_VERSION is pinned at the exact latest by gateway_plan's runtime assertion.)
}

// ---- Test 8 — the projector folds ONLY IntegrationConnectionRegistered ------

#[test]
fn test_ignores_other_event_types() {
    // spec(§7.2): the projector folds ONLY IntegrationConnectionRegistered — a different event writes
    // no proj_integration_connection row.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let mut i = intent("{\"status\":\"active\"}");
    i.event_type = "SessionStarted".to_string();
    store.append(i).unwrap();
    assert_eq!(conn_rows(&path).len(), 0);
}

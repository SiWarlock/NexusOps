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
use nexusops_shared::events::{
    IntegrationConnectionRegistered, IntegrationLiveWritesSet, Provider,
};
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

// ---- Test 8 — the projector folds ONLY its two events ----------------------

#[test]
fn test_ignores_other_event_types() {
    // spec(§7.2): the projector folds ONLY IntegrationConnectionRegistered (upsert the row) +
    // IntegrationLiveWritesSet (flip the toggle) — an UNRELATED event writes no row.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let mut i = intent("{\"status\":\"active\"}");
    i.event_type = "SessionStarted".to_string();
    store.append(i).unwrap();
    assert_eq!(conn_rows(&path).len(), 0);

    // an IntegrationLiveWritesSet for a connection that was NEVER registered → UPDATE matches 0 rows →
    // a healthy no-op (no row materialized; the executor's registered-gate prevents emitting this in
    // prod, but the projector must tolerate it idempotently — derive-from-event, never an INSERT).
    append_live_writes_set(&mut store, "conn_never_registered", true);
    assert_eq!(
        conn_rows(&path).len(),
        0,
        "IntegrationLiveWritesSet on an unknown connection materializes no row (UPDATE-0-rows no-op)"
    );
}

// ---- P4.7 (083 Q3) — the live-writes toggle fold (derive-from-event, default OFF, rebuild-safe) ----

/// append an `IntegrationLiveWritesSet{connection_id, enabled}` (the integration.set_live_writes emit).
fn append_live_writes_set(store: &mut EventStore, connection_id: &str, enabled: bool) {
    let payload = IntegrationLiveWritesSet {
        connection_id: connection_id.to_string(),
        enabled,
    };
    let mut i = intent(&serde_json::to_string(&payload).unwrap());
    i.event_type = IntegrationLiveWritesSet::EVENT_TYPE.to_string();
    store.append(i).unwrap();
}

/// read just `live_writes_enabled` for a connection (rusqlite INTEGER 0/1 → bool).
fn live_writes_enabled(path: &std::path::Path, connection_id: &str) -> bool {
    open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT live_writes_enabled FROM proj_integration_connection WHERE connection_id = ?1",
            [connection_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
        != 0
}

#[test]
fn test_live_writes_set_folds_to_proj() {
    // spec(§7.2 / 083 Q3 / LESSON 17): a registered connection defaults live_writes_enabled=OFF; an
    // IntegrationLiveWritesSet{enabled:true} folds it ON (derive-from-event, NOT a bare column write);
    // enabled:false folds it back OFF; rebuild-equivalent (the flag survives a projection rebuild).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(
        &mut store,
        &conn_payload("conn_X", Provider::Github, Some("me")),
    );
    assert!(
        !live_writes_enabled(&path, "conn_X"),
        "a freshly-registered connection defaults live_writes_enabled=OFF (the post-re-review gate)"
    );

    append_live_writes_set(&mut store, "conn_X", true);
    assert!(
        live_writes_enabled(&path, "conn_X"),
        "IntegrationLiveWritesSet{{enabled:true}} folds live_writes_enabled ON"
    );

    append_live_writes_set(&mut store, "conn_X", false);
    assert!(
        !live_writes_enabled(&path, "conn_X"),
        "enabled:false folds it back OFF (derive-from-event)"
    );

    // rebuild-equivalent: the toggle is reproduced from the event stream (not a lost column write).
    append_live_writes_set(&mut store, "conn_X", true);
    let before = live_writes_enabled(&path, "conn_X");
    store.rebuild_projections().unwrap();
    assert_eq!(
        before,
        live_writes_enabled(&path, "conn_X"),
        "the live_writes_enabled flag is reproduced byte-identically on rebuild (LESSON 17)"
    );
}

#[test]
fn test_reregister_preserves_live_writes_toggle() {
    // spec(083 Q3): a re-registration (IntegrationConnectionRegistered for an existing connection) MUST
    // NOT reset live_writes_enabled — the apply_registered ON CONFLICT set list deliberately omits the
    // toggle, so re-connecting never silently RE-ENABLES *or* disables a deliberately-set authorization
    // (the brief's "re-connecting never silently re-enables live writes"). A regression that added
    // `live_writes_enabled=excluded.live_writes_enabled` to the conflict set would flip this back to OFF.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(
        &mut store,
        &conn_payload("conn_X", Provider::Github, Some("me")),
    );
    append_live_writes_set(&mut store, "conn_X", true);
    assert!(live_writes_enabled(&path, "conn_X"), "toggle set ON");

    // re-register the SAME connection (e.g. a re-connect with a changed account) — the toggle survives.
    append_connection(
        &mut store,
        &conn_payload("conn_X", Provider::Github, Some("you")),
    );
    assert!(
        live_writes_enabled(&path, "conn_X"),
        "re-registration preserves the ON toggle (the ON CONFLICT set omits live_writes_enabled)"
    );
}

// ---- Test — MIGRATION_18 adds the live_writes_enabled column (LESSON §50 floor) ----

#[test]
fn test_migration_18_live_writes_column() {
    // spec(LESSON §50 forward-only migration floor): once MIGRATION_18 is applied the
    // proj_integration_connection table carries the live_writes_enabled column. Asserted as a FLOOR
    // (>= 18) + column existence, NOT the exact latest (a later migration would raise the version while
    // this column persists); gateway_plan.rs pins the exact latest for the double-bump guard.
    let (_d, path) = temp_db();
    let store = open(&path);
    assert!(
        store.user_version().unwrap() >= 18,
        "MIGRATION_18 applied (v18+)"
    );

    let conn = open_read_only(&path).unwrap();
    let has_col: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('proj_integration_connection') WHERE name=?1",
            ["live_writes_enabled"],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        has_col, 1,
        "MIGRATION_18 must add the live_writes_enabled column to proj_integration_connection"
    );
}

// ---- P4.7 (083 C3) — the SqliteLiveWritesGate live read (the per-owner toggle the github clients consult)

#[test]
fn test_sqlite_live_writes_gate_reads_per_account_toggle() {
    use nexusopsd::integrations::auth::LiveWritesGate;
    use nexusopsd::integrations::connections::SqliteLiveWritesGate;

    // spec(C3 / fail-closed): the gate reads live_writes_enabled keyed by (provider, account). Default OFF
    // on registration; flips ON via IntegrationLiveWritesSet; a DIFFERENT/unknown account → false
    // (fail-closed — never default-ON, so the authed client stays unauth for an un-toggled owner).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(
        &mut store,
        &conn_payload("conn_octo", Provider::Github, Some("octocat")),
    );
    let gate = SqliteLiveWritesGate::new(path.clone());

    // freshly registered → OFF (the default-OFF gate).
    assert!(
        !gate.is_enabled_for_account(Provider::Github, "octocat"),
        "a freshly-registered connection defaults to live-writes OFF"
    );
    // an UNKNOWN account → false (no row → fail-closed).
    assert!(
        !gate.is_enabled_for_account(Provider::Github, "nobody"),
        "an unknown account → false (fail-closed, never default-ON)"
    );

    // flip ON for conn_octo (account=octocat).
    append_live_writes_set(&mut store, "conn_octo", true);
    assert!(
        gate.is_enabled_for_account(Provider::Github, "octocat"),
        "toggle ON for octocat's connection → the gate reads enabled"
    );
    // a divergent account is STILL false (per-account; octocat's ON doesn't enable another owner).
    assert!(
        !gate.is_enabled_for_account(Provider::Github, "other-corp"),
        "octocat's ON does NOT enable a divergent owner (per-account toggle)"
    );

    // flip OFF → the gate reads disabled again (derive-from-event).
    append_live_writes_set(&mut store, "conn_octo", false);
    assert!(
        !gate.is_enabled_for_account(Provider::Github, "octocat"),
        "toggle OFF → the gate reads disabled"
    );
}

#[test]
fn test_sqlite_live_writes_gate_null_account_never_matches() {
    use nexusopsd::integrations::auth::LiveWritesGate;
    use nexusopsd::integrations::connections::SqliteLiveWritesGate;

    // spec(C3 / fail-closed): a connection registered WITHOUT an account (account=NULL) can NEVER match the
    // gate (`WHERE account = ?2` with a NULL column is never TRUE) — even with its toggle flipped ON. So an
    // account-less connection is permanently fail-closed (unauth). Documents/pins the intentional posture.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    append_connection(
        &mut store,
        &conn_payload("conn_noacct", Provider::Github, None),
    );
    append_live_writes_set(&mut store, "conn_noacct", true); // toggle ON, but account is NULL

    let gate = SqliteLiveWritesGate::new(path.clone());
    assert!(
        !gate.is_enabled_for_account(Provider::Github, "octocat"),
        "an account-less connection never matches the per-account gate (fail-closed, even toggled ON)"
    );
    assert!(
        !gate.is_enabled_for_account(Provider::Github, ""),
        "an empty account string also never matches (no NULL=NULL match)"
    );
}

//! Brief 093 — W1-prof: the `get_execution_profiles` read RPC (§6.1) serving a typed, SECRET-FREE
//! `ProfileRow` list from the §2.8 `execution_profiles` registry — the cockpit profile-picker read
//! surface (unblocks `session.profile_change`). §15 #4: the keychain POINTER/secret is NEVER served
//! (only the derived `has_credential` bool). `read_execution_profile_rows` is the pub testable core
//! (the `read_*_typed` family precedent); the dispatch reachability pin lives in `tests/ipc.rs` (the
//! `serve_connection` harness). The §15 keychain-never-served pin (test 2) is the load-bearing assertion.

use nexusops_shared::ids::{ExecutionProfileId, WorkspaceId};
use nexusops_shared::ipc::IpcErrorCode;
use nexusops_shared::status::ExecutionProfile;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;
use nexusopsd::ipc::read_execution_profile_rows;
use nexusopsd::profiles::{self, ProfileSpec};

const OCCURRED_AT: &str = "2026-06-26T00:00:00Z";

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(OCCURRED_AT)),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

#[test]
fn get_execution_profiles_returns_seeded_default() {
    // spec(§2.8) — after the cold-start seed, the RPC returns >=1 ProfileRow incl. the default
    // (is_default == true). The default = the FIRST ExecutionProfileRegistered (the seed, LESSON 62).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let default_id = {
        let mut store = open(&path);
        profiles::seed_default_profile(&mut store, OCCURRED_AT).expect("seed default")
    };
    let rows = read_execution_profile_rows(&path).expect("read rows");
    assert!(!rows.is_empty(), "the seeded default is returned");
    let default = rows
        .iter()
        .find(|r| r.execution_profile_id == default_id.as_str())
        .expect("the default profile is present");
    assert!(
        default.is_default,
        "the seeded default is flagged is_default"
    );
    assert!(
        !default.has_credential,
        "the non-secret seed has no credential (keychain_ref None)"
    );
    assert_eq!(
        rows.iter().filter(|r| r.is_default).count(),
        1,
        "exactly one default across the list (the seed)"
    );
}

#[test]
fn profile_row_never_serves_keychain_ref_or_secret() {
    // spec(§15 #4) — the LOAD-BEARING pin: register a profile with keychain_ref = Some(POINTER); the
    // serialized ProfileRow has NO keychain_ref/secret field; the credential state is exposed ONLY as
    // has_credential == true (the POINTER-never-served posture, LESSON 49/64/65).
    //
    // ALSO pins the `is_default` DISCRIMINATION in a multi-profile case (≥2 rows): the SEED (the FIRST
    // ExecutionProfileRegistered) is is_default=true, the later registered profile is is_default=false,
    // and exactly ONE row is default — a derivation that flagged every/the-wrong row would pass tests
    // 1/5 (single-profile) but fails HERE (the first-event provenance, the SqliteProfileLookup::default_id source).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let sentinel_ref = "nexusops/profile/SENTINEL_KEYCHAIN_REF";
    let id = ExecutionProfileId::new();
    let default_id = {
        let mut store = open(&path);
        // (a) the cold-start seed FIRST → the default (is_default=true; non-secret → has_credential=false).
        let default_id =
            profiles::seed_default_profile(&mut store, OCCURRED_AT).expect("seed default");
        // (b) a SECOND profile carrying a keychain_ref POINTER → has_credential=true, is_default=false.
        let spec = ProfileSpec {
            workspace_id: WorkspaceId::system(),
            provider: "anthropic".to_string(),
            harness: "claude_code".to_string(),
            model: Some("claude-3.7".to_string()),
            account_alias: Some("work".to_string()),
            keychain_ref: Some(sentinel_ref.to_string()),
            usage_policy_json: None,
            status: ExecutionProfile::Available,
        };
        profiles::register_profile(&mut store, &id, &spec, OCCURRED_AT).expect("register profile");
        default_id
    };
    let rows = read_execution_profile_rows(&path).expect("read rows");
    let row = rows
        .iter()
        .find(|r| r.execution_profile_id == id.as_str())
        .expect("the registered profile is present");
    assert!(
        row.has_credential,
        "a profile with a keychain_ref → has_credential true"
    );
    // the serialized row carries NO keychain_ref/secret field + never the POINTER value (§15 #4).
    let json = serde_json::to_string(row).expect("serialize ProfileRow");
    assert!(
        !json.contains("keychain_ref"),
        "no keychain_ref field on the wire row"
    );
    assert!(
        !json.contains("SENTINEL_KEYCHAIN_REF"),
        "the keychain POINTER value never appears in the serialized row"
    );
    assert!(!json.contains("secret"), "no secret field on the wire row");

    // is_default discrimination (≥2 profiles): the registered profile is NOT default; the seed IS.
    assert!(
        !row.is_default,
        "a later-registered profile is NOT the default (it is not the first ExecutionProfileRegistered)"
    );
    let seed = rows
        .iter()
        .find(|r| r.execution_profile_id == default_id.as_str())
        .expect("the seeded default is present");
    assert!(
        seed.is_default,
        "the seed (first ExecutionProfileRegistered) IS the default"
    );
    assert_eq!(
        rows.iter().filter(|r| r.is_default).count(),
        1,
        "exactly ONE default among >=2 profiles (no flag-everything derivation bug)"
    );
}

#[test]
fn get_execution_profiles_empty_registry_returns_empty_list() {
    // spec(§6.1) — an empty registry → [] (a valid result, not an error). Opening the store runs
    // migrations (execution_profiles exists) but seeds NOTHING (the seed is a cold-start step, not open()).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    {
        let _store = open(&path);
    }
    let rows = read_execution_profile_rows(&path).expect("read rows");
    assert!(rows.is_empty(), "no profiles → empty list, not an error");
}

#[test]
fn get_execution_profiles_no_default_event_degrades_soft() {
    // spec(LESSON 30) — the INTENTIONAL soft-degrade (distinct from the row-data fail-closed below): when
    // there is NO `ExecutionProfileRegistered` seed event, `is_default` resolves to None → the rows are
    // STILL served (not fail-closed) with NONE flagged default. `is_default` is a non-load-bearing UI
    // pre-select hint — its absence never blanks the picker, and it can never flag the WRONG default.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    {
        let _store = open(&path); // migrations create execution_profiles; drop before the raw write.
    }
    // TEST-FIXTURE-ONLY raw writable conn — seed a VALID registry row but NO matching event (production
    // always emits the event via register_profile; this isolates the "no seed event" degrade path).
    let conn = rusqlite::Connection::open(&path).expect("writable conn");
    conn.execute(
        "INSERT INTO execution_profiles
           (execution_profile_id, workspace_id, provider, harness, model, account_alias,
            keychain_ref, usage_policy_json, status, created_at)
         VALUES (?1, ?2, 'anthropic', 'claude_code', NULL, NULL, NULL, NULL, 'available', ?3)",
        rusqlite::params![
            ExecutionProfileId::new().as_str(),
            WorkspaceId::system().as_str(),
            OCCURRED_AT
        ],
    )
    .expect("insert valid fixture row");
    drop(conn);
    let rows = read_execution_profile_rows(&path)
        .expect("rows still served (soft-degrade, not fail-closed)");
    assert_eq!(
        rows.len(),
        1,
        "the registry row is still served despite no seed event"
    );
    assert!(
        rows.iter().all(|r| !r.is_default),
        "no row is flagged default when there is no ExecutionProfileRegistered seed event"
    );
}

#[test]
fn get_execution_profiles_corrupt_row_fails_closed() {
    // spec(§6.1) — a row whose status TEXT is not a valid §5.1 ExecutionProfile wire value is an
    // integrity error → the WHOLE read fails closed (InternalError), never a silent partial list (the
    // LESSON-37 typed-serve precedent). Seeds a corrupt row via a raw fixture conn (production goes
    // through register_profile, which only ever writes a valid §5.1 status).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    {
        let _store = open(&path); // migrations create execution_profiles; drop before the raw write.
    }
    // TEST-FIXTURE-ONLY raw writable conn — NOT a production pattern (single-writer is the actor).
    let conn = rusqlite::Connection::open(&path).expect("writable conn");
    conn.execute(
        "INSERT INTO execution_profiles
           (execution_profile_id, workspace_id, provider, harness, model, account_alias,
            keychain_ref, usage_policy_json, status, created_at)
         VALUES (?1, ?2, 'anthropic', 'claude_code', NULL, NULL, NULL, NULL, 'not_a_real_status', ?3)",
        rusqlite::params![
            ExecutionProfileId::new().as_str(),
            WorkspaceId::system().as_str(),
            OCCURRED_AT
        ],
    )
    .expect("insert corrupt fixture row");
    drop(conn);
    let err = read_execution_profile_rows(&path).expect_err("a corrupt status row fails closed");
    assert_eq!(
        err,
        IpcErrorCode::InternalError,
        "fail-closed on a mis-typed registry row (no silent partial list)"
    );
}

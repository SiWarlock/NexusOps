//! P5.3a — the `execution_profiles` durable registry (Option B, DATA_MODEL §2.8): the canonical-row
//! register-mutator (LESSON-16 dual-gate {row + audit event}, atomic + fail-closed), the §15 #4
//! defense-in-depth row redaction, the rebuild-does-NOT-clear invariant (NOT in `REBUILD_TABLES`,
//! LESSON 17/48), and the cold-start register-if-absent seed-default (idempotent across restarts).
//!
//! These pin the daemon's FIRST DATA_MODEL-2.8 canonical OBJECT registry (the twice-deferred
//! `register_project`/`integration.connect` row write, now BUILT). The §15 #8 resolve-at-start /
//! fail-closed-on-unknown lives in `session_executor.rs`. The keychain SECRET write + the startup
//! self-test are 5.3b (this slice is non-secret — `keychain_ref` is a POINTER only by construction).

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, EventStore, PrefixRedactor};
use nexusopsd::fault::{arm, FaultPoint};
use nexusopsd::idgen::UlidGen;
use nexusopsd::profiles::{self, ProfileSpec};

use nexusops_shared::actor::ActorType;
use nexusops_shared::events::ExecutionProfileRegistered;
use nexusops_shared::ids::{ExecutionProfileId, WorkspaceId};
use nexusops_shared::status::ExecutionProfile;

const OCCURRED_AT: &str = "2026-06-21T00:00:00Z";

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(OCCURRED_AT)),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// The non-secret default profile shape (Q2 default vote: claude-first, PTY-primary cat-4). The
/// SECRET/account binding lands in 5.3b — `keychain_ref` is a nullable POINTER (None here).
fn default_spec() -> ProfileSpec {
    ProfileSpec {
        workspace_id: WorkspaceId::system(),
        provider: "anthropic".to_string(),
        harness: "claude_code".to_string(),
        model: None,
        account_alias: None,
        keychain_ref: None,
        usage_policy_json: None,
        status: ExecutionProfile::Available,
    }
}

fn count_rows(path: &std::path::Path, sql: &str) -> i64 {
    let conn = open_read_only(path).expect("read-only conn");
    conn.query_row(sql, [], |r| r.get(0)).expect("count query")
}

fn registered_event_count(store: &EventStore) -> usize {
    store
        .read_all()
        .expect("read_all")
        .iter()
        .filter(|e| e.event_type == ExecutionProfileRegistered::EVENT_TYPE)
        .count()
}

#[test]
fn migration_16_creates_execution_profiles_floor() {
    // spec(LESSON 50) — the per-migration FLOOR pin (not exact-latest; that single pin is gateway_plan):
    // after open, the user_version is AT LEAST 16 and the `execution_profiles` table exists.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let store = open(&path);
    assert!(
        store.user_version().unwrap() >= 16,
        "MIGRATION_16 raises the user_version floor to >= 16"
    );
    let table_present = count_rows(
        &path,
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='execution_profiles'",
    );
    assert_eq!(
        table_present, 1,
        "MIGRATION_16 created the execution_profiles table"
    );
}

#[test]
fn register_writes_canonical_row_and_event_atomically() {
    // spec(LESSON 16 / DATA_MODEL §2.8) — ONE register → EXACTLY one canonical `execution_profiles`
    // row AND exactly one `ExecutionProfileRegistered` audit event (the dual-gate, same txn); the row
    // PK matches the registered id.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = ExecutionProfileId::new();
    profiles::register_profile(&mut store, &id, &default_spec(), OCCURRED_AT)
        .expect("register a profile");

    assert_eq!(
        count_rows(&path, "SELECT count(*) FROM execution_profiles"),
        1,
        "exactly one canonical row written"
    );
    assert_eq!(
        registered_event_count(&store),
        1,
        "exactly one ExecutionProfileRegistered audit event appended (the trail)"
    );
    let conn = open_read_only(&path).expect("read-only conn");
    let row_id: String = conn
        .query_row(
            "SELECT execution_profile_id FROM execution_profiles",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        row_id,
        id.as_str(),
        "the canonical row PK is the registered id"
    );
}

#[test]
fn register_fails_closed_on_event_write_fault() {
    // spec(LESSON 16 / §15 #5) — an injected audit-event-write fault → the WHOLE register rolls back:
    // NO canonical row AND no event persist (the {row + event} are one atomic txn; the row is durable
    // ONLY if its audit trail is — fail-closed on audit-write).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = ExecutionProfileId::new();

    arm(FaultPoint::RegistryEventWrite); // the next register's event append fails
    let res = profiles::register_profile(&mut store, &id, &default_spec(), OCCURRED_AT);
    assert!(
        res.is_err(),
        "an event-write fault fails the register closed"
    );

    assert_eq!(
        count_rows(&path, "SELECT count(*) FROM execution_profiles"),
        0,
        "LESSON-16 fail-closed: event-write fault → NO row persisted (rollback)"
    );
    assert_eq!(
        registered_event_count(&store),
        0,
        "no audit event persisted either (the txn rolled back wholly)"
    );
}

#[test]
fn row_payload_is_redacted_before_insert() {
    // spec(§15 #4) — defense-in-depth: a secret-shaped value slipped into a row field is masked by the
    // canonical Redactor BEFORE the INSERT (the row carries only a `keychain_ref` POINTER by
    // construction; this proves the dual-gate, not a real secret path).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = ExecutionProfileId::new();
    let secret = "ghp_0123456789abcdefghijklmnopqrstuvwxyzABCD";
    let mut spec = default_spec();
    spec.account_alias = Some(secret.to_string());
    profiles::register_profile(&mut store, &id, &spec, OCCURRED_AT).expect("register");

    let conn = open_read_only(&path).expect("read-only conn");
    let stored: Option<String> = conn
        .query_row("SELECT account_alias FROM execution_profiles", [], |r| {
            r.get(0)
        })
        .unwrap();
    let stored = stored.expect("account_alias persisted");
    assert!(
        !stored.contains("ghp_0123456789"),
        "the secret-shaped value must NOT persist raw in the row (§15 #4)"
    );
    assert!(
        stored.contains("[REDACTED]"),
        "the canonical Redactor masked the row value before INSERT: {stored}"
    );
}

#[test]
fn rebuild_does_not_clear_execution_profiles() {
    // spec(LESSON 17 / DATA_MODEL §2.8) — `execution_profiles` is the canonical SOURCE OF TRUTH, NOT a
    // projection: it is NOT in `REBUILD_TABLES`, so a projection rebuild leaves the row intact (a
    // truncate-and-refold would EMPTY it — no projector folds ExecutionProfileRegistered).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);
    let id = ExecutionProfileId::new();
    profiles::register_profile(&mut store, &id, &default_spec(), OCCURRED_AT).expect("register");

    store.rebuild_projections().expect("rebuild projections");

    assert_eq!(
        count_rows(&path, "SELECT count(*) FROM execution_profiles"),
        1,
        "a rebuild does NOT clear the canonical registry (NOT in REBUILD_TABLES)"
    );
}

#[test]
fn seed_default_is_idempotent_across_restarts() {
    // spec(LESSON 10 — register-if-absent; sub-decision 1) — two cold-starts on the same db register
    // exactly ONE default profile: the 2nd reuses the 1st (same id, no duplicate row/event).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    let mut store = open(&path);

    let id1 = profiles::seed_default_profile(&mut store, OCCURRED_AT).expect("seed #1");
    let id2 = profiles::seed_default_profile(&mut store, OCCURRED_AT).expect("seed #2 (restart)");

    assert_eq!(
        id1.as_str(),
        id2.as_str(),
        "the 2nd cold-start REUSES the 1st default (register-if-absent)"
    );
    assert_eq!(
        count_rows(&path, "SELECT count(*) FROM execution_profiles"),
        1,
        "exactly ONE default row across restarts"
    );
    assert_eq!(
        registered_event_count(&store),
        1,
        "no duplicate seed event across restarts"
    );

    // the seed is a System-actor event in the reserved system workspace (mirrors register_device —
    // the daemon establishing its OWN registry substrate, NOT a policy-gated Gateway Action).
    let seed = store
        .read_all()
        .unwrap()
        .into_iter()
        .find(|e| e.event_type == "ExecutionProfileRegistered")
        .expect("the seed event is in the stream");
    assert_eq!(
        seed.actor_type,
        ActorType::System,
        "the cold-start seed is a System-actor event (LESSON 10)"
    );
    assert_eq!(
        seed.workspace_id,
        WorkspaceId::system(),
        "the seed lives in the reserved system workspace"
    );
}

//! 075d — the durable §15-redacted `ScrollbackStore` (`FileScrollbackStore`). SAFETY-CORE tests
//! (commit 1): the store contract + the 🔴 §15 redaction-before-persist + fail-closed + perms + the
//! end-to-end `Replayed` path. (The lifecycle tests — evict/sweep/backstop — are commit 2.)

use std::collections::HashSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{RedactionStatus, Sensitivity, SourceType};
use nexusops_shared::events::SessionStarted;
use nexusops_shared::harness::ResumeMode;
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
use nexusops_shared::status::Session;

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    open_read_only, AppendIntent, EventStore, PrefixRedactor, RedactionOutcome, Redactor,
};
use nexusopsd::harness::resume::{decide_resume, ResumeInputs};
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::recovery::enumerate_recoverable_sessions;
use nexusopsd::scrollback::FileScrollbackStore;
use nexusopsd::terminal::{HeadlessVt, ScrollbackStore, VtSnapshot};

// ---- scaffold -----------------------------------------------------------------------------------

/// A fresh 0700-able temp dir for a `FileScrollbackStore`.
fn store_in(dir: &Path) -> FileScrollbackStore {
    FileScrollbackStore::new(dir.join("scrollback"), Arc::new(PrefixRedactor))
        .expect("0700 scrollback dir")
}

/// Build a `VtSnapshot` by feeding `stream` into a fresh `HeadlessVt`.
fn snapshot_from(dims: (u16, u16, usize), stream: &[u8]) -> VtSnapshot {
    let mut vt = HeadlessVt::new(dims.0, dims.1, dims.2);
    vt.process(stream);
    vt.snapshot()
}

/// A Redactor that REFUSES to redact (returns `Unredacted`) — drives the §15 fail-closed path.
struct FailingRedactor;
impl Redactor for FailingRedactor {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        RedactionOutcome {
            status: RedactionStatus::Unredacted,
            payload_json: payload_json.to_string(),
            engine_version: "test-failing".to_string(),
            quarantine: None,
        }
    }
}

fn open_store(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .unwrap()
}

/// Commit a `SessionStarted` so the projector folds a recoverable `proj_session` row (test 9).
fn commit_session_started(store: &mut EventStore, session_id: &SessionId) {
    let payload = serde_json::to_string(&SessionStarted {
        status: Session::Active,
        harness: None,
        model: None,
        display_name: None,
        execution_profile_id: None,
    })
    .unwrap();
    let intent = AppendIntent {
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
        payload_json: payload,
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: Some(ProjectId::new()),
        session_id: Some(session_id.clone()),
        agent_team_id: None,
        visibility: None,
        action_request_id: None,
        approval_id: None,
        causation_id: None,
    };
    store.append(intent).unwrap();
}

// ---- Test 1 — the store contract (round-trip; plain-text) ---------------------------------------

#[test]
fn test_durable_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let sid = SessionId::new();
    let snap = snapshot_from((24, 80, 1000), b"hello durable\r\nsecond line");
    let expected_screen = HeadlessVt::from_snapshot(&snap).screen_contents();

    assert!(store.load(&sid).is_none(), "absent → None");
    store.save(&sid, &snap);
    let loaded = store.load(&sid).expect("a saved sidecar loads");
    assert_eq!(
        HeadlessVt::from_snapshot(&loaded).screen_contents(),
        expected_screen,
        "the round-tripped (plain-text) screen matches"
    );
}

/// 075d Test 1b — the SCROLLBACK round-trips: save a snapshot WITH scrollback → load → the row count
/// AND the oldest scrolled-off line survive save→`from_plain`. (Test 1's 24-row screen has zero
/// scrollback, so it can't exercise the reconstruction's flush mechanics — this does.)
#[test]
fn test_durable_scrollback_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let sid = SessionId::new();
    // 5 lines on a 2-row screen → 3 rows scroll into scrollback (none are secrets → unmasked).
    let snap = snapshot_from((2, 10, 100), b"alpha\r\nbravo\r\ncharlie\r\ndelta\r\necho");
    let original_sb = snap.scrollback_rows();
    assert!(original_sb > 0, "the fixture overflows into scrollback");

    store.save(&sid, &snap);
    let loaded = store.load(&sid).expect("loads");
    assert_eq!(
        loaded.scrollback_rows(),
        original_sb,
        "the scrollback row count survives save → load → from_plain"
    );
    let mut loaded_vt = HeadlessVt::from_snapshot(&loaded);
    let oldest = loaded_vt.view_at_scrollback(loaded.scrollback_rows());
    assert!(
        oldest.contains("alpha"),
        "the oldest scrolled-off line survives the round-trip: {oldest:?}"
    );
}

// ---- Test 2 🔴 — redaction-before-persist (realistic terminal-secret sample) --------------------

#[test]
fn test_redaction_before_persist() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let sid = SessionId::new();

    // A REALISTIC multi-line command-output blob the way an agent echoes secrets — sk-, AKIA, and a
    // KEY=value export — across visible screen AND scrollback (a small screen → some lines scroll off).
    // Terminal scrollback is a NEW input domain for the Redactor; this proves recall holds on it.
    let blob = b"$ env | grep -iE 'key|token'\r\n\
                 OPENAI_API_KEY=sk-proj-SECRETkey1234567890abcdefGHIJKLM\r\n\
                 AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\r\n\
                 $ cat .env\r\n\
                 DEPLOY_TOKEN=tok_aB3xK9mP2qR7sT1vW5yZ8nL4QjcD6eF\r\n\
                 $ echo done";
    let snap = snapshot_from((4, 80, 1000), blob);
    store.save(&sid, &snap);

    // read the RAW sidecar bytes on disk and assert NONE of the raw secrets are present (all masked).
    let sidecar = dir
        .path()
        .join("scrollback")
        .join(format!("{}.json", sid.as_str()));
    let on_disk = std::fs::read_to_string(&sidecar).expect("the sidecar exists");
    for secret in [
        "sk-proj-SECRETkey1234567890abcdefGHIJKLM",
        "AKIAIOSFODNN7EXAMPLE",
        "tok_aB3xK9mP2qR7sT1vW5yZ8nL4QjcD6eF",
    ] {
        assert!(
            !on_disk.contains(secret),
            "🔴 §15: the raw secret `{secret}` must NEVER reach the sidecar (redaction-before-persist)"
        );
    }
    assert!(
        on_disk.contains("REDACTED"),
        "something WAS masked (the secrets were detected + redacted)"
    );
}

// ---- Test 3 🔴 — redaction failure → fail-closed (no sidecar) ------------------------------------

#[test]
fn test_redaction_failure_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    // a store whose Redactor REFUSES (returns Unredacted) → save must write NOTHING.
    let store = FileScrollbackStore::new(dir.path().join("scrollback"), Arc::new(FailingRedactor))
        .expect("0700 dir");
    let sid = SessionId::new();
    let snap = snapshot_from(
        (24, 80, 1000),
        b"OPENAI_API_KEY=sk-proj-SECRETkey1234567890abc",
    );

    store.save(&sid, &snap);

    let sidecar = dir
        .path()
        .join("scrollback")
        .join(format!("{}.json", sid.as_str()));
    assert!(
        !sidecar.exists(),
        "🔴 §15 fail-closed: redaction did not complete → NO sidecar written (never persist unredacted)"
    );
    assert!(
        store.load(&sid).is_none(),
        "→ load is None → Relaunched (safe)"
    );
}

// ---- Test 4 — perms 0700 dir / 0600 file --------------------------------------------------------

#[test]
fn test_sidecar_perms_0700_0600() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let sid = SessionId::new();
    store.save(&sid, &snapshot_from((24, 80, 1000), b"perms check"));

    let scrollback_dir = dir.path().join("scrollback");
    let dir_mode = std::fs::metadata(&scrollback_dir)
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(dir_mode, 0o700, "the sidecar dir is 0700 (§15 #11)");

    let sidecar = scrollback_dir.join(format!("{}.json", sid.as_str()));
    let file_mode = std::fs::metadata(&sidecar).unwrap().permissions().mode() & 0o777;
    assert_eq!(file_mode, 0o600, "the sidecar file is 0600 (§15 #11)");
}

// ---- Test 5 — absent / corrupt → None (fail-safe, no panic) -------------------------------------

#[test]
fn test_load_absent_or_corrupt_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());

    assert!(
        store.load(&SessionId::new()).is_none(),
        "absent sidecar → None"
    );

    // a truncated/garbage sidecar at the expected path → None (no panic).
    let sid = SessionId::new();
    let sidecar = dir
        .path()
        .join("scrollback")
        .join(format!("{}.json", sid.as_str()));
    std::fs::write(&sidecar, b"{ this is not valid json \x00\xff").unwrap();
    assert!(
        store.load(&sid).is_none(),
        "corrupt sidecar → None (fail-safe)"
    );

    // a VALID-JSON sidecar with an UNKNOWN version → None (the migration guard, load-bearing for 075d+).
    let sid2 = SessionId::new();
    let future = dir
        .path()
        .join("scrollback")
        .join(format!("{}.json", sid2.as_str()));
    std::fs::write(
        &future,
        br#"{"version":99,"rows":24,"cols":80,"screen_text":"x","scrollback_rows":[]}"#,
    )
    .unwrap();
    assert!(
        store.load(&sid2).is_none(),
        "an unknown sidecar version → None (forward-compat migration guard)"
    );
}

// ---- Test 9 — end-to-end: the real store feeds the recovery consumer → Replayed -----------------

#[test]
fn test_recovery_end_to_end_replays() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = store_in(store_dir.path());

    let db = tempfile::tempdir().unwrap();
    let db_path = db.path().join("nexusops.db");
    let mut event_store = open_store(&db_path);
    let sid = SessionId::new();
    commit_session_started(&mut event_store, &sid);

    // save a scrollback-bearing snapshot through the REAL store (5 lines on a 2-row screen → scroll).
    let snap = snapshot_from((2, 10, 100), b"line0\r\nline1\r\nline2\r\nline3\r\nline4");
    assert!(snap.has_restorable_content());
    store.save(&sid, &snap);

    let conn = open_read_only(&db_path).unwrap();
    let recoverable = enumerate_recoverable_sessions(&conn, &store).unwrap();
    let r = recoverable
        .iter()
        .find(|r| r.session_id == sid)
        .expect("the session");
    assert!(
        r.has_scrollback,
        "the durable store fed has_scrollback=true to the recovery consumer"
    );

    let result = decide_resume(&ResumeInputs {
        broker_has_live_session: false,
        supports_resume: false,
        has_resume_handle: false,
        has_scrollback: r.has_scrollback,
        replayed_event_count: r.replayed_event_count,
    });
    assert_eq!(
        result.mode,
        ResumeMode::Replayed,
        "the real durable store makes Replayed-after-restart reachable (§8/§17)"
    );
}

// ============================================================================================
// 075d/2 — LIFECYCLE (evict / startup orphan-sweep / size+age backstop). NON-safety (deletions only).
// ============================================================================================

/// Test 6 — `evict` removes the sidecar (terminal reap → not recoverable) + is idempotent.
#[test]
fn test_evict_removes_sidecar() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let sid = SessionId::new();
    store.save(&sid, &snapshot_from((24, 80, 1000), b"evict me"));
    let sidecar = dir
        .path()
        .join("scrollback")
        .join(format!("{}.json", sid.as_str()));
    assert!(sidecar.exists());

    store.evict(&sid);
    assert!(!sidecar.exists(), "evict removes the sidecar");
    assert!(store.load(&sid).is_none(), "→ load is None");
    store.evict(&sid); // idempotent — a second evict is a no-op (no panic).
}

/// Test 7 — the startup orphan-sweep removes sidecars with no matching `proj_session` (+ leftover
/// `.tmp`), and KEEPS a live session's sidecar.
#[test]
fn test_startup_orphan_sweep() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let scrollback_dir = dir.path().join("scrollback");
    let live = SessionId::new();
    let orphan = SessionId::new();
    store.save(&live, &snapshot_from((24, 80, 1000), b"live session"));
    store.save(&orphan, &snapshot_from((24, 80, 1000), b"orphan session"));
    // a leftover temp from a crashed write.
    std::fs::write(scrollback_dir.join("crashed.json.tmp"), b"partial").unwrap();

    let mut known = HashSet::new();
    known.insert(live.clone());
    store.sweep_orphans(&known);

    assert!(
        store.load(&live).is_some(),
        "the live session's sidecar is KEPT"
    );
    assert!(store.load(&orphan).is_none(), "the orphan sidecar is swept");
    assert!(
        !scrollback_dir.join("crashed.json.tmp").exists(),
        "the leftover .tmp is swept"
    );
}

/// Test 8 — the size/age backstop evicts an over-size sidecar AND an over-age one, keeping a fresh
/// under-cap one.
#[test]
fn test_size_age_backstop() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let scrollback_dir = dir.path().join("scrollback");

    // SIZE — a tiny per-sidecar cap evicts the (>1-byte) sidecar.
    let big = SessionId::new();
    store.save(
        &big,
        &snapshot_from((24, 80, 1000), b"over the tiny size cap"),
    );
    store.enforce_backstop_with(1, u64::MAX, Duration::from_secs(86400));
    assert!(
        store.load(&big).is_none(),
        "a sidecar over the per-sidecar size cap is evicted"
    );

    // AGE — a sidecar older than the TTL is evicted (set its mtime 10 days into the past).
    let old = SessionId::new();
    store.save(&old, &snapshot_from((24, 80, 1000), b"stale session"));
    let old_path = scrollback_dir.join(format!("{}.json", old.as_str()));
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(&old_path)
        .unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(10 * 86400))
        .unwrap();
    drop(f);
    store.enforce_backstop_with(u64::MAX, u64::MAX, Duration::from_secs(7 * 86400));
    assert!(
        store.load(&old).is_none(),
        "a sidecar older than the age TTL is evicted"
    );

    // a FRESH under-cap sidecar survives the backstop.
    let fresh = SessionId::new();
    store.save(&fresh, &snapshot_from((24, 80, 1000), b"fresh session"));
    store.enforce_backstop_with(u64::MAX, u64::MAX, Duration::from_secs(7 * 86400));
    assert!(
        store.load(&fresh).is_some(),
        "a fresh, under-cap sidecar survives the backstop"
    );
}

/// Test 8b — the DIR-CAP backstop evicts OLDEST-first until under the total cap (the most intricate
/// `enforce_backstop_with` sub-path).
#[test]
fn test_dir_cap_evicts_oldest_first() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let scrollback_dir = dir.path().join("scrollback");
    let older = SessionId::new();
    let newer = SessionId::new();
    // identical-length content → identical sidecar byte sizes.
    store.save(&older, &snapshot_from((24, 80, 1000), b"session A content"));
    store.save(&newer, &snapshot_from((24, 80, 1000), b"session B content"));

    // make `older` actually older.
    let older_path = scrollback_dir.join(format!("{}.json", older.as_str()));
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(&older_path)
        .unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
    drop(f);

    // a dir cap that fits ONE sidecar but not both → only the OLDEST is evicted.
    let one = std::fs::metadata(scrollback_dir.join(format!("{}.json", newer.as_str())))
        .unwrap()
        .len();
    store.enforce_backstop_with(u64::MAX, one + 10, Duration::from_secs(86400));
    assert!(
        store.load(&older).is_none(),
        "the OLDEST sidecar is evicted to get under the dir cap"
    );
    assert!(
        store.load(&newer).is_some(),
        "the newest sidecar is kept (under the cap after one eviction)"
    );
}

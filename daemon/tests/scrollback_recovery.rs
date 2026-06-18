//! 075c — the scrollback producer→`ScrollbackStore`→recovery wiring (P3.4-VT; ARCHITECTURE §8/§8.1/
//! §17 survival ladder; the `Replayed` rung).
//!
//! Commit 1 (this file's first tests): the `ScrollbackStore` seam + `FakeScrollbackStore` + the
//! recovery consumer (`enumerate_recoverable_sessions` reads the store → `has_replayable_snapshot`/
//! `replayed_event_count` → `decide_resume`→`Replayed` reachable). Commit 2 appends the producer-tap
//! tests (the `SessionActor` read-pump → per-session `HeadlessVt` → save).

use std::path::Path;

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::events::SessionStarted;
use nexusops_shared::harness::ResumeMode;
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
use nexusops_shared::status::Session;

use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, AppendIntent, EventStore, PrefixRedactor};
use nexusopsd::harness::resume::{decide_resume, ResumeInputs};
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::recovery::enumerate_recoverable_sessions;
use nexusopsd::terminal::{FakeScrollbackStore, HeadlessVt, ScrollbackStore, VtSnapshot};

// ---- scaffold (the recovery_restart_wiring.rs pattern; each integration test file is its own crate) -

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
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

/// Commit a `SessionStarted` so the projector folds a `proj_session` row (the recoverable set the
/// enumerator reads). `Active` is non-terminal → recoverable.
fn commit_session_started(store: &mut EventStore, session_id: &SessionId, status: Session) {
    let payload = serde_json::to_string(&SessionStarted {
        status,
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
        causation_id: None,
        approval_id: None,
    };
    store.append(intent).unwrap();
}

/// Build a `VtSnapshot` by feeding `stream` into a fresh `HeadlessVt`.
fn snapshot_from(dims: (u16, u16, usize), stream: &[u8]) -> VtSnapshot {
    let mut vt = HeadlessVt::new(dims.0, dims.1, dims.2);
    vt.process(stream);
    vt.snapshot()
}

/// Build the `ResumeInputs` the recovery consumer would (broker has no survivor → the scrollback axis
/// decides), so `decide_resume` exercises the SAME mapping `recover_sessions_on_restart` uses.
fn inputs_for(r: &nexusopsd::session::recovery::RecoverableSession) -> ResumeInputs {
    ResumeInputs {
        broker_has_live_session: false,
        supports_resume: r.supports_resume,
        has_resume_handle: r.has_resume_handle,
        has_replayable_snapshot: r.has_replayable_snapshot,
        replayed_event_count: r.replayed_event_count,
    }
}

// ---- Test 1 — the seam contract (round-trip) ----------------------------------------------------

#[test]
fn test_scrollback_store_fake_round_trip() {
    let store = FakeScrollbackStore::new();
    let sid = SessionId::new();
    let snap = snapshot_from((24, 80, 100), b"hello");

    assert!(store.load(&sid).is_none(), "unknown session → None");
    store.save(&sid, &snap);
    assert_eq!(store.load(&sid), Some(snap), "save → load round-trips");
    assert!(
        store.load(&SessionId::new()).is_none(),
        "a different session id → None"
    );
}

// ---- Test 2 — a populated store drives the Replayed rung -----------------------------------------

#[test]
fn test_recovery_populated_store_replays() {
    let (_d, path) = temp_db();
    let mut store = open_store(&path);
    let sid = SessionId::new();
    commit_session_started(&mut store, &sid, Session::Active);
    let conn = open_read_only(&path).unwrap();

    let sb = FakeScrollbackStore::new();
    // 5 lines on a 2-row screen → 3 rows scroll off into scrollback.
    let snap = snapshot_from((2, 10, 100), b"line0\r\nline1\r\nline2\r\nline3\r\nline4");
    let expected = u64::try_from(snap.scrollback_rows()).unwrap();
    sb.save(&sid, &snap);

    let recoverable = enumerate_recoverable_sessions(&conn, &sb).unwrap();
    assert_eq!(recoverable.len(), 1);
    let r = &recoverable[0];
    assert!(
        r.has_replayable_snapshot,
        "a populated store → has_replayable_snapshot"
    );
    assert_eq!(
        r.replayed_event_count, expected,
        "replayed_event_count == the snapshot's scrollback row count"
    );

    let result = decide_resume(&inputs_for(r));
    assert_eq!(
        result.mode,
        ResumeMode::Replayed,
        "scrollback present → the Replayed rung (§8.1 / LESSON §36)"
    );
    assert_eq!(result.replayed_event_count, expected);
}

// ---- Test 3 — an empty store preserves today's Relaunched ----------------------------------------

#[test]
fn test_recovery_empty_store_relaunches() {
    let (_d, path) = temp_db();
    let mut store = open_store(&path);
    let sid = SessionId::new();
    commit_session_started(&mut store, &sid, Session::Active);
    let conn = open_read_only(&path).unwrap();

    let sb = FakeScrollbackStore::new(); // empty — no snapshot saved
    let recoverable = enumerate_recoverable_sessions(&conn, &sb).unwrap();
    let r = &recoverable[0];
    assert!(
        !r.has_replayable_snapshot,
        "empty store → no replayable snapshot"
    );
    assert_eq!(r.replayed_event_count, 0);

    let result = decide_resume(&inputs_for(r));
    assert_eq!(
        result.mode,
        ResumeMode::Relaunched,
        "no replayable snapshot → today's Relaunched rung (no regression, LESSON §38)"
    );
}

// ---- Test 4 — an alt-active snapshot still replays (the 075b design-input) -----------------------

#[test]
fn test_recovery_alt_active_snapshot_replays() {
    let (_d, path) = temp_db();
    let mut store = open_store(&path);
    let sid = SessionId::new();
    commit_session_started(&mut store, &sid, Session::Active);
    let conn = open_read_only(&path).unwrap();

    let sb = FakeScrollbackStore::new();
    // a mid-alt-screen session: ZERO scrollback rows, but a non-blank alt screen → restorable content.
    let snap = snapshot_from((10, 20, 100), b"normal text\r\n\x1b[?1049halt editor");
    assert_eq!(
        snap.scrollback_rows(),
        0,
        "an alt-active snapshot carries no scrollback rows"
    );
    assert!(
        snap.has_restorable_content(),
        "but it IS restorable — a non-blank alt screen to re-render"
    );
    sb.save(&sid, &snap);

    let recoverable = enumerate_recoverable_sessions(&conn, &sb).unwrap();
    let r = &recoverable[0];
    assert!(
        r.has_replayable_snapshot,
        "restorable content → has_replayable_snapshot=true even with 0 scrollback rows (keyed on has_restorable_content)"
    );
    assert_eq!(r.replayed_event_count, 0, "honest 0 for alt-active");

    let result = decide_resume(&inputs_for(r));
    assert_eq!(
        result.mode,
        ResumeMode::Replayed,
        "a re-renderable alt screen → Replayed, NOT Relaunched (§0.1 O-2)"
    );
}

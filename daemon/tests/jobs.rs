//! Phase 4.3 — background maintenance jobs: the WAL checkpointer + the session-staleness poller.
//! ARCHITECTURE §10 (the long-lived Tokio interval-task family — the `spawn_drainer`/`spawn_reaper`
//! neighbors), §17 (the daemon-owned failure-mode surfaces — the stale-by-age signal), §5.1 (Session
//! — `Stale` is time-derived, not event-folded). forbidden #3 / LESSON §9 (a WRITE pass rides the
//! single write-actor command path — NEVER a rogue writer; a derived signal is a live-read recompute).
//!
//! RED-first: `is_stale` / `recompute_staleness_once` / `spawn_wal_checkpointer` /
//! `spawn_staleness_poller` / `EventStore::wal_checkpoint` / `WalCheckpointMode` don't exist yet.

use std::path::{Path, PathBuf};
use std::time::Duration;

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::ids::{SessionId, WorkspaceId};
use nexusopsd::clock::{Clock, FixedClock};
use nexusopsd::eventstore::{
    open_read_only, AppendIntent, EventStore, PrefixRedactor, WalCheckpointMode,
};
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::{
    is_stale, recompute_staleness_once, spawn_staleness_poller, spawn_wal_checkpointer, WriteActor,
};

use std::sync::Arc;
use tokio::sync::watch;

const TS: &str = "2026-06-14T00:00:00Z";

fn temp_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open_store(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(TS)),
        Box::new(PrefixRedactor),
    )
    .unwrap()
}

fn stub_gateway() -> nexusopsd::gateway::Gateway {
    nexusopsd::gateway::Gateway::new(
        Box::new(nexusopsd::gateway::policy::StubPolicy),
        Box::new(nexusopsd::gateway::executor::StubExecutor),
    )
}

fn test_intent() -> AppendIntent {
    AppendIntent {
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: TS.to_string(),
        workspace_id: WorkspaceId::new(),
        actor_type: ActorType::User,
        actor_id: "u_1".to_string(),
        source_type: SourceType::DesktopUi,
        source_id: "src_1".to_string(),
        correlation_id: "corr_1".to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json: "{}".to_string(),
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

/// the `-wal` sidecar file for a db path.
fn wal_file(db: &Path) -> PathBuf {
    let mut s = db.as_os_str().to_os_string();
    s.push("-wal");
    PathBuf::from(s)
}

/// TEST-FIXTURE-ONLY: seed a `proj_session` row directly (production folds it from `SessionStarted`;
/// `last_heartbeat_at` has no event producer yet — the live writer is a 4.3-follow-on, brief Q4 — so a
/// staleness test simulates it). The store is dropped before this opens (clean WAL locking).
fn seed_session(conn: &rusqlite::Connection, sid: &str, status: &str, last_hb: Option<&str>) {
    conn.execute(
        "INSERT INTO proj_session (session_id, project_id, status, last_heartbeat_at, updated_at_seq) \
         VALUES (?1, 'proj_1', ?2, ?3, 1)",
        rusqlite::params![sid, status, last_hb],
    )
    .expect("seed proj_session row");
}

// ---- #1 — is_stale: the pure age derivation (§5.1 / §17) ----------------------------------------

#[test]
fn test_is_stale_by_age() {
    // spec(§5.1) spec(§17) — `is_stale(last_heartbeat_at, now, threshold_secs)` is the pure
    // stale-by-age derivation: age > threshold → stale. Strict `>` (the exact boundary is NOT stale);
    // a missing heartbeat (None) is NOT stale (brief Q4 — absence ≠ stale, no false-stale on a fresh
    // session); an unparseable heartbeat is NOT stale (can't prove age → never assert stale).
    // age 120s > 60 → stale.
    assert!(is_stale(Some(TS), "2026-06-14T00:02:00Z", 60));
    // age 30s < 60 → not stale.
    assert!(!is_stale(Some(TS), "2026-06-14T00:00:30Z", 60));
    // age == 60 exactly → NOT stale (strict greater-than).
    assert!(!is_stale(Some(TS), "2026-06-14T00:01:00Z", 60));
    // None heartbeat → NOT stale (Q4 default).
    assert!(!is_stale(None, "2026-06-14T00:02:00Z", 60));
    // unparseable heartbeat → NOT stale (cannot compute age).
    assert!(!is_stale(
        Some("not-a-timestamp"),
        "2026-06-14T00:02:00Z",
        60
    ));
    // a future-dated heartbeat (clock skew) → negative age → NOT stale.
    assert!(!is_stale(Some("2026-06-14T02:00:00Z"), TS, 60));
}

// ---- #2 — staleness is a live recompute, never an event-folded/stored value (§7.2 / §5.1) -------

#[test]
fn test_staleness_recomputed_live_not_replayed() {
    // spec(§7.2) spec(§5.1) — staleness is a LIVE recompute over `proj_session` (the worktree-status
    // live-read precedent), NOT a stored/event-folded value: the SAME rows flip not-stale → stale
    // purely as the poll clock advances (a stored bit would itself go stale, Q2). Only NON-terminal
    // sessions are candidates (a terminal `failed`/`completed` is never "stale"); a None heartbeat is
    // never stale.
    let (_d, path) = temp_db();
    drop(open_store(&path)); // create the schema, then close (clean WAL for the fixture writer)

    let a = SessionId::new();
    let b = SessionId::new();
    let c = SessionId::new();
    {
        let conn = rusqlite::Connection::open(&path).expect("writable fixture conn");
        seed_session(&conn, a.as_str(), "active", Some(TS)); // non-terminal, has heartbeat
        seed_session(&conn, b.as_str(), "failed", Some(TS)); // TERMINAL — excluded even when old
        seed_session(&conn, c.as_str(), "active", None); // non-terminal, NO heartbeat
    }

    let conn = open_read_only(&path).unwrap();

    // poll soon after the heartbeat (30s, threshold 60) → nothing stale.
    let fresh = recompute_staleness_once(&conn, "2026-06-14T00:00:30Z", 60).unwrap();
    assert!(
        fresh.is_empty(),
        "no session is stale 30s after a 60s-threshold heartbeat"
    );

    // poll an hour later → ONLY the live session with the old heartbeat (a) is stale. b is terminal
    // (excluded); c has no heartbeat (never stale). The flip on the SAME rows proves a LIVE recompute.
    let stale = recompute_staleness_once(&conn, "2026-06-14T01:00:00Z", 60).unwrap();
    let stale_ids: Vec<String> = stale.iter().map(|s| s.as_str().to_string()).collect();
    assert_eq!(
        stale_ids,
        vec![a.as_str().to_string()],
        "only the live, long-silent session is stale"
    );
}

// ---- #3 — a checkpoint pass bounds the WAL (§10) ------------------------------------------------

#[tokio::test]
async fn test_wal_checkpoint_once_bounds_wal() {
    // spec(§10) — a bounded checkpoint pass via the write-actor bounds the WAL. PASSIVE (the
    // production interval mode, brief Q3 — non-blocking) flushes every WAL frame into the main db;
    // TRUNCATE then bounds the `-wal` FILE itself to zero bytes. Both ride the single write-actor.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new(TS)),
        stub_gateway(),
    );
    let handle = actor.handle();

    // generate WAL frames (each append is a committed txn; autocheckpoint=1000 keeps them in the WAL).
    for _ in 0..20 {
        handle.append(test_intent()).await.unwrap();
    }

    // PASSIVE: not blocked (single writer, no contending reader) + every frame flushed.
    let s = handle
        .wal_checkpoint(WalCheckpointMode::Passive)
        .await
        .unwrap();
    assert_eq!(s.busy, 0, "PASSIVE checkpoint is not blocked");
    assert!(s.log_frames > 0, "the appends produced WAL frames");
    assert_eq!(
        s.checkpointed_frames, s.log_frames,
        "PASSIVE flushed every WAL frame into the db"
    );

    // TRUNCATE: bound the -wal file to zero bytes.
    let t = handle
        .wal_checkpoint(WalCheckpointMode::Truncate)
        .await
        .unwrap();
    assert_eq!(t.busy, 0, "TRUNCATE checkpoint is not blocked");
    let wal_len = std::fs::metadata(wal_file(&path))
        .map(|m| m.len())
        .unwrap_or(0);
    assert_eq!(wal_len, 0, "TRUNCATE bounds the WAL file to zero bytes");

    actor.shutdown().await;
}

// ---- #4 — the checkpoint rides the single write-actor; the jobs open NO writable conn (#3) -------

#[test]
fn test_wal_checkpoint_rides_write_actor_no_rogue_writer() {
    // forbidden #3 / LESSON §9 — the WAL checkpointer rides the SINGLE write-actor command path
    // (`WriteHandle::wal_checkpoint`); the staleness poller reads the READ-ONLY WAL. The jobs module
    // must open NO writable connection (a structural grep-pin, the LESSON §24/§28 idiom).
    let src = include_str!("../src/runtime/jobs.rs");
    assert!(
        src.contains("open_read_only"),
        "the staleness poller must read the read-only WAL"
    );
    assert!(
        !src.contains("Connection::open("),
        "jobs must NOT open a writable connection (forbidden #3 — no rogue writer)"
    );
    assert!(
        !src.contains("EventStore::open"),
        "jobs must NOT open the writable store (forbidden #3)"
    );
    // the checkpoint command goes through the handle, not a direct PRAGMA on a side connection.
    assert!(
        src.contains("wal_checkpoint"),
        "the checkpointer commands the write-actor via WriteHandle::wal_checkpoint"
    );
}

// ---- #5 — the checkpointer loop checkpoints on its tick + shuts down cleanly (§10) --------------

#[tokio::test]
async fn test_checkpointer_loop_checkpoints_and_shuts_down() {
    // spec(§10) — the checkpointer spawn loop ticks on the injected interval (MissedTickBehavior::Delay,
    // the drainer/reaper precedent), commands a bounded checkpoint through the write-actor, and stops
    // cleanly on the shutdown watch. TRUNCATE mode → the -wal file is bounded to zero after a tick
    // (proves the loop invoked the checkpoint — the `test_reaper_loop_invokes_reap_once` analog).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new(TS)),
        stub_gateway(),
    );
    let handle = actor.handle();
    for _ in 0..20 {
        handle.append(test_intent()).await.unwrap();
    }

    let (sd_tx, sd_rx) = watch::channel(false);
    let job = spawn_wal_checkpointer(
        handle,
        WalCheckpointMode::Truncate,
        Duration::from_millis(2),
        sd_rx,
    );
    tokio::time::sleep(Duration::from_millis(40)).await; // let it tick + checkpoint
    sd_tx.send(true).unwrap();
    job.await
        .expect("checkpointer loop joins cleanly on shutdown");

    let wal_len = std::fs::metadata(wal_file(&path))
        .map(|m| m.len())
        .unwrap_or(0);
    assert_eq!(
        wal_len, 0,
        "the checkpointer loop truncated the WAL on its tick"
    );

    actor.shutdown().await;
}

#[tokio::test]
async fn test_checkpointer_loop_survives_a_failed_pass() {
    // LESSON §9 (liveness) — a failing checkpoint pass must NOT kill the loop. Point it at an
    // already-shut-down actor → every pass fails (ActorGone); the loop logs + keeps ticking, and still
    // shuts down cleanly (mirrors `test_drain_loop_survives_a_failed_pass`).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new(TS)),
        stub_gateway(),
    );
    let handle = actor.handle();
    actor.shutdown().await; // every handle.wal_checkpoint → Err(ActorGone)

    let (sd_tx, sd_rx) = watch::channel(false);
    let job = spawn_wal_checkpointer(
        handle,
        WalCheckpointMode::Passive,
        Duration::from_millis(2),
        sd_rx,
    );
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !job.is_finished(),
        "a failing pass does NOT kill the checkpointer loop"
    );
    sd_tx.send(true).unwrap();
    job.await
        .expect("checkpointer loop joins cleanly on shutdown");
}

// ---- #6 — the staleness poller loop ticks read-only + shuts down cleanly (§7.2) -----------------

#[tokio::test]
async fn test_staleness_poller_loop_shuts_down_cleanly() {
    // spec(§7.2) — the staleness poller spawn loop recomputes over the READ-ONLY WAL each tick (never
    // back-pressures the writer, forbidden #3) and stops cleanly on the shutdown watch. A seeded
    // long-silent session means the recompute does real work each tick.
    let (_d, path) = temp_db();
    drop(open_store(&path));
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        seed_session(&conn, SessionId::new().as_str(), "active", Some(TS));
    }

    let (sd_tx, sd_rx) = watch::channel(false);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock::new("2026-06-14T01:00:00Z")); // an hour later
    let job = spawn_staleness_poller(path.clone(), clock, 60, Duration::from_millis(2), sd_rx);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(!job.is_finished(), "the staleness poller keeps ticking");
    sd_tx.send(true).unwrap();
    job.await
        .expect("staleness poller loop joins cleanly on shutdown");
}

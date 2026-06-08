//! Phase 1.6b L1 — daemon runtime: the single write-actor + graceful shutdown (RED first).
//! ARCHITECTURE §12 (runtime tasks), §16 (drain+exit), §4.2/forbidden #3/LESSON §3 (the single
//! write-actor — every mutation through one owner; reads via open_read_only).
//!
//! The write-actor is a dedicated blocking thread owning the writable `EventStore` (rusqlite is
//! synchronous) + an mpsc command channel; async callers (drainer/reaper/accept-loop) send
//! commands and await oneshot replies (Q1 default — confirmed at Step 2.5). L2–L4 tests
//! (drainer/reaper loops, accept-loop, subscribe) finalize against this handle post-Q1.

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{Sensitivity, SourceType};
use nexusops_shared::ids::WorkspaceId;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    open_read_only, AppendIntent, EventStore, JsonlMirror, PrefixRedactor, DRAIN_BATCH_LIMIT,
};
use nexusopsd::idgen::UlidGen;
use nexusopsd::locks::{LeaseKind, OwnerId, ResourceId};
use nexusopsd::runtime::{spawn_drainer, spawn_reaper, WriteActor};

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

fn temp_db() -> (tempfile::TempDir, PathBuf) {
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

fn test_intent() -> AppendIntent {
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
        payload_json: "{}".to_string(),
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: None,
        session_id: None,
        agent_team_id: None,
        visibility: None,
    }
}

#[tokio::test]
async fn test_write_actor_is_sole_writer() {
    // forbidden #3 / LESSON §3: ALL THREE mutators (append, drain_once, reap_leases) route through
    // the ONE write-actor handle — the runtime risk is "many tasks, one writer", not just append.
    // The actor owns the only writable EventStore; reads use a read-only WAL conn, never the actor.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
    );
    let handle = actor.handle();

    // (1) append routes through the actor and commits…
    let id = handle
        .append(test_intent())
        .await
        .expect("append via the write-actor");
    // (2) drain_once routes through the actor (drains the append's outbox row to the mirror sink)…
    let mirror = Arc::new(JsonlMirror::new(_d.path().join("mirror.jsonl")));
    handle
        .drain_once(mirror)
        .await
        .expect("drain_once via the write-actor");
    // (3) reap_leases routes through the actor (no live leases → empty, but it is the same writer).
    handle
        .reap_leases()
        .await
        .expect("reap_leases via the write-actor");

    // the append is visible to an INDEPENDENT read-only WAL connection (the reader never writes).
    let conn = open_read_only(&path).unwrap();
    let seen: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_id = ?1",
            [id.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        seen, 1,
        "the write-actor's append is durable + readable read-only"
    );

    actor.shutdown().await;
}

#[tokio::test]
async fn test_graceful_shutdown_stops_loops_clean() {
    // §16 drain+exit: a shutdown stops the actor + closes the writer cleanly. After shutdown the
    // handle's mutation API fails (the actor is gone) rather than hanging or half-applying — no
    // command is silently accepted post-shutdown.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
    );
    let handle = actor.handle();
    handle
        .append(test_intent())
        .await
        .expect("append before shutdown");

    actor.shutdown().await; // stops the actor thread + closes the writer

    // a post-shutdown mutation is refused (channel closed), never accepted or hung.
    let after = handle.append(test_intent()).await;
    assert!(
        after.is_err(),
        "the write-actor refuses mutations after a clean shutdown"
    );
}

// ---- L2 — drainer + reaper interval loops (+ bounded drain) ------------------

#[tokio::test]
async fn test_bounded_drain_pass_respects_limit() {
    // 1.3 backlog-starvation deferral: one drain pass delivers AT MOST DRAIN_BATCH_LIMIT rows so a
    // large backlog can't starve the writer in a single pass; the remainder drains the next pass.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
    );
    let handle = actor.handle();

    // each append writes one due jsonl_mirror outbox row → LIMIT + 5 due rows.
    let extra = 5;
    for _ in 0..(DRAIN_BATCH_LIMIT + extra) {
        handle.append(test_intent()).await.unwrap();
    }

    let mirror = Arc::new(JsonlMirror::new(_d.path().join("mirror.jsonl")));
    let pass1 = handle.drain_once(mirror.clone()).await.unwrap();
    assert_eq!(
        pass1.delivered, DRAIN_BATCH_LIMIT,
        "one pass is capped at the batch limit (can't starve the writer)"
    );
    let pass2 = handle.drain_once(mirror).await.unwrap();
    assert_eq!(
        pass2.delivered, extra,
        "the backlog beyond the cap drains on the next pass"
    );

    actor.shutdown().await;
}

#[tokio::test]
async fn test_drain_loop_survives_a_failed_pass() {
    // liveness: a failing drain pass must NOT kill the loop. We point the drainer at an actor that
    // has already shut down → EVERY pass fails (ActorGone); the loop logs + keeps ticking, and
    // still stops cleanly on the shutdown signal (no panic, no early exit).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
    );
    let handle = actor.handle();
    actor.shutdown().await; // now every handle.drain_once → Err(ActorGone)

    let (sd_tx, sd_rx) = watch::channel(false);
    let mirror = Arc::new(JsonlMirror::new(_d.path().join("mirror.jsonl")));
    let loop_task = spawn_drainer(handle, mirror, Duration::from_millis(2), sd_rx);

    // let several failing passes tick by…
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !loop_task.is_finished(),
        "a failing pass does NOT kill the drainer loop"
    );
    // …and it still shuts down cleanly.
    sd_tx.send(true).unwrap();
    loop_task
        .await
        .expect("drainer loop joins cleanly on shutdown");
}

#[tokio::test]
async fn test_reaper_loop_invokes_reap_once() {
    // §17 reaper spawn: the reaper task calls reap_leases on its tick. We seed an expired lease,
    // run the reaper with the actor's clock an hour ahead (so the lease is expired from its view),
    // and observe the slot freed (owner_id NULL) — the reaper invoked reap_once through the writer.
    let (_d, path) = temp_db();
    let mut store = open_store(&path);
    store
        .acquire_lease(
            &ResourceId("wt_1".to_string()),
            &LeaseKind("worktree".to_string()),
            &OwnerId("sess_a".to_string()),
            1, // ttl 1s → expires 00:00:01
            &FixedClock::new("2026-06-08T00:00:00Z"),
        )
        .unwrap();

    // the actor's clock is an hour later → the lease is expired from the reaper's view.
    let actor = WriteActor::spawn(store, Box::new(FixedClock::new("2026-06-08T01:00:00Z")));
    let handle = actor.handle();

    let (sd_tx, sd_rx) = watch::channel(false);
    let reaper = spawn_reaper(handle, Duration::from_millis(2), sd_rx);
    tokio::time::sleep(Duration::from_millis(40)).await; // let the reaper tick
    sd_tx.send(true).unwrap();
    reaper.await.expect("reaper loop joins cleanly on shutdown");

    let conn = open_read_only(&path).unwrap();
    let owner: Option<String> = conn
        .query_row(
            "SELECT owner_id FROM leases WHERE resource_id = 'wt_1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        owner.is_none(),
        "the reaper loop freed the expired lease (called reap_once through the writer)"
    );

    actor.shutdown().await;
}

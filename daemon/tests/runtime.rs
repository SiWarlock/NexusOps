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
    open_read_only, AppendIntent, EventStore, JsonlMirror, PrefixRedactor,
};
use nexusopsd::idgen::UlidGen;
use nexusopsd::runtime::WriteActor;

use std::path::{Path, PathBuf};
use std::sync::Arc;

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

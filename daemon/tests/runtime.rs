//! Phase 1.6b L1 — daemon runtime: the single write-actor + graceful shutdown (RED first).
//! ARCHITECTURE §12 (runtime tasks), §16 (drain+exit), §4.2/forbidden #3/LESSON §3 (the single
//! write-actor — every mutation through one owner; reads via open_read_only).
//!
//! The write-actor is a dedicated blocking thread owning the writable `EventStore` (rusqlite is
//! synchronous) + an mpsc command channel; async callers (drainer/reaper/accept-loop) send
//! commands and await oneshot replies (Q1 default — confirmed at Step 2.5). L2–L4 tests
//! (drainer/reaper loops, accept-loop, subscribe) finalize against this handle post-Q1.

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{RedactionStatus, Sensitivity, SourceType};
use nexusops_shared::events::{SessionFailed, SessionRecovered, TelemetrySampled};
use nexusops_shared::harness::{MetricQuality, TelemetrySample};
use nexusops_shared::ids::{SessionId, WorkspaceId};
use nexusops_shared::ipc::{
    DeltaKind, GetProjectionParams, HelloAck, HelloFrame, ProjectionDelta, ProjectionName,
    RpcRequest, RpcResponse, ServerFrame,
};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{
    open_read_only, AppendIntent, EventStore, JsonlMirror, PrefixRedactor, RedactionOutcome,
    Redactor, DRAIN_BATCH_LIMIT,
};
use nexusopsd::idgen::UlidGen;
use nexusopsd::ipc::{current_euid, read_frame, write_frame};
use nexusopsd::locks::{LeaseKind, OwnerId, ResourceId};
use nexusopsd::runtime::{
    bind, spawn_accept_loop, spawn_drainer, spawn_reaper, WriteActor, BROADCAST_CAPACITY,
};

use std::net::Shutdown;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

/// throwaway broadcast sender for `spawn_accept_loop`'s `deltas` param on accept-loop tests that
/// don't exercise the live subscribe push (1.6d; the serve-layer push is covered by tests/ipc.rs).
fn no_deltas() -> tokio::sync::broadcast::Sender<nexusops_shared::ipc::ProjectionDelta> {
    tokio::sync::broadcast::channel(1).0
}

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
        action_request_id: None,
        approval_id: None,
        causation_id: None,
    }
}

/// a SessionStarted intent with a session_id + a valid payload → the live delta source maps it to
/// a Session-projection Upsert delta (L4).
fn session_intent(session_id: &str) -> AppendIntent {
    let mut i = test_intent();
    i.session_id = Some(SessionId::parse(session_id).unwrap());
    i.payload_json = r#"{"status":"active"}"#.to_string();
    i
}

/// a SessionFailed intent with a session_id (4.2 — empty-payload) → the live delta source maps it to
/// a Session-projection Upsert nudge (L4, D3).
fn session_failed_intent(session_id: &str) -> AppendIntent {
    let mut i = test_intent();
    i.event_type = SessionFailed::EVENT_TYPE.to_string();
    i.session_id = Some(SessionId::parse(session_id).unwrap());
    i.payload_json = "{}".to_string(); // SessionFailed is empty-payload
    i
}

/// a SessionRecovered intent with a session_id (D2/4.4) → the live delta source maps it to a
/// Session-projection Upsert nudge (L4, D3). The payload BINDS cleanly (no Decode-degrade noise); the
/// in-band fold itself is a healthy no-op in these tests (no prior SessionStarted row for the id), and
/// the delta fires regardless — it is payload-agnostic (reads only event_type + session_id).
fn session_recovered_intent(session_id: &str) -> AppendIntent {
    let mut i = test_intent();
    i.event_type = SessionRecovered::EVENT_TYPE.to_string();
    i.session_id = Some(SessionId::parse(session_id).unwrap());
    i.payload_json =
        r#"{"mode":"replayed","replayed_event_count":3,"execution_profile_id":null}"#.to_string();
    i
}

/// a TelemetrySampled intent (4.0c — observation-appended via the telemetry sink) → the live delta
/// source maps it to a UsageLedger Upsert nudge (L4, D4a). The payload binds cleanly (no Decode noise);
/// the delta is payload-agnostic (reads only event_type).
fn telemetry_intent() -> AppendIntent {
    let payload = TelemetrySampled {
        sample: TelemetrySample {
            tokens_in: 10,
            tokens_out: 5,
            context_pct: None,
            cost_estimate: 0.0,
            metric_quality: MetricQuality::Exact,
        },
        model: None,
        execution_profile_id: None,
    };
    let mut i = test_intent();
    i.event_type = TelemetrySampled::EVENT_TYPE.to_string();
    i.session_id = Some(SessionId::parse("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap());
    i.source_type = SourceType::UsageMeter;
    i.payload_json = serde_json::to_string(&payload).unwrap();
    i
}

/// a generic (non-projection-specific) committed event — proves the AuditTrail blanket arm fires for
/// EVERY event, even one no projection-specific arm handles.
fn generic_intent(event_type: &str) -> AppendIntent {
    let mut i = test_intent();
    i.event_type = event_type.to_string();
    i.session_id = None;
    i.payload_json = "{}".to_string();
    i
}

/// drain every delta currently buffered on the receiver. D4a: an append now publishes MULTIPLE deltas
/// (the projection-specific one(s) + the blanket AuditTrail), and all of an append's deltas are in the
/// broadcast buffer by the time `append().await` returns (publish-after-commit) — so tests drain-and-find
/// rather than rely on a single `recv()` / push order.
fn drain_deltas(
    rx: &mut tokio::sync::broadcast::Receiver<ProjectionDelta>,
) -> Vec<ProjectionDelta> {
    let mut out = Vec::new();
    while let Ok(d) = rx.try_recv() {
        out.push(d);
    }
    out
}

/// whether `deltas` contains an `Upsert` for `projection`.
fn has_upsert(deltas: &[ProjectionDelta], projection: ProjectionName) -> bool {
    deltas
        .iter()
        .any(|d| d.projection == projection && matches!(d.kind, DeltaKind::Upsert))
}

/// a Redactor that refuses to redact — forces `append` to fail the §15 gate (so the append rolls
/// back, exercising "a rolled-back append publishes NO delta").
struct NeverRedacts;
impl Redactor for NeverRedacts {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        RedactionOutcome {
            status: RedactionStatus::Unredacted,
            payload_json: payload_json.to_string(),
            engine_version: "never".to_string(),
            quarantine: None,
        }
    }
}

/// a 2.1b stub Action Gateway (require-approval-for-all policy + no-side-effect executor) for the
/// write-actor in runtime tests.
fn stub_gateway() -> nexusopsd::gateway::Gateway {
    nexusopsd::gateway::Gateway::new(
        Box::new(nexusopsd::gateway::policy::StubPolicy),
        Box::new(nexusopsd::gateway::executor::StubExecutor),
    )
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
        stub_gateway(),
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
        stub_gateway(),
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
        stub_gateway(),
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
        stub_gateway(),
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
async fn test_reaper_loop_survives_a_failed_pass() {
    // liveness (symmetric to the drainer): a failing reap pass must NOT kill the loop. Point the
    // reaper at an already-shutdown actor → every pass fails (ActorGone); the loop logs + keeps
    // ticking, and still shuts down cleanly.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    actor.shutdown().await; // every handle.reap_leases → Err(ActorGone)

    let (sd_tx, sd_rx) = watch::channel(false);
    let loop_task = spawn_reaper(handle, Duration::from_millis(2), sd_rx);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !loop_task.is_finished(),
        "a failing pass does NOT kill the reaper loop"
    );
    sd_tx.send(true).unwrap();
    loop_task
        .await
        .expect("reaper loop joins cleanly on shutdown");
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
    let actor = WriteActor::spawn(
        store,
        Box::new(FixedClock::new("2026-06-08T01:00:00Z")),
        stub_gateway(),
    );
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

// ---- L3 — UDS bind + accept-loop ---------------------------------------------

/// connect + send a HelloFrame; return `true` if the server REJECTED us (dropped the connection
/// before a HelloAck — e.g. a foreign-uid reject or at-cap refusal → read_frame errs on EOF).
fn client_rejected(sock: &Path) -> bool {
    let mut stream = StdUnixStream::connect(sock).expect("connect");
    let hello = HelloFrame {
        protocol_version: 1,
        client_kind: "test".to_string(),
        app_version: "0".to_string(),
    };
    let _ = write_frame(&mut stream, &serde_json::to_vec(&hello).unwrap());
    read_frame(&mut stream).is_err()
}

/// connect + handshake; return the open stream (held → holds the server-side permit) + the ack.
fn client_handshake(sock: &Path) -> (StdUnixStream, HelloAck) {
    let mut stream = StdUnixStream::connect(sock).expect("connect");
    let hello = HelloFrame {
        protocol_version: 1,
        client_kind: "test".to_string(),
        app_version: "0".to_string(),
    };
    write_frame(&mut stream, &serde_json::to_vec(&hello).unwrap()).expect("write hello");
    let ack: HelloAck =
        serde_json::from_slice(&read_frame(&mut stream).expect("read ack")).expect("decode ack");
    (stream, ack)
}

/// connect → handshake → one get_projection → half-close → read the RpcResponse (a full session).
fn client_get_projection(sock: &Path, name: ProjectionName) -> RpcResponse {
    let mut stream = StdUnixStream::connect(sock).expect("connect");
    let hello = HelloFrame {
        protocol_version: 1,
        client_kind: "test".to_string(),
        app_version: "0".to_string(),
    };
    write_frame(&mut stream, &serde_json::to_vec(&hello).unwrap()).expect("write hello");
    let req = RpcRequest {
        method: "get_projection".to_string(),
        params: serde_json::to_value(GetProjectionParams { name, scope: None }).unwrap(),
        id: 1,
    };
    write_frame(&mut stream, &serde_json::to_vec(&req).unwrap()).expect("write req");
    stream
        .shutdown(Shutdown::Write)
        .expect("half-close write → serve loop ends on EOF");
    let _ack: HelloAck =
        serde_json::from_slice(&read_frame(&mut stream).expect("read ack")).expect("decode ack");
    match serde_json::from_slice::<ServerFrame>(&read_frame(&mut stream).expect("read resp"))
        .expect("decode ServerFrame")
    {
        ServerFrame::RpcResponse(r) => r,
        ServerFrame::SubscriptionPush(_) => {
            panic!("expected an RpcResponse, got a SubscriptionPush")
        }
        ServerFrame::TerminalOutput(_) => {
            panic!("expected an RpcResponse, got a TerminalOutput frame")
        }
    }
}

#[tokio::test]
async fn test_bind_reclaims_stale_socket() {
    // §16 stale-socket reclaim: a leftover socket file must not block bind (the pidlock already
    // guarantees single-instance, so any existing socket is stale → unlink-first, no EADDRINUSE).
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("gw.sock");
    let stale = bind(&sock).unwrap();
    drop(stale); // the socket FILE remains on disk after the listener drops
    assert!(sock.exists(), "a leftover socket file remains");
    let _listener = bind(&sock).expect("a fresh bind reclaims the stale socket");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_foreign_peer_rejected_in_accept_path() {
    // safety rule #7, end-to-end through the accept path: a peer whose uid ≠ the daemon-uid is
    // rejected. We force the mismatch by configuring a daemon_uid ≠ our real euid.
    let (_d, path) = temp_db();
    open_store(&path);
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("gw.sock");
    let listener = bind(&sock).unwrap();
    let (sd_tx, sd_rx) = watch::channel(false);
    let wrong_daemon_uid = current_euid().wrapping_add(1); // ≠ our real peer uid
    let accept = spawn_accept_loop(
        listener,
        path.clone(),
        wrong_daemon_uid,
        8,
        no_deltas(),
        nexusopsd::runtime::WriteHandle::disconnected(),
        std::sync::Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
        sd_rx,
    );

    let sock2 = sock.clone();
    let rejected = tokio::task::spawn_blocking(move || client_rejected(&sock2))
        .await
        .unwrap();
    assert!(
        rejected,
        "a foreign-uid peer is rejected through the accept path (rule #7)"
    );

    sd_tx.send(true).unwrap();
    let _ = accept.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_connection_cap_enforced() {
    // anti-DoS: at the concurrency cap, a new connection is refused. cap=1; A handshakes + holds
    // its connection open (holding the one permit); B is then refused.
    let (_d, path) = temp_db();
    open_store(&path);
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("gw.sock");
    let listener = bind(&sock).unwrap();
    let (sd_tx, sd_rx) = watch::channel(false);
    let uid = current_euid();
    let accept = spawn_accept_loop(
        listener,
        path.clone(),
        uid,
        1,
        no_deltas(),
        nexusopsd::runtime::WriteHandle::disconnected(),
        std::sync::Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
        sd_rx,
    ); // cap = 1

    let sock_a = sock.clone();
    // A handshakes + keeps the connection open → holds the single permit (acquired before its ack).
    let (a_stream, _ack) = tokio::task::spawn_blocking(move || client_handshake(&sock_a))
        .await
        .unwrap();

    let sock_b = sock.clone();
    let b_rejected = tokio::task::spawn_blocking(move || client_rejected(&sock_b))
        .await
        .unwrap();
    assert!(
        b_rejected,
        "at the cap, a new connection is refused (anti-DoS)"
    );

    drop(a_stream);
    sd_tx.send(true).unwrap();
    let _ = accept.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_connection_permit_released_on_close() {
    // the completing half of the anti-DoS invariant: a permit is RELEASED on connection close, so
    // a later connection succeeds (no permit leak / self-DoS). cap=1; A does a full session + closes;
    // B then connects successfully.
    let (_d, path) = temp_db();
    open_store(&path);
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("gw.sock");
    let listener = bind(&sock).unwrap();
    let (sd_tx, sd_rx) = watch::channel(false);
    let uid = current_euid();
    let accept = spawn_accept_loop(
        listener,
        path.clone(),
        uid,
        1,
        no_deltas(),
        nexusopsd::runtime::WriteHandle::disconnected(),
        std::sync::Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
        sd_rx,
    ); // cap = 1

    let sock_a = sock.clone();
    let _resp = tokio::task::spawn_blocking(move || {
        client_get_projection(&sock_a, ProjectionName::Session)
    })
    .await
    .unwrap();
    // give A's serve task a beat to finish + release the permit (accept-loop liveness timing).
    tokio::time::sleep(Duration::from_millis(20)).await;

    let sock_b = sock.clone();
    let b_rejected = tokio::task::spawn_blocking(move || client_rejected(&sock_b))
        .await
        .unwrap();
    assert!(
        !b_rejected,
        "a later connection succeeds — the permit was released on the prior connection's close"
    );

    sd_tx.send(true).unwrap();
    let _ = accept.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_read_projection_over_real_socket() {
    // §6.1 live read, in-process: a client handshakes + reads a projection over a REAL bound UDS
    // (accept-loop → getpeereid → serve_connection → dispatch over read-only WAL).
    let (_d, path) = temp_db();
    open_store(&path); // create + migrate the DB so the read-only conn + projections exist
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("gw.sock");
    let listener = bind(&sock).unwrap();
    let (sd_tx, sd_rx) = watch::channel(false);
    let uid = current_euid();
    let accept = spawn_accept_loop(
        listener,
        path.clone(),
        uid,
        8,
        no_deltas(),
        nexusopsd::runtime::WriteHandle::disconnected(),
        std::sync::Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
        sd_rx,
    );

    let sock2 = sock.clone();
    let resp =
        tokio::task::spawn_blocking(move || client_get_projection(&sock2, ProjectionName::Session))
            .await
            .unwrap();
    assert_eq!(resp.id, 1, "the response correlates by id");
    assert!(
        resp.error.is_none(),
        "the projection read succeeded over the real socket"
    );

    sd_tx.send(true).unwrap();
    let _ = accept.await;
}

// ---- L4 — live subscribe delta-source (broadcast publish-after-commit) --------

#[tokio::test]
async fn test_append_publishes_delta_after_commit() {
    // publish-after-commit: a COMMITTED append publishes the matching ProjectionDelta to the
    // write-actor's broadcast; a ROLLED-BACK append (refused by the §15 gate) publishes NOTHING.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();

    // committed → a Session Upsert delta is published.
    handle
        .append(session_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV"))
        .await
        .expect("committed append");
    // drain-and-find: the Session Upsert is AMONG the published deltas (D4a — every event also nudges
    // AuditTrail, so an append now publishes >1 delta; don't rely on a single recv / push order).
    let deltas = drain_deltas(&mut rx);
    assert!(has_upsert(&deltas, ProjectionName::Session));
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::Session
                && d.id.as_deref() == Some("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV")),
        "the Session delta carries the session id"
    );
    actor.shutdown().await;

    // rolled-back (the redaction gate refuses) → NO delta is published.
    let (_d2, path2) = temp_db();
    let store2 = EventStore::open(
        &path2,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(NeverRedacts),
    )
    .unwrap();
    let actor2 = WriteActor::spawn(
        store2,
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle2 = actor2.handle();
    let mut rx2 = handle2.subscribe();
    let refused = handle2
        .append(session_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAW"))
        .await;
    assert!(refused.is_err(), "the §15 gate refused the append");
    assert!(
        drain_deltas(&mut rx2).is_empty(),
        "a rolled-back append publishes no delta (the whole Vec is gated on commit)"
    );
    actor2.shutdown().await;
}

// ---- D3 (P4.5) — the live Session nudge on every proj_session-mutating event ------------------

#[tokio::test]
async fn test_session_failed_publishes_delta_after_commit() {
    // spec(§7/§11.4) — proj_session is mutated by SessionFailed (4.2 fold, LESSON §17) → the §6.1
    // subscriber must be nudged to re-read or the §11.4 Failed-session restart card goes stale.
    // Publish-after-commit (LESSON §9).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    handle
        .append(session_failed_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV"))
        .await
        .expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::Session
                && matches!(d.kind, DeltaKind::Upsert)
                && d.id.as_deref() == Some("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV")),
        "a committed SessionFailed publishes a Session Upsert keyed by its id"
    );
    actor.shutdown().await;
}

#[tokio::test]
async fn test_session_recovered_publishes_delta_after_commit() {
    // spec(§11.4) — without the nudge the resumed/replayed recovery banner never refreshes after restart
    // recovery; the D2/4.4 fold mutates resume_mode/replayed_event_count/recovered_at (status unchanged,
    // but the ROW changes — the subscriber must re-read).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    handle
        .append(session_recovered_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV"))
        .await
        .expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::Session
                && matches!(d.kind, DeltaKind::Upsert)
                && d.id.as_deref() == Some("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV")),
        "a committed SessionRecovered publishes a Session Upsert keyed by its id"
    );
    actor.shutdown().await;
}

#[tokio::test]
async fn test_session_event_without_session_id_publishes_no_session_delta() {
    // spec — the Session nudge carries the id (row: None); no session_id → NO Session delta (parity with
    // the SessionStarted `if let Some(sid)` guard). (D4a: the blanket AuditTrail nudge may still fire —
    // every event nudges the audit view — so this asserts the absence of the SESSION nudge specifically.)
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    let mut intent = session_failed_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV");
    intent.session_id = None;
    handle.append(intent).await.expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(
        !has_upsert(&deltas, ProjectionName::Session),
        "an id-less session event publishes NO Session delta"
    );
    actor.shutdown().await;
}

#[tokio::test]
async fn test_proj_session_folded_events_each_publish_a_session_delta() {
    // spec(LESSON §50, generalized to delta-source↔projector) — EVERY event type the SessionProjector
    // folds into proj_session MUST publish a Session Upsert nudge (a row-mutating event without a delta is
    // a silent stale-UI bug). If a future proj_session-folding event is added to `SessionProjector::apply`,
    // add its arm to `deltas_for_append` too — extend BOTH lists together (the keep-two-lists-honest guard).
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    // the event types keyed on the SAME token-form both lists use (consts where they exist; the
    // SessionStarted literal — it has no EVENT_TYPE const, and the projector uses the literal too).
    let cases = [
        (
            "SessionStarted",
            session_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV"),
        ),
        (
            SessionFailed::EVENT_TYPE,
            session_failed_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAW"),
        ),
        (
            SessionRecovered::EVENT_TYPE,
            session_recovered_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAX"),
        ),
    ];
    for (label, intent) in cases {
        let sid = intent.session_id.as_ref().unwrap().as_str().to_string();
        handle
            .append(intent)
            .await
            .unwrap_or_else(|_| panic!("{label} commits"));
        let deltas = drain_deltas(&mut rx);
        assert!(
            deltas
                .iter()
                .any(|d| d.projection == ProjectionName::Session
                    && matches!(d.kind, DeltaKind::Upsert)
                    && d.id.as_deref() == Some(sid.as_str())),
            "{label} publishes a Session Upsert delta keyed by its id"
        );
    }
    actor.shutdown().await;
}

// ---- D4a (P4.5) — the observation-path nudges (UsageLedger on TelemetrySampled; AuditTrail blanket) --

#[tokio::test]
async fn test_telemetry_sampled_publishes_usage_ledger_delta() {
    // spec(§7/§11) — proj_usage_ledger is mutated by TelemetrySampled (the only event the UsageProjector
    // folds) → the §6.1 usage view must re-read. id: None (payload-agnostic — the subscriber re-reads the
    // small aggregate). Publish-after-commit.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    handle
        .append(telemetry_intent())
        .await
        .expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(has_upsert(&deltas, ProjectionName::UsageLedger));
    assert!(
        deltas
            .iter()
            .any(|d| d.projection == ProjectionName::UsageLedger && d.id.is_none()),
        "the UsageLedger nudge is id-less (payload-agnostic)"
    );
    actor.shutdown().await;
}

#[tokio::test]
async fn test_every_event_publishes_an_audit_trail_delta() {
    // spec(§7) — the AuditProjector folds EVERY event into an audit row → every committed Command::Append
    // event nudges the audit view (the blanket arm, id: None). Several distinct event types (incl. a generic
    // one no projection-specific arm handles) each publish an AuditTrail Upsert.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    let cases = [
        session_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV"),
        session_failed_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAW"),
        telemetry_intent(),
        generic_intent("AuditIntegrityViolation"),
    ];
    for intent in cases {
        let et = intent.event_type.clone();
        handle
            .append(intent)
            .await
            .unwrap_or_else(|_| panic!("{et} commits"));
        let deltas = drain_deltas(&mut rx);
        assert!(
            deltas
                .iter()
                .any(|d| d.projection == ProjectionName::AuditTrail
                    && matches!(d.kind, DeltaKind::Upsert)
                    && d.id.is_none()),
            "{et} publishes a blanket AuditTrail Upsert (id-less)"
        );
    }
    actor.shutdown().await;
}

#[tokio::test]
async fn test_event_publishes_multiple_projection_deltas() {
    // spec — a single event mutating N projections nudges all N (the Vec<ProjectionDelta> contract):
    // TelemetrySampled → {UsageLedger, AuditTrail}; SessionFailed → {Session, AuditTrail}.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();

    handle
        .append(telemetry_intent())
        .await
        .expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(has_upsert(&deltas, ProjectionName::UsageLedger));
    assert!(has_upsert(&deltas, ProjectionName::AuditTrail));

    handle
        .append(session_failed_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV"))
        .await
        .expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(has_upsert(&deltas, ProjectionName::Session));
    assert!(has_upsert(&deltas, ProjectionName::AuditTrail));
    actor.shutdown().await;
}

#[tokio::test]
async fn test_proj_usage_ledger_folded_events_match_delta_source() {
    // spec(LESSON §51, generalized to delta-source↔projector) — proj_usage_ledger folds EXACTLY
    // {TelemetrySampled}; assert a committed TelemetrySampled (keyed on the const) publishes a UsageLedger
    // delta. A future proj_usage_ledger-folding event added to the UsageProjector without a delta-source arm
    // is a silent stale-UI bug — extend BOTH lists together.
    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();
    assert_eq!(TelemetrySampled::EVENT_TYPE, "TelemetrySampled");
    handle
        .append(telemetry_intent())
        .await
        .expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(
        has_upsert(&deltas, ProjectionName::UsageLedger),
        "the UsageLedger nudge fires on TelemetrySampled::EVENT_TYPE"
    );
    // the NEGATIVE arm — the UsageLedger nudge is SELECTIVE (not blanket like AuditTrail): a
    // non-TelemetrySampled event must NOT fire it (catches a UsageLedger arm that drifts too broad).
    handle
        .append(generic_intent("SomeOtherEvent"))
        .await
        .expect("committed append");
    let deltas = drain_deltas(&mut rx);
    assert!(
        !has_upsert(&deltas, ProjectionName::UsageLedger),
        "a non-TelemetrySampled event publishes NO UsageLedger delta (the arm is selective)"
    );
    actor.shutdown().await;
}

// ---- P2.1c L1 — a committed gateway approval publishes an ApprovalQueue delta -----------------

/// a minimal §6.2 ActionRequest at risk `level` (the gateway delta-source input).
fn sample_action_request(
    risk: nexusops_shared::actions::RiskLevel,
) -> nexusops_shared::actions::ActionRequest {
    use nexusops_shared::actions::{ActionRequest, RequesterType};
    use nexusops_shared::ids::ActionRequestId;
    use nexusops_shared::status::ActionRequestStatus;
    use nexusops_shared::time::Timestamp;
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "git.create_worktree".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({ "branch": "feature/x" }),
        risk_level: risk,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-08T00:00:00Z").unwrap(),
    }
}

#[tokio::test]
async fn test_gateway_approval_publishes_queue_delta() {
    // spec(§6.1 subscribe / forbidden #3) — a COMMITTED submit_action publishes an ApprovalQueue
    // Upsert delta (the approval row it opened) on the write-actor broadcast, publish-after-commit;
    // a ROLLED-BACK submit (the §15 gate refuses) publishes NOTHING. Closes the 2.1b flag (b).
    use nexusops_shared::actions::RiskLevel;

    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe();

    // committed submit → an ApprovalQueue Upsert delta keyed by the appr_ approval_id (Q6).
    let h = handle.clone();
    let ack = tokio::task::spawn_blocking(move || {
        h.submit_action_blocking(sample_action_request(RiskLevel::Level2))
    })
    .await
    .unwrap()
    .expect("write-actor reachable")
    .expect("submit");
    assert!(ack.action_request_id.starts_with("act_"));
    let delta = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("a queue delta was published within 2s")
        .expect("a committed gateway submit published a queue delta");
    assert_eq!(delta.projection, ProjectionName::ApprovalQueue);
    assert!(matches!(delta.kind, DeltaKind::Upsert));
    let appr_id = delta
        .id
        .clone()
        .expect("the delta is keyed by the approval_id");
    assert!(
        appr_id.starts_with("appr_"),
        "keyed by the appr_ approval_id"
    );

    // a committed approve ALSO publishes a queue delta (the row status advances) — exercises the
    // GatewayApprove → approve_collecting → publish_after_commit wiring, not just submit.
    let h = handle.clone();
    let appr = appr_id.clone();
    tokio::task::spawn_blocking(move || h.approve_blocking(appr))
        .await
        .unwrap()
        .expect("write-actor reachable")
        .expect("approve");
    let approve_delta = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("a queue delta was published within 2s")
        .expect("a committed gateway approve published a queue delta");
    assert_eq!(approve_delta.projection, ProjectionName::ApprovalQueue);
    assert_eq!(
        approve_delta.id.as_deref(),
        Some(appr_id.as_str()),
        "the approve delta is keyed by the same approval_id"
    );
    actor.shutdown().await;

    // rolled-back submit (NeverRedacts refuses the §15 gate) → no delta published.
    let (_d2, path2) = temp_db();
    let store2 = EventStore::open(
        &path2,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(NeverRedacts),
    )
    .unwrap();
    let actor2 = WriteActor::spawn(
        store2,
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle2 = actor2.handle();
    let mut rx2 = handle2.subscribe();
    let h2 = handle2.clone();
    let refused = tokio::task::spawn_blocking(move || {
        h2.submit_action_blocking(sample_action_request(RiskLevel::Level3))
    })
    .await
    .unwrap()
    .expect("write-actor reachable");
    assert!(refused.is_err(), "the §15 gate refused the gateway submit");
    // precise: Empty (nothing sent) — NOT Lagged (a dropped delta would also be `is_err`).
    assert!(
        matches!(
            rx2.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ),
        "a rolled-back gateway submit publishes no delta (Empty, not Lagged)"
    );
    actor2.shutdown().await;
}

#[tokio::test]
async fn test_lagging_subscriber_never_stalls_writer() {
    // forbidden #3 — a reader must NEVER back-pressure the writer. A subscriber that never drains
    // its receiver lags; the writer keeps appending (broadcast::send never blocks), and the lagging
    // receiver observes Lagged (dropped deltas), not a stalled writer.
    use tokio::sync::broadcast::error::TryRecvError;

    let (_d, path) = temp_db();
    let actor = WriteActor::spawn(
        open_store(&path),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        stub_gateway(),
    );
    let handle = actor.handle();
    let mut rx = handle.subscribe(); // a subscriber that NEVER drains (lags)

    // append well past the broadcast capacity WITHOUT draining rx — every append must still commit.
    for _ in 0..(BROADCAST_CAPACITY + 20) {
        handle
            .append(session_intent("sess_01ARZ3NDEKTSV4RRFFQ69G5FAV"))
            .await
            .expect("the writer is never back-pressured by a lagging subscriber");
    }

    // the lagging receiver sees Lagged (deltas were dropped for it), proving no writer stall.
    assert!(
        matches!(rx.try_recv(), Err(TryRecvError::Lagged(_))),
        "a lagging subscriber observes Lagged — the writer was not back-pressured (forbidden #3)"
    );

    actor.shutdown().await;
}

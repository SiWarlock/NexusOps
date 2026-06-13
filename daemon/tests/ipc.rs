//! Phase 1.5 — UDS GatewayPort transport (RED first). ARCHITECTURE §6.1/§6.4 [LOCKED ADR-004]
//! (4-byte big-endian length-prefix + JSON body; `MAX_FRAME_SIZE`; HelloFrame→HelloAck|
//! VersionSkewError handshake; read methods get_projection/subscribe/get_capabilities), §15 /
//! **Key safety rule #7** (UDS peer-auth = `getpeereid()`, reject uid≠daemon-uid; NOT SO_PEERCRED),
//! §12 (accept-loop as a Tokio task — spawn 1.6). Must satisfy the merged ui `GatewayPort`
//! contract (`ui/src/gateway-client/types.ts`, `SUPPORTED_PROTOCOL_RANGE {1,1}`).
//!
//! Layered:
//!   L1 (1–4) — length-prefix framing (bounded) + getpeereid peer-auth + reject-path. ⚠️ safety-critical
//!   L2 (5–7) — handshake + version negotiation + shared/ ipc schema.
//!   L3 (8–10) — JSON-RPC dispatch + get_projection + get_capabilities.
//!   L4 (11–14) — subscribe streaming + Terminal-Channel frame-type seam.

use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};

use nexusops_shared::ipc::{
    Capabilities, HelloAck, HelloFrame, IpcErrorCode, VersionSkewError, WireError,
};
use nexusopsd::ipc::{
    authorize_peer, decode_len, encode_frame, peer_uid, read_frame, serve_connection, write_frame,
    IpcError, MAX_FRAME_SIZE,
};

// ===================== L1 — framing + peer-auth (1–4) ========================

// ---- Test 1 — frame codec round-trips; 4-byte big-endian length prefix (§6.4) -

#[test]
fn test_frame_codec_roundtrip() {
    let body = br#"{"jsonrpc":"2.0","method":"get_capabilities"}"#;
    let framed = encode_frame(body).unwrap();
    // the first 4 bytes are the body length, big-endian
    assert_eq!(
        &framed[0..4],
        &(body.len() as u32).to_be_bytes(),
        "4-byte big-endian length prefix"
    );
    let len = decode_len(framed[0..4].try_into().unwrap()).unwrap();
    assert_eq!(len, body.len(), "decoded length matches the body length");
    assert_eq!(
        &framed[4..4 + len],
        body,
        "body bytes preserved across the codec"
    );
}

// ---- Test 2 — over-MAX_FRAME_SIZE rejected from the prefix, no alloc (§6.4) ---

#[test]
fn test_frame_too_large_rejected() {
    // a declared length one byte over the cap is rejected from the 4-byte prefix ALONE —
    // before any body buffer is allocated (the anti-DoS pin).
    let oversized = (MAX_FRAME_SIZE as u32 + 1).to_be_bytes();
    assert!(
        matches!(decode_len(&oversized), Err(IpcError::FrameTooLarge { .. })),
        "a frame larger than MAX_FRAME_SIZE is rejected from its length prefix"
    );
    // exactly at the cap is allowed
    assert!(
        decode_len(&(MAX_FRAME_SIZE as u32).to_be_bytes()).is_ok(),
        "a frame at exactly MAX_FRAME_SIZE is accepted"
    );
    // encode refuses to emit an oversized body too (symmetry)
    let too_big = vec![0u8; MAX_FRAME_SIZE + 1];
    assert!(
        matches!(encode_frame(&too_big), Err(IpcError::FrameTooLarge { .. })),
        "encode refuses an oversized body"
    );
    // …but a body at exactly the cap is accepted (the encode boundary mirrors decode_len)
    assert!(
        encode_frame(&vec![0u8; MAX_FRAME_SIZE]).is_ok(),
        "encode accepts a body at exactly MAX_FRAME_SIZE"
    );
}

// ---- Test 3 — getpeereid peer-auth: reject uid≠daemon-uid (rule #7) ----------

#[test]
fn test_wrong_uid_peer_rejected() {
    // The REJECTION is pinned at the pure authz gate: a unit test runs as a single uid, so a
    // foreign uid can't be produced by a real connection — `authorize_peer` is the oracle. A
    // peer uid ≠ the daemon uid → unauthorized_peer (getpeereid gate, NOT SO_PEERCRED).
    let daemon_uid = 1000u32;
    assert!(
        matches!(
            authorize_peer(daemon_uid + 1, daemon_uid),
            Err(IpcError::UnauthorizedPeer { .. })
        ),
        "a peer uid ≠ daemon uid is rejected"
    );
    assert!(
        authorize_peer(daemon_uid, daemon_uid).is_ok(),
        "a same-uid peer is accepted"
    );

    // The getpeereid-reads-the-fd WIRING is covered by a real same-uid socket: bind a temp UDS,
    // connect from this same process, read the accepted peer's uid (== our uid), and authorize.
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("gw.sock");
    let listener = UnixListener::bind(&sock).unwrap();
    let _client = UnixStream::connect(&sock).unwrap();
    let (accepted, _addr) = listener.accept().unwrap();
    let uid = peer_uid(accepted.as_raw_fd()).unwrap();
    assert!(
        authorize_peer(uid, uid).is_ok(),
        "getpeereid returns this process's uid over a real same-process socket; it authorizes against itself"
    );
}

// ---- Test 4 — foreign uid: per-connection handler disconnects, serves nothing -

#[test]
fn test_unauthorized_peer_disconnects_unserved() {
    // pins rule #7's ENFORCEMENT (not just the predicate): the per-connection handler is
    // given a peer uid (the accept-loop reads it via getpeereid, passes it in). A FOREIGN
    // uid must reject BEFORE serving any method — and the auth gate runs before any frame
    // read, so a non-hanging Err proves "zero methods served" (the client writes nothing;
    // if auth ran after a read, the handler would block on the absent HelloFrame forever).
    let (server, client) = UnixStream::pair().unwrap();
    let daemon_uid = 1000u32;
    let (_d, path) = temp_db();

    let outcome = serve_connection(
        server,
        daemon_uid + 1,
        daemon_uid,
        &path,
        no_deltas(),
        &nexusopsd::runtime::WriteHandle::disconnected(),
        &decision_registry(),
    );
    assert!(
        matches!(outcome, Err(IpcError::UnauthorizedPeer { .. })),
        "a foreign-uid connection is rejected by the handler"
    );

    // the handler consumed `server` and dropped it on the reject return → the client sees
    // EOF: the connection is disconnected and no method was ever served.
    let mut client = client;
    let mut buf = [0u8; 1];
    assert_eq!(
        client.read(&mut buf).unwrap(),
        0,
        "server disconnected (EOF) without serving a method"
    );
}

// ================= L2 — handshake + version negotiation (5–7) =================

/// frame + send a serializable value over a client handle (the test acts as the ui client).
fn send<T: serde::Serialize>(stream: &UnixStream, value: &T) {
    let mut w = stream;
    let body = serde_json::to_vec(value).expect("send: serialize value");
    write_frame(&mut w, &body).expect("send: write frame");
}

/// read one frame and deserialize it (the test reads the daemon's response).
fn recv<T: serde::de::DeserializeOwned>(stream: &UnixStream) -> T {
    let mut r = stream;
    let body = read_frame(&mut r).expect("recv: read frame");
    serde_json::from_slice(&body).expect("recv: deserialize frame")
}

/// a throwaway broadcast sender for the `serve_connection` `deltas` param on tests that don't
/// exercise the live subscribe push (1.6d). The handshake / RPC / auth tests never `subscribe`,
/// so the sender is never `.subscribe()`'d; `test_subscriber_receives_delta_frame` passes a real one.
fn no_deltas() -> tokio::sync::broadcast::Sender<nexusops_shared::ipc::ProjectionDelta> {
    tokio::sync::broadcast::channel(1).0
}

/// a fresh C2 decision_sink registry for the `serve_connection` `registry` param — these tests don't
/// exercise the `intercept` method, so it stays empty (the transport param is just plumbing here).
fn decision_registry() -> nexusopsd::decisions::DecisionRegistry {
    nexusopsd::decisions::DecisionRegistry::new()
}

// ---- Test 5 — handshake: in-range HelloFrame → HelloAck (§6.4) ---------------

#[test]
fn test_handshake_hello_ack() {
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    // the client sends the handshake first (buffered in the socket), then the handler reads it
    let hello = HelloFrame {
        protocol_version: 1,
        client_kind: "desktop_ui".to_string(),
        app_version: "0.1.0".to_string(),
    };
    send(&client, &hello);
    client.shutdown(Shutdown::Write).unwrap(); // no methods follow → the serve loop exits at EOF

    // same-uid auth passes → in-range handshake → HelloAck written, handler returns Ok
    let (_d, path) = temp_db();
    serve_connection(
        server,
        uid,
        uid,
        &path,
        no_deltas(),
        &nexusopsd::runtime::WriteHandle::disconnected(),
        &decision_registry(),
    )
    .expect("authorized in-range handshake succeeds");

    let ack: HelloAck = recv(&client);
    assert_eq!(
        ack.protocol_version, 1,
        "ack echoes the negotiated protocol_version"
    );
    assert_eq!(
        ack.capabilities.contract_version,
        nexusops_shared::CONTRACT_VERSION,
        "capabilities carry the daemon's CONTRACT_VERSION (§5.0)"
    );
    // capabilities shape matches the ui's pinned `Capabilities` (protocol_version + contract_version)
    let _: Capabilities = ack.capabilities;
}

// ---- Test 6 — handshake: out-of-range protocol_version → VersionSkewError ----

#[test]
fn test_version_skew_disconnects() {
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    let hello = HelloFrame {
        protocol_version: 99, // outside SUPPORTED_PROTOCOL_RANGE {1,1}
        client_kind: "desktop_ui".to_string(),
        app_version: "0.1.0".to_string(),
    };
    send(&client, &hello);

    // out-of-range → the handler writes a VersionSkewError and returns Err (disconnect)
    let (_d, path) = temp_db();
    let outcome = serve_connection(
        server,
        uid,
        uid,
        &path,
        no_deltas(),
        &nexusopsd::runtime::WriteHandle::disconnected(),
        &decision_registry(),
    );
    assert!(
        matches!(outcome, Err(IpcError::VersionSkew { .. })),
        "an out-of-range protocol_version is a version-skew failure"
    );

    let skew: VersionSkewError = recv(&client);
    assert_eq!(skew.client_protocol_version, 99);
    assert_eq!(
        skew.supported_max, 1,
        "the daemon advertises its supported range"
    );

    // no method is served — after the skew frame the connection is closed (EOF)
    let mut r = &client;
    let mut buf = [0u8; 1];
    assert_eq!(
        r.read(&mut buf).unwrap(),
        0,
        "disconnected after the skew, no method served"
    );
}

// ---- Test 7 — a non-handshake first frame is rejected + disconnects ----------

#[test]
fn test_method_before_handshake_rejected() {
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    // a method-shaped first frame (NOT a HelloFrame) — handshake-first is violated
    send(
        &client,
        &serde_json::json!({ "method": "get_capabilities", "id": 1 }),
    );

    let (_d, path) = temp_db();
    let outcome = serve_connection(
        server,
        uid,
        uid,
        &path,
        no_deltas(),
        &nexusopsd::runtime::WriteHandle::disconnected(),
        &decision_registry(),
    );
    assert!(
        matches!(outcome, Err(IpcError::Protocol(_))),
        "a method frame before the handshake is a protocol violation"
    );
    // the daemon wrote a structured WireError frame before disconnecting: a non-handshake first
    // frame is a protocol violation → `protocol_error` (the §6.4 code added per the lead-ratified
    // gap resolution — distinct from an unknown method name)
    let err: WireError = recv(&client);
    assert_eq!(err.code, IpcErrorCode::ProtocolError);
    // …and then disconnected: no method was served
    let mut r = &client;
    let mut buf = [0u8; 1];
    assert_eq!(
        r.read(&mut buf).unwrap(),
        0,
        "disconnected; the pre-handshake method was never served"
    );
}

// ============== L3 — JSON-RPC dispatch + read methods (8–10) ==================

use std::net::Shutdown;
use std::path::Path;

use nexusops_shared::ipc::{
    GetProjectionParams, ProjectionName, ProjectionScope, RpcRequest, RpcResponse,
};

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

/// seed an event store with one `SessionStarted` (1.2 folds it into `proj_session`), then drop
/// the writer so the IPC read-only WAL path reads a settled file.
fn seed_session(path: &Path) {
    use nexusops_shared::actor::ActorType;
    use nexusops_shared::event_envelope::{Sensitivity, SourceType};
    use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
    use nexusopsd::clock::FixedClock;
    use nexusopsd::eventstore::{AppendIntent, EventStore, PrefixRedactor};

    let mut store = EventStore::open(
        path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .unwrap();
    store
        .append(AppendIntent {
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
            payload_json: "{\"status\":\"active\"}".to_string(),
            schema_version: "event-envelope-v1".to_string(),
            idempotency_key: None,
            project_id: Some(ProjectId::new()),
            session_id: Some(SessionId::new()),
            agent_team_id: None,
            visibility: None,
            action_request_id: None,
            approval_id: None,
            causation_id: None,
        })
        .unwrap();
}

/// drive a full client session: handshake, then the given requests; half-close the write side
/// so the daemon's serve loop sees EOF and returns. Returns the daemon's per-request responses.
fn client_session(client: &UnixStream, requests: &[RpcRequest]) -> Vec<RpcResponse> {
    let hello = HelloFrame {
        protocol_version: 1,
        client_kind: "desktop_ui".to_string(),
        app_version: "0.1.0".to_string(),
    };
    send(client, &hello);
    for req in requests {
        send(client, req);
    }
    client.shutdown(Shutdown::Write).unwrap(); // EOF → the serve loop terminates
    let _ack: HelloAck = recv(client);
    // responses ride the frame-type-tagged ServerFrame envelope (§6.4) — unwrap the RpcResponse.
    requests
        .iter()
        .map(
            |_| match recv::<nexusops_shared::ipc::ServerFrame>(client) {
                nexusops_shared::ipc::ServerFrame::RpcResponse(r) => r,
                nexusops_shared::ipc::ServerFrame::SubscriptionPush(_) => {
                    panic!("expected an RpcResponse frame, got a SubscriptionPush")
                }
                nexusops_shared::ipc::ServerFrame::TerminalOutput(_) => {
                    panic!("expected an RpcResponse frame, got a TerminalOutput")
                }
            },
        )
        .collect()
}

// ---- Test 8 — get_projection returns rows over read-only WAL (§6.1) ----------

#[test]
fn test_get_projection_returns_rows() {
    let (_d, path) = temp_db();
    seed_session(&path);
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    let req = RpcRequest {
        method: "get_projection".to_string(),
        params: serde_json::to_value(GetProjectionParams {
            name: ProjectionName::Session,
            scope: None,
        })
        .unwrap(),
        id: 1,
    };
    // serve over the seeded db (read-only WAL); the client gets the proj_session row(s)
    let responses = std::thread::scope(|s| {
        let h = s.spawn(|| client_session(&client, std::slice::from_ref(&req)));
        serve_connection(
            server,
            uid,
            uid,
            &path,
            no_deltas(),
            &nexusopsd::runtime::WriteHandle::disconnected(),
            &decision_registry(),
        )
        .expect("serve get_projection");
        h.join().unwrap()
    });

    assert_eq!(responses.len(), 1);
    let resp = &responses[0];
    assert_eq!(resp.id, 1, "response correlates by id");
    assert!(resp.error.is_none(), "no error");
    let rows = resp.result.as_ref().unwrap().as_array().unwrap();
    assert_eq!(rows.len(), 1, "the seeded session row is returned");
    assert_eq!(
        rows[0].get("status").and_then(|v| v.as_str()),
        Some("active"),
        "the proj_session row carries the folded status"
    );
}

// ---- §14 reachability — submit_action reaches the Gateway through the REAL IPC dispatch -------

/// a minimal §6.2 ActionRequest at Submitted (for the IPC reachability test).
fn sample_action_request() -> nexusops_shared::actions::ActionRequest {
    use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
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
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
    }
}

#[test]
fn test_submit_action_reachable_through_ipc_dispatch() {
    // §14 / Step-7.5 — the mutation arm is wired to the REAL `methods::dispatch` path (IPC →
    // write-actor → Gateway pipeline), NOT only the in-process Gateway calls. A LIVE write-actor
    // (vs `disconnected()`) is required — the action only persists if the dispatch reaches it.
    use nexusopsd::clock::FixedClock;
    use nexusopsd::eventstore::{EventStore, PrefixRedactor};
    use nexusopsd::gateway::{executor::StubExecutor, policy::StubPolicy, Gateway};
    use nexusopsd::runtime::WriteActor;

    let (_d, path) = temp_db();
    let store = EventStore::open(
        &path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .unwrap();
    let gateway = Gateway::new(Box::new(StubPolicy), Box::new(StubExecutor));
    let actor = WriteActor::spawn(
        store,
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        gateway,
    );
    let handle = actor.handle();

    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;
    let req = RpcRequest {
        method: "submit_action".to_string(),
        params: serde_json::to_value(sample_action_request()).unwrap(),
        id: 7,
    };
    let responses = std::thread::scope(|s| {
        let h = s.spawn(|| client_session(&client, std::slice::from_ref(&req)));
        serve_connection(
            server,
            uid,
            uid,
            &path,
            no_deltas(),
            &handle,
            &decision_registry(),
        )
        .expect("serve submit");
        h.join().unwrap()
    });

    assert_eq!(responses.len(), 1);
    let resp = &responses[0];
    assert!(resp.error.is_none(), "submit succeeded: {:?}", resp.error);
    let ack = resp.result.as_ref().expect("an ActionAck result");
    assert_eq!(
        ack.get("status").and_then(|v| v.as_str()),
        Some("awaiting_approval"),
        "the ActionAck status (stub policy → awaiting approval)"
    );
    // the durable row persisted via the dispatch→actor→Gateway path — the reachability proof.
    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM action_requests", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1, "the action persisted via the REAL IPC dispatch path");
}

// ---- Test 9 — an unfed projection returns its empty table, not an error ------

#[test]
fn test_get_projection_unfed_is_empty_not_error() {
    let (_d, path) = temp_db();
    seed_session(&path); // creates all proj_* tables; PullRequest's body is re-homed to 7.1
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    let req = RpcRequest {
        method: "get_projection".to_string(),
        params: serde_json::to_value(GetProjectionParams {
            name: ProjectionName::PullRequest,
            scope: None,
        })
        .unwrap(),
        id: 7,
    };
    let responses = std::thread::scope(|s| {
        let h = s.spawn(|| client_session(&client, std::slice::from_ref(&req)));
        serve_connection(
            server,
            uid,
            uid,
            &path,
            no_deltas(),
            &nexusopsd::runtime::WriteHandle::disconnected(),
            &decision_registry(),
        )
        .expect("serve unfed projection");
        h.join().unwrap()
    });

    let resp = &responses[0];
    assert!(resp.error.is_none(), "an unfed projection is NOT an error");
    let rows = resp.result.as_ref().unwrap().as_array().unwrap();
    assert_eq!(
        rows.len(),
        0,
        "its (existing, empty) table returns zero rows"
    );
}

// ---- Test 10 — unknown method → unknown_method; get_capabilities (§6.1) ------

#[test]
fn test_unknown_method_and_get_capabilities() {
    let (_d, path) = temp_db();
    seed_session(&path);
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    let reqs = vec![
        RpcRequest {
            method: "get_capabilities".to_string(),
            params: serde_json::Value::Null,
            id: 1,
        },
        RpcRequest {
            method: "frobnicate".to_string(), // not a §6.1 method
            params: serde_json::Value::Null,
            id: 2,
        },
    ];
    let responses = std::thread::scope(|s| {
        let h = s.spawn(|| client_session(&client, &reqs));
        serve_connection(
            server,
            uid,
            uid,
            &path,
            no_deltas(),
            &nexusopsd::runtime::WriteHandle::disconnected(),
            &decision_registry(),
        )
        .expect("serve capabilities + unknown");
        h.join().unwrap()
    });

    // get_capabilities → Capabilities{protocol_version, contract_version}
    let caps: Capabilities = serde_json::from_value(responses[0].result.clone().unwrap()).unwrap();
    assert_eq!(caps.protocol_version, 1);
    assert_eq!(caps.contract_version, nexusops_shared::CONTRACT_VERSION);
    // unknown method → a structured unknown_method error (not served, not a panic)
    assert_eq!(
        responses[1].error.as_ref().unwrap().code,
        IpcErrorCode::UnknownMethod
    );
    assert!(responses[1].result.is_none());
}

// ---- Test (L3) — scope is accepted but NOT YET enforced (MVP; pins non-filtering) -

#[test]
fn test_get_projection_scope_not_yet_enforced() {
    let (_d, path) = temp_db();
    seed_session(&path);
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    // a scope naming a project that does NOT exist — MVP does not filter, so the row still returns.
    // This PINS the documented non-enforcement (so a future "scope ignored" isn't a silent surprise).
    let req = RpcRequest {
        method: "get_projection".to_string(),
        params: serde_json::to_value(GetProjectionParams {
            name: ProjectionName::Session,
            scope: Some(ProjectionScope {
                project_id: Some("proj_does_not_exist".to_string()),
            }),
        })
        .unwrap(),
        id: 3,
    };
    let responses = std::thread::scope(|s| {
        let h = s.spawn(|| client_session(&client, std::slice::from_ref(&req)));
        serve_connection(
            server,
            uid,
            uid,
            &path,
            no_deltas(),
            &nexusopsd::runtime::WriteHandle::disconnected(),
            &decision_registry(),
        )
        .expect("serve scoped projection");
        h.join().unwrap()
    });

    let rows = responses[0].result.as_ref().unwrap().as_array().unwrap();
    assert_eq!(
        rows.len(),
        1,
        "scope is accepted but NOT enforced in MVP — the read is unscoped (all rows return)"
    );
}

// =========== L4 — subscribe streaming + frame-type seam (11–12) ===============

use nexusops_shared::ipc::{DeltaKind, ProjectionDelta, ServerFrame, SubscribeParams};
use nexusopsd::ipc::push_subscription;

// ---- Test 11 — subscribe pushes a frame-type-tagged ProjectionDelta (§6.1) ---

#[test]
fn test_subscribe_pushes_projection_delta() {
    // the PUSH MECHANISM: a delta source → `push_subscription` writes a `SubscriptionPush` frame.
    // (The live delta source — EventStore.append → broadcast → subscriber — is 1.6-wired, like
    // the accept-loop; this is the deterministic unit, the "ship mechanism, wire source at 1.6".)
    let (server, client) = UnixStream::pair().unwrap();
    let delta = ProjectionDelta {
        projection: ProjectionName::Session,
        kind: DeltaKind::Upsert,
        row: Some(serde_json::json!({"session_id": "sess_1", "status": "active"})),
        id: Some("sess_1".to_string()),
    };

    let mut w = &server;
    let n = push_subscription(&mut w, std::iter::once(delta)).unwrap();
    assert_eq!(n, 1, "one delta pushed");
    drop(server); // close → the client reads the buffered frame then EOF

    // the client receives a frame-type-tagged SubscriptionPush carrying the delta
    let frame: ServerFrame = recv(&client);
    match frame {
        ServerFrame::SubscriptionPush(d) => {
            assert_eq!(d.projection, ProjectionName::Session);
            assert_eq!(d.kind, DeltaKind::Upsert);
            assert_eq!(d.id.as_deref(), Some("sess_1"));
        }
        ServerFrame::RpcResponse(_) => panic!("expected a SubscriptionPush, got an RpcResponse"),
        ServerFrame::TerminalOutput(_) => {
            panic!("expected a SubscriptionPush, got a TerminalOutput")
        }
    }
}

// ---- Test 12 — the frame-type tag distinguishes rpc-response vs push (§6.4) --

#[test]
fn test_frame_type_tag_distinguishes_streams() {
    // both server→client frame kinds carry a distinguishing `frame_type` discriminant so the
    // client can demultiplex one connection. The Terminal-Channel tag space is RESERVED (no
    // variant yet — its encoding is a Phase-3 decision).
    let rpc = ServerFrame::RpcResponse(RpcResponse {
        id: 1,
        result: Some(serde_json::json!({})),
        error: None,
    });
    let push = ServerFrame::SubscriptionPush(ProjectionDelta {
        projection: ProjectionName::Session,
        kind: DeltaKind::Upsert,
        row: None,
        id: None,
    });

    let rpc_json = serde_json::to_value(&rpc).unwrap();
    let push_json = serde_json::to_value(&push).unwrap();
    assert_eq!(
        rpc_json.get("frame_type").and_then(|v| v.as_str()),
        Some("rpc_response"),
        "rpc-response carries its frame-type tag"
    );
    assert_eq!(
        push_json.get("frame_type").and_then(|v| v.as_str()),
        Some("subscription_push"),
        "subscription-push carries its frame-type tag"
    );
    assert_ne!(
        rpc_json.get("frame_type"),
        push_json.get("frame_type"),
        "the two stream kinds are distinguishable on one connection"
    );
}

// ---- Test 13 — the dispatch RECOGNIZES "subscribe" (not unknown_method) ------

#[test]
fn test_subscribe_method_recognized() {
    // the dispatch recognizes "subscribe" as a valid §6.1 method and acks it (the live delta
    // stream over the connection is 1.6-wired). It is NOT unknown_method.
    let (_d, path) = temp_db();
    seed_session(&path);
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;

    let req = RpcRequest {
        method: "subscribe".to_string(),
        params: serde_json::to_value(SubscribeParams {
            projection: ProjectionName::Session,
            filter: None,
        })
        .unwrap(),
        id: 9,
    };
    let responses = std::thread::scope(|s| {
        let h = s.spawn(|| client_session(&client, std::slice::from_ref(&req)));
        serve_connection(
            server,
            uid,
            uid,
            &path,
            no_deltas(),
            &nexusopsd::runtime::WriteHandle::disconnected(),
            &decision_registry(),
        )
        .expect("serve subscribe ack");
        h.join().unwrap()
    });

    assert!(
        responses[0].error.is_none(),
        "subscribe is recognized, not unknown_method"
    );
    assert_eq!(
        responses[0]
            .result
            .as_ref()
            .unwrap()
            .get("subscribed")
            .and_then(|v| v.as_str()),
        Some("Session"),
        "the ack echoes the validated projection name"
    );
}

// ---- Test 14 — ServerFrame round-trips from the wire (serde tag robustness) --

#[test]
fn test_server_frame_roundtrips_from_wire() {
    // pin the internally-tagged `ServerFrame` + `deny_unknown_fields`-on-inner combo against a
    // serde upgrade: serialize → bytes → deserialize must reconstruct (both variants, both delta
    // kinds). ServerFrame isn't PartialEq, so compare via the JSON value.
    let frames = vec![
        ServerFrame::RpcResponse(RpcResponse {
            id: 5,
            result: Some(serde_json::json!({"k": "v"})),
            error: None,
        }),
        ServerFrame::SubscriptionPush(ProjectionDelta {
            projection: ProjectionName::Session,
            kind: DeltaKind::Upsert,
            row: Some(serde_json::json!({"a": 1})),
            id: Some("x".to_string()),
        }),
        ServerFrame::SubscriptionPush(ProjectionDelta {
            projection: ProjectionName::Session,
            kind: DeltaKind::Remove,
            row: None,
            id: Some("y".to_string()),
        }),
    ];
    for f in &frames {
        let bytes = serde_json::to_vec(f).unwrap();
        let back: ServerFrame = serde_json::from_slice(&bytes).expect("ServerFrame round-trips");
        assert_eq!(
            serde_json::to_value(f).unwrap(),
            serde_json::to_value(&back).unwrap(),
            "the frame-type tag survives the serialize→deserialize round-trip"
        );
    }
}

// ---- Test 11 (1.6d) — a subscribed client receives the LIVE SubscriptionPush frame ----

#[test]
fn test_subscriber_receives_delta_frame() {
    use nexusops_shared::ipc::{
        DeltaKind, ProjectionDelta, ProjectionName, RpcRequest, ServerFrame, SubscribeParams,
    };
    use tokio::sync::broadcast;

    // §6.1 subscribe LIVE (1.6d closes the carved-out 1.6b L4 piece): after acking a subscribe,
    // the serve layer subscribes a broadcast::Receiver BEFORE writing the ack (so a delta published
    // right after the ack is never missed), try_clones the write half, and pushes each MATCHING
    // ProjectionDelta as a frame-tagged ServerFrame::SubscriptionPush while the read half blocks.
    let (_d, path) = temp_db();
    seed_session(&path);
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;
    // `_keepalive` retains a Receiver so the channel stays open + `send` never errors "no receivers";
    // `deltas` is the test's publish handle (standing in for the write-actor's post-commit broadcast).
    // `_keepalive` is HELD alive (a named binding, not a dropped `_`) so the channel stays open +
    // `send` never errors "no receivers"; it just isn't read.
    let (deltas, _keepalive) = broadcast::channel::<ProjectionDelta>(16);

    // the client signals "subscribed (ack read)" so the test publishes only once the serve-side
    // receiver is live (broadcast delivers only to receivers present at send time).
    let (subscribed_tx, subscribed_rx) = std::sync::mpsc::channel::<()>();

    let push = std::thread::scope(|s| {
        let deltas_serve = deltas.clone();
        s.spawn(move || {
            let _ = serve_connection(
                server,
                uid,
                uid,
                &path,
                deltas_serve,
                &nexusopsd::runtime::WriteHandle::disconnected(),
                &decision_registry(),
            );
        });
        let client_h = s.spawn(move || {
            send(
                &client,
                &HelloFrame {
                    protocol_version: 1,
                    client_kind: "desktop_ui".to_string(),
                    app_version: "0.1.0".to_string(),
                },
            );
            let _ack: HelloAck = recv(&client);
            send(
                &client,
                &RpcRequest {
                    method: "subscribe".to_string(),
                    params: serde_json::to_value(SubscribeParams {
                        projection: ProjectionName::Session,
                        filter: None,
                    })
                    .unwrap(),
                    id: 1,
                },
            );
            let _sub_ack: ServerFrame = recv(&client); // the subscribe ack (frame-tagged RpcResponse)
            subscribed_tx.send(()).unwrap();
            recv::<ServerFrame>(&client) // the next server→client frame is the live push
        });
        subscribed_rx.recv().unwrap();
        deltas
            .send(ProjectionDelta {
                projection: ProjectionName::Session,
                kind: DeltaKind::Upsert,
                row: None,
                id: Some("sess_x".to_string()),
            })
            .unwrap();
        client_h.join().unwrap()
    });

    match push {
        ServerFrame::SubscriptionPush(d) => {
            assert_eq!(
                d.projection,
                ProjectionName::Session,
                "the pushed delta is the subscribed projection"
            );
            assert_eq!(
                d.id.as_deref(),
                Some("sess_x"),
                "the pushed delta carries the appended id"
            );
        }
        other => panic!("expected ServerFrame::SubscriptionPush, got {other:?}"),
    }
}

// ---- Test 11b (1.6d) — a subscribe connection is DEDICATED (no concurrent main-loop writes) ----

#[test]
fn test_subscribe_connection_is_dedicated() {
    use nexusops_shared::ipc::{ProjectionDelta, ProjectionName, RpcRequest, SubscribeParams};
    use std::io::Read;
    use tokio::sync::broadcast;

    // after subscribing, the connection is push-only: a further RPC request makes the server CLOSE
    // (no second main-loop response write that could race the push thread → no frame interleave).
    // The client sees EOF, never a second RpcResponse. This pins the §6.4 single-writer enforcement.
    let (_d, path) = temp_db();
    seed_session(&path);
    let (server, client) = UnixStream::pair().unwrap();
    let uid = 1000u32;
    let (deltas, _keepalive) = broadcast::channel::<ProjectionDelta>(16);

    let closed = std::thread::scope(|s| {
        let deltas_serve = deltas.clone();
        s.spawn(move || {
            let _ = serve_connection(
                server,
                uid,
                uid,
                &path,
                deltas_serve,
                &nexusopsd::runtime::WriteHandle::disconnected(),
                &decision_registry(),
            );
        });
        let client_h = s.spawn(move || {
            send(
                &client,
                &HelloFrame {
                    protocol_version: 1,
                    client_kind: "desktop_ui".to_string(),
                    app_version: "0.1.0".to_string(),
                },
            );
            let _ack: HelloAck = recv(&client);
            send(
                &client,
                &RpcRequest {
                    method: "subscribe".to_string(),
                    params: serde_json::to_value(SubscribeParams {
                        projection: ProjectionName::Session,
                        filter: None,
                    })
                    .unwrap(),
                    id: 1,
                },
            );
            let _sub_ack: ServerFrame = recv(&client);
            // a SECOND request on the (now dedicated) subscribe connection → the server closes.
            send(
                &client,
                &RpcRequest {
                    method: "get_capabilities".to_string(),
                    params: serde_json::Value::Null,
                    id: 2,
                },
            );
            // the connection is closed (read returns EOF), never a second RpcResponse.
            let mut buf = [0u8; 1];
            let mut r = &client;
            r.read(&mut buf).unwrap()
        });
        client_h.join().unwrap()
    });

    assert_eq!(
        closed, 0,
        "a request after subscribe → the dedicated connection is CLOSED (EOF), never a 2nd response"
    );
}

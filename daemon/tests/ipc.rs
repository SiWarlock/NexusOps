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
//!   L4 (11–12) — subscribe streaming + Terminal-Channel frame-type seam.              [pending]

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

    let outcome = serve_connection(server, daemon_uid + 1, daemon_uid, &path);
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
    serve_connection(server, uid, uid, &path).expect("authorized in-range handshake succeeds");

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
    let outcome = serve_connection(server, uid, uid, &path);
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
    let outcome = serve_connection(server, uid, uid, &path);
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
    requests.iter().map(|_| recv(client)).collect()
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
        serve_connection(server, uid, uid, &path).expect("serve get_projection");
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
        serve_connection(server, uid, uid, &path).expect("serve unfed projection");
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
        serve_connection(server, uid, uid, &path).expect("serve capabilities + unknown");
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
        serve_connection(server, uid, uid, &path).expect("serve scoped projection");
        h.join().unwrap()
    });

    let rows = responses[0].result.as_ref().unwrap().as_array().unwrap();
    assert_eq!(
        rows.len(),
        1,
        "scope is accepted but NOT enforced in MVP — the read is unscoped (all rows return)"
    );
}

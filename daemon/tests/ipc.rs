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
//!   L3 (8–10) — JSON-RPC dispatch + get_projection + get_capabilities.                [pending]
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

    let outcome = serve_connection(server, daemon_uid + 1, daemon_uid);
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

    // same-uid auth passes → in-range handshake → HelloAck written, handler returns Ok
    serve_connection(server, uid, uid).expect("authorized in-range handshake succeeds");

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
    let outcome = serve_connection(server, uid, uid);
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

    let outcome = serve_connection(server, uid, uid);
    assert!(
        matches!(outcome, Err(IpcError::Protocol(_))),
        "a method frame before the handshake is a protocol violation"
    );
    // the daemon wrote a structured WireError frame before disconnecting (the closest §6.4 code
    // for a non-handshake first frame is unknown_method — see the §6.4 error-code-gap flag)
    let err: WireError = recv(&client);
    assert_eq!(err.code, IpcErrorCode::UnknownMethod);
    // …and then disconnected: no method was served
    let mut r = &client;
    let mut buf = [0u8; 1];
    assert_eq!(
        r.read(&mut buf).unwrap(),
        0,
        "disconnected; the pre-handshake method was never served"
    );
}

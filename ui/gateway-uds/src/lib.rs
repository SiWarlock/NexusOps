//! The ui-side UDS read-transport core (P6.8 L1 — §6.4 / §6.1).
//!
//! A pure-Rust client of the daemon's frozen `§6.4` wire contract: the 4-byte-BE-len + JSON
//! frame codec (8 MiB anti-DoS bound), the `HelloFrame`→`HelloAck` handshake (version-skew
//! fail-closed), and the single-shot read RPC (`RpcRequest` → `ServerFrame::RpcResponse` demux
//! → result / `WireError`) for the read methods `get_projection` / `get_diff` /
//! `get_capabilities`. The deterministic core works over a generic `Read + Write` stream so it
//! is TDD'd against a fake in-memory stream; the real `UnixStream` connect is a thin adapter
//! ([`connect_and_call`], exercised by the `#[ignore]` integration test). Seeded from the
//! daemon's `nexusopsd smoke` dev-client (`daemon/src/smoke.rs` `call()`) + the codec
//! (`daemon/src/ipc/transport.rs`). Depends ONLY on `nexusops-shared` — never the daemon crate.
//!
//! **NON-cat-1: reads only** — no `submit_action`/`approve`/`deny`. INV-SEC-1 stays daemon-side.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use nexusops_shared::ipc::{
    Capabilities, DiffResult, GetDiffParams, GetProjectionParams, HelloAck, HelloFrame,
    IpcErrorCode, ProjectionName, ProjectionScope, RpcRequest, ServerFrame, VersionSkewError,
    PROTOCOL_VERSION,
};
use serde_json::Value;

/// Anti-DoS cap on a single frame's JSON body (§6.4) — mirrors the daemon's `MAX_FRAME_SIZE`.
/// The declared length is validated against this **before** the body buffer is allocated.
pub const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;

/// the GatewayPort UDS file within the daemon's app-support dir (mirrors `smoke.rs`).
const SOCKET_FILE: &str = "gateway.sock";

/// the default per-call read deadline so a stalled daemon can't hang the client forever.
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// the client's offered protocol version + identity for the handshake.
const CLIENT_KIND: &str = "nexusops-ui";

/// A typed transport-client error (§6.4 fail-closed). `Wire` carries the verbatim §6.4 code so
/// the TS layer (050) maps it through `describeRejection`; the rest are fail-closed transport faults.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("frame too large: declared {declared} bytes > max {max}")]
    FrameTooLarge { declared: usize, max: usize },
    #[error("(de)serialize: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("version skew: client offered {client_protocol_version}, daemon supports {supported_min}..={supported_max}")]
    VersionSkew {
        supported_min: u32,
        supported_max: u32,
        client_protocol_version: u32,
    },
    /// The daemon rejected the request with a structured §6.4 code (verbatim, never remapped).
    #[error("daemon rejected the request: {0:?}")]
    Wire(IpcErrorCode),
    /// A wire/frame-discipline violation (a non-RpcResponse frame on a single-shot read, an
    /// id-mismatch, a non-Hello/non-skew handshake frame). Fail-closed.
    #[error("protocol error: {0}")]
    Protocol(String),
}

// ─── the §6.4 frame codec (4-byte-BE-len + JSON; 8 MiB bound) ────────────────────────────────

/// Encode a JSON `body` as a wire frame (4-byte BE length prefix + body); refuses a body over
/// [`MAX_FRAME_SIZE`].
pub fn encode_frame(body: &[u8]) -> Result<Vec<u8>, ClientError> {
    if body.len() > MAX_FRAME_SIZE {
        return Err(ClientError::FrameTooLarge {
            declared: body.len(),
            max: MAX_FRAME_SIZE,
        });
    }
    let mut out = Vec::with_capacity(4 + body.len());
    // body.len() <= MAX_FRAME_SIZE (8 MiB) << u32::MAX, so the cast cannot truncate.
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    Ok(out)
}

/// Decode the declared body length from a 4-byte BE prefix, rejecting a length over
/// [`MAX_FRAME_SIZE`] **before** any body allocation (the anti-DoS pin).
pub fn decode_len(prefix: &[u8; 4]) -> Result<usize, ClientError> {
    let len = u32::from_be_bytes(*prefix) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(ClientError::FrameTooLarge {
            declared: len,
            max: MAX_FRAME_SIZE,
        });
    }
    Ok(len)
}

/// Read one wire frame: the 4-byte BE prefix, then the bounded body (length checked pre-alloc).
pub fn read_frame<R: Read>(r: &mut R) -> Result<Vec<u8>, ClientError> {
    let mut prefix = [0u8; 4];
    r.read_exact(&mut prefix)?;
    let len = decode_len(&prefix)?; // rejects > MAX_FRAME_SIZE BEFORE the alloc below
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok(body)
}

/// Write `body` as one wire frame (encode + write_all).
pub fn write_frame<W: Write>(w: &mut W, body: &[u8]) -> Result<(), ClientError> {
    w.write_all(&encode_frame(body)?)?;
    Ok(())
}

// ─── handshake + single-shot call ────────────────────────────────────────────────────────────

/// §6.4 handshake-first: write a `HelloFrame{protocol_version:1,…}`, read + validate the first
/// frame. A valid `HelloAck` → `Ok(ack)`; a `VersionSkewError` → `Err(ClientError::VersionSkew)`;
/// any other first frame → `Err(ClientError::Protocol)`. Fail-closed: never proceeds to RPC.
pub fn handshake<S: Read + Write>(
    stream: &mut S,
    client_kind: &str,
    app_version: &str,
) -> Result<HelloAck, ClientError> {
    let hello = HelloFrame {
        protocol_version: PROTOCOL_VERSION,
        client_kind: client_kind.to_string(),
        app_version: app_version.to_string(),
    };
    write_frame(stream, &serde_json::to_vec(&hello)?)?;
    let body = read_frame(stream)?;
    // The daemon sends a BARE HelloAck (success) or a BARE VersionSkewError (skew) pre-RPC
    // (verified server.rs). Their field-sets are disjoint under deny_unknown_fields, so try
    // HelloAck first, then VersionSkewError; neither → a Protocol error (fail-closed, no RPC).
    if let Ok(ack) = serde_json::from_slice::<HelloAck>(&body) {
        return Ok(ack);
    }
    if let Ok(skew) = serde_json::from_slice::<VersionSkewError>(&body) {
        return Err(ClientError::VersionSkew {
            supported_min: skew.supported_min,
            supported_max: skew.supported_max,
            client_protocol_version: skew.client_protocol_version,
        });
    }
    Err(ClientError::Protocol(format!(
        "handshake: first frame ({} bytes) is neither a HelloAck nor a VersionSkewError",
        body.len()
    )))
}

/// One single-shot read RPC: write `RpcRequest{method,params,id}`, read one `ServerFrame`,
/// demux `RpcResponse` by `id`. `result` → `Ok(value)`; `WireError` → `Err(Wire(code))` (verbatim);
/// a non-`RpcResponse` frame or an id-mismatch → `Err(Protocol)`.
pub fn call<S: Read + Write>(
    stream: &mut S,
    method: &str,
    params: Value,
    id: u64,
) -> Result<Value, ClientError> {
    let req = RpcRequest {
        method: method.to_string(),
        params,
        id,
    };
    write_frame(stream, &serde_json::to_vec(&req)?)?;
    let body = read_frame(stream)?;
    // parse-don't-trust: deny_unknown_fields on the shared frame types rejects a malformed/
    // extra-field frame as a serde error (fail-closed).
    let frame: ServerFrame = serde_json::from_slice(&body)?;
    match frame {
        ServerFrame::RpcResponse(resp) => {
            // demux by id FIRST — an uncorrelated/stale response is a protocol violation, not a
            // result to surface (even if it carries one).
            if resp.id != id {
                return Err(ClientError::Protocol(format!(
                    "response id {} does not correlate to request id {id}",
                    resp.id
                )));
            }
            // a WireError → the verbatim §6.4 code (never collapsed/remapped).
            if let Some(err) = resp.error {
                return Err(ClientError::Wire(err.code));
            }
            // §6.1: a response carries EXACTLY ONE of result / error. A dual-None frame violates
            // the contract → Protocol (fail-closed). An explicit `result: null` is Some(Null), a
            // valid no-body success, and passes. (Stricter than the smoke.rs DEV-client, which is
            // explicitly not-fail-closed — a production transport client honors the contract.)
            match resp.result {
                Some(value) => Ok(value),
                None => Err(ClientError::Protocol(format!(
                    "RpcResponse for id {id} carried neither a result nor an error (§6.1 requires exactly one)"
                ))),
            }
        }
        // a SubscriptionPush / TerminalOutput on a single-shot read is a frame-discipline error.
        other => Err(ClientError::Protocol(format!(
            "expected an RpcResponse on a single-shot read, got {other:?}"
        ))),
    }
}

// ─── typed read helpers (form the params the daemon expects — match methods.rs) ───────────────

/// `get_projection` — forms `GetProjectionParams{name,scope?}` + calls. Returns the raw page Value
/// (the TS layer Zod-validates the page shape; this crate validates the frame envelope).
pub fn get_projection<S: Read + Write>(
    stream: &mut S,
    name: ProjectionName,
    scope: Option<ProjectionScope>,
    id: u64,
) -> Result<Value, ClientError> {
    let params = serde_json::to_value(GetProjectionParams { name, scope })?;
    call(stream, "get_projection", params, id)
}

/// `get_diff` — forms `GetDiffParams{worktree_id,file}` + calls; deserializes the `DiffResult`.
pub fn get_diff<S: Read + Write>(
    stream: &mut S,
    worktree_id: &str,
    file: &str,
    id: u64,
) -> Result<DiffResult, ClientError> {
    let params = serde_json::to_value(GetDiffParams {
        worktree_id: worktree_id.to_string(),
        file: file.to_string(),
    })?;
    let value = call(stream, "get_diff", params, id)?;
    Ok(serde_json::from_value(value)?)
}

/// `get_capabilities` — the no-param RPC (methods.rs:68); deserializes the `Capabilities`.
pub fn get_capabilities<S: Read + Write>(
    stream: &mut S,
    id: u64,
) -> Result<Capabilities, ClientError> {
    let value = call(stream, "get_capabilities", serde_json::json!({}), id)?;
    Ok(serde_json::from_value(value)?)
}

// ─── the real-socket adapter (thin; the `#[ignore]` integration test exercises it) ───────────

/// Resolve the daemon's GatewayPort UDS path (the app-support dir `main.rs` binds). The path is
/// **macOS-specific** (`~/Library/Application Support/…`) — NexusOps is a macOS-only product (the
/// daemon's `smoke.rs` uses the same path); a Linux build would resolve the wrong path.
pub fn socket_path() -> Result<PathBuf, ClientError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| ClientError::Protocol("HOME is not set".to_string()))?;
    Ok(PathBuf::from(home)
        .join("Library/Application Support/NexusOps")
        .join(SOCKET_FILE))
}

/// Connect → handshake → one `call` → drop (the smoke.rs one-shot pattern). The L1 connection
/// model; a persistent + dedicated-subscribe connection rides slice 051. Every read is bounded
/// by [`DEFAULT_READ_TIMEOUT`] so a stalled daemon can't hang the caller.
pub fn connect_and_call(method: &str, params: Value) -> Result<Value, ClientError> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path)?;
    stream.set_read_timeout(Some(DEFAULT_READ_TIMEOUT))?;
    handshake(&mut stream, CLIENT_KIND, env!("CARGO_PKG_VERSION"))?;
    call(&mut stream, method, params, 1)
    // stream drops here (the one-shot connection model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A fake bidirectional stream: pre-loaded daemon→client bytes (`reads`) + a capture buffer
    /// for client→daemon bytes (`writes`). No real socket, no daemon (the L1 TDD seam).
    struct FakeStream {
        reads: Cursor<Vec<u8>>,
        writes: Vec<u8>,
    }
    impl FakeStream {
        fn new(reads: Vec<u8>) -> Self {
            FakeStream {
                reads: Cursor::new(reads),
                writes: Vec::new(),
            }
        }
    }
    impl Read for FakeStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.reads.read(buf)
        }
    }
    impl Write for FakeStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.writes.write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Frame a body MANUALLY (independent of the impl under test) so the RED is the function
    /// under test failing, not the test setup.
    fn framed(body: &[u8]) -> Vec<u8> {
        let mut v = (body.len() as u32).to_be_bytes().to_vec();
        v.extend_from_slice(body);
        v
    }
    fn frame_json<T: serde::Serialize>(v: &T) -> Vec<u8> {
        framed(&serde_json::to_vec(v).unwrap())
    }
    fn hello_ack() -> HelloAck {
        HelloAck {
            protocol_version: 1,
            daemon_version: "0.0.0".into(),
            capabilities: Capabilities {
                protocol_version: 1,
                contract_version: "0.28.0".into(),
            },
        }
    }

    #[test]
    fn frame_roundtrips() {
        // spec(§6.4) — encode→decode preserves the body; a 4-byte BE length prefix.
        let body = br#"{"x":1}"#;
        let mut w = Vec::new();
        write_frame(&mut w, body).unwrap();
        assert_eq!(&w[..4], &(body.len() as u32).to_be_bytes()); // 4-byte BE len
        let mut r = Cursor::new(w);
        assert_eq!(read_frame(&mut r).unwrap(), body);
    }

    #[test]
    fn oversized_frame_rejected_before_alloc() {
        // spec(§6.4) — a declared length > 8 MiB → FrameTooLarge from the PREFIX, never allocates.
        let prefix = ((MAX_FRAME_SIZE + 1) as u32).to_be_bytes();
        assert!(matches!(
            decode_len(&prefix),
            Err(ClientError::FrameTooLarge { .. })
        ));
        // read_frame must reject from the prefix alone (body absent — proving no alloc/read of it).
        let mut r = Cursor::new(prefix.to_vec());
        assert!(matches!(
            read_frame(&mut r),
            Err(ClientError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn handshake_writes_hello_reads_ack() {
        // spec(§6.4) — handshake writes a HelloFrame{protocol_version:1} + accepts a valid HelloAck.
        let mut s = FakeStream::new(frame_json(&hello_ack()));
        let ack = handshake(&mut s, "nexusops-ui", "0.0.0").unwrap();
        assert_eq!(ack.capabilities.contract_version, "0.28.0");
        // the client wrote a framed HelloFrame with protocol_version 1.
        let written_body = &s.writes[4..];
        let hello: HelloFrame = serde_json::from_slice(written_body).unwrap();
        assert_eq!(hello.protocol_version, 1);
    }

    #[test]
    fn handshake_version_skew_fails_closed() {
        // spec(§6.4) — a VersionSkewError first frame → typed VersionSkew error, no RPC issued.
        let skew = VersionSkewError {
            supported_min: 2,
            supported_max: 2,
            client_protocol_version: 1,
        };
        let mut s = FakeStream::new(frame_json(&skew));
        assert!(matches!(
            handshake(&mut s, "nexusops-ui", "0.0.0"),
            Err(ClientError::VersionSkew { .. })
        ));
        // a non-Hello/non-skew first frame (e.g. an RpcResponse) → Protocol, fail-closed.
        let frame = ServerFrame::RpcResponse(nexusops_shared::ipc::RpcResponse {
            id: 1,
            result: Some(Value::Null),
            error: None,
        });
        let mut s2 = FakeStream::new(frame_json(&frame));
        assert!(matches!(
            handshake(&mut s2, "nexusops-ui", "0.0.0"),
            Err(ClientError::Protocol(_))
        ));
    }

    #[test]
    fn call_demuxes_rpc_response_result() {
        // spec(§6.1/§6.4) — an RpcResponse{id,result} matching the request id → Ok(value).
        let frame = ServerFrame::RpcResponse(nexusops_shared::ipc::RpcResponse {
            id: 7,
            result: Some(serde_json::json!({ "ok": true })),
            error: None,
        });
        let mut s = FakeStream::new(frame_json(&frame));
        let got = call(&mut s, "get_capabilities", serde_json::json!({}), 7).unwrap();
        assert_eq!(got, serde_json::json!({ "ok": true }));
    }

    #[test]
    fn call_wire_error_returns_typed_code() {
        // spec(§6.4) — an RpcResponse{error: WireError{code}} → Err(Wire(code)) (verbatim §6.4 code).
        let frame = ServerFrame::RpcResponse(nexusops_shared::ipc::RpcResponse {
            id: 7,
            result: None,
            error: Some(nexusops_shared::ipc::WireError {
                code: IpcErrorCode::NotFound,
            }),
        });
        let mut s = FakeStream::new(frame_json(&frame));
        assert!(matches!(
            call(&mut s, "get_diff", serde_json::json!({}), 7),
            Err(ClientError::Wire(IpcErrorCode::NotFound))
        ));
    }

    #[test]
    fn call_non_rpcresponse_frame_is_protocol_error() {
        // spec(§6.4) — a non-RpcResponse frame on a single-shot read → Protocol error.
        let term = ServerFrame::TerminalOutput(nexusops_shared::ipc::TerminalOutputFrame {
            terminal_id: "t1".into(),
            seq: 0,
            data: "aGk=".into(),
        });
        let mut s = FakeStream::new(frame_json(&term));
        assert!(matches!(
            call(&mut s, "get_capabilities", serde_json::json!({}), 7),
            Err(ClientError::Protocol(_))
        ));
        // an id-mismatch is also a protocol error (a stale/uncorrelated response).
        let frame = ServerFrame::RpcResponse(nexusops_shared::ipc::RpcResponse {
            id: 99,
            result: Some(Value::Null),
            error: None,
        });
        let mut s2 = FakeStream::new(frame_json(&frame));
        assert!(matches!(
            call(&mut s2, "get_capabilities", serde_json::json!({}), 7),
            Err(ClientError::Protocol(_))
        ));
    }

    #[test]
    fn subscription_push_frame_is_protocol_error() {
        // spec(§6.4) — a SubscriptionPush (the streaming variant, slice 051) on a single-shot
        // read is also a frame-discipline error (the third ServerFrame variant, by name).
        let push = ServerFrame::SubscriptionPush(nexusops_shared::ipc::ProjectionDelta {
            projection: nexusops_shared::ipc::ProjectionName::Session,
            kind: nexusops_shared::ipc::DeltaKind::Upsert,
            id: None,
            row: None,
        });
        let mut s = FakeStream::new(frame_json(&push));
        assert!(matches!(
            call(&mut s, "get_projection", serde_json::json!({}), 7),
            Err(ClientError::Protocol(_))
        ));
    }

    #[test]
    fn response_without_result_or_error_is_protocol_error() {
        // spec(§6.1) — a response carries EXACTLY ONE of result/error; a dual-None frame violates
        // the contract → Protocol (fail-closed; stricter than the not-fail-closed dev-client seed).
        let frame = ServerFrame::RpcResponse(nexusops_shared::ipc::RpcResponse {
            id: 7,
            result: None,
            error: None,
        });
        let mut s = FakeStream::new(frame_json(&frame));
        assert!(matches!(
            call(&mut s, "get_capabilities", serde_json::json!({}), 7),
            Err(ClientError::Protocol(_))
        ));
    }

    #[test]
    fn unknown_field_in_frame_rejected() {
        // spec(§5.0) — parse-don't-trust: a frame with an extra field → a serde parse error
        // (deny_unknown_fields on the shared frame types).
        let body = br#"{"frame_type":"rpc_response","id":7,"result":null,"bogus":1}"#;
        let mut s = FakeStream::new(framed(body));
        assert!(matches!(
            call(&mut s, "get_capabilities", serde_json::json!({}), 7),
            Err(ClientError::Serde(_))
        ));
    }

    // ─── the typed read helpers (direct coverage — params formation + typed deserialize) ─────

    /// Build a framed RpcResponse{id, result} for a read-helper happy path.
    fn rpc_ok(id: u64, result: serde_json::Value) -> Vec<u8> {
        frame_json(&ServerFrame::RpcResponse(
            nexusops_shared::ipc::RpcResponse {
                id,
                result: Some(result),
                error: None,
            },
        ))
    }

    #[test]
    fn get_diff_returns_typed_diffresult() {
        // spec(§6.1) — get_diff forms GetDiffParams + calls + deserializes the typed DiffResult.
        let diff = serde_json::json!({
            "hunks": [{
                "header": "@@ -1,1 +1,1 @@",
                "old_start": 1, "old_lines": 1, "new_start": 1, "new_lines": 1,
                "lines": [{ "kind": "context", "content": "x\n" }]
            }]
        });
        let mut s = FakeStream::new(rpc_ok(3, diff));
        let result = get_diff(&mut s, "wt_1", "a.ts", 3).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].old_start, 1);
        // the request carried the get_diff method + the GetDiffParams the daemon expects.
        let req: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(req.method, "get_diff");
        assert_eq!(req.params["worktree_id"], "wt_1");
        assert_eq!(req.params["file"], "a.ts");
    }

    #[test]
    fn get_diff_malformed_result_is_serde_error() {
        // spec(§5.0) — a structurally valid RpcResponse whose result is NOT a DiffResult → a typed
        // Serde error (the typed-deserialize fail-closed path), never a bad partial value.
        let mut s = FakeStream::new(rpc_ok(3, serde_json::json!({ "not_a_diff": true })));
        assert!(matches!(
            get_diff(&mut s, "wt_1", "a.ts", 3),
            Err(ClientError::Serde(_))
        ));
    }

    #[test]
    fn get_capabilities_returns_typed_capabilities() {
        // spec(§6.1/§6.4) — get_capabilities (no-param RPC) deserializes the typed Capabilities.
        let caps = serde_json::json!({ "protocol_version": 1, "contract_version": "0.28.0" });
        let mut s = FakeStream::new(rpc_ok(5, caps));
        let got = get_capabilities(&mut s, 5).unwrap();
        assert_eq!(got.protocol_version, 1);
        assert_eq!(got.contract_version, "0.28.0");
    }

    #[test]
    fn get_projection_forms_params_and_returns_value() {
        // spec(§6.1) — get_projection forms GetProjectionParams{name} + returns the raw page Value
        // (the TS layer Zod-validates the page shape; the crate validates the frame envelope).
        let page = serde_json::json!({ "projection": "Session", "rows": [] });
        let mut s = FakeStream::new(rpc_ok(9, page.clone()));
        let got = get_projection(&mut s, ProjectionName::Session, None, 9).unwrap();
        assert_eq!(got, page);
        let req: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(req.method, "get_projection");
        assert_eq!(req.params["name"], "Session");
    }
}

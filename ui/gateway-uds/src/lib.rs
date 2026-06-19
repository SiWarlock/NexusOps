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
//! **Mutation-intent RPCs (L2-A):** the crate also carries `submit_action`/`preview_action`/
//! `approve`/`deny` — transport only (each SENDS a typed RPC to the daemon's Action Gateway, the
//! INV-SEC-1 chokepoint; no execution/mutation here, L2-D1 pure pass-through). They REUSE `call` +
//! `demux_rpc_response` so the verbatim §6.4 `WireError`→code path is inherited identically. NO UI
//! consumer reaches them yet — the Tauri command + the TS caller are L2-B; the live submit-enable is
//! L2-C (USER-gated). INV-SEC-1 stays daemon-side regardless.

use std::io::{Read, Write};
use std::ops::ControlFlow;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::ipc::{
    ActionAck, Capabilities, DiffResult, GetDiffParams, GetProjectionParams, HelloAck, HelloFrame,
    IpcErrorCode, ProjectionDelta, ProjectionName, ProjectionScope, RpcRequest, ServerFrame,
    SubscribeParams, VersionSkewError, PROTOCOL_VERSION,
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
    demux_rpc_response(frame, id)
}

/// Demux a `ServerFrame` as the `RpcResponse` correlated to request `id`: id-mismatch / a
/// non-`RpcResponse` frame / a dual-None body → `Protocol` (fail-closed); a `WireError` → the
/// verbatim §6.4 `Wire(code)`; a `result` → `Ok(value)`. Shared by the single-shot [`call`] and
/// the subscribe ACK (`subscribe_stream`) so both honor the §6.1 exactly-one-of contract identically.
fn demux_rpc_response(frame: ServerFrame, id: u64) -> Result<Value, ClientError> {
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
        // a SubscriptionPush / TerminalOutput where an RpcResponse was expected is a discipline error.
        other => Err(ClientError::Protocol(format!(
            "expected an RpcResponse, got {other:?}"
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

// ─── typed mutation-intent helpers (L2-A; transport only, INV-SEC-1 daemon-side) ──────────────
// Each SENDS a typed §6.1 RPC + returns the daemon's typed result — NO execution/mutation here
// (L2-D1 pure pass-through). They reuse `call`/`demux_rpc_response`, so a `WireError{code}` →
// `Err(Wire(code))` VERBATIM (never collapsed — L2-D6, the distinct §11.5 rejection cards at L2-C
// depend on it) and the fail-closed frame discipline (non-RpcResponse/id-mismatch/dual-None →
// `Protocol`; a mis-typed result → `Serde`) are inherited identically from the read path.

/// `submit_action` — serializes the `ActionRequest` AS-IS (its `idempotency_key`/`fencing_token`/
/// `resource_refs` ride OPAQUELY to the daemon, which owns dedup+fencing — L2-O4 pass-through; the
/// crate forms no tokens) + calls; deserializes the typed `ActionAck`.
pub fn submit_action<S: Read + Write>(
    stream: &mut S,
    request: &ActionRequest,
    id: u64,
) -> Result<ActionAck, ClientError> {
    let value = call(stream, "submit_action", serde_json::to_value(request)?, id)?;
    Ok(serde_json::from_value(value)?)
}

/// `preview_action` — forms `{action_request_id}` (methods.rs:441) for a prior-submitted action +
/// calls; deserializes the typed `ActionPreview`. (L2-O2: the preview helper lands in slice A
/// alongside submit, so the L2-C modal can fetch the daemon's real risk/consequences before approving.)
pub fn preview_action<S: Read + Write>(
    stream: &mut S,
    action_request_id: &str,
    id: u64,
) -> Result<ActionPreview, ClientError> {
    let params = serde_json::json!({ "action_request_id": action_request_id });
    let value = call(stream, "preview_action", params, id)?;
    Ok(serde_json::from_value(value)?)
}

/// `approve` — forms `{approval_id, step_id?}` (methods.rs:212; `step_id` is accepted-but-RESERVED
/// by the daemon in 2.1c — included only when `Some`, absent [not null] when `None`) + calls;
/// deserializes the typed `ActionAck`.
pub fn approve<S: Read + Write>(
    stream: &mut S,
    approval_id: &str,
    step_id: Option<&str>,
    id: u64,
) -> Result<ActionAck, ClientError> {
    let params = match step_id {
        Some(step) => serde_json::json!({ "approval_id": approval_id, "step_id": step }),
        None => serde_json::json!({ "approval_id": approval_id }),
    };
    let value = call(stream, "approve", params, id)?;
    Ok(serde_json::from_value(value)?)
}

/// `deny` — forms `{approval_id, reason}` (methods.rs:227) + calls; deserializes the typed `ActionAck`.
pub fn deny<S: Read + Write>(
    stream: &mut S,
    approval_id: &str,
    reason: &str,
    id: u64,
) -> Result<ActionAck, ClientError> {
    let params = serde_json::json!({ "approval_id": approval_id, "reason": reason });
    let value = call(stream, "deny", params, id)?;
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

// ─── the subscribe stream (052 — the dedicated persistent push-read connection) ───────────────

/// Subscribe to a projection's live delta stream over a DEDICATED persistent connection: write the
/// `subscribe` `RpcRequest` (id `id`) → read + demux the ack `RpcResponse` (a `WireError` ack →
/// `Err(Wire)`, fail-closed exactly as [`call`] via the shared [`demux_rpc_response`]) → then loop
/// `read_frame` → demux each `ServerFrame::SubscriptionPush(ProjectionDelta)` to `on_delta`. The
/// daemon makes a subscribe connection terminal/single-writer and **closes it on lag**
/// (`ProjectionDelta` carries no `seq` → a gap is client-undetectable; recovery is reconnect +
/// re-`get_projection`, NOT client gap-fill — daemon LESSON 12 / §11.7): so a clean EOF/`Io` on the
/// read loop **terminates the stream successfully** (`Ok(())`), not as a fault. A non-`SubscriptionPush`
/// frame mid-stream → `Protocol`. The 8 MiB per-frame bound holds (the codec rejects an oversized
/// frame pre-alloc). `on_delta` returns [`ControlFlow::Break`] to stop early (e.g. the downstream
/// sink/channel was dropped — the teardown path, so a dropped subscriber leaves no spinning read).
pub fn subscribe_stream<S, F>(
    stream: &mut S,
    projection: ProjectionName,
    id: u64,
    mut on_delta: F,
) -> Result<(), ClientError>
where
    S: Read + Write,
    F: FnMut(ProjectionDelta) -> ControlFlow<()>,
{
    // write the subscribe RPC + validate the ack (the daemon spawns its push thread only on an
    // accepted ack; a WireError ack → fail-closed, the push loop is never entered).
    let params = serde_json::to_value(SubscribeParams {
        projection,
        filter: None,
    })?;
    write_frame(
        stream,
        &serde_json::to_vec(&RpcRequest {
            method: "subscribe".to_string(),
            params,
            id,
        })?,
    )?;
    let ack_body = read_frame(stream)?;
    let ack_frame: ServerFrame = serde_json::from_slice(&ack_body)?;
    demux_rpc_response(ack_frame, id)?; // Ok(value) is the echoed {subscribed:…}; only success matters

    // the push-read loop: every frame after the ack is a SubscriptionPush for this projection (the
    // connection is dedicated/single-writer post-ack — daemon LESSON 12).
    loop {
        let body = match read_frame(stream) {
            Ok(b) => b,
            // the daemon closed the connection (lag/EOF) → terminate CLEANLY (the recovery
            // contract: reconnect + re-get_projection, never a gap-fill on a seq-less stream).
            Err(ClientError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(())
            }
            Err(e) => return Err(e), // FrameTooLarge / a real io fault → surface it
        };
        let frame: ServerFrame = serde_json::from_slice(&body)?;
        match frame {
            ServerFrame::SubscriptionPush(delta) => {
                if on_delta(delta).is_break() {
                    return Ok(());
                }
            }
            other => {
                return Err(ClientError::Protocol(format!(
                    "expected a SubscriptionPush mid-stream, got {other:?}"
                )))
            }
        }
    }
}

/// Connect → handshake → [`subscribe_stream`] over a DEDICATED `UnixStream` (the 052 persistent
/// connection model). Unlike [`connect_and_call`], this **deliberately sets NO read timeout**: a
/// healthy idle subscription blocks on the next push indefinitely (deltas are sparse) — only the
/// daemon's lag-close (EOF) ends it; the 30 s single-shot timeout would kill a quiet-but-live
/// subscription. Exactly one RPC is issued (the subscribe, id 1); multiplexed unique-correlation-ids
/// stay deferred (the daemon MVP is 1-subscription-per-connection, terminal post-ack). The
/// `#[ignore]` integration test exercises this against a live daemon.
pub fn connect_and_subscribe<F>(projection: ProjectionName, on_delta: F) -> Result<(), ClientError>
where
    F: FnMut(ProjectionDelta) -> ControlFlow<()>,
{
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path)?;
    // NB: NO set_read_timeout — see the doc comment (a subscribe must not time out while idle).
    handshake(&mut stream, CLIENT_KIND, env!("CARGO_PKG_VERSION"))?;
    subscribe_stream(&mut stream, projection, 1, on_delta)
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

    // ─── subscribe_stream (052 — the dedicated persistent push-read connection) ───────────────

    use std::ops::ControlFlow;

    /// A framed subscribe ACK (RpcResponse{id, result:{subscribed}}) — the daemon's first frame.
    fn subscribe_ack(id: u64, projection: &str) -> Vec<u8> {
        frame_json(&ServerFrame::RpcResponse(
            nexusops_shared::ipc::RpcResponse {
                id,
                result: Some(serde_json::json!({ "subscribed": projection })),
                error: None,
            },
        ))
    }
    /// A framed SubscriptionPush(ProjectionDelta) push frame.
    fn push(delta: nexusops_shared::ipc::ProjectionDelta) -> Vec<u8> {
        frame_json(&ServerFrame::SubscriptionPush(delta))
    }
    fn session_delta(id: &str) -> nexusops_shared::ipc::ProjectionDelta {
        nexusops_shared::ipc::ProjectionDelta {
            projection: ProjectionName::Session,
            kind: nexusops_shared::ipc::DeltaKind::Upsert,
            row: None,
            id: Some(id.to_string()),
        }
    }

    #[test]
    fn subscribe_stream_yields_each_push_delta() {
        // spec(§6.4) — ack then 2 SubscriptionPush frames → both deltas reach the sink, in order.
        let mut reads = subscribe_ack(1, "Session");
        reads.extend(push(session_delta("sess_1")));
        reads.extend(push(session_delta("sess_2")));
        let mut s = FakeStream::new(reads);
        let mut got: Vec<Option<String>> = Vec::new();
        let r = subscribe_stream(&mut s, ProjectionName::Session, 1, |d| {
            got.push(d.id.clone());
            ControlFlow::Continue(())
        });
        assert!(r.is_ok());
        assert_eq!(
            got,
            vec![Some("sess_1".to_string()), Some("sess_2".to_string())]
        );
        // the client wrote a `subscribe` RpcRequest carrying the projection.
        let req: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(req.method, "subscribe");
        assert_eq!(req.params["projection"], "Session");
    }

    #[test]
    fn subscribe_stream_terminates_on_close() {
        // spec(§11.7) — ack then EOF (the daemon closed on lag) → a CLEAN terminate, not a fault;
        // no gap-fill (ProjectionDelta carries no seq — recovery is reconnect, daemon LESSON 12).
        let mut s = FakeStream::new(subscribe_ack(1, "Session")); // ack, then the cursor EOFs
        let mut count = 0;
        let r = subscribe_stream(&mut s, ProjectionName::Session, 1, |_d| {
            count += 1;
            ControlFlow::Continue(())
        });
        assert!(r.is_ok(), "EOF after the ack is a clean close, not a fault");
        assert_eq!(count, 0);
    }

    #[test]
    fn subscribe_ack_wire_error_fails_closed() {
        // spec(§6.4) — a WireError ack → Err(Wire(code)) verbatim; the push loop is never entered.
        let ack = frame_json(&ServerFrame::RpcResponse(
            nexusops_shared::ipc::RpcResponse {
                id: 1,
                result: None,
                error: Some(nexusops_shared::ipc::WireError {
                    code: IpcErrorCode::ProtocolError,
                }),
            },
        ));
        let mut s = FakeStream::new(ack);
        let r = subscribe_stream(&mut s, ProjectionName::Session, 1, |_d| ControlFlow::Continue(()));
        assert!(matches!(
            r,
            Err(ClientError::Wire(IpcErrorCode::ProtocolError))
        ));
    }

    #[test]
    fn subscribe_push_over_8mib_rejected() {
        // spec(§6.4) — an oversized push frame → FrameTooLarge from the prefix alone (pre-alloc).
        let mut reads = subscribe_ack(1, "Session");
        reads.extend_from_slice(&((MAX_FRAME_SIZE + 1) as u32).to_be_bytes()); // oversized prefix, no body
        let mut s = FakeStream::new(reads);
        let r = subscribe_stream(&mut s, ProjectionName::Session, 1, |_d| ControlFlow::Continue(()));
        assert!(matches!(r, Err(ClientError::FrameTooLarge { .. })));
    }

    #[test]
    fn subscribe_non_push_frame_mid_stream_is_protocol() {
        // spec(§6.4) — a non-SubscriptionPush frame after the ack → Protocol (frame-discipline).
        let mut reads = subscribe_ack(1, "Session");
        reads.extend(rpc_ok(2, serde_json::json!({ "x": 1 }))); // an RpcResponse mid-stream
        let mut s = FakeStream::new(reads);
        let r = subscribe_stream(&mut s, ProjectionName::Session, 1, |_d| ControlFlow::Continue(()));
        assert!(matches!(r, Err(ClientError::Protocol(_))));
    }

    #[test]
    fn subscribe_stream_sink_break_stops_early() {
        // the sink can stop the loop (e.g. the Tauri channel closed) — ControlFlow::Break ends it.
        let mut reads = subscribe_ack(1, "Session");
        reads.extend(push(session_delta("sess_1")));
        reads.extend(push(session_delta("sess_2")));
        let mut s = FakeStream::new(reads);
        let mut count = 0;
        let r = subscribe_stream(&mut s, ProjectionName::Session, 1, |_d| {
            count += 1;
            ControlFlow::Break(())
        });
        assert!(r.is_ok());
        assert_eq!(count, 1, "Break after the first delta stops the loop");
    }

    // ─── 055 / L2-A — the mutation-intent RPC helpers (transport only; exposed-ahead) ──────────
    // submit_action / preview_action / approve / deny mirror the read helpers + REUSE call +
    // demux_rpc_response → the verbatim §6.4 WireError code + the fail-closed frame discipline are
    // inherited identically (L2-D6). The UI never mutates — these SEND a typed RPC; the daemon's
    // Action Gateway is the INV-SEC-1 chokepoint (L2-D1 pure pass-through).
    use nexusops_shared::actions::{
        ActionPreview, ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
    };
    use nexusops_shared::ids::ActionRequestId;
    use nexusops_shared::status::ActionRequest as ActionRequestStatus;
    use nexusops_shared::time::Timestamp;

    // ui-067 item 4 — a fixed, deterministic act_<ULID> for the test fixtures (no random
    // ::new() id in a fixture; two calls return equal ids so assertions are reproducible).
    const FIXED_AR_ID: &str = "act_01ARZ3NDEKTSV4RRFFQ69G5FAV";

    /// A sample ActionRequest carrying an idempotency_key + fencing_token + a resource_ref so the
    /// L2-O4 pass-through pin can assert they ride to the daemon opaquely (the crate forms none).
    fn sample_action_request() -> ActionRequest {
        ActionRequest {
            // deterministic fixture id (ui-067 item 4) — never a random ::new()
            action_request_id: ActionRequestId::parse(FIXED_AR_ID).unwrap(),
            project_id: None,
            action_type: "git.stage_hunk".to_string(),
            requester_type: RequesterType::User,
            requester_id: "ui".to_string(),
            resource_refs: vec![ResourceRef {
                resource_type: ResourceType::File,
                id: "wt_1\u{1f}a.ts\u{1f}1,1,1,1".to_string(),
                uri: None,
            }],
            inputs: serde_json::json!({}),
            risk_level: RiskLevel::Level2,
            idempotency_key: Some("idem-abc".to_string()),
            fencing_token: Some(42),
            status: ActionRequestStatus::Submitted,
            preview: None,
            created_at: Timestamp::parse("2026-06-14T00:00:00Z").unwrap(),
        }
    }
    fn sample_ack() -> ActionAck {
        ActionAck {
            action_request_id: "act_test".to_string(),
            status: ActionRequestStatus::Submitted,
        }
    }
    fn sample_preview() -> ActionPreview {
        ActionPreview {
            // deterministic fixture id (ui-067 item 4) — never a random ::new()
            action_request_id: ActionRequestId::parse(FIXED_AR_ID).unwrap(),
            generated_at: Timestamp::parse("2026-06-14T00:00:00Z").unwrap(),
            risk_level: RiskLevel::Level2,
            risk_reasons: vec!["touches a tracked file".to_string()],
            summary: "Would stage 1 hunk.".to_string(),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }

    #[test]
    fn sample_fixtures_use_fixed_action_request_ids() {
        // spec(§6.1) — sample_action_request() + sample_preview() return the FIXED act_<ULID>,
        // never a fresh ::new() id → deterministic fixtures (two calls are equal).
        let a = sample_action_request().action_request_id;
        let b = sample_action_request().action_request_id;
        assert_eq!(a, b);
        assert_eq!(sample_preview().action_request_id, a);
        assert_eq!(a, ActionRequestId::parse(FIXED_AR_ID).unwrap());
    }

    #[test]
    fn submit_action_forms_params_and_returns_ack() {
        // spec(§6.1) — submit_action serializes the ActionRequest AS-IS into params + calls +
        // deserializes the typed ActionAck. L2-O4: idempotency_key/fencing_token/resource_refs ride
        // OPAQUELY (the crate forms none — pure pass-through).
        let req = sample_action_request();
        let mut s = FakeStream::new(rpc_ok(11, serde_json::to_value(sample_ack()).unwrap()));
        let ack = submit_action(&mut s, &req, 11).unwrap();
        assert_eq!(ack.action_request_id, "act_test");
        assert!(matches!(ack.status, ActionRequestStatus::Submitted));
        // the request carried the submit_action method + the ActionRequest fields verbatim.
        let sent: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(sent.method, "submit_action");
        assert_eq!(sent.params["action_type"], "git.stage_hunk");
        assert_eq!(sent.params["idempotency_key"], "idem-abc"); // rides opaquely (L2-O4)
        assert_eq!(sent.params["fencing_token"], 42); // rides opaquely (L2-O4)
        // and round-trips back to the SAME ActionRequest (no field dropped/reshaped by the crate).
        let echoed: ActionRequest = serde_json::from_value(sent.params).unwrap();
        assert_eq!(echoed, req);
    }

    #[test]
    fn preview_action_returns_typed_preview() {
        // spec(§6.1 / L2-O2) — preview_action forms {action_request_id} + deserializes ActionPreview.
        let preview = sample_preview();
        let mut s = FakeStream::new(rpc_ok(12, serde_json::to_value(&preview).unwrap()));
        let got = preview_action(&mut s, "act_xyz", 12).unwrap();
        assert_eq!(got, preview);
        let sent: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(sent.method, "preview_action");
        assert_eq!(sent.params["action_request_id"], "act_xyz");
    }

    #[test]
    fn approve_forms_params_and_returns_ack() {
        // spec(§6.1) — approve forms {approval_id, step_id?} + deserializes ActionAck. step_id is
        // included only when Some (the daemon accepts-but-reserves it, 2.1c).
        let mut s = FakeStream::new(rpc_ok(13, serde_json::to_value(sample_ack()).unwrap()));
        let ack = approve(&mut s, "appr_1", Some("step_2"), 13).unwrap();
        assert_eq!(ack.action_request_id, "act_test");
        assert!(matches!(ack.status, ActionRequestStatus::Submitted)); // both ack fields verified
        let sent: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(sent.method, "approve");
        assert_eq!(sent.params["approval_id"], "appr_1");
        assert_eq!(sent.params["step_id"], "step_2");
    }

    #[test]
    fn approve_omits_step_id_when_none() {
        // spec(§6.1) — step_id None → the field is ABSENT (not null); the single-action L2 core.
        let mut s = FakeStream::new(rpc_ok(14, serde_json::to_value(sample_ack()).unwrap()));
        approve(&mut s, "appr_1", None, 14).unwrap();
        let sent: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(sent.params["approval_id"], "appr_1");
        assert!(
            sent.params.get("step_id").is_none(),
            "step_id is absent (not null) when None"
        );
    }

    #[test]
    fn deny_forms_params_and_returns_ack() {
        // spec(§6.1) — deny forms {approval_id, reason} + deserializes ActionAck.
        let mut s = FakeStream::new(rpc_ok(15, serde_json::to_value(sample_ack()).unwrap()));
        let ack = deny(&mut s, "appr_9", "not now", 15).unwrap();
        assert!(matches!(ack.status, ActionRequestStatus::Submitted));
        assert_eq!(ack.action_request_id, "act_test"); // both ack fields verified
        let sent: RpcRequest = serde_json::from_slice(&s.writes[4..]).unwrap();
        assert_eq!(sent.method, "deny");
        assert_eq!(sent.params["approval_id"], "appr_9");
        assert_eq!(sent.params["reason"], "not now");
    }

    #[test]
    fn mutation_wire_error_returns_verbatim_code() {
        // spec(L2-D6 / §6.4) — a WireError on a mutation → Err(Wire(code)) VERBATIM, never collapsed
        // (so L2-C routes fencing_conflict→hard-conflict / precondition_stale→re-approvable). Pinned
        // on submit_action (fencing_conflict) + approve (precondition_stale): two methods, two codes.
        let wire = |code| {
            frame_json(&ServerFrame::RpcResponse(nexusops_shared::ipc::RpcResponse {
                id: 7,
                result: None,
                error: Some(nexusops_shared::ipc::WireError { code }),
            }))
        };
        let mut s1 = FakeStream::new(wire(IpcErrorCode::FencingConflict));
        assert!(matches!(
            submit_action(&mut s1, &sample_action_request(), 7),
            Err(ClientError::Wire(IpcErrorCode::FencingConflict))
        ));
        let mut s2 = FakeStream::new(wire(IpcErrorCode::PreconditionStale));
        assert!(matches!(
            approve(&mut s2, "appr_1", None, 7),
            Err(ClientError::Wire(IpcErrorCode::PreconditionStale))
        ));
    }

    #[test]
    fn mutation_malformed_result_is_serde_error() {
        // spec(§5.0) — a structurally valid RpcResponse whose result is NOT an ActionAck → Serde
        // (the typed-deserialize fail-closed path), never a bad partial value.
        let mut s = FakeStream::new(rpc_ok(3, serde_json::json!({ "not_an_ack": true })));
        assert!(matches!(
            submit_action(&mut s, &sample_action_request(), 3),
            Err(ClientError::Serde(_))
        ));
    }

    #[test]
    fn mutation_non_rpcresponse_or_id_mismatch_is_protocol() {
        // spec(§6.4) — a non-RpcResponse frame / an id-mismatch on a mutation call → Protocol
        // (inherited from demux_rpc_response; re-pinned on a mutation path).
        let term = ServerFrame::TerminalOutput(nexusops_shared::ipc::TerminalOutputFrame {
            terminal_id: "t1".into(),
            seq: 0,
            data: "aGk=".into(),
        });
        let mut s = FakeStream::new(frame_json(&term));
        assert!(matches!(
            deny(&mut s, "appr_1", "x", 7),
            Err(ClientError::Protocol(_))
        ));
        // an id-mismatch is also Protocol (a stale/uncorrelated response, even carrying a valid ack).
        let mismatch = frame_json(&ServerFrame::RpcResponse(nexusops_shared::ipc::RpcResponse {
            id: 99,
            result: Some(serde_json::to_value(sample_ack()).unwrap()),
            error: None,
        }));
        let mut s2 = FakeStream::new(mismatch);
        assert!(matches!(
            submit_action(&mut s2, &sample_action_request(), 7),
            Err(ClientError::Protocol(_))
        ));
    }
}

//! Per-connection GatewayPort handler (§6.1/§6.4, §15 rule #7).
//!
//! The async accept-loop (1.6-bootstrap-wired, §16) accepts a connection, reads the peer's uid
//! via [`peer_uid`](super::peer_uid)/`getpeereid`, and hands the connection + uid to
//! [`serve_connection`] (on a blocking task, since the read path is synchronous rusqlite). The
//! handler runs the **rule-#7 peer-auth gate FIRST**, then the **§6.4 handshake** (the first
//! frame MUST be a `HelloFrame`; an in-range `protocol_version` → `HelloAck`, an out-of-range
//! one → `VersionSkewError` + disconnect, a non-handshake frame → a structured error +
//! disconnect). The JSON-RPC dispatch + read methods (L3) and subscribe streaming (L4) extend
//! the authorized path after the ack.

use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;

use nexusops_shared::ipc::{
    self, Capabilities, HelloAck, HelloFrame, IpcErrorCode, ProjectionDelta, RpcRequest,
    ServerFrame, SubscribeParams, VersionSkewError, WireError,
};
use tokio::sync::broadcast;

use super::{authorize_peer, methods, read_frame, run_push_loop, write_frame, IpcError};

/// Serve one accepted GatewayPort connection over a synchronous stream. `peer_uid` is the uid
/// the accept-loop read via `getpeereid`; the rule-#7 auth gate is FIRST (a foreign uid is
/// rejected before any frame is read → disconnect, zero methods served). Then the §6.4
/// handshake: a valid in-range `HelloFrame` → `HelloAck`; otherwise a structured error frame is
/// written and the connection disconnects (the stream drops on the error return). L3/L4 extend
/// the authorized post-ack path with the JSON-RPC serve loop + subscribe streaming.
#[allow(clippy::too_many_arguments)]
pub fn serve_connection(
    mut stream: UnixStream,
    peer_uid: u32,
    daemon_uid: u32,
    db_path: &Path,
    deltas: broadcast::Sender<ProjectionDelta>,
    write: &crate::runtime::WriteHandle,
    // C2 — the per-session decision_sink registry (the `intercept` wait + the approve/deny resolve);
    // shared across connections (the supervisor reap cancels too). The `intercept` bridge gets its
    // runtime handle via `Handle::try_current()` (the serve thread is a `spawn_blocking` task).
    registry: &crate::decisions::DecisionRegistry,
) -> Result<(), IpcError> {
    // Rule #7 (§15 / ADR-004): peer-auth before anything else — before any frame is read.
    authorize_peer(peer_uid, daemon_uid)?;

    // §6.4 handshake-first: the first frame MUST be a `HelloFrame`.
    let body = read_frame(&mut stream)?;
    let hello: HelloFrame = match serde_json::from_slice(&body) {
        Ok(h) => h,
        Err(e) => {
            // a non-handshake first frame (a method, or garbage) is a protocol violation →
            // `protocol_error` + disconnect (the §6.4 `protocol_error` code added per the
            // lead-ratified gap resolution; distinct from an unknown method name).
            write_wire_error(&mut stream, IpcErrorCode::ProtocolError);
            return Err(IpcError::Protocol(format!("expected HelloFrame: {e}")));
        }
    };

    if !ipc::protocol_in_range(hello.protocol_version) {
        // version skew → advertise the supported range, then disconnect (no method served).
        let skew = VersionSkewError {
            supported_min: ipc::SUPPORTED_PROTOCOL_MIN,
            supported_max: ipc::SUPPORTED_PROTOCOL_MAX,
            client_protocol_version: hello.protocol_version,
        };
        // best-effort error-write (mirrors `write_wire_error`): a failed write must not mask
        // the version-skew Err below. (Open-coded because the payload is a VersionSkewError,
        // not a WireError, so the WireError helper doesn't apply.)
        if let Ok(buf) = serde_json::to_vec(&skew) {
            let _ = write_frame(&mut stream, &buf);
        }
        return Err(IpcError::VersionSkew {
            client_version: hello.protocol_version,
            supported_min: ipc::SUPPORTED_PROTOCOL_MIN,
            supported_max: ipc::SUPPORTED_PROTOCOL_MAX,
        });
    }

    // handshake OK → HelloAck with the daemon's capabilities (§6.4).
    let ack = HelloAck {
        protocol_version: ipc::PROTOCOL_VERSION,
        daemon_version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: Capabilities {
            protocol_version: ipc::PROTOCOL_VERSION,
            contract_version: nexusops_shared::CONTRACT_VERSION.to_string(),
        },
    };
    let buf = serde_json::to_vec(&ack).map_err(|e| IpcError::Protocol(e.to_string()))?;
    write_frame(&mut stream, &buf)?;

    // L3/L4 — the §6.1 JSON-RPC read/serve loop: read request frames until the client half-closes
    // (EOF), dispatch each (get_projection/get_capabilities over read-only WAL), write responses.
    // A client error is a structured `WireError` response (loop continues); an infra read error
    // disconnects. A `subscribe` (1.6d) additionally spawns a push stream on a `try_clone`'d write
    // half (the read/write split) so deltas push while this loop keeps blocking on the next frame.
    loop {
        let body = match read_frame(&mut stream) {
            Ok(b) => b,
            // the client half-closed after its last request → a clean end of the session.
            Err(IpcError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        };
        let req: RpcRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(e) => {
                // a malformed request frame is a protocol violation → protocol_error + disconnect.
                write_wire_error(&mut stream, IpcErrorCode::ProtocolError);
                return Err(IpcError::Protocol(format!("malformed request: {e}")));
            }
        };
        // §6.1 subscribe (1.6d) — a subscribe makes this a DEDICATED push connection: there is
        // exactly ONE writer at any time, so frames never interleave. The receiver is minted BEFORE
        // the ack (a delta published right after the ack isn't missed — broadcast delivers only to
        // receivers live at send time); the ack is written by THIS thread while no push thread yet
        // exists (no race); the push thread is spawned only if the ack SUCCEEDED (no drift vs
        // subscribe_ack's validation); then this thread goes READ-ONLY until EOF — it writes nothing
        // more, so it can never race the (now sole-writer) push thread. Exactly one push thread per
        // connection. MVP: 1 subscription/conn; multiplexing RPC + a subscription on one connection
        // (and the `subscription_id` that enables it) is deferred — a subscribe connection is dedicated.
        if req.method == "subscribe" {
            if let Ok(params) = serde_json::from_value::<SubscribeParams>(req.params.clone()) {
                let rx = deltas.subscribe();
                let ack = methods::dispatch(&req, db_path, write, registry)?;
                let accepted = ack.error.is_none();
                let buf = serde_json::to_vec(&ServerFrame::RpcResponse(ack))
                    .map_err(|e| IpcError::Protocol(e.to_string()))?;
                write_frame(&mut stream, &buf)?;
                if accepted {
                    // the push thread is the SOLE writer henceforth (detached — it self-terminates
                    // on Lagged/Closed/write-fail via `shutdown(Both)`, which also unblocks the read
                    // below). Reachable only post-auth + post-handshake (rule #7 stays first).
                    let write_half = stream.try_clone().map_err(IpcError::Io)?;
                    std::thread::spawn(move || run_push_loop(write_half, rx, params.projection));
                    // block on a SINGLE read: it returns when the client disconnects (EOF) OR sends
                    // any further frame (unsupported on a dedicated subscribe connection). This holds
                    // the serve task (+ its semaphore permit) for the connection's life and detects
                    // disconnect, while writing NOTHING (so it never races the push thread). Either
                    // outcome → close.
                    let _ = read_frame(&mut stream);
                    // close the socket BOTH directions so the client sees EOF AND the push thread's
                    // next write fails → it exits. Dropping `stream` alone wouldn't close the socket
                    // — the push thread's `try_clone`'d fd would keep it half-open.
                    let _ = stream.shutdown(Shutdown::Both);
                    return Ok(());
                }
                // a rejected subscribe (error ack) consumes nothing — keep serving the connection.
                continue;
            }
            // a malformed SubscribeParams falls through to dispatch → the protocol_error ack below.
        }
        // wrap the response in the frame-type-tagged ServerFrame envelope (§6.4 multiplexing) so
        // the client demuxes rpc-responses from subscription-push frames on one connection.
        let frame = ServerFrame::RpcResponse(methods::dispatch(&req, db_path, write, registry)?);
        let buf = serde_json::to_vec(&frame).map_err(|e| IpcError::Protocol(e.to_string()))?;
        write_frame(&mut stream, &buf)?;
    }
    Ok(())
}

/// best-effort: write a structured `WireError` frame before disconnecting on an error path (a
/// failed error-write must not mask the original failure).
fn write_wire_error<W: Write>(w: &mut W, code: IpcErrorCode) {
    if let Ok(buf) = serde_json::to_vec(&WireError { code }) {
        let _ = write_frame(w, &buf);
    }
}

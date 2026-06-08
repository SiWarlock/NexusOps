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

use std::io::{Read, Write};

use nexusops_shared::ipc::{
    self, Capabilities, HelloAck, HelloFrame, IpcErrorCode, VersionSkewError, WireError,
};

use super::{authorize_peer, read_frame, write_frame, IpcError};

/// Serve one accepted GatewayPort connection over a synchronous stream. `peer_uid` is the uid
/// the accept-loop read via `getpeereid`; the rule-#7 auth gate is FIRST (a foreign uid is
/// rejected before any frame is read → disconnect, zero methods served). Then the §6.4
/// handshake: a valid in-range `HelloFrame` → `HelloAck`; otherwise a structured error frame is
/// written and the connection disconnects (the stream drops on the error return). L3/L4 extend
/// the authorized post-ack path with the JSON-RPC serve loop + subscribe streaming.
pub fn serve_connection<S: Read + Write>(
    mut stream: S,
    peer_uid: u32,
    daemon_uid: u32,
) -> Result<(), IpcError> {
    // Rule #7 (§15 / ADR-004): peer-auth before anything else — before any frame is read.
    authorize_peer(peer_uid, daemon_uid)?;

    // §6.4 handshake-first: the first frame MUST be a `HelloFrame`.
    let body = read_frame(&mut stream)?;
    let hello: HelloFrame = match serde_json::from_slice(&body) {
        Ok(h) => h,
        Err(e) => {
            // a non-handshake first frame (a method, or garbage) → structured error + disconnect.
            // NOTE: the closed §6.4 error-code set has no dedicated "bad first frame / handshake
            // -required" code, so `unknown_method` is the closest available — flagged to the
            // orchestrator (a §6.4 `protocol_error` code would be a LOCKED-contract change).
            // Fail-closed regardless of the wire code (the connection disconnects, unserved).
            write_wire_error(&mut stream, IpcErrorCode::UnknownMethod);
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

    // L3+ extends the authorized path here: the JSON-RPC method read/serve loop, then L4 subscribe.
    Ok(())
}

/// best-effort: write a structured `WireError` frame before disconnecting on an error path (a
/// failed error-write must not mask the original failure).
fn write_wire_error<W: Write>(w: &mut W, code: IpcErrorCode) {
    if let Ok(buf) = serde_json::to_vec(&WireError { code }) {
        let _ = write_frame(w, &buf);
    }
}

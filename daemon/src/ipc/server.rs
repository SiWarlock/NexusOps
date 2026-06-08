//! Per-connection GatewayPort handler (§6.1/§6.4, §15 rule #7).
//!
//! The async accept-loop (1.6-bootstrap-wired, §16) accepts a connection, reads the peer's uid
//! via [`peer_uid`](super::peer_uid)/`getpeereid`, and hands the connection + uid to
//! [`serve_connection`] (on a blocking task, since the read path is synchronous rusqlite). The
//! handler runs the **rule-#7 peer-auth gate FIRST** — a foreign uid is rejected and the
//! connection dropped (disconnect) before any frame is read or any method served. The
//! post-auth handshake + JSON-RPC serve loop land in L2–L4.

use std::io::{Read, Write};

use super::{authorize_peer, IpcError};

/// Serve one accepted GatewayPort connection over a synchronous stream. `peer_uid` is the uid
/// the accept-loop read via `getpeereid`; the auth gate (safety rule #7) is the FIRST step, so
/// a foreign uid is rejected before any frame is read — the connection is dropped on the error
/// return, disconnecting the peer with zero methods served.
///
/// L1 stops at the auth gate; L2 extends the authorized path with the handshake
/// (`HelloFrame`→`HelloAck`|`VersionSkewError`) and L3/L4 the JSON-RPC read/serve + subscribe.
pub fn serve_connection<S: Read + Write>(
    // the LIVE connection — `_`-prefixed only because L1 stops at the auth gate; L2+ reads the
    // handshake + serves over it. Consumed by value so a reject drops it here → disconnect.
    _stream: S,
    peer_uid: u32,
    daemon_uid: u32,
) -> Result<(), IpcError> {
    // Rule #7 (§15 / ADR-004): peer-auth before anything else. On rejection the moved
    // `_stream` drops here → the socket closes → the peer is disconnected, unserved.
    authorize_peer(peer_uid, daemon_uid)?;
    // L2+ extends the authorized path here (handshake, then the JSON-RPC serve loop).
    Ok(())
}

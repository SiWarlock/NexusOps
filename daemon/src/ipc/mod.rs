//! The UDS `GatewayPort` transport (§6.1/§6.4 [LOCKED — ADR-004], §15, §12).
//!
//! A Unix-domain-socket server speaking **4-byte big-endian length-prefixed JSON-RPC**, gated
//! by a `getpeereid()` peer-auth check (reject uid ≠ daemon-uid — safety rule #7; NOT the
//! Linux-only `SO_PEERCRED`), a `HelloFrame`→`HelloAck`|`VersionSkewError` handshake, and the
//! **read-only** method surface (`get_projection`/`subscribe`/`get_capabilities`). It is the
//! real `UdsGatewayPort` the ui's `MockGatewayPort` swaps to — matching the contract the ui
//! already pinned (`ui/src/gateway-client/types.ts`, `SUPPORTED_PROTOCOL_RANGE {1,1}`).
//!
//! **L1 (this layer):** the length-prefix codec (bounded by `MAX_FRAME_SIZE` — rejected from
//! the prefix *before* allocating, anti-DoS) + the `getpeereid` peer-auth gate. The
//! per-connection handler [`serve_connection`] runs the auth gate FIRST (a foreign uid is
//! rejected + disconnected before any frame is read). The handshake (L2), JSON-RPC dispatch
//! (L3), and subscribe streaming (L4) extend the handler; the async accept-loop + `bind()`
//! are 1.6-bootstrap-wired (§16), like the 1.3 drainer / 1.4 reaper.
//!
//! Reads go over **read-only WAL** connections — the write-actor stays the sole writer
//! (Forbidden #3 / LESSON §3).

mod methods;
mod peer;
mod server;
mod subscribe;
mod transport;

pub use peer::{authorize_peer, current_euid, peer_uid};
pub use server::serve_connection;
pub use subscribe::push_subscription;
pub use transport::{decode_len, encode_frame, read_frame, write_frame, MAX_FRAME_SIZE};

/// Typed IPC failures — fail-closed (§15): a frame or peer that can't be validated is
/// rejected, never served. The §6.4 *wire* error-code enum (`version_skew`, `unknown_method`,
/// `precondition_stale`, …) is authored in `shared/` with the L2 IPC schema; this is the
/// daemon-internal transport error type the L1 trust-boundary units return.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// a frame whose declared length exceeds `MAX_FRAME_SIZE` (§6.4) — rejected pre-allocation.
    #[error("frame too large: {declared} bytes exceeds MAX_FRAME_SIZE {max}")]
    FrameTooLarge { declared: usize, max: usize },
    /// the connected peer's uid ≠ the daemon uid (safety rule #7) — connection refused.
    #[error("unauthorized peer: uid {peer_uid} != daemon uid {daemon_uid}")]
    UnauthorizedPeer { peer_uid: u32, daemon_uid: u32 },
    /// the client's handshake `protocol_version` is outside the daemon's supported range (§6.4).
    #[error(
        "version skew: client protocol {client_version} not in [{supported_min}, {supported_max}]"
    )]
    VersionSkew {
        client_version: u32,
        supported_min: u32,
        supported_max: u32,
    },
    /// a protocol violation — e.g. a non-handshake first frame, or a malformed handshake (§6.4).
    #[error("protocol violation: {0}")]
    Protocol(String),
    /// a projection read failed over the read-only WAL connection (§6.1) — an infrastructure
    /// error (vs a client error, which is a structured `WireError` response). Disconnects.
    #[error("projection read error: {0}")]
    Read(String),
    /// transport / syscall IO error. A `getpeereid` failure lands here → fail-closed (no uid
    /// is produced, so the peer can never be authorized).
    #[error("ipc io error: {0}")]
    Io(#[from] std::io::Error),
}

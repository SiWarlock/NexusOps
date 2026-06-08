//! Subscription push stream (§6.1 subscribe).
//!
//! The deterministic unit: write a frame-type-tagged `ServerFrame::SubscriptionPush` frame per
//! [`ProjectionDelta`] from a source. The **live delta source** — `EventStore::append` →
//! after `apply_all` → broadcast → subscriber — and the dispatch routing are **1.6-wired** (the
//! daemon runtime), like the accept-loop spawn; 1.5 ships the mechanism + the frame-type contract.

use std::io::Write;

use nexusops_shared::ipc::{ProjectionDelta, ServerFrame};

use super::{write_frame, IpcError};

/// Push each `ProjectionDelta` as a frame-type-tagged `ServerFrame::SubscriptionPush` frame to
/// `writer` — the connection's WRITE half. At the 1.6 dispatch this is a `UnixStream::try_clone`
/// of the connection so the push loop can write while the read half blocks on the next client
/// frame (the read/write split); the unit just needs a `Write`. Returns the number pushed.
pub fn push_subscription<W: Write>(
    writer: &mut W,
    deltas: impl IntoIterator<Item = ProjectionDelta>,
) -> Result<usize, IpcError> {
    let mut pushed = 0;
    for delta in deltas {
        let frame = ServerFrame::SubscriptionPush(delta);
        let buf = serde_json::to_vec(&frame).map_err(|e| IpcError::Protocol(e.to_string()))?;
        write_frame(writer, &buf)?;
        pushed += 1;
    }
    Ok(pushed)
}

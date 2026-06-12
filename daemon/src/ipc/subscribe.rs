//! Subscription push stream (§6.1 subscribe).
//!
//! The deterministic unit: write a frame-type-tagged `ServerFrame::SubscriptionPush` frame per
//! [`ProjectionDelta`] from a source. The **live delta source** — `EventStore::append` →
//! after `apply_all` → broadcast → subscriber — and the dispatch routing are **1.6-wired** (the
//! daemon runtime), like the accept-loop spawn; 1.5 ships the mechanism + the frame-type contract.

use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use nexusops_shared::ipc::{ProjectionDelta, ProjectionName, ServerFrame};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

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

/// What the push loop does with one `broadcast::Receiver::blocking_recv()` result, for a connection
/// subscribed to `subscribed`. A PURE decision (no I/O) so the back-pressure policy is unit-pinned.
pub(crate) enum PushAction {
    /// a delta for the subscribed projection — push it to the client.
    Push(ProjectionDelta),
    /// a delta for a DIFFERENT projection — not relevant to this subscription (NOT a gap); continue.
    Skip,
    /// stop the stream + CLOSE the connection. `Lagged`: dropped deltas are an UNDETECTABLE gap
    /// (`ProjectionDelta` carries no seq), so continuing would silently diverge the client — close
    /// so it reconnects + re-`get_projection` from a consistent snapshot (LESSON §9 resync trigger).
    /// `Closed`: the writer's broadcast sender dropped (daemon shutdown).
    Stop,
}

/// Classify one recv result. Forbidden-#3 corollary: a lagging subscriber must RESYNC (not get a
/// silently-gapped feed); the writer itself is already isolated by the non-blocking broadcast, so
/// this never back-pressures it — it just closes the connection on `Lagged` (LESSON §9).
pub(crate) fn next_push_action(
    recv: Result<ProjectionDelta, RecvError>,
    subscribed: ProjectionName,
) -> PushAction {
    match recv {
        Ok(d) if d.projection == subscribed => PushAction::Push(d),
        Ok(_) => PushAction::Skip,
        Err(RecvError::Lagged(_)) => PushAction::Stop,
        Err(RecvError::Closed) => PushAction::Stop,
    }
}

/// Drive a subscription's push stream over `write_half` (a `try_clone` of the connection's write
/// side) until Stop or a write-fail. On ANY exit — Stop (`Lagged` close-on-lag / `Closed` shutdown)
/// OR a push write-fail (the client is already gone) — `shutdown(Both)` the socket so the
/// connection's main read loop unblocks (EOF) and the client re-establishes. NEVER back-pressures
/// the writer (the broadcast drops a lagging receiver; this loop merely closes on the `Lagged`).
pub(crate) fn run_push_loop(
    mut write_half: UnixStream,
    mut rx: broadcast::Receiver<ProjectionDelta>,
    subscribed: ProjectionName,
) {
    loop {
        match next_push_action(rx.blocking_recv(), subscribed) {
            PushAction::Push(delta) => {
                if push_subscription(&mut write_half, [delta]).is_err() {
                    break; // I/O write-fail: the client is gone; the connection is already broken.
                }
            }
            PushAction::Skip => {}
            PushAction::Stop => break,
        }
    }
    // close on ANY exit → the main read loop unblocks (EOF) and the client reconnects + resyncs
    // (close-on-lag = LESSON §9's resync trigger). Best-effort: a failed shutdown is a dead socket.
    let _ = write_half.shutdown(Shutdown::Both);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::read_frame;
    use nexusops_shared::ipc::{DeltaKind, ProjectionName};
    use tokio::sync::broadcast;
    use tokio::sync::broadcast::error::RecvError;

    fn delta(p: ProjectionName) -> ProjectionDelta {
        ProjectionDelta {
            projection: p,
            kind: DeltaKind::Upsert,
            row: None,
            id: Some("sess_x".to_string()),
        }
    }

    #[test]
    fn next_push_action_classifies() {
        // a delta for the SUBSCRIBED projection → Push
        assert!(matches!(
            next_push_action(Ok(delta(ProjectionName::Session)), ProjectionName::Session),
            PushAction::Push(_)
        ));
        // a delta for a DIFFERENT projection → Skip (not relevant to this subscription; NOT a gap)
        assert!(matches!(
            next_push_action(
                Ok(delta(ProjectionName::AuditTrail)),
                ProjectionName::Session
            ),
            PushAction::Skip
        ));
        // Lagged → STOP: dropped deltas are an UNDETECTABLE gap (ProjectionDelta carries no seq), so
        // continuing would silently diverge the client → close instead (LESSON §9 resync trigger).
        assert!(matches!(
            next_push_action(Err(RecvError::Lagged(3)), ProjectionName::Session),
            PushAction::Stop
        ));
        // Closed (daemon shutdown / all senders dropped) → Stop
        assert!(matches!(
            next_push_action(Err(RecvError::Closed), ProjectionName::Session),
            PushAction::Stop
        ));
    }

    #[test]
    fn run_push_loop_closes_connection_on_lag() {
        use std::io::Read;
        use std::os::unix::net::UnixStream;

        // a receiver lagged BEFORE the loop runs: capacity 1, 3 sent → the first recv is Lagged.
        let (tx, rx) = broadcast::channel::<ProjectionDelta>(1);
        for _ in 0..3 {
            let _ = tx.send(delta(ProjectionName::Session));
        }
        let (push_write, mut client_read) = UnixStream::pair().unwrap();
        let h = std::thread::spawn(move || run_push_loop(push_write, rx, ProjectionName::Session));

        // the loop sees Lagged → shutdown(Both) → the client read end observes EOF (a CLOSED
        // connection → reconnect + resync), never a silently-gapped stream (forbidden #3 corollary).
        let mut buf = [0u8; 1];
        assert_eq!(
            client_read.read(&mut buf).unwrap(),
            0,
            "a lagged subscriber → the connection is CLOSED (client sees EOF), never gapped"
        );
        h.join().unwrap();
    }

    #[test]
    fn run_push_loop_pushes_matching_then_closes_on_broadcast_drop() {
        use std::os::unix::net::UnixStream;

        // a healthy matching delta is pushed; dropping the last sender closes the broadcast →
        // the loop sees Closed → Stop → shuts the connection (no lingering thread on shutdown).
        let (tx, rx) = broadcast::channel::<ProjectionDelta>(8);
        tx.send(delta(ProjectionName::Session)).unwrap();
        let (push_write, client_read) = UnixStream::pair().unwrap();
        let h = std::thread::spawn(move || run_push_loop(push_write, rx, ProjectionName::Session));
        drop(tx); // broadcast closes after the queued delta is consumed

        let frame: ServerFrame = {
            let mut r = &client_read;
            let body = read_frame(&mut r).expect("the matching delta is pushed");
            serde_json::from_slice(&body).unwrap()
        };
        assert!(
            matches!(frame, ServerFrame::SubscriptionPush(d) if d.projection == ProjectionName::Session),
            "the matching-projection delta is pushed as a SubscriptionPush frame"
        );
        h.join().unwrap();
    }
}

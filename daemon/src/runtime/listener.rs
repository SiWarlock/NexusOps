//! UDS bind + accept-loop (§6.1/§6.4/§16, safety rule #7).
//!
//! [`bind`] opens the GatewayPort Unix-domain socket, reclaiming a stale socket file first
//! (unlink — safe because the 1.4 pidlock already guarantees single-instance, so any leftover
//! socket is from a dead daemon). [`spawn_accept_loop`] accepts connections, reads each peer's
//! uid via `getpeereid`, bounds concurrent live connections with a semaphore (anti-DoS), and
//! hands each connection to the synchronous [`serve_connection`] on a blocking task (the read
//! path is blocking rusqlite). The rule-#7 auth gate runs FIRST inside `serve_connection`.
//!
//! Platform note: the `getpeereid`-based peer-auth stack is macOS/BSD (ADR-004). The daemon is
//! macOS-MVP; a Linux build-link cfg-guard over the getpeereid surface is a reviewer-sanctioned
//! deferred portability item (flagged at Step 9 — no Linux CI exists yet to verify against).

use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinHandle;

use crate::ipc::{peer_uid, serve_connection};

/// Bind the GatewayPort UDS at `socket_path`, reclaiming a stale socket first (unlink — the
/// pidlock guarantees single-instance, so a leftover socket is stale; without this, `bind` fails
/// `EADDRINUSE`). Sets `0600` perms (defense-in-depth, §15; the primary trust gate is getpeereid).
pub fn bind(socket_path: &Path) -> std::io::Result<UnixListener> {
    match std::fs::remove_file(socket_path) {
        Ok(()) => {}
        // nothing to reclaim on a clean first bind.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let listener = UnixListener::bind(socket_path)?;
    // 0600 owner-only (§15 defense-in-depth — NOT the primary gate, which is rule #7 getpeereid).
    std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Spawn the accept-loop. Each accepted connection: take a semaphore permit (at the cap →
/// REFUSE/drop, bounding concurrent 8-MiB frame buffers + threads — anti-DoS); convert the tokio
/// stream to a blocking std stream; read the peer uid via `getpeereid`; serve it synchronously on
/// a blocking task (`serve_connection` runs the rule-#7 gate first, then the §6.4 handshake +
/// §6.1 read methods). The permit is released when the connection's serve task finishes. The
/// `shutdown` watch breaking to `true` stops the loop.
pub fn spawn_accept_loop(
    listener: UnixListener,
    db_path: PathBuf,
    daemon_uid: u32,
    max_connections: usize,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    let permits = Arc::new(Semaphore::new(max_connections));
    tokio::spawn(async move {
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let stream = match accepted {
                        Ok((s, _addr)) => s,
                        Err(e) => {
                            eprintln!("nexusopsd: accept failed: {e}");
                            continue;
                        }
                    };
                    // anti-DoS: at the cap, REFUSE (drop the connection) rather than queue
                    // unboundedly — bounds concurrent connections + their frame buffers.
                    let permit = match Arc::clone(&permits).try_acquire_owned() {
                        Ok(p) => p,
                        Err(_) => continue, // at cap → the stream drops here (refused)
                    };
                    let db_path = db_path.clone();
                    tokio::task::spawn_blocking(move || {
                        // the permit is held for the connection's lifetime; it RELEASES when this
                        // closure ends (connection closed) — no leak / self-DoS.
                        let _permit = permit;
                        // tokio UnixStream → blocking std stream for the synchronous serve path.
                        let std_stream = match stream.into_std() {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!("nexusopsd: into_std failed: {e}");
                                return;
                            }
                        };
                        // serve_connection's loop is blocking; the std stream must block, not spin.
                        if let Err(e) = std_stream.set_nonblocking(false) {
                            eprintln!("nexusopsd: set_nonblocking(false) failed: {e}");
                            return;
                        }
                        // rule #7: read the peer's uid (getpeereid). A failure fails closed — no
                        // uid is produced, so serve_connection is never reached for this peer.
                        let uid = match peer_uid(std_stream.as_raw_fd()) {
                            Ok(u) => u,
                            Err(e) => {
                                eprintln!("nexusopsd: getpeereid failed: {e}");
                                return;
                            }
                        };
                        // a disconnect / version-skew / unauthorized-peer is a normal close, logged.
                        if let Err(e) = serve_connection(std_stream, uid, daemon_uid, &db_path) {
                            eprintln!("nexusopsd: connection closed: {e}");
                        }
                    });
                }
                _ = shutdown.changed() => break,
            }
        }
    })
}

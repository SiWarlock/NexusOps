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

use nexusops_shared::ipc::ProjectionDelta;
use tokio::net::UnixListener;
use tokio::sync::{broadcast, watch, Semaphore};
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
#[allow(clippy::too_many_arguments)]
pub fn spawn_accept_loop(
    listener: UnixListener,
    db_path: PathBuf,
    daemon_uid: u32,
    max_connections: usize,
    deltas: broadcast::Sender<ProjectionDelta>,
    // the write-actor handle each served connection uses for §6.1 mutation methods (2.1b); cloned
    // per connection into its blocking serve task (the handle is cheap to clone — an mpsc sender).
    write: super::WriteHandle,
    // C2 — the per-session decision_sink registry (shared across the `intercept` + approve/deny
    // connections + the supervisor reap); cloned (Arc) per connection into its serve task.
    registry: Arc<crate::decisions::DecisionRegistry>,
    // D7 — the GitHub read client for `get_pr_diff` (the first IPC-layer network read); cloned (Arc) per
    // connection into its serve task. The production client's per-repo keychain auth is the deferred
    // follow-on (constructed unauthenticated in main.rs).
    github: Arc<dyn crate::integrations::github::GithubReadClient>,
    // P4.7/083 (C3b) — the "Connect via gh" connector for the `connect_via_gh` IPC trigger (the daemon
    // sources the `gh` token → keychain; NO token over IPC); cloned (Arc) per connection. Peer-authed by
    // the rule-#7 gate that runs first in `serve_connection`.
    gh_connector: Arc<dyn crate::integrations::auth::GhConnector>,
    mut shutdown: watch::Receiver<bool>,
) -> JoinHandle<()> {
    let permits = Arc::new(Semaphore::new(max_connections));
    // P4.0b-2-F2 — the intercept-wait permit class (§6.4/§10): a reserved sub-bound
    // (`wait_cap = max − max/4`) DERIVED from the SAME `max_connections` as the general pool (so the
    // two are consistent, no duplicated constant). A waiting `intercept` parks here for the approval-
    // wait; because `wait_cap < max_connections`, the reserved general headroom (`max/4`) is NEVER held
    // by waits → the UI `approve`/`deny` + reads can't be starved. Shared (cloned per connection).
    let wait_class = super::InterceptWaitClass::new(max_connections, max_connections / 4);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                // shutdown takes priority over a pending accept (prompt, deterministic stop).
                biased;
                _ = shutdown.changed() => break,
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
                    // a per-connection clone of the broadcast sender — each served connection mints
                    // its own subscriber receiver from it per `subscribe` request (1.6d).
                    let deltas = deltas.clone();
                    // a per-connection clone of the write-actor handle (the §6.1 mutation path).
                    let write = write.clone();
                    // a per-connection clone of the C2 decision_sink registry (Arc, cheap).
                    let registry = Arc::clone(&registry);
                    // a per-connection clone of the F2 intercept-wait permit class (cheap — Arc inside).
                    let wait_class = wait_class.clone();
                    // a per-connection clone of the D7 GitHub read client (Arc, cheap).
                    let github = Arc::clone(&github);
                    // a per-connection clone of the P4.7/083 "Connect via gh" connector (Arc, cheap).
                    let gh_connector = Arc::clone(&gh_connector);
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
                        if let Err(e) = serve_connection(
                            std_stream,
                            uid,
                            daemon_uid,
                            &db_path,
                            deltas,
                            &write,
                            &registry,
                            &wait_class,
                            github.as_ref(),
                            gh_connector.as_ref(),
                        ) {
                            eprintln!("nexusopsd: connection closed: {e}");
                        }
                    });
                }
            }
        }
    })
}

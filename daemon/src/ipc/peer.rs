//! UDS peer authentication (§15 / **Key safety rule #7** / ADR-004).
//!
//! The primary trust gate on a GatewayPort connection is the **peer's uid**, read via
//! `getpeereid(2)` (macOS/BSD — ADR-004 chose this over the Linux-only `SO_PEERCRED`). Any
//! peer whose uid ≠ the daemon-uid is rejected. Socket-directory `0700` + socket `0600` perms
//! are defense-in-depth, NOT the primary gate. The FFI is isolated to [`peer_uid`] and is
//! **fail-closed**: a `getpeereid` failure yields no uid, so the peer can never authorize.

use std::os::unix::io::RawFd;

use super::IpcError;

/// Authorize a connected peer (safety rule #7): the peer's uid MUST equal the daemon's uid.
/// A mismatch is `unauthorized_peer` (the connection is refused). This is the pure decision —
/// the accept-loop reads the uid via [`peer_uid`] and passes both in.
pub fn authorize_peer(peer_uid: u32, daemon_uid: u32) -> Result<(), IpcError> {
    if peer_uid == daemon_uid {
        Ok(())
    } else {
        Err(IpcError::UnauthorizedPeer {
            peer_uid,
            daemon_uid,
        })
    }
}

/// Read the connected peer's effective uid from a UDS file descriptor via `getpeereid(2)`.
/// **Fail-closed:** on a syscall error this returns `Err` (an `IpcError::Io`) — the caller
/// gets no uid and therefore can never authorize the peer (safety rule #7).
pub fn peer_uid(fd: RawFd) -> Result<u32, IpcError> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: `fd` is a valid, open AF_UNIX stream fd (an accepted GatewayPort connection).
    // `getpeereid` writes the peer's effective uid/gid into the two out-params, which point at
    // local stack ints valid for the call; nothing is shared past it. We check the return code
    // and fail closed on any non-zero result (no uid is trusted unless the syscall succeeded).
    let rc = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
    if rc != 0 {
        return Err(IpcError::Io(std::io::Error::last_os_error()));
    }
    // `libc::uid_t` is u32 on every getpeereid platform (macOS/BSD), so this returns the uid
    // with NO cast — a hypothetical wider-uid_t target would be a compile error here (fail-
    // closed at build time), never a silent truncation of this trust value.
    Ok(uid)
}

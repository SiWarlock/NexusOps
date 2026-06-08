//! Single-instance enforcement via a std advisory file lock (ADR-008 / §16).
//!
//! The OS holds an exclusive advisory lock against the open file descriptor, so it is
//! **auto-released when the process dies** — even on a crash. That makes single-instance
//! detection **immune to PID reuse**: there is no `kill(pid, 0)` liveness probe (which
//! false-positives when the OS recycles a dead daemon's PID onto an unrelated process).
//! The PID is written into the lock file for **diagnostics only** — it is never read back
//! as the liveness oracle.
//!
//! `File::try_lock` is stable since Rust 1.89 (toolchain is 1.93) → zero new dependencies.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::Path;

/// Typed pidlock failures — fail-closed: an acquire that cannot *prove* it took the lock
/// returns an error, never a `PidLock`. In particular an IO error is NEVER treated as
/// "acquired" (that would let a second instance run and break the single-writer invariant).
#[derive(Debug, thiserror::Error)]
pub enum PidLockError {
    /// another live daemon instance holds the lock (the OS lock is taken).
    #[error("another daemon instance holds the single-instance lock")]
    AlreadyHeld,
    /// the lock file could not be opened or locked — fail-closed (NOT acquired).
    #[error("pidlock io error: {0}")]
    Io(std::io::Error),
}

/// A held single-instance lock. The OS releases the advisory lock when this value (and its
/// inner [`File`]) is dropped — including on process crash. Keep it alive for the daemon's
/// lifetime; the production cold-start *call site* is the 1.6 bootstrap (§16 ordering).
#[derive(Debug)]
pub struct PidLock {
    // holding the open File keeps the OS advisory lock; Drop closes the fd → auto-release.
    _file: File,
}

impl PidLock {
    /// Acquire the single-instance lock at `path`, creating the file if needed. Returns
    /// [`PidLockError::AlreadyHeld`] if another instance holds it, or [`PidLockError::Io`]
    /// on any IO failure (fail-closed — an error is never a successful acquire).
    pub fn acquire(path: &Path) -> Result<Self, PidLockError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(PidLockError::Io)?;

        // non-blocking exclusive lock. WouldBlock = another instance holds it; a real IO
        // error fails closed (never reported as acquired).
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(PidLockError::AlreadyHeld),
            Err(TryLockError::Error(e)) => return Err(PidLockError::Io(e)),
        }

        // diagnostic only: record our PID (truncate any stale foreign PID first). This is
        // NOT the liveness oracle — the OS-held lock above is. Non-fatal: we already hold
        // the authoritative lock. Only write the PID if the truncate succeeded, so a failed
        // truncate can't leave a partially-overwritten (misleading) PID behind.
        let mut handle = &file;
        if handle.set_len(0).is_ok() {
            let _ = write!(handle, "{}", std::process::id());
        }

        Ok(Self { _file: file })
    }
}

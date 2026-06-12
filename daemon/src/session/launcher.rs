//! The `SessionLauncher` seam (P4.0a L2) — produces a launchable [`LaunchedSession`] bundle (the
//! adapter + the terminal + the minted session id) the [`SessionActor`](super::actor) drives. The
//! **4.1 B2-strict survival broker is a drop-in `SessionLauncher` impl behind this same seam**
//! (the non-surviving daemon-owned-PTY default lives here); see the `TODO(4.1)` in [`PtyLauncher`].
//!
//! **Cat-1 boundary (4.0a)** — the daemon-owned-PTY launcher runs a **benign** program; it NEVER
//! spawns a real `claude`/`codex` (a live un-intercepted agent is the INV-SEC-1 gap the cat-1 4.0b
//! closes by spawning the real agent WITH the interception). The terminal's exit-event sink is the
//! no-op [`NullTerminalSink`] — no emission, no mutation (the write-actor binding is 4.0b/4.2).

use std::io;
use std::path::PathBuf;

use nexusops_shared::events::TerminalProcessExited;
use nexusops_shared::harness::HarnessCapabilities;
use nexusops_shared::ids::SessionId;

use crate::harness::{FakeHarness, HarnessAdapter};
use crate::terminal::{
    ExitStatus, FakePty, PtyRead, PtySpawner, TerminalEventSink, TerminalId, TerminalSession,
};

/// The daemon-owned PTY window size for a launched session (the UI resizes via the §6.4 control path).
const ROWS: u16 = 24;
const COLS: u16 = 80;

/// A launched session bundle: the normalized [`HarnessAdapter`], the daemon-owned [`TerminalSession`]
/// read-pump, and the minted [`SessionId`]. The [`SessionActor`](super::actor) drives this. The 4.1
/// survival broker returns the same bundle from its own [`SessionLauncher`] impl (the swap contract).
pub struct LaunchedSession {
    pub session_id: SessionId,
    pub adapter: Box<dyn HarnessAdapter>,
    pub terminal: TerminalSession,
}

/// The seam that produces a [`LaunchedSession`]. `FakeLauncher` (tests) + `PtyLauncher` (the
/// daemon-owned-PTY default) implement it now; the 4.1 survival broker is a drop-in impl.
pub trait SessionLauncher: Send {
    fn launch_session(&self) -> io::Result<LaunchedSession>;
}

/// A no-op [`TerminalEventSink`] — the launched session's pump drops its `TerminalProcessExited`
/// (no emission, cat-1). The production write-actor binding (so the exit persists + the §17 cascade
/// fires) is 4.0b/4.2.
pub struct NullTerminalSink;

impl TerminalEventSink for NullTerminalSink {
    fn emit_process_exited(&self, _event: TerminalProcessExited) {}
}

/// The daemon runtime terminal handle derived from the session id (`term_<sess_…>` — matching the
/// `TerminalId::mint` `term_<ULID>` underscore convention; one terminal per session in 4.0a).
fn terminal_id_for(session_id: &SessionId) -> TerminalId {
    TerminalId::from_raw(format!("term_{}", session_id.as_str()))
}

/// A [`SessionLauncher`] that yields a [`FakeHarness`] + [`FakePty`] session (deterministic tests).
pub struct FakeLauncher {
    caps: HarnessCapabilities,
}

impl FakeLauncher {
    pub fn new(caps: HarnessCapabilities) -> Self {
        Self { caps }
    }
}

impl SessionLauncher for FakeLauncher {
    fn launch_session(&self) -> io::Result<LaunchedSession> {
        let session_id = SessionId::new();
        let adapter = Box::new(FakeHarness::new(self.caps.clone()));
        let pty = FakePty::new(
            vec![PtyRead::Chunk(b"fake session output\n".to_vec())],
            ExitStatus {
                exit_code: Some(0),
                signal: None,
            },
        );
        let terminal = TerminalSession::new(
            terminal_id_for(&session_id),
            Box::new(pty),
            Box::new(NullTerminalSink),
        );
        Ok(LaunchedSession {
            session_id,
            adapter,
            terminal,
        })
    }
}

/// A [`SessionLauncher`] that spawns a **daemon-owned PTY** via the §14 [`PtySpawner`] seam (the 3.4
/// `PortablePtySpawner` in production). **4.0a runs a benign program** (the live agent + interception
/// are 4.0b); the adapter is a [`FakeHarness`] placeholder until the real `ClaudeAdapter` lands at
/// 4.0b. **TODO(4.1):** the B2-strict survival broker swaps in here as a drop-in `SessionLauncher`.
pub struct PtyLauncher {
    spawner: Box<dyn PtySpawner>,
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    caps: HarnessCapabilities,
}

impl PtyLauncher {
    pub fn new(
        spawner: Box<dyn PtySpawner>,
        program: impl Into<String>,
        args: Vec<String>,
        cwd: PathBuf,
        caps: HarnessCapabilities,
    ) -> Self {
        Self {
            spawner,
            program: program.into(),
            args,
            cwd,
            caps,
        }
    }
}

impl SessionLauncher for PtyLauncher {
    fn launch_session(&self) -> io::Result<LaunchedSession> {
        let session_id = SessionId::new();
        // the daemon owns the PTY. 4.0a runs the configured BENIGN program — NEVER a real
        // claude/codex (a live un-intercepted agent is the INV-SEC-1 gap the cat-1 4.0b closes).
        let pty = self
            .spawner
            .spawn(&self.program, &self.args, &self.cwd, ROWS, COLS)?;
        let terminal = TerminalSession::new(
            terminal_id_for(&session_id),
            pty,
            Box::new(NullTerminalSink),
        );
        // placeholder adapter — the LIVE ClaudeAdapter (the real agent) lands at the cat-1 4.0b,
        // WITH the interception. The daemon-owned-PTY transport + the seam are what 4.0a proves.
        let adapter = Box::new(FakeHarness::new(self.caps.clone()));
        Ok(LaunchedSession {
            session_id,
            adapter,
            terminal,
        })
    }
}

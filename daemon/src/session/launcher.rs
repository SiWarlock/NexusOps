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
use nexusops_shared::ids::SessionId;

use crate::harness::claude::telemetry::TelemetrySinkFactory;
use crate::harness::claude::{ClaudeAdapter, ClaudeLaunchSpec};
use crate::harness::HarnessAdapter;
use crate::terminal::{PtySpawner, TerminalEventSink, TerminalId, TerminalSession};

// test-only imports — the `FakeLauncher` fake is `test-support`-gated (P4.0b-2 L3).
#[cfg(any(test, feature = "test-support"))]
use crate::harness::FakeHarness;
#[cfg(any(test, feature = "test-support"))]
use crate::terminal::{ExitStatus, FakePty, PtyRead};
#[cfg(any(test, feature = "test-support"))]
use nexusops_shared::harness::HarnessCapabilities;

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
/// **`test-support`-gated (P4.0b-2 L3)** — test-only (production launches via `PtyLauncher`).
#[cfg(any(test, feature = "test-support"))]
pub struct FakeLauncher {
    caps: HarnessCapabilities,
}

#[cfg(any(test, feature = "test-support"))]
impl FakeLauncher {
    pub fn new(caps: HarnessCapabilities) -> Self {
        Self { caps }
    }
}

#[cfg(any(test, feature = "test-support"))]
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

/// A [`SessionLauncher`] that spawns the **single live-`claude` PTY** via the §14 [`PtySpawner`] seam
/// (the 3.4 `PortablePtySpawner` in production). **P4.0b-2 Option A (lead-ruled): the launcher OWNS
/// the spawn site + the O-13 #10 enforcement surface** — it builds the [`ClaudeLaunchSpec`] (default
/// mode · no `-p` · no bg · 0600 generated settings), writes the settings **fail-closed** (a write
/// error → no session, never an un-hooked live agent), spawns the ONE claude PTY into the
/// [`TerminalSession`] (display), and constructs a [`ClaudeAdapter`] that does NOT spawn (status from
/// hook signals only, safety #9). The live interception is wired atomically at the same 4.0b-2 commit.
/// **TODO(4.1):** the B2-strict survival broker swaps in here as a drop-in `SessionLauncher`.
pub struct PtyLauncher {
    spawner: Box<dyn PtySpawner>,
    cwd: PathBuf,
    /// the hook-receiver command the generated `ClaudeSettings` wires the `PreToolUse` hook to (the
    /// `nexusopsd` hook-subcommand that bridges Claude's hook protocol ↔ the daemon's UDS `intercept`).
    hook_receiver: String,
    /// (4.0c) the OPAQUE per-session telemetry-sink factory — `main.rs` builds the production
    /// `WriteActorTelemetrySinkFactory` (closing over the `WriteHandle`) and hands it here; this
    /// launcher mints a sink per session + injects it into the `ClaudeAdapter`. Held as `Box<dyn
    /// TelemetrySinkFactory>` (a `harness/` trait, NOT `WriteHandle`) so `session/` stays cat-1
    /// import-grep-clean. `None` = no telemetry emission (tests / a sink-less launch).
    telemetry_sink_factory: Option<Box<dyn TelemetrySinkFactory>>,
}

impl PtyLauncher {
    pub fn new(
        spawner: Box<dyn PtySpawner>,
        cwd: PathBuf,
        hook_receiver: impl Into<String>,
    ) -> Self {
        Self {
            spawner,
            cwd,
            hook_receiver: hook_receiver.into(),
            telemetry_sink_factory: None,
        }
    }

    /// Inject the production telemetry-sink factory (4.0c; the `ClaudeAdapter::with_telemetry_sink`
    /// builder precedent). `main.rs` calls this with the `WriteActorTelemetrySinkFactory` so each
    /// launched session emits its `TelemetrySampled` observations via the write-actor. Opaque — the
    /// launcher never sees the `WriteHandle` inside (cat-1).
    pub fn with_telemetry_sink_factory(mut self, factory: Box<dyn TelemetrySinkFactory>) -> Self {
        self.telemetry_sink_factory = Some(factory);
        self
    }
}

impl SessionLauncher for PtyLauncher {
    fn launch_session(&self) -> io::Result<LaunchedSession> {
        let session_id = SessionId::new();
        // Option A — the launcher is the SINGLE live-claude spawn site + the O-13 #10 enforcement
        // surface. Build the spec (default-mode/no-`-p`/no-bg by construction) + write the 0600
        // per-session settings FAIL-CLOSED: a settings-write error returns `Err` → NO session spawned
        // (never a live agent without its `PreToolUse` hook — INV-SEC-1).
        let spec = ClaudeLaunchSpec::build(&self.cwd, session_id.as_str(), &self.hook_receiver);
        spec.write_settings()?;
        // env hygiene + correlation (note-1): strip ANTHROPIC_API_KEY (subscription/OAuth auth, §15 #8)
        // + carry NEXUSOPS_SESSION_ID (the hook→session correlation key) into the spawned child.
        let pty = self.spawner.spawn(
            spec.program(),
            spec.args(),
            &self.cwd,
            ROWS,
            COLS,
            &spec.env_mutations(),
        )?;
        let terminal = TerminalSession::new(
            terminal_id_for(&session_id),
            pty,
            Box::new(NullTerminalSink),
        );
        // the LIVE ClaudeAdapter — it does NOT spawn (the launcher just did); status derives from the
        // live hook signals via `push_signal` (safety #9), never the PTY. `session_id` is a best-effort
        // transcript-path default (the live hook input carries the authoritative `transcript_path`).
        let mut adapter = ClaudeAdapter::new(self.cwd.clone(), session_id.as_str().to_string());
        // (4.0c) inject the per-session telemetry sink from the opaque factory BEFORE the session takes
        // the adapter (the production `WriteActorTelemetrySink` → write-actor append). `project_id` is
        // None for the MVP cwd-based launch (threading session.create's project_id is a P5 follow-on).
        // The pump's live `UsageSource` is the P4 ingress seam — the sink is bound now, ready for it.
        if let Some(factory) = &self.telemetry_sink_factory {
            adapter = adapter.with_telemetry_sink(factory.make_sink(&session_id, None));
        }
        Ok(LaunchedSession {
            session_id,
            adapter: Box::new(adapter),
            terminal,
        })
    }
}

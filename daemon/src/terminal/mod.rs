//! The §6.4 Terminal Channel data plane (ARCHITECTURE §9 / ADR-009; 3.4).
//!
//! The daemon owns each agent PTY: it spawns the child, pumps its output into base64
//! [`TerminalOutputFrame`](nexusops_shared::ipc::TerminalOutputFrame)s, forwards client input, and
//! records the OS-derived exit as a [`TerminalProcessExited`] observation event.
//!
//! **Safety #9 — the PTY is DISPLAY-ONLY.** This module is a byte pipe + an OS exit signal. It NEVER
//! scrapes terminal output to infer a session/agent status — that comes from the SDK/app-server
//! streams (the `harness` layer), not from here. The module exposes NO status-derivation API
//! (asserted structurally by `daemon/tests/terminal.rs::test_terminal_module_has_no_status_derivation_api`,
//! forbidden #4).
//!
//! **Layering.** `terminal` is an EDGE (it does not write the DB). The `TerminalProcessExited` event
//! is emitted through the injected [`TerminalEventSink`] seam (`FakeEventSink` in tests; the
//! production binding to the write-actor's `WriteHandle::append` — the §15-gated, non-mutation
//! observation write, LESSON §10/§23 — lands with the per-session drive loop at 3.2/3.3). The
//! `PortablePtyHost` (a live child) is the non-deterministic surface; the deterministic pump/exit
//! logic is exercised via the [`FakePty`] §14 seam.
//!
//! **L2** (this commit): the `Pty` trait + `PortablePtyHost` + `FakePty` + `TerminalSession` (the
//! basic 1-frame-per-read pump + the exit emission). **L3** layers the watermark backpressure +
//! ~30 fps batching on top (a pure `next_terminal_action` classifier, LESSON §12).

use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::Mutex;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use nexusops_shared::events::TerminalProcessExited;
use nexusops_shared::ipc::TerminalOutputFrame;

use crate::idgen::IdGen;

// ---- terminal_id — a daemon runtime handle (NOT a frozen-22 ID; Q1 lead-ratified) ---------------

/// An opaque daemon-minted terminal runtime handle (`term_<ULID>`). NOT one of the 22 frozen
/// `shared/` IDs — a connection/session-scoped handle, re-minted on attach/resume (the
/// `subscription_id` precedent). On the wire it is a plain `String`; daemon-internally it is this
/// typed newtype.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TerminalId(String);

impl TerminalId {
    /// Mint a fresh handle via the §14 [`IdGen`] seam (`term_<ULID>`).
    pub fn mint(idgen: &dyn IdGen) -> Self {
        Self(idgen.new_terminal_id())
    }

    /// Wrap a known handle string (from the wire / a test).
    pub fn from_raw(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ---- the PTY abstraction (object-safe seam; §14) ------------------------------------------------

/// One read from a PTY: a chunk of raw output bytes, or end-of-file (the child closed the PTY).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PtyRead {
    Chunk(Vec<u8>),
    Eof,
}

/// A child's exit detail, **from the OS** (waitpid) — `exit_code` XOR `signal`. NEVER inferred from
/// output bytes (safety #9). Mirrors the `TerminalProcessExited` payload's exit fields.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExitStatus {
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
}

/// The PTY I/O surface (object-safe → `Box<dyn Pty>`; the `ActionExecutor`/`HarnessAdapter`
/// precedent). **Synchronous** — `portable-pty`'s reader is blocking; the production drive loop runs
/// the reader on a `spawn_blocking` thread (the write-actor-thread precedent, LESSON §9). This is a
/// byte pipe + an OS exit signal — there is NO method that derives a status from output (safety #9).
pub trait Pty: Send {
    /// Read the next output chunk (blocking) or EOF. The bytes are forwarded verbatim, never parsed.
    fn read(&mut self) -> io::Result<PtyRead>;
    /// Write input bytes to the child's stdin (keystrokes / paste).
    fn write(&mut self, bytes: &[u8]) -> io::Result<()>;
    /// Resize the PTY window.
    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()>;
    /// The child's final exit status, from the OS. Call AFTER [`read`](Pty::read) returns
    /// [`PtyRead::Eof`] (or a read error) — it reaps the child. NEVER inferred from output (#9).
    fn exit_status(&mut self) -> ExitStatus;
    /// Kill the child.
    fn kill(&mut self) -> io::Result<()>;
}

// ---- the event-emission seam (daemon-internal; production binds the write-actor at 3.2/3.3) ------

/// The seam through which a [`TerminalSession`] emits its [`TerminalProcessExited`] observation
/// event. The production impl binds the write-actor's `WriteHandle::append` (the §15-gated,
/// non-mutation observation write — INV-SEC-1 governs *mutations*, a process-exit notice is none;
/// LESSON §10/§23) at the per-session drive loop (3.2/3.3); tests use a collecting double.
pub trait TerminalEventSink: Send {
    fn emit_process_exited(&self, event: TerminalProcessExited);
}

// ---- TerminalSession — the per-terminal pump (L2 basic form; L3 adds backpressure) --------------

/// One attached terminal: pumps PTY output into seq-numbered output frames and emits the
/// `TerminalProcessExited` observation event once, on child exit.
pub struct TerminalSession {
    terminal_id: TerminalId,
    pty: Box<dyn Pty>,
    sink: Box<dyn TerminalEventSink>,
    next_seq: u64,
    exited: bool,
}

impl TerminalSession {
    pub fn new(
        terminal_id: TerminalId,
        pty: Box<dyn Pty>,
        sink: Box<dyn TerminalEventSink>,
    ) -> Self {
        Self {
            terminal_id,
            pty,
            sink,
            next_seq: 0,
            exited: false,
        }
    }

    pub fn terminal_id(&self) -> &TerminalId {
        &self.terminal_id
    }

    pub fn is_exited(&self) -> bool {
        self.exited
    }

    /// Forward client input bytes to the PTY child (the client→daemon write path).
    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.pty.write(bytes)
    }

    /// Resize the PTY window.
    pub fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()> {
        self.pty.resize(rows, cols)
    }

    /// Kill the PTY child (session abort). The next [`pump`](TerminalSession::pump) observes EOF (the
    /// child is gone) and emits the `TerminalProcessExited` event exactly once.
    pub fn kill(&mut self) -> io::Result<()> {
        self.pty.kill()
    }

    /// One pump step: read one available output chunk → 0-or-1 output frame; on EOF / a read error,
    /// finish (emit the exit event exactly once). After exit, a pump is a no-op (returns `[]`). The
    /// production drive loop calls this in a `spawn_blocking` loop until `is_exited()`.
    pub fn pump(&mut self) -> Vec<TerminalOutputFrame> {
        if self.exited {
            return Vec::new();
        }
        match self.pty.read() {
            Ok(PtyRead::Chunk(bytes)) if !bytes.is_empty() => vec![self.frame(&bytes)],
            Ok(PtyRead::Chunk(_)) => Vec::new(), // an empty chunk = nothing available this step
            Ok(PtyRead::Eof) => {
                self.finish();
                Vec::new()
            }
            // a read error means the child/PTY is gone → terminal (the reader hung up).
            Err(_) => {
                self.finish();
                Vec::new()
            }
        }
    }

    /// Base64-encode a raw output chunk into a seq-numbered [`TerminalOutputFrame`].
    fn frame(&mut self, bytes: &[u8]) -> TerminalOutputFrame {
        let seq = self.next_seq;
        self.next_seq += 1;
        TerminalOutputFrame {
            terminal_id: self.terminal_id.as_str().to_string(),
            seq,
            data: STANDARD.encode(bytes),
        }
    }

    /// Emit the `TerminalProcessExited` observation event exactly once (idempotent). `exit_code` /
    /// `signal` come from the OS exit status — NEVER from parsing output (safety #9). NOTE: for
    /// `PortablePtyHost` this calls a BLOCKING `child.wait()` to reap the child; the production drive
    /// loop runs `pump` (hence `finish`) on a `spawn_blocking` thread (LESSON §9), so the block is
    /// expected + correct — including on the read-error → `finish` path (the child is terminating).
    fn finish(&mut self) {
        if self.exited {
            return;
        }
        self.exited = true;
        let status = self.pty.exit_status();
        self.sink.emit_process_exited(TerminalProcessExited {
            terminal_id: self.terminal_id.as_str().to_string(),
            exit_code: status.exit_code,
            signal: status.signal,
        });
    }
}

// ---- PortablePtyHost — the production PTY (a live child; the non-deterministic surface) ----------

/// A live PTY child via `portable-pty`. The daemon owns the master end (read/write/resize) and the
/// child handle (waitpid/kill). This is the non-deterministic surface — the deterministic pump/exit
/// logic is exercised via [`FakePty`]; this host's real-child path runs in the per-session drive
/// loop (3.2/3.3) + the 3.5 terminal-attach benchmark.
pub struct PortablePtyHost {
    /// the master end — retained for `resize` (the `reader`/`writer` are cloned/taken off it).
    master: Box<dyn portable_pty::MasterPty + Send>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PortablePtyHost {
    /// Spawn `program` (with `args`) in a fresh PTY at `cwd`, sized `rows`×`cols`.
    pub fn spawn(
        program: &str,
        args: &[String],
        cwd: &std::path::Path,
        rows: u16,
        cols: u16,
    ) -> io::Result<Self> {
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(to_io)?;

        let mut builder = portable_pty::CommandBuilder::new(program);
        for a in args {
            builder.arg(a);
        }
        builder.cwd(cwd);

        let child = pair.slave.spawn_command(builder).map_err(to_io)?;
        let reader = pair.master.try_clone_reader().map_err(to_io)?;
        let writer = pair.master.take_writer().map_err(to_io)?;
        // drop the slave so the master reader sees EOF once the child closes its PTY end.
        drop(pair.slave);

        Ok(Self {
            master: pair.master,
            reader,
            writer,
            child,
        })
    }
}

impl Pty for PortablePtyHost {
    fn read(&mut self) -> io::Result<PtyRead> {
        let mut buf = [0u8; 8192];
        let n = self.reader.read(&mut buf)?;
        if n == 0 {
            Ok(PtyRead::Eof)
        } else {
            Ok(PtyRead::Chunk(buf[..n].to_vec()))
        }
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()> {
        self.master
            .resize(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(to_io)
    }

    fn exit_status(&mut self) -> ExitStatus {
        match self.child.wait() {
            Ok(status) => map_exit(&status),
            // an unreapable child → unknown outcome (both None); rare, honest (never faked).
            Err(_) => ExitStatus {
                exit_code: None,
                signal: None,
            },
        }
    }

    fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }
}

/// Map a `portable-pty` exit status to our OS-derived [`ExitStatus`] — `signal` wins (exit_code XOR
/// signal), matching the `TerminalProcessExited` semantics.
fn map_exit(status: &portable_pty::ExitStatus) -> ExitStatus {
    match status.signal() {
        Some(sig) => ExitStatus {
            exit_code: None,
            signal: Some(sig.to_string()),
        },
        None => ExitStatus {
            // unix exit codes are 0–255 (WEXITSTATUS) → `try_from` is exact in range. The saturating
            // fallback guards a hypothetical > i32::MAX code (unreachable on macOS) WITHOUT a silent
            // wrapping cast (`u32 as i32` would flip the sign on a huge code).
            exit_code: Some(i32::try_from(status.exit_code()).unwrap_or(i32::MAX)),
            signal: None,
        },
    }
}

/// `portable-pty` returns `anyhow::Error`; map to `io::Error` without naming the `anyhow` dep.
fn to_io(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

// ---- FakePty — the §14 deterministic test double ------------------------------------------------

/// A scripted PTY for deterministic tests (§14): a fixed sequence of [`PtyRead`]s ending in EOF + a
/// scripted exit status. Records writes into an `input_sink` the test can assert on.
pub struct FakePty {
    reads: std::collections::VecDeque<PtyRead>,
    exit: ExitStatus,
    input: Arc<Mutex<Vec<u8>>>,
}

impl FakePty {
    pub fn new(reads: Vec<PtyRead>, exit: ExitStatus) -> Self {
        Self {
            reads: reads.into(),
            exit,
            input: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// A handle to the recorded child input — grab it BEFORE moving the `FakePty` into a session.
    pub fn input_sink(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.input)
    }
}

impl Pty for FakePty {
    fn read(&mut self) -> io::Result<PtyRead> {
        // an exhausted script is EOF (a child that closed its PTY).
        Ok(self.reads.pop_front().unwrap_or(PtyRead::Eof))
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.input.lock().unwrap().extend_from_slice(bytes);
        Ok(())
    }

    fn resize(&mut self, _rows: u16, _cols: u16) -> io::Result<()> {
        Ok(())
    }

    fn exit_status(&mut self) -> ExitStatus {
        self.exit.clone()
    }

    fn kill(&mut self) -> io::Result<()> {
        self.reads.clear();
        Ok(())
    }
}

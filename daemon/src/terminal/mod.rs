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
//! **L2:** the `Pty` trait + `PortablePtyHost` + `FakePty` + `TerminalSession` + the exit emission.
//! **L3:** the watermark backpressure + ~30 fps batching — a pure `next_terminal_action` classifier
//! (LESSON §12, timing-free) + a bounded buffer (reading STOPS at the high watermark, so the queue
//! never grows unbounded → no OOM/stall) + `read_step`/`flush` (the reader cadence vs the ~33 ms
//! tick). The flow control is entirely between the PTY child and the socket — it NEVER back-pressures
//! the write-actor (forbidden #3 / LESSON §9).

// 075a — the headless VT screen model (a `vt100`-backed `bytes → screen+scrollback` fold).
// DISPLAY-ONLY (#9 — `vt.rs` is scoped by the `tests/terminal.rs` whole-dir grep-pin + its own
// `tests/vt.rs` pin). The read-pump tap that feeds it lands in 075c (NO wiring here this slice).
mod vt;
pub use vt::{HeadlessVt, VtSnapshot, DEFAULT_SCROLLBACK_CAPACITY};

// 075c — the producer→recovery `ScrollbackStore` seam (a cat-1-neutral trait in `terminal/`, so the
// `session/` import-grep stays clean; the durable §15-gated impl is 075d). `FakeScrollbackStore` is
// test-support-gated.
mod scrollback_store;
#[cfg(any(test, feature = "test-support"))]
pub use scrollback_store::FakeScrollbackStore;
pub use scrollback_store::{NoopScrollbackStore, ScrollbackStore};

use std::io::{self, Read, Write};
// Arc + Mutex back the production `PortablePtyHost`'s shared cross-thread writer (W1-exec/094) AND the
// `test-support`-gated `FakePty` recorded-input handle.
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use nexusops_shared::events::TerminalProcessExited;
use nexusops_shared::ipc::{TerminalControlFrame, TerminalControlKind, TerminalOutputFrame};

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
    /// A cross-thread [`PtyKiller`] handle for this PTY's child (P4.0b-2 L3). EXTRACTED BEFORE the
    /// blocking read-pump takes ownership of the `Pty`, so the `SessionActor` can break the pump's
    /// BLOCKED `read` on Kill/shutdown: a `spawn_blocking` read can't be `abort()`ed, so killing the
    /// child is what makes the read return EOF (a live long-running agent never EOFs on its own → the
    /// pump would hang forever without this, blocking daemon shutdown).
    fn killer(&self) -> Box<dyn PtyKiller>;
    /// A cross-thread [`PtyWriter`] handle for this PTY's stdin (W1-exec/094). EXTRACTED BEFORE the
    /// blocking read-pump takes ownership of the `Pty`, so the `SessionActor` can feed input
    /// (`session.send_message`) WHILE the pump owns the `Pty` on a `spawn_blocking` thread (the
    /// [`killer`](Pty::killer) precedent — the master PTY read + write are independent handles).
    fn writer(&self) -> Box<dyn PtyWriter>;
}

/// A cross-thread handle that kills a [`Pty`]'s child (P4.0b-2 L3 — the live-agent shutdown-bounding
/// path). `Send + Sync` + `kill(&self)` so the `SessionActor` holds it WHILE the read-pump owns the
/// `Pty` on a `spawn_blocking` thread, and kills the child to unblock the pump's blocked `read`.
pub trait PtyKiller: Send + Sync {
    fn kill(&self);
}

/// A cross-thread handle that WRITES input to a [`Pty`]'s child stdin (W1-exec/094). `Send + Sync` +
/// `write(&self, …)` so the `SessionActor` holds it WHILE the read-pump owns the `Pty` on a
/// `spawn_blocking` thread and feeds `session.send_message` keystrokes. **WRITE-ONLY by construction**
/// — there is NO read/output method on this trait, so it can never be a status source (safety #9 holds
/// structurally, not just by convention; status derives from the adapter stream, never the PTY).
pub trait PtyWriter: Send + Sync {
    /// Write input bytes to the child's stdin (keystrokes / a fed message). Errors are the caller's to
    /// degrade-soft (the feed landing is not a safety invariant — LESSON 30).
    fn write(&self, bytes: &[u8]) -> io::Result<()>;
}

// ---- the event-emission seam (daemon-internal; production binds the write-actor at 3.2/3.3) ------

/// The seam through which a [`TerminalSession`] emits its [`TerminalProcessExited`] observation
/// event. The production impl binds the write-actor's `WriteHandle::append` (the §15-gated,
/// non-mutation observation write — INV-SEC-1 governs *mutations*, a process-exit notice is none;
/// LESSON §10/§23) at the per-session drive loop (3.2/3.3); tests use a collecting double.
pub trait TerminalEventSink: Send {
    fn emit_process_exited(&self, event: TerminalProcessExited);
}

// ---- the backpressure classifier + watermarks (L3; LESSON §12 — timing-free) --------------------

/// The default per-terminal output watermarks (bytes). Crossing HIGH pauses reading the child;
/// draining below LOW resumes. **Named consts — 3.5 tunes them with terminal-attach throughput data**
/// (no §18 budget exists until then); sized to bound the in-memory queue without choking interactive use.
pub const DEFAULT_HIGH_WATERMARK: usize = 256 * 1024; // 256 KiB
pub const DEFAULT_LOW_WATERMARK: usize = 64 * 1024; //   64 KiB

/// The flow-control decision for one pump step — a PURE function of the buffered byte count + the
/// watermarks + the paused flag (LESSON §12; mirrors `next_push_action`, never timing-dependent).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerminalAction {
    /// flowing (unpaused, below high) → keep reading the child.
    Emit,
    /// crossed the high watermark while flowing → emit {pause} + stop reading (OS pipe backpressure).
    Pause,
    /// drained to/below the low watermark while paused → emit {resume} + resume reading.
    Resume,
    /// paused + still above low → hold (wait for the client to drain; don't read).
    Hold,
}

/// The pure watermark classifier (LESSON §12). `buffered` = bytes queued for the client but not yet
/// sent (the backpressure signal). Both boundaries are INCLUSIVE: `>= high` pauses, `<= low` resumes.
pub fn next_terminal_action(
    buffered: usize,
    high: usize,
    low: usize,
    paused: bool,
) -> TerminalAction {
    if !paused {
        if buffered >= high {
            TerminalAction::Pause
        } else {
            TerminalAction::Emit
        }
    } else if buffered <= low {
        TerminalAction::Resume
    } else {
        TerminalAction::Hold
    }
}

/// What a pump step produces — a batched output data frame or a flow-control (pause/resume) frame.
///
/// `Output` carries the RAW coalesced bytes (`raw`) ALONGSIDE the §6.4 base64 wire `frame` (075e): the
/// `SessionActor` producer tap feeds the headless VT from `raw` directly, dropping a per-frame
/// `base64::decode` round-trip, while the wire forward still uses `frame` (base64 unchanged). `raw` is
/// exactly `base64::decode(frame.data)` by construction. Keeping `raw` on the emit (vs inside
/// `TerminalSession`) preserves the byte-pipe — `TerminalSession` references no VT type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TerminalEmit {
    Output {
        frame: TerminalOutputFrame,
        raw: Vec<u8>,
    },
    Control(TerminalControlFrame),
}

// ---- TerminalSession — read-accumulate + watermark backpressure + ~30 fps batch -----------------

/// One attached terminal: reads PTY output into a bounded buffer (watermark backpressure — never an
/// unbounded queue), flushes it into seq-numbered **batched** output frames (~30 fps, the caller's
/// flush cadence), and emits the `TerminalProcessExited` observation event once, on child exit.
pub struct TerminalSession {
    terminal_id: TerminalId,
    pty: Box<dyn Pty>,
    sink: Box<dyn TerminalEventSink>,
    /// output read from the child but not yet flushed into a frame (the backpressure buffer; bounded
    /// by `high` — at the high watermark reading STOPS, so this never grows unbounded → no OOM/stall).
    pending: Vec<u8>,
    paused: bool,
    high: usize,
    low: usize,
    next_seq: u64,
    exited: bool,
}

impl TerminalSession {
    /// A session with the default watermarks ([`DEFAULT_HIGH_WATERMARK`] / [`DEFAULT_LOW_WATERMARK`]).
    pub fn new(
        terminal_id: TerminalId,
        pty: Box<dyn Pty>,
        sink: Box<dyn TerminalEventSink>,
    ) -> Self {
        Self::with_watermarks(
            terminal_id,
            pty,
            sink,
            DEFAULT_HIGH_WATERMARK,
            DEFAULT_LOW_WATERMARK,
        )
    }

    /// A session with explicit watermarks (tests use small ones to observe the bound).
    pub fn with_watermarks(
        terminal_id: TerminalId,
        pty: Box<dyn Pty>,
        sink: Box<dyn TerminalEventSink>,
        high: usize,
        low: usize,
    ) -> Self {
        // the hysteresis gap must be ordered (`low < high`); an inverted pair only changes cadence
        // (never breaks the buffer bound or the no-backpressure property), but it is always a caller bug.
        debug_assert!(
            low < high,
            "low watermark ({low}) must be below high ({high})"
        );
        Self {
            terminal_id,
            pty,
            sink,
            pending: Vec::new(),
            paused: false,
            high,
            low,
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

    /// A cross-thread [`PtyKiller`] for this session's PTY child (P4.0b-2 L3) — grab it BEFORE moving
    /// the session into the (blocking) read-pump, so the actor can break the pump's blocked `read` on
    /// Kill/shutdown (a live agent never EOFs on its own; killing the child makes the read return EOF).
    pub fn killer(&self) -> Box<dyn PtyKiller> {
        self.pty.killer()
    }

    /// A cross-thread [`PtyWriter`] for this session's PTY stdin (W1-exec/094) — grab it BEFORE moving
    /// the session into the (blocking) read-pump, so the actor can feed `session.send_message` input
    /// while the pump owns the terminal (the [`killer`](TerminalSession::killer) precedent). WRITE-ONLY.
    pub fn writer(&self) -> Box<dyn PtyWriter> {
        self.pty.writer()
    }

    /// Bytes currently buffered (read but not yet flushed) — the backpressure level. Bounded by the
    /// high watermark (reading stops there), so it never grows unbounded.
    pub fn buffered_bytes(&self) -> usize {
        self.pending.len()
    }

    /// Forward client input bytes to the PTY child (the client→daemon write path).
    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.pty.write(bytes)
    }

    /// Resize the PTY window.
    pub fn resize(&mut self, rows: u16, cols: u16) -> io::Result<()> {
        self.pty.resize(rows, cols)
    }

    /// Kill the PTY child (session abort). The next [`read_step`](TerminalSession::read_step) observes
    /// EOF (the child is gone) and emits the `TerminalProcessExited` event exactly once.
    pub fn kill(&mut self) -> io::Result<()> {
        self.pty.kill()
    }

    /// One READ step (the reader cadence — production: a `spawn_blocking` loop). Applies the watermark
    /// classifier: while flowing, read ONE chunk into the buffer; crossing the high watermark → emit
    /// {pause} + stop reading (OS pipe backpressure propagates to the child); while paused → don't
    /// read. On EOF / a read error → finish (the exit event, exactly once). Data is emitted by
    /// [`flush`](TerminalSession::flush), not here — this returns only any flow-control frame.
    pub fn read_step(&mut self) -> Vec<TerminalEmit> {
        if self.exited {
            return Vec::new();
        }
        match next_terminal_action(self.pending.len(), self.high, self.low, self.paused) {
            TerminalAction::Pause => {
                self.paused = true;
                return vec![TerminalEmit::Control(
                    self.control(TerminalControlKind::Pause),
                )];
            }
            // paused + still above low → wait for the client to drain (flush owns the drain + resume).
            TerminalAction::Hold => return Vec::new(),
            // Defensive: the buffer is ≤ low while paused — normally `flush` already cleared `paused`
            // (it owns the drain + resume), so in the read→flush flow this arm is unreached. If it IS
            // reached (a drained-while-paused state), un-pause and read rather than stall; the
            // {resume} control frame is emitted by `flush`, the drain owner (no duplicate here).
            TerminalAction::Resume => self.paused = false,
            // flowing → read.
            TerminalAction::Emit => {}
        }
        // Resume + Emit both fall through here → read ONE chunk into the buffer.
        match self.pty.read() {
            Ok(PtyRead::Chunk(b)) if !b.is_empty() => {
                self.pending.extend_from_slice(&b);
                Vec::new()
            }
            Ok(PtyRead::Chunk(_)) => Vec::new(), // an empty chunk = nothing available this step
            // EOF / a read error → the child/PTY is gone. Flush any trailing buffered output into a
            // final frame BEFORE the exit event, so (a) the last output precedes the exit signal and
            // (b) the bytes are NEVER dropped even if the caller stops iterating after `is_exited()`
            // without a separate flush (the split read_step/flush drive-loop path).
            Ok(PtyRead::Eof) | Err(_) => {
                let emits = self.drain_pending();
                self.finish();
                emits
            }
        }
    }

    /// One FLUSH tick (the ~30 fps cadence — production: a ~33 ms tokio interval; deterministic, NOT
    /// wall-clock — the caller drives the cadence). Coalesces ALL buffered bytes into ONE **batched**
    /// output frame (if any), then routes the resume decision through the SAME classifier on the
    /// now-drained buffer. NOTE: because flush coalesces EVERYTHING, the buffer is empty afterwards →
    /// the classifier returns `Resume` (0 ≤ low) → a paused terminal resumes immediately. The high/low
    /// hysteresis GAP only bites once a per-frame size cap / partial flush lands (a 3.5 tuning
    /// concern — the classifier is already forward-compatible: a partial drain resumes only at ≤ low).
    pub fn flush(&mut self) -> Vec<TerminalEmit> {
        let mut emits = self.drain_pending();
        if matches!(
            next_terminal_action(self.pending.len(), self.high, self.low, self.paused),
            TerminalAction::Resume
        ) {
            self.paused = false;
            emits.push(TerminalEmit::Control(
                self.control(TerminalControlKind::Resume),
            ));
        }
        emits
    }

    /// One pump = a read step + a flush tick (the L2-compatible convenience). The production drive
    /// loop calls [`read_step`](TerminalSession::read_step) continuously + [`flush`](TerminalSession::flush)
    /// on its own ~33 ms tick instead (so reads BATCH across a tick).
    pub fn pump(&mut self) -> Vec<TerminalEmit> {
        let mut emits = self.read_step();
        emits.extend(self.flush());
        emits
    }

    /// Build a flow-control frame for this terminal.
    fn control(&self, kind: TerminalControlKind) -> TerminalControlFrame {
        TerminalControlFrame {
            terminal_id: self.terminal_id.as_str().to_string(),
            kind,
        }
    }

    /// Coalesce ALL currently-buffered output into ONE batched output frame (if any). Shared by
    /// `flush` (the tick) and the EOF path (the final trailing flush before the exit event).
    fn drain_pending(&mut self) -> Vec<TerminalEmit> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        let bytes = std::mem::take(&mut self.pending);
        // build the base64 wire frame, then hand the RAW bytes out alongside it (075e raw-tap) — the
        // VT consumer feeds from `raw`, the wire forward from `frame`; `raw == base64::decode(frame.data)`.
        let frame = self.frame(&bytes);
        vec![TerminalEmit::Output { frame, raw: bytes }]
    }

    /// Base64-encode a coalesced output buffer into a seq-numbered [`TerminalOutputFrame`] (one `seq`
    /// per EMITTED frame — a gap is client-detectable).
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
    /// the master writer, SHARED behind `Arc<Mutex>` so the cross-thread [`PortableWriter`] handle
    /// (W1-exec/094 — `session.send_message`) and the `&mut self` [`write`](Pty::write) path drive the
    /// SAME PTY stdin (the master read + write are independent; the pump never writes, so it is uncontended).
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PortablePtyHost {
    /// Spawn `program` (with `args`) in a fresh PTY at `cwd`, sized `rows`×`cols`, applying `env`
    /// mutations on top of the INHERITED environment (P4.0b-2: the live `claude` launch removes
    /// `ANTHROPIC_API_KEY` + sets `NEXUSOPS_SESSION_ID`). `CommandBuilder::new` captures the parent
    /// env, so `env_remove` strips a single var while PATH/HOME/… are preserved.
    pub fn spawn(
        program: &str,
        args: &[String],
        cwd: &std::path::Path,
        rows: u16,
        cols: u16,
        env: &[EnvMutation],
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
        // env hygiene: applied on top of the inherited base env (`new` captures the parent's).
        // `Some` overrides; `None` removes the inherited var (PATH/HOME/… untouched).
        for m in env {
            match &m.value {
                Some(v) => builder.env(&m.key, v),
                None => builder.env_remove(&m.key),
            }
        }

        let child = pair.slave.spawn_command(builder).map_err(to_io)?;
        let reader = pair.master.try_clone_reader().map_err(to_io)?;
        let writer = pair.master.take_writer().map_err(to_io)?;
        // drop the slave so the master reader sees EOF once the child closes its PTY end.
        drop(pair.slave);

        Ok(Self {
            master: pair.master,
            reader,
            writer: Arc::new(Mutex::new(writer)),
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
        let mut w = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        w.write_all(bytes)?;
        w.flush()
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

    fn killer(&self) -> Box<dyn PtyKiller> {
        // portable-pty's `clone_killer()` is the cross-thread kill handle — it kills the SAME child
        // from another thread (the actor) while the pump owns this `Pty`. Wrapped in a Mutex because
        // `ChildKiller::kill` takes `&mut self` but `PtyKiller::kill` is `&self` (a shared handle).
        Box::new(PortableKiller(std::sync::Mutex::new(
            self.child.clone_killer(),
        )))
    }

    fn writer(&self) -> Box<dyn PtyWriter> {
        // share the master writer (Arc clone) — the cross-thread `&self` handle drives the SAME PTY stdin
        // as `write(&mut self)`. The `Arc<Mutex>` is what lets the `&self` `PtyWriter` own a share of the
        // single master writer (NOT a concurrency fix — the two write sites are already serialized: the
        // pre-spawn `write_initial_prompt` and the post-spawn `session.send_message` both run on the
        // single write-actor thread / the actor that only exists after create completes). The pump owns
        // the master READER (it never writes), so read + write are independent handles, never contended.
        Box::new(PortableWriter(Arc::clone(&self.writer)))
    }
}

/// The production [`PtyKiller`] — a `portable-pty` `ChildKiller` (cross-thread kill of the live child).
struct PortableKiller(std::sync::Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>);

impl PtyKiller for PortableKiller {
    fn kill(&self) {
        // best-effort: the child may already have exited (a benign kill error); the point is to break
        // the pump's blocked read. Recover a poisoned lock rather than panic (no new failure mode).
        let _ = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .kill();
    }
}

/// The production [`PtyWriter`] (W1-exec/094) — a cross-thread handle over the SHARED master writer, so
/// the `SessionActor` feeds `session.send_message` stdin while the pump owns the `Pty`. WRITE-ONLY.
struct PortableWriter(Arc<Mutex<Box<dyn Write + Send>>>);

impl PtyWriter for PortableWriter {
    fn write(&self, bytes: &[u8]) -> io::Result<()> {
        let mut w = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        w.write_all(bytes)?;
        w.flush()
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

// ---- PtySpawner — the object-safe factory seam (a harness `launch()` injects this) ---------------

/// One environment mutation applied to a spawned child PTY (P4.0b-2 env hygiene + correlation):
/// `value = Some(v)` SETS `key=v` (overriding any inherited value); `value = None` REMOVES `key` from
/// the inherited env. The live `claude` launch strips `ANTHROPIC_API_KEY` (so it rides
/// subscription/OAuth auth, not the API-key billing pool — §15 #8) and sets `NEXUSOPS_SESSION_ID` (so
/// the `PreToolUse` hook subprocess, inheriting it, reports the daemon session it belongs to).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvMutation {
    pub key: String,
    pub value: Option<String>,
}

impl EnvMutation {
    /// REMOVE `key` from the child's inherited env.
    pub fn remove(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: None,
        }
    }

    /// SET `key=value` in the child's env (overriding any inherited value).
    pub fn set(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: Some(value.into()),
        }
    }
}

/// A factory that spawns a fresh [`Pty`] from a launch spec. Object-safe (it produces a
/// `Box<dyn Pty>`) — unlike [`PortablePtyHost::spawn`], which returns `Self` and so cannot live on a
/// `dyn` trait. This is the §14 seam a harness adapter's `launch()` injects: [`PortablePtySpawner`]
/// in production, a recording / fake spawner in tests (returning a [`FakePty`]). The `env` slice is
/// the P4.0b-2 env-hygiene mutations applied to the spawned child (see [`EnvMutation`]).
pub trait PtySpawner: Send {
    fn spawn(
        &self,
        program: &str,
        args: &[String],
        cwd: &std::path::Path,
        rows: u16,
        cols: u16,
        env: &[EnvMutation],
    ) -> io::Result<Box<dyn Pty>>;
}

/// The production [`PtySpawner`] — spawns a real [`PortablePtyHost`] (a live child).
#[derive(Debug, Default, Clone, Copy)]
pub struct PortablePtySpawner;

impl PtySpawner for PortablePtySpawner {
    fn spawn(
        &self,
        program: &str,
        args: &[String],
        cwd: &std::path::Path,
        rows: u16,
        cols: u16,
        env: &[EnvMutation],
    ) -> io::Result<Box<dyn Pty>> {
        Ok(Box::new(PortablePtyHost::spawn(
            program, args, cwd, rows, cols, env,
        )?))
    }
}

// ---- FakePty — the §14 deterministic test double ------------------------------------------------

/// A scripted PTY for deterministic tests (§14): a fixed sequence of [`PtyRead`]s ending in EOF + a
/// scripted exit status. Records writes into an `input_sink` the test can assert on. **`test-support`-
/// gated (P4.0b-2 L3)** — test-only (the production PTY is `PortablePtyHost`); excluded from release.
#[cfg(any(test, feature = "test-support"))]
pub struct FakePty {
    reads: std::collections::VecDeque<PtyRead>,
    exit: ExitStatus,
    input: Arc<Mutex<Vec<u8>>>,
    /// when true, an EXHAUSTED script makes `read` BLOCK (idle, no output, no EOF) until `killed` —
    /// modeling a live long-running agent whose read never returns on its own (P4.0b-2 L3 kill-path).
    /// Default false (the existing scripted-then-EOF behavior).
    loop_until_killed: bool,
    /// set by the [`PtyKiller`] this `FakePty` hands out (or by `kill`) — once set, a blocked/looping
    /// `read` returns EOF (the kill broke the read), so the actor's pump terminates.
    killed: Arc<std::sync::atomic::AtomicBool>,
    /// W1-exec/094 — when set, the [`PtyWriter`] this `FakePty` hands out returns an `Err` on every
    /// write (the LESSON-30 fail-soft test for `session.send_message`). Default false (writes succeed).
    write_fails: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(any(test, feature = "test-support"))]
impl FakePty {
    pub fn new(reads: Vec<PtyRead>, exit: ExitStatus) -> Self {
        Self {
            reads: reads.into(),
            exit,
            input: Arc::new(Mutex::new(Vec::new())),
            loop_until_killed: false,
            killed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            write_fails: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// A NON-self-terminating `FakePty` (P4.0b-2 L3): once its (empty) script is exhausted, `read`
    /// BLOCKS forever — modeling a live agent whose PTY read never EOFs on its own. The actor's
    /// kill-path ([`PtyKiller::kill`]) is what unblocks it (→ EOF → the pump terminates). Without the
    /// kill-path the actor would hang on `pump.await` here.
    pub fn looping() -> Self {
        Self {
            reads: std::collections::VecDeque::new(),
            exit: ExitStatus {
                exit_code: None,
                signal: Some("SIGKILL".to_string()),
            },
            input: Arc::new(Mutex::new(Vec::new())),
            loop_until_killed: true,
            killed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            write_fails: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// W1-exec/094 — a `FakePty` whose handed-out [`PtyWriter`] FAILS every write (the LESSON-30
    /// soft-degrade test for `session.send_message`). Builder form (grab `input_sink`/spawn after).
    pub fn fail_writes(self) -> Self {
        self.write_fails
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self
    }

    /// A handle to the recorded child input — grab it BEFORE moving the `FakePty` into a session.
    pub fn input_sink(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.input)
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Pty for FakePty {
    fn read(&mut self) -> io::Result<PtyRead> {
        use std::sync::atomic::Ordering;
        // a looping FakePty must not ALSO carry scripted reads — they'd be delivered first, so the
        // blocking loop (and thus the kill-path) is never reached. `looping()` guarantees empty reads;
        // this catches a module-internal misuse (the fields are private — unreachable from test crates).
        debug_assert!(
            !self.loop_until_killed || self.reads.is_empty(),
            "a looping FakePty must not carry scripted reads (they defeat the kill-path)"
        );
        if self.killed.load(Ordering::SeqCst) {
            return Ok(PtyRead::Eof); // the killer broke the read.
        }
        match self.reads.pop_front() {
            Some(r) => Ok(r),
            // a looping FakePty's exhausted read BLOCKS (idle) until killed — then EOF (the kill broke
            // the blocked read). This is what makes the kill-path test meaningful: without the actor
            // killing the child, this read never returns and the pump hangs.
            None if self.loop_until_killed => {
                while !self.killed.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Ok(PtyRead::Eof)
            }
            // an exhausted script is EOF (a child that closed its PTY).
            None => Ok(PtyRead::Eof),
        }
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
        self.killed.store(true, std::sync::atomic::Ordering::SeqCst);
        self.reads.clear();
        Ok(())
    }

    fn killer(&self) -> Box<dyn PtyKiller> {
        Box::new(FakeKiller(self.killed.clone()))
    }

    fn writer(&self) -> Box<dyn PtyWriter> {
        // a cross-thread writer over the SAME recorded-input sink (so a test asserts the fed bytes via
        // `input_sink`); honors the `write_fails` flag (the LESSON-30 fail-soft test). The `killer`/`input`
        // shared-handle precedent.
        Box::new(FakeWriter {
            input: Arc::clone(&self.input),
            fail: Arc::clone(&self.write_fails),
        })
    }
}

/// The [`FakePty`]'s cross-thread killer — sets the shared `killed` flag so a blocked/looping `read`
/// EOFs (the test analog of `portable-pty`'s `ChildKiller`). `test-support`-gated with `FakePty`.
#[cfg(any(test, feature = "test-support"))]
struct FakeKiller(Arc<std::sync::atomic::AtomicBool>);

#[cfg(any(test, feature = "test-support"))]
impl PtyKiller for FakeKiller {
    fn kill(&self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// The [`FakePty`]'s cross-thread [`PtyWriter`] (W1-exec/094) — appends to the SAME recorded-input sink
/// the test asserts on (`input_sink`), or returns an `Err` when the `fail` flag is set (the LESSON-30
/// soft-degrade test). `test-support`-gated with `FakePty`.
#[cfg(any(test, feature = "test-support"))]
struct FakeWriter {
    input: Arc<Mutex<Vec<u8>>>,
    fail: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(any(test, feature = "test-support"))]
impl PtyWriter for FakeWriter {
    fn write(&self, bytes: &[u8]) -> io::Result<()> {
        if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(io::Error::other("fake pty write failure (test)"));
        }
        self.input
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(())
    }
}

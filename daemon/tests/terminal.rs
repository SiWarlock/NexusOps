//! P3.4 L2/L3 — the daemon PTY session host + backpressure (integration; FakePty-driven, §14).
//!
//! The PTY is **display-only** (safety #9): these tests drive byte I/O + an OS-derived exit signal,
//! NEVER a status inferred from output. `FakePty` is the deterministic §14 seam (scripted output
//! chunks + a scripted exit); the real `PortablePtyHost` (a live child) is the non-deterministic
//! surface, exercised via `FakePty` for logic + the 3.5 benchmark/manual path for real I/O.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::sync::{Arc, Mutex};

use nexusops_shared::events::TerminalProcessExited;
use nexusopsd::terminal::{
    ExitStatus, FakePty, PtyRead, TerminalEventSink, TerminalId, TerminalSession,
};

/// A collecting [`TerminalEventSink`] for assertions — records every emitted `TerminalProcessExited`
/// (the L1-frozen §7.1 observation event). The PRODUCTION sink binds `WriteHandle::append` (the
/// §15-gated, write-actor observation write) at the 3.2/3.3 per-session drive loop; here the test
/// owns the seam.
#[derive(Clone, Default)]
struct CollectingSink {
    exits: Arc<Mutex<Vec<TerminalProcessExited>>>,
}

impl TerminalEventSink for CollectingSink {
    fn emit_process_exited(&self, event: TerminalProcessExited) {
        self.exits.lock().unwrap().push(event);
    }
}

/// drive a session to completion, collecting every emitted output frame.
fn run_to_exit(session: &mut TerminalSession) -> Vec<nexusops_shared::ipc::TerminalOutputFrame> {
    let mut frames = Vec::new();
    let mut guard = 0;
    while !session.is_exited() {
        frames.extend(session.pump());
        guard += 1;
        assert!(
            guard < 10_000,
            "pump must terminate (reader EOF → exit), not hang"
        );
    }
    frames
}

// ---- 3.4 L2 RED #7 — FakePty spawn → output streams as seq-numbered frames (3.4 happy) ----

#[test]
fn test_fake_pty_spawn_streams_output() {
    // 3.4 happy — scripted FakePty chunks stream out as TerminalOutputFrames; the concatenated,
    // base64-decoded `data` == the scripted bytes; `seq` is strictly monotonic (a gap is
    // client-detectable). Robust to L3 batching (we assert content + monotonic seq, NOT a frame count).
    let pty = FakePty::new(
        vec![
            PtyRead::Chunk(b"hello ".to_vec()),
            PtyRead::Chunk(b"world".to_vec()),
            PtyRead::Eof,
        ],
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        },
    );
    let sink = CollectingSink::default();
    let mut session = TerminalSession::new(
        TerminalId::from_raw("term_test"),
        Box::new(pty),
        Box::new(sink),
    );

    let frames = run_to_exit(&mut session);

    // every frame carries the session's terminal_id
    assert!(frames.iter().all(|f| f.terminal_id == "term_test"));
    // seq strictly increasing
    let seqs: Vec<u64> = frames.iter().map(|f| f.seq).collect();
    assert!(
        seqs.windows(2).all(|w| w[0] < w[1]),
        "seq strictly increasing: {seqs:?}"
    );
    // concatenated base64-decoded data == the scripted output bytes
    let mut out = Vec::new();
    for f in &frames {
        out.extend(STANDARD.decode(&f.data).expect("frame data is base64"));
    }
    assert_eq!(out, b"hello world", "all scripted bytes stream through");
}

// ---- 3.4 L2 RED #8 — input bytes reach the child (the client→daemon write path) ----

#[test]
fn test_pty_write_reaches_child() {
    // 3.4 input — bytes written to the session reach the PTY child's input sink (its stdin). The
    // FakePty records writes; the real host writes to the portable-pty master writer.
    let pty = FakePty::new(
        vec![PtyRead::Eof],
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        },
    );
    let input_sink = pty.input_sink(); // grab the Arc handle BEFORE moving the pty into the session
    let sink = CollectingSink::default();
    let mut session = TerminalSession::new(
        TerminalId::from_raw("term_in"),
        Box::new(pty),
        Box::new(sink),
    );

    session.write(b"ls -la\n").expect("write reaches the child");
    assert_eq!(
        &*input_sink.lock().unwrap(),
        b"ls -la\n",
        "the child's input sink received the exact bytes"
    );
}

// ---- 3.4 L2 RED #9 — child exit → exactly one TerminalProcessExited (OS-derived, #9) ----

#[test]
fn test_pty_exit_emits_terminal_process_exited_once() {
    // 3.4 error + safety #9 — a scripted exit(code=3) emits exactly one TerminalProcessExited
    // {exit_code:Some(3)} via the sink; the exit detail comes from the (fake) OS exit status, NEVER
    // parsed from output bytes. Reader EOF → a clean terminal state; pumping again after exit is a
    // no-op (idempotent — emits the event exactly once, never on a second pump).
    let pty = FakePty::new(
        vec![PtyRead::Chunk(b"bye".to_vec()), PtyRead::Eof],
        ExitStatus {
            exit_code: Some(3),
            signal: None,
        },
    );
    let sink = CollectingSink::default();
    let mut session = TerminalSession::new(
        TerminalId::from_raw("term_x"),
        Box::new(pty),
        Box::new(sink.clone()),
    );

    run_to_exit(&mut session);
    // pump again AFTER exit — must NOT emit a second event, must NOT panic/hang
    session.pump();
    session.pump();

    let exits = sink.exits.lock().unwrap();
    assert_eq!(exits.len(), 1, "exactly one TerminalProcessExited");
    assert_eq!(exits[0].terminal_id, "term_x");
    assert_eq!(
        exits[0].exit_code,
        Some(3),
        "exit detail from the OS status, not output"
    );
    assert_eq!(exits[0].signal, None);
}

// ---- 3.4 L2 RED #9b — a signal-kill maps to signal (OS-derived; the exit_code XOR signal mirror) ----

#[test]
fn test_pty_signal_kill_emits_signal() {
    // safety #9 — a child killed by a signal surfaces `signal:Some`, `exit_code:None` (the mirror of
    // the normal-exit case). Still OS-derived, never output-parsed.
    let pty = FakePty::new(
        vec![PtyRead::Eof],
        ExitStatus {
            exit_code: None,
            signal: Some("SIGKILL".to_string()),
        },
    );
    let sink = CollectingSink::default();
    let mut session = TerminalSession::new(
        TerminalId::from_raw("term_sig"),
        Box::new(pty),
        Box::new(sink.clone()),
    );
    run_to_exit(&mut session);
    let exits = sink.exits.lock().unwrap();
    assert_eq!(exits.len(), 1);
    assert_eq!(exits[0].terminal_id, "term_sig");
    assert_eq!(exits[0].exit_code, None);
    assert_eq!(exits[0].signal, Some("SIGKILL".to_string()));
}

// ---- 3.4 L2 RED #9c — an empty output chunk produces no frame (the defensive skip arm) ----

#[test]
fn test_empty_chunk_produces_no_frame() {
    // A degenerate empty chunk (a real blocking PTY read of 0 bytes is EOF, never an empty Chunk)
    // produces NO frame; the real bytes still stream and `seq` stays gapless (the empty chunk never
    // consumes a seq). Exercises the pump's defensive empty-chunk arm.
    let pty = FakePty::new(
        vec![
            PtyRead::Chunk(Vec::new()),
            PtyRead::Chunk(b"data".to_vec()),
            PtyRead::Eof,
        ],
        ExitStatus {
            exit_code: Some(0),
            signal: None,
        },
    );
    let sink = CollectingSink::default();
    let mut session = TerminalSession::new(
        TerminalId::from_raw("term_empty"),
        Box::new(pty),
        Box::new(sink),
    );
    let frames = run_to_exit(&mut session);
    assert_eq!(
        frames.len(),
        1,
        "empty chunk skipped; exactly one real frame"
    );
    assert_eq!(frames[0].seq, 0, "the empty chunk did not consume a seq");
    assert_eq!(STANDARD.decode(&frames[0].data).unwrap(), b"data");
}

// ---- 3.4 L2 RED #9d — session kill aborts the child → exactly one exit event ----

#[test]
fn test_session_kill_aborts_and_emits_exit() {
    // session.kill() aborts the child (FakePty drops its queued reads) → the next pump observes EOF
    // → exactly one TerminalProcessExited (idempotent). The drive loop (3.2/3.3) calls this to abort.
    let pty = FakePty::new(
        vec![
            PtyRead::Chunk(b"running...".to_vec()),
            PtyRead::Chunk(b"more".to_vec()),
        ],
        ExitStatus {
            exit_code: None,
            signal: Some("SIGKILL".to_string()),
        },
    );
    let sink = CollectingSink::default();
    let mut session = TerminalSession::new(
        TerminalId::from_raw("term_kill"),
        Box::new(pty),
        Box::new(sink.clone()),
    );
    let _ = session.pump(); // stream the first chunk
    session.kill().expect("kill aborts the child");
    run_to_exit(&mut session);
    let exits = sink.exits.lock().unwrap();
    assert_eq!(exits.len(), 1, "kill → exactly one exit event");
    assert_eq!(exits[0].terminal_id, "term_kill");
    assert_eq!(exits[0].signal.as_deref(), Some("SIGKILL"));
}

// ---- 3.4 L2 RED #10 — the terminal module is display-only: no status-derivation API (#9 / #4) ----

#[test]
fn test_terminal_module_has_no_status_derivation_api() {
    // safety #9 / forbidden #4 — the terminal module is DISPLAY-ONLY: its surface is byte I/O + an
    // OS-derived exit signal, NEVER a session/agent status inferred from terminal output. A
    // forbidden-pattern grep pin over every source file in src/terminal/ (the brief-sanctioned
    // structural assertion). The forbidden tokens are IDENTIFIER-like — they appear only in actual
    // status-derivation code, never in display-only prose: the §5.1 status-MACHINE import + any
    // infer/parse/derive-status API. (`ExitStatus`/`exit_status` — the OS exit detail — are ALLOWED;
    // the prose word "scrape" is deliberately NOT a token: it occurs legitimately in "never scrapes"
    // safety comments, so matching it would false-positive on the very comments documenting #9.)
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/terminal");
    let forbidden = [
        "NormalizedStatus",
        "nexusops_shared::status",
        // `harness::` catches any import/use of the harness layer (`crate::harness::…`,
        // `nexusops_shared::harness::…`, a bare `harness::Foo` reference) — while NOT matching the
        // legitimate prose "the `harness` layer" (no `::`) in this module's #9 doc comments.
        "harness::",
        "infer_status",
        "parse_status",
        "derive_status",
    ];
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).expect("src/terminal/ present") {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let src = std::fs::read_to_string(&path).unwrap();
            for tok in forbidden {
                assert!(
                    !src.contains(tok),
                    "{path:?} must not reference `{tok}` — the PTY is display-only (#9 / forbidden #4)"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 1, "at least src/terminal/mod.rs was scanned");
}

// ---- 3.4 L2 RED #11 — terminal_id is a daemon runtime handle minted via the IdGen seam ----

#[test]
fn test_terminal_id_minted_via_idgen() {
    // Q1 (lead-ratified) — terminal_id = an opaque daemon runtime handle (term_<ULID>), minted via
    // the §14 IdGen seam (NOT a 23rd LOCKED-22 IdKind). Re-minted per attach/resume.
    use nexusopsd::idgen::FixedIdGen;
    let gen = FixedIdGen::new();
    let a = TerminalId::mint(&gen);
    let b = TerminalId::mint(&gen);
    assert!(a.as_str().starts_with("term_"), "carries the term_ prefix");
    assert_ne!(a.as_str(), b.as_str(), "each mint is unique");
    // FixedIdGen mints a deterministic terminal-id sequence (§14) via a SEPARATE counter (the
    // new_outbox_id precedent) — so minting terminal ids never perturbs the event-id golden log.
    let gen2 = FixedIdGen::new();
    assert_eq!(
        TerminalId::mint(&gen2).as_str(),
        a.as_str(),
        "deterministic mint sequence"
    );
}

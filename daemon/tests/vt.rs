//! 075a — the headless VT screen model (`terminal::HeadlessVt`, wrapping `vt100::Parser`).
//!
//! Golden-fixture unit tests for the deterministic `bytes → screen-state` core (ARCHITECTURE
//! §6.4 / §9 / §9.1; the survival `Replayed` rung's serialize/replay input, 075b/c). The model is
//! **DISPLAY-ONLY** (#9) — Test 6 structurally pins that it derives no status. Each test feeds a
//! fixed byte sequence and asserts the resulting screen/scrollback state — no live PTY, no timing.

use nexusopsd::terminal::HeadlessVt;

/// Test 1 — Plain text folds into the visible screen — the `bytes → screen-state` core (§6.4/§9).
#[test]
fn test_vt_plain_text_renders_to_screen() {
    let mut vt = HeadlessVt::new(24, 80, 1000);
    vt.process(b"hello");
    assert_eq!(
        vt.screen_contents().lines().next(),
        Some("hello"),
        "plain bytes render onto row 0"
    );
}

/// Test 2 — A CSI cursor-move + overwrite leaves the FINAL cell state, not the literal escape bytes —
/// the whole point of a VT emulator (§9 ADR-009).
#[test]
fn test_vt_csi_cursor_move_and_overwrite() {
    let mut vt = HeadlessVt::new(24, 80, 1000);
    vt.process(b"abc");
    vt.process(b"\x1b[H"); // CSI cursor-home → (row 1, col 1)
    vt.process(b"X"); // overwrites the 'a'
    assert_eq!(
        vt.screen_contents().lines().next(),
        Some("Xbc"),
        "escape sequences mutate cell state, not literal bytes"
    );
}

/// Test 3 — Scrollback accumulates once content scrolls off the top; `has_scrollback()` flips false→true
/// and the scrolled-off content is retrievable — the `Replayed` rung keys off scrollback presence
/// (LESSON §36 `decide_resume` `has_scrollback`).
#[test]
fn test_vt_scrollback_accumulates() {
    // Capacity (100) is STRICTLY GREATER than the filled count we drive (3) — so `scrollback_len()`
    // asserting `== 3` locks the probe to the FILLED count, not the construction capacity (a
    // capacity==filled test would mask a capacity/filled regression). Step-2.5 strengthening.
    let mut vt = HeadlessVt::new(2, 10, 100);
    assert!(!vt.has_scrollback(), "nothing has scrolled off yet");
    assert_eq!(vt.scrollback_len(), 0);

    vt.process(b"line0\r\nline1"); // fills both rows exactly — no scroll
    assert!(
        !vt.has_scrollback(),
        "two lines fit a 2-row screen exactly — still no scrollback"
    );

    vt.process(b"\r\nline2\r\nline3\r\nline4"); // 3 line-feeds at the bottom → 3 rows scroll off
    assert!(vt.has_scrollback(), "lines scrolled off into scrollback");
    assert_eq!(
        vt.scrollback_len(),
        3,
        "exactly the 3 overflow rows (line0/line1/line2) are in scrollback — the FILLED count, not the 100-row capacity"
    );

    // content retrievable: scroll fully back → the oldest line is at the viewport top.
    let oldest = vt.view_at_scrollback(vt.scrollback_len());
    assert!(
        oldest.contains("line0"),
        "oldest scrolled-off line is retrievable: {oldest:?}"
    );
}

/// Test 4 — Alt-screen state is tracked across `\e[?1049h` / `\e[?1049l` — O-2 "accurate alt-screen VT
/// re-render" (§0.1).
#[test]
fn test_vt_alternate_screen_toggle() {
    let mut vt = HeadlessVt::new(24, 80, 0);
    assert!(!vt.alternate_screen(), "starts on the normal screen");
    vt.process(b"\x1b[?1049h");
    assert!(vt.alternate_screen(), "?1049h enters the alternate screen");
    vt.process(b"\x1b[?1049l");
    assert!(!vt.alternate_screen(), "?1049l exits the alternate screen");
}

/// Test 5 — Dimensions are explicit at construction and mutable via `resize` — dims are load-bearing for
/// the 075b snapshot fidelity (the §6.4 frames carry none; the model owns them).
#[test]
fn test_vt_size_and_resize() {
    let mut vt = HeadlessVt::new(24, 80, 100);
    assert_eq!(vt.size(), (24, 80), "construction dims");
    vt.resize(10, 40);
    assert_eq!(vt.size(), (10, 40), "resized dims");
}

/// Test 6 — STRUCTURAL #9 pin (scoped to `vt.rs`): the model exposes NO status-derivation surface and
/// imports nothing from the harness status layer — the PTY/VT screen is display-only, status is
/// NEVER derived from it (§7.2 / §9.1; the `terminal/mod.rs` grep-pin precedent, LESSON §24).
#[test]
fn test_vt_is_display_only_never_status() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/terminal/vt.rs");
    let src = std::fs::read_to_string(path).expect("src/terminal/vt.rs present");
    let forbidden = [
        "derive_status",
        "NormalizedStatus",
        "status::Session",
        "nexusops_shared::status",
        "harness::",
    ];
    for tok in forbidden {
        assert!(
            !src.contains(tok),
            "src/terminal/vt.rs must not reference `{tok}` — the VT model is display-only (#9 / forbidden #4)"
        );
    }
}

/// Test 7 — Untrusted/garbage/partial-escape input parses tolerantly — no panic, screen stays readable
/// (the LESSON §42 untrusted-input parser family; an agent PTY stream is untrusted ingress).
#[test]
fn test_vt_garbage_input_no_panic() {
    let mut vt = HeadlessVt::new(24, 80, 100);
    vt.process(&[0xff, 0xfe, 0x00, 0x1b]); // invalid UTF-8 + a dangling ESC
    vt.process(b"\x1b["); // a truncated CSI (no final byte)
    vt.process(&[0x07, 0x08, 0x1b, 0x5b, 0x99, 0x6d]); // BEL, BS, a bogus CSI param
    let _ = vt.screen_contents(); // still consistent + readable
    let _ = vt.alternate_screen();
    let _ = vt.scrollback_len(); // the probe path also survives garbage ingress
                                 // the assertion is implicit: none of the above panicked.
}

/// Test 8 — entering the alternate screen does NOT lose the normal screen's scrollback count. A
/// vim-style session (fill scrollback → enter alt-screen → emit output → exit) must still report
/// the normal-screen scrollback throughout. Guards the alt-grid edge: vt100 constructs the alternate
/// grid with scrollback capacity 0, so a naive probe during alt-screen would clobber the real count.
#[test]
fn test_vt_scrollback_preserved_across_alt_screen() {
    let mut vt = HeadlessVt::new(2, 10, 100);
    vt.process(b"line0\r\nline1\r\nline2\r\nline3"); // 2 rows scroll off the 2-row screen
    assert_eq!(
        vt.scrollback_len(),
        2,
        "two rows scrolled off the normal screen"
    );

    vt.process(b"\x1b[?1049h"); // enter the alternate screen
    assert!(vt.alternate_screen());
    assert_eq!(
        vt.scrollback_len(),
        2,
        "entering the alt screen must not clobber the normal-screen scrollback count"
    );

    vt.process(b"editor output"); // alt-screen activity
    assert_eq!(
        vt.scrollback_len(),
        2,
        "alt-screen activity must not clobber the normal-screen scrollback count"
    );
    assert!(vt.has_scrollback());

    vt.process(b"\x1b[?1049l"); // exit → back to the normal screen
    assert!(!vt.alternate_screen());
    assert_eq!(
        vt.scrollback_len(),
        2,
        "scrollback count intact after returning to the normal screen"
    );
}

// ============================================================================================
// 075b — snapshot serialize + replay fidelity (extends the 075a model)
// ============================================================================================

/// One golden-corpus case: `(label, (rows, cols, scrollback_capacity), byte-stream)`.
type VtCase = (&'static str, (u16, u16, usize), Vec<u8>);

/// A synthesized golden corpus — deterministic, committed fixtures (the recorded-fixture
/// discipline; no live PTY capture). Covers plain text, escape-heavy formatting, cursor moves,
/// scrollback overflow, and BOTH alt-screen cases (active + exited-with-scrollback).
fn vt_corpus() -> Vec<VtCase> {
    // A large screen (24 rows) fed 27 lines → exactly 3 rows scroll off: the `1 <= N < rows` case
    // (the reconstruction's cursor sweeps to the bottom WITHOUT scrolling, then scrolls exactly N).
    let mut big_small_sb = Vec::new();
    for i in 0..27u8 {
        if i > 0 {
            big_small_sb.extend_from_slice(b"\r\n");
        }
        big_small_sb.extend_from_slice(format!("bigrow{i}").as_bytes());
    }
    vec![
        ("plain", (24, 80, 1000), b"hello world".to_vec()),
        ("large-screen-small-scrollback", (24, 80, 100), big_small_sb),
        (
            "multiline",
            (24, 80, 1000),
            b"alpha\r\nbeta\r\ngamma".to_vec(),
        ),
        (
            "escape-heavy",
            (24, 80, 1000),
            b"\x1b[1;31mRED\x1b[0m plain \x1b[4mUL\x1b[0m \x1b[7minv\x1b[0m".to_vec(),
        ),
        (
            "cursor-moves",
            (24, 80, 1000),
            b"abc\x1b[Hxyz\x1b[2;3HZ".to_vec(),
        ),
        (
            "scrollback",
            (2, 10, 100),
            b"line0\r\nline1\r\nline2\r\nline3\r\nline4".to_vec(),
        ),
        (
            "alt-active",
            (10, 20, 100),
            b"normal text\r\n\x1b[?1049halt editor".to_vec(),
        ),
        (
            "alt-exited",
            (2, 10, 100),
            b"a0\r\na1\r\na2\r\n\x1b[?1049halt\x1b[?1049l".to_vec(),
        ),
    ]
}

/// Build a model from `(rows, cols, capacity)` and feed it `stream`.
fn feed(dims: (u16, u16, usize), stream: &[u8]) -> HeadlessVt {
    let mut vt = HeadlessVt::new(dims.0, dims.1, dims.2);
    vt.process(stream);
    vt
}

/// 075b Test 1 — plain text survives snapshot → restore: identical `screen_contents()` (§6.4/§9).
#[test]
fn test_vt_snapshot_restore_plain_text() {
    let mut vt = feed((24, 80, 1000), b"hello\r\nworld");
    let snap = vt.snapshot();
    let restored = HeadlessVt::from_snapshot(&snap);
    assert_eq!(restored.screen_contents(), vt.screen_contents());
}

/// 075b Test 2 — an escape-heavy (color/attr/cursor) stream round-trips: plain content matches AND
/// the idempotent snapshot proves the FORMATTING state survives (O-2 accurate VT re-render, §0.1).
#[test]
fn test_vt_snapshot_restore_escape_heavy() {
    let mut vt = feed(
        (24, 80, 1000),
        b"\x1b[1;31mRED\x1b[0m plain \x1b[4mUL\x1b[0m \x1b[7minv\x1b[0m",
    );
    let snap = vt.snapshot();
    let mut restored = HeadlessVt::from_snapshot(&snap);
    assert_eq!(restored.screen_contents(), vt.screen_contents());
    assert_eq!(
        restored.snapshot(),
        snap,
        "formatting survives — the restored screen re-snapshots identically"
    );
}

/// 075b Test 3 — a stream that overflows the screen round-trips: restored `scrollback_len()` equals
/// the original and `view_at_scrollback(k)` matches for every k in range (LESSONS §57/§36).
#[test]
fn test_vt_snapshot_restore_scrollback() {
    let mut vt = feed((2, 10, 100), b"line0\r\nline1\r\nline2\r\nline3\r\nline4");
    assert!(vt.has_scrollback());
    let snap = vt.snapshot();
    let mut restored = HeadlessVt::from_snapshot(&snap);
    assert_eq!(
        restored.scrollback_len(),
        vt.scrollback_len(),
        "restored scrollback row count matches"
    );
    let n = vt.scrollback_len();
    for k in 0..=n {
        assert_eq!(
            restored.view_at_scrollback(k),
            vt.view_at_scrollback(k),
            "view at scrollback offset {k} matches"
        );
    }
}

/// 075b Test 4 — a snapshot taken while the alt screen is active restores with
/// `alternate_screen()==true` and the alt content intact (O-2 alt-screen fidelity — the two-buffer
/// case).
#[test]
fn test_vt_snapshot_alt_screen_active() {
    let mut vt = feed((10, 20, 100), b"normal text\r\n\x1b[?1049halt editor");
    assert!(vt.alternate_screen());
    let snap = vt.snapshot();
    let restored = HeadlessVt::from_snapshot(&snap);
    assert!(restored.alternate_screen(), "alt-screen flag round-trips");
    assert_eq!(
        restored.screen_contents(),
        vt.screen_contents(),
        "alt-screen content intact"
    );
}

/// 075b Test 5 — a snapshot taken after `?1049l` restores the NORMAL screen + its preserved
/// scrollback (the alt-grid-clobber regression's serialize cousin, LESSONS §57).
#[test]
fn test_vt_snapshot_alt_screen_exited_preserves_scrollback() {
    let mut vt = feed((2, 10, 100), b"a0\r\na1\r\na2\r\n\x1b[?1049halt\x1b[?1049l");
    assert!(!vt.alternate_screen());
    assert!(vt.has_scrollback());
    let snap = vt.snapshot();
    let mut restored = HeadlessVt::from_snapshot(&snap);
    assert!(!restored.alternate_screen());
    assert_eq!(restored.screen_contents(), vt.screen_contents());
    assert_eq!(restored.scrollback_len(), vt.scrollback_len());
    let n = vt.scrollback_len();
    for k in 0..=n {
        assert_eq!(restored.view_at_scrollback(k), vt.view_at_scrollback(k));
    }
}

/// 075b Test 6 — the loss-proof: `snapshot(from_snapshot(s)) == s` byte-identical across the whole
/// corpus. The strongest fidelity pin (no private cell API needed).
#[test]
fn test_vt_snapshot_idempotent() {
    for (label, dims, stream) in vt_corpus() {
        let mut vt = feed(dims, &stream);
        let snap1 = vt.snapshot();
        let mut restored = HeadlessVt::from_snapshot(&snap1);
        let snap2 = restored.snapshot();
        assert_eq!(
            snap2, snap1,
            "snapshot(from_snapshot(s)) == s for corpus `{label}`"
        );
    }
}

/// 075b Test 8 — REGRESSION PIN for the alt-screen two-buffer limitation (the named two-buffer
/// edge): a snapshot taken while the alt screen is ACTIVE carries an EMPTY scrollback — the hidden
/// normal buffer (and its scrollback) is deliberately NOT retained. This converts the known gap into
/// a test so a future (b)-style two-buffer upgrade is a DELIBERATE test change, not a silent fidelity
/// regression. (The idempotent test proves serialize/restore CONSISTENCY but not first-snapshot
/// COMPLETENESS — this is the honest completeness record.)
#[test]
fn test_vt_snapshot_alt_active_drops_hidden_scrollback() {
    // Fill the NORMAL buffer's scrollback, THEN enter the alt screen.
    let mut vt = feed((2, 10, 100), b"n0\r\nn1\r\nn2\r\nn3\x1b[?1049halt");
    assert!(vt.alternate_screen());
    let snap = vt.snapshot();
    assert!(snap.alternate_screen());
    assert_eq!(
        snap.scrollback_rows(),
        0,
        "an alt-active snapshot retains no scrollback — the hidden normal buffer is not captured (two-buffer edge)"
    );

    // The RESTORED alt-active model honestly reflects the lossy snapshot: no scrollback. This is the
    // accepted two-buffer limitation's CONSEQUENCE (orch-ruled future-TODO) — pinned here so 075c's
    // decide_resume wiring (has_scrollback → the Replayed rung, LESSON §36) sees an honest `false`
    // for a session snapshotted mid-alt-screen, rather than a fabricated count it cannot reproduce.
    let restored = HeadlessVt::from_snapshot(&snap);
    assert!(restored.alternate_screen());
    assert!(
        !restored.has_scrollback(),
        "a restored alt-active model has no scrollback — honestly reflecting the lossy snapshot"
    );
    assert_eq!(restored.scrollback_len(), 0);

    // Contrast: the SAME stream after exiting the alt screen DOES retain the normal scrollback.
    let mut vt2 = feed(
        (2, 10, 100),
        b"n0\r\nn1\r\nn2\r\nn3\x1b[?1049halt\x1b[?1049l",
    );
    assert!(!vt2.alternate_screen());
    let snap2 = vt2.snapshot();
    assert!(
        snap2.scrollback_rows() > 0,
        "exiting the alt screen restores the normal buffer's scrollback into the snapshot"
    );
}

/// 075b Test 7 — a restored model is fully LIVE, not a frozen husk: `resize()` and subsequent
/// `process()` behave identically to the original (the model is reused post-restore, 075c).
#[test]
fn test_vt_resize_after_restore() {
    let mut vt = feed((24, 80, 1000), b"resize me\r\nsecond line");
    let snap = vt.snapshot();
    let mut restored = HeadlessVt::from_snapshot(&snap);

    vt.resize(10, 40);
    restored.resize(10, 40);
    assert_eq!(restored.size(), vt.size());
    assert_eq!(
        restored.screen_contents(),
        vt.screen_contents(),
        "resize behaves identically post-restore"
    );

    vt.process(b"\r\nmore output");
    restored.process(b"\r\nmore output");
    assert_eq!(
        restored.screen_contents(),
        vt.screen_contents(),
        "the restored model keeps folding new bytes"
    );
}

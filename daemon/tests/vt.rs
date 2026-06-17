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
    assert_eq!(vt.scrollback_len(), 2, "two rows scrolled off the normal screen");

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

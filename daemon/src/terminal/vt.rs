//! 075a — the headless VT screen model: a [`vt100::Parser`]-backed `bytes → screen + scrollback`
//! fold (ARCHITECTURE §6.4 / §9 / §9.1). The deterministic screen-state core the survival
//! `Replayed` rung serializes/replays (075b) and the `TerminalSession` read-pump feeds (075c).
//!
//! **DISPLAY-ONLY (#9).** This is a screen MODEL, never a status source — it exposes no
//! status-derivation surface and imports nothing from the harness status layer (asserted
//! structurally by `tests/vt.rs::test_vt_is_display_only_never_status` + the whole-`src/terminal/`
//! grep-pin in `tests/terminal.rs`). Session/agent status comes from the SDK/app-server streams (the
//! harness layer), NEVER from terminal output bytes (§7.2 / §9.1; the sibling `terminal/mod.rs`
//! byte-pipe precedent, LESSON §24).
//!
//! **No wiring this slice.** The read-pump tap that feeds this model — and the
//! `has_scrollback`/`replayed_event_count` recovery-seam plumbing — land in 075c; this slice is the
//! mechanism-first core (the 4.0a/3.3a "mechanism built test-first, driven next slice" precedent).

/// A headless VT/ANSI screen emulator: folds a raw PTY byte stream into an in-memory screen +
/// scrollback (wrapping [`vt100::Parser`]). Owns its dimensions — the §6.4 Terminal-Channel frames
/// carry none. DISPLAY-ONLY: it never derives a session/agent status from screen content (#9).
pub struct HeadlessVt {
    parser: vt100::Parser,
    /// The number of rows currently held in scrollback (rows that have scrolled off the top of the
    /// live screen) — cached after each mutating op. vt100's [`vt100::Screen::scrollback`] reports
    /// the scroll *position*, NOT the filled length, so the filled count is derived by an
    /// observationally-pure probe ([`refresh_scrollback_rows`](Self::refresh_scrollback_rows))
    /// rather than re-derived on every read — keeping [`has_scrollback`](Self::has_scrollback) /
    /// [`scrollback_len`](Self::scrollback_len) clean `&self` reads.
    scrollback_rows: usize,
}

impl HeadlessVt {
    /// Build a model with explicit `rows`×`cols` dimensions and a scrollback ring holding up to
    /// `scrollback_capacity` rows (the vt100 ring CAPACITY — the max retained; the *filled* count is
    /// [`scrollback_len`](Self::scrollback_len)). No magic default — the caller states the dims.
    #[must_use]
    pub fn new(rows: u16, cols: u16, scrollback_capacity: usize) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, scrollback_capacity),
            scrollback_rows: 0,
        }
    }

    /// Fold a chunk of raw PTY output into the screen — plain text AND VT escape sequences (a
    /// cursor-move + overwrite leaves the final cell state, not the literal bytes). Tolerant:
    /// invalid UTF-8 / truncated escapes never panic (untrusted ingress, LESSON §42).
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.refresh_scrollback_rows();
    }

    /// The plain-text contents of the visible screen (no formatting; vt100 trims trailing blank
    /// cells/rows, rows joined by `\n`). The scrollback offset is always 0 outside
    /// [`view_at_scrollback`](Self::view_at_scrollback), so this is a clean `&self` read of the live
    /// screen.
    #[must_use]
    pub fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    /// Whether the alternate screen is active (`\e[?1049h` enters, `\e[?1049l` exits) — O-2
    /// accurate alt-screen VT re-render (§0.1).
    #[must_use]
    pub fn alternate_screen(&self) -> bool {
        self.parser.screen().alternate_screen()
    }

    /// The current dimensions as `(rows, cols)`.
    #[must_use]
    pub fn size(&self) -> (u16, u16) {
        self.parser.screen().size()
    }

    /// Resize the screen to `rows`×`cols` (vt100 reflows the grid). Refreshes the cached scrollback
    /// count, since a reflow can change what has scrolled off the top.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.screen_mut().set_size(rows, cols);
        self.refresh_scrollback_rows();
    }

    /// Whether any content has scrolled off the top into scrollback (false before the first scroll,
    /// true after) — the `Replayed`-rung presence signal (LESSON §36 `decide_resume`).
    #[must_use]
    pub fn has_scrollback(&self) -> bool {
        self.scrollback_rows > 0
    }

    /// The number of rows currently held in scrollback — the FILLED count (≤ the construction
    /// capacity), not the capacity itself.
    #[must_use]
    pub fn scrollback_len(&self) -> usize {
        self.scrollback_rows
    }

    /// Read the screen as it would appear scrolled back by `offset` rows (clamped to the filled
    /// scrollback). `offset == scrollback_len()` puts the oldest retained rows at the viewport top.
    /// Restores the live view (offset 0) before returning — observationally pure. The minimal read
    /// primitive the 075b serialize/replay and the 075c read-pump build on.
    #[must_use]
    pub fn view_at_scrollback(&mut self, offset: usize) -> String {
        let screen = self.parser.screen_mut();
        screen.set_scrollback(offset);
        let contents = screen.contents();
        screen.set_scrollback(0);
        contents
    }

    /// Recompute the FILLED scrollback row count via an observationally-pure probe: vt100 only
    /// exposes the scroll *position*, so scroll to the maximum (`set_scrollback` clamps to the
    /// filled length), read the resulting position (== the filled count), then restore the live view
    /// (offset 0). Called after every mutating op so the count accessors stay `&self`. The screen is
    /// always at offset 0 on entry (we never leave it scrolled), so this leaves it unchanged.
    ///
    /// **Alt-screen guard:** vt100's ALTERNATE grid is constructed with scrollback capacity 0, and
    /// while the alt screen is active `screen_mut()` resolves to it — so probing then would read 0
    /// and clobber the real normal-screen count. The normal grid's scrollback is FROZEN during an
    /// alt-screen session (new content goes to the alt grid, and `\e[?1049l` restores the untouched
    /// normal grid), so the cached count stays correct: skip the probe while alt-screen is active and
    /// it re-runs once the next mutating op returns to the normal screen.
    fn refresh_scrollback_rows(&mut self) {
        if self.parser.screen().alternate_screen() {
            return;
        }
        let screen = self.parser.screen_mut();
        screen.set_scrollback(usize::MAX);
        self.scrollback_rows = screen.scrollback();
        screen.set_scrollback(0);
    }
}

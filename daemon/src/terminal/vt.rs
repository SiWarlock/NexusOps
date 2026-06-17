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
    /// The scrollback ring CAPACITY this model was built with (vt100's `Screen` does not expose it,
    /// so we retain it — `snapshot`/`from_snapshot` (075b) round-trip it so a restored model has the
    /// same ring bound).
    scrollback_capacity: usize,
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
            scrollback_capacity,
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

    // ---- 075b — serialize (snapshot) + restore (from_snapshot) -----------------------------------

    /// Serialize the model into a [`VtSnapshot`] — the `Replayed`-rung "accurate alt-screen VT
    /// re-render" mechanism (O-2, §0.1). Captures the dims + ring capacity + the alt-screen flag +
    /// the visible screen (vt100 `state_formatted()` — escape codes that reproduce the screen
    /// content, cursor, attrs and modes, so formatting survives) + the scrollback (plain rows,
    /// oldest→newest; scrollback formatting is not part of the `Replayed` fidelity surface —
    /// [`view_at_scrollback`](Self::view_at_scrollback) reads plain `contents()`).
    ///
    /// Takes `&mut self` because reading the scrollback rows scrolls the vt100 viewport — but the
    /// probe restores the live view (offset 0) before returning, so it is observationally pure.
    ///
    /// **Alt-screen note:** while the alt screen is active, vt100 exposes only the alt grid (the
    /// normal buffer + its scrollback are hidden behind it), so an alt-active snapshot captures the
    /// alt screen with an empty scrollback. That is self-consistent (restore reproduces exactly what
    /// was captured → the idempotent round-trip holds) but the hidden normal buffer is not retained;
    /// the brief's named two-buffer edge (075d/future).
    pub fn snapshot(&mut self) -> VtSnapshot {
        let (rows, cols) = self.size();
        // Captured at offset 0 (the live view) BEFORE `capture_scrollback` scrolls: whether the
        // active visible screen has any content (vt100 `contents()` trims trailing blanks → "" iff
        // blank). The 075c `has_restorable_content()` signal — a mid-alt-screen session has a
        // non-blank (alt) screen worth re-rendering even with zero scrollback rows.
        let screen_nonblank = !self.screen_contents().is_empty();
        VtSnapshot {
            version: SNAPSHOT_VERSION,
            rows,
            cols,
            scrollback_capacity: self.scrollback_capacity,
            alternate_screen: self.alternate_screen(),
            screen_nonblank,
            screen: self.parser.screen().state_formatted(),
            scrollback: self.capture_scrollback(),
        }
    }

    /// Rebuild an equivalent model from a [`VtSnapshot`] (mirrors [`new`](Self::new)). Replays the
    /// captured scrollback rows so they land in the ring, (re-)enters the alt screen if the snapshot
    /// was taken there, then renders the visible screen from the captured `state_formatted` bytes
    /// (which begin with a clear-screen + clear-attrs, so they overwrite the live screen cleanly
    /// without disturbing scrollback). The result re-snapshots byte-identically (the loss-proof).
    #[must_use]
    pub fn from_snapshot(snap: &VtSnapshot) -> Self {
        // 075b only ever restores same-version snapshots built in-memory; a persisted cross-version
        // snapshot is 075d's concern (a `Result`-returning deserialize + migration off this header).
        debug_assert_eq!(
            snap.version, SNAPSHOT_VERSION,
            "VtSnapshot version mismatch — cross-version restore is a 075d migration concern"
        );
        let mut model = Self::new(snap.rows, snap.cols, snap.scrollback_capacity);

        // 1. Replay scrollback (oldest→newest) so the rows scroll off the top into the ring. Feeding
        //    the N rows joined by CRLF leaves the last (≤ rows) of them on the visible screen; the
        //    `rows` trailing line-feeds then scroll EXACTLY those off (cursor walks to the bottom,
        //    then each LF scrolls one content row) → exactly N rows in scrollback, screen blank. Skip
        //    entirely when empty (feeding line-feeds onto an empty screen would push BLANK rows in).
        if !snap.scrollback.is_empty() {
            for (i, row) in snap.scrollback.iter().enumerate() {
                if i > 0 {
                    model.parser.process(b"\r\n");
                }
                model.parser.process(row.as_bytes());
            }
            for _ in 0..snap.rows {
                model.parser.process(b"\r\n");
            }
        }

        // 2. If the snapshot was taken on the alternate screen, enter it before rendering its content.
        if snap.alternate_screen {
            model.parser.process(b"\x1b[?1049h");
        }

        // 3. Render the visible screen (state_formatted clears + redraws — scrollback untouched).
        model.parser.process(&snap.screen);

        model.refresh_scrollback_rows();
        model
    }

    /// Capture the ACTIVE grid's scrollback as plain rows, oldest→newest. Probes the real filled
    /// count off the active grid (`set_scrollback(MAX)` clamps to it — for the alt grid that is 0),
    /// then reads each row by scrolling it to the viewport top (`offset = n - i` puts scrollback row
    /// `i` at visible row 0). Restores the live view (offset 0). Uses the active-grid count, NOT the
    /// cached `scrollback_rows`, so it stays consistent with what is actually readable.
    fn capture_scrollback(&mut self) -> Vec<String> {
        let cols = self.size().1;
        self.parser.screen_mut().set_scrollback(usize::MAX);
        let n = self.parser.screen().scrollback();
        let mut rows = Vec::with_capacity(n);
        for i in 0..n {
            self.parser.screen_mut().set_scrollback(n - i);
            // `set_scrollback(n - i)` with `i < n` always places scrollback row `i` at visible row 0,
            // so `next()` is always `Some`; the empty-string fallback is a defensive non-panic floor.
            let row0 = self.parser.screen().rows(0, cols).next();
            debug_assert!(
                row0.is_some(),
                "scrollback row {i} must exist at offset {}",
                n - i
            );
            rows.push(row0.unwrap_or_default());
        }
        self.parser.screen_mut().set_scrollback(0);
        rows
    }
}

/// The snapshot format version — a header byte so 075d's persisted format is migratable.
const SNAPSHOT_VERSION: u8 = 1;

/// A serialized [`HeadlessVt`] — the `Replayed`-rung re-render payload (075b). **Daemon-internal**,
/// NOT a `shared/` contract: if it ever crosses the §6.4 Terminal-Channel wire it rides
/// `ServerFrame::TerminalOutput` as bytes (no new frame); 075d persists/redacts it to the survival
/// sidecar. The `version` header makes that persisted format migratable. Equality is byte-exact so
/// the idempotent round-trip (`snapshot(from_snapshot(s)) == s`) is a precise loss-proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtSnapshot {
    /// Format version (currently [`SNAPSHOT_VERSION`]); a 075d migration hook.
    version: u8,
    /// Screen dimensions at capture.
    rows: u16,
    cols: u16,
    /// The scrollback ring capacity (round-tripped so a restored model has the same bound).
    scrollback_capacity: usize,
    /// Whether the alternate screen was active at capture.
    alternate_screen: bool,
    /// Whether the active visible screen had any content at capture (075c — the
    /// [`has_restorable_content`](Self::has_restorable_content) signal; a non-blank screen is worth
    /// re-rendering even with zero scrollback rows, e.g. a mid-`vim` alt-screen session).
    screen_nonblank: bool,
    /// The visible screen as vt100 `state_formatted()` bytes (content + cursor + attrs + modes).
    screen: Vec<u8>,
    /// The active grid's scrollback as plain rows, oldest→newest.
    scrollback: Vec<String>,
}

impl VtSnapshot {
    /// Whether the alternate screen was active at capture.
    #[must_use]
    pub fn alternate_screen(&self) -> bool {
        self.alternate_screen
    }

    /// Whether this snapshot has anything worth replaying on restart — scrollback rows OR a non-blank
    /// visible screen. The 075c survival signal feeding `ResumeInputs.has_scrollback` → the §8.1
    /// `Replayed` rung (LESSONS §36): a mid-alt-screen session (zero scrollback but a re-renderable
    /// screen) STILL replays, while a truly-blank session falls through to `Relaunched`. Broader than
    /// raw [`scrollback_rows`](Self::scrollback_rows) — that is why the recovery consumer keys on this.
    #[must_use]
    pub fn has_restorable_content(&self) -> bool {
        !self.scrollback.is_empty() || self.screen_nonblank
    }

    /// The number of scrollback rows this snapshot carries. **0 for an alt-screen-active snapshot**:
    /// while the alt screen is active vt100 exposes only the alt grid, so the hidden normal buffer's
    /// scrollback is NOT retained (the named two-buffer edge — a deliberate `Replayed`-rung scope
    /// limit, pinned by `tests/vt.rs::test_vt_snapshot_alt_active_drops_hidden_scrollback`).
    #[must_use]
    pub fn scrollback_rows(&self) -> usize {
        self.scrollback.len()
    }
}

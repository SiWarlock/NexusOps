# /tdd brief — session_terminal_display

## Feature
Build the **Session Terminal display half** (the 6.3d terminal-well) — replace the
`terminal-well-pending` placeholder in `SessionTerminal.tsx` with a real **xterm.js**
terminal host that renders the daemon's frozen **`terminal_output`** stream
(`{terminal_id, seq, data:base64}`), driven by a **`MockGatewayPort` fixture stream** (the
established Phase-6 fixture pattern). **DISPLAY-ONLY by invariant #9** — the UI renders the
daemon's PTY output and **never scrapes it for machine state, and never sends keystroke
input**. The live transport stream + the inbound `{pause}`/`{resume}` flow-control +
watermark/backpressure are **P4** (the live drive loop) — explicitly deferred.

> **This is the LAST in-lane ui slice before a clean cross-track PAUSE** (user-ruled
> 2026-06-13: pause the ui track after the in-lane work; no cross-track packet now). It
> completes the 6.3d Session Terminal DISPLAY surface (the permission card already landed at
> 043/044). **Safety rule #9 is the load-bearing invariant** ("Never scrape the PTY for machine
> state — PTY is display-only; §9.1") → **`security-reviewer` REQUIRED** (invariant policy — a
> light pass: confirm no input→PTY path + no output-scraping-for-state).

## Use case + traceability
- **Task ID:** P6.3d
- **Architecture sections it implements:** `ARCHITECTURE.md §6.4` (the Terminal Channel + the
  `terminal_output` frame), `§9.1` (harness/PTY — **#9 PTY-display-only**), `§11` (the Session
  Terminal screen). Within Phase 6's `§11.1–§11.7` + the §6.4 Terminal-Channel contract the
  P3.4 boundary merge brought in (the same cross-phase contract-consumption widen as 040–044).
- **Widens phase scope because** the terminal-well is a CLIENT of the daemon's §6.4 Terminal
  Channel (a Phase-3/daemon contract a Phase-6 UI surface now consumes — unparked by the P3.4
  merge that made the §6.4 frame an ancestor of `track/ui`). *(The `§6`/`§9` tokens in prose are
  arch refs; lesson tokens are LESSONS refs.)*
- **Related context:** `SessionTerminal.tsx` (the placeholder well — `terminal-well-pending`;
  the header is real session data; the Pause button stays disabled — it's a session MUTATION,
  intent-seam/P4-gated); the frozen `ServerFrame` `terminal_output` variant + `TerminalProcessExited`
  (schema `nexusops-contract.schema.json`); `ui/src/gateway-client/{types.ts,mock.ts}` (the read
  surface + the fixture pattern — `subscribe` yields a fixture delta today); `ui/LESSONS.md 10`/`12`
  (the visual gate — xterm rendering is a TDD-exempt visual surface) + `8` (fixture/side-map pattern);
  the kit prototype `NexusOps-ui-kit/ui_kits/control-plane/` terminal surface to ground the visual gate.

## Safety design (#9 — PTY display-only) — each invariant → a PINNED TEST
> #9 is a Key safety rule (root `CLAUDE.md`): "Never scrape the PTY for machine state — derive
> status from SDK/app-server streams; PTY is display-only (§9.1)." The terminal-well ENFORCES it.
> `security-reviewer` verifies these pins.
1. **No PTY input path (#9 / display-only):** the terminal host has **NO** keystroke/`onData`→daemon
   path — the UI never sends input to the PTY (input is a P4 mutation surface, not this slice).
   **Pin `terminal_host_has_no_input_path`** — the component wires only an OUTPUT sink (frames →
   xterm), no input handler to the GatewayPort.
2. **No output-scraping-for-state (#9):** terminal status/state is **never** derived from the
   `data` bytes — the well renders bytes for display only; session status comes from the frozen
   `Session` projection (the existing `StatusPill`), never parsed from terminal output. **Pin
   `terminal_status_never_derived_from_output_bytes`** — the rendered status reads the session
   projection, not the stream.
3. **Frozen-frame fidelity:** the consumer decodes the frozen `terminal_output` frame
   (`base64 data` → bytes) and never invents transcript content (the placeholder's honest promise:
   "transcript lines are never invented"). **Pin `consumer_decodes_frozen_frame_no_invention`**
   (+ the `terminal_output` shadow drift-pinned to the frozen frame field-set).

## Acceptance criteria (what "done" means)
- [ ] **`TerminalOutputFrame` provisional shadow** (`contracts/`) drift-pinned to the frozen
      `ServerFrame.terminal_output` variant (`{frame_type:"terminal_output", terminal_id, seq:uint64≥0,
      data:base64}`) — the field-set drift-pin pattern (Lesson 2/14). (Plus `TerminalProcessExited` if
      the exit render needs it.)
- [ ] **GatewayPort gains a terminal READ surface** — `subscribe_terminal(terminal_id):
      AsyncIterable<TerminalOutputFrame>` (a UI-client display subscribe; a READ — NOT a mutation, so
      qualified to build provisionally, unlike the 043 mutation path). `MockGatewayPort` yields a
      **deterministic fixture terminal stream** (canned `terminal_output` frames). The real
      `UdsGatewayPort` demux of `ServerFrame.terminal_output` is a transport/P4 spread.
- [ ] **A pure terminal-stream consumer** (xterm-free, unit-testable): frames → base64-decode → byte
      chunks fed to a sink; **monotonic `seq`** assumed (the daemon closes the connection on lag →
      reconnect/re-subscribe is the recovery model — no client-side gap-fill; document it). Pinned by units.
- [ ] **`TerminalDisplay` (xterm.js) host** — mounts an xterm `Terminal` (+ `FitAddon`), the consumer's
      sink = `term.write(bytes)`. **Display-only #9** (pins above). `SessionTerminal.tsx` replaces the
      `terminal-well-pending` placeholder with `TerminalDisplay` when the session has a terminal stream;
      keeps the honest placeholder when there is none.
- [ ] **`TerminalProcessExited`** → an honest exit render (exit_code/signal), never a faked "still running".
- [ ] **`@xterm/xterm`** (+ `@xterm/addon-fit`) added to `ui/package.json` (the manifest change the slice owns).
- [ ] **All #9 safety pins green** (Safety design above). **`security-reviewer` PASS** (invariant policy).
- [ ] **VISUAL gate** (Lesson 10/12): the xterm well rendered (dev server, fixture stream) vs the kit
      prototype terminal surface — green tests do NOT verify xterm renders. Confirm at Step 9.
- [ ] Whole suite green; `/preflight` clean. Cross-doc invariant flagged at Step 9 (the GatewayPort
      terminal read surface + the `TerminalOutputFrame` shadow; the P4 deferrals).

## Wiring / entry point (Step 7.5)
**REAL entry:** the Session Terminal view is reachable in the Shell (the sidebar workspace tree →
a session → `contentView` Session Terminal; `SessionTerminal.tsx`). The new path is **session →
`TerminalDisplay` → `subscribe_terminal` (Mock fixture stream) → xterm render**. `/wired` the
terminal-well. The `MockGatewayPort` is the §14 test/dev seam; the real `UdsGatewayPort` terminal
demux is the transport/P4 slice.

## Files expected to touch
**New:**
- `ui/src/views/terminal/TerminalDisplay.tsx` (the xterm host) + its test.
- `ui/src/views/terminal/terminal-stream.ts` (the pure consumer: frames → decoded bytes → sink) + its test.
- `ui/src/contracts/` terminal-frame shadow (or extend an existing contracts file) + drift-pin test.
- a `MockGatewayPort` terminal fixture (`projections/fixtures/` or a terminal fixture module).

**Modified:**
- `ui/src/gateway-client/types.ts` (+ `subscribe_terminal`) + `mock.ts` (the fixture stream).
- `ui/src/views/terminal/SessionTerminal.tsx` (placeholder → `TerminalDisplay`).
- `ui/package.json` (xterm deps).

If implementation needs files beyond this, flag at Step 2.5.

## Things to flag at Step 2.5
1. **The consumer/host split (determinism).** xterm.js renders to canvas/DOM (NOT deterministically
   unit-testable in jsdom) — so isolate the **pure consumer** (decode/seq/sink, TDD) from the **xterm
   host** (a thin adapter, visual-gated + a structural "sink wired, no input handler" test, xterm mocked).
   **Default vote:** pure consumer is test-first; the xterm mount is visual-gate + a structural #9 test.
   Confirm the split + how you mock xterm in the host test.
2. **The terminal READ surface shape.** `subscribe_terminal(terminal_id): AsyncIterable<TerminalOutputFrame>`
   on GatewayPort (mirrors `subscribe`). **Default vote: yes** (a display READ; the Mock yields a fixture
   stream; the real demux is P4/transport). Confirm vs an alternative (consuming `ServerFrame.terminal_output`
   through the existing `subscribe` — messier, it's typed to `ProjectionDelta`).
3. **Commit layering.** Default **2 commits** (Lesson 7): (L1) the `TerminalOutputFrame` shadow + the pure
   consumer + the GatewayPort `subscribe_terminal` + the Mock fixture (the TDD core); (L2) the xterm
   `TerminalDisplay` + `SessionTerminal` integration + the xterm dep + the visual gate. **L2 carries the #9
   `security-reviewer`** (the host is where an input path would be added). Confirm or collapse.

## Dependencies + sequencing
- **Depends on:** the frozen §6.4 `terminal_output` frame (0.23.0, P3.4-merged) + xterm.js (new dep).
  Nothing else; fully in-lane.
- **Blocks:** nothing in-lane. The **live transport terminal stream** (`UdsGatewayPort` demux of
  `ServerFrame.terminal_output`) + the **inbound `{pause}`/`{resume}` flow-control** + watermark/backpressure
  are **P4** (cross-track spread; the well swaps the fixture for the live stream there).

## Deferred (explicitly OUT — P4 / cross-track, captured)
- **The live transport terminal stream** (the real `UdsGatewayPort` `ServerFrame.terminal_output` demux;
  replaces the fixture). `last-consumer-slice: the UdsGatewayPort terminal transport slice (P4)`.
- **Inbound `{pause}`/`{resume}` flow-control** (a session MUTATION → the intent seam; the Pause button
  stays disabled) + **watermark/backpressure** (the §6.4 pump / 30fps batch). `last-consumer-slice: P4`.
- **Scrollback/headless-VT fidelity** (the separate follow-on brief). `last-consumer-slice: a terminal-VT brief`.

## Estimated commit count
**2** (Lesson 7 layers): (L1) the contract shadow + pure consumer + GatewayPort `subscribe_terminal` + Mock
fixture (TDD core); (L2) the xterm `TerminalDisplay` + `SessionTerminal` integration + the xterm dep + the
visual gate. **L2 carries the #9 `security-reviewer`** (invariant policy). The orchestrator drives L1→L2.

## Lessons-logged candidates anticipated
- **Convention candidate** — the terminal-well display pattern: a pure frame-consumer (frozen
  `terminal_output` decode/seq/sink) separated from the xterm.js host (visual-gated); **display-only #9**
  (no input path, no output-scraping-for-state); the real transport stream swaps the fixture at P4.
- **Future TODO — next-brief working set / cross-track** — the P4 live terminal transport + inbound
  pause/resume + backpressure; the headless-VT/scrollback follow-on. (These join the PAUSE-handoff unblock list.)

## How to invoke
1. **Read this brief end-to-end** — especially the **Safety design (#9)** pins + "Things to flag at Step 2.5".
2. Pre-flight: confirm `track/ui` in the `NexusOps-ui` worktree, `cd ui`; `pnpm install` after adding xterm.
3. **Run `/tdd session_terminal_display`.**
4. Step 0 (Restate) + Step 1 (files).
5. **Step 2.5** — answer the 3 design questions + the coverage map (each acceptance/safety bullet → its test);
   send the write-up; wait for `APPROVED.`/`TWEAK:`/`ADD:` before GREEN.
6. Drive L1 (consumer + contract) → commit; then L2 (xterm host + integration).
7. **Step 8** — `security-reviewer` (invariant policy — the #9 surface) + `code-quality-reviewer` (every-slice).
8. **Step 7.5** — `/wired` the terminal-well (real Shell→session→TerminalDisplay path).
9. **VISUAL gate** before the L2 commit (dev server vs the kit prototype terminal surface).
10. Step 9 — cross-doc flags (the GatewayPort terminal read surface + the `TerminalOutputFrame` shadow) + the
    P4 deferrals (for the PAUSE handoff) + confirm #9 pins green + visual gate PASS.

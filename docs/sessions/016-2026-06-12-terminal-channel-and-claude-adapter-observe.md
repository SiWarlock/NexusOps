# Session 016 — P3.4 Terminal Channel + P3.2-part-1 Claude adapter (observe path)

- **Date:** 2026-06-12
- **Phase:** Phase 3 (harness adapters & embedded terminal) — two slices: **3.4** (the §6.4 Terminal Channel + PTY host + backpressure) then **3.2 part 1** (the Claude `HarnessAdapter` observe path). PTY-primary per the resolved cat-4.
- **Predecessor:** [015 — §9.1 HarnessAdapter contract freeze + proj_usage_ledger projector](015-2026-06-12-harness-adapter-contract-and-usage-ledger.md)
- **Successor:** [017 — Claude MutationIntercept→Gateway interception (P3.2 part 2, brief 043)](017-2026-06-12-claude-mutation-intercept-gateway.md)

## Why this session existed

Phase 3.1 froze the `HarnessAdapter` contract. This round implemented against it, in two sequenced slices:

1. **3.4 (the ungated ui-track unblocker)** — the §6.4 Terminal Channel data plane: freeze the terminal wire frames, build the daemon PTY host, wire app-level backpressure. ui 6.3d's xterm.js host consumes the frozen contract at its resume.
2. **3.2 part 1 (Claude adapter, observe path)** — the first real harness adapter: an O-13-compliant `launch()`, a pure `stream_status` derivation, `read_transcript`. PTY-primary (cat-4 resolved = Branch A). The safety-critical interception (INV-SEC-1) + telemetry are deliberately a separate brief (043).

## What was built

### Files created
- `daemon/src/terminal/mod.rs` — the §6.4 Terminal Channel data plane: the `Pty` trait + `PortablePtyHost` (portable-pty) + `FakePty` (§14 seam) + `TerminalSession` (read-accumulate pump + `TerminalProcessExited` emission via an injected `TerminalEventSink`) + `TerminalId` + the `next_terminal_action` watermark classifier (LESSON §12) + `TerminalEmit` + (P3.2) the `PtySpawner`/`PortablePtySpawner` object-safe factory seam.
- `daemon/tests/terminal.rs` — FakePty-driven integration tests (spawn→stream, write→child, exit→one event, signal-kill, no-status-API structural pin, backpressure flood/batch/no-writer-backpressure, EOF-flush).
- `daemon/src/harness/claude/mod.rs` — `ClaudeAdapter` + `ClaudeLaunchSpec` (the O-13 enforcement surface) + `ClaudeSettings` (generated per-session hooks+statusLine) + `CLAUDE_CAPABILITIES` + `read_transcript`/`project_slug`.
- `daemon/src/harness/claude/status.rs` — the pure `derive_status(prev, ClaudeSignal) -> Session` + `ClaudeSignal` + `NotificationKind` (the §5.1 derivation; #9 grep-pin scoped here).
- `daemon/tests/claude_adapter.rs` — launch O-13 pin · spawn→Starting / spawn-fail→Failed · the `derive_status` mapping/exit/terminal-sink/#9-structural tests · transcript locate · observe-path stubs · the §14 conformance fixture.

### Files modified
- `shared/src/{ipc,events,schema,lib}.rs` + `shared/contracts/schema/nexusops-contract.schema.json` (regen) — the §6.4 Terminal Channel freeze: `TerminalOutputFrame`/`TerminalInputFrame`/`TerminalControlFrame` + `TerminalControlKind` + `ServerFrame::TerminalOutput` (the reserved slot filled) + the `TerminalProcessExited` event; **CONTRACT 0.20.0 → 0.21.0**.
- `shared/tests/contract.rs` + `shared/tests/envelope.rs` — the §2.5-seam snapshot tests; version-pin consolidated to ONE current equality pin + a monotonic floor.
- `daemon/src/harness/mod.rs` — the `HarnessAdapter` trait reshape (`Send+Sync`→`Send`, `launch(&self)`→`launch(&mut self)`); `pub mod claude`.
- `daemon/src/idgen.rs` — `new_terminal_id()` + a separate `terminal_counter` (mirrors `new_outbox_id`/`outbox_counter`).
- `daemon/src/lib.rs` — `pub mod terminal`.
- `daemon/Cargo.toml` — `base64`, `portable-pty`.
- `daemon/tests/{ipc,runtime}.rs` — `ServerFrame::TerminalOutput` match arms.

## Decisions made
- **§6.4 encoding = JSON-base64 MVP** (raw PTY bytes base64 over the unchanged 4-byte-len+JSON codec, LESSON §7); the binary fast-path is additively deferred to 3.5-with-throughput. (Lead-ratified.)
- **`terminal_id` = a daemon runtime handle** (`String` wire / typed newtype over a ULID via the §14 IdGen seam), NOT a 23rd LOCKED-22 `IdKind` (the `subscription_id` precedent). (Lead-ratified.)
- **cat-4 = PTY-primary** applied: the Claude `launch()` is an interactive PTY via `PortablePtyHost`, `default`-mode-only, no `-p`, no bg subagents, hooks+statusLine wired — the spec IS the safety-#10 enforcement surface. The SDK is a secondary signal, not the drive transport.
- **`TerminalProcessExited` = a non-mutation OBSERVATION event** (write-actor via the injected sink, NOT the Gateway — LESSON §10/§23 family).
- **`HarnessAdapter` trait reshape** (daemon-internal/UNFROZEN, LESSON §23): `Send+Sync`→`Send` + `launch(&mut self)` — the real adapter owns a `Box<dyn Pty>` (Send-only) + a mutable status machine, so it cannot be `Sync`/`&self`. security-reviewer confirmed SAFE.
- **`PtySpawner` factory seam** (`terminal/mod.rs`): the object-safe `spawn(...) -> Box<dyn Pty>` factory a harness `launch()` injects (`PortablePtyHost::spawn` returns `Self`, not object-safe). (Q7, lead-approved.)
- **`derive_status` = a pure `(prev, ClaudeSignal) -> Session` fold** over STRUCTURED signals (hook events + the 3.4 exit), never PTY bytes (#9). R-9 terminal-sink guard (no resurrection); fail-closed exit (`(0,None)`→Completed, else→Failed). Q2 mapping confirmed; `RunningTests`/`Thinking`/`ChangesReady`/`WaitingOnExternalService` deferred (no reliable signal); `Creating`/`Stale`/`Archived`/`Killed` are daemon overlays, not adapter-derived.
- **Q4 — never the user's `~/.claude/settings.json`:** a generated per-session settings file (`--settings`, 0600), fail-closed on serialize (→ launch=Failed, no hook-less session).
- **P3.4 finish-ordering robustness fix (folded into P3.2 L1's terminal touch):** EOF flushes trailing `pending` into a final frame BEFORE the exit event (the split read_step/flush path otherwise dropped bytes).

## Decisions explicitly NOT made (deferred)
- **Interception (Claude `MutationIntercept`→Gateway, INV-SEC-1) + telemetry emission** → **brief 043** (the cat-1 safety-design slice; the daemon hook-receiver ingress lives there).
- **`resume()` (survival) + the live session-lifecycle drive caller** → **Phase 4** (§8/§17).
- **The settings-file hardening** (daemon app-support dir + `O_EXCL` against the predictable temp name + per-session auth) → 043 (0600 added now as defense-in-depth; the L1 settings carry no secret yet — only the receiver path).
- **The binary terminal encoding + watermark/tick tuning + the partial-flush hysteresis** → 3.5 (with throughput data).
- **The VT/scrollback alt-screen golden-corpus fidelity** → a follow-on brief.
- **`RunningTests`/transcript-marker states** → a status-refinement follow-on (once the JSONL parse lands).

## TDD compliance
**Clean — no violations.** Every layer of both slices ran RED → Step-2.5 (orchestrator-approved) → GREEN, with RED confirmed for the right reason before each GREEN. Review-fix tests (the P3.4 finish-fix, the spawn-fail / unknown-exit / dot-slug / reject-unknown additions) landed alongside their fixes. The non-deterministic surfaces (the real `claude` spawn, the live PTY child) are covered via `FakePty` / the recording spawner per the §14 path; the `not-tested-because` items (live spawn, live hook-receiver/transcript I/O, the production drive caller) are named.

## Reachability
- **§6.4 Terminal Channel contract (L1):** reachable via `ContractBundle` (schema gen) + the `ServerFrame` mux + the snapshot tests; the ui xterm.js consumer lands cross-track at 6.3d.
- **Terminal data plane (`TerminalSession`/`PortablePtyHost`/backpressure):** `/wired` = NO production caller — built + FakePty-tested; the `attach_terminal`→host binding + the `spawn_blocking` drive loop are the named **3.2/3.3 + P4** wiring.
- **Claude adapter (`ClaudeAdapter`):** `/wired` = NO production caller — built + fixture/FakePty-tested; the **P4** session-lifecycle drive loop (feeds live signals into `push_signal`, binds the write-actor sink) + the **043** hook-receiver ingress are the named wiring. `ClaudeAdapter::launch()` is the first real `Pty`-spawner (via `PortablePtySpawner`), partially closing the 3.4 `PortablePtyHost` gap.

## Open follow-ups
- **Cross-doc routes (held for the orchestrator's `/orchestrate-end` seal — flagged + ACK'd at each Step 9):** CONTRACT 0.20.0→0.21.0 + the terminal frames + `TerminalProcessExited` → `ARCHITECTURE.md` Appendix A (GatewayPort/§6.4 + EventTypeRegistry) + the §6.4 "reserved → AS-BUILT JSON-base64 variant" prose + `daemon/CLAUDE.md` rows; the `HarnessAdapter` trait reshape (Send+Sync→Send, launch &mut self) → the §9.1 cross-doc row; the §9.1 AS-BUILT notes (Claude adapter = PTY-primary observe path; the pure-`(prev,signal)→state` derivation; the transcript best-effort-slug/043-authoritative + honest-empty-hash); the **convention/LESSON candidates** (`PtySpawner` object-safe factory · the pure-status-fold · the #9 no-status-API grep-pin).
- **Future TODO → 043:** the live hook-receiver ingress + the settings-file hardening + the `MutationIntercept`→Gateway routing + telemetry emission (the 3.1 emission pins: token DELTAS-not-cumulative · UTC-Z `occurred_at`).
- **Future TODO → P4:** the session-lifecycle drive loop (the production caller of the terminal host + the Claude adapter; `resume()`); the §17 PTY-death cascade (consumes `TerminalProcessExited`); the inbound client→daemon `{pause}`/`{resume}` handling (6.3d).
- **Future TODO → 3.5:** the terminal-attach benchmark + the binary encoding + watermark tuning + partial-flush hysteresis.
- **Minor (deferred lows):** the #9 grep-pin token-set extension when the real PTY reader API lands (043/P4); the `HOME`-unset→None transcript path untested (correct + commented).

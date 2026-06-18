# Session 031 — VT-arc tie-off + build-hygiene (075e)

- **Date:** 2026-06-18
- **Phase:** 3.4 (Embedded terminal) tie-off, bridging Phase 4 §8.1/§17 (the survival/recovery ladder). Task **P3.4-VT** (same family as 075a–d).
- **Predecessor:** [030 — the headless-VT / scrollback survival arc (075a–d)](030-2026-06-17-headless-vt-scrollback-arc-075abcd.md)
- **Successor:** _(next implementer session)_

## Why this session existed

The 075a–d headless-VT/scrollback arc (session 030, sealed `ffbabad`) left tracked debt + a recurring build irritant. This single low-risk slice (075e, brief `075e-P3.4-VT-arc-tieoff-and-build-hygiene.md`) tied them off as one bundle — **NO contract change, NO schema-snapshot, NO `CONTRACT_VERSION` bump** (held 0.38.0):

1. The recovery/resume **input field `has_scrollback` lied** — 075c broadened its semantics from "scrollback present" to `has_restorable_content()` (scrollback rows **OR** a non-blank restorable screen), but the name stayed.
2. The **075c producer tap** still base64-**decoded** each §6.4 wire frame back to raw bytes to feed the headless VT — a redundant per-frame encode→decode round-trip (075c residual).
3. The **075c LOW-2 `save_tick` cadence test** was deferred (the periodic §17 crash-survival checkpoint had no deterministic pin).
4. **No `rust-toolchain.toml`** → `cargo fmt --check` was non-deterministic across rustfmt versions (the `ca5d789` fmt-drift root cause).

## What was built

All in **one commit `f8ed60d`** (`refactor(terminal,session): VT-arc tie-off …`; Q4 collapse-to-1 — `actor.rs` carries both the hygiene `pub const` and the raw-tap, so a clean file-split into 2 commits wasn't possible; raw-tap was trivial, the pre-approved collapse condition).

### Files created
- `rust-toolchain.toml` (repo root) — pins the **exact** channel `1.93.0` + `components = ["rustfmt", "clippy"]`. Exact, not floating `stable` (a floating channel still drifts rustfmt). No-op against the current toolchain (team already on 1.93.0 / rustfmt 1.8.0-stable).

### Files modified
- `daemon/src/harness/resume.rs` — rename `ResumeInputs.has_scrollback` → `has_replayable_snapshot` (field + the `decide_resume` body + docstrings).
- `daemon/src/session/recovery.rs` — rename `RecoverableSession.has_scrollback` → `has_replayable_snapshot` (field + the `recover_sessions_on_restart` build site + doc).
- `daemon/src/runtime/recovery.rs` — rename the feeders (the `enumerate_recoverable_sessions` local binding + struct-init) + docstrings.
- `daemon/src/terminal/vt.rs` — **1 prose reference** to the renamed input field updated; the VT primitive `HeadlessVt::has_scrollback()` (`scrollback_rows>0`) **left unchanged** — it feeds the input, it is not the broadened survival signal (the load-bearing Q1 boundary).
- `daemon/src/terminal/mod.rs` — `TerminalEmit::Output(TerminalOutputFrame)` → struct variant `Output { frame, raw: Vec<u8> }`; `drain_pending` builds both. The §6.4 wire type `TerminalOutputFrame` is **unchanged**.
- `daemon/src/session/actor.rs` — the raw-tap: the producer pump feeds the headless VT from `raw` directly (dropped the per-frame `base64::decode` + its silent `if let Ok` drop branch + the now-unused `base64`/`STANDARD` imports); `pub SCROLLBACK_SAVE_INTERVAL` (so the cadence test advances by the canonical const); updated the tap comment.
- `daemon/src/terminal/scrollback_store.rs` — `FakeScrollbackStore::save_calls()` (an `AtomicUsize` call-counter, `test-support`-gated) — `saved_count()` counts distinct sessions (stays 1 for N saves on one session), so a call-counter is required to measure cadence.
- `daemon/benches/terminal_attach.rs` — mechanical `TerminalEmit::Output` pattern update (`(_)` → `{ .. }`) + rustfmt reflow.
- `daemon/tests/{resume,recovery_restart,recovery_restart_wiring,tmux_broker,durable_scrollback,scrollback_recovery}.rs` — follow the input-field rename (a guarded token-replace; `vt.rs` tests left untouched — the VT primitive).
- `daemon/tests/session.rs` — **NEW** `test_save_tick_periodic_checkpoint` (+ the `drive_until` bounded-drain helper + `MAX_DRAIN_YIELDS`).
- `daemon/tests/terminal.rs` — **NEW** `test_output_emit_carries_raw_matching_the_wire_frame`; the `outputs_of` helper follows the enum shape.

### Tests added (2 integration, deterministic)
- `test_save_tick_periodic_checkpoint` — under PAUSED time (`#[tokio::test(start_paused = true)]`), advancing `N × SCROLLBACK_SAVE_INTERVAL` drives **EXACTLY N** periodic survival checkpoints, isolated from the interval's immediate t=0 save (`baseline == 1`, pinned) and the reap save (post-`join`, uncounted). Deterministic, no wall-clock: a bounded `drive_until` yields until each interval's save lands (the 5ms status ticker shares the loop → a ready save tick may need several turns; we yield until it lands rather than guess a count).
- `test_output_emit_carries_raw_matching_the_wire_frame` — pins `raw == base64::decode(frame.data)` (the raw-tap is byte-identical to the unchanged base64 wire frame).

## Decisions made
- **Q1 rename boundary = input field only.** `ResumeInputs`/`RecoverableSession.has_scrollback` → `has_replayable_snapshot`; `HeadlessVt::has_scrollback()` left unchanged (renaming the VT primitive would be semantically wrong — it literally counts scrollback rows). The compiler is the guard: a struct-field rename cannot touch a method symbol.
- **Q2 raw-tap seam = carry `raw` on the daemon-internal `TerminalEmit::Output`.** Keeps `TerminalSession` a pure byte-pipe (no VT coupling — `raw` rides the emit, not the struct); the wire type stays base64. Per the orchestrator's refinement, the redundant decode **and** its silent decode-error drop branch were both removed (our own encoder always emits valid base64; the raw bytes never needed the round-trip).
- **Q3 toolchain = exact `1.93.0` pin** (not floating `stable`) + rustfmt/clippy components.
- **Q4 commits = collapse to 1** (`actor.rs` overlap forced it; raw-tap trivial — pre-approved).
- **2 code-quality mediums fixed in-slice:** the `baseline == 1` pin on the cadence test (prevents a wrong-reason pass if a stray t=0 save sneaks in) + a stale `"no scrollback"` → `"no replayable snapshot"` assertion message. **1 low fixed:** the magic `1_000_000` drain cap → `MAX_DRAIN_YIELDS`.

## Decisions explicitly NOT made (deferred)
- **The two-buffer / B cell-level-formatted-redaction fidelity edge** — explicitly EXCLUDED by the brief; its own later §15-adjacent design slice.
- **2 deferred code-quality lows** (handed to the orchestrator's Carry-forward): (1) a one-word `vt.rs:13` module-doc clarification (the `HeadlessVt::has_scrollback()` **method** vs the `has_replayable_snapshot` recovery **field**); (2) `FakeScrollbackStore::save_calls` `SeqCst` → `Relaxed` (a test-support micro-cleanup — `SeqCst` is conservative-correct, just heavier than needed).
- **The "rename-promptly" convention candidate** — the orchestrator declined it for the LESSONS index as too minor.

## TDD compliance
**Clean.** The 2 genuinely-new tests were written FIRST and confirmed RED for the right reason (private const + missing `save_calls()`; the `Output` variant still a tuple) before GREEN. The rename + raw-tap + toolchain pin are green-stays-green refactors (covered by the existing recovery/resume/producer-tap suites + `/preflight` fmt-check). No TDD violation; nothing safety-critical skipped.

## Cross-doc invariant audit
**No cross-doc invariant change.** The renamed types (`ResumeInputs`, `RecoverableSession`) and the changed enum (`TerminalEmit`) are **daemon-internal** — none appears in the `daemon/CLAUDE.md` "Cross-doc invariants" table, and none derives `Serialize`/`JsonSchema` (grep-confirmed). `shared/tests/contract.rs` schema-snapshot stayed GREEN (no `shared/` type moved). `git diff` over `ARCHITECTURE.md` / `daemon/CLAUDE.md` / `daemon/LESSONS.md` is empty — no doc edit owed. Flagged "Cross-doc invariant change: NONE" at Step 9; the orchestrator confirmed no hot doc-write.

## Reachability
No new production entry point — all three code items refactor existing wired paths (Step 7.5):
- **Rename** flows on the live restart-recovery path: `run_restart_recovery` (`main.rs`, post-supervisor) → `enumerate_recoverable_sessions` → `decide_resume` → the `Replayed` rung. Pure rename; reachability unchanged.
- **Raw-tap** is on the live `SessionActor` read-pump (reachable from `spawn_session_actor` ← `SessionSupervisor` ← `session.create`). Pure refactor.
- **`save_tick`** is already wired in the actor `select!` loop; this slice only added its deferred test.
No tested-but-unwired gaps. (`save_calls()` + `pub SCROLLBACK_SAVE_INTERVAL` are test-support only.)

## Open follow-ups
- **Step-9 categorized (routed hot; orchestrator owns):**
  - Future TODO (operational): the raw-tap removed a per-frame base64 decode **and** the silent decode-error drop branch on the hot pump path (modest throughput; the real win is killing the silent-drop).
  - Future TODO (deferred code-quality lows): `vt.rs:13` doc clarification · `save_calls` `SeqCst`→`Relaxed` (both in the orchestrator's Carry-forward).
  - Convention candidate "broaden a field's semantics → rename it in the SAME arc's tie-off" — **declined** for the LESSONS index (too minor).
- **The two-buffer fidelity edge** stays the tracked 3.4-VT future-§15 design slice (EXCLUDED here).
- **5.3a** is HELD (orchestrator parked it at a §15 persistence-pattern fork pending the user's steer); the team idles at a clean STABLE main boundary for the user's main→ui merge, resuming 5.3a after.

## Gate
Suite **895/0** (893 baseline + 2 new); clippy `-D warnings` clean; `cargo fmt --check` clean (the toolchain pin is a no-op against the current toolchain). security-reviewer **CLEAR** every dimension (#9 display-only, §15 redaction-downstream, cat-1 byte-pipe purity, INV-SEC-1, no-exfil byte-identity) — no Step-9 Finding.

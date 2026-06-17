# Headless-VT / scrollback — the load-bearing terminal-emulation approach (Option A/B/C)

> **Status:** ✅ **RULED A (`vt100`)** (lead, away-dial, 2026-06-17; away-log Decision 20). Trust-core posture = minimize bespoke VT complexity; A gives fidelity + clean serialize/replay for-free with a focused pure-Rust, cargo-audited dep. **Condition:** verify `vt100`'s serialize/scrollback API meets the `Replayed`-rung snapshot/restore + alt-screen fidelity AT AUTHORING — if it can't serialize cleanly, **B is the sanctioned fallback (NOT C)** → re-surface. C rejected (heavy/opaque/hardest-to-serialize). Reversible (daemon-internal, §6.4 wire frozen → no new contract). **+ PROACTIVE WHOLESALE CYCLE first** (clean Codex-arc round boundary + a large VT arc → fresh headroom; orch DECOMPOSES into bounded sub-slices, e.g. 075a/b/c). ⚠️ return-review: the trust-core dep choice (`vt100`).
> **Slice:** the named 3.4 follow-on — *"headless-VT state + scrollback serialize = a follow-on brief"* (`IMPLEMENTATION_PLAN.md` 3.4 line 459/463; Phase-4 acceptance line 474: *"Terminal backpressure + scrollback fidelity tested with FakePty/recorded corpus"*).

## Where we are
3.3c (`eb358e1`) + 3.3d (`389ee28`+`1f6664c`) shipped clean — **the deterministic Codex arc (3.3a/b/c/d) is COMPLETE** (only the live-Codex drive loop remains, and it's HITL). Next deterministic-queue item per your sequence = the headless-VT/scrollback brief. Per-slice context: last two snapshots `OK 44%` → `OK 60%` (climbing; run `/context-check nexusops-daemon` for the live trajectory).

## What the slice IS
The daemon's `terminal/mod.rs` spine (3.4, DONE) streams PTY bytes to the UI but keeps **no headless screen/scrollback model**. The survival path needs one: `decide_resume`'s **`Replayed`** rung (daemon-restart, no live survivor, no resume handle → "scrollback replay + relaunch", §8/§17, O-2 *"accurate alt-screen VT re-render"*) requires the daemon to maintain a **headless VT screen + scrollback** from the byte stream, **serialize** it, and **replay/re-render** it with fidelity — tested vs a **golden corpus** (FakePty). It is **daemon-internal**: the §6.4 Terminal-Channel wire is already frozen (CONTRACT 0.21.0); this does NOT add a new contract surface (confirm at authoring). Deterministic + TDD-able (a VT parser is pure bytes→state; serialize/replay fidelity pins vs recorded corpora).

## The load-bearing choice — the terminal-emulation dependency/approach
A trust-core daemon dependency addition; the choice shapes fidelity, the serialize format, dep weight, and maintenance.

| | **A — `vt100` (purpose-built headless screen-state)** | **B — `vte` parser-only + custom grid/scrollback** | **C — `alacritty_terminal` (full emulator)** |
|---|---|---|---|
| What | A crate built exactly for "maintain terminal screen + scrollback headlessly"; gives a `Screen` of cells + scrollback + a diff surface | Alacritty's VT escape-sequence parser ONLY; we build the grid + scrollback model + serialize ourselves | Alacritty's full `Term`/`Grid` model (scrollback, modes, all state) |
| Dep weight | Focused, single-purpose | **Minimal** (just the parser) | **Heavy** (a large crate + its data model) |
| Fidelity | High, for-free (purpose-built) | As good as our model (more code = more edge-case risk) | Highest (battle-tested) but coupled to its model |
| Serialize/replay | Clean (a screen/scrollback snapshot is the design point) | **Full control** of a compact, versionable format | Hardest (serialize a foreign rich model) |
| Code/test surface | Low-medium | **Highest** (we own the grid + all VT edge cases) | Low (but opaque) |
| Trust-core fit | Good (focused, pure-Rust) | Best on dep-minimalism; worst on code surface | Worst on dep weight |

All three are pure-Rust, widely used; cargo-audit at the phase gate either way. (Exact crate API/maintenance/serialize support verified at authoring, the 3.3c-grammar pattern — the choice here is the APPROACH.)

## Recommendation
**Option A (`vt100`)** — purpose-built for exactly this (headless screen + scrollback state with a clean serialize/diff surface), so we get fidelity for-free with a focused, single-purpose dependency and the least bespoke VT edge-case risk. **B (`vte` + custom)** is the close runner-up if you want **absolute minimal dependency + full serialize-format control** for the trust core (at the cost of owning the grid + all VT edge cases ourselves — more code + more test surface). **C is not recommended** (heavy dep, opaque foreign model, hardest to serialize cleanly). I lean **A**; **B** is defensible on dep-minimalism grounds.

## Sequencing note
Headless-VT is a **large** slice (a VT emulator + scrollback serialize + a golden-corpus replay-fidelity harness) and the implementer's context is climbing. If you'd prefer a **fresh context** for it, a cycle before this slice is reasonable; otherwise I author brief 075 on your A/B/C steer and dispatch. Your call on (1) the approach and (2) proceed-now vs cycle-first.

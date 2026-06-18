# Session 030 — the headless-VT / scrollback survival arc (075a–d)

- **Date:** 2026-06-17
- **Phase:** 3.4 (Embedded terminal) → bridging into Phase 4 §8/§8.1/§17 (survival ladder). Task **P3.4-VT**.
- **Predecessor:** [029 — Codex arc 3.3c interception + 3.3d telemetry](029-2026-06-17-codex-arc-3.3cd-interception-and-telemetry.md)
- **Successor:** [031 — VT-arc tie-off + build-hygiene (075e)](031-2026-06-18-vt-arc-tieoff-and-build-hygiene.md)

## Why this session existed

The 3.4 terminal spine (daemon `TerminalSession`/`FakePty`, sealed `18b10e9`/`41afa80`/`9a89b1a`) left a named follow-on: the **headless-VT/scrollback-fidelity** mechanism that the §8/§17 survival `Replayed` rung needs. A session whose daemon restarts must re-render its terminal screen + scrollback — but nothing built the in-memory screen model, serialized it, fed it from the live pump, or persisted it (§15-redacted). This session built that end-to-end as a decomposed 4-slice arc (075a→d), taking **`Replayed`-after-daemon-restart from unreachable → LIVE in production**.

## What was built

The arc, slice by slice (each a `/tdd` cycle, Step-2.5 reviewed, both reviewers run, committed on `main`):

| Slice | Commit | What |
|---|---|---|
| 075a | `733ab31` | `HeadlessVt` screen+scrollback model (wraps `vt100::Parser`); display-only #9 |
| 075b | `2232463` | `VtSnapshot` serialize (`snapshot`) + restore (`from_snapshot`) + replay fidelity |
| 075c | `ca227be` | producer tap (`SessionActor` pump → `HeadlessVt` → save) + `ScrollbackStore` seam + recovery wiring (`decide_resume`→`Replayed` reachable) |
| 075d/1 | `61527fa` | durable §15-redacted `FileScrollbackStore` (safety core) — **`Replayed`-after-restart LIVE** |
| 075d/2 | `9dabc1a` | retention lifecycle — `evict` + reap-wiring + startup orphan-sweep + size/age backstop |

### Files created
- `daemon/src/terminal/vt.rs` (075a) — `HeadlessVt` (bytes→screen+scrollback fold; `process`/`screen_contents`/`view_at_scrollback`/`has_scrollback`/`scrollback_len`/`alternate_screen`/`size`/`resize`); + (075b) `VtSnapshot` + `snapshot`/`from_snapshot`; + (075d) `from_plain`/`scrollback_text`/`DEFAULT_SCROLLBACK_CAPACITY`.
- `daemon/src/terminal/scrollback_store.rs` (075c) — the `ScrollbackStore` trait + `NoopScrollbackStore` (prod placeholder) + `FakeScrollbackStore` (tests); + (075d) `evict`.
- `daemon/src/scrollback/mod.rs` (075d) — `FileScrollbackStore` + `PersistedScrollback` (the only-`Serialize` post-redaction on-disk type) + `save`/`load`/`evict`/`sweep_orphans`/`enforce_backstop`.
- `daemon/tests/vt.rs` (075a, extended 075b/d) — 19 golden-fixture tests.
- `daemon/tests/scrollback_recovery.rs` (075c) — seam + recovery-consumer tests.
- `daemon/tests/durable_scrollback.rs` (075d) — 11 §15 + lifecycle tests.

### Files modified
- `daemon/src/terminal/mod.rs` — `mod vt`/`mod scrollback_store` + re-exports.
- `daemon/src/session/actor.rs` (075c) — the read-pump producer tap (`Arc<Mutex<HeadlessVt>>`, base64-decode, save tick + post-`pump.await` reap save); (075d) the shared capacity const.
- `daemon/src/session/mod.rs` (075c) — supervisor store-threading; (075d) reap-`evict` wiring.
- `daemon/src/runtime/recovery.rs` (075c) — `enumerate_recoverable_sessions`/`run_restart_recovery` take `&dyn ScrollbackStore` → feed `has_scrollback`/`replayed_event_count`.
- `daemon/src/main.rs` — inject the placeholder (075c) → the real `FileScrollbackStore` (075d) + startup sweep/backstop + `read_known_session_ids`.
- `daemon/Cargo.toml`/`Cargo.lock` (075a) — `vt100 = "0.16"` (→0.16.2, pure-Rust).
- `daemon/src/lib.rs` (075d) — `pub mod scrollback`.
- `.gitleaksignore` (075d/1) — 3 fingerprints for the §15 test's deliberately-fake secrets (the sanctioned "add an artifact to ignore" exception).
- tests touched by signature changes: `session.rs`, `session_executor.rs`, `session_prompt.rs`, `recovery_restart_wiring.rs` (075c store-threading).

## Decisions made
- **075a approach A = `vt100`** (Decision 20) — verified viable against the crate source (Context7 lacks the Rust crate); `scrollback()` is the scroll POSITION not the filled count → derive filled via a `set_scrollback(MAX)` probe cached behind `&self`, guarded on `alternate_screen()` (the alt grid has capacity 0 → an unguarded probe clobbers the count — the 075a MED bug, TDD-fixed).
- **075b strategy (a)** — serialize via `state_formatted()` (visible, round-trips) + captured plain scrollback rows; restore by replaying. Idempotent byte-identical round-trip proven. Alt-active snapshots drop the hidden normal buffer (the two-buffer edge — pinned, honest, future-TODO).
- **075c Q1 = `has_restorable_content()`** (scrollback OR a non-blank screen) so mid-alt-screen sessions still `Replayed`; needed a daemon-internal `screen_nonblank` field. `decide_resume` left BYTE-UNCHANGED (caller populates the input, LESSON §36). Store shared as **`Arc<dyn ScrollbackStore>`** (not the brief's `Box` — sharing across actors + recovery; the `Arc<dyn Clock>` precedent).
- **075d USER ruling ①=A** — persist redacted PLAIN-TEXT (formatting dropped; live re-render stays formatted). The **structural §15 guarantee**: `PersistedScrollback` is the ONLY `Serialize` type, post-redaction-only; `VtSnapshot` has no `Serialize`. Fail-closed on non-`Redacted`. 0700/0600 + atomic temp+rename. Substrate write, not Gateway (LESSON §10).
- **075d 2-commit split** — safety core first (own §15 security review), then the non-safety lifecycle.

## Decisions explicitly NOT made (deferred)
- **B (cell-level formatted redaction)** — the fidelity upgrade over ①=A plain-text; deferred.
- The `has_scrollback`→`has_replayable_snapshot` **rename** — the orchestrator tracks it as a separate isolated refactor (075c semantics broadened; field kept + documented).
- The **base64 raw-tap optimization** (075c — tap raw bytes before encode, avoiding the encode→decode round-trip).
- **LIVE survival HITL** — kill-daemon-mid-run reattach/recover; the live property the deterministic arc decides *over* (not unit-testable).
- Backstop-threshold tuning (075d — conservative defaults: 4 MiB/sidecar, 256 MiB dir, 7d TTL).

## TDD compliance
- **075a:** strict RED-first (stub → 5/7 behavioral tests RED → real impl GREEN; the alt-grid MED bug pinned by Test 8 RED→GREEN). Clean.
- **075b–d (core mechanism):** tests co-authored with the implementation and design-validated to GREEN, rather than a separately-observed RED-first phase — a **deliberate process choice** for the design-first / invasive-plumbing slices, where a compiling RED stub ≈ doing the work (flagged + approved at each Step-2.5 as "design-first"). All behavior is **deterministically pinned**, the suite is green, and **every reviewer-found bug got a RED→GREEN regression test** (075a Test 8; 075b alt-active pin; 075c MED-2 reframe; 075d's added tests 1b/5-version/8b). **Not a safety skip** — 075d (the §15 slice) got the mandatory security-reviewer pass EVERY layer (CLEAN ×7). Noted honestly as a process nuance, not a violation.

## Cross-doc invariant audit
**CLEAN — no `shared/` schema-seam model changed this session.** Every type added is daemon-internal: `HeadlessVt`/`VtSnapshot`/`ScrollbackStore`/`PersistedScrollback`, and the `ResumeInputs` feed used real values with no shape change. **CONTRACT_VERSION stayed 0.38.0 across all 5 commits**; no schema-snapshot owed. The `ARCHITECTURE.md`/`daemon/CLAUDE.md`/`LESSONS.md` edits in the working tree are the orchestrator's §15/§8.1/§17 **AS-BUILT prose + LESSON** hot-routing (its `/orchestrate-end` territory), not paired schema edits.

## Reachability
- **075a `HeadlessVt`** — reachable via golden-fixture tests + the 075c producer consumer (mechanism-first).
- **075b `snapshot`/`from_snapshot`** — the 075c producer (`snapshot` on tick/reap) + 075d (`from_snapshot`/`from_plain` on load).
- **075c producer tap** — `SessionActor` pump (reachable from `spawn_session_actor` ← supervisor ← `session.create`); **recovery consumer** — `run_restart_recovery` (`main.rs`, post-supervisor).
- **075d `FileScrollbackStore`** — injected in `main.rs` (replaces the no-op): `save`/`load`/`evict` wired; `sweep_orphans`/`enforce_backstop` run at startup. **`Replayed`-after-restart is LIVE.**
- **No tested-but-unwired gaps** — 075d wired the whole producer→store→recovery path into production.

## Open follow-ups
- **Future-TODO (tracked):** B cell-level formatted redaction · `has_scrollback`→`has_replayable_snapshot` rename · base64 raw-tap optimization · the periodic `save_tick` paused-time test (075c LOW-2) · the two-buffer alt-active edge (075b) · backstop-threshold tuning (075d).
- **Pre-arc carry-forwards (still open):** the `rust-toolchain.toml` pin (fmt-determinism) · the **live-Codex follow-on** bundle (HITL, needs the user's account) · LIVE survival HITL (kill-daemon-mid-run).
- **Process note (orch-raised):** flag a root-dotfile add (`.gitleaksignore`) at the *introducing* commit's Step-9, not one commit late (done one commit late this session; harmless — fixtures are fake).

## How to use what was built
On daemon restart, `run_restart_recovery` loads each live session's persisted scrollback from the 0700 `<base>/scrollback/<sess_…>.json` sidecar (redacted plain-text); a session with restorable content lands on the `Replayed` rung. The live re-render + the eventual xterm.js host are cross-track ui (Phase 7). The durable store is the §15 boundary — never add a `Serialize` to `VtSnapshot` (route any persist through `PersistedScrollback`'s redact-before-build).

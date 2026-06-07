# Spike brief — Phase 0 runnable spikes (0.3, 0.4, 0.1-measurable + HITL prep)

> **This is a SPIKE brief, not a standard `/tdd` red-green brief.** Per `docs/tdd-brief-template.md` "When NOT to use a /tdd brief," exploratory spikes don't get a test-first loop — the deliverable is a **recorded decision** in `docs/spikes/<ID>.md`, plus scaffolding/checklists. The one code-bearing piece (0.4's load harness) is throwaway measurement code, not product code — no production wiring, no Step-7.5 reachability. Do **not** run `/tdd` for these; run them as investigations and write up the decision docs.
>
> Implementer: read this end-to-end, then work the spikes in the **sequence** below. There is no Step-2.5 review gate for research work — but the **two hard guardrails** below are non-negotiable; if a spike's findings push against either, **stop and message the orchestrator** before recording a decision.

## Scope (this brief)

| Spike | MVP task | Mode | Deliverable doc |
|---|---|---|---|
| Codex app-server schema-pin + git2/octocrab spot-check | 0.3 | in-repo, agent-run | `docs/spikes/OQ-HARN-SPIKE-4.md`, `docs/spikes/OQ-INT-SPIKE-6.md` |
| SQLite single-writer load test | 0.4 | in-repo, agent-run (real Rust harness) | `docs/spikes/OQ-DATA-SPIKE-3.md` |
| Claude supervision-mode — measurable parts | 0.1 | in-repo, agent-run | `docs/spikes/OQ-HARN-SPIKE-7.md` |
| Claude credit-pool **drain** — HITL checklist | 0.1 | draft checklist only (user runs ≥ 2026-06-15) | (section in `OQ-HARN-SPIKE-7.md`) |
| macOS sidecar notarization — HITL checklist | 0.2 | draft checklist/scaffolding only (user runs) | `docs/spikes/OQ-PLAT-SPIKE-1.md` |

**Out of scope:** 0.5 contract freeze (the orchestrator authors that brief once 0.1 + 0.3 land). The drain measurement itself (≥ 6/15) and the actual notarization run (Apple Developer creds) are the user's HITL part — you only draft turnkey checklists so their hands-on time is minimal.

---

## ⚠️ Hard guardrails (read first)

1. **The SDK-vs-PTY primary/fallback call is a cat-4 (load-bearing) decision and is NOT yours to lock.** ADR-006 currently leans **Option C Hybrid** (SDK streaming-input + `can_use_tool` chokepoint + PTY mirror; fallback = interactive-PTY + hooks-to-disk + statusLine — `RESEARCH.md:65`). The 2026-06-15 credit-pool change **may invert that** (SDK/`-p` hard-stops on a separate capped pool with no fallback; interactive terminal is exempt — `RESEARCH.md:65` [VERIFIED]). In 0.1 you produce **the decision criterion + all data measurable before 6/15**, and you write the drain measurement as a checklist. You do **NOT** pick SDK-primary vs PTY-primary. Record both branches and the criterion; the orchestrator carries the call back to the lead/user with the drain data once it exists.

2. **O-13 is locked regardless of O-4 (`ARCHITECTURE.md:254-257`).** NexusOps-driven Claude sessions run **`default` permission mode only** (never acceptEdits/bypass/auto); mutation interception is **defense-in-depth** = `can_use_tool` **plus** `PreToolUse` hooks + deny-rules; **background subagents are forbidden** until #27203 is fixed or a pinned version proves coverage. Your 0.1 coverage map must treat #27203 as a **confirmation target**, not an open question — confirm the gap still exists on the pinned version; do not propose a design that relies on `can_use_tool` alone or that enables background subagents.

---

## 0.4 — SQLite single-writer load test (`OQ-DATA-SPIKE-3`)

**Objective:** quantify the single-writer event-store path under concurrent mutating agents and **commit the `[OPEN]` §18 budget numbers** so Phase 1's event-store freeze (1.1) binds to real thresholds — not guesses.

**Read first:** `ARCHITECTURE.md §18` (budgets — currently `[OPEN]`), §7/§7.1 (event envelope + single writer), `docs/planning/DECISIONS.md` ADR-003, `docs/planning/DATA_MODEL.md` (events DDL + WAL pragmas). `BEGIN CONCURRENT` is **NOT** a relied-upon mitigation (ADR-003).

**Method:**
- Stand up a **minimal throwaway** rusqlite WAL harness (one writer task + an append fn matching the §7.1 envelope shape closely enough to be representative — this is measurement scaffolding, *not* the real event store; keep it isolated, e.g. `daemon/benches/` or a `#[ignore]`-gated test, so it never becomes a load-bearing import).
- Drive **N = 20 concurrent mutating agents** (the §18 target). Measure: **intent-commit p95** (submit→durable-commit), **reader latency** under write load (target reads sub-100ms), and the **ceiling** (where does p95 blow up — N=?).
- Record WAL settings used (`journal_mode=WAL`, `synchronous`, `busy_timeout`, `wal_autocheckpoint`).

**Deliverable + acceptance (`docs/spikes/OQ-DATA-SPIKE-3.md`):**
- [ ] Measured intent-commit p95 at N=20, reader p95 under load, and the contention ceiling, with the harness config + how to re-run.
- [ ] A **recommended committed §18 budget set** (concrete numbers) for the orchestrator to write into `ARCHITECTURE.md §18` (you flag the numbers; you do **not** edit §18 — that's orchestrator territory).
- [ ] Explicit statement: does single-writer hold at N=20, and what's the documented ceiling.

---

## 0.3 — Codex app-server schema-pin + git2/octocrab spot-check (`OQ-HARN-SPIKE-4`, `OQ-INT-SPIKE-6`)

**Objective:** pin the Codex version, snapshot its app-server JSON-RPC schema so Phase 3's Codex adapter binds to a stable surface, wire a CI schema-diff gate, and spot-check the git2/octocrab read+merge assumptions ADR-007 depends on.

**Read first:** `ARCHITECTURE.md §9.1` (harness adapter, Codex side), ADR-006 (Codex = app-server JSON-RPC), ADR-007 (dual-git, octocrab, relative-worktrees), `OPEN_QUESTIONS.md` OQ-HARN-SPIKE-4 + OQ-INT-SPIKE-6.

**Method (`OQ-HARN-SPIKE-4`):**
- Detect whether the `codex` CLI is installed/available locally (`codex --version`). **If absent, do not block** — record the intended pin procedure, generate the schema-diff CI gate as scaffolding, and mark the live schema capture as a follow-up the user runs once Codex is installed. State clearly what was run vs deferred.
- If present: pin the version; generate + commit the app-server JSON-RPC schema bundle; confirm the **stable method set** + the **modern vs legacy approval shapes**; wire a **CI schema-diff gate** that fails on an unreviewed schema drift on version bump.

**Method (`OQ-INT-SPIKE-6`):**
- Verify the **git2 read path survives `extensions.relativeworktrees` repos** (git ≥ 2.48) — create a relative-worktree repo, confirm git2 status/diff/branch/worktree-list read correctly; if not, confirm the **CLI-read fallback** for those repos.
- Spot-check **`octocrab`** `pulls().merge()` ergonomics + the **`gh auth token` bootstrap** flow (don't perform a real merge against a live repo without authorization — confirm the API shape + token path; note anything needing the user's GitHub creds as HITL).

**Deliverable + acceptance:**
- [ ] `docs/spikes/OQ-HARN-SPIKE-4.md` — pinned Codex version (or "Codex absent → deferred"), schema bundle location, stable method set + approval shapes, CI gate wired (or scaffolded), re-run instructions.
- [ ] `docs/spikes/OQ-INT-SPIKE-6.md` — git2 relative-worktree read result (survives / falls back to CLI), octocrab merge + token-bootstrap spot-check findings, any HITL-gated pieces flagged.

---

## 0.1 — Claude supervision-mode spike, measurable parts (`OQ-HARN-SPIKE-7`)

> **File-name note:** MVP_TASKS 0.1 names the file `OQ-HARN-SPIKE-7.md`; the underlying open question in `OPEN_QUESTIONS.md` is **OQ-HARN-SPIKE-2** (human-PTY-vs-SDK handoff, ADR-006, resolves **O-4**). Use the MVP_TASKS file name and cross-reference OQ-HARN-SPIKE-2 / ADR-006 / O-4 in the doc. (The orchestrator will reconcile the numbering.)

**Objective:** map what's empirically knowable about Claude supervision **now**, define the SDK-vs-PTY decision **criterion**, and stage the credit-pool drain measurement as a HITL checklist — without locking the cat-4 call (guardrail 1).

**Read first:** `ARCHITECTURE.md §9.1` (Claude side, mode PENDING-SPIKE), `:254-257` (#27203 + O-13), ADR-006, `RESEARCH.md:60-70` (status detection, telemetry fragmentation, credit-pool [VERIFIED]).

**Method — do now (measurable before 6/15):**
- **`can_use_tool` coverage map:** empirically map the callback's coverage across direct `bash`/`Write`/`Edit`, MCP tools, and Task subagents (foreground vs background) under **`default` permission mode** (O-13: that's the only mode we ship). Tabulate intercepted vs bypassed.
- **#27203 confirmation:** confirm the background-subagent bypass still reproduces on the pinned SDK version (confirmation target, not open question).
- **SDK-vs-PTY decision criterion:** write the explicit criterion that will decide SDK-primary vs interactive-PTY-primary once drain data exists — i.e. *"if SDK credit-pool exhaustion can halt a supervised session with no interactive fallback, then …; else …"*. Record **both branches**.

**Method — draft as HITL checklist (user runs ≥ 2026-06-15):**
- A turnkey **credit-pool drain measurement** checklist: exact commands to drive SDK/`-p` usage to exhaustion, what to observe (hard-stop behavior, error surface, whether the interactive terminal stays exempt), and how to feed the result back. The user's hands-on part must be minimal — copy-paste steps + a results template.

**Deliverable + acceptance (`docs/spikes/OQ-HARN-SPIKE-7.md`):**
- [ ] `can_use_tool` coverage table (mode = default; fg/bg subagents distinguished).
- [ ] #27203 reproduction confirmed on the pinned version (or noted if it no longer repros — that would be a finding to escalate).
- [ ] SDK-vs-PTY decision **criterion** stated with both branches — **no pick made** (guardrail 1).
- [ ] Drain-measurement HITL checklist ready to run ≥ 6/15.
- [ ] Pinned SDK version recorded (statusLine/transcript schema churns across v2.1.x — `RESEARCH.md`).

---

## 0.2 — macOS sidecar notarization (`OQ-PLAT-SPIKE-1`) — HITL checklist/scaffolding

**Objective:** make the user's notarization validation turnkey — they have the Apple Developer creds; you remove every other unknown.

**Read first:** `ARCHITECTURE.md §16` (deployment/bootstrap), ADR-005 (Brain = PyInstaller sidecar via Tauri `externalBin`), ADR-011 (Developer ID signing + notarization = early release-blocker), `OPEN_QUESTIONS.md` OQ-PLAT-SPIKE-1 (#11992 + deep-sign + `com.apple.security.cs.allow-unsigned-executable-memory`).

**Method:** draft the deep-sign + notarize checklist for a bundled PyInstaller Brain sidecar in a real signed Tauri build — `externalBin` config, deep-sign order for bundled libs, required entitlements, `notarytool` submit/staple steps, and the **success criteria**. Include the **loopback-HTTP Brain fallback** (§13.1) as the documented contingency **if** notarization of the bundled sidecar proves blocked by #11992.

**Deliverable + acceptance (`docs/spikes/OQ-PLAT-SPIKE-1.md`):**
- [ ] Turnkey checklist (config + commands + entitlements + staple/verify) the user runs against a real signed build.
- [ ] Explicit success criteria + the documented loopback-HTTP fallback decision tree if blocked.
- [ ] Clearly marks the steps that need the user's Developer ID / notary creds (the HITL boundary).

---

## Sequencing

1. **0.4** and **0.3** first — fully in-repo, unblock nothing-but-themselves, and 0.3 is on the 0.5 critical path.
2. **0.1 measurable parts** next — also on the 0.5 critical path; produces the criterion + HITL checklist.
3. **0.2 + 0.1-drain checklists** — pure authoring; do whenever convenient in the session.

**0.5 gate:** once **0.1 + 0.3 land**, message the orchestrator — the orchestrator authors the 0.5 contract-freeze brief. **Per guardrail 1, 0.5 will deliberately exclude any supervision-touching contract surface that depends on the unresolved SDK-vs-PTY call** until the drain data resolves it.

## How to invoke

This is the first slice of the session → run `/session-start` once to orient, then work the spikes above in sequence. These are investigations, not `/tdd` slices — no red-green loop, no Step-2.5 gate. When all four land (or you close out the session), summarize per spike: what was measured, what was deferred to HITL, and any findings that push on a guardrail (those escalate to the orchestrator immediately, not at close-out).

## Flags expected back at the orchestrator

- **Cross-doc / architecture-doc notes:** §18 budget numbers to write (from 0.4); the `OQ-HARN-SPIKE-7` vs `-2` numbering reconcile; any §9.1 Claude-mode detail that firms up (routes via `/arch-finalize` if it changes a LOCKED anchor — do not edit §9.1 directly).
- **Decisions to record:** SDK-vs-PTY criterion + branches (cat-4 — orchestrator carries to lead/user with drain data).
- **Findings (escalate hot):** if #27203 no longer repros, if single-writer fails at N=20, or if notarization is blocked with no working fallback.

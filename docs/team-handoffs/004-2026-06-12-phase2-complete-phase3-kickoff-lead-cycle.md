# Team Handoff 004 — Phase 2 COMPLETE + Phase 3 kickoff (P3.1 done) → lead-cycle full shutdown

**Date:** 2026-06-12
**Track:** daemon (single-track; root checkout, branch `main`, no worktree)
**Predecessor handoff:** `003-2026-06-11-phase2-2.0sec-done-scaffolding-upgrade.md`
**Successor handoff:** _(filled in when the next /team-end runs)_
**Round-seal commit at handoff:** `a387e20` (LOCAL — ahead 28 of origin `0578d60`; **NOT pushed** — push is user-gated)

## Why this handoff exists
User-directed **full team shutdown** for a **lead-cycle** — the team-lead session reached 71% [WARN] after a long, dense arc (the entire Phase 2 build + Phase 3 kickoff). Teammates are fine (orch 37% / impl 43%); this is purely a fresh-budget restart of the lead. The next `/team-start daemon` spawns a fresh lead + team to resume at Phase 3 cont.

## Team composition at close
- **Lead:** this session (track `daemon`) — terminated at this `/team-end`.
- **Orchestrator:** `daemon-orchestrator` — `/orchestrate-end`-sealed the P3.1 round at `a387e20`; terminated.
- **Implementer:** `daemon-implementer` — `/session-end`-closed (session doc `015`, `3d2bc7f`); terminated.
- All teammates closed at round-seal `a387e20`.

## Active arc + where it landed
**PHASE 2 — the Action Gateway (the single audited mutator / INV-SEC-1 chokepoint) is COMPLETE + gated + sealed** (`21d5969`; `/phase-exit 2` CLEAR; CONTRACT 0.19.0; 247 tests; 0 findings). Then **PHASE 3 kicked off**: **3.1 DONE** — the §9.1 **HarnessAdapter contract freeze** (`CONTRACT 0.19.0→0.20.0`) + the `proj_usage_ledger` projector (folds `TelemetrySampled` into per-day usage rollups; telemetry modeled as a **System-actor non-mutation event**, outside the Gateway mutation path — precedent: DeviceRegistered/AuditIntegrityViolation). Briefs 032–040; session docs 010–015; LESSONS through §23.

**NEXT: Phase 3 cont.** The outgoing orch's lead-named next is 3.2, **but it strongly RECOMMENDS 3.4-FIRST**: do **P3.4 (Terminal Channel, §6.4/ADR-009)** before 3.2/3.3, because (a) P3.4 is the **ungated ui-track unblocker** (the ui track's parked 6.3d Session-Terminal well waits on it), and (b) 3.2/3.3 (harness drive-mode) are **gated on HITL that isn't resolved yet** — the **0.1 cat-4 SDK-vs-PTY** decision (needs the user's credit-pool drain data ≥2026-06-15; today is 06-12) + the **0.3 Codex schema** spike. Going 3.4-first lets those HITL rulings land before drive-mode. The 3.4-first rec is banked in the tracker + Carry-forward.

## In-flight at close
**None — clean close.** P3.1 fully sealed (`a387e20`); no slice open; tree clean (only untracked `.codegraph/` tooling artifact).

## Carry-forward to next team session
- **Currently in progress (IMPLEMENTATION_PLAN.md):** Phase 2 ✅ sealed `21d5969` → P3.1 ✅ `a387e20` → **Phase 3 cont (3.2/3.3/3.4); orch rec = 3.4-first.**
- **Next session target:** Phase 3 cont — **author brief 041; lead may take the 3.4-first rec** (Terminal Channel — the ui-track unblocker) given 3.2/3.3 are HITL-gated.
- Open Carry-forward: the P3.1 forward-pin SPREADs (orch added 3); the cross-track ui Usage-shapes item partially advanced by the 0.20.0 freeze. Full detail: `IMPLEMENTATION_PLAN.md` Carry-forward + Phase-3 section.

## Open decisions / blockers for the human
- **⬆️ PUSH PENDING (user action):** **28 commits local-unpushed** at `a387e20` (origin at `0578d60`); autonomous pushes are harness-blocked → **the user authorizes/runs `git push origin main`.** This banks the whole Phase-2 + P3.1 build, runs the §14 GitHub-Actions CI for the first time, and gives the parallel-track worktrees a pushed base. (Local commits are durable on disk meanwhile.)
- **TRACK FAN-OUT (user is opening ui + edges themselves):** Phase 2's contract is frozen + audit-clean → the ui + edges tracks are unblocked. The user has the full package (worktree commands + `/team-start ui`/`edges` + spawn prompts) — ui resumes by syncing `../NexusOps-ui` (`git merge main` → 0.20.0) then `/team-start ui`; edges is new (`git worktree add -b track/edges ../NexusOps-edges main` then `/team-start edges`). 3 tracks → the user is the escalation conduit for all three. `shared/` is daemon-owned (only daemon bumps CONTRACT_VERSION); ui/edges consume via regen. ui's 6.3d terminal well waits on daemon **P3.4** (hence the 3.4-first rec).
- **HITL (parked, time/account-gated):** **0.1** credit-pool drain (≥2026-06-15) → then cat-4 SDK-vs-PTY (gates 3.2/3.3 drive-mode) → 0.5b ExecutionProfile re-freeze. **0.2** notarization (Apple Developer-ID + notary creds). **0.3** Codex schema capture (gates 3.3, mockable meanwhile).
- **10 away/user-authority rulings flagged for review** (in Decisions-tabled + the lead away-log `~/.claude/nexusops-lead-away-log.md`), all flagged for the next `/arch-finalize`: Gateway IDs=A · status-enum $def=B · ActionRequest::Denied=terminal · §15 inputs/resource_refs row-redaction=A · workflow.command.invoke=risk-4 · §15 idempotency-key=SHA-256-of-raw=A · §6.4 codes +internal_error/+fencing_conflict=A · §18 perf re-baseline (USER-ruled) · the §6.4-Q7 · the §9.1 telemetry-as-System-event design.
- **Scaffolding/tooling notes (non-blocking, for the user):** `scripts/spec-lint.sh` had ~4 real bugs fixed locally (alpha-vs-numeric task-IDs; backtick-waiver false-fail; `target/`-scan hang) — likely upstream-template bugs worth fixing in the scaffolding repo. The `/context-check` post-cycle 10-min false-WARN window (`STALE_SECONDS=600`) is a known artifact (disambiguate by heartbeat freshness; or fix via deregister-on-shutdown / env-configurable staleness).

## Spawn prompts ready for the next team session

**Orchestrator:**
```
You are daemon-orchestrator on the NexusOps agent team.
Track: daemon. Team: nexusops-daemon. Single-track — repo root, branch main (no worktree). Ignore peer DMs without the `daemon-` prefix.
Activated because: resuming after a /team-end full shutdown (handoff 004) — lead-cycle for fresh budget. Phase 2 COMPLETE + sealed (21d5969); P3.1 DONE (§9.1 HarnessAdapter contract freeze → CONTRACT 0.20.0 + proj_usage_ledger projector; seal a387e20, 28 ahead of origin, push USER-GATED — NEVER git push). Action Gateway complete; 247+ tests. NEXT = Phase 3 cont. ⚠️ ORCH REC = 3.4-FIRST: do P3.4 (Terminal Channel, §6.4/ADR-009) before 3.2/3.3 — it's the ungated ui-track unblocker (ui 6.3d waits on it) + 3.2/3.3 drive-mode are gated on the 0.1 SDK-vs-PTY HITL (credit-pool ≥6/15) + 0.3 Codex HITL (not yet resolved). Author the next brief (041, daemon NNN); spec-lint before dispatch; §2.5-seam snapshot for any shared-contract touch.
PHASE-3 SAFETY RULES (load-bearing, quote by name): #9 NEVER scrape the PTY for machine state (status from SDK/app-server streams; PTY display-only); #10 Brain proposes never executes (Claude default permission mode only; NO bg subagents, #27203); #11 Codex rollout files hardened (0700/0600). TEST PATTERN: deterministic logic (status derivation, parsers) TEST-FIRST; live agents/PTY via FakeHarness/FakePty + injectable Clock/IdGen (§14).
DECIDED — don't re-litigate (Decisions-tabled, flagged for user review): the 10 rulings (Gateway IDs=A · status $def=B · Denied=terminal · §15 row-redaction=A · risk-4 · §15 idempotency=A · §6.4 codes=A · §18 re-baseline · §6.4-Q7 · §9.1 telemetry-as-System-event).
OPERATING MODEL: build autonomously under standing authorization; escalate ONLY the 4 categories to the lead; park genuine HITL/account/taste. CONTEXT: /context-check per layer; ping the lead at WARN; post-cycle staleness ~10min (disambiguate by heartbeat freshness).
FIRST ACTION — register: ~/.claude/scripts/team-register.sh "daemon-orchestrator" orchestrator "nexusops-daemon" "" "daemon" "main"
Then run /orchestrate-start (NOT /session-start). Confirm: (1) start command, (2) registry written.
```

**Implementer (`daemon`):**
```
You are daemon-implementer on the NexusOps agent team.
Track: daemon. Team: nexusops-daemon. Working directory: daemon/ (repo root, branch main — single-track). Talk only to daemon-orchestrator; ignore other-prefix peer DMs.
Activated because: resuming after a /team-end full shutdown (handoff 004). Phase 2 COMPLETE + sealed; P3.1 DONE (HarnessAdapter contract freeze 0.20.0; seal a387e20, 28 ahead, push USER-GATED → commit normally, NEVER git push). NEXT = Phase 3 cont (likely 3.4 Terminal Channel first — the orch's rec). Wait for the orchestrator's dispatch (brief + spec-lint PASS) before RED.
PHASE-3 SAFETY RULES (don't paraphrase): #9 NEVER scrape the PTY for machine state (status from SDK/app-server streams); #10 Brain proposes never executes (no bg subagents, #27203); #11 Codex rollout files hardened (0700/0600). TEST PATTERN: deterministic logic TEST-FIRST; live agents/PTY via FakeHarness/FakePty + Clock/IdGen (§14). Rust strict per daemon/CLAUDE.md (no unwrap/expect w/o justification, clippy -D, keychain-only secrets, NO PTY-scraping). PreToolUse territory-guard ACTIVE — orch-territory writes blocked; flag at Step 9.
FIRST ACTION — register: ~/.claude/scripts/team-register.sh "daemon-implementer" implementer "nexusops-daemon" "daemon" "daemon" "main"
Then run /session-start (NOT /orchestrate-start). Confirm: (1) start command, (2) registry written.
```

## How to resume
Next team session: run `/team-start daemon`, read this handoff 004 + `~/.claude/nexusops-lead-away-log.md` (the full decision/cycle record) + `IMPLEMENTATION_PLAN.md` "Currently in progress" + the Phase-3 section, then spawn the two teammates with the prompts above (verify read-backs). Decide 3.2 vs the orch's **3.4-first** rec (recommended — the ungated ui-track unblocker; 3.2/3.3 are HITL-gated). The ui + edges tracks are the **user's** separate lead sessions (not this lead's). **Remind the user to push** (`git push origin main`) when ready — 28 commits await origin.

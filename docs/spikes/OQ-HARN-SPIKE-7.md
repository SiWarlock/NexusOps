# OQ-HARN-SPIKE-7 — Claude supervision-mode spike (measurable parts + drain HITL)

| | |
|---|---|
| **MVP task** | 0.1 (Phase 0) |
| **Open question** | `OQ-HARN-SPIKE-7` (MVP-TASKS name) ≡ **`OQ-HARN-SPIKE-2`** in `OPEN_QUESTIONS.md` (human-PTY-vs-SDK handoff) — resolves **O-4**, informs **ADR-006** |
| **Spec anchors** | `ARCHITECTURE.md §9.1` (Claude adapter, mode PENDING-SPIKE), `:254-257`/`§9.1` (#27203 + O-13), ADR-006, `RESEARCH.md` R-CC (status/telemetry/credit-pool [VERIFIED]) |
| **Status** | ⚠️ MEASURABLE PARTS RESOLVED — coverage map + #27203 confirmed + criterion stated (NO pick); **drain measurement + live coverage re-run = HITL ≥ 2026-06-15** |
| **Date** | 2026-06-07 |
| **Gates** | Phase 3 Claude adapter |

> **Numbering note:** MVP-TASKS calls this file `OQ-HARN-SPIKE-7`; the underlying open
> question is `OQ-HARN-SPIKE-2` (ADR-006, O-4). Using the MVP-TASKS name; orchestrator
> reconciles the numbering.

> **⚠️ Guardrail 1 (honored):** the SDK-primary vs interactive-PTY-primary call is a **cat-4
> load-bearing decision and is NOT locked here.** This doc states the **criterion + both
> branches**; the orchestrator carries the call to the lead/user **once the ≥6/15 drain data
> exists**. **Guardrail 2 (honored):** #27203 treated as a confirmation target (§2).

---

## 1. What was measurable now vs HITL (today = 2026-06-07, before the 6/15 split)

| Item | Status |
|---|---|
| Pinned versions | ✅ Claude Code CLI **2.1.168**; Agent SDK **~0.3.x** (not locally installed) |
| `can_use_tool` coverage map (default mode; fg/bg subagents) | ✅ documented from LOCKED §9.1 + RESEARCH R-CC [VERIFIED]; live re-run = HITL probe (§4) |
| #27203 background-subagent bypass — still present on pinned version? | ✅ **CONFIRMED still present** (§2) |
| SDK-vs-PTY decision **criterion** + both branches | ✅ stated, **no pick** (§3) |
| **Credit-pool drain** measurement | ⛔ **HITL ≥ 2026-06-15** — cannot run before the split (§5) |
| Live `can_use_tool` coverage probe | ⛔ deferred — needs SDK install + auth; folds with the 6/15 run (§4) |

**Why the live SDK runs are deferred, not skipped:** the Agent SDK is not installed and no
auth is configured here, and driving real SDK sessions **burns the user's Claude quota** —
especially sensitive right before the **2026-06-15** change where SDK/`-p` draws a *separate
capped pool* (`RESEARCH.md` R-CC [VERIFIED]). The coverage map is already [VERIFIED] in
RESEARCH + LOCKED in §9.1, so the design is unblocked; §4's probe is provided turnkey for the
user to confirm-on-pinned-version in the same ≥6/15 session as the drain.

---

## 2. `can_use_tool` coverage map (default mode) + #27203 confirmation

Authoritative source = `ARCHITECTURE.md §9.1` mutation-coverage matrix (LOCKED) + RESEARCH
R-CC [VERIFIED]. NexusOps ships **`default` permission mode only** (O-13).

| Tool category | Claude `can_use_tool` (default mode) | Source / status |
|---|---|---|
| direct `bash` / `Write` / `Edit` | **INTERCEPTED** | §9.1 / R-CC [VERIFIED] |
| MCP tools (`mcp__*`), direct | **INTERCEPTED** (falls through to mode+callback) | §9.1 |
| Task subagent — **foreground** | **NOT guaranteed** (inherits parent mode; bypassed if parent ever in acceptEdits/bypass) | §9.1 |
| Task subagent — **background** | **BYPASSED** (#27203) | §9.1 + **confirmed below** |

**#27203 — confirmed still present on the pinned version (guardrail 2):**
- Issue: *"`canUseTool` callback not invoked for background subagent tool calls in default
  permission mode"* — **state: Closed as "not planned" (won't-fix).**
- Repro'd on SDK 0.2.42 / Claude Code 2.1.42; **no fix shipped through 2.1.168** → the bypass
  **persists** on the pinned stack. *Closed ≠ fixed* — "not planned" means Anthropic declined
  to fix it.
- Secondary hazards reported on the same issue, both reinforcing O-13:
  1. background-subagent permission denials can **corrupt the parent session transport**
     ("Stream closed", self-heals after minutes) — a chatty-failure mode, not just a gap;
  2. requesting `bypassPermissions` without account access **silently downgrades to `default`**
     (safe direction for us, but means mode-request errors can't be relied on for misconfig
     detection).

**Implication (no new design — confirms §9.1 / O-13):** keep **background subagents
FORBIDDEN**; since #27203 is won't-fix, the "until #27203 is fixed" unlock is effectively
dead — the only path to ever enabling them is §9.1's *"a pinned version empirically proves
coverage"* (the §4 probe is how you'd prove it). Mutation interception stays
**defense-in-depth**: `can_use_tool` **+** `PreToolUse` hooks + deny-rules, never the callback
alone.

---

## 3. ⭐ SDK-vs-PTY decision criterion (cat-4 — stated, NOT decided)

**Deciding question** (resolved only by the ≥6/15 drain, §5):
*Does Agent-SDK/`-p` credit-pool exhaustion **hard-stop a supervised session with no
interactive fallback**, while an interactive terminal session stays **exempt** and keeps
working?*

- **Branch A — if YES** (SDK pool hard-stops supervision; interactive PTY exempt) →
  **interactive-PTY-primary.** Drive sessions via interactive PTY + `PreToolUse` hooks +
  deny-rules + statusLine heartbeat; accept **coarser** mutation interception (hooks, not
  `can_use_tool`). The SDK becomes secondary/optional. *Rationale:* a control plane cannot
  let supervised sessions die on a separate capped pool mid-work when the user's own
  interactive usage would have survived.
- **Branch B — if NO** (pool exhaustion behaves like ordinary rate-limiting, or interactive
  is equally affected, or the pool is generous enough) → **SDK-streaming-input-primary**
  (ADR-006 Option C Hybrid, the current lean): `can_use_tool` as the in-harness mutation
  chokepoint + **PTY mirror** for human display/takeover + transcript-JSONL tail for durable
  replay. *Rationale:* `can_use_tool` is the strongest single supervision primitive (typed
  allow/deny/rewrite **at** the mutation point).

**Invariant in BOTH branches** (not contingent on the call): O-13 `default` mode only;
`PreToolUse` hooks + deny-rules as a redundant interception layer; background subagents
forbidden (§2). Only the *primary drive mode* differs — the defense-in-depth posture does not.

**No pick is made here** (guardrail 1). The orchestrator carries this to the lead/user once
§5's drain data lands.

---

## 4. HITL — live `can_use_tool` coverage probe (turnkey; run with §5)

Confirms the §2 matrix on the pinned stack. Script: `docs/spikes/claude-supervision/can_use_tool_probe.mjs`
(marked `TODO-VERIFY` for SDK call shape — confirm against the pinned SDK).

```bash
npm i @anthropic-ai/claude-agent-sdk@<PINNED>      # pin == §16 version tuple
export ANTHROPIC_API_KEY=...                        # or: claude setup-token
node docs/spikes/claude-supervision/can_use_tool_probe.mjs
```
**Expect:** direct bash/Write/Edit + MCP(direct) appear as `INTERCEPTED`; a **background**
subagent's inner tool call is **ABSENT** (= #27203 bypass reproduced). If the background call
*does* appear intercepted → that's a **finding to escalate** (gap closed → re-evaluate the
background-subagent ban).

---

## 5. HITL — credit-pool drain measurement checklist (run ≥ 2026-06-15)

**Goal:** answer §3's deciding question. Minimal hands-on; copy-paste + fill the template.

**Pre-reqs:** date ≥ 2026-06-15; `claude --version` == 2.1.168 (or the then-pinned floor);
Agent SDK installed + authed (§4); note the subscription plan + the SDK credit-pool size if
surfaced.

**Steps:**
1. **Baseline:** record plan, `claude --version`, date, and (if shown) the SDK pool balance.
2. **Drain the SDK pool** — in terminal **A**, loop SDK/`-p` work until it stops:
   ```bash
   for i in $(seq 1 1000); do claude -p "Summarize the number $i in one word." || break; done
   ```
   Capture the **exact stop surface**: error code/message, HTTP status, whether it's a
   **hard stop** vs a `Retry-After`/transient, and whether any in-session fallback offered.
3. **Interactive-exempt test** — while A is exhausted, in terminal **B** start an
   **interactive** session (`claude`, NOT `-p`) and run one small task. Record: did B **still
   work** (interactive exempt from the SDK pool) — yes/no?
4. **Classify** against §3:
   - A hard-stops **AND** B still works → **Branch A (interactive-PTY-primary).**
   - A behaves like normal rate-limiting **OR** B is equally blocked → **Branch B
     (SDK-primary).**
5. **Feed back:** paste the filled template below into this doc and ping the orchestrator →
   the orchestrator carries the cat-4 call to the lead/user.

**Results template (fill + paste):**
```
date: 2026-06-__        claude --version: ______        plan: ______
SDK/-p drain stop surface: [hard-stop | retry-after | other] — code/msg: ______
in-session fallback offered? : [yes/no]
interactive terminal (B) still worked while SDK pool exhausted? : [yes/no]
=> criterion result: [Branch A: PTY-primary | Branch B: SDK-primary]
notes: ______
```

---

## 6. Flags back to the orchestrator

- **Decision to record (cat-4 — orchestrator → lead/user):** the SDK-vs-PTY **criterion +
  both branches** (§3). **No pick made** (guardrail 1); decide once §5 drain data exists.
- **Confirmation (no escalation needed):** #27203 still present on 2.1.168 (won't-fix) →
  keep background subagents forbidden (§2). Matches §9.1 — **no architecture change.**
- **Deferred to HITL ≥ 6/15:** the drain measurement (§5) + the live coverage probe (§4).
- **§16 version tuple:** pin Claude Code CLI **2.1.168** + the Agent SDK version when the
  adapter is built; statusLine/transcript schema churns across v2.1.x (`total_input_tokens`
  semantics changed at v2.1.132) → re-verify on every bump (RESEARCH R-CC).
- **Escalate-hot ONLY IF** the §4 probe later shows #27203 no longer repros (gap closed) —
  not the case today.

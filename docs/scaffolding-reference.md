# NexusOps — Scaffolding Reference

> Project-specific map of this repo's Claude Code scaffolding. Documents what each piece is and how this project adapts the universal agent-team pattern.
>
> **For the universal pattern documented end-to-end**, see `SCAFFOLDING-GUIDE.md` (the project-agnostic guide this scaffolding was generated from). This file is the project-specific instance.

---

## TL;DR

This project runs the **agent-team orchestrator + implementer pattern** (three roles + human). Same slash commands, same Step-9 routing matrix, same N+2 commit cadence, same cross-doc invariants discipline as the universal pattern. **Adaptations are project-shaped:** this project's stack, code areas, phase plan (Phase <N> (tasks <N>.<M>, e.g. Phase 1 / task 1.1)), forbidden patterns, and architecture are its own.

_(Single-operator fallback: drop the team-lead row + `/team-start`/`/team-end`; the human bridges between orchestrator + implementer sessions.)_

---

## File inventory

```
NexusOps/
├── .claude/
│   ├── commands/                       # Slash commands
│   └── agents/                         # Subagents (opt-in starter set + reactive additions)
├── daemon/
│   ├── CLAUDE.md                       # Code-area conventions
│   └── LESSONS.md                      # Lessons logged (§1+)
├── docs/
│   ├── team-protocol.md                # Loaded by /team-start (team pattern only)
│   ├── orchestrator-briefing.md        # Loaded by /orchestrate-start
│   ├── tdd-brief-template.md           # /tdd brief format
│   ├── scaffolding-reference.md        # THIS FILE
│   ├── team-handoffs/                  # /team-end outputs (team pattern only)
│   ├── briefs/                         # Numbered /tdd briefs (NNN-<task-id>-<topic>.md)
│   ├── sessions/                       # Numbered chronological session docs
│   └── runbooks/                       # Operational procedures
├── CLAUDE.md                           # Root — project conventions + shared comm rules
├── MVP_TASKS.md                        # Task tracker
└── ARCHITECTURE.md                     # Architecture / design contract

# User-global (~/.claude/) — populated at /team-start by spawn prompts (team mode only):
~/.claude/
├── statusline-command.sh               # Status line + heartbeat writer (install once)
├── scripts/
│   └── check-team-context.sh           # /context-check helper (install once)
├── team-registry/                      # Per-session: {session_id, name, team, role, cwd, ts}
│   └── <session_id>.json               # Written by teammate at startup via spawn prompt
├── heartbeats/                         # Per-session ctx_pct heartbeats (status line writes IFF registry exists)
│   └── <session_id>.json               # Updated every status line refresh
└── team-history/                       # Per-slice trajectory data
    └── <team>/<name>.jsonl             # Per-slice ctx snapshot (/context-check --snapshot) for 3-slice rolling growth
```

<!-- ▼ EXAMPLE BLOCK [id=inventory-extension]: extend the inventory with the project's real layout — extra code areas, deliverable docs, eval suites. ▼ -->

This is a **2-area** project — both code areas carry their own `CLAUDE.md` + `LESSONS.md` and field their own implementer session:

```
NexusOps/
├── daemon/                             # AREA 1 — the Rust daemon (trust core): nexusopsd, the single audited mutator
│   ├── CLAUDE.md                       # Rust conventions (clippy, rustc via cargo check, cargo test)
│   ├── LESSONS.md                      # Banked daemon lessons (§1+)
│   └── …                               # Tokio runtime: rusqlite, portable-pty, git2, octocrab, keyring, rmcp
├── ui/                                 # AREA 2 — the Tauri desktop UI (TS frontend + thin Rust host)
│   ├── CLAUDE.md                       # TS/Tauri conventions (oxlint, tsc --noEmit, Vitest + Tauri-driver e2e)
│   ├── LESSONS.md                      # Banked UI lessons (§1+)
│   └── …                               # React 19 + Vite + Tauri 2.x; Zod-validated projections
└── docs/
    ├── architecture/                   # Pre-existing architecture artifacts (ARCHITECTURE.md is the binding contract)
    └── planning/                       # Pre-existing planning artifacts (/arch-draft → /arch-finalize → /tasks-gen)
```

**Per-area command modes:** `/preflight` and `/run-tests` carry **two** mode blocks — a `daemon` mode (`cargo` toolchain) and a `ui` mode (`pnpm` toolchain). The implementer runs the mode matching its cwd. Neither block is removed; a single-area project would keep only one.

The **trust boundary is the local machine** — the daemon is the only process that mutates state, and every mutation is a typed, risk-classified, approved Action recorded as an immutable event. The UI reads projections; agents and the Project Brain only propose intents. This makes the `daemon/` ↔ `ui/` split a hard contract line, not a convenience — cross-area work is never bundled into one slice (see "Estimated commit count" in `docs/tdd-brief-template.md`).

<!-- ▲ END EXAMPLE BLOCK [id=inventory-extension] ▲ -->

---

## Team pattern (three roles + human)

The full topology, role/cwd/loads table, escalation taxonomy, and naming/cross-bleed rule are **canonical in root `CLAUDE.md` "Team coordination — shared rules"** (+ `docs/team-protocol.md` for the lead). One-line map:

- **Team lead** — thin, durable; `/team-start` + `/team-end`; escalation conduit to the human; stateless between events. Reads progress from the task list + free idle-notifications; pings nobody per-slice.
- **Orchestrator** — planning, scope, docs, Step-2.5 review, Step-9 routing, commit messages, push, `/orchestrate-end`.
- **Implementer (per area)** — `/tdd` cycles, `/preflight`, `/session-end`, code commits only.

Orchestrator ↔ implementer communicate **directly** (`SendMessage` for checkpoints, the **task list** for status); the lead is looped in only for the 4 escalation categories + tier-crossing context.

---

## Slash commands

The full command list + descriptions is **canonical in root `CLAUDE.md` "Slash commands available."** Pairs: lead `/team-start`+`/team-end`; orchestrator `/orchestrate-start`+`/orchestrate-end`; implementer `/session-start`+`/session-end`; plus `/tdd`, `/wired`, `/context-check`, `/preflight`, `/run-tests`, `/check-arch` (+ optional `/eval`, `/trace`). _(Single-operator: no `/team-start`/`/team-end`.)_

---

## Workflow patterns

### Per-slice TDD round

1. Orchestrator authors a brief → `docs/briefs/NNN-<task-id>-<topic>.md`
2. Orchestrator **creates + assigns the slice's task** (`TaskCreate` + `TaskUpdate owner`) + sends the brief reference (one line) to the area implementer
3. Implementer runs `/tdd <feature>` → Step 0 (self-check; user-confirm in single-operator) → Step 1 → Step 2 RED → Step 2.5 tight test-design write-up
4. Orchestrator reviews + replies directly (`APPROVED.`/`TWEAK:`/`ADD:`); escalates a safety design Q if needed
5. Implementer Steps 3-7 (confirm RED → GREEN → refactor → suite)
6. **Step 7.5 reachability** — confirm wiring from a production entry point (`/wired`)
7. Step 8 lint+typecheck + **policy-gated** reviewer fan-out (per root `CLAUDE.md` reviewer policy — security on invariant slices, code-quality lite)
8. Implementer sends categorized Step-9 flags directly to orchestrator
9. Orchestrator routes hot (commit-message-first reply); escalates deferments / safety findings / load-bearing architectural calls
10. Implementer Step 10: commits with the orchestrator-authored message, then **marks the task `completed`** (hash in metadata) + a one-line wake to the orch
11. **(Team mode)** Orchestrator runs `/context-check <team> --snapshot <hash>` locally; pings the lead **only if a tier ≥ WARN is crossed**; then dispatches the next slice without waiting
12. Repeat

### Context monitoring + auto-cycle (team mode only)

Status lines write per-session heartbeats (gated on a `~/.claude/team-registry/` entry, so solo sessions are silent). The orchestrator runs `/context-check <team>` **locally each slice** but pings the lead **only on a tier crossing** (≥ WARN) — OK slices produce no ping; the lead's free idle-notifications + the task list cover progress. Tiers (WARN 70 / ACTION 75 / HARD-STOP 80, env-overridable via `CLAUDE_TEAM_CTX_*`) and the full auto-cycle flow are **canonical in root `CLAUDE.md` "Context monitoring" + `docs/team-protocol.md` "Context monitoring + auto-cycle."**

### Step-9 routing matrix

Canonical in `docs/orchestrator-briefing.md` "Step-9 routing matrix." (Implementer categorizes; orchestrator routes hot.)

### Carry-forward triage at `/orchestrate-end`

Five outcomes: DELETE / KEEP / **INLINE-TARGET (→ real task checkbox, not an annotation)** / DEFER (escalate) / SPREAD.

### Reachability (tested ≠ shipped)

`/tdd` Step 7.5 + `/wired` prove each feature is invoked from a real entry point; the `reachability-auditor` subagent (if installed) audits an entire code area at phase boundaries.

### Commit cadence

N slice commits + 1 session-doc commit + 1 round commit = **N + 2** per round. Push once at `/orchestrate-end` — **only when a remote is configured** (to **origin (git@github.com:SiWarlock/NexusOps.git)**).

---

## Project-specific conventions

<!-- ▼ EXAMPLE BLOCK [id=instance-conventions]: the conventions unique to this project — its architecture sentence (if any), its forbidden patterns, its key safety rules, its layer dependency rule, its cross-doc invariant set. These distinguish this project's instance from the universal pattern. ▼ -->

- **Architecture sentence:** *A desktop-first cockpit whose detached Rust daemon is the single, audited mutator of all state — every change is a typed, risk-classified, approved Action recorded as an immutable event; the UI reads projections, agents and the Project Brain only propose intents, and the local machine is the trust boundary.*
- **Two code areas, one contract line:** `daemon/` (the Rust daemon — trust core; `cargo` toolchain, clippy, cargo test) and `ui/` (the Tauri desktop UI — TS frontend + thin Rust host; `pnpm` toolchain, oxlint, Vitest + Tauri-driver e2e). Each fields its own implementer session and its own `CLAUDE.md` + `LESSONS.md`. **Cross-area slices are never bundled** — the daemon↔UI boundary is the trust boundary, not a refactor convenience.
- **Single-mutator invariant:** the daemon is the **only** process that mutates state; the UI reads **projections** only, and agents + the Project Brain **propose intents**, never apply them. Any UI-side or agent-side write path is a forbidden pattern.
- **Action / event invariant:** every state change is a **typed, risk-classified, approved Action** recorded as an **immutable event**. Mutations that bypass the typed-Action path, skip risk classification, skip approval, or mutate an already-recorded event are forbidden.
- **Trust boundary = the local machine.** Remote/agent inputs cross into the daemon only as proposed intents subject to the approval gate; nothing remote mutates directly.
- **Forbidden patterns:** see `daemon/CLAUDE.md` (Rust core) and `ui/CLAUDE.md` (Tauri/TS host).
- **Key safety rules:** see root `CLAUDE.md` "Key safety rules."
- **Cross-doc invariants:** the `daemon/CLAUDE.md` table tracks typed Action/event models mirroring `ARCHITECTURE.md` sections; the `ui/CLAUDE.md` table tracks the Zod-validated projection contracts the UI reads. Field changes require atomic doc edits paired with the model change in the same slice.
- **Phase plan:** `MVP_TASKS.md` drives state via Phase <N> (tasks <N>.<M>, e.g. Phase 1 / task 1.1); `ARCHITECTURE.md` is the binding design contract every brief anchors to.

<!-- ▲ END EXAMPLE BLOCK [id=instance-conventions] ▲ -->

---

## State sources of truth

| Concern | Source of truth | Loaded by |
|---|---|---|
| Team topology + escalation rules | Root `CLAUDE.md` "Team coordination" + `docs/team-protocol.md` | `/team-start` (lead-specific) |
| Current state, "what's done, what's next" | `MVP_TASKS.md` | `/orchestrate-start` + `/session-start` |
| Technical narrative of just-landed work | Most recent `docs/sessions/<NNN>-*.md` | `/orchestrate-start` |
| Round ledger (thin pointer-lines) | `MVP_TASKS.md` "Log" | `/orchestrate-start` |
| Per-slice design audit trail | `docs/briefs/<NNN>-<task-id>-<topic>.md` | On-demand; latest at `/orchestrate-start` |
| Team-pause handoff state | Most recent `docs/team-handoffs/<NNN>-*.md` | `/team-start` (when resuming) |
| Conventions / patterns | `daemon/LESSONS.md` + `ui/LESSONS.md` (prose) + each area's `CLAUDE.md` (index) | On-demand |
| Architecture / design contract | `ARCHITECTURE.md` | On-demand via `/check-arch` |
| Workflow rules | `docs/orchestrator-briefing.md` (Step-9 matrix canonical); this doc | `/orchestrate-start` |
| `/tdd` brief format | `docs/tdd-brief-template.md` | Via `/orchestrate-start` |
| Universal scaffolding pattern | `SCAFFOLDING-GUIDE.md` (in the source scaffolding repo) | Reference only; not loaded per session |

The principle: **single source of truth per concern.** Drift between sources is a bug.

---

## How to evolve this scaffolding

- **New slash command** → file in `.claude/commands/` + add to root `CLAUDE.md` "Slash commands" + reference in `docs/orchestrator-briefing.md`.
- **New subagent** → file in `.claude/agents/` + `.claude/agents/README.md` inventory + area `CLAUDE.md` "Subagents".
- **New lesson** → next anchor in the area's `LESSONS.md` (`daemon/LESSONS.md` or `ui/LESSONS.md`) + row in that area's `CLAUDE.md` index. Hot-routed at Step 9.
- **New convention** → entry in the area's `CLAUDE.md` Forbidden patterns or root `CLAUDE.md` Key safety rules + a `LESSONS.md` entry if durable.
- **New cross-doc invariant** → row in the area's `CLAUDE.md` table + atomic `ARCHITECTURE.md` edit.
- **New escalation category** → root `CLAUDE.md` "Escalation taxonomy" + `docs/team-protocol.md` "What the lead does NOT do" cross-reference.

**Don't** add project *state* to scaffolding files — state lives in `MVP_TASKS.md`. **Don't** rename the cross-referenced files (`MVP_TASKS.md`, the `CLAUDE.md` files, `daemon/LESSONS.md` + `ui/LESSONS.md`, `docs/team-protocol.md`, `docs/orchestrator-briefing.md`, `docs/tdd-brief-template.md`) casually — they're named inside slash command bodies; renaming is a multi-file ripple.

---

## Limits / known gaps

1. **Cross-team channel-bleed is a real failure mode** — the track-prefix naming rule + ignore-mismatched-prefix posture mitigate it but don't fully eliminate it.
2. **Documentation drift between a lesson and the code it governs is only audit-caught** — the cross-doc invariants table catches model↔spec drift mechanically; lesson↔code drift is not.
3. **Subagents aren't sandboxed** — their forbidden-patterns section is the only guard.
4. **HITL chokepoints stay HITL** — deploys, scope cuts, load-bearing architectural decisions, push approvals keep the user in the loop.
5. **The brief-drafter subagent (if installed) requires a quality trial before standard adoption** — briefs are load-bearing.

See `SCAFFOLDING-GUIDE.md §12` for the full list.

---

**End of scaffolding reference.** For the universal pattern, see `SCAFFOLDING-GUIDE.md`.

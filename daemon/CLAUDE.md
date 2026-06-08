# NexusOps `daemon/` — Build Guide

> **You're in `daemon/`.** This file plus root `CLAUDE.md` both load. The root file covers global project conventions + shared comm rules (track-prefix, escalation taxonomy, messaging budget); this file owns code-area conventions for the Rust daemon (trust core).

## Launch protocol

| Working on... | cwd | Loads |
|---|---|---|
| Planning / docs / commits | repo root (`NexusOps/`) | root `CLAUDE.md` only |
| the Rust daemon (trust core) code | `daemon/` | this `CLAUDE.md` + root |

<!-- For a multi-area project, add a row per additional code area. -->

If you find yourself fighting the wrong conventions, check your cwd.

## Session start/end protocol

**At session start:**
1. Read `MVP_TASKS.md` (repo root) **by section, not whole** — `grep -n "^##" MVP_TASKS.md` for offsets, then Read with offset/limit just "Currently in progress" + the active phase. (The file grows; never load it whole.)
2. Confirm with the user what feature this session is targeting.
3. Read the relevant section of `ARCHITECTURE.md` from the lookup table below.

**At session end** (only when the user explicitly says we're done):

1. **Implementer runs `/session-end`.** Implementer writes ONLY:
   - `daemon/` code files (the slice's implementation)
   - test files (the slice's tests)
   - dependency manifest / lockfile (deps the slice adds)
   - `docs/sessions/<NNN>-<date>-<topic>.md` (session doc, created at `/session-end` Step 5)

   **Implementer must NOT touch (all orchestrator territory):**
   - `MVP_TASKS.md`
   - `daemon/LESSONS.md`
   - `daemon/CLAUDE.md` (entire file — both the Cross-doc invariants table AND the Lessons logged index)
   - `ARCHITECTURE.md`
   - `docs/orchestrator-briefing.md` / `docs/tdd-brief-template.md` / `docs/briefs/` / `docs/runbooks/`
   - other top-level deliverable / design docs
   - `.gitignore` and root-level dotfiles (unless adding a new artifact to ignore, flagged at Step 9)

   At Step 10: **explicit `git add <path>` per slice file; never `git add -A`/`.`; never stage an orchestrator-territory file.** Changes to any orchestrator-territory file (a new cross-doc model, a lesson, an arch note) are **flagged at Step 9**, not edited here — the orchestrator writes them hot (root `CLAUDE.md` + the Step-9 matrix).

2. **Orchestrator runs `/orchestrate-end`** for round close-out + Carry-forward triage + round terminal commit + push.

## Lookup table — where to find canonical info

Don't paste these sections into the prompt. Grep the file:section, read only what you need. `/check-arch <topic>` dispatches off this table.

| Topic | File (relative to repo root) | Section |
|---|---|---|
| Action Gateway / mutation invariant | `ARCHITECTURE.md` | §X |
| Lessons logged (full prose) | `daemon/LESSONS.md` | by lesson # |

<!-- Starts near-empty. Add a row whenever a topic is looked up twice. Populate as the project accretes. -->

**Code intelligence & docs (when available):** prefer a code-intelligence MCP / docs MCP over grep+read loops — see root `CLAUDE.md` "Code intelligence & docs."

## Stack

<!-- ▼ EXAMPLE BLOCK [id=area-stack]: stack quick-reference for implementer sessions. Canonical stack lives in root CLAUDE.md + ARCHITECTURE.md; this is the cheat sheet. ▼ -->

- **Runtime:** Rust (stable, edition 2021) · cargo.
- **Framework:** Tokio async runtime — rusqlite (event store + projections), portable-pty (agent harness), git2 (read-only repo introspection), octocrab (GitHub), keyring (secret refs), rmcp (MCP host).
- **Validation:** serde + serde_json (schemars for JSON-RPC schemas).
- **Lint / types / tests:** clippy / rustc via `cargo check` / `cargo test`.
- **Role:** this is the **trust core** — the sole DB writer and the sole mutator of all state. Every change is a typed, risk-classified, approved Action recorded as an immutable event; agents, the UI, and the Project Brain only *propose* intents to this daemon.

<!-- ▲ END EXAMPLE BLOCK [id=area-stack] ▲ -->

## Standard commands

```bash
# Install deps (run once; re-run when the manifest changes)
cargo fetch

# Run the dev server (if applicable)
cargo run -p nexusopsd

# Tests
cargo test

# Quality
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo check --all-targets

# Preflight (use before saying "done" with a feature)
# fmt-check is FIRST — /tdd Step 8 was clippy-only, which let 0.5 (06f9576) land unformatted (needed style follow-up 407be7c). fmt is part of the gate.
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo check --all-targets && cargo test
```

## TDD protocol

**Write the failing test first.** Applies to deterministic code — see the TDD posture in root `CLAUDE.md` for what is test-first vs. exempt.

**Commit per slice when practical.** Never bundle a safety-critical slice with anything else.

## Forbidden patterns

<!-- ▼ EXAMPLE BLOCK [id=forbidden-patterns]: forbidden patterns — 3-5 narrow, enforceable, domain-specific rules. Shape: "Don't <pattern X> because <reason / past incident>; use <alternative Y>." Test-pin them where possible. Starts small; accretes as lessons surface. ▼ -->

Do not:

1. **Write code without a failing test first** (for deterministic code). Even one-line functions.
2. **Mutate state outside the Action Gateway** — the daemon is the single audited mutator (INV-SEC-1); every change must flow through a typed `ActionRequest` so it is risk-classified, approved, and recorded as an immutable event. Never reach around the Gateway to flip state directly; submit an `ActionRequest`.
3. **Write `nexusops.db` from anywhere but the single daemon write-actor** — there is exactly one writer task. Every other path (UI projections, queries, edge modules) opens a **read-only WAL connection**. Concurrent writers corrupt the event log / break ordering; route all writes through the write-actor.
4. **Scrape PTY output to infer agent status** — terminal bytes are display-only and lie about lifecycle. Read status from the SDK event stream / Codex app-server push; never regex the PTY to decide an agent's state.
5. **Put secrets in events, payloads, logs, or rows** — persist **keychain references only**, and run the Redactor over any payload before it is persisted or emitted (INV-SEC). A secret in the immutable event log cannot be unwritten.
6. **Use git2 for mutations** — git2 is **read-only** (repo/branch/diff introspection). All worktree, branch, commit, and merge operations go through the **git CLI as Gateway actions** so they are typed, approved, and audited. Never call a git2 mutating API.

<!-- ▲ END EXAMPLE BLOCK [id=forbidden-patterns] ▲ -->

## Cross-doc invariants — schema/docs mirroring

Several typed models in this codebase are **contracts** mirrored in `ARCHITECTURE.md` and indexed in the table below. The architecture doc is the canonical contract; the model is the executable enforcement. Drift produces silent disagreement.

**Authoring discipline (orchestrator owns this table).** The implementer never edits this table or `ARCHITECTURE.md` directly — it flags a field add/remove/rename at Step 9 as a `Cross-doc invariant change`; the orchestrator writes the row + the arch edit hot the same round (see root `CLAUDE.md` + `docs/orchestrator-briefing.md`). Commits stagger; the working tree stays aligned within the round.

| Model | `ARCHITECTURE.md` section | Notes |
|---|---|---|
| Status state machines (10 total) | §5.1 / Appendix A | **9 frozen in `shared/` (0.5):** Session(17), Task, Worktree(2-axis), PullRequest, WorkflowInstance, ProjectBrain, Approval(10), ActionRequest(15), AgentTeam. Wire = snake_case `TEXT`. The 10th, **ExecutionProfile, HELD for 0.5b** (cat-4 SDK-vs-PTY). |
| 22 shared IDs + prefix map | §5.2 / Appendix A | **Frozen in `shared/` (0.5)** as prefixed-ULID newtypes. 16 platform: `ws_ proj_ repo_ wt_ sess_ team_ prof_ pack_ wfi_ cmd_ plan_ task_ act_ evt_ artf_ evid_`; 4 desktop: `dev_ rc_ lr_ eprj_`; 6 external = native. `parse()` fail-closes on wrong-prefix/bad-ULID (§15). |
| Actor enum (R-2) | §7.1 / §5.3 / Appendix A | **Frozen in `shared/` (0.5).** 10 values incl. `remote_client` (legacy EM §7 `remote_device` superseded). |
| Desktop-addendum objects (4) | §5.3 / Appendix A | **Frozen in `shared/` (0.5).** LocalRunner + EventProjection (MVP-live); Device + RemoteClient (deferred iOS scaffolding). |
| Contract SoT mechanism | §5.0 | Option A: Rust authority → schemars JSON-Schema (`shared/contracts/schema/`, versioned, diff-gated) → generated Zod/Pydantic. The pattern for every future contract addition. |
| Event envelope + enums | §7.1 / Appendix A | **Frozen in `shared/` across 1.1: envelope + `source_type`(15, **closed**)/`sensitivity`(5)/`visibility`(4) @ 0.6.0 (df753aa); `redaction_status`(unredacted\|redacted, §15) + `redaction_engine_version` @ 0.7.0 (redaction commit).** EventEnvelope + ObjectRef; `deny_unknown_fields` (reject-unknown); `redaction_status`/`redaction_engine_version` = new DATA_MODEL §2.1 columns; the writer **fail-closes** (never persists `redaction_status='unredacted'`); `causation_id` = `EventId`. |
| `ActionRequest` | §6.2 | typed mutation envelope; risk class + approval state. Lands Phase 2 (2.1). |

<!-- Populated as contract models land. The four 0.5 rows above are the frozen foundation (the serial neck). -->

## Module organization

<!-- ▼ EXAMPLE BLOCK [id=module-layout]: module layout + layer dependency rule. Replace with the project's real directory tree and import-direction DAG. ▼ -->

```
daemon/
  model/          # shared typed domain (Action, Event, RiskClass, projections schema) — depended on by everything
  eventstore/     # append-only event log writer (the single write-actor) + read-only WAL readers
  projections/    # derived read models rebuilt from the event log
  locks/          # resource/lease arbitration for the persistence core
  gateway/        # the Action Gateway — orchestrates + risk-classifies + approves; SOLE mutator
  harness/        # agent process supervision (PTY/SDK lifecycle) — submits intents to gateway
  terminal/       # portable-pty session handling — submits intents to gateway
  git/            # git2 read-only introspection + git-CLI actions — submits intents to gateway
  integrations/   # octocrab (GitHub) + external adapters — submit intents to gateway
  brainclient/    # Project Brain proposal client — submits intents to gateway
  workflow/       # multi-step plays composed of gateway actions
  ipc/            # exposes GatewayPort over a Unix domain socket (UDS)
```

Layer dependency direction (top depends on bottom, never reverse):

```
ipc                                            (edge: exposes GatewayPort over UDS)
harness · terminal · git · integrations ·      (executor/adapter edges:
  brainclient · workflow                        submit intents to gateway; NEVER write the DB)
        │  (propose intents)
        ▼
gateway                                         (sole mutator: orchestrates, risk-classifies, approves)
        │
        ▼
eventstore · projections · locks               (persistence core: single write-actor + read-only readers)
        │
        ▼
model                                           (shared typed domain — depended on by ALL, depends on none)
```

**No edge module writes the DB directly** — harness/terminal/git/integrations/brainclient/workflow/ipc are edges that submit intents to `gateway`; only the `eventstore` write-actor (driven by `gateway`) touches `nexusops.db` for writes. Cross-cutting layers can be imported from anywhere. Enforce the import direction mechanically with a test where possible — the test *is* the spec for the rule.

<!-- ▲ END EXAMPLE BLOCK [id=module-layout] ▲ -->

## Subagents

See `.claude/agents/README.md` for the canonical inventory + integration points.

<!-- ▼ EXAMPLE BLOCK [id=area-subagent-candidates]: area-specific subagent candidates — list candidates that would earn their keep specifically in this area (e.g. an ABI/types syncer for a frontend area, a Pyth/feed verifier for a contracts area). Build only on real friction. ▼ -->

<!-- ▲ END EXAMPLE BLOCK [id=area-subagent-candidates] ▲ -->

## Lessons logged from prior sessions

The full prose for each lesson lives in `daemon/LESSONS.md`. This index is the compact orientation surface.

**Lesson numbers are stable IDs** — once assigned, they don't change. New lessons get the next sequential number. `/session-end` proposes additions when it detects them; the user approves before the entry is written and a row is added here.

Lessons start at §1.

| # | Date | Topic | Rule (one-liner) |
|--:|---|---|---|
| 1 | 2026-06-07 | [Broken cargo shims](LESSONS.md#1) | `rustup default stable` won't fix *broken* (vs missing) `~/.cargo/bin` proxies — repoint each to the `rustup` binary, then verify with plain shims |
| 2 | 2026-06-07 | [Wire value is the contract; SoT = §5.0](LESSONS.md#2) | Freeze the snake_case wire value (not the identifier; pin with a round-trip test); author every contract in Rust `shared/` per §5.0, regenerating the published schema + consumers — never hand-write a consumer or invert the authority |
| 3 | 2026-06-07 | [Single-write-actor + atomic seq](LESSONS.md#3) | One writable `Connection` (write-actor); all else read-only WAL (`open_read_only`); assign canonical `seq` via `SELECT max+1`+`INSERT` in one `BEGIN IMMEDIATE` txn (atomic, not borrow-checker-only); inject `Clock`+`IdGen` for deterministic replay |

<!-- Starts empty. Each row links to its `LESSONS.md` anchor. Populate as the project accretes. -->

<!-- Slash commands: see root CLAUDE.md "Slash commands available." Implementer pair: /session-start + /session-end. -->

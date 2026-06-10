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
| Desktop-addendum objects (4) | §5.3 / Appendix A | **Frozen in `shared/` (0.5).** LocalRunner + EventProjection + **Device (desktop-host) MVP-live** — Device registered at cold-start §16 (register-if-absent; user-ruled Option A 2026-06-10; `is_deferred()` true only for RemoteClient). RemoteClient + the iOS multi-device/pairing dimension of Device stay deferred. `dev_`/`lr_` ids via `desktop_minted_id!` off `DesktopObjectKind` (frozen-22 untouched). |
| Contract SoT mechanism | §5.0 | Option A: Rust authority → schemars JSON-Schema (`shared/contracts/schema/`, versioned, diff-gated) → generated Zod/Pydantic. The pattern for every future contract addition. |
| Event envelope + enums | §7.1 / Appendix A | **Frozen in `shared/` across 1.1: envelope + `source_type`(15, **closed**)/`sensitivity`(5)/`visibility`(4) @ 0.6.0 (df753aa); `redaction_status`(unredacted\|redacted, §15) + `redaction_engine_version` @ 0.7.0 (redaction commit).** EventEnvelope + ObjectRef; `deny_unknown_fields` (reject-unknown); `redaction_status`/`redaction_engine_version` = new DATA_MODEL §2.1 columns; the writer **fail-closes** (never persists `redaction_status='unredacted'`); `causation_id` = `EventId`. |
| `ActionRequest` | §6.2 | typed mutation envelope; risk class + approval state. Lands Phase 2 (2.1). |
| Event-type registry (`EventTypeRegistry`) | §7.1 / Appendix A | **`SessionStarted` = first entry (1.2, `shared/src/events.rs`); CONTRACT_VERSION → 0.8.0. `DeviceRegistered` + `LocalRunnerRegistered` added 1.6a-L3** (System-actor cold-start registration via the write-actor; `actor_type=System`; reserved `SYSTEM_WORKSPACE_ID` sentinel); **CONTRACT_VERSION → 0.12.0**. **`AuditIntegrityViolation`{seq,reason} added 1.6c** (§17 degradable-replay quarantine record; System-actor; **CONTRACT_VERSION → 0.13.0**). **`SensitiveOutputRedacted`{original_event_type,reason,detector} added 1.7** (§15 redactor quarantine-divert record; content-free; sensitivity Internal; **CONTRACT_VERSION → 0.14.0**; 3-way verify @ 0.14.0). Per-type payload schema, contract-first per §5.0 (schemars → schema → Zod/Pydantic); the golden-log tests bind. Accretes per phase as event types land. |
| MVP projections (10 `proj_*`) + `object_refs` + `projection_offsets` | §7 / §7.2 / DATA_MODEL §2.2–§2.4 | **Implemented 1.2** (`daemon/src/projections/`). Folded **in-band in the event-commit txn**; offsets advance same-txn (never ahead of rows); **status columns bind the frozen §5.1 enums** (reject-unknown). Projector BODIES = 4 Phase-1-feedable (Session/ProjectGraph/AuditTrail/ProjectActivity); 6 re-homed to producing phases (all 10 DDLs present). LESSONS §4. |
| `outbox` (transactional outbox) | §7 / §12 / §17 / DATA_MODEL §2.5 | **Implemented 1.3** (`daemon/src/eventstore/outbox.rs`). **Daemon-internal** — `out_` id is **NOT** one of the 22 frozen contract IDs; `destination`/`status` enums are **not** a `shared/` contract (UI/Brain don't read the outbox). Rows written in the event-commit txn; the **§15 *sync* sink** (payload from the already-redacted event); at-least-once drainer (Tokio spawn 1.6-wired). LESSONS §5. |
| `Lease` (lease locks + fencing) | §5.1 / §7.2 / §17 / Appendix A | **Implemented 1.4** (`daemon/src/locks/`). One row per `(resource_id, lease_kind)`: `owner_id, fencing_token, acquired_at, heartbeat_at, expires_at`. **Daemon-internal** — no `proj_lease`, no `lease_` among the 22 frozen IDs, **not** one of the 10 §5.1 status machines (UI/Brain don't read `leases`) → **no `shared/` surface, no CONTRACT_VERSION bump** (outbox-analogous). Fencing token = **persisted monotonic high-water mark** (acquire/reclaim mints +1 under `BEGIN IMMEDIATE`; release/reap keep it; survives restart). **Authority = a LIVE lease** (Option B, §17): `validate_held` = owner + token-match + `expires_at > now`; "stale" = expired **OR** superseded → `fencing_conflict` (safety rule #6, never auto-resolved). pidlock single-instance via a std advisory file lock; reaper `reap_once` (Tokio spawn 1.6-wired). LESSONS §6. |
| IPC `GatewayPort` wire contract | §6.1 / §6.4 / §5.0 / Appendix A | **Implemented 1.5** (`daemon/src/ipc/` + `shared/src/ipc.rs`). **Cross-language `shared/` contract** (the ui's gateway-client consumes it) — HelloFrame/HelloAck/VersionSkewError + `IpcErrorCode` (7: incl. **`protocol_error`**) + the closed `ProjectionName` enum (`UsageLedger`,…) + RpcRequest/RpcResponse + **`ServerFrame`** (`rpc_response`\|`subscription_push`; Terminal reserved) + ProjectionDelta/DeltaKind/SubscribeParams + Capabilities → **CONTRACT_VERSION 0.11.0** (byte-diff gate + 3-way verify). **Two version axes:** `CONTRACT_VERSION` (schema/codegen) vs **`protocol_version`** (wire handshake; `SUPPORTED_PROTOCOL_RANGE {min:1,max:1}` daemon-authored). §6.1 **READ surface live** (get_projection/subscribe/get_capabilities); mutation methods → P2. `getpeereid` peer-auth (rule #7); read methods over **read-only WAL** (single-writer). **Runtime LANDED 1.6b** (`main.rs` `#[tokio::main]` + write-actor + accept-loop + broadcast subscribe-source; reads + subscribe-SOURCE live) **+ subscribe-SERVE push LANDED 1.6d** (`93f70a5`; the broadcast Receiver→socket push, a `next_push_action` classifier [matching→Push, other→Skip, Lagged→**Stop=close-on-lag**, Closed→Stop], a **structurally-dedicated single-writer** subscribe connection [post-ack main loop read-only, sole push thread/conn]; closes the ui live-subscribe path). LESSONS §7 + §9 + §12. |
| `quarantine` (degradable replay, §17) | §17 / DATA_MODEL | **Implemented 1.6c** (`daemon/src/eventstore/`). **Daemon-internal** — MIGRATION_6 `quarantine` table (`seq` PK, structural reason, `detected_at`, `audit_emitted`); SUPPORTED_USER_VERSION 5→6; **no `shared/` surface, no CONTRACT_VERSION bump** (outbox/leases-analogous). The replay read path (shared by `catch_up_replay` + `rebuild`) quarantines a corrupt/unredacted row (raw spine untouched) + marks the offset degraded + continues (Option C, daemon starts); `ON CONFLICT(seq) DO NOTHING` preserves the record across rebuild/re-read. Drives the `AuditIntegrityViolation` event (the CONTRACT-bearing half — EventTypeRegistry row above). LESSONS §11. |

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
| 4 | 2026-06-07 | [In-band projections + offsets + recovery](LESSONS.md#4) | Fold projections in-band in the event-commit txn (after the redaction gate), each under its own SAVEPOINT (logic error → degrade+skip; Db error → fail closed); advance offsets in the same txn (never ahead of rows); recover via catch-up replay (strict `seq>last_seq`, no-op on current) + full rebuild (truncate a const derived-table list, replay-all, byte-equivalent, raw events untouched); bind status columns to the frozen §5.1 enums |
| 5 | 2026-06-08 | [Transactional outbox + §15 sync-sink](LESSONS.md#5) | Route external side-effects through the outbox: write rows in the event-commit txn (recorded-iff-intended); §15 sync sink (payload from the already-redacted event, filter-only; rebuild never re-emits); deliver at-least-once via deterministic `drain_once` (in_flight claim-before-deliver, reset-on-open, backoff + retryable/terminal + bounded dead-letter, idempotent consumers); UTC-`Z` timestamps for the lexical due-compare |
| 6 | 2026-06-08 | [Cross-restart leases + live-lease fencing + pidlock](LESSONS.md#6) | `leases` row per `(resource_id, lease_kind)` with a **persisted monotonic fencing high-water mark** (minted +1 on acquire/reclaim under `BEGIN IMMEDIATE`; release/reap keep it; survives restart); **authority is a LIVE lease** — `validate_held` = owner + token-match + `expires_at > now`, so "stale" = expired OR superseded (safety rule #6; gateway heartbeats long actions); single-instance via a **std advisory file lock** (OS-fd-held, auto-released on death, immune to PID reuse — never `kill(pid,0)`); daemon-internal (no `shared/` surface, no CONTRACT_VERSION bump) |
| 7 | 2026-06-08 | [UDS GatewayPort transport](LESSONS.md#7) | UDS server: 4-byte-BE-len+JSON framing (oversized rejected pre-alloc; `MAX_FRAME_SIZE` 8 MiB); **`getpeereid()` peer-auth FIRST** (reject uid≠daemon-uid; NOT `SO_PEERCRED`; isolated unsafe + fail-closed; test the *enforcement* path via an injectable `peer_uid`); handshake-first; **two version axes** (`CONTRACT_VERSION` schema/codegen vs `protocol_version` wire-skew `{1,1}` daemon-authored); LOCKED-error-set gap → escalate (`protocol_error`); frame-mux = internally-tagged `ServerFrame` over the **unchanged** codec (Terminal encoding = Phase-3 measured); read-only-WAL reads + closed-enum table map (no injection); ship the mechanism, wire the runtime source at 1.6 |
| 8 | 2026-06-08 | [Cold-start bootstrap ordering + FS-failure determinism](LESSONS.md#8) | Cold-start = `create_dir_all` → `PidLock` (first gate, strictly before any DB write — forbidden #3) → `EventStore::open` (compose the already-shipped migrate/backup-rollback/floor/replay; don't re-implement); the §16 version matrix enforces ONLY the DB `user_version` floor at bootstrap (rest = report-only/handshake/deferred); pin an FS-failure path via a pure `on_migration_failure` classifier + inline unit (preserve the original error context; distinct typed `RestoreFailed`; no `-1` sentinels) — not a forced disk fault |
| 9 | 2026-06-08 | [Daemon runtime — write-actor + publish-after-commit + unlink-before-bind](LESSONS.md#9) | Runtime = single write-actor (dedicated **blocking thread** + mpsc + a Clone+Send `WriteHandle`; reads always `open_read_only`, never the actor — forbidden #3); drainer/reaper = async interval tasks sending **bounded** commands (`MissedTickBehavior::Delay`); live subscribe = broadcast publishing **AFTER commit**, a lagging subscriber resyncs + NEVER back-pressures the writer (no wire frame / no CONTRACT bump); UDS bind = **unlink-before-bind** (pidlock makes a stale socket safe) + `spawn_blocking(serve_connection)` under a semaphore cap (pin rejection AND permit-release); `main.rs` `#[tokio::main]` closes the 1.3/1.4/1.5 runtime-deferral chain |
| 10 | 2026-06-10 | [Daemon self-registration = a System-actor event, not a Gateway Action](LESSONS.md#10) | The daemon's own cold-start lifecycle identity (desktop-host `Device` register-if-absent + per-start `LocalRunner`) is a **System-actor system event** (`actor_type=System`, `SYSTEM_WORKSPACE_ID` sentinel) appended via the write-actor — through the §15 redaction gate + projector fold (audited + immutable) but **NOT policy-gated** (INV-SEC-1 governs untrusted proposer intents, not the substrate those Actions record into; the Gateway is Phase 2). Desktop ids (`dev_`/`lr_`) mint via a `desktop_minted_id!` sibling keyed off `DesktopObjectKind` — frozen-22 `IdKind` untouched (`from_prefix==None`); object_refs payload-sourced (rebuild-safe). Binding §16 beats a stale frozen §5.3 row → escalate the reconcile (Device → MVP-live, Option A). |
| 11 | 2026-06-10 | [Degradable catch-up replay — quarantine + audit-integrity, Option C](LESSONS.md#11) | The replay read path (shared by `catch_up_replay` + `rebuild`) **degrades instead of aborting `open()`** on a bad event row so a single corrupt row can't stop the daemon (§17 Option C): corrupt (reconstruction-fail) **or** unredacted → **quarantined** (daemon-internal `quarantine` table, raw spine untouched, offset degraded, row skipped) + a loud `AuditIntegrityViolation` System-actor event per newly-quarantined seq (emitted post-`open` from the caller; structural/content-free reason §15; exactly-once across restart **and** rebuild via `audit_emitted` + idempotency_key + `ON CONFLICT(seq) DO NOTHING`); unknown `event_version` → **degraded-only** (Degraded≠Quarantined — forward-compat, not integrity). Healthy logs replay byte-identically (rebuild-equivalence preserved). |
| 12 | 2026-06-10 | [Socket subscribe-SERVE — close-on-lag resync + dedicated single-writer connection](LESSONS.md#12) | A push-stream over a shared socket must be **structurally single-writer**: make a subscribe connection **terminal/dedicated** (write the ack on the main thread, spawn the *one* push thread only on ack-success, main loop read-only-until-EOF after) — concurrent ack/RPC writes + push writes otherwise interleave + corrupt the frame stream (a HIGH). Mint the broadcast receiver **before** the ack (don't miss a just-after-ack delta). Drive the per-recv decision through a **pure `next_push_action` classifier** (Push/Skip/Stop) so it's unit-testable without timing flakiness; **`Lagged → Stop → close the connection`** IS LESSON §9's resync trigger (client reconnects + re-`get_projection`) — never silently continue (`ProjectionDelta` has no seq → a gapped stream is undetectable → silent divergence). Subscriber never back-pressures the writer (forbidden #3); `getpeereid` stays first (rule #7, post-auth/post-ack). |
| 13 | 2026-06-10 | [§15 redactor — entropy fallback + mask-in-place; quarantine-divert as the MVP-unreached net](LESSONS.md#13) | Secret detection = high-recall prefix set + Shannon-entropy on `KEY=value` (base64/quote-aware value span, **sub-run-scored** to resist padding-dilution) + bare-run masking — all **mask-in-place** (fail-closed, deterministic/golden-log-safe; engine `prefix-entropy-v2`). The quarantine→`SensitiveOutputRedacted` **divert** (original event diverted, content-free record in its place — namespaced `divert-{k}` dedup, sensitivity-reclassified Internal, fail-closed no-recurse guard) is the §15 "can't-safely-bound → divert" net, **wired + `ForcesQuarantine`-tested but MVP-unreached** (the real redactor masks everything) — §17-AIV-analogous. Prefer **mask-in-place over divert** when the span is boundable (a false divert loses the whole event; a false mask loses only the blob). A statistical detector has an inherent recall envelope (JSON-value short secrets, hex≈SHA, adversarial pad) — the §15 **gate** invariant holds regardless; recall acceptance is a human gate; the primary control is keychain-refs-only (rule #5). |

<!-- Starts empty. Each row links to its `LESSONS.md` anchor. Populate as the project accretes. -->

<!-- Slash commands: see root CLAUDE.md "Slash commands available." Implementer pair: /session-start + /session-end. -->

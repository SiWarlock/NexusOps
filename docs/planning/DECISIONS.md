# DECISIONS — NexusOps Architecture ADRs

> **Phase 11–12 output.** ADR-style decision log for the **load-bearing technical** decisions. Product-level decisions (Session-as-atomic-unit, Gateway-as-chokepoint, events-immutable, Brain-plans/platform-executes, Workflow-Packs-optional, etc.) are already locked in `PRODUCT_CANON.md §17` and the subsystem specs — those are **inherited, not re-litigated here**. This file records the decisions the existing docs left **open** (`PRD.md §21`), informed by `RESEARCH.md` and confirmed with the user on **2026-06-06**.
>
> **Status key:** `Locked` (confirmed by user or VERIFIED-with-no-blocker) · `Locked-pending-spike` (locked direction, one validation owed before build) · `Proposed` (recommended, not yet confirmed).
> **Tags:** decisions feed `ARCHITECTURE_DRAFT.md` anchors; reference `RESEARCH.md` cluster IDs for evidence.

---

## Locked Decision Summary

| ADR | Area | Decision | Status | Fallback |
|---|---|---|---|---|
| 001 | Desktop stack | **All-Rust: standalone Rust daemon + Tauri 2.x shell**, macOS-only MVP | Locked | Electron/Node stack (R-STACK) |
| 002 | Process topology | **Detached long-lived daemon from day one** owns PTYs/store/Gateway/adapters/Brain; UI is a reattaching client | Locked | n/a (chosen over in-process) |
| 003 | Event store | **SQLite (WAL)**, single-writer = the daemon; FTS5 + projections + outbox + projection_offsets | Locked | + append-only JSONL mirror |
| 004 | Action Gateway transport | **`GatewayPort` interface over Unix-domain-socket IPC** (daemon↔UI), length-prefixed JSON-RPC; in-daemon execution | Locked | named-pipe/loopback-HTTP+token |
| 005 | Brain seam | **stdio MCP sidecar**, daemon-owned lifecycle; PyInstaller bundle | Locked-pending-spike | streamable-HTTP+loopback token |
| 006 | Harness adapters | **One contract over two lifecycle models** — Claude: Agent SDK `can_use_tool` + PTY display + JSONL; Codex: `codex app-server` JSON-RPC | Locked | Claude PTY+hooks; Codex `exec --json` |
| 007 | Git + integrations + creds | **Dual git** (git2 reads / git-CLI mutations), **octocrab** + gh-token bootstrap, Linear PKCE/key, **keyring** crate | Locked | git-CLI-only; shell `gh`; pasted key |
| 008 | Cross-restart locks | **SQLite lease table** (owner + monotonic fencing token + heartbeat/expiry) + `pidlock` single-instance | Locked | n/a (OS file locks rejected) |
| 009 | Terminal capture | `portable-pty` in daemon → headless VT state → Tauri Channel → **xterm.js (WebGL)** in shell; backpressure/flow-control mandatory | Locked | — |
| 010 | Session survival policy | **Reconnect-live on UI restart** (daemon alive); **resume-or-replay on daemon restart** (`claude --resume` / `codex thread/resume`, else serialized-scrollback replay) | Locked | replay-only + relaunch |
| 011 | Credentials & signing | OS keychain via `keyring`; **Developer ID code-signing + notarization is an early release-blocker** | Locked | app-scoped encrypted file pre-signing |

---

## ADR-001 — Desktop stack: All-Rust (Rust daemon + Tauri 2.x), macOS-only MVP

**Status:** Locked (user, 2026-06-06). Informs `ARCHITECTURE_DRAFT §4, §11, §12`.

### Context
The #1 open question (`PRD.md §21`). The product is local-first, terminal-heavy, multi-process, with a security-critical mutation chokepoint and a Python Brain sidecar. User confirmed **macOS-only for MVP** (Windows/Linux post-MVP) and **detached daemon from day one** — the latter splits "the stack" into daemon-language + UI-shell-host, both hostable from one daemon.

### Options Considered
| Option | Pros | Cons | Fit |
|---|---|---|---|
| **All-Rust: Rust daemon + Tauri shell** | Memory/type-safe Gateway+store; in-process `portable-pty`; ~30–40MB idle / <10MB installer; no native-module ABI tax (rusqlite/portable-pty/git2/octocrab/keyring/rmcp compile in); daemonizes cleanly (setsid/launchd); iOS-core path; macOS-only deletes WebKitGTK risk | 3 languages (Rust+TS+Python); slower compiles; Tauri externalBin notarization open issue #11992 to validate | **Best** |
| All-Node: Node daemon + Electron | Single language (TS) daemon+UI+adapters; fastest iteration; most-proven node-pty terminal stack; biggest ecosystem | 200–400MB footprint before workload; recurring `@electron/rebuild` native-module tax; JS (not Rust) chokepoint; no iOS reuse | Strong fallback |
| Spike both | Evidence-based | Delays MVP build | Rejected (research was decisive enough) |

### Decision
**Standalone Rust daemon + Tauri 2.x UI shell; macOS-only MVP.** Linux/WebKitGTK (Tauri's only material 2026 weakness, R-STACK) is neutralized by the OS choice; the detached-daemon choice plays to Rust's strengths (clean daemonization, no ABI tax, memory-safe credential/mutation core).

### Rationale
The Rust core is the natural home for the Action Gateway and append-only event store (the two security-critical seams). Footprint matters on a box already spawning many agents + their own Claude/Codex processes + a Python sidecar. R-STACK independently flagged "Rust daemon + UI" as the ideal end-state; the daemon decision reaches it now.

### Tradeoffs
Accept Rust build velocity (offset: solo+AI build via cc-crew, where compiler strictness aids AI codegen) and a three-language system (Rust daemon / TS frontend / Python Brain — Brain is a separate product anyway).

### Fallback
Electron/Node stack (same daemon shape, `node-pty`+xterm.js, accept footprint + ABI tax + JS chokepoint). Reversal is expensive but the `GatewayPort` interface + SQLite-durable state keep the *daemon-internal* contracts portable.

### What Would Change This
Linux becoming an MVP target (re-weigh vs Electron's single Chromium target); Tauri sidecar notarization (#11992) proving unsolvable for the Python Brain on a real signed build.

---

## ADR-002 — Process topology: detached long-lived daemon from day one

**Status:** Locked (user, 2026-06-06). Informs `ARCHITECTURE_DRAFT §4, §10`.

### Context
`PRD.md DESK-7` ("local runner SHOULD survive UI reloads/restarts; running sessions recoverable/re-attachable"). R-PTY/R-PROC: PTY children die (SIGHUP) when their owner exits, so survival across UI quit requires an owner process that is **not** a child of the UI.

### Decision
A **separate, long-lived daemon** (detached via launchd/`setsid`) owns: all PTYs, the SQLite event store, the Action Gateway (sole writer), harness adapters, the git/integration workers, the lease-lock table, and the Project Brain sidecar's lifecycle. The Tauri UI is a **reattaching client** driven by projections over IPC.

### Rationale
Directly satisfies "agents keep running when the app is closed." Cleanly separates the trust-critical core from the (restartable) UI. Forces the Action Gateway behind an explicit, testable transport seam from day one (which the iOS companion will later reuse).

### Tradeoffs
Takes on the hardest part of desktop apps — daemon lifecycle (single-instance, stale-socket cleanup, UI↔daemon version skew, orphan/zombie kill). Mitigations: `pidlock` single-instance (ADR-008), versioned IPC handshake, process-group kill for the Brain sidecar, durable SQLite state so a daemon crash recovers via replay.

### Fallback
None chosen — this was selected *over* the simpler in-process model. If daemon lifecycle proves intractable, the in-process model remains a (regressive) option since `GatewayPort` + SQLite state are topology-independent.

### What Would Change This
If "agents survive UI quit" turns out not to be valued in practice, in-process would be simpler — but the user explicitly prioritized survival.

---

## ADR-003 — Event store: SQLite (WAL), single-writer

**Status:** Locked (R-STORE VERIFIED, no blocker). Informs `ARCHITECTURE_DRAFT §7, §12`; realizes `EVENT_MODEL_AND_AUDIT_TRAIL.md §12`.

### Decision
**SQLite in WAL mode**, bundled via `rusqlite`. The daemon is the **single writer** (= the Action-Gateway-as-sole-mutator discipline, ADR-004/007); the UI opens read-only connections. Schema: append-only `events` + `projections` + `projection_offsets` + transactional `outbox`, all written in one transaction. **FTS5** (external-content + triggers) for search. `user_version` migrations (`rusqlite_migration`/`refinery`). Large artifacts (transcripts, big diffs, embeddings) stored as **path + content_hash references**, never BLOBs. Reserve `payload_hash`/`previous_event_hash` columns (hash-chain is post-MVP).

### Rationale
WAL = one writer + many concurrent readers, multi-process-safe on one host (satisfied: local trust boundary). Single-writer is a *feature* — it is the chokepoint. Event-sourcing-on-SQLite is a standard, well-documented pattern. Projections are rebuildable; corrupt projection → drop & replay without touching raw events (`EVENT_MODEL §13.2`).

### Discipline (from R-STORE)
`busy_timeout` ~5–15s; `BEGIN IMMEDIATE` for writes; `synchronous=NORMAL`; scheduled `wal_checkpoint(TRUNCATE)` to prevent checkpoint starvation under long-lived UI readers. Honor `EVENT_MODEL §10.10`: do **not** persist raw terminal output as first-class events (volume + sensitivity) — store references/summaries.

### Fallback
Add an append-only JSONL mirror for fail-closed audit durability/export (cheap belt-and-suspenders). Rejected alternatives: DuckDB (OLAP, slow small writes), redb/sled (no SQL/FTS/inspectability), libSQL/Turso (cloud-oriented sync, not production-ready).

### What Would Change This
Heavy concurrent multi-agent write contention making single-writer a bottleneck (quantify; `BEGIN CONCURRENT` is still experimental — not a relied-upon mitigation).

---

## ADR-004 — Action Gateway transport: `GatewayPort` over Unix-domain-socket IPC

**Status:** Locked. Informs `ARCHITECTURE_DRAFT §6, §10`; realizes `ACTION_GATEWAY.md §25`.

### Context
ADR-002's detached daemon means the UI is out-of-process, so the Gateway **must** be reachable over IPC from day one (the in-process MVP option from R-PROC is off the table once the daemon is detached). `ACTION_GATEWAY.md §25` leaves transport open.

### Decision
Define the Gateway as a **`GatewayPort` interface**; implement execution **inside the daemon** (the single SQLite writer). Transport between UI/clients and the daemon is a **Unix domain socket** (OS-enforced peer auth via filesystem perms / `SO_PEERCRED`; no exposed TCP port — best for the local trust boundary), carrying length-prefixed/newline-framed JSON-RPC. Agents and the Brain submit **intents** that flow *into* the Gateway; they never mutate directly (preserves the single chokepoint). MCP tool calls from agents/Brain are *producers of intents*, not the Gateway transport itself.

### Rationale
UDS beats localhost-HTTP for same-machine IPC (a loopback port is reachable by any local process; UDS gives kernel peer auth and no port). The `GatewayPort` interface keeps the transport decoupled from Tauri and from a future iOS/daemon-multi-client world (escalate to named pipe on Windows / loopback-HTTP+token only if a non-UDS client forces it).

### Fallback
Windows named pipe (with explicit DACL) when Windows is added; loopback-HTTP + per-launch bearer token only if a non-native client (iOS bridge) requires it.

### What Would Change This
A second concurrent client needing the chokepoint with a transport UDS can't serve.

---

## ADR-005 — Project Brain seam: stdio MCP sidecar

**Status:** Locked-pending-spike (macOS notarization). Informs `ARCHITECTURE_DRAFT §6, §13`; realizes `PROJECT_BRAIN_INTERFACE.md §9`. **Brain scope = integration seam only** (user, 2026-06-06).

### Decision
Run Project Brain as a **managed stdio MCP sidecar** owned by the daemon: bundle CPython via **PyInstaller**, spawn via `rmcp` (`TokioChildProcess`), supervise with MCP `ping` + process-group kill (`command_group`) + backoff restart. Brain **tools = Action-Gateway proposals/queries** (never direct writes); Brain feeds the event store via an adapter that translates **MCP resource-update / list-changed notifications** (and the experimental Tasks primitive) into events. Opens **no local port**. Pin **FastMCP 3.x** as the contract.

### Rationale
Matches FastMCP's default + intended desktop model; no listening socket (trust boundary); official Rust client (`rmcp` 1.7) exists; reuses the same stdio-JSON-RPC idiom as the Codex `app-server` adapter (shared framing/supervision layer).

### Pending spike (owed before build)
Validate **macOS codesigning + notarization of the bundled PyInstaller sidecar in a real signed Tauri build** (entitlement `com.apple.security.cs.allow-unsigned-executable-memory`; deep-sign all bundled libs; Tauri externalBin notarization **issue #11992**). This is the sharpest packaging risk in the whole plan.

### Fallback
Flip Brain to **FastMCP streamable-HTTP on 127.0.0.1 + per-launch loopback token** (same server code) for concurrency/debug or the future iOS seam; raw newline-JSON-RPC if `rmcp` API churns. If bundling proves intractable for MVP, require a user-installed Brain CLI the daemon discovers (degrade-gracefully; Brain is optional).

### What Would Change This
Notarization #11992 unsolvable for the sidecar → require user-installed Brain, or move Brain to loopback-HTTP.

---

## ADR-006 — Harness adapters: one contract, two lifecycle models

**Status:** Locked. Informs `ARCHITECTURE_DRAFT §5, §9 (adapter layer)`; realizes `PRD.md §10.3 HARN-*`. **Both Claude Code AND Codex are MVP-blocking** (user, 2026-06-06).

### Decision
A single **`HarnessAdapter` contract** — `{ launch, stream-status, intercept-mutation, read-transcript, telemetry-heartbeat, resume }` — over two concrete lifecycle models:
- **Claude Code:** Agent SDK **streaming-input** session with the **`can_use_tool` callback as the in-harness mutation chokepoint** (allow/deny/rewrite → feeds the Action Gateway); status from the SDK message stream (`SystemMessage`→`AssistantMessage`→`UserMessage`→`ResultMessage`) + `Notification` hooks (`permission_prompt`/`idle_prompt`); a **PTY mirror for human display/takeover**; transcript JSONL (`~/.claude/projects/.../<id>.jsonl`) tailed as durable replay; telemetry merged from `ResultMessage.usage` + statusLine (`refreshInterval` heartbeat) + transcript `message.usage`. Reconcile on the shared `session_id`.
- **Codex:** **`codex app-server --stdio`** (JSON-RPC) — `thread/start{cwd}` returns the thread id (no stdout race); `thread/list?cwd=` for re-association; push status (`thread/status/changed`, `turn/completed`, `thread/tokenUsage/updated`); **host-routed approvals** (`item/commandExecution/requestApproval`) → Action Gateway; rollout JSONL (`~/.codex/sessions/...`) as forensic read, hardened to **0600** (bug #21660). Pin Codex version + regenerate the app-server schema bundle in CI on every bump.

**Cross-cutting rule:** **never scrape the PTY for machine state** (TUI/spinner is version-fragile, a spinner ≠ "working") — PTY is human display only.

### Rationale
Both harnesses now expose a structured supervision + approval surface (Claude `can_use_tool`; Codex app-server approvals) → rough parity on the two MVP-blocking axes (reliable status + reliable worktree/task association). One contract keeps the Gateway + event store agent-agnostic and ready for future adapters (`HARN-4`).

### Known gaps (→ `RISKS.md`)
Codex has **no settable session id** (key on `cwd + returned thread_id`) and **exposes no context-window %** (UI shows cumulative tokens only / estimate). Claude SDK schema + statusLine fields churn across the fast v2.1.x cadence (pin + re-verify on upgrade). Human-interactive-PTY *simultaneously* SDK-driven needs a spike (`arch-finalize`).

### Fallback
Claude: interactive PTY + hooks-to-disk + statusLine (coarser permission interception via `PreToolUse`). Codex: `codex exec --json` per task (parse `thread.started` for the id; `--cd` sets worktree; coarser polled status). Codex Cloud stays out of MVP (no-cloud constraint) but the seam stays cloud-aware.

### What Would Change This
Codex `app-server` instability forcing the `exec` fallback; Anthropic shipping a first-class daemon/headless mode that supersedes the SDK+PTY hybrid.

---

## ADR-007 — Git engine + integrations + credentials

**Status:** Locked. Informs `ARCHITECTURE_DRAFT §9 (git/worktree), §9 (integrations)`; realizes `PRD.md §10.9 GIT-*, §10.7 TASK-*, §10.10 PR-*`.

### Decision
- **Dual git backend:** `git2-rs` (libgit2) for **hot structured reads** (status/diff/log/branch/worktree-list); the **git CLI for ALL mutations** (worktree add/remove, branch, commit, merge, checkout) and libgit2-gap cases. libgit2 **can't do `extensions.relativeworktrees`** (git ≥2.48; fix unreleased) and **misreports sparse-checkout** — and since NexusOps runs git in embedded PTYs and agents make their own worktrees, libgit2 disagreeing with the user's terminal is a credibility bug. CLI mutations = one Action-Gateway chokepoint + terminal parity.
- **GitHub:** `octocrab` (typed REST + GraphQL, in-process) for issues/PRs/checks/merges; **bootstrap auth by reusing `gh auth token`** if present, else own **OAuth Device Flow**.
- **Linear:** `@linear/sdk`/GraphQL; **no device flow** → **auth-code + PKCE (loopback)** or pasted **personal API key**; 24h token refresh; budget query complexity (10k-point cap, 50-record pages). Staged link→one-way→bidirectional (inherited product decision).
- **Credentials:** the **`keyring`** crate (covers macOS/Win/Linux **and iOS** for the future companion); explicit per-OS feature flags + a **startup self-test** (v3 misconfig → silent no-op store). One `CredentialProvider` abstraction shared by GitHub + Linear + Brain.

### Rationale
Terminal parity + a single mutation chokepoint are core to a *control plane's* credibility; typed in-process reads keep projections fast; `keyring` future-proofs iOS.

### Fallback
git-CLI-only (`--porcelain=v2`/`-z`) if dual divergence bites; shell `gh --json` for octocrab gaps; Linear personal-key-only for MVP; app-scoped encrypted file store until Developer ID signing lands.

### What Would Change This
libgit2 shipping relative-worktree support (could widen git2's read scope); octocrab merge/GitHub-App ergonomics proving inadequate (spot-check owed).

---

## ADR-008 — Cross-restart resource locks: SQLite lease table

**Status:** Locked. Informs `ARCHITECTURE_DRAFT §6, §12`; realizes `ACTION_GATEWAY.md §16.2`.

### Decision
Implement resource locks (project, worktree, branch, session, agent-team, workflow-instance, integration-resource, brain-index-writer) as a **SQLite lease table**: `(resource_id, owner_id, fencing_token monotonic, acquired_at, heartbeat_at, expires_at)`. Owner-guarded atomic renew; expired-lease reclaim after crash; **fencing tokens are mandatory** (reject stale/paused holders). Project the table into the UI ("who holds this worktree/repo"). A coarse **`pidlock`** handles single-instance.

### Rationale
**OS advisory/`flock` locks do NOT survive restart** (they vanish with the fd/process) — they cannot satisfy the durable-lock requirement. The lease table lives in the SQLite store NexusOps already has, composes with the event log, and is reclaimable after crash. This is the documented lease-lock + fencing pattern (Kleppmann; Hangfire.SQLite).

### Tradeoffs
Lease expiry is probabilistic (a paused holder can exceed its lease) → fencing tokens required, not optional; acquire/renew SQL must be race-free under WAL. PID-reuse edge for `pidlock` (mitigate with start-time check).

### Fallback
None viable for the durable requirement (OS locks rejected). OS advisory locks may still guard a single live git-index operation *in addition to* the lease table.

---

## ADR-009 — Terminal capture pipeline

**Status:** Locked. Informs `ARCHITECTURE_DRAFT §10, §11`.

### Decision
The daemon owns each PTY via **`portable-pty`**; raw bytes → a **headless VT state model** (for screen snapshot + OSC/CWD tracking) → streamed over a **Tauri Channel** to **xterm.js (WebGL addon)** in the shell for display. **App-level flow control / backpressure is mandatory** (xterm.js caps ~5–35 MB/s, 50MB buffer; batch output ~30fps; honor XON/XOFF) so chatty agents can't stall the UI or OOM the event store. Per-session Windows Job Object + serialized ConPTY spawns reserved for the Windows port.

### Rationale
Validated dominant Tauri pattern (Terax/Terminon/tauri-plugin-pty). Embedded real terminals are mandatory because both agent CLIs require a real TTY (R-PTY VERIFIED).

---

## ADR-010 — Session survival policy

**Status:** Locked. Informs `ARCHITECTURE_DRAFT §7 (state), §10`.

### Decision
Two-tier survival (the VS Code model, adapted to the detached daemon):
- **UI restart (daemon alive):** sessions keep running; the UI **reconnects** and replays serialized scrollback. ✅ live.
- **Daemon restart / crash:** the agent processes do not survive; on recovery, **resume the agent** where the harness supports it (`claude --resume <id>`, `codex thread/resume <id>`), otherwise **replay serialized scrollback for context and relaunch** fresh. State/audit recovered from the event store; locks reclaimed via expired leases.

### Rationale
True live-process survival across a full daemon crash is the genuinely fragile part (R-PTY) and no library provides it; resume-or-replay gives the user continuity without betting on fragile process resurrection. Accurate alt-screen/raw-mode TUI re-attach is lossy → QA against real Claude/Codex sessions (`RISKS.md`).

### Fallback
Replay-only + relaunch (drop resume) if harness resume proves unreliable across versions.

---

## ADR-011 — Credentials handling & code-signing

**Status:** Locked (signing = early release-blocker). Informs `ARCHITECTURE_DRAFT §15, §16`.

### Decision
All secrets via the OS keychain (`keyring`); prefer reusing existing local CLI auth contexts (gh token, Claude/Codex local auth) over storing our own where possible (`PRD.md DESK-5`). **Developer ID code-signing + notarization is an early, release-blocking pipeline task** — macOS keychain ACLs prompt repeatedly without a stable signed identity, and the Brain sidecar must be deep-signed (ADR-005 spike).

### Fallback
App-scoped encrypted file store until Developer ID is set up; migrate to keychain once signing lands.

---

## Decision dependencies & open spikes (carry to `arch-finalize`)
1. **ADR-005 macOS notarization spike** (Tauri externalBin #11992 + PyInstaller deep-sign) — *before build.*
2. **ADR-006 human-PTY-vs-SDK-driven handoff** model for Claude (when does the human "own" a session vs the Gateway) — *spike in finalize.*
3. **ADR-003 write-contention quantification** under many concurrent agents — *load test before freezing single-writer.*
4. **Codex app-server schema-pin + CI regen** policy — *build-time.*
5. Objects the desktop addendum requires but the Shared Object Model lacks (`Device`, `RemoteClient`, `LocalRunner`, `EventProjection`) — *close in `DATA_MODEL.md` / finalize.*

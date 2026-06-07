# NexusOps Architecture (ROUGH DRAFT)

> **Status:** First-draft architecture spec for the MVP — **Brain 1 / `arch-draft` output**. This is a *rough draft for adversarial finalization*, NOT the binding contract. `arch-finalize` (Brain 2, Claude/Opus) runs a gap audit + adversarial scrutiny and produces the binding `ARCHITECTURE.md` at the repo root from the project's template.
> **Audience:** Project owner; `arch-finalize`; future Claude Code / cc-crew build sessions; technical reviewers.
> **Primary implementation constraint:** Solo build driven largely by AI agents (via the cc-crew workflow); optimize for a credible, demoable MVP slice (the PRD §25 scenario) over a calendar deadline. macOS-only MVP.
> **Companion docs (this planning chain):** `PRESEARCH.md` (intake + de-dup map), `RESEARCH.md` (sourced tech findings), `DECISIONS.md` (11 ADRs), `DATA_MODEL.md` (SQLite persistence), `THREAT_MODEL.md`, `RISKS.md`, `OPEN_QUESTIONS.md`, `DIAGRAM_PLAN.md`, `CLAUDE_CODE_HANDOFF.md`.
> **Upstream product/spec docs (authoritative; referenced, not restated):** `docs/product/PRD.md`, `docs/product/PRODUCT_CANON.md`, `docs/architecture/SHARED_OBJECT_MODEL.md` (SOM), `docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md` (EM), `docs/domains/ACTION_GATEWAY.md` (AG), `docs/domains/WORKFLOW_PACKS.md` + `docs/domains/CC_CREW_WORKFLOW_PACK.md`, `docs/architecture/PROJECT_BRAIN_INTERFACE.md` (PBI), `docs/architecture/DESKTOP_FIRST_RUNTIME.md` (DFR), `docs/ux/UX_INFORMATION_ARCHITECTURE.md` (UX) + `docs/ux/UI_COMPONENT_INVENTORY.md`.
> **Build contract:** Treat this as the first-draft source of truth. `arch-finalize` should resolve the open questions / spikes (`§22`, `OPEN_QUESTIONS.md`), reconcile any drift against the upstream specs, finalize, and only then let `tasks-gen` produce `MVP_TASKS.md`. Every section carries a stable `§N` anchor; downstream task planning binds to these.
> **Tag legend:** `[LOCKED]` · `[PROPOSED]` · `[OPEN]` · `[MVP-SIMP]` · `[DEFERRED]` · `[RESEARCH]`.

---

## §1. Executive Summary

NexusOps is a **desktop-first, local-runtime AI engineering control plane** for macOS: a cockpit that lets one developer **dispatch, supervise, review, deliver, and remember** the work of many AI coding agents (Claude Code + Codex) across multiple local projects (`PRD §1–2`, `PRODUCT_CANON §2`). The **Session** is the atomic operational unit; a strict chain of ownership (Project → Session → Worktree/Branch → Terminal → Task → Harness → Execution Profile → Diffs → Commits → PR → Brain episode → Events) is preserved on every surface (`SOM §12`, `UX §4.1`).

Architecturally it is **three local processes** plus the agent subprocesses they supervise:

1. **NexusOps Daemon** (Rust, detached, long-lived) — the trust core and the only thing that mutates state. Owns the SQLite event store (sole writer), the Action Gateway (single mutation chokepoint), the harness adapters, PTYs, git/worktree ops, GitHub/Linear syncers, the lease-lock manager, the projection engine, and the Project Brain sidecar's lifecycle. Survives UI restarts (`ADR-002`).
2. **NexusOps UI** (Tauri 2.x shell — Rust host + system WebView) — a *reattaching client*. Renders projections; hosts xterm.js terminals; never writes the DB; reaches the daemon over a Unix-domain-socket `GatewayPort` (`ADR-001/004`).
3. **Project Brain sidecar** (Python / FastMCP, stdio MCP) — a *sibling product* that reasons over project memory and **proposes** action plans; it never executes. Daemon-owned lifecycle; opens no port (`ADR-005`, `PBI §1`). Brain internals are out of scope for this architecture (integration seam only).

The design is an **event-sourced, projection-driven, capability-gated** system: every important fact is an immutable event in append-only SQLite; the UI reads rebuildable projections; every mutation is a typed, risk-classified, previewed, approved, audited `ActionRequest` through the Action Gateway (`EM`, `AG`). Both harnesses expose a structured supervision + approval surface (Claude's `can_use_tool` callback; Codex's `app-server` approval requests) that maps directly onto the Gateway, giving reliable status detection and worktree/task association without scraping terminals (`ADR-006`, `RESEARCH R-CC/R-CODEX`).

The MVP proves the full thesis via the **PRD §25 demo** (`§18.1`): add project → detect git/Brain/cc-crew → create execution profile → pick a plan task → create worktree + launch a Claude Code session → observe it in the sidebar/graph → agent requests permission → approve via the Action Gateway → review the diff → ask Brain (evidence chips) → Brain proposes a PR action plan → approve → PR created, with task/session/worktree/PR linked and every step recorded as events.

---

## §1A. Goals & Non-Goals

**Goals (MVP)** `[LOCKED]` — derived from `PRD §15`, scoped by the demo (`§18.1`):
- One cockpit to supervise many local agent sessions (Claude Code **and** Codex) with reliable, non-scraped status.
- Session is atomic; full ownership chain visible everywhere; attention-first ordering.
- Every mutation typed + previewed + approved + audited through the Action Gateway; nothing mutates outside it.
- Append-only event store + projections; immutable audit; crash-recoverable; UI restart-safe; agents survive UI quit.
- Worktree-isolated agent work; git basics + PR creation; manual task/plan/session/worktree/PR linking.
- Project Brain drawer (read + propose/preview action plans); Brain never executes directly.
- Workflow-pack detection (cc-crew first) with the platform fully usable on **basic** projects (no pack).
- Local trust boundary; secrets in the OS keychain; redaction before persist/embed/sync.

**Non-Goals (MVP)** `[LOCKED]` — `PRD §15 (Non-Goals)`, `§17`, `DFR §6`:
- Web app; cloud/remote runner; multi-user collaboration. `[DEFERRED]`
- iOS companion (design the seam, don't build it). `[DEFERRED]`
- Full IDE replacement (review-focused editor only). `[DEFERRED]`
- Bidirectional Linear sync; fully autonomous merges; policy-automation mode. `[DEFERRED]`
- Full cc-crew personalization/upgrade UI; full transcript→commit linking. `[DEFERRED]`
- Windows/Linux support (architecture stays portable, but macOS-only is the MVP target). `[DEFERRED]`
- Project Brain internals (vector store, embeddings, CodeGraph federation) — sibling product. `[OPEN — out of scope]`

---

## §2. Product Definition and Scope

Authoritative: `PRD §1–7`, `PRODUCT_CANON §2–16`. Not restated. Key load-bearing framings the architecture must honor:
- **Five responsibilities:** Dispatch · Supervise · Review · Deliver · Remember/Reason (`PRD §2.1`).
- **Six product layers** (Project/Work/Agent/Execution/Code/Delivery) with two cross-cutting systems (Project Brain, Workflow Packs) (`PRODUCT_CANON §7`).
- **11 product surfaces / 20 screens** (`PRD §7`, `UX §9`) — the screen contracts the UI must satisfy (`§11`).
- **MVP / P1 / P2 scope tiers** (`PRD §15–17`, `PRODUCT_CANON §16`) — mapped in `§18`.

---

## §3. Locked Architecture Decisions

Full ADRs with options/rationale/fallbacks: `DECISIONS.md`. Product-level decisions are inherited from `PRODUCT_CANON §17`. Summary (the binding spine of this draft):

| ADR | Decision |
|---|---|
| 001 | All-Rust: standalone **Rust daemon + Tauri 2.x shell**; **macOS-only** MVP |
| 002 | **Detached long-lived daemon from day one** owns PTYs/store/Gateway/adapters/Brain/locks; UI reattaches |
| 003 | **SQLite (WAL)**, single-writer = daemon; events + projections + projection_offsets + outbox; FTS5; `user_version` migrations; artifacts by path+hash |
| 004 | Action Gateway = **`GatewayPort` over Unix-domain-socket IPC**, JSON-RPC; execution in-daemon; agents/Brain submit intents only |
| 005 | Project Brain = **stdio MCP sidecar** (FastMCP 3.x, PyInstaller), daemon-owned lifecycle; *spike: macOS notarization #11992* |
| 006 | **One `HarnessAdapter` contract over two lifecycle models** — Claude: SDK `can_use_tool` + PTY display + JSONL; Codex: `app-server` JSON-RPC |
| 007 | **Dual git** (git2 reads / git-CLI mutations), **octocrab** (+gh-token bootstrap), Linear PKCE/key, **keyring** crate |
| 008 | Cross-restart locks = **SQLite lease table** (owner + monotonic fencing token + heartbeat) + `pidlock` |
| 009 | Terminal: **`portable-pty` in daemon → headless VT → Tauri Channel → xterm.js(WebGL)**; backpressure mandatory |
| 010 | Survival: UI-restart → reconnect live; daemon-restart → resume (`claude --resume`/`codex thread/resume`) else replay+relaunch |
| 011 | Secrets in OS keychain; **Developer ID signing + notarization = early release-blocker** |

---

## §4. System Overview

### §4.1 Process topology `[LOCKED — ADR-001/002]`

```text
┌──────────────────────────────────────────────────────────────────────────┐
│  macOS host = the trust boundary (DFR §5)                                   │
│                                                                            │
│  ┌─────────────────────────┐        UDS JSON-RPC (GatewayPort, ADR-004)    │
│  │  NexusOps UI            │◄──────────────────────────────────────┐       │
│  │  Tauri 2.x shell        │   projection reads + intent submits     │       │
│  │  (Rust host + WebView)  │   + terminal byte stream (Tauri Channel)│       │
│  │  · xterm.js (WebGL)     │                                         │       │
│  │  · 20 screens (UX §9)   │                                         ▼       │
│  └─────────────────────────┘                          ┌──────────────────────┐
│                                                        │  NexusOps Daemon      │
│  ┌─────────────────────────┐   stdio MCP (ADR-005)     │  (Rust, detached)     │
│  │  Project Brain sidecar  │◄─────────────────────────►│  THE TRUST CORE       │
│  │  Python / FastMCP       │   tools=propose/query      │  · Action Gateway     │
│  │  (sibling product)      │   notifications→events     │    (sole mutator)     │
│  └─────────────────────────┘                           │  · SQLite event store │
│                                                        │    (sole writer, WAL) │
│  ┌─────────────────────────┐  PTY + SDK/app-server      │  · Projection engine  │
│  │  Agent subprocesses     │◄─────────────────────────►│  · Harness adapters   │
│  │  Claude Code / Codex     │  can_use_tool / approvals  │  · Terminal mgr (PTY) │
│  │  (one PTY each)         │                            │  · Git/worktree mgr   │
│  └─────────────────────────┘                           │  · GitHub/Linear sync │
│                                                        │  · Lease-lock mgr     │
│  ┌─────────────────────────┐   octocrab / git CLI / SDK │  · Brain client       │
│  │  External: GitHub,      │◄─────────────────────────►│  · Outbox drainers    │
│  │  Linear, git remotes    │   tokens ← OS keychain      │  · pidlock single-inst│
│  └─────────────────────────┘                           └──────────────────────┘
└──────────────────────────────────────────────────────────────────────────┘
```

### §4.2 The three architectural laws `[LOCKED]`
1. **Single mutation chokepoint.** All state change flows through the Action Gateway in the daemon. Agents (`can_use_tool`/app-server approvals), the Brain (MCP tool proposals), the UI (intents), and any future RemoteClient submit *intents*; only the Gateway executes and only the daemon writes the DB. (`AG §2`, `ADR-004`)
2. **Events are facts; the UI reads projections.** Every important change is an immutable event; the UI reads rebuildable projections, never raw events or live truth it shouldn't (`EM §1/§4.6`, `ADR-003`). Source-of-truth discipline per `DATA_MODEL §3`.
3. **Reason vs execute split.** Project Brain plans/queries; the platform executes. Brain has zero direct privileged operations (`PBI §1`, `ADR-005`).

### §4.3 Why three processes (not one) `[LOCKED — ADR-002, RESEARCH R-PTY/R-PROC]`
A PTY child dies (SIGHUP) when its owner exits, so "agents survive UI quit" (`PRD DESK-7`) requires an owner that is **not** the UI → a detached daemon. The daemon also makes the trust core (credentials, mutations, audit) a small, memory-safe Rust surface separate from the restartable UI, and forces the Gateway behind an explicit IPC seam from day one (which the future iOS companion reuses). Cost: daemon lifecycle (single-instance via `pidlock`, stale-socket cleanup, UI↔daemon version handshake, orphan/zombie kill) — accepted and mitigated (`RISKS TR-13`).

---

## §5. Domain Model

Authoritative object catalog: `SOM` (~30 objects with fields/lifecycles; 4 canonical chains `§35`). Persistence-concrete realization: `DATA_MODEL.md`. The architecture adds:
- **8 status state machines** (Session/Task/Worktree/PR/WorkflowInstance/ProjectBrain/Approval/ExecutionProfile) — canonical enums in `SOM`/`UX §8`, persistence + derivation rules in `DATA_MODEL §4`.
- **Gap closed:** the four objects `DFR §7` requires but `SOM` lacks — **Device, RemoteClient, LocalRunner, EventProjection** — are specified in `DATA_MODEL §6` `[PROPOSED]`. Actor enum extended with `remote_client` (reconcile `EM §7`'s `remote_device` naming in finalize — `OPEN`).
- **22 shared IDs** (`PBI §3`) are the cross-product contract; format = prefixed ULIDs (`DATA_MODEL §5`). Harness→`session_id` mapping (Claude settable / Codex via `cwd`+returned `thread_id`) in `DATA_MODEL §5`.
- **4 canonical chains** (`SOM §35`) are invariants the event/data model must keep traceable end-to-end: ticket→merge, plan→implementation, brain-action, workflow-personalization.

---

## §6. Core Module / Service / Contract Architecture (daemon internals)

The daemon is decomposed into these modules (Rust crates/modules) `[PROPOSED]`. Each is a clear ownership boundary; all writes funnel through the Gateway → event store.

| Module | Responsibility | Key contracts / refs |
|---|---|---|
| **`gateway`** (Action Gateway) | The single mutation chokepoint: normalize → resolve resources → policy (risk 0–4) → preview/dry-run → approval → execute via executor adapters → emit authoritative `ActionExecution*` events. Holds the `GatewayPort` interface. | `AG §8/§12/§15`; pipeline `AG §15.1`; executors `AG §15.3`; idempotency `AG §16.1`; `ADR-004` |
| **`eventstore`** | Sole SQLite writer; append events + projections + outbox in one txn; FTS5; migrations. | `DATA_MODEL §2`, `EM §12`; `ADR-003` |
| **`projections`** | Rebuildable read-model engine; 8 MVP projections; `projection_offsets`; crash-safe replay; degraded handling. | `DATA_MODEL §2.3/§7.2`, `EM §13` |
| **`harness`** | `HarnessAdapter` trait + Claude & Codex impls (`§9.3`); status stream → events; `intercept-mutation` → Gateway intents; transcript refs; telemetry heartbeats; resume. | `ADR-006`; `RESEARCH R-CC/R-CODEX` |
| **`terminal`** | PTY ownership (`portable-pty`), headless VT state, scrollback serialize, backpressure/flow-control, stream to UI over Tauri Channel. | `ADR-009`; `RESEARCH R-PTY` |
| **`git`** | Dual backend: git2 for reads/projections; git CLI for mutations + worktree lifecycle; all mutations are Gateway actions. | `ADR-007`; `RESEARCH R-GIT` |
| **`integrations`** | GitHub (octocrab + gh-token bootstrap), Linear (SDK/GraphQL, PKCE/key); read/link first; outbox-driven writes. | `ADR-007`; `PRD §10.7/§10.10` |
| **`locks`** (lease mgr) | SQLite lease table; monotonic fencing tokens (mandatory); expiry/reclaim; pidlock single-instance. | `ADR-008`; `DATA_MODEL §2.6` |
| **`brainclient`** | Owns the Brain stdio MCP sidecar lifecycle (spawn/ping/restart/process-group kill); routes Brain tool calls as Gateway intents; translates MCP notifications → events. | `ADR-005`; `PBI §6/§7` |
| **`workflow`** | Workflow-pack detection (advisory) → readiness checks; command registry; cc-crew parsers (plan/architecture-anchor); personalization runs as Gateway action plans. | `WORKFLOW_PACKS`, `CC_CREW`; `PRD §10.12` |
| **`ipc`** | UDS server: versioned handshake, length-prefixed JSON-RPC, peer auth; serves projection reads + intent submits; the `GatewayPort` transport. | `ADR-004`; `RESEARCH R-PROC` |
| **`policy`** | Per-project policy/consent (transcript ingestion, auto-approval allowlists, execution-profile allowlists), redaction rules. | `PRD §10.19`, `PBI §8`, `THREAT_MODEL §4` |
| **`usage`** | Token/context/cost metering; accuracy labels (exact/estimated/unavailable); merges Claude statusLine + ResultMessage + Codex tokenUsage. | `PRD §10.18`, `EM §18`; `ADR-006` |

**`GatewayPort` contract** `[LOCKED — ADR-004]`: a Rust trait the daemon implements in-process and exposes over UDS. Callers depend on the trait, not the transport (keeps it framework-decoupled and iOS-ready). Agent/Brain mutations are *intents into* this port, never direct writes.

---

## §7. Data and State Model

Authoritative: `DATA_MODEL.md`. Highlights the architecture binds to:
- One daemon-owned SQLite DB (WAL) at `~/Library/Application Support/NexusOps/nexusops.db`; large artifacts content-addressed on disk; harness transcripts referenced in place (Codex rollout hardened to 0600).
- Tables: `events` (append-only spine, `seq`-ordered, reserved hash-chain columns), `object_refs`, 8 `proj_*` projections, `projection_offsets`, `outbox`, `leases`, `artifacts`, registry tables (projects/repositories/execution_profiles/workflow_instances/integration_connections/plan_tasks/command_registry), `action_requests`/`approvals`, `harness_session_map`, FTS5 (`DATA_MODEL §2`).
- **Source-of-truth matrix** (`DATA_MODEL §3`): event-derived vs durable-registry vs git/FS-derived (live git2) vs harness-derived vs keychain vs Brain — the daemon never treats a projection as truth when a live source exists.
- Migrations (`user_version`), projection rebuild/crash recovery, retention, and the optional JSONL mirror fallback (`DATA_MODEL §7`).

---

## §8. User Flows

Authoritative flows: `PRD §9` (14 flows) + `UX §10` (flows A–M). The architecture maps the demo-critical flows to components (full mapping is `arch-finalize`'s requirements→flow audit — flagged `RESEARCH`):

| Flow | Component path |
|---|---|
| Add local project (`PRD §9.2`) | UI intent → `gateway` → `git`(detect) + `workflow`(detect) + `brainclient`(status) → registry rows + events → projections |
| Start from plan task (`PRD §9.4`) | UI → `gateway`(create worktree=git CLI, create session) → `locks`(lease) → `harness`(launch) → `terminal`(PTY) → events |
| Respond to permission (`PRD §9.8`) | `harness`(can_use_tool / app-server approval) → `gateway`(ActionRequest, risk) → `proj_approval_queue` → UI approve → execute → events |
| Review diff (`PRD §9.9`) | UI → `git`(git2 diff read) → review workspace; ask-agent = `gateway` intent |
| Ask Brain + action plan (`PRD §9.11/§9.12`) | UI → `brainclient`(MCP tool) → Brain proposes plan → `gateway`(preview/approve) → execute → events → Brain re-indexes |
| Create PR (`PRD §9.10`) | `gateway` → `integrations`(octocrab) → outbox → events; PR linked to session/worktree/task |

---

## §9. Integration Architecture

`[LOCKED — ADR-007]`. **Git:** dual backend — git2-rs for hot structured reads (status/diff/log/branch/worktree-list, feeding projections); **git CLI for all mutations + worktree lifecycle** (terminal parity + single Gateway chokepoint; libgit2 can't do relative worktrees / misreports sparse-checkout). **GitHub:** octocrab (typed REST+GraphQL) for issues/PRs/checks/merges; bootstrap auth by reusing `gh auth token`, else OAuth Device Flow. **Linear:** `@linear/sdk`/GraphQL; auth-code+PKCE (loopback) or pasted personal key; 24h refresh; budget query complexity (10k-point cap). **Staged sync** (inherited): link (P0) → one-way create (P1) → bidirectional (P2). All integration writes are Gateway actions drained via the outbox; reads cached as projections.

### §9.1 Harness adapter layer `[LOCKED — ADR-006]`
One `HarnessAdapter` trait — `{ launch, stream-status, intercept-mutation, read-transcript, telemetry-heartbeat, resume }` — over two lifecycle models:
- **Claude Code adapter:** Agent SDK **streaming-input** session; **`can_use_tool` callback = in-harness mutation chokepoint** (allow/deny/rewrite → Gateway intent); status from SDK message stream + `Notification` hooks; **PTY mirror for human display/takeover**; transcript JSONL (`~/.claude/projects/.../<id>.jsonl`) tailed for durable replay; telemetry merged from `ResultMessage.usage` + statusLine (`refreshInterval` heartbeat). Settable session id → clean 1:1 mapping.
- **Codex adapter:** **`codex app-server --stdio`** JSON-RPC; `thread/start{cwd}` returns thread id (mapped to a platform `sess_` ULID); `thread/list?cwd=` re-association; push status; **host-routed approvals** → Gateway; rollout JSONL hardened to 0600. Gaps: no settable id (`RISKS TR-02`), no context-window % (UI shows estimate/cumulative).
- **Cross-cutting rule:** never scrape the PTY for machine state (`ADR-006`). Adapter contract keeps the Gateway/event store agent-agnostic and ready for future adapters (`PRD HARN-4`).

---

## §10. Automation / Background Jobs

Daemon-internal background workers (Tokio tasks) `[PROPOSED]`:
- **Projection workers** — fold new events into the 8 projections, advance `projection_offsets` in-txn (`DATA_MODEL §2.4`).
- **Outbox drainers** — deliver to Brain (MCP), GitHub/Linear syncers, the notifier, optional JSONL mirror; backoff/retry (`DATA_MODEL §2.5`).
- **Heartbeat/status pollers** — Claude statusLine (`refreshInterval`) + Codex app-server push; derive `stale` by heartbeat age (time-derived transition).
- **Lease reaper** — expire stale leases, mint new fencing tokens (`DATA_MODEL §2.6`).
- **Git watcher** — refresh worktree/branch projection caches via git2 + git hooks (post-commit/merge/checkout) `[PROPOSED]`.
- **WAL checkpointer** — scheduled `wal_checkpoint(TRUNCATE)` to avoid checkpoint starvation (`RESEARCH R-STORE`).
- **Sidecar supervisor** — Brain MCP ping + restart/backoff + process-group kill (`ADR-005`).

---

## §11. Frontend Architecture

`[LOCKED — ADR-001]`. Tauri 2.x shell (Rust host + system WebView). The UI is a **projection-driven reattaching client**: it reads projections + submits intents over the UDS `GatewayPort`; it holds no authoritative state and never writes the DB. Terminals render via **xterm.js (WebGL addon)** fed by the daemon's `terminal` module over a Tauri Channel, with app-level backpressure (`ADR-009`). It must deliver the **20 screens / 8 status enums / attention-first ordering** in `UX §9/§8` and the component inventory (`UI_COMPONENT_INVENTORY`); approval prompts appear as **structured cards outside terminal text** (`UX §5.2`). Graph surfaces always have list/table fallbacks (`UX §5.7`). `[MVP-SIMP]` MVP prioritizes the demo screens (`§18.1`): Command Center, Project Home/Graph, Sessions, Session Terminal, Code/Diff Review, Plan View, Human Input Queue, Action Gateway modal, Project Brain drawer, Execution Profiles, Events/Audit.

---

## §12. Backend / Daemon / Adapter Strategy

`[LOCKED]`. The Rust daemon is the backend; there is no server. It is detached (launchd/`setsid`), single-instance (`pidlock`), and survives UI restarts. Concurrency: one serialized write-actor (the Gateway path) owns SQLite writes; projection reads use read-only WAL connections; long-lived work is Tokio tasks (`§10`). The daemon also owns the Brain sidecar and all agent subprocesses (process-group kill on shutdown to prevent orphans). Survival policy per `ADR-010` (reconnect on UI restart; resume-or-replay on daemon restart). The adapter layer (`§9.1`) and executor adapters (`AG §15.3`) are the pluggable extension points for new harnesses / action types.

---

## §13. Shared Package / Config Strategy & Project Brain Seam

- **Config/state locations** `[PROPOSED]`: app data under `~/Library/Application Support/NexusOps/`; secrets in OS keychain (`keyring`, pointers-only in DB); per-project policy rows; reuse existing local CLI auth (gh, Claude/Codex) where possible (`PRD DESK-5`).
- **Project Brain seam** `[LOCKED — ADR-005, PBI]`: stdio MCP sidecar; Brain **tools = proposals/queries**, fed into the Gateway as intents; Brain consumes events via an outbox→MCP-notification adapter and cites platform objects by shared ID; opens no port; degrades gracefully when absent/stale (platform never hard-depends on Brain). **Brain internals out of scope** (sibling product). **Spike owed:** macOS notarization of the bundled PyInstaller sidecar (`#11992`, `OPEN_QUESTIONS OQ-PLAT-SPIKE-1`).
- **Workflow Packs** `[LOCKED]`: optional; basic projects fully work; detection advisory → readiness checks; cc-crew is the first pack (parsers for `MVP_TASKS.md` + `ARCHITECTURE.md §N` anchors); personalization runs as Gateway action plans; workflow-owned manifests (`.scaffolding/manifest.json`) read-only to the platform (`WORKFLOW_PACKS`, `CC_CREW`).

---

## §14. Testing Strategy `[PROPOSED]`

- **Event-store/projection tests:** golden event logs → rebuild projections → assert read models; crash-recovery (interrupt mid-apply → replay → consistent); migration round-trips.
- **Gateway tests:** per action type — risk classification, preview/dry-run, idempotency dedup, stale-precondition re-check, fencing-token rejection of stale holders, fail-closed-on-audit-write.
- **Adapter contract tests:** a shared `HarnessAdapter` conformance suite run against both Claude and Codex (status transitions, mutation interception, resume, telemetry); **pin agent CLI versions** and re-run on bump (`RISKS TR-02/TR-03`).
- **Lease/concurrency tests:** simulated daemon restart → lease reclaim + new fencing token; two-session contention.
- **Terminal tests:** backpressure under high-output; scrollback serialize/replay fidelity for alt-screen/raw-mode TUIs (the agent CLIs) — the hardest, needs real-session QA (`RISKS TR-01`).
- **Security tests:** redaction-before-persist; secrets never in events; injection via stdin not argv; Codex rollout 0600 (`THREAT_MODEL`).
- **Demo integration test:** the PRD §25 path end-to-end (`§18.1`).
- TDD via the cc-crew `/tdd` engine once scaffolding lands (the build engine), per the project's workflow.

---

## §15. Security and Risk

Consolidated threat model: `THREAT_MODEL.md` (assets, 6 trust boundaries, Action-Gateway-as-control, 5-level sensitivity/redaction, 13-row threat table, MVP non-goals). Risk register: `RISKS.md` (top-5 + 10 product + 13 technical). Architecture-binding security invariants `[LOCKED]`:
- Local machine = trust boundary; daemon brokers all FS/git/PTY/credential access (`DFR §5`).
- No mutation except via a typed, approved, audited Gateway action (`AG`, `ADR-004`).
- Secrets only in the keychain; never in events/payloads; redaction before persist/embed/sync; terminal output defaults `restricted` (`EM §9`).
- UDS peer auth (no TCP port); injection-safe input via stdin/streaming (not argv); Codex rollout 0600; execution profiles explicit/auditable (no silent account-hopping — ToS-sensitive).
- **Largest residual MVP risk:** no agent egress isolation (`THREAT_MODEL T-01`) — a prompt-injected agent can still act within its granted permissions; mitigated by Gateway gating of mutations + dangerous-command detection, not eliminated.

---

## §16. Deployment Strategy

`[LOCKED — ADR-001/011]`. macOS-only MVP. Tauri bundler → signed/notarized `.app` + first-party updater. **Code-signing + notarization is an early release-blocker** (keychain ACLs prompt without a stable Developer ID; the Python Brain sidecar must be deep-signed). **Pre-build spike (owed):** validate notarizing the bundled PyInstaller sidecar via Tauri `externalBin` (`#11992`) on a real signed build (`OPEN_QUESTIONS OQ-PLAT-SPIKE-1`). Codex app-server schema bundle is version-pinned + regenerated in CI on every Codex bump (`OQ-HARN-SPIKE-4`). Demo runs locally; no cloud anything.

---

## §17. Alternatives Considered

Per-decision options/tradeoffs are in `DECISIONS.md` (each ADR) and `RESEARCH.md`. The consequential roads not taken:
- **Electron/Node stack** (most-proven node-pty terminal, single Chromium) — rejected for footprint, native-module ABI tax, JS (not Rust) chokepoint, no iOS-core reuse; kept as the fallback if Rust velocity or Tauri sidecar signing blocks (`ADR-001`).
- **In-process topology** — simpler, but can't survive UI quit; rejected per the user's daemon choice (`ADR-002`).
- **Codex `exec --json` per task** — simpler but coarser status + association race; kept as fallback to `app-server` (`ADR-006`).
- **Brain embedded via PyO3 / loopback-HTTP** — rejected for MVP (breaks the clean swappable seam / opens a port); HTTP is the documented future-iOS upgrade (`ADR-005`).
- **DuckDB / KV / libSQL** for the store — rejected vs SQLite (`ADR-003`).

---

## §18. MVP Boundaries and Deferred Work

Scope tiers: `PRD §15–17`, `PRODUCT_CANON §16`, mapped to the locked architecture. Deferred work is tagged `[DEFERRED]` throughout and consolidated in `OPEN_QUESTIONS.md`.

### §18.1 MVP technical slice (built backward from the PRD §25 demo) `[LOCKED]`
The smallest vertical that proves the thesis. Build order prioritizes invariants → the spine → the demo path:
1. **Daemon skeleton + SQLite event store + projections + UDS `GatewayPort`** (the spine; `§4`, `§6`, `§7`). pidlock single-instance.
2. **Action Gateway** with risk classification, preview, approval, audit events, idempotency, lease locks — the ~20 MVP action types (`AG §28.2`).
3. **Tauri shell** reading projections; Command Center + Project Home/Graph + Sessions + Human Input Queue + Action Gateway modal (`§11`).
4. **Project add + detection** (git via git2/CLI; workflow/cc-crew detect; Brain status).
5. **Execution Profiles** (keychain-backed; explicit, auditable).
6. **Claude Code adapter** (SDK + can_use_tool + PTY mirror + JSONL) — launch a session from a plan task into a new worktree; status in sidebar/graph.
7. **Codex adapter** (app-server) — parity path (MVP-blocking per user).
8. **Terminal** (portable-pty → xterm.js) for human visibility/takeover.
9. **Permission flow** → Human Input Queue → approve via Gateway (the safety demo beat).
10. **Code/diff review** (git2 diffs).
11. **Project Brain drawer** (read + propose a PR action plan; evidence chips) → approve via Gateway.
12. **PR creation** (octocrab) with task/session/worktree/PR linking + events.

### §18.2 Explicitly deferred (P1/P2) `[DEFERRED]`
Agent Team View + `/team-start` orchestration; full cc-crew personalization/upgrade UI; TDD slice tracker; PR checks + agent-fix flow; one-way/bidirectional Linear sync; Brain policy-automation; conflict resolver; usage budgets; iOS companion; Windows/Linux; multi-repo projects; hash-chain tamper-evidence (`PRD §16/§17`, `OPEN_QUESTIONS`).

---

## §19. Diagrams

See `DIAGRAM_PLAN.md` for the full plan. Priority diagrams: (1) full-system process/trust-boundary map (`§4.1`); (2) Action-Gateway pipeline + event/approval chain (`§6`, `AG §15.1`); (3) the 4 canonical object chains (`SOM §35`); (4) session lifecycle + status-derivation (no PTY scraping) (`DATA_MODEL §4.1`); (5) event-store/projection/outbox data flow (`§7`); (6) harness-adapter two-model seam (`§9.1`); (7) Brain seam (MCP intents/notifications ↔ Gateway/events) (`§13`); (8) the PRD §25 demo sequence (`§18.1`).

---

## §20. Repo Scaffold `[PROPOSED]`

```text
nexusops/
  daemon/                  Rust — the trust core (binary: nexusopsd)
    src/
      gateway/             Action Gateway: pipeline, policy, executors, idempotency
      eventstore/          SQLite writer, schema, migrations, FTS5
      projections/         8 projections, offsets, rebuild/recovery
      harness/             HarnessAdapter trait + claude/ + codex/ impls
      terminal/            portable-pty, headless VT, scrollback, backpressure
      git/                 git2 reads + git-CLI mutations, worktree lifecycle
      integrations/        github (octocrab), linear, auth bootstrap
      locks/               lease table, fencing, pidlock
      brainclient/         MCP sidecar lifecycle + notifications→events
      workflow/            pack detection, readiness, command registry, cc-crew parsers
      ipc/                 UDS GatewayPort server (JSON-RPC, handshake)
      policy/  usage/      per-project policy/consent; metering
      model/               shared domain types, the 22 shared IDs, ULID kinds
  ui/                      Tauri 2.x app
    src-tauri/             Rust host (thin: window, channel, UDS client)
    src/                   WebView frontend (xterm.js, 20 screens, projection views)
  brain/                   (sibling product — referenced, not built here)
  shared/                  IPC schema (JSON-RPC types), shared-ID contract, event-type registry
  docs/                    (this planning chain → finalized ARCHITECTURE.md + MVP_TASKS.md)
```
`[OPEN]` exact crate split + workspace layout to confirm in finalize; cc-crew scaffolding personalizes this after the architecture is finalized.

---

## §21. Decision Summary Table

See `§3` (this file) and `DECISIONS.md` (full ADRs with fallbacks/what-would-change-this). Product decisions: `PRODUCT_CANON §17`.

---

## §22. Spec Anchor Index (downstream task planning binds here)

| Anchor | Topic | Detailed source |
|---|---|---|
| `§4` | Process topology / 3 laws / trust boundary | `DFR §5`, `ADR-002` |
| `§5` | Domain model + gap objects + shared IDs | `SOM`, `DATA_MODEL §5/§6` |
| `§6` | Daemon module decomposition | `AG §15`, this draft |
| `§7` | Persistence / source-of-truth | `DATA_MODEL` |
| `§8` | User flows → components | `PRD §9`, `UX §10` |
| `§9` / `§9.1` | Integrations / harness adapters | `ADR-006/007` |
| `§10` | Background jobs | this draft |
| `§11` | Frontend / 20 screens | `UX §9` |
| `§13` | Brain seam / workflow packs | `PBI`, `WORKFLOW_PACKS`, `CC_CREW` |
| `§15` | Security | `THREAT_MODEL`, `RISKS` |
| `§16` | Deployment / signing | `ADR-011` |
| `§18.1` | MVP technical slice / demo | `PRD §25` |
| `§20` | Repo scaffold | this draft |

**Open spikes that gate the build** (`OPEN_QUESTIONS.md`): OQ-PLAT-SPIKE-1 (sidecar notarization #11992), OQ-HARN-SPIKE-2 (human-PTY vs SDK-driven handoff), OQ-DATA-SPIKE-3 (SQLite write-contention load test), OQ-HARN-SPIKE-4 (Codex app-server schema pin/CI), OQ-DATA-SPIKE-5 (gap-object reconciliation), OQ-INT-SPIKE-6 (octocrab/libgit2 spot-check).

---

## §23. Claude Code Review Instructions

For `arch-finalize` (Brain 2). Full handoff: `CLAUDE_CODE_HANDOFF.md`. In brief:
1. Read all of `docs/planning/*` + `docs/**` (PRD, canon, SOM, EM, AG, workflow packs, PBI, DFR, UX) end-to-end. **Do not start implementation.**
2. Run the second-pass gap audit (~13 dimensions, `CLAUDE_CODE_HANDOFF.md`): missing flows / lifecycle states / failure modes / interfaces-schemas / source-of-truth / unresearched deps / inconsistent decisions / overbuilt scope / missing tests / deploy path / trust boundaries / diagrams / task-planning anchors.
3. **Specifically verify:** every MVP requirement (`PRD §10/§15`) maps to a flow (`§8`) — the requirements→flow matrix this draft flagged `RESEARCH`; the 8 state machines reconcile across `SOM`/`UX`/`DATA_MODEL`; the actor-enum `remote_device`/`remote_client` naming (`DATA_MODEL §6.2`); the 4 gap objects; the 6 open spikes (`§22`).
4. Resolve load-bearing open questions with the human; apply confirmed edits; produce the binding `ARCHITECTURE.md` (repo root) from `templates/ARCHITECTURE.md`, preserving stable `§N` anchors.
5. Only then `tasks-gen` → `MVP_TASKS.md`, every task referencing these anchors; do not invent architecture.

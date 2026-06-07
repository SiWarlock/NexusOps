# OPEN QUESTIONS — Consolidated Register (NexusOps)

> **Phase 11–12 output · Brain 1 / arch-draft · ROUGH DRAFT for adversarial finalization.**
> This is the **single deduplicated, project-wide open-questions register**. It rolls up the scattered
> open-question lists from every spec into one scannable table, marks which are already settled by the
> ADRs/user, and flags the work owed before/around build.
>
> **Sources consolidated:** `PRD.md §21` + `§18` (canon dup), `PRODUCT_CANON.md §18`,
> `SHARED_OBJECT_MODEL.md §37`, `EVENT_MODEL_AND_AUDIT_TRAIL.md §27`, `ACTION_GATEWAY.md §32`,
> `WORKFLOW_PACKS.md §22`, `CC_CREW_WORKFLOW_PACK.md §25`, `PROJECT_BRAIN_INTERFACE.md §9`,
> `DESKTOP_FIRST_RUNTIME.md §8`, and the spikes/dependencies in `DECISIONS.md §"Decision dependencies & open spikes"`.
> **Do not restate source content** — this register references anchors and dedupes overlapping questions.

---

## 0. How to read this register

**Columns**

| Col | Meaning |
|---|---|
| **ID** | Stable handle, `OQ-<AREA>-<n>`. Cite these in `ARCHITECTURE_DRAFT`/`arch-finalize`. |
| **Question** | The decision still owed (deduped across specs). |
| **Why it matters** | What breaks / stays ambiguous if unanswered. |
| **Current best guess** | The leaning today (not binding unless marked RESOLVED). |
| **When** | `MVP-blocking` · `pre-build spike` · `P1` · `P2`. |
| **Fallback** | The safe regressive option if the guess fails. |
| **Status** | `RESOLVED-by-decision` (→ ADR/canon) · `OPEN` · `SPIKE`. |

**"When" semantics**
- **MVP-blocking** — cannot ship the PRD §25 demo without an answer.
- **pre-build spike** — answer needs an experiment *before* the relevant code is written (the 5 carry-to-finalize items live here).
- **P1 / P2** — fast-follow / later; explicitly out of the MVP critical path.

**Status counts (this draft):** RESOLVED-by-decision = 17 · SPIKE = 6 · OPEN = 40. *(proposed recommendation — recount on each edit.)*

---

## 1. ⚑ The 5 (now 6) carry-to-finalize spikes — read first

These are the `DECISIONS.md` "Decision dependencies & open spikes" items: locked *directions* with one
validation owed. They are the sharpest residual risk in the plan. *(locked decision: direction; **research required**: validation.)*

| ID | Spike | Owed before | Source |
|---|---|---|---|
| **OQ-PLAT-SPIKE-1** | macOS codesign + **notarization of the bundled PyInstaller Brain sidecar** in a real signed Tauri build (Tauri externalBin **#11992** + deep-sign bundled libs + entitlement `com.apple.security.cs.allow-unsigned-executable-memory`). | **build** | ADR-005, dep #1 |
| **OQ-HARN-SPIKE-2** | **Human-interactive-PTY vs SDK-driven handoff** model for Claude Code — when does the human "own" a session vs the Action Gateway, without double-driving the harness. | **finalize** | ADR-006, dep #2 |
| **OQ-DATA-SPIKE-3** | **SQLite write-contention quantification** under many concurrent agents — load-test before freezing single-writer (`BEGIN CONCURRENT` is *not* a relied-upon mitigation). | **build** (load test) | ADR-003, dep #3 |
| **OQ-HARN-SPIKE-4** | **Codex `app-server` schema-pin + CI-regen policy** — pin Codex version, regenerate the app-server JSON-RPC schema bundle in CI on every bump. | **build-time** | ADR-006, dep #4 |
| **OQ-DATA-SPIKE-5** | **Missing desktop objects** — `Device`, `RemoteClient`, `LocalRunner`, `EventProjection` exist in `DESKTOP_FIRST_RUNTIME §7` but not in `SHARED_OBJECT_MODEL` (only Terminal does). Define in `DATA_MODEL.md`. | **finalize** | ADR-005-adjacent, dep #5 |
| **OQ-INT-SPIKE-6** | **octocrab merge / GitHub-App ergonomics spot-check** + libgit2 relative-worktree gap re-check (`extensions.relativeworktrees`, git ≥2.48). Lower-stakes but called out in ADR-007 "What Would Change This". | **pre-build spike** | ADR-007 |

> Anything else tagged `SPIKE` below is a *candidate* spike surfaced by a spec but not yet on the
> `DECISIONS.md` critical list — promote in `arch-finalize` if it proves load-bearing.

---

## 2. Platform / Runtime

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-PLAT-1 | Desktop stack / framework? | The #1 PRD open Q; sizes everything. | **All-Rust: Rust daemon + Tauri 2.x**, macOS-only MVP. | — | Electron/Node (`node-pty`+xterm.js). | **RESOLVED-by-decision** → ADR-001 |
| OQ-PLAT-2 | OS target for MVP? | Deletes/keeps WebKitGTK + per-OS work. | **macOS-only**; Win/Linux post-MVP. | — | Add Linux → re-weigh vs Electron. | **RESOLVED-by-decision** → ADR-001 |
| OQ-PLAT-3 | Background helper/daemon separate from UI? | Survival of agents across UI quit. | **Detached long-lived daemon from day one** owns PTYs/store/Gateway/adapters/Brain. | — | Regressive in-process (topology-independent contracts). | **RESOLVED-by-decision** → ADR-002 |
| OQ-PLAT-4 | How does the local runner talk to the UI? | Transport for the whole client↔core seam. | **`GatewayPort` over UDS**, length-prefixed JSON-RPC, no TCP port. | — | Named pipe (Win) / loopback-HTTP+token. | **RESOLVED-by-decision** → ADR-004 |
| OQ-PLAT-5 | Should the runner/sessions survive UI restart? | Core PRD `DESK-7` promise. | **Yes** (daemon alive → reconnect-live; daemon crash → resume-or-replay). | — | Replay-only + relaunch. | **RESOLVED-by-decision** → ADR-002/010 |
| OQ-PLAT-6 | How much terminal state recovers after restart? | TUI/alt-screen re-attach is lossy; sets user expectation. | Serialized scrollback replay + harness `--resume`; **accept lossy alt-screen**. QA vs real Claude/Codex. | pre-build spike | Replay-only + relaunch; show "fresh session" banner. | **SPIKE** (ADR-010) |
| OQ-PLAT-7 | How do app updates handle local stores + running sessions? | Auto-update can orphan PTYs / skew UI↔daemon. | Versioned IPC handshake; daemon outlives UI update; `user_version` migrations gate store. | P1 | Require full quit before daemon upgrade. | OPEN |
| OQ-PLAT-8 | Single-instance + stale-socket / orphan-zombie cleanup policy? | Daemon lifecycle is "the hardest part of desktop apps." | `pidlock` single-instance (start-time check for PID reuse); process-group kill; stale-UDS reclaim on boot. | MVP-blocking | — | OPEN (direction from ADR-002/008) |
| OQ-PLAT-9 | Can we safely manage multiple Claude/Codex execution profiles at the OS process level? | Profile isolation = a named PRD/canon Q (`§18.4`). | Per-session env/cwd + reuse local CLI auth contexts; explicit user-owned profiles, no hidden routing. | MVP-blocking | One profile per harness in MVP. | OPEN |

---

## 3. Harness adapters

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-HARN-1 | MVP harness scope — Claude only, or Claude + Codex? | Doubles/halves adapter work. | **Both Claude Code AND Codex are MVP-blocking** (user). | — | n/a (settled). | **RESOLVED-by-decision** → ADR-006 |
| OQ-HARN-2 | Adapter shape across two very different harnesses? | Keeps Gateway+store agent-agnostic. | **One `HarnessAdapter` contract** `{launch,stream-status,intercept-mutation,read-transcript,telemetry-heartbeat,resume}` over two lifecycle models. | — | Per-harness bespoke adapters. | **RESOLVED-by-decision** → ADR-006 |
| OQ-HARN-3 | What terminal-capture mechanism is reliable enough? | PRD/canon `§18.3`; brittleness RISK-2. | `portable-pty` → headless VT → Channel → xterm.js; **never scrape PTY for machine state** (structured streams only). | — | — | **RESOLVED-by-decision** → ADR-006/009 |
| OQ-HARN-4 | Canonical session ID when wrapping an existing terminal session? | `SHARED_OBJECT_MODEL §37.5`; re-association after restart. | Claude: reconcile on `session_id`. **Codex: no settable id → key on `cwd + returned thread_id`**, `thread/list?cwd=` to re-assoc. | MVP-blocking | Codex `exec --json`, parse `thread.started`. | OPEN (gap noted in ADR-006) |
| OQ-HARN-5 | Human-PTY vs SDK-driven handoff for Claude (ownership model)? | Double-driving the harness corrupts state. | Gateway owns mutation interception via `can_use_tool`; PTY is display/takeover only — exact handoff TBD. | **pre-build spike** | Interactive-PTY + `PreToolUse` hooks (coarser). | **SPIKE** (= dep #2) |
| OQ-HARN-6 | How much tool-call/transcript state can be captured without fragile formats? | `EVENT_MODEL §27`, `SHARED_OBJECT_MODEL §37.6`. | Structured SDK/app-server streams primary; transcript JSONL as *forensic replay only*. Pin + re-verify on harness bumps. | MVP-blocking | Coarser status (polled / `ResultMessage`-only). | OPEN |
| OQ-HARN-7 | Codex has **no context-window %** — how to show usage? | Users expect a context gauge; Codex exposes cumulative tokens only. | Show cumulative tokens + estimate; label as estimate. | P1 | Hide the gauge for Codex; tokens only. | OPEN (gap noted in ADR-006) |
| OQ-HARN-8 | Codex `app-server` schema-pin + CI-regen policy? | Schema drift silently breaks the adapter. | Pin Codex version; regen schema bundle in CI per bump. | **build-time** | `codex exec --json` per task. | **SPIKE** (= dep #4) |
| OQ-HARN-9 | Claude SDK / statusLine field churn across fast v2.1.x cadence? | Status/telemetry fields move between releases. | Version-tolerant parsers + degraded states (RISK-2); re-verify on upgrade. | P1 | Pin Claude version; coarse status. | OPEN |
| OQ-HARN-10 | Codex rollout JSONL permissions hardening (bug #21660)? | `~/.codex/sessions` may be world-readable. | Harden to **0600** on first read; startup self-check. | MVP-blocking | Refuse to ingest until perms fixed; warn user. | OPEN (direction in ADR-006) |

---

## 4. Event store / data

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-DATA-1 | Canonical event-store backend? | PRD/canon/`EVENT_MODEL §27` core Q. | **SQLite (WAL)**, single-writer = daemon; events+projections+offsets+outbox in one txn; FTS5; `user_version` migrations. | — | + append-only JSONL mirror. | **RESOLVED-by-decision** → ADR-003 |
| OQ-DATA-2 | SQLite-only, or + append-only JSONL mirror from MVP? | Fail-closed audit durability/export. | SQLite-only for MVP; JSONL mirror is the cheap belt-and-suspenders fallback. | P1 | Add JSONL mirror. | OPEN (fallback in ADR-003) |
| OQ-DATA-3 | Hash-chain raw events from MVP or later? | Tamper-evidence vs MVP cost. | **Reserve `payload_hash`/`previous_event_hash` columns now; chain post-MVP.** | P2 | Enable chaining later (columns pre-reserved). | **RESOLVED-by-decision** (deferred) → ADR-003 |
| OQ-DATA-4 | Single-writer contention under many concurrent agents? | Could make the chokepoint a bottleneck. | Single-writer holds (WAL = 1 writer/many readers); **load-test to confirm before freeze.** | **build (load test)** | Quantify; revisit (not `BEGIN CONCURRENT`). | **SPIKE** (= dep #3) |
| OQ-DATA-5 | How much terminal output retained by default? | Volume + sensitivity (`D-007`, `EVENT_MODEL §10.10`). | **Do not persist raw PTY as first-class events**; store references/summaries; cap scrollback. | MVP-blocking | Configurable retention window. | OPEN (direction in ADR-003) |
| OQ-DATA-6 | Missing desktop objects — `Device`/`RemoteClient`/`LocalRunner`/`EventProjection`? | `DESKTOP_FIRST_RUNTIME §7` requires them; `SHARED_OBJECT_MODEL` lacks all but Terminal. | Define in `DATA_MODEL.md` during finalize. | **finalize** | Stub minimal fields for MVP (only `LocalRunner` likely needed pre-iOS). | **SPIKE** (= dep #5) |
| OQ-DATA-7 | Task vs PlanTask — separate objects or subtype? | `SHARED_OBJECT_MODEL §37.2`; affects schema + linking. | PlanTask as a specialized Task (lean toward one base). | MVP-blocking | Two separate objects. | OPEN |
| OQ-DATA-8 | Worktree strictly under Repository, or virtual worktrees across multi-repo Projects? | `SHARED_OBJECT_MODEL §37.1`; constrains git model. | Worktree→Repository for MVP (single-repo Project). | MVP-blocking | Virtual multi-repo worktrees (P1). | OPEN |
| OQ-DATA-9 | Event schema versioning strategy (`EVENT_MODEL §22`)? | Forward/back-compat as events evolve. | Envelope carries schema version; additive-first; migrations rebuild projections. | MVP-blocking | Version-tag + tolerant readers. | OPEN |
| OQ-DATA-10 | Which events does Brain index immediately vs summarize first? | `EVENT_MODEL §27`; cost + privacy of indexing. | Index structured low-sensitivity events; summarize/redact high-sensitivity. | P1 | Index nothing automatically; on-demand. | OPEN |

---

## 5. Action Gateway

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-GW-1 | Gateway transport — HTTP, IPC, in-process, or MCP tool? | `ACTION_GATEWAY §32` technical Q. | **`GatewayPort` over UDS**, in-daemon execution; agents/Brain submit **intents**, never mutate. | — | Named pipe / loopback-HTTP+token. | **RESOLVED-by-decision** → ADR-004 |
| OQ-GW-2 | Where does the audit log live? | `ACTION_GATEWAY §32`. | The SQLite event store (daemon-owned), same single-writer txn. | — | + JSONL mirror. | **RESOLVED-by-decision** → ADR-003/004 |
| OQ-GW-3 | How do locks survive app restarts? | `ACTION_GATEWAY §32`; OS locks die with the process. | **SQLite lease table** (owner + monotonic fencing token + heartbeat/expiry); fencing mandatory. | — | n/a (OS file locks rejected). | **RESOLVED-by-decision** → ADR-008 |
| OQ-GW-4 | Minimum useful action set for the first demo (P0 actions)? | `ACTION_GATEWAY §32`, PRD §21; scopes MVP executors. | Demo-driven set: `create_worktree`, `start_session`, approve/deny mutation, `review_diff`, `create_PR` (per `MVP action types §28.2`). | MVP-blocking | Trim to PRD §25 demo steps only. | OPEN |
| OQ-GW-5 | How are executor adapters registered? | `ACTION_GATEWAY §32`; extensibility. | Typed executor registry keyed by action type (`§15.3`). | MVP-blocking | Hardcoded executor table for MVP set. | OPEN |
| OQ-GW-6 | How are action schemas versioned? | `ACTION_GATEWAY §32`; ties to OQ-DATA-9. | Same envelope/versioning approach as events. | MVP-blocking | Version field + tolerant validation. | OPEN |
| OQ-GW-7 | How do action events feed Brain without a circular dependency? | `ACTION_GATEWAY §32`. | Brain consumes events read-only via adapter; emits *intents* back in — never reads-then-writes inline. | MVP-blocking | One-way: Brain reads, never re-submits in same loop. | OPEN |
| OQ-GW-8 | How are idempotency keys derived consistently? | `ACTION_GATEWAY §32, §16.1`; double-execution risk. | Key = action type + canonical resource ref + payload hash. | MVP-blocking | Caller-supplied idempotency key. | OPEN |
| OQ-GW-9 | How is shell-command risk classified (0–4)? | `ACTION_GATEWAY §32, §7`; gates approvals. | Allowlist + heuristic classifier into risk 0–4 (`§12.2`). | MVP-blocking | Treat all shell as high-risk (manual approve). | OPEN |
| OQ-GW-10 | Standing grants — which commands are safe for project-level allowlisting? | `ACTION_GATEWAY §32`; reduces approval fatigue. | Low-risk reads + `run tests` allowlistable per project. | P1 | No standing grants (approve each). | OPEN |
| OQ-GW-11 | `create_worktree` / `session.send_message` — approve every time or allowlist per session/project? | `ACTION_GATEWAY §32`; UX vs safety. | Policy-allowable per project/session; default = confirm first time. | MVP-blocking | Always confirm. | OPEN |
| OQ-GW-12 | What can be batched under "Approve All"? | `ACTION_GATEWAY §32`. | Risk ≤1 batchable; **critical never in Approve-All** (`§33`). | MVP-blocking | No Approve-All in MVP. | OPEN |
| OQ-GW-13 | Default policy for running tests? | `ACTION_GATEWAY §32`. | Allowlistable low-risk; default confirm-once. | P1 | Always confirm. | OPEN |
| OQ-GW-14 | How is partial-success recovery presented? | `ACTION_GATEWAY §32` UX; multi-step plans fail mid-way. | Per-step status on the ActionPlan card; resume/rollback affordances. | P1 | Show plan as failed; manual redo. | OPEN |
| OQ-GW-15 | How to keep frequent low-risk approvals from being annoying? | `ACTION_GATEWAY §32` UX; adoption risk. | Standing grants (OQ-GW-10) + risk-tiered prompts + batch. | P1 | Accept friction in MVP. | OPEN |

---

## 6. Workflow Packs / cc-crew

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-WP-1 | How should Workflow Pack schemas be standardized / what is the manifest called? | `WORKFLOW_PACKS §22`, canon `§18.7`. | Platform-native pack manifest schema (name TBD); cc-crew maps onto it. | MVP-blocking | Read cc-crew `.scaffolding/manifest.json` directly, no generic schema yet. | OPEN |
| OQ-WP-2 | Ship built-in packs, or import cc-crew as a user-installed pack? | `WORKFLOW_PACKS §22`; trust + distribution. | cc-crew as user-installed pack; platform stays pack-agnostic. | MVP-blocking | Bundle cc-crew as the one built-in pack. | OPEN |
| OQ-WP-3 | How much Claude skill/command parsing is generic vs pack-specific? | `WORKFLOW_PACKS §22`. | Generic command registry + pack-specific adapters where formats differ. | MVP-blocking | cc-crew-specific parsing only. | OPEN |
| OQ-WP-4 | Normalize commands into one cross-harness format? | `WORKFLOW_PACKS §22`. | Normalize into one `WorkflowCommand` shape; per-source binding (`§9`). | P1 | Per-harness command lists, no normalization. | OPEN |
| OQ-WP-5 | Declare schemas for older command formats with no explicit input schema? | `SHARED_OBJECT_MODEL §37.9`, `WORKFLOW_PACKS §22`. | Infer minimal schema / freeform-args fallback. | P1 | Freeform string args (no validation). | OPEN |
| OQ-WP-6 | Should personalization run via terminal, SDK, or platform-supervised session? | `WORKFLOW_PACKS §22`, canon `§18.8`; it writes files (high-risk action). | Platform-supervised session through the Gateway (high-risk gate). | MVP-blocking | Terminal-native run, observed only. | OPEN |
| OQ-WP-7 | Minimum useful generic plan-parser abstraction? | `WORKFLOW_PACKS §22`. | Parse `MVP_TASKS.md` for cc-crew; generic parser is the abstraction target. | MVP-blocking | cc-crew-specific MVP_TASKS parser only. | OPEN |
| OQ-WP-8 | How do pack upgrades interact with uncommitted changes? | `WORKFLOW_PACKS §22`; data-loss risk. | Block/stash + warn via Gateway preview before write. | P1 | Refuse upgrade with dirty tree. | OPEN |
| OQ-WP-9 | How is pack trust represented before signed packs exist? | `WORKFLOW_PACKS §22`. | "Unverified pack" badge + explicit consent on install/personalize. | P1 | All packs untrusted; max friction. | OPEN |
| OQ-WP-10 | Can a workflow instance safely span multiple repos? | `WORKFLOW_PACKS §22`; ties to OQ-DATA-8. | Single-repo instance for MVP. | MVP-blocking | Single-repo only (hard limit). | OPEN |
| OQ-CC-1 | Exact fields cc-crew exposes in `.scaffolding/manifest.json` for readiness? | `CC_CREW §25`; Workflow Setup screen accuracy. | Negotiate a readiness contract (version, stages, commands, team recipe). | MVP-blocking | Best-effort file sniffing; degraded readiness. | OPEN |
| OQ-CC-2 | Separate platform-side cc-crew instance/companion manifest? | `CC_CREW §25`; where platform metadata lives. | Platform metadata in store, not in the repo, for MVP. | P1 | Companion manifest file in repo. | OPEN |
| OQ-CC-3 | `/team-start` — one shared worktree or one per implementer? | `CC_CREW §25`; AgentTeam→PR shape (`SHARED_OBJECT_MODEL §37.4`). | One worktree per implementer (lean); reconcile to PR(s) — see OQ-CC-4. | MVP-blocking | Single shared worktree. | OPEN |
| OQ-CC-4 | How do AgentTeam outputs reconcile into one PR vs many? | `SHARED_OBJECT_MODEL §37.4`, `CC_CREW §25`. | One PR per implementer in MVP; aggregation later. | MVP-blocking | One PR per implementer. | OPEN |
| OQ-CC-5 | Can cc-crew emit structured markers/events for team membership + TDD stage? | `CC_CREW §25`; reliable observability vs scraping. | Request cc-crew emit machine markers; ingest as events. | pre-build spike | Parse known artifacts/log lines (fragile). | **SPIKE** (candidate) |
| OQ-CC-6 | How much of `scaffold-generate` is platform-supervised vs terminal-native? | `CC_CREW §25`; ties to OQ-WP-6. | Supervised through Gateway (writes files). | MVP-blocking | Terminal-native, observed. | OPEN |
| OQ-CC-7 | Minimum reliable way to detect spawned sessions (from `/team-start`)? | `CC_CREW §25`; AgentTeam membership accuracy. | Adapter-reported launches keyed on cwd+thread/session id (ties OQ-HARN-4). | MVP-blocking | Poll `thread/list?cwd=` / process scan. | OPEN |
| OQ-CC-8 | How are execution profiles assigned across lead/orchestrator/implementers? | `CC_CREW §25`; ties to OQ-PLAT-9. | Per-role profile assignment in team launch plan. | P1 | One profile for the whole team. | OPEN |

---

## 7. Project Brain seam

> **Scope reminder (locked):** Brain **internals are out of scope** (sibling product). Only the **integration seam** is in scope (user, 2026-06-06).

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-BR-1 | Does Brain run embedded, sidecar, local service, or library? | `PROJECT_BRAIN_INTERFACE §9`, `DESKTOP_FIRST_RUNTIME §8`. | **stdio MCP sidecar**, daemon-owned lifecycle (ping + process-group kill + backoff), PyInstaller bundle, **no port**. | — | Streamable-HTTP on 127.0.0.1 + loopback token. | **RESOLVED-by-decision** → ADR-005 |
| OQ-BR-2 | Brain scope — full product or just the seam? | Bounds this whole planning effort. | **Integration seam only.** | — | n/a. | **RESOLVED-by-decision** (user) |
| OQ-BR-3 | macOS notarization of the bundled Python sidecar? | Sharpest packaging risk in the plan. | Validate #11992 + deep-sign + entitlement on a real signed build. | **pre-build spike** | User-installed Brain CLI the daemon discovers (Brain optional). | **SPIKE** (= dep #1) |
| OQ-BR-4 | What Brain APIs/tools are needed for MVP? | PRD §21, canon `§18`; sizes the seam. | Read/query + propose-action-plan tools (per drawer `§6`); resource-update notifications → events. | MVP-blocking | Read/query only, no proposals. | OPEN |
| OQ-BR-5 | Which Brain actions run without confirmation; which always require it? | `ACTION_GATEWAY §32`, canon `§18.9/18.10`, `PROJECT_BRAIN_INTERFACE §8`. | Read/query auto; **all mutations via Gateway with approval**; auto-draft text per policy (see OQ-BR-6). | MVP-blocking | Everything Brain-initiated requires approval. | OPEN |
| OQ-BR-6 | May Brain auto-create draft text without approval? | `ACTION_GATEWAY §32`; convenience vs trust (RISK-4). | Drafts allowed (non-mutating); persisting/sending requires approval. | P1 | No auto-drafts. | OPEN |
| OQ-BR-7 | Are EpisodeCards generated on session completion or only on archive? | `SHARED_OBJECT_MODEL §37.7`, `EVENT_MODEL §27`. | On session completion (opt-in per project). | P1 | On archive only. | OPEN |
| OQ-BR-8 | Brain emits episode cards, or the session-history ingestor does? | `EVENT_MODEL §27`. | Ingestor emits; Brain enriches. | P1 | Brain emits directly. | OPEN |
| OQ-BR-9 | What event schema do platform + Brain share? | PRD §21, canon `§18.11`; ties to OQ-DATA-9. | The platform event envelope; Brain consumes via MCP-notification→event adapter. | MVP-blocking | Brain-specific projection view. | OPEN |
| OQ-BR-10 | How does Brain handle unavailable/stale platform data? | `PROJECT_BRAIN_INTERFACE §9`; degrade gracefully (Brain optional). | Degrade to read-only / "stale" badges; daemon supervises liveness. | P1 | Disable Brain features when stale. | OPEN |
| OQ-BR-11 | How does event deletion interact with Brain's historical answers? | `EVENT_MODEL §27`; privacy vs answer integrity. | Deletions tombstone; Brain must re-derive, not cite deleted facts. | P2 | Brain answers may go stale on deletion (documented). | OPEN |

---

## 8. Integrations / auth

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-INT-1 | Git engine — libgit2, CLI, or both? | Terminal parity + single mutation chokepoint = control-plane credibility. | **Dual: git2-rs reads / git CLI all mutations + worktree lifecycle.** | — | git-CLI-only (`--porcelain=v2`). | **RESOLVED-by-decision** → ADR-007 |
| OQ-INT-2 | GitHub client + auth bootstrap? | Issues/PRs/checks/merges. | **octocrab** + reuse `gh auth token`, else OAuth Device Flow. | — | shell `gh --json`. | **RESOLVED-by-decision** → ADR-007 |
| OQ-INT-3 | Linear client + auth (no device flow available)? | Task sync. | `@linear/sdk`; **PKCE loopback or pasted key**; 24h refresh; query-complexity budget. | — | Personal-key-only for MVP. | **RESOLVED-by-decision** → ADR-007 |
| OQ-INT-4 | Secret storage mechanism? | Creds for GitHub/Linear/Brain; iOS future. | **`keyring` crate** + per-OS feature flags + startup self-test; one `CredentialProvider`. | — | App-scoped encrypted file pre-signing. | **RESOLVED-by-decision** → ADR-007/011 |
| OQ-INT-5 | How do execution profiles map to local authenticated Claude/Codex contexts? | PRD §21, canon `§18.4`; ties to OQ-PLAT-9. | Reuse local CLI auth contexts; explicit user-owned profiles, no hidden routing. | MVP-blocking | One profile per harness. | OPEN |
| OQ-INT-6 | How do profiles expose usage limits without encouraging account-hopping? | `SHARED_OBJECT_MODEL §37.8`; RISK-6 (subscription-circumvention optics). | Show usage transparency only; no auto-routing; project allowlists. | P1 | Hide usage limits; manual profile pick. | OPEN |
| OQ-INT-7 | Safest first write-back path for plan-task ↔ Linear links? | `CC_CREW §25`, `ACTION_GATEWAY §22`. | Link-only first → one-way create → bidirectional later (inherited staged decision). | P1 | Link-only (read) for MVP. | OPEN (staging decided; path open) |
| OQ-INT-8 | How are execution-profile credentials isolated between sessions? | `ACTION_GATEWAY §32` security. | Per-session credential scoping via `CredentialProvider`; no cross-session bleed. | MVP-blocking | One profile context process-wide. | OPEN |
| OQ-INT-9 | octocrab merge / GitHub-App ergonomics + libgit2 relative-worktree gap re-check? | ADR-007 "What Would Change This." | Spot-check octocrab merge; re-check `extensions.relativeworktrees` (git ≥2.48). | **pre-build spike** | shell `gh` for merge; CLI-only worktrees. | **SPIKE** (= dep #6) |

---

## 9. Security / privacy

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-SEC-1 | Code-signing + notarization? | macOS keychain prompts repeatedly without a stable signed identity; sidecar must deep-sign. | **Developer ID code-signing + notarization is an early release-blocker.** | — | App-scoped encrypted file store until signing lands. | **RESOLVED-by-decision** → ADR-011 |
| OQ-SEC-2 | How are secrets redacted from command previews and logs? | `ACTION_GATEWAY §32`; secret-leak risk; sensitivity model (`EVENT_MODEL §9`). | Redaction pass on previews/audit; sensitivity tiers (public→restricted) gate display. | MVP-blocking | Redact aggressively (mask all token-shaped strings). | OPEN |
| OQ-SEC-3 | What approval is required before sending raw transcript snippets to cloud models? | `ACTION_GATEWAY §32`, RISK-9; strictest privacy gate. | Explicit per-snippet consent; exclude thinking blocks; local-by-default. | MVP-blocking | Never send transcripts to cloud in MVP. | OPEN |
| OQ-SEC-4 | How is shell-command risk classified for standing grants? | `ACTION_GATEWAY §32`; overlaps OQ-GW-9/10. | Risk 0–4 classifier (`§7/§12.2`); only low-risk allowlistable. | MVP-blocking | All shell = high-risk. | OPEN |
| OQ-SEC-5 | Default sensitivity classification per event category? | `EVENT_MODEL §9/§10`; drives redaction + iOS sync. | Map 16 categories → {public/internal/confidential/secret/restricted}; PTY/transcript = high. | MVP-blocking | Default everything to `confidential`. | OPEN |
| OQ-SEC-6 | Brain session-memory privacy (opt-in, redaction, cloud consent)? | RISK-9; overlaps OQ-DATA-10. | Opt-in per project; local embeddings default; exclude thinking blocks; explicit cloud consent. | P1 | Brain indexing off by default. | OPEN |

---

## 10. UX

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-UX-1 | What is the first demo workflow? | PRD §21, canon `§18.12`; anchors the whole MVP slice. | **PRD §25 MVP demo scenario** (add project → worktree → session → approve → review → Brain → PR). | — | n/a (settled). | **RESOLVED-by-decision** → PRD §25 |
| OQ-UX-2 | Final product name? | PRD §21, canon `§17.14`. | **NexusOps.** | — | n/a (settled). | **RESOLVED-by-decision** (user / NAMING.md) |
| OQ-UX-3 | Minimum useful embedded code editor? | PRD §21, canon `§18.5`; scope-creep RISK-7. | **Review-focused diff viewer first**, not a full IDE. | MVP-blocking | Read-only diff view; external IDE for edits. | OPEN (direction strong) |
| OQ-UX-4 | Where does the global approval/Human-Input queue live? | `ACTION_GATEWAY §32`; canon `§9.11` global attention queue. | Persistent global queue in the shell (right-panel + badge). | MVP-blocking | Per-project queue only. | OPEN |
| OQ-UX-5 | Brain drawer — action plans inline or separate panel? | `ACTION_GATEWAY §32`, `PROJECT_BRAIN_INTERFACE §6`. | Inline in the drawer with expandable plan card. | P1 | Separate panel. | OPEN |
| OQ-UX-6 | How much detail on the default action card before expanding? | `ACTION_GATEWAY §32`. | Summary + risk tier collapsed; preview/diff on expand. | P1 | Always-expanded card. | OPEN |
| OQ-UX-7 | Terminal display fidelity / flow-control UX (xterm.js caps)? | ADR-009 backpressure; chatty agents can stall UI. | ~30fps batched render, XON/XOFF, 50MB buffer cap; "output throttled" indicator. | MVP-blocking | Drop/sample output with a visible notice. | OPEN (direction in ADR-009) |
| OQ-UX-8 | Graph operability guarantees (RISK-10 "decorative graph")? | Canon `§17.10`; graph must have actions/status/inspector/filters/list-fallback. | Every node: actions + status + inspector + filters + list fallback. | MVP-blocking | List view only if graph slips. | OPEN (direction strong) |

---

## 11. iOS / future

> **Locked stance:** iOS companion is a **stretch goal, not MVP** (`EVENT_MODEL D-003`, `DESKTOP_FIRST_RUNTIME §9`); observability-first, control-second; **all remote actions via the Action Gateway**; **never a raw remote shell** (RISK-8). The questions below are explicitly **deferred** but the seams (UDS `GatewayPort`, `keyring` incl. iOS, event sensitivity tiers) are designed to keep them open.

| ID | Question | Why it matters | Current best guess | When | Fallback | Status |
|---|---|---|---|---|---|---|
| OQ-IOS-1 | Is the main app desktop, web+local-daemon, or both? | Canon `§18.1`; foundational. | **Desktop-first; web not MVP**; daemon already detached for future multi-client. | — | n/a (settled). | **RESOLVED-by-decision** → ADR-001/002, DESKTOP_FIRST §9 |
| OQ-IOS-2 | Minimum useful iOS read-only projection? | `DESKTOP_FIRST §8`, `EVENT_MODEL §27`. | Status + Human-Input queue + recent events (read-only). | P2 | Defer entirely. | OPEN (deferred) |
| OQ-IOS-3 | Remote transport — hosted relay, local VPN/Tailscale, iCloud, or direct pairing? | `DESKTOP_FIRST §8`; security posture of the companion. | Direct pairing / local VPN leaning (no hosted relay). | P2 | Defer; no remote until decided. | OPEN (deferred) |
| OQ-IOS-4 | What event payload fields may sync to iOS? | `EVENT_MODEL §27`; ties to sensitivity tiers (OQ-SEC-5). | Only `public`/`internal`; never `secret`/`restricted`. | P2 | Sync nothing until tiers finalized. | OPEN (deferred) |
| OQ-IOS-5 | What remote action risk level is allowed from iOS? | `EVENT_MODEL §27`, `DESKTOP_FIRST §8`. | Low-risk only; **critical actions desktop-only**. | P2 | Observability-only (no remote actions). | OPEN (deferred) |
| OQ-IOS-6 | Should iOS approvals require biometric confirmation? | `EVENT_MODEL §27`. | Yes for any mutation. | P2 | No remote approvals. | OPEN (deferred) |
| OQ-IOS-7 | Should remote approval require desktop unlock / presence signal for high-risk? | `EVENT_MODEL §27`. | Yes — desktop presence gate for elevated risk. | P2 | High-risk = desktop-only. | OPEN (deferred) |
| OQ-IOS-8 | How does iOS display workflow actions without becoming a remote shell? | `WORKFLOW_PACKS §22`, RISK-8. | Structured action cards only; no terminal stream-through. | P2 | Defer workflow surfacing on iOS. | OPEN (deferred) |

---

## 12. Dedup notes & provenance

- **Stack / daemon / store / transport / Brain-seam / harness-scope / git / locks / signing** appeared in
  PRD §21, canon §18, `DESKTOP_FIRST §8`, `ACTION_GATEWAY §32`, and `EVENT_MODEL §27` simultaneously —
  collapsed into single rows and marked **RESOLVED-by-decision** with ADR pointers.
- **Session-ID / session-detection** appeared in `SHARED_OBJECT_MODEL §37.5`, `CC_CREW §25`, and ADR-006
  gaps — merged into **OQ-HARN-4** + **OQ-CC-7**.
- **EpisodeCard timing** (`SHARED_OBJECT_MODEL §37.7` + `EVENT_MODEL §27`) → **OQ-BR-7/8**.
- **Personalization run mode** (`WORKFLOW_PACKS §22` + canon §18.8 + `CC_CREW §25`) → **OQ-WP-6 / OQ-CC-6**.
- **Locks-across-restart** (`ACTION_GATEWAY §32` + canon) → **OQ-GW-3** (RESOLVED → ADR-008).
- **iOS questions** from `EVENT_MODEL §27` and `DESKTOP_FIRST §8` overlapped heavily → merged into §11.
- **Linear staging** is *decided* (link→one-way→bidirectional, canon §17.12); only the **write-back path**
  remains open (**OQ-INT-7**).

## 13. Next steps for `arch-finalize`

1. **Close the 6 spikes in §1** — especially **OQ-PLAT-SPIKE-1 / OQ-BR-3** (notarization) before any build, and
   **OQ-DATA-SPIKE-3** (write-contention load test) before freezing single-writer.
2. **Resolve all MVP-blocking OPEN rows** into the binding `ARCHITECTURE.md` (anchor each answer).
3. **Land OQ-DATA-6** (`Device`/`RemoteClient`/`LocalRunner`/`EventProjection`) into `DATA_MODEL.md`.
4. Re-tag any candidate `SPIKE` (e.g. **OQ-CC-5**, **OQ-PLAT-6**) as promoted or demoted.
5. Recompute the status counts in §0 after edits.

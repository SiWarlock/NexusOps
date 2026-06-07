# RESEARCH — NexusOps Open Technical Decisions

> **Phase 10 output.** Current-facts research (web + Context7) on the load-bearing open technical decisions, run as an 8-way parallel fan-out on 2026-06-06. Findings are tagged **VERIFIED** (confirmed via a cited current source), **LIKELY**, **UNVERIFIED** (from model knowledge, needs checking), or **ASSUMPTION**. Each cluster gives options, a recommendation *lean* (not a lock — locks happen in `DECISIONS.md` after user confirmation), a fallback, and remaining risk.
>
> **Headline:** Nothing blocks the architecture. Two findings materially **de-risk** the plan: (1) Codex now ships **`codex app-server`** (long-running JSON-RPC/stdio, the surface the official VS Code extension uses) giving push-based status + native worktree association — **rough parity with Claude Code**, which de-risks the "both harnesses in MVP" choice; (2) Claude Code's **`can_use_tool` SDK callback IS the Action-Gateway primitive** (allow/deny/rewrite a tool call in-process). The one genuine MVP risk cluster is **terminal/PTY ownership + cross-restart session survival**.

---

## Research Questions → Decisions map

| ID | Cluster | Decision it informs | Lean |
|---|---|---|---|
| R-STACK | Desktop framework/runtime | ADR-001 stack | **Tauri 2.x (Rust)**, Electron fallback |
| R-PTY | PTY capture, terminal, survival | ADR-002 process/terminal model | daemon-owns-PTY vs in-process (contested) |
| R-CC | Claude Code automation | ADR-005 harness adapter | hybrid: SDK `can_use_tool` + PTY display + JSONL |
| R-CODEX | Codex automation | ADR-005 harness adapter | `codex app-server` stdio JSON-RPC |
| R-STORE | Event store engine | ADR-003 persistence | **SQLite (WAL)**, single-writer |
| R-BRAIN | Project Brain seam | ADR-004 Brain IPC | **stdio MCP sidecar** (PyInstaller) |
| R-GIT | Git + GitHub/Linear + creds | ADR-006 git/integrations | **dual git** (git2 reads / CLI mutations), octocrab, keyring |
| R-PROC | Process model, gateway transport, locks | ADR-002/007 | in-process **GatewayPort**, SQLite **lease locks** |

---

## R-STACK — Desktop framework & runtime

**Lean: Tauri 2.x (Rust core + system webview).** Fallback: Electron. Escape hatch: Rust local-daemon + browser UI.

**Why Tauri:** the Rust core is the natural home for the two hard seams — the **Action Gateway (single mutation chokepoint)** and the **append-only event store** — type-safe, no GC. Terminals are first-class *without* a fragile sidecar: the dominant 2026 pattern owns every PTY in the Rust backend via **`portable-pty`** (in-process threads) streamed over Tauri **Channels** to **xterm.js (WebGL addon)** — shipping examples: Terax, Terminon, `tauri-plugin-pty` [VERIFIED]. Footprint is decisive on a machine already running many terminals + agents + a Python sidecar: **~30–40MB idle / <10MB installer** vs Electron's **200–400MB / 80–150MB** [LIKELY]. First-party updater + documented macOS notarization / Windows signing [VERIFIED]. Same Rust core keeps a future iOS companion feasible (Tauri v2 unified desktop+mobile) [LIKELY].

**The one material risk — Linux WebKitGTK:** Tauri maintainers say they "cannot fully recommend Tauri" where strong Linux support is needed *now* (WebKitGTK instability, broken video/WASM, upstream bugs); CEF/Servo replacements are **not production-ready** as of late-2025 [VERIFIED]. Three webview engines (WKWebView/WebView2/WebKitGTK) = 3× rendering QA. **This risk is almost entirely a function of whether Linux is an MVP target** → see decision questions.

**Why not Electron (fallback):** the most battle-tested terminal stack on earth (node-pty + xterm.js = VS Code/Hyper) and a single Chromium target with **no WebKitGTK risk** [VERIFIED] — but heavy footprint, a JS (not Rust) Gateway, `node-pty` not thread-safe, and no iOS-core reuse.

**Shared truths (both stacks):** xterm.js is the throughput bottleneck (~5–35 MB/s, 50MB buffer) → **app-level flow control/backpressure is mandatory** for chatty agents [VERIFIED]. Process-lifecycle (orphan/zombie kill via Windows Job Objects + Unix signals) is the most error-prone part of wrapper apps and is **NexusOps's job either way** [VERIFIED].

**Remaining risk:** Linux WebKitGTK severity for *this* terminal-heavy UI is unverified (needs an early spike if Linux is in scope); no benchmark of either stack with 20+ concurrent live PTYs; Rust build velocity is a throughput (non-technical) risk.

---

## R-PTY — Terminal capture, embedded terminal & session survival  ⚠️ the genuine risk cluster

**Both Claude Code AND Codex effectively REQUIRE a real PTY/TTY** [VERIFIED] — Claude Code crashes/hangs without one (even `claude -p` has a no-TTY hang, issue #9026); Codex historically lacked a headless mode (#4219). **This makes embedded real terminals mandatory, not optional** — it validates a core PRD stance.

**Session "survival" is a process-ownership problem, not a library feature** [VERIFIED]: a PTY child dies (SIGHUP) when its owner exits. Neither `node-pty` nor `portable-pty` can make a child outlive its owner. VS Code is the reference architecture: a dedicated **"pty host" process** separate from the UI, with a **headless xterm.js** maintaining scrollback for replay on re-attach [VERIFIED]. VS Code distinguishes **two survival modes** [VERIFIED]:
- **Reconnect on UI reload** — the shell keeps running because the pty host didn't die; the renderer reconnects. ✅ achievable.
- **Revive on full quit** — the process does **not** survive; only scrollback is persisted to disk and the process is **relaunched fresh**. ⚠️ "survival" here = replay + relaunch, not a live process.

**True live survival across a full app quit** requires an independent **detached daemon** (double-fork + `setsid()` so it's not in the UI's session/process group) that owns the PTYs — the genuinely hard/fragile part (e.g. Superset's terminal daemon, Jan 2026) [VERIFIED]. Scrollback restore uses **`@xterm/headless` + `@xterm/addon-serialize`** (VT-sequence serialization); accurate re-attach of **alt-screen/raw-mode TUIs (which the agent CLIs use)** is lossy and the hardest correctness problem [VERIFIED/LIKELY].

**This is the contested architecture fork** (ADR-002): in-process PTYs (simpler, UI-open supervision) vs detached daemon (agents survive UI quit, per PRD DESK-7 "SHOULD survive"). See decision questions.

**Remaining risk:** full-quit live survival is fragile in any stack (budget real engineering or scope MVP to reload-survival + relaunch-on-quit); accurate TUI re-attach needs QA against real Claude Code/Codex; native-module packaging for node-pty (ABI/prebuild) if Electron; agent-CLI TTY behavior is a moving target.

---

## R-CC — Claude Code automation realities  ✅ rich & reliable

Claude Code is highly automatable in 2026 (CLI ~v2.1.167; Agent SDK TS `@anthropic-ai/claude-agent-sdk` ~0.3.x / PyPI `claude-agent-sdk`) [VERIFIED]. Key facts:
- **`can_use_tool` SDK callback** (streaming-input mode) = `async (tool, input, ctx) -> Allow|Deny`, can return `updated_input`/`updated_permissions` — **literally the Action-Gateway mutation chokepoint inside the harness** [VERIFIED]. This is the strongest single finding for the supervision design.
- **Status detection** is clean from the SDK message stream (`SystemMessage`init → `AssistantMessage`{thinking/tool_use} → `UserMessage`{tool_result} → terminal `ResultMessage`) and from **hooks** (`Notification` subtypes `permission_prompt`/`idle_prompt` = waiting; `Stop`, `SessionStart/End`, `Pre/PostToolUse`) [VERIFIED]. **Never scrape the PTY for state** — TUI repaints/spinners are version-fragile and a spinner ≠ "working" (SSE stalls) [LIKELY].
- **Telemetry is fragmented by design** [VERIFIED]: hooks carry **no** usage/context data; context %/cost/rate-limits live in the **statusLine JSON** (`context_window.used_percentage`, debounced heartbeat, goes quiet when idle → set `refreshInterval`); authoritative usage/cost in `ResultMessage.usage` + `total_cost_usd`; per-response usage in transcript `message.usage`. The projection layer must **merge** these.
- **Transcripts:** JSONL at `~/.claude/projects/<sanitized-cwd>/<session-id>.jsonl`; the path is also handed to you in statusLine + every hook payload [VERIFIED].
- **Resume:** `--continue` / `--resume <id>`; SDK `resume=<id>` + `session_store` [VERIFIED].
- **Injection safety:** feed untrusted input via SDK streaming-input or stdin (not interpolated CLI args), 10MB stdin cap [VERIFIED].
- **Quota note:** from **2026-06-15**, SDK/`-p` usage on subscription plans draws a **separate Agent-SDK credit pool** [VERIFIED] — surface in cost projections.

**Lean (Option C, Hybrid):** SDK streaming-input + `can_use_tool` = supervision + mutation chokepoint; **PTY mirror for human visibility/takeover**; transcript JSONL tail = durable replay into the event store. Shared `session_id` across SDK/transcript/statusLine makes reconciliation tractable. **Fallback:** interactive PTY + hooks-to-disk + statusLine if SDK-driving and PTY-attach conflict (coarser permission interception).

**Remaining risk:** exact semantics of a human-interactive PTY attached to a session *simultaneously* SDK-driven needs a spike; JSONL/statusLine schema churns across the fast v2.1.x cadence (pin a tested version, re-verify on upgrade — `total_input_tokens` semantics already changed at v2.1.132).

---

## R-CODEX — Codex automation realities  ✅ de-risked via app-server

CLI 0.137.0 (2026-06-04) [VERIFIED]. The pivotal finding:
- **`codex app-server --stdio`** — a long-running **JSON-RPC 2.0** server, the documented surface for deep integration, used **exclusively by the official VS Code extension** [VERIFIED]. It gives: `thread/start` (accepts `cwd`, **returns the thread id** — no stdout race) / `thread/resume` / `thread/list?cwd=` (re-association) / `thread/read`; `turn/start|steer|interrupt`; **host-routed approval requests** (`item/commandExecution/requestApproval`) → maps onto the Action Gateway; **push status** via `thread/status/changed` (idle/active{waitingOnApproval}/systemError), `thread/closed`, `turn/completed`, `thread/tokenUsage/updated` [VERIFIED]. **This is rough parity with Claude Code on the two MVP-blocking axes (reliable status + reliable worktree/task association).**
- **Same stdio-JSON-RPC idiom as the Brain seam** → share the process/framing layer.
- **Transcripts:** rollout JSONL at `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` (full conversation/tools/usage) [VERIFIED]. ⚠️ **created world-readable 0644** (issue #21660, OPEN) — platform must harden to 0600 or prefer `--ephemeral` + its own store. Trust-boundary-relevant.
- **Gaps vs Claude Code:** no **settable** session id (#15271 closed unaddressed) — key on `cwd + server-returned thread_id`; **no context-window %** exposed (only cumulative token counts, #21295) — UI can't show accurate "Context % used" for Codex without estimation; app-server is **partly experimental** (some methods need `experimentalApi`; pin Codex version + regenerate schema bundle in CI on every bump).

**Lean:** build the Codex adapter on **`codex app-server`** (primary), **`codex exec --json`** as fallback. The adapter seam must abstract **two lifecycle models** (Claude SDK streaming vs Codex app-server JSON-RPC) behind one contract: `{launch, stream-status, intercept-mutation, read-transcript, telemetry-heartbeat}`. **Codex Cloud is out of MVP (no-cloud constraint)** but keep the seam cloud-aware.

**Remaining risk:** app-server protocol churn (mitigate: version-pin + CI schema regen); no settable id (association race in `exec` fallback); no context-% metric; rollout-JSONL 0644 disclosure bug.

---

## R-STORE — Local event store engine  ✅ confirm SQLite

**Lean: confirm SQLite (WAL) — no blocker found.** [VERIFIED throughout]
- WAL = **one writer + many concurrent readers** (readers don't block the writer); **multiple OS processes on the same host** can share it safely (UI reads + daemon writes) — but **not over network FS** (fine; local trust boundary). Single-writer is a **feature** here: it *is* the Action-Gateway-as-sole-writer discipline.
- Discipline: route all writes through one process; `busy_timeout` ~5–15s; `BEGIN IMMEDIATE` for writes; `synchronous=NORMAL`; periodic `wal_checkpoint(TRUNCATE)` to avoid **checkpoint starvation** under continuous readers.
- **Event-sourcing on SQLite is a standard pattern:** append-only `events` + `projections` + `projection_offsets` + **transactional outbox** (event + outbox + projection in one txn) → exactly the `EVENT_MODEL §12` schema. Projections are rebuildable; corrupt projection → drop & replay without touching raw events.
- **Binding follows the host language:** `rusqlite` (bundled) if Rust/Tauri (**no native-module ABI tax**) — community default for desktop; `better-sqlite3` if Electron/Node (fast, but `@electron/rebuild` per Electron version; Node 24 build issues reported). `node:sqlite` still experimental — not yet safe to bet on.
- **FTS5** (external-content + triggers) for audit/timeline/Brain-evidence search; **`user_version`** migrations (rusqlite_migration/refinery, or knex).
- **Rejected:** DuckDB (OLAP, slow small writes), redb/sled KV (no SQL/FTS/inspectability; sled abandoned), libSQL/Turso (sync is cloud-oriented, Rust rewrite not production-ready) — all worse fits.
- **Large artifacts** (transcripts, big diffs, embeddings) stay **path+hash references**, never BLOBs.
- **Brain keeps its OWN store** and consumes events via the outbox/projection seam — it must **not** be a second writer to the platform DB.

**Fallback:** if write contention bites, tighten to a single in-process write queue (still SQLite); adopt the **SQLite + append-only JSONL mirror** for fail-closed audit durability/export regardless. Hash-chain tamper-evidence is orthogonal and out of MVP (reserve `payload_hash`/`previous_event_hash` columns now).

---

## R-BRAIN — Project Brain integration seam  ✅ stdio MCP sidecar

**Lean: managed stdio MCP sidecar.** [VERIFIED throughout]
- **FastMCP 3.0** reached GA 2026-02-18 (moved to **PrefectHQ/fastmcp**, v3.2.x); default transport is **stdio** — "the client spawns the server process and manages its lifecycle" (the Claude Desktop model). Also supports **streamable-HTTP** by flipping one flag (the future iOS/remoting path).
- **Host clients exist and are maintained:** Rust **`rmcp` 1.7.0** (`TokioChildProcess` spawns the child) ; TS **`@modelcontextprotocol/sdk` 1.29.0** (`StdioClientTransport`). MCP stdio framing is **newline-delimited JSON-RPC** — host can speak it with no SDK if needed.
- **Lifecycle:** initialize handshake → `initialized`; health via **`ping`**; shutdown = close stdin → wait → SIGTERM→SIGKILL. **Push:** resource subscriptions + list-changed notifications (+ experimental **Tasks** primitive for long ops) — "something changed" then host reads; NOT a raw event stream → an adapter **translates MCP notifications into events**.
- **Seam contracts:** Brain tools = **Action-Gateway proposals/queries, never direct writes**; MCP notifications → event feed; opens **no local port** (best for the trust boundary).
- **Packaging — the sharpest risk:** bundle CPython via **PyInstaller** (or PyApp/uv standalone); **macOS codesigning of a Python sidecar is hard** — needs entitlement `com.apple.security.cs.allow-unsigned-executable-memory`, **deep-sign all bundled libs**, and Tauri has a **known open notarization issue for externalBin sidecars (#11992)** — must be validated on a real signed/notarized build. **Orphan cleanup** via process groups (`command_group`/`process_group(0)`); PyInstaller single-file is hard to fully terminate.
- Choosing FastMCP 3.x pins Brain to a major version (async state, decorator mode, explicit auth) — a Brain-internal concern that pins the contract.

**Fallback:** flip Brain to **streamable-HTTP on 127.0.0.1** + per-launch loopback token (same server code) for concurrency/debug/iOS; raw JSON-RPC if SDK churns.

**Remaining risk:** validate Tauri sidecar **notarization #11992** on a real macOS build before committing; rmcp shutdown/reconnect API needs a spike; MCP-subscription throughput over stdio for a high-frequency feed unproven; PyInstaller cold-start latency; Windows process-group kill.

---

## R-GIT — Git engine + GitHub/Linear + credentials  ✅ dual backend

**Lean: dual git backend.** [VERIFIED throughout]
- **`git2-rs` (libgit2) for hot structured reads** (status/diff/log/branch/worktree-list) — fast, typed, no per-call subprocess.
- **git CLI for ALL mutations + libgit2-gap cases.** Critical gaps: libgit2 **does not support `extensions.relativeworktrees`** (git ≥2.48; errors outright; fix in-flight #7210/#7254) and **misreports status under sparse-checkout**. Since NexusOps runs git in embedded PTYs and agents make their own worktrees (Claude Code ships native `--worktree`), **libgit2 disagreeing with the user's terminal is a credibility bug** → mutations + worktree lifecycle go through the CLI (also = one clean Action-Gateway chokepoint, terminal parity). This is the pattern shipping tools (vibe-kanban) converged on.
- **GitHub: `octocrab`** (v0.53.0, typed REST + GraphQL, in-process, typed errors) for issues/PRs/checks/merges; **bootstrap auth by reusing `gh auth token`** when gh is present, else own **OAuth Device Flow**. (Or shell `gh --json` as a fast MVP path / fallback.)
- **Linear:** official `@linear/sdk` over GraphQL; **no device flow** → desktop must run **auth-code + PKCE (loopback)** or accept a pasted **personal API key**; OAuth tokens expire 24h (refresh); rate limit = leaky bucket, 10k-point/query cap, default 50-record pages → budget query complexity. `actor=app` agent mode available.
- **Credentials: `keyring` crate** (Rust, v3.6.x) — covers macOS/Windows/Linux **and iOS** (future companion), binary secrets, sync mode. v3 needs **explicit per-OS feature flags** (misconfig → silent no-op store; add a startup self-test). keytar is stale. **macOS keychain ACLs require a stable code-signed Developer ID** or users get repeated prompts → code-signing enters the build pipeline early.

**Fallback:** git-CLI-only (`--porcelain=v2`/`-z`) if dual divergence bites (slower but guaranteed terminal parity); shell `gh` for octocrab gaps; Linear personal-key-only for MVP; app-scoped encrypted file store until code-signing lands.

**Remaining risk:** octocrab merge-PR/GitHub-App ergonomics need a docs.rs spot-check; libgit2 relative-worktrees fix unreleased (keep worktree ops on CLI); Linear loopback-OAuth UX + 24h refresh; keyring v3 feature-flag self-test; macOS keychain needs Developer ID.

---

## R-PROC — Process model, Action Gateway transport, cross-restart locks

**Lean (MVP):** single app process + in-process workers; **Action Gateway as an in-process service behind a `GatewayPort` interface** (the single serialized SQLite writer); Brain as stdio sidecar with hardened lifecycle; cross-restart locks as a **SQLite lease table**. [VERIFIED throughout]
- **Topology:** in-process workers (Tauri Tokio tasks / Electron `utilityProcess`) minimize the orphan/zombie/port-leak/second-launch failure class that dominates wrapper apps, at the cost of crash isolation. **The append-only event store (replay) + SQLite leases (durable locks) already provide the crash-recovery the in-process model otherwise lacks** — so the main weakness (process death) is largely mitigated by the architecture's own invariants. *This is the same fork as R-PTY's survival question (ADR-002).*
- **Gateway transport:** define a **`GatewayPort` interface**, implement **in-process** for MVP (zero serialization, impossible to bypass, trivially the single writer, framework-decoupled). Agents/Brain submit **intents** that flow *into* the Gateway (MCP tools feed in; they are not the Gateway). **Escalation path:** when a second client (future daemon/iOS) needs the chokepoint, move behind the same interface onto a **Unix-domain-socket / named-pipe** transport (+ per-launch token; explicit Windows DACL) — **preferred over localhost HTTP** (a loopback port is reachable by any local process).
- **Locks (cross-restart):** **OS advisory/`flock` locks do NOT survive restart** (rejected as the durable mechanism). Use a **SQLite lease table** `(resource_id, owner_id, fencing_token monotonic, heartbeat, expires_at)`: owner-guarded renew, expired-lease reclaim after crash, **fencing tokens** reject stale/paused holders (Kleppmann pattern; Hangfire.SQLite implements it). Projectable to the UI ("who holds this worktree/repo"). Add a coarse **`pidlock`** for single-instance.

**Fallback:** promote coordinator/runner to a **separate long-lived daemon** + move the Gateway behind the `GatewayPort` onto a UDS/named-pipe transport — a transport+supervisor swap, **not a rewrite**, because the Gateway was an interface and state already lives durably in SQLite.

**Remaining risk:** lease expiry is probabilistic → **fencing tokens mandatory**, acquire/renew SQL must be race-free under WAL; PID-reuse edge for pidlock; SQLite single-writer = Gateway is a throughput bottleneck under heavy concurrent agent mutation (fine for single-user desktop MVP; quantify if many agents mutate at once); Tauri sidecar SIGTERM/exit(0) semantics maintainer-sourced — validate on the chosen version.

---

## Cross-cutting conclusions feeding the decisions
1. **Stack choice is dominated by the target-OS choice** (WebKitGTK Linux risk). macOS/Windows-first → Tauri is a clear win; Linux-day-one → re-weigh vs Electron.
2. **The Action Gateway has a ready-made primitive in both harnesses** (Claude `can_use_tool`; Codex app-server approval requests) — design it to accept both feeds behind one interface.
3. **One unified harness-adapter contract** over two lifecycle models (Claude SDK-streaming + PTY; Codex app-server) keeps the Gateway/event-store agent-agnostic.
4. **SQLite single-writer = the Gateway** is the same idea three clusters arrived at independently (store, proc, gateway) — strong convergence.
5. **The real MVP risk is terminal/PTY survival**, and it forks the process topology (in-process vs detached daemon) — the decision the user most needs to weigh.
6. **Both adapters + the Brain all speak stdio JSON-RPC-ish** → one process-supervision + framing layer serves all three.
7. **Code-signing/notarization is an early, release-blocking task** (keychain ACLs, Python sidecar deep-signing, Tauri externalBin #11992).

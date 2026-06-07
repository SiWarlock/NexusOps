# NexusOps — Architecture Risk Register (rough draft)

> **Status:** Brain 1 / arch-draft ROUGH DRAFT for adversarial finalization (`/arch-finalize`).
> **Scope:** Architecture-level risk register. Inherits the 10 product risks from `docs/product/PRD.md §20` (RISK-1..10), re-maps them onto the locked architecture (`docs/planning/DECISIONS.md`, ADR-001..011), and EXTENDS them with the technical risks surfaced in `docs/planning/RESEARCH.md` (R-PTY, R-CC, R-CODEX, R-STORE, R-BRAIN, R-PROC, R-STACK, R-GIT).
> **How to read claim tags:** `[locked decision]` `[proposed recommendation]` `[open question]` `[MVP simplification]` `[deferred work]` `[research required]`.
> **Reference convention:** anchors (`DOC §N`) point at existing specs; this register does NOT restate their content.
> **Severity scale:** low / med / high / critical. **Likelihood:** low / med / high.
> **"In ARCH.md?"** column = whether `arch-finalize` MUST land this risk (and where) in the binding `ARCHITECTURE.md`.

---

## Top 5 Risks to Watch

These five dominate the MVP's technical risk surface. Each forks a load-bearing decision or is an outright release-blocker. If any one is mishandled, the demo (`PRD §25`) or the trust model (`DESKTOP_FIRST_RUNTIME §5`) breaks.

| # | Risk | Why it tops the list | Lead mitigation |
|---|------|----------------------|-----------------|
| 1 | **TR-01 — PTY / session survival across daemon restart** (incl. lossy alt-screen / raw-mode TUI re-attach) | This is *the* genuine risk cluster (`RESEARCH.md R-PTY`). ADR-002's detached daemon promises live survival; full-quit live survival is fragile in every stack, and accurate re-attach of the TUIs the agent CLIs use is the hardest correctness problem. Drives ADR-009/010. | Resume-first (`claude --resume` / `codex thread/resume`), serialized-scrollback replay + relaunch fallback (ADR-010); never claim live alt-screen fidelity we can't prove. |
| 2 | **TR-08 — macOS codesign + notarization of the bundled Python Brain sidecar** (Tauri externalBin #11992) | Early **release-blocker** (ADR-011, `RESEARCH.md` cross-cutting §7). A known-open Tauri bug + Python deep-sign + `allow-unsigned-executable-memory` entitlement can block shipping a notarized build entirely. | SPIKE owed (ADR-005): validate on a real signed/notarized build EARLY; fallback = streamable-HTTP loopback Brain (no bundled sidecar) (ADR-005 fallback). |
| 3 | **TR-02 — Codex app-server protocol churn + association race + no context-%** | The Codex adapter rests on a *partly experimental* app-server (`RESEARCH.md R-CODEX`); no settable session id forces `cwd + thread_id` keying (race), and no context-window % means the UI cannot show a real Codex "Context % used". | Version-pin Codex + CI schema regen on every bump; key on `cwd + server-returned thread_id`; UI shows estimate/cumulative-only for Codex (ADR-006). |
| 4 | **TR-05 — SQLite single-writer contention under many concurrent agents** | The daemon is the sole writer = the Gateway chokepoint (ADR-003/004). Correct by design, but a throughput ceiling once many agents mutate at once (`RESEARCH.md R-PROC` remaining risk). | In-process serialized write queue; `BEGIN IMMEDIATE` not `BEGIN CONCURRENT`; quantify under load; intents queue, never block reads (ADR-003). |
| 5 | **TR-06 — Lease-lock expiry is probabilistic** | Cross-restart locks are SQLite leases (ADR-008); expiry timing is inherently racy, so a stale/paused holder could double-mutate a worktree without fencing. | Fencing tokens MANDATORY + monotonic; race-free acquire/renew SQL under WAL; executors reject stale fencing tokens at mutation time (ADR-008). |

---

## Inherited Product Risks (PRD §20), re-mapped to the locked architecture

| ID | Risk | Cat | Sev | Likely | Mitigation (architecture-mapped) | Fallback | Test / validation signal | In ARCH.md? |
|----|------|-----|-----|--------|----------------------------------|----------|--------------------------|-------------|
| **PR-01** (RISK-1) | Product scope too large | scope | high | high | MVP gate = `PRD §15`; build the locked seams only (Session, Action Gateway, event store, two adapters, Brain seam); everything else `[deferred work]`. Each ADR is single-purpose. | Cut to Claude-Code-only adapter + read-only Brain for first demo (`PRD §25`). | MVP_TASKS.md maps every task to an ADR/anchor; no task without a spec anchor. | **Y** — §Scope & Non-Goals |
| **PR-02** (RISK-2) | Terminal/harness status detection is brittle | technical | high | high | `[locked]` ADR-006 one `HarnessAdapter` contract; machine state from SDK message stream / app-server push, **never** PTY scrape (`RESEARCH.md R-CC`/`R-CODEX`); degraded status states; version-tolerant parsers. | Coarser hooks-to-disk + statusLine (CC) / `codex exec --json` (Codex) fallback. | Adapter conformance suite asserts status transitions from fixtures; "spinner ≠ working" negative test. | **Y** — §Harness Adapter Layer |
| **PR-03** (RISK-3) | Git automation can be dangerous | technical | high | med | `[locked]` ADR-007 git CLI for ALL mutations + worktree lifecycle (terminal parity); ADR-004 every mutation = a typed intent through the Gateway; risk classification + preview + audit (`ACTION_GATEWAY §7`, `§13`, `§17.1`). | git-CLI-only mode; high-risk git ops require confirmation regardless of policy. | Preview-then-apply diff equals actual mutation; destructive ops (force-push, branch delete) gated by Level-3+ (`ACTION_GATEWAY §7.4`). | **Y** — §Action Gateway, §Git Layer |
| **PR-04** (RISK-4) | Project Brain becomes too powerful too early | security | high | med | `[locked]` ADR-005 Brain tools = proposals/queries only; ADR-004 all mutation via Gateway; capability modes read-only → draft → confirmed (`ACTION_GATEWAY §6`); see TR-10/TR-11. | Ship MVP Brain in read-only/draft mode only; no policy-automation mode in MVP `[MVP simplification]`. | No code path lets a Brain tool write SQLite or call an executor directly (architecture invariant test). | **Y** — §Project Brain Seam |
| **PR-05** (RISK-5) | Workflow Packs overfit cc-crew | scope | med | med | `[locked]` generic Pack/Instance/Personalization abstraction (`WORKFLOW_PACKS.md`); cc-crew is first pack not the contract (`CC_CREW_WORKFLOW_PACK.md`). Basic/Claude-Aware/Pack project modes. | Ship Basic + Claude-Aware modes; cc-crew pack optional. | Pack runtime loads a second (toy) pack without code change. | Partial — §Workflow Pack Runtime |
| **PR-06** (RISK-6) | Execution profiles look like subscription circumvention | product | med | low | `[locked]` explicit user-owned profiles, no hidden routing, usage transparency via `UsageRecord` (`SHARED_OBJECT_MODEL §34`), project allowlists. Surface CC Agent-SDK credit-pool split (2026-06-15, `RESEARCH.md R-CC`). | Single-profile MVP; show provider/account on every session. | Every session displays its execution profile + account; usage projection reconciles to authoritative `ResultMessage.usage`. | Partial — §Execution Profiles |
| **PR-07** (RISK-7) | Code editor scope creep | scope | med | med | `[locked]` review-focused diff workspace first; external IDE handoff; no full IDE in MVP (`PRD §15`). | Read-only diff viewer only for first demo. | Editor module ships zero file-tree/build/debug features in MVP. | Partial — §Review Workspace |
| **PR-08** (RISK-8) | Mobile companion security | security | high | low | `[deferred work]` iOS is post-MVP (`DESKTOP_FIRST_RUNTIME §6.3`); when it lands: observability-first, no raw shell, Gateway required, high-risk → desktop confirmation (`§6.2` hard boundary). keyring covers iOS (ADR-007). | iOS entirely out of MVP; trust boundary is the local daemon only (`DESKTOP_FIRST_RUNTIME §5`). | N/A in MVP; re-open at iOS planning. | **Y** — §Trust Boundary / §Deferred: iOS |
| **PR-09** (RISK-9) | Project Brain session-memory privacy | data | high | med | `[locked]` opt-in per project, local embeddings default, redaction, exclude thinking blocks, explicit cloud consent (`PRD §20`); sensitivity model gates Brain consumption (`EVENT_MODEL §9`, `§15`). Brain keeps its OWN store (ADR-003). | Brain consumes only `public`/`internal` events in MVP `[MVP simplification]`. | Brain-bound projection filters `confidential`/`secret`/`restricted` (`EVENT_MODEL §9`). | **Y** — §Project Brain Seam, §Event Sensitivity |
| **PR-10** (RISK-10) | Graph becomes decorative | product | med | med | `[locked]` every graph node has actions/status/inspector/filters + list fallback (`PRD §20`); graph is a projection over the event store, not a separate model (ADR-003). | Ship list views first; graph as enhancement. | Each node type opens an inspector + at least one Gateway intent. | Partial — §UI Projections |

---

## Extended Technical Risks (from RESEARCH.md + DECISIONS.md)

### TR-01 — PTY / session survival fragility across daemon restart (incl. lossy alt-screen / raw-mode TUI re-attach)
- **Category:** technical | **Severity:** critical | **Likelihood:** high
- **Source:** `RESEARCH.md R-PTY` (the genuine risk cluster); ADR-002, ADR-009, ADR-010.
- **Risk:** A PTY child dies on SIGHUP when its owner exits; neither `portable-pty` nor any lib makes a child outlive its owner. ADR-002's detached daemon (double-fork + `setsid()`) is the *only* path to live survival, and it is the genuinely fragile part. Worse: the agent CLIs run **alt-screen / raw-mode TUIs**, whose accurate re-attach via VT-sequence serialization is **lossy** and the hardest correctness problem. A botched re-attach shows the user a corrupted terminal and undermines the whole "control plane" credibility.
- **Mitigation `[locked]`:** ADR-010 layered survival — (1) UI restart while daemon alive → reconnect to live PTY; (2) daemon restart → **resume the harness session first** (`claude --resume <id>` / `codex thread/resume`) so a fresh, clean process replaces the lost one; (3) only if resume unavailable → **serialized-scrollback replay** (`@xterm/headless` + `addon-serialize` equivalent in the headless VT) **+ relaunch**, explicitly labeled "restored view, process relaunched" so we never claim live fidelity we don't have.
- **Fallback:** Scope MVP to reload-survival + relaunch-on-quit (VS Code's two-mode model) `[MVP simplification]`; degrade alt-screen TUIs to a "scrollback snapshot, press R to relaunch" card rather than a lossy live re-render.
- **Test / validation signal `[research required]`:** Kill the daemon mid-`/tdd` run with Claude Code AND Codex in alt-screen; assert (a) resume restores the session, (b) scrollback-replay fallback produces a readable (not garbled) screen, (c) no orphaned PTY children remain. Snapshot-diff serialized VT state against ground truth.
- **In ARCH.md?** **Y** — §Session Lifecycle & Survival, §Terminal Subsystem.

### TR-02 — Codex app-server protocol churn + no settable session id (association race) + no context-window %
- **Category:** integration | **Severity:** high | **Likelihood:** high
- **Source:** `RESEARCH.md R-CODEX`; ADR-006. Codex CLI 0.137.0; app-server is *partly experimental* (some methods need `experimentalApi`).
- **Risk:** Three coupled hazards. (a) **Churn** — methods/shapes shift between Codex releases. (b) **Association race** — no settable session id (#15271 closed unaddressed); must key on `cwd + server-returned thread_id`, and two sessions in the same `cwd` can race association. (c) **No context-% metric** (#21295) — only cumulative token counts, so the UI cannot show a real "Context % used" for Codex the way it can for Claude Code.
- **Mitigation `[locked]`:** **Version-pin Codex** + **regenerate the JSON-RPC schema bundle in CI on every Codex bump** (schema-drift = red CI). Key sessions on `cwd + returned thread_id`, use `thread/list?cwd=` for re-association on restart, and harden against same-cwd races (one in-flight `thread/start` per cwd at a time). For context: **UI shows an estimate or cumulative-token count for Codex only**, clearly labeled distinct from Claude Code's authoritative `%`.
- **Fallback:** `codex exec --json` as the degraded adapter path (coarser status, weaker association) `[MVP simplification]`.
- **Test / validation signal:** CI schema-regen job diffs the pinned app-server schema; adapter conformance test drives `thread/start{cwd}` → asserts returned id is captured before any output; same-cwd double-start test asserts no cross-wired status.
- **In ARCH.md?** **Y** — §Harness Adapter Layer (Codex model), §Telemetry/Context Projection.

### TR-03 — Claude SDK / statusLine / transcript JSONL schema churn (fast v2.1.x cadence)
- **Category:** integration | **Severity:** high | **Likelihood:** high
- **Source:** `RESEARCH.md R-CC` remaining risk; ADR-006. CLI ~v2.1.167; SDK ~0.3.x.
- **Risk:** Claude Code ships fast; the three data surfaces we merge (SDK message stream, statusLine JSON, transcript JSONL) churn. Concretely, **`total_input_tokens` semantics already changed at v2.1.132** — silently mis-merging usage corrupts cost/context projections. statusLine goes quiet when idle (debounced heartbeat) and can be mistaken for a dead session.
- **Mitigation `[locked]`:** **Pin a tested Claude Code version**, **re-verify the schema on every upgrade** (golden-fixture regression), and isolate the three surfaces behind the adapter's `telemetry-heartbeat` + `read-transcript` contract so the merge logic is one well-tested place. Treat statusLine silence-while-idle as "idle," not "dead," via `refreshInterval` + SDK-stream cross-check (`RESEARCH.md R-CC`).
- **Fallback:** interactive PTY + hooks-to-disk + statusLine (coarser permission interception) if SDK-driving and PTY-attach conflict.
- **Test / validation signal:** Golden JSONL/statusLine fixtures per pinned version; a `total_input_tokens`-semantics test that fails if the merged usage projection drifts from `ResultMessage.usage` + `total_cost_usd`.
- **In ARCH.md?** **Y** — §Harness Adapter Layer (Claude model), §Telemetry/Usage Projection.

### TR-04 — macOS codesign + notarization of the bundled Python Brain sidecar (Tauri externalBin #11992)
- **Category:** operational | **Severity:** critical | **Likelihood:** med
- **Source:** `RESEARCH.md R-BRAIN`; ADR-005, ADR-011; SPIKE explicitly owed in ADR-005.
- **Risk:** The Brain sidecar is a PyInstaller-bundled CPython MCP server (ADR-005). macOS notarization of a Python sidecar is hard: needs entitlement `com.apple.security.cs.allow-unsigned-executable-memory`, **deep-signing of all bundled libs**, and there is a **known-open Tauri externalBin notarization issue (#11992)**. If unresolved, the app cannot ship a notarized build = an outright **release-blocker** (ADR-011).
- **Mitigation `[locked]` + `[research required]`:** Run the **owed SPIKE EARLY** (ADR-005): produce one real signed + notarized build containing the bundled sidecar before committing the bundled-sidecar topology. Add code-signing/notarization to the build pipeline from day one (`RESEARCH.md` cross-cutting §7).
- **Fallback:** Flip the Brain seam to **streamable-HTTP on 127.0.0.1 + per-launch loopback token** (same FastMCP server code, no bundled binary to notarize) (ADR-005 fallback) — trades the closed-port property for shippability.
- **Test / validation signal:** A CI/manual gate that runs `spctl --assess` + `codesign --verify --deep` + a notarization staple check on the produced `.app`; the sidecar launches under the hardened runtime without Gatekeeper rejection.
- **In ARCH.md?** **Y** — §Project Brain Seam, §Build/Release & Signing (release-blocker callout).

### TR-05 — SQLite single-writer contention under many concurrent agents
- **Category:** technical | **Severity:** med | **Likelihood:** med
- **Source:** `RESEARCH.md R-STORE`, `R-PROC` remaining risk; ADR-003, ADR-004.
- **Risk:** The daemon is the **sole SQLite writer** = the Gateway chokepoint (correct by design, ADR-003/004). But every agent mutation, event append, and projection update funnels through one writer; with many agents mutating at once it becomes a throughput ceiling and could stall intent execution.
- **Mitigation `[locked]`:** Keep the single in-process serialized write queue; `BEGIN IMMEDIATE` for writes, `busy_timeout` 5–15s, `synchronous=NORMAL`, periodic `wal_checkpoint(TRUNCATE)` to avoid checkpoint starvation (`RESEARCH.md R-STORE`). **WAL means readers never block the writer**, so the UI stays responsive even when writes queue. Event + outbox + projection in **one txn** (ADR-003). Explicitly **do NOT rely on `BEGIN CONCURRENT`** (not a safe bet) — serialize instead.
- **Fallback:** Tighten to a stricter single in-process write queue; add an append-only JSONL audit mirror for fail-closed durability regardless (`RESEARCH.md R-STORE` fallback) `[deferred work]`.
- **Test / validation signal `[research required]`:** Load test = N concurrent agents (start at N=20) each submitting mutating intents; measure write-queue depth, p95 intent-commit latency, and reader latency. Quantify the ceiling and document it; assert reads stay sub-100ms while writes queue.
- **In ARCH.md?** **Y** — §Event Store & Concurrency Model.

### TR-06 — Lease-lock expiry is probabilistic → fencing tokens mandatory, race-free acquire/renew under WAL
- **Category:** data | **Severity:** high | **Likelihood:** med
- **Source:** `RESEARCH.md R-PROC` remaining risk; ADR-008.
- **Risk:** Cross-restart locks are a SQLite LEASE table (ADR-008); lease expiry is inherently time-based and probabilistic. A paused/stalled holder whose lease expired could resume and mutate a worktree/repo another owner now holds — double-mutation / corruption. OS advisory/flock locks were REJECTED because they don't survive restart (ADR-008).
- **Mitigation `[locked]`:** **Fencing tokens MANDATORY** — monotonic per resource; every executor records the fencing token it acted under and **rejects any mutation carrying a stale (lower) token** (Kleppmann pattern). Acquire/renew SQL must be **race-free under WAL** (owner-guarded `UPDATE ... WHERE owner_id=? AND fencing_token=?`; expired-lease reclaim bumps the token). Lease holdership is projectable to the UI ("who holds this worktree").
- **Fallback:** Shorten lease TTL + raise heartbeat frequency; on any fencing-token rejection, surface a hard conflict to the user rather than auto-resolving `[MVP simplification]`.
- **Test / validation signal:** Race test = two owners contend for one `resource_id` across a simulated daemon restart; assert exactly one holds a valid (highest) fencing token and stale-token mutations are rejected at the executor. Property test on monotonicity.
- **In ARCH.md?** **Y** — §Locking & Leases, §Action Gateway Executors.

### TR-07 — Orphaned / zombie processes (daemon + Brain sidecar + agent children)
- **Category:** operational | **Severity:** high | **Likelihood:** high
- **Source:** `RESEARCH.md R-STACK`/`R-BRAIN`/`R-PROC` (the most error-prone part of wrapper apps); ADR-002, ADR-005.
- **Risk:** The detached daemon owns PTYs, spawns agent CLIs (which spawn their own children), and manages the Python sidecar. Orphans/zombies are "the most error-prone part of wrapper apps." PyInstaller single-file is hard to fully terminate; a crashed daemon can leak whole process trees that keep PTYs and resources alive.
- **Mitigation `[locked]`:** **Process groups** for every spawned tree (`process_group(0)` / `command_group`); kill by **process-group**, not pid (ADR-005's "process-group kill + backoff"). Brain lifecycle = `ping` health + process-group SIGTERM→SIGKILL with backoff (ADR-005). On daemon startup, reap orphans from the prior run via recorded pgids in the lease/process table; `pidlock` single-instance prevents a second daemon racing the first (ADR-008). `[research required]` Windows Job Objects deferred with non-macOS (TR-12).
- **Fallback:** A startup "stale process sweep" that matches NexusOps-spawned pgids and force-kills before re-launching; surface leaked-process count to the user.
- **Test / validation signal:** Crash-kill the daemon (SIGKILL) mid-session; assert no orphaned agent CLI, Brain sidecar, or PTY child survives the next clean start; assert sidecar fully terminates (no lingering PyInstaller process).
- **In ARCH.md?** **Y** — §Process Model & Lifecycle, §Project Brain Lifecycle.

### TR-08 — Codex rollout JSONL world-readable 0644 (#21660)
- **Category:** security | **Severity:** high | **Likelihood:** high
- **Source:** `RESEARCH.md R-CODEX` (OPEN upstream bug #21660); ADR-006; trust boundary `DESKTOP_FIRST_RUNTIME §5`.
- **Risk:** Codex writes rollout transcripts to `~/.codex/sessions/.../rollout-*.jsonl` **world-readable (0644)** — full conversations, tool calls, and usage exposed to any local user/process. Directly violates the local trust boundary the product depends on.
- **Mitigation `[locked]`:** The Codex adapter **hardens these files to 0600 on discovery** (chmod after each read/tail), or prefers **`codex --ephemeral`** + NexusOps's own store as the source of truth (ADR-006). Same hardening posture applies to `~/.codex/sessions` dir perms.
- **Fallback:** `--ephemeral`-only mode (don't depend on Codex's on-disk rollout at all) `[MVP simplification]`.
- **Test / validation signal:** After a Codex session, assert `stat` on every rollout JSONL is `0600`; CSO/security review (`/cso`) checks no `confidential`+ content sits in a world-readable path.
- **In ARCH.md?** **Y** — §Trust Boundary, §Harness Adapter (Codex transcript hardening).

### TR-09 — Agent prompt / command injection
- **Category:** security | **Severity:** high | **Likelihood:** med
- **Source:** `RESEARCH.md R-CC` (injection safety); ADR-006; `ACTION_GATEWAY §4.2` typed actions.
- **Risk:** Untrusted content (ticket bodies, PR text, file contents, Brain proposals) flows into agent prompts/commands. Interpolating it into CLI args enables command injection; oversized input can DoS or exfiltrate.
- **Mitigation `[locked]`:** Feed all untrusted input via **SDK streaming-input / stdin, never interpolated CLI args** (`RESEARCH.md R-CC`); enforce a **10MB stdin cap**. Agent mutations are still typed intents through the Gateway (ADR-004), so even a hijacked agent can't bypass risk classification/preview/approval (`ACTION_GATEWAY §6`/`§7`).
- **Fallback:** Sanitize + length-truncate at the adapter boundary; quarantine oversized payloads with a user warning.
- **Test / validation signal:** Injection fixtures (e.g. ticket body containing shell metacharacters / `--dangerously` flags) assert no arg interpolation occurs and the 10MB cap rejects oversized stdin; Gateway still classifies the resulting mutation.
- **In ARCH.md?** **Y** — §Harness Adapter (input handling), §Action Gateway (defense-in-depth).

### TR-10 — Brain over-reach / silent mutation
- **Category:** security | **Severity:** high | **Likelihood:** med
- **Source:** PRD RISK-4 (deepened); `RESEARCH.md R-BRAIN`; ADR-004, ADR-005; `PROJECT_BRAIN_INTERFACE §8`.
- **Risk:** The Brain reasons but must never execute. A design slip (Brain holding a DB handle, calling an executor, or writing files directly) would let it mutate silently, defeating the audit model (`ACTION_GATEWAY §4.1` "no invisible mutation").
- **Mitigation `[locked]`:** **All Brain mutations go through the Gateway only** as proposals/intents (ADR-004/005); Brain tools are proposals/queries by contract (ADR-005); Brain opens no port and is not a SQLite writer (ADR-003/005). Every Brain-originated action carries `actor=brain` in the event envelope (`EVENT_MODEL §7`) and lands on the Brain action chain (`SHARED_OBJECT_MODEL §35.3`).
- **Fallback:** MVP Brain in read-only/draft mode only (no confirmed-action mode) `[MVP simplification]`.
- **Test / validation signal:** Architecture invariant test — the Brain seam has **no** import/handle path to the SQLite writer or executors; every Brain action appears in the audit timeline with `actor=brain` and an approval state.
- **In ARCH.md?** **Y** — §Project Brain Seam, §Action Gateway Actors.

### TR-11 — Brain absent / stale → platform must degrade gracefully (decoupled)
- **Category:** operational | **Severity:** med | **Likelihood:** med
- **Source:** `RESEARCH.md R-BRAIN`; ADR-005 (daemon-owned lifecycle, ping + backoff); `PROJECT_BRAIN_INTERFACE §1` boundary.
- **Risk:** The Brain is a sibling product with its own store; it may be uninstalled, crashed, mid-restart, or returning stale data. If the control plane hard-depends on it, the core cockpit (sessions/terminals/git/Gateway) becomes unusable when the Brain is down.
- **Mitigation `[locked]`:** Brain is **decoupled** — daemon owns its lifecycle (ping + process-group kill + backoff, ADR-005) and the platform's core loop **does not block on the Brain**. Brain notifications flow through the events adapter (ADR-005) and surface in a drawer (`PROJECT_BRAIN_INTERFACE §6`); when absent, the drawer shows a clear "Brain unavailable" state and everything else works.
- **Fallback:** Disable Brain features entirely via a feature flag; cache last-known Brain outputs and mark them stale.
- **Test / validation signal:** Kill the Brain sidecar; assert sessions/terminals/git/Gateway remain fully functional and the UI shows a non-blocking degraded state; backoff re-spawn works.
- **In ARCH.md?** **Y** — §Project Brain Seam (degradation contract).

### TR-12 — Tauri Linux / WebKitGTK (deferred by macOS-only MVP) — P1 re-open risk
- **Category:** technical | **Severity:** med | **Likelihood:** low (MVP) / high (if Linux re-opens)
- **Source:** `RESEARCH.md R-STACK` ("the one material risk"); ADR-001 (macOS-only MVP).
- **Risk:** Tauri maintainers cannot fully recommend Tauri where strong Linux support is needed now (WebKitGTK instability, broken video/WASM); CEF/Servo replacements aren't production-ready. The macOS-only MVP (ADR-001) **defers, not solves** this — adding Linux means a third webview engine (3× rendering QA) for this terminal-heavy UI.
- **Mitigation `[deferred work]`:** Explicitly out of MVP scope (ADR-001). Keep the daemon/core Rust and webview-agnostic so a future engine swap is contained. Flag this as a **P1 re-open risk** at any Linux planning gate.
- **Fallback:** Electron (single Chromium target, no WebKitGTK risk, battle-tested node-pty + xterm.js) is the documented stack fallback if Linux becomes day-one (`RESEARCH.md R-STACK`).
- **Test / validation signal `[research required]`:** Early Linux spike (only if re-opened) benchmarking 20+ concurrent live PTYs under WebKitGTK before committing.
- **In ARCH.md?** **Y** — §Platform Targets & Non-Goals (explicit deferral + re-open trigger).

### TR-13 — Detached-daemon lifecycle complexity (single-instance, stale socket, UI↔daemon version skew)
- **Category:** operational | **Severity:** high | **Likelihood:** med
- **Source:** `RESEARCH.md R-PROC`; ADR-002 (detached daemon), ADR-004 (UDS transport), ADR-008 (pidlock + versioned IPC handshake).
- **Risk:** A long-lived detached daemon introduces a class of failure modes the in-process model avoids: a second daemon racing the first (single-instance), a **stale UDS socket file** left after a crash blocking re-bind, and **UI↔daemon version skew** (a freshly-updated Tauri UI reattaching to an old daemon with a changed IPC contract → silent protocol mismatch).
- **Mitigation `[locked]`:** `pidlock` single-instance guard (ADR-008); on startup, detect + remove a **stale UDS socket** (connect-probe then unlink) before binding (ADR-004); a **versioned IPC handshake** on every reattach (ADR-008) that refuses mismatched versions and prompts the UI to restart/relaunch the daemon. UDS uses kernel peer auth, no TCP port (ADR-004) — closes the loopback-reachable-by-any-process hole.
- **Fallback:** On version skew, the UI offers "restart daemon" (clean relaunch picks up the new binary); on stale-socket failure, a guided "another instance may be running" recovery.
- **Test / validation signal:** Start two daemons → second refuses via pidlock; kill -9 the daemon then restart → stale socket is reclaimed and bind succeeds; bump IPC version on one side → handshake refuses and surfaces a skew error (no silent mismatch).
- **In ARCH.md?** **Y** — §Process Topology, §IPC / Gateway Transport, §Versioning & Handshake.

---

## Coverage map (ensure every called-out risk is present)

| Required item (from task spec) | Risk ID(s) |
|--------------------------------|-----------|
| (a) PTY/session survival incl. lossy alt-screen re-attach | TR-01 |
| (b) Codex protocol churn + no settable id + no context-% | TR-02 |
| (c) Claude SDK/statusLine/JSONL churn (`total_input_tokens` @ v2.1.132) | TR-03 |
| (d) macOS notarization of bundled Python Brain sidecar (#11992) | TR-04 |
| (e) SQLite single-writer contention (no `BEGIN CONCURRENT`) | TR-05 |
| (f) lease expiry probabilistic → mandatory fencing tokens | TR-06 |
| (g) orphaned/zombie processes (daemon + sidecar + agents) | TR-07 |
| (h) Codex rollout JSONL 0644 (#21660) → 0600/ephemeral | TR-08 |
| (i) agent prompt/command injection (stdin, 10MB cap) | TR-09 |
| (j) Brain over-reach / silent mutation | TR-10 |
| (k) Brain absent/stale → graceful degradation | TR-11 |
| (l) Tauri Linux/WebKitGTK P1 re-open | TR-12 |
| (m) detached-daemon lifecycle complexity | TR-13 |
| Inherited PRD §20 RISK-1..10 | PR-01..PR-10 |

---

## Open questions for `/arch-finalize` to resolve `[open question]`

1. **TR-01:** Is MVP survival "live across full quit" (ADR-002 promise) or "resume-or-replay-and-relaunch" (ADR-010 reality)? Pin the exact guarantee per harness in ARCHITECTURE.md so the UI never over-promises.
2. **TR-05:** What is the documented concurrent-agent write-throughput ceiling, and at what N does intent-commit p95 exceed the UX budget? Needs the load test before ARCHITECTURE.md states a number.
3. **TR-04:** Bundled sidecar (ADR-005 primary) vs loopback-HTTP Brain (ADR-005 fallback) — the notarization SPIKE outcome decides; ARCHITECTURE.md should state the decision criterion, not pre-commit.
4. **TR-02 / TR-03:** Define the exact pinned Codex + Claude Code versions and the CI gate that fails on schema drift — name them in ARCHITECTURE.md §Versioning.
5. **PR-09 / TR-10:** Confirm the Brain-bound projection's sensitivity filter set for MVP (`public`/`internal` only?) in `EVENT_MODEL §9` terms.

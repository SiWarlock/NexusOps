# NexusOps Threat Model v0.1 (Brain 1 / arch-draft — ROUGH DRAFT)

Status: **rough draft for adversarial finalization.** This is the first consolidated
threat model for NexusOps. Until now security content was scattered across
`docs/domains/ACTION_GATEWAY.md` (risk model §7, policy §12.2, locks §16.2,
audit §17.1), `docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md` (sensitivity §9),
and `docs/architecture/DESKTOP_FIRST_RUNTIME.md` (security boundary §5). This doc
pulls them into one place, enumerates trust boundaries, and gives a threat/mitigation
table with MVP-vs-deferred tags. It does **not** restate those docs — it references
their anchors and fills the gaps.

Every claim is tagged: **[locked]** (from `docs/planning/DECISIONS.md`, do not
re-litigate), **[proposed]** (recommendation, finalizer may change), **[open-q]**
(unresolved), **[mvp]** (in MVP threat scope), **[mvp-simplification]** (deliberately
weaker for MVP), **[deferred]** (post-MVP), **[research]** (spike owed).

---

## 0. Scope, attacker model, and one-line posture

**Posture in one line:** NexusOps is a *single-user, single-machine, local-runtime*
tool whose entire security story rests on one idea — **the local machine is the trust
boundary, and the daemon is the only thing allowed to mutate anything** (filesystem,
git, credentials, external APIs) on behalf of any AI agent or the Brain. Every other
control (UDS peer auth, the Action Gateway chokepoint, redaction-before-persist,
fencing-token locks) exists to keep that one idea true under adversarial inputs that
flow *through* the agents we host.

**Primary attacker we DO defend against in MVP [mvp]:**
- A **compromised / jailbroken / prompt-injected AI agent** (Claude Code or Codex)
  running under our daemon that tries to exfiltrate secrets, run destructive git, or
  mutate outside its session scope. This is the realistic, day-one threat: we are
  *deliberately hosting semi-trusted code-executing agents* and pointing them at the
  user's real repos and real credentials.
- A **malicious or careless prompt / task** (from the user, from Linear, from a
  Workflow Pack) that steers an agent into a destructive action.
- A **buggy or hostile Workflow Pack** that tries to run arbitrary scripts or install
  binaries without approval.
- A **local non-NexusOps process** on the same machine probing our IPC socket.
- **Secrets leaking into the durable event/audit log** (the most likely *accidental*
  breach, since transcripts and terminal output are full of tokens and keys).

**Attacker we do NOT defend against in MVP** — see §6 non-goals: remote/network
attackers beyond localhost, multi-user RBAC, a fully compromised host OS / root, a
malicious signed-and-notarized OS, or a user who is themselves the adversary against
their own machine.

---

## 1. Assets to protect

Ranked roughly by blast radius. Each asset notes its store and its default
sensitivity class (5-level model, EVENT_MODEL §9).

| # | Asset | Where it lives | Default sensitivity (§9) | Why it matters |
|---|-------|----------------|--------------------------|----------------|
| A1 | **User source code / working trees** | local FS, git worktrees (ADR-007 dual-git) | `confidential` | The product *is* the user's code; destructive git or leaked diffs are the worst-case. |
| A2 | **Local git credentials** (`~/.gitconfig`, credential helper, SSH keys) | OS keychain / FS, used by `git` CLI mutations | `secret` | Theft → push to arbitrary remotes as the user. |
| A3 | **Claude Code auth context** | `~/.claude/` (token/session), Agent SDK env | `secret` | Theft → impersonate user's Claude subscription/API. |
| A4 | **Codex auth context** | `~/.codex/` + `codex app-server` env | `secret` | Same, for Codex. |
| A5 | **GitHub token** (octocrab; reused `gh` token or Device-Flow OAuth, ADR-007) | OS keychain | `secret` | Repo read/write, PR creation, org access. |
| A6 | **Linear token** (PKCE loopback or pasted key, 24h refresh, ADR-007) | OS keychain | `secret` | Issue read/write, workspace data. |
| A7 | **OS keychain secrets (aggregate)** | macOS Keychain via `keyring` crate (ADR-007/011) | `secret` | The single vault for A2/A3/A4/A5/A6; ACLs depend on a stable signed identity. |
| A8 | **Session transcripts** (Claude JSONL `~/.claude/projects/.../<id>.jsonl`; Codex rollout JSONL `~/.codex/sessions`) | local FS | `restricted` | Contain pasted secrets, full code, prompts; default-restricted (§9). Codex rollout files ship `0644` — must harden to `0600` (ADR-006, issue #21660). |
| A9 | **Project Brain indexes / embeddings** | Brain-owned store (sibling product; seam only, ADR-005) | `confidential`/`restricted` | If built from un-redacted code/transcripts they become a secondary leak surface; embeddings can memorize secrets. |
| A10 | **Event / audit log** (events + projections + outbox, SQLite WAL, ADR-003) | `~/.../nexusops.db` (single-writer daemon) | mixed; rows carry per-event `sensitivity` | The system of record for "what happened." Tampering or secret-injection here is high-value and durable. |
| A11 | **The user's machine itself** (PTYs, FS, processes, network egress) | local | n/a | We broker shell-capable agents; the ultimate asset is "the daemon never becomes a confused deputy for arbitrary local harm." |
| A12 | **Lease / lock table integrity** (ADR-008) | SQLite `lease` table | `internal` | If fencing/leases are bypassed, two writers can corrupt a worktree or the store. |

---

## 2. Trust boundaries

Notation per boundary: **what crosses**, **who controls each side**, **validation**,
**threats**, **mitigations** (tagged), **MVP stance**.

### [a] Local machine = THE trust boundary [locked — DESKTOP_FIRST_RUNTIME §5]

This is the root assumption. Everything inside the user's macOS user account is the
trusted zone; the desktop app brokers **all** FS / git / PTY / credential access on
behalf of agents and the Brain. There is no expectation of defending against a process
already running as the same OS user with full privileges (that is the OS's job).

- **Crosses:** nothing crosses *this* boundary in MVP except via the OS itself
  (keychain, FS perms, process isolation) and the future iOS relay (boundary [f]).
- **Controls:** user / macOS on one side; NexusOps daemon as the privileged broker
  inside.
- **Threats:** confused-deputy (an agent inside the trust zone causes the daemon to do
  harm the user didn't intend); accidental egress of secrets to the network via an agent.
- **Mitigations:** the entire rest of this doc — chokepoints, redaction, least
  privilege. **[mvp]**
- **MVP stance:** **single-user, single-machine, fully trusted local OS user.** We do
  *not* sandbox the daemon from the rest of the user account in MVP. **[mvp-simplification]**
- **[open-q]** Should agent subprocesses run under macOS App Sandbox / a restricted
  profile (e.g. `sandbox-exec`, seatbelt) to limit FS reach to declared worktrees?
  Strongly desirable but interacts badly with agents that legitimately roam `$HOME`.
  Flagged for finalizer. **[research]**

### [b] UI (Tauri webview) ↔ daemon over UDS [locked — ADR-002, ADR-004]

The Tauri UI is a *reattaching client* (ADR-002); the daemon is detached and
long-lived. They talk over a **Unix-domain socket**, length-prefixed JSON-RPC, with a
**versioned handshake** (ADR-004).

- **Crosses:** GatewayPort RPCs (intents, queries, subscriptions), Terminal Channel
  frames (ADR-009 headless VT state → Tauri Channel → xterm.js), event/projection
  streams.
- **Controls:** daemon owns the socket and is the sole SQLite writer (ADR-003/004);
  the webview is the lower-trust side (it renders remote/untrusted-ish content, e.g.
  rendered transcripts, markdown from agents).
- **Validation:**
  - **Kernel peer auth via `SO_PEERCRED`** (Linux-ism) / on macOS use
    `getsockopt(LOCAL_PEERCRED)` / `getpeereid()` to confirm the connecting peer's
    **uid == daemon's uid**; reject otherwise. **[proposed — ADR-004 says "kernel peer
    auth"; the exact macOS syscall is the implementer's choice]** **[research]**
  - **Filesystem permissions** on the socket path: socket in a `0700` dir under the
    user's data dir, socket node `0600`, so no other user can `connect()`. **[mvp]**
  - **Versioned handshake**: first frame negotiates protocol version; mismatch →
    structured error + disconnect (prevents a stale UI from issuing malformed intents).
    **[locked — ADR-004]**
  - **No TCP port, ever** (ADR-004) — removes the entire remote-attacker surface.
    **[locked]**
- **Threats:** another local process (possibly another malicious agent spawned outside
  NexusOps) connects to the socket and submits intents as if it were the UI; malformed
  / oversized frames as DoS; a compromised webview (XSS via rendered agent output)
  issuing intents.
- **Mitigations:** peer-uid check + socket perms (above) **[mvp]**; strict
  length-prefix bounds + frame-size cap + per-connection rate limit **[mvp]**;
  treat *all* webview-originated intents as untrusted and re-validate server-side
  (never trust the client to have enforced risk/approval — the daemon's Action Gateway
  re-decides) **[mvp]**; Tauri CSP locked down, no `nodeIntegration`-style escapes,
  Tauri `withGlobalTauri` minimized **[proposed]**.
- **[open-q]** UDS peer-uid is necessary but not *sufficient* to distinguish "our UI"
  from "another local process owned by the same user." MVP accepts any same-uid peer
  (single-user assumption). A future capability token in the handshake (daemon hands
  the UI a per-launch nonce out-of-band) would tighten this. **[deferred]**

### [c] daemon ↔ Project Brain stdio MCP sidecar [locked — ADR-005]

The Brain is a **stdio MCP sidecar** (FastMCP, PyInstaller), daemon-owned lifecycle
(ping + process-group kill + backoff). It **opens no port**. The seam is the only thing
in scope; Brain internals are a sibling product (ADR-005, PROJECT_BRAIN_INTERFACE §1).

- **Crosses:** MCP tool calls *from* daemon → Brain (queries) and Brain → daemon
  (**proposals only** + queries); MCP notifications → events adapter (ADR-005).
- **Controls:** daemon owns the process group and stdio pipes; Brain is the lower-trust
  side (it reasons over project data and could be steered by injected content).
- **Validation:** Brain tools are typed; **the Brain can only emit proposals/queries,
  never mutations** — a Brain "proposal" is an intent that still flows through the
  Action Gateway and still requires risk-gated approval (PROJECT_BRAIN_INTERFACE §7,
  §8). **[locked]**
- **Threats:** **Brain over-reach / silent mutation** — Brain attempts to mutate
  directly or smuggles a destructive action inside a proposal; Brain ingests injected
  text (from a transcript or issue) and emits a malicious proposal; Brain index becomes
  a secondary secret store (asset A9).
- **Mitigations:**
  - Architecturally the Brain has **no mutation path** — it cannot reach SQLite or git;
    it can only call MCP tools that produce *proposals* (ADR-004/005). **[locked][mvp]**
  - All proposals are **rendered for human review with preview/dry-run** before any
    Gateway execution (ACTION_GATEWAY §13). **[mvp]**
  - **Redact before embed** — content handed to the Brain for indexing goes through the
    same redaction as persist (see §4). **[proposed][mvp]**
  - Daemon-owned lifecycle with process-group kill prevents a wedged/hostile sidecar
    from lingering. **[locked]**
  - **[research]** Notarization spike owed: the bundled PyInstaller Python sidecar must
    be deep-signed + notarized (Tauri externalBin #11992 +
    `com.apple.security.cs.allow-unsigned-executable-memory`) — ADR-005. Until that
    lands, an *unsigned* sidecar is itself a supply-chain/tamper surface.

### [d] daemon ↔ agent subprocesses (Claude / Codex) [locked — ADR-006]

The highest-risk boundary. We host two shell-capable AI agents and point them at real
repos. One `HarnessAdapter` contract over two models (ADR-006): Claude via Agent SDK
streaming + `can_use_tool` callback; Codex via `codex app-server --stdio` JSON-RPC.

- **Crosses:** prompts/inputs **into** the agent (stdin, app-server JSON-RPC, SDK
  messages); status/telemetry/transcript **out**; **tool-use / mutation requests** that
  the agent wants to perform (intercepted at the in-harness chokepoint).
- **Controls:** daemon spawns & owns the PTY (portable-pty, ADR-009) and the app-server
  pipe; the **agent is untrusted** (it runs model-generated, prompt-injectable code).
- **Validation / chokepoints:**
  - **Claude:** `can_use_tool` callback is the **in-harness mutation chokepoint** —
    the daemon sees every tool the model wants to run and can route it to the Action
    Gateway for policy + approval before it executes (ADR-006). **[locked]**
  - **Codex:** host-routed approvals via app-server — approval requests come to the
    daemon, not auto-granted (ADR-006). **[locked]**
  - **Command/prompt injection defense is structural:** pass prompts/inputs via
    **stdin / structured JSON-RPC, never via shell `args`** (ADR-006), so a malicious
    prompt cannot become argv that the shell re-interprets. **[locked][mvp]**
  - **NEVER scrape PTY for machine state** (ADR-006) — the PTY mirror is for *human
    display only*; machine decisions come from SDK/app-server structured channels. This
    also prevents an agent from spoofing state by printing fake control text to the PTY.
    **[locked][mvp]**
- **Threats:** prompt/command **injection driving destructive git** (e.g. injected text
  → `git push --force`, `git reset --hard`, branch deletion, history rewrite);
  credential exfiltration (agent reads `~/.ssh`, keychain-helper output, env, then
  egresses); agent escaping its session's worktree to touch other projects;
  PTY-spoofing of machine state; agent issuing high-risk network calls.
- **Mitigations:**
  - Route **all** agent-requested mutations through the Action Gateway (§3) where they
    are typed, risk-classified 0–4, previewed, approval-gated, and audited
    (ACTION_GATEWAY §7, §8, §13). Destructive git (force-push, hard reset, branch
    delete, history rewrite) is **risk 3–4 → explicit per-action human approval**
    (ACTION_GATEWAY §7.4/§7.5). **[mvp]**
  - **Worktree scoping:** each Session operates in a declared worktree (ADR-007 worktree
    lifecycle); Gateway executors should reject git mutations whose target path is
    outside the session's worktree. **[proposed][mvp]**
  - Harden transcript files to `0600` (Codex rollout #21660, ADR-006) so a *second*
    agent can't read the first's transcript (which contains secrets). **[mvp]**
  - **[open-q]** Egress control: in MVP we do **not** firewall agent network access
    (agents legitimately call their model APIs and package registries). A compromised
    agent *can* exfiltrate over its own allowed egress. True egress isolation
    (per-agent network namespace / proxy allowlist) is **[deferred]**; flagged as the
    biggest residual MVP risk. **[research]**
  - **[open-q]** Should we deny the agent direct `git`/`gh`/`ssh` binaries on `PATH`
    and force *all* git through the Gateway? Today an agent under `can_use_tool` can
    still try to invoke `git` as a shell tool — that tool call is *visible* and
    *gateable*, but the policy default for shell tools needs to be conservative.
    Finalizer to set the default policy. **[open-q]**

### [e] daemon ↔ external integrations (GitHub / Linear) [locked — ADR-007]

- **Crosses:** API calls over **TLS** to GitHub (octocrab) and Linear (`@linear/sdk`);
  tokens read from keychain; OAuth Device Flow (GitHub) / PKCE loopback (Linear).
- **Controls:** daemon (trusted client) ↔ remote provider (TLS-authenticated, but
  treated as an external boundary).
- **Validation:** standard TLS cert validation (no custom CA, no pinning in MVP
  **[mvp-simplification]**); OAuth Device Flow / PKCE so we never handle a password;
  Linear 24h token refresh (ADR-007); GitHub token bootstrapped from existing `gh`
  auth where present, else Device Flow.
- **Threats:** token theft from keychain (covered by A7 mitigations); token leaking
  into logs/transcripts (covered by §4 redaction); over-broad OAuth scopes; SSRF-style
  abuse where an injected agent tricks the Gateway into calling an attacker-chosen
  GitHub/Linear endpoint.
- **Mitigations:** tokens **only** in keychain, never in the DB or event payloads
  (§4) **[mvp]**; request **least-privilege OAuth scopes** (ADR-007 / ACTION_GATEWAY
  §4.7) **[proposed][mvp]**; the **set of reachable endpoints is fixed by typed action
  executors** (ADR-006 / ACTION_GATEWAY §15.3) — an agent cannot ask the Gateway to hit
  an arbitrary URL, only to perform a typed action (e.g. `github.pr.create`), which
  bounds SSRF **[mvp]**; **startup self-test** of keychain availability per-OS
  (ADR-007) so a silent keychain failure doesn't degrade into plaintext fallback
  **[mvp]**.
- **[deferred]** Cert pinning, token scope downgrade UI, per-integration kill switch.

### [f] FUTURE iOS companion → encrypted relay → daemon → Action Gateway [locked seam — DESKTOP_FIRST_RUNTIME §6 / EVENT_MODEL §2.2]

Out of MVP scope, but the seam is designed now so MVP choices don't preclude it.

- **Crosses (future):** redacted event **projections** down to the phone; **approvals /
  intents** up from the phone — **never raw shell, never raw transcripts** (DESKTOP_FIRST
  §6.2 hard boundary; EVENT_MODEL §2.2/§3.5).
- **Controls:** phone (lowest trust, leaves the machine) ↔ encrypted relay ↔ daemon.
- **Hard rules carried forward [locked]:** iOS reads **redacted projections only**
  (sensitivity ≤ `internal`, never `restricted`/`secret`); iOS can submit **intents and
  approvals** that *still* land in the Action Gateway and *still* obey risk gating; the
  phone gets **no PTY, no FS, no credential** access.
- **MVP stance:** **not built.** The design seam = (1) projections are already
  redaction-classified per event (§4), and (2) every mutation already funnels through
  the Gateway, so "add a remote intent source later" requires no new mutation path.
  **[mvp-simplification / deferred]**
- **[open-q]** relay auth model (device pairing, E2E key exchange) — deferred, but must
  not be retrofitted as a TCP port on the daemon (would violate ADR-004); the relay must
  be an *outbound* daemon connection. **[deferred][research]**

---

## 3. The Action Gateway as the central security control [locked — ACTION_GATEWAY]

Every boundary above ([c] Brain, [d] agents, [f] future iOS) terminates at the same
chokepoint: **all mutation is a typed intent submitted into the Action Gateway; nothing
mutates directly** (ADR-004, ACTION_GATEWAY §2.1/§4.1 "no invisible mutation"). This is
the single most important security property of NexusOps. Summary of the controls it
provides (do not restate the spec — see anchors):

| Control | Anchor | Security role | Tag |
|---------|--------|---------------|-----|
| **Typed actions only** | ACTION_GATEWAY §4.2, MVP types §28.2 | No free-form shell as a "mutation"; bounds SSRF & arbitrary exec. | [locked][mvp] |
| **Risk model 0–4** | §7 | Human control scales with blast radius; destructive git = 3–4 → explicit approval. | [locked][mvp] |
| **Preview / dry-run** | §13 | Human (or future remote approver) sees the diff/plan before it runs. | [locked][mvp] |
| **Policy engine + decision** | §12 / §12.2 | Policy *before* preference (§4.6); deny by default for high risk. | [locked][mvp] |
| **Approval gating** | §7.4/§7.5, §8 lifecycle | The actual confused-deputy brake. | [locked][mvp] |
| **Audit on every action** | §17.1 core events | Tamper-evident record (see §5/T-11). | [locked][mvp] |
| **Redaction before persist** | §4 (this doc) + EVENT_MODEL §9 | Secrets never enter the durable log. | [proposed][mvp] |
| **Idempotency keys** | §16.1 | Replay/retry can't double-apply a mutation. | [locked][mvp] |
| **Fencing-token-guarded locks** | §16.2 + ADR-008 | Two writers can't corrupt one worktree/store. | [locked][mvp] |

**Key invariant for the finalizer to assert as binding:** *there exists no code path by
which an agent, the Brain, or (future) the phone mutates FS / git / external state
except by a typed Action that passed policy + (risk-appropriate) approval and produced
an audit event.* The `can_use_tool` (Claude) and host-routed approvals (Codex)
boundaries are the *enforcement points* for agents; the Brain's proposal-only seam is
the enforcement point for [c].

---

## 4. Data sensitivity & redaction

Builds directly on **EVENT_MODEL §9** (the 5-level model) — do not restate the table;
the levels are `public / internal / confidential / restricted / secret`.

- **Default-conservative classification [locked — §9]:** terminal output and transcripts
  **default to `restricted`** (§9 + SHARED_OBJECT_MODEL Terminal note). Restricted ⇒
  *not synced remotely without explicit consent* (this is what makes boundary [f] safe).
- **Secrets NEVER in event payloads [proposed][mvp]:** the durable store (A10) must
  never contain tokens, keys, or pasted secrets. Where a secret is referenced, store a
  **redacted summary** (EVENT_MODEL §4.5 "redacted summary, not raw pasted secret") and,
  for large artifacts, a **path + content_hash ref** (ADR-003) — never the raw bytes.
- **Redaction happens BEFORE three sinks [proposed][mvp]:**
  1. **persist** → before an event row is written (single-writer daemon, ADR-003);
  2. **embed** → before content reaches the Brain index (boundary [c] / asset A9);
  3. **sync** → before any projection leaves the machine (boundary [f], future).
  The event envelope already reserves `redaction_status` / `redaction_policy_id`
  (EVENT_MODEL §6 / §12 schema) — MVP must populate them, not just reserve them.
- **Codex rollout 0644 → 0600 hardening [locked — ADR-006, issue #21660][mvp]:** Codex
  writes session rollout JSONL world-readable; daemon must `chmod 0600` (or
  pre-create/own the dir) so transcript secrets (A8) aren't readable by other agents/
  processes. Same hardening posture for `~/.codex/sessions` generally.
- **Keychain ACLs need Developer ID signing [locked — ADR-011][mvp / release-blocker]:**
  macOS Keychain ACLs bind secrets (A7) to a **stable code identity**. Without
  Developer ID code-signing + notarization, the app's identity is unstable → keychain
  access prompts on every rebuild and weaker ACL guarantees. Signing/notarization is an
  **early release-blocker** (ADR-011), and is *also* a security control, not just a
  distribution chore.
- **[open-q]** What is the redaction detection engine? Regex/entropy secret scanners
  miss novel formats; an over-aggressive scanner corrupts legitimate code in the log.
  MVP proposal: a curated set of high-recall patterns (known token prefixes:
  `ghp_`, `github_pat_`, `sk-`, `xox`, PEM blocks, `AKIA`, JWT shape) + Shannon-entropy
  fallback on `KEY=value` lines, applied at the persist boundary, with the *original*
  kept only in the non-synced transcript file (which is itself `restricted` + `0600`).
  Finalizer to confirm. **[research]**

---

## 5. Key threats & mitigations (T-IDs)

Risk = qualitative (Hi/Med/Lo) likelihood × impact given the single-user local model.
"Residual" = what's still exposed after MVP mitigations.

| ID | Threat | Boundary | Risk | Mitigations | Tag | Residual |
|----|--------|----------|------|-------------|-----|----------|
| **T-01** | **Credential exfiltration** — agent/Brain reads keychain-helper output, `~/.ssh`, env, or tokens and egresses them. | [d][e] | Hi | Secrets only in keychain (A7); never in DB/payloads (§4); typed-action executors bound which endpoints are reachable; least-privilege OAuth scopes. | [mvp] / egress isolation [deferred] | Agent can still exfil over its own allowed model-API/network egress (no egress firewall in MVP). |
| **T-02** | **Secrets-in-logs** — token/key lands in the durable event/audit log via transcript or terminal capture. | [a][d] | Hi | Redaction-before-persist (§4); terminal/transcript default `restricted`; store refs+hashes not raw bytes (ADR-003); Codex rollout `0600`. | [mvp] | Novel secret formats may slip past the scanner → kept in `restricted` transcript only, never synced. |
| **T-03** | **Prompt/command injection → destructive git** — injected text steers agent to force-push / hard-reset / delete branch / rewrite history. | [d] | Hi | Inputs via stdin/JSON-RPC not argv (ADR-006); all git mutation is typed Gateway actions; destructive ops = risk 3–4 → explicit approval (§7); worktree-scoped executors; idempotency keys (§16.1). | [mvp] | A user who reflexively approves can still be tricked; UX must make destructive previews unmistakable (§13). |
| **T-04** | **Brain over-reach / silent mutation** — Brain tries to mutate directly or smuggles a destructive action into a proposal. | [c] | Med | Brain has *no* mutation path (proposal-only seam, ADR-004/005); proposals are previewed + approval-gated (PB §7/§8); daemon-owned process-group kill. | [mvp] | Brain can still produce *persuasive bad advice*; mitigated only by human review. |
| **T-05** | **Malicious / untrusted Workflow Pack** — pack tries to run arbitrary scripts or install binaries without approval. | [a][d] | Med | Pack **trust levels** (WORKFLOW_PACKS §16.1); no arbitrary script exec / no binary install without explicit approval (§16.1 non-goals, lines anchoring "Do not run arbitrary pack scripts without explicit trust and approval"); pack-requested mutations route through Gateway approval. | [mvp] | Untrusted (locally-imported) packs are the default trust level — relies on user not approving blindly; **no sandbox** for approved pack scripts in MVP. |
| **T-06** | **Local IPC socket snooping / spoofing** — another local process connects to the UDS and submits intents as the UI. | [b] | Med | UDS only, no TCP (ADR-004); peer-uid check (`LOCAL_PEERCRED`/`getpeereid`); socket dir `0700`, node `0600`; versioned handshake; server-side re-validation of every intent. | [mvp] | Same-uid local processes are accepted (single-user model); capability-token handshake is [deferred]. |
| **T-07** | **Supply-chain compromise** — malicious update to Claude/Codex CLI, the MCP sidecar, a Workflow Pack, or a Rust/PyPI dependency. | [c][d] | Med | Pin & lock dependencies (`Cargo.lock`, pinned sidecar build); deep-sign + notarize bundled sidecar (ADR-005 spike); pack trust levels + future signed packs (WORKFLOW_PACKS §16.1); treat agent CLIs as untrusted-by-design (already gated at [d]). | [mvp: pinning/signing] / signed-pack ecosystem [deferred] | We do not yet verify upstream agent-CLI integrity beyond OS-level; SBOM/`cargo-audit` in CI is [proposed]. |
| **T-08** | **Audit-log tampering** — an actor edits/deletes events to hide what happened. | [a][b] | Med | Append-only by default (EVENT_MODEL §4.2); single-writer daemon (ADR-003) — UI/agents/Brain have no write path to SQLite; **reserved `payload_hash` / `previous_event_hash` columns** for a post-MVP hash-chain (ADR-003). | [mvp: append-only + reserved cols] / hash-chain [deferred] | A same-uid attacker with direct SQLite file access can still tamper in MVP (no hash-chain yet); detection-only after chain lands. |
| **T-09** | **Lock/fencing bypass → store/worktree corruption** — two writers act on one resource. | [a] | Med | SQLite LEASE table + **mandatory fencing tokens** + pidlock single-instance (ADR-008); OS advisory/flock explicitly rejected (don't survive restart). Executors must present a valid fencing token (§16.2). | [mvp] | Correctness control; assumes executors actually check the token — finalizer to make "fenced write" a binding invariant. |
| **T-10** | **PTY state spoofing** — agent prints fake control/status text to the PTY to mislead the daemon. | [d] | Lo | **Never scrape PTY for machine state** (ADR-006); machine decisions come only from SDK/app-server structured channels; PTY is human-display-only (ADR-009). | [mvp] | Cosmetic only — can mislead the *human* watching the terminal, not the daemon. |
| **T-11** | **Confused-deputy via external data** — a Linear issue / git remote / web content the agent reads contains injection that steers a mutation. | [d][e] | Med | Same chokepoint as T-03 (typed actions + approval); external inputs are never auto-actioned; redaction on ingest. | [mvp] | Inherent to hosting agents on real data; reduced, not eliminated, by approval gating. |
| **T-12** | **Execution-profile / ToS / subscription-circumvention concern** — orchestrating many agent sessions could violate a vendor's ToS or be perceived as subscription circumvention. | [d][e] | Med (non-technical) | Respect each harness's official entrypoints (Agent SDK, `codex app-server`) — we drive them as documented, not by spoofing clients (ADR-006); surface per-agent session/usage so the user stays within their own entitlements; **do not** multiplex one credential across users (single-user model). | [open-q / policy] | Legal/ToS question, not a code control — flag for product/legal. **[open-q]** |
| **T-13** | **Keychain identity instability → plaintext fallback** — unsigned builds can't hold stable keychain ACLs. | [a][e] | Med | Developer ID signing + notarization (ADR-011, release-blocker); keychain startup self-test (ADR-007) with **hard fail, no plaintext fallback**. | [mvp / release-blocker] | Until signed, dev builds re-prompt and have weaker ACLs. |

---

## 6. Explicit non-goals for MVP threat scope

These are **deliberately out of scope** for the MVP threat model. Listing them is itself
a control: it stops scope creep and tells the finalizer what *not* to demand.

- **Multi-user / RBAC.** Single-user, single OS account. No per-user roles, no shared
  daemon, no tenant isolation. **[mvp-simplification]**
- **Network / remote attackers beyond localhost.** No TCP port exists (ADR-004); we do
  not model an attacker on the LAN/WAN. The future iOS relay ([f]) introduces this and
  is explicitly deferred with its own (unbuilt) auth model. **[deferred]**
- **Defending against a compromised host OS / root / malicious signed OS.** If the OS or
  another same-uid privileged process is fully compromised, the trust boundary [a] has
  already failed; out of scope.
- **Cloud / remote runner.** All execution is on the local machine (ADR-001/002). No
  cloud-runner threat surface in MVP. **[deferred]**
- **Sandboxing agent subprocesses from the user account.** Desirable (see [a] open-q)
  but not MVP. **[deferred / research]**
- **Hash-chained tamper-*proof* audit.** MVP is append-only + single-writer +
  *reserved* hash columns; cryptographic tamper-evidence is post-MVP (T-08). **[deferred]**
- **Egress firewalling of agents.** The single largest residual MVP risk (T-01); not
  built in MVP. **[deferred / research]**

---

## 7. Open questions & spikes owed (for the finalizer)

1. **[research]** Exact macOS peer-auth syscall for UDS ([b]) — `LOCAL_PEERCRED` vs
   `getpeereid`; confirm uid-equality is the right (and only) MVP check.
2. **[research]** Notarization of the bundled PyInstaller Brain sidecar (ADR-005 spike,
   Tauri #11992 + entitlement) — until done, the sidecar is an unsigned tamper surface
   (T-07).
3. **[research]** Redaction engine design (§4 open-q) — pattern set + entropy fallback +
   where it runs; needs a small spike + test corpus of real secret formats.
4. **[open-q]** Agent FS sandboxing / worktree confinement ([a], [d]) — `sandbox-exec`
   seatbelt profile vs accept full-`$HOME` reach.
5. **[open-q]** Default Gateway policy for an agent invoking the `git`/`gh` shell binary
   directly under `can_use_tool` ([d]).
6. **[open-q]** ToS / subscription-circumvention posture (T-12) — product/legal, not eng.
7. **[deferred]** iOS relay auth model ([f]) must be outbound-only (no daemon port).

---

## 8. Cross-references (anchors, not restated)

- Action Gateway controls: `docs/domains/ACTION_GATEWAY.md` §7 (risk), §8 (lifecycle),
  §12.2 (policy result), §13 (preview/dry-run), §15.3 (executors), §16.1 (idempotency),
  §16.2 (locks), §17.1 (audit events), §28.2 (MVP action types).
- Sensitivity & audit: `docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md` §4.2
  (append-only), §4.5 (privacy/redacted summary), §6 (envelope, `redaction_status`),
  §9 (sensitivity model), §12 (schema), §13 (projections), §14 (audit), §16 (Gateway
  relationship).
- Trust boundary: `docs/architecture/DESKTOP_FIRST_RUNTIME.md` §5 (security boundary),
  §6.2 (iOS hard boundary), §6.3 (MVP stance).
- Brain seam: `docs/architecture/PROJECT_BRAIN_INTERFACE.md` §1 (boundary), §7 (action
  planning flow), §8 (safety requirements).
- Workflow Pack trust: `docs/domains/WORKFLOW_PACKS.md` §16.1 (pack trust levels) +
  non-goals (no arbitrary script exec / binary install without approval).
- Locked decisions: `docs/planning/DECISIONS.md` ADR-001…ADR-011.

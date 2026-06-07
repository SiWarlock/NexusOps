# ARCHITECTURE.md — NexusOps

> **Status:** Binding architecture contract for the MVP. Finalized by `/arch-finalize` (Brain 2, Claude/Opus) from the `/arch-draft` rough draft + planning artifacts + a 14-dimension adversarial gap audit (`docs/gap-audits/`), with load-bearing decisions confirmed by the project owner on 2026-06-07.
> **Audience:** Project owner; `tasks-gen`; the cc-crew `/tdd` build crew; technical reviewers.
> **Primary implementation constraint:** Solo build driven largely by AI agents (cc-crew). macOS-only MVP. The owner has chosen a **comprehensive MVP** (both harnesses live, full session survival, full Brain action plans) over a minimal slice — sequence accordingly (§19).
> **Architecture sentence:** *A desktop-first, local-runtime cockpit whose detached Rust daemon is the single, audited mutator of all state — every change is a typed, risk-classified, approved Action recorded as an immutable event; the UI reads projections, agents and the Project Brain only propose intents, and the local machine is the trust boundary.*
> **Build contract:** This file is the source of truth. `tasks-gen` must bind every task to a `§N` anchor here and must not invent architecture; if a task needs architecture absent here, flag it. Companion (non-binding) detail lives in `docs/planning/*` and `docs/gap-audits/*`; the authoritative upstream specs are `docs/product/PRD.md`, `docs/product/PRODUCT_CANON.md`, `docs/architecture/{SHARED_OBJECT_MODEL,EVENT_MODEL_AND_AUDIT_TRAIL,PROJECT_BRAIN_INTERFACE,DESKTOP_FIRST_RUNTIME}.md`, `docs/domains/{ACTION_GATEWAY,WORKFLOW_PACKS,CC_CREW_WORKFLOW_PACK}.md`, `docs/ux/{UX_INFORMATION_ARCHITECTURE,UI_COMPONENT_INVENTORY}.md`.
> **Anchor stability:** `§N` anchors are **stable IDs** — never reused or reordered (LESSONS-style). New sections append; they are not inserted mid-document. `tasks-gen` and the area `CLAUDE.md` cross-doc-invariants table bind to these + to Appendix A.
> **Tag legend:** `[LOCKED]` · `[PENDING-SPIKE]` · `[MVP]` · `[P1]` · `[P2]` · `[DEFERRED]` · `[OPEN]`.

---

## §0 — Decision Ledger

The reconciled, once-and-only-once record of every load-bearing decision. The 11 ADRs (full rationale in `docs/planning/DECISIONS.md`) plus the finalize-pass resolutions of the audit's coupled shared-contract cluster. Where the upstream specs drifted, the canonical choice is recorded here and is binding.

### §0.1 — Owner decisions (2026-06-07)
| # | Decision | Resolution |
|---|---|---|
| O-1 | Codex in MVP | **Both Claude Code AND Codex are MVP-blocking** (owner kept the comprehensive scope; the PRD-permitted Codex-as-P1 was declined). Codex risks (§9.1, RISKS TR-02/TR-08) are managed in-MVP, not deferred. |
| O-2 | Session survival | **Full resume-or-replay in MVP** (`claude --resume` / `codex thread/resume` with accurate alt-screen VT re-render; replay+relaunch as fallback). The #1 risk (TR-01) is accepted into MVP with mandated deterministic test seams (§14). |
| O-3 | Brain write-path | **Full multi-step bundled Brain action plans + step-by-step approval in MVP** (PRD §25 steps 14-16 as-is). The Gateway MUST support `ActionPlan` (not just single `ActionRequest`) in MVP (§6.2). |
| O-4 | Claude supervision mode | **`[PENDING-SPIKE]`** — primary/fallback of SDK-driven (`can_use_tool`) vs interactive-PTY-driven is decided by `OQ-HARN-SPIKE-7` (§24). Both paths kept viable; the `can_use_tool`-holes mitigations are locked regardless (§9.1, O-13). |
| O-5 | Design system | **Adopt `NexusOps-ui-kit` (tokens + components + "Graphite Arc") as the canonical design system** (2026-06-07 prototype review). Color is a re-hueable first pass. The UI reconciliation is folded into §11; detail in `docs/ui-review/UI_RECONCILIATION.md`. |
| O-6 | PR Review Workspace | **Full PR Review surface in MVP** (a review mode in Code/Diff: header/checks/reviews/mergeability/risk/Brain-evidence/agent-summary + merge actions; merge re-fetches GitHub per §7.2 and routes through the Gateway at risk≥3). §11.2. |

### §0.2 — Architecture ADRs (locked spine; status-bearing)
| ADR | Decision | Status |
|---|---|---|
| 001 | All-Rust: Rust **daemon** + **Tauri 2.x** shell; **macOS-only** MVP | `[LOCKED]` |
| 002 | **Detached long-lived daemon from day one** owns PTYs/store/Gateway/adapters/Brain/locks; UI reattaches over IPC | `[LOCKED]` |
| 003 | **SQLite (WAL)**, single-writer = daemon; events + projections + projection_offsets + outbox (one txn); FTS5; `user_version` migrations; artifacts by path+hash | `[LOCKED]` |
| 004 | Action Gateway = **`GatewayPort` trait, exposed over Unix-domain-socket IPC**, JSON-RPC; execution **in-daemon**; agents/Brain/UI submit **intents** only | `[LOCKED]` |
| 005 | Project Brain = **stdio MCP sidecar** (FastMCP 3.x, daemon-owned lifecycle) | `[LOCKED-PENDING-SPIKE]` — macOS sidecar notarization `#11992` (`OQ-PLAT-SPIKE-1`); fallback = FastMCP streamable-HTTP on loopback (§13.1) |
| 006 | **One `HarnessAdapter` contract over two lifecycle models** — Claude (SDK/PTY) + Codex (`app-server`) | `[LOCKED]`; Claude drive-mode `[PENDING-SPIKE]` (O-4) |
| 007 | **Dual git** (git2 reads / git-CLI mutations), **octocrab** (+gh-token bootstrap), Linear PKCE/key, **keyring** crate | `[LOCKED]`; git2 read-path on `extensions.relativeworktrees` repos re-verified by `OQ-INT-SPIKE-6` (§24) |
| 008 | Cross-restart locks = **SQLite lease table** (owner + monotonic fencing token + heartbeat) + `pidlock` single-instance | `[LOCKED]` |
| 009 | Terminal: **`portable-pty` in daemon → headless VT → Tauri Channel → xterm.js (WebGL)**; backpressure mandatory | `[LOCKED]` |
| 010 | Survival: UI-restart → reconnect-live; daemon-restart → **resume-or-replay** (O-2) | `[LOCKED]` |
| 011 | Secrets in OS keychain; **Developer ID signing + notarization = early release-blocker** | `[LOCKED]` |

### §0.3 — Shared-contract reconciliations (resolved here; were drifting across specs)
Resolved in dependency order (ID format → actor enum → event envelope → state machines → task model), per the gap-audit completeness critic.
| # | Contract | Canonical resolution | Anchor |
|---|---|---|---|
| R-1 | **ID format** | **Prefixed ULID** `<prefix>_<ULID>` (Crockford base32, lexicographically sortable). External IDs (`branch_name`, `commit_sha`, `pr_number`, `linear_issue_id`, `architecture_anchor`) kept native. | §5.2 |
| R-2 | **Actor/requester enum** | Canonical **audit** actor (`Event.actor_type`) = EM §7 set: `user, project_brain, action_gateway, workflow_runtime, local_runner, session_adapter, integration_syncer, system, remote_client, automation_policy` (EM's `remote_device` → **`remote_client`**). **Request-time** `ActionRequest.requester_type` (AG §9.7) aliases map: `agent_session→session_adapter`, `workflow_pack→workflow_runtime`, `system_policy→automation_policy`. Mapping table is binding (Appendix A). | §5.1, §6.2, §7.1 |
| R-3 | **Event envelope** | EM §6 envelope is canonical; actor/source/sensitivity/visibility enums inlined at §7.1; `remote_client` per R-2. | §7.1 |
| R-4 | **Session state machine** | **17 states** — PRD SESS-3 is authoritative: add **`changes_ready`** (was missing from SOM/UX/DATA_MODEL). Waiting token canonicalized to **`waiting_on_human_input`** (event `SessionWaitingOnHumanInput`). | §5.1 |
| R-5 | **Approval vs ActionRequest** | **Split into two machines.** Approval = `{requested, previewed, awaiting_approval, approved, denied, edited, auto_approved_by_policy, expired, cancelled, escalated}`. ActionRequest/execution = `{submitted, previewed, policy_decided, awaiting_approval, approved, denied, expired, cancelled, queued, executing, succeeded, failed, partially_succeeded, rolled_back, rollback_failed}`. ActionRequest is a **9th** first-class state machine. | §5.1 |
| R-6 | **AgentTeam state machine** | Promoted to a first-class machine (SOM §14, 9 states); MVP exercises `draft/starting/active/waiting_on_human/blocked/completed/archived`; a `agent_teams` registry row + `proj_agent_team` projection back the `active_teams` counter. | §5.1 |
| R-7 | **ExecutionProfile / WorkflowInstance / Worktree enums** | ExecutionProfile = `{available, active, in_use, rate_limited, auth_expired, misconfigured, disabled, unknown}` (config vs runtime split, §7.2). WorkflowInstance = PRD WF-3 set with `ready_for_team_run` (not `_mode`); team-run-in-progress tracked on AgentTeam, not the instance. Worktree status = **derived**: git-sync axis (clean/dirty/untracked/conflicts/ahead/behind) + lifecycle overlay (creating/locked/pr_open/merged/prunable/deleted) collapsed by a precedence function (locked/conflicts > dirty > ahead/behind > clean), §7.2. | §5.1, §7.2 |
| R-8 | **Task vs PlanTask** | **One `tasks` table** with `kind ∈ {plan_task, external_task}` + a **superset** state machine; Plan View renders the plan-task subset (`not_started, ready, in_progress, in_review, blocked, done, deferred`); external tasks render the GitHub/Linear subset. Resolves SOM §37 Q2 (ADR-012). | §5.1, §7.2 |
| R-9 | **Illegal/terminal transitions** | Each machine declares its legal edge set + terminal/absorbing states (Appendix A links the enums); an event implying an out-of-set transition is logged as a projection-degraded marker (EM §23), never silently applied. | §5.1, §17 |
| R-10 | **Hash-chain** | `payload_hash`/`previous_event_hash` columns reserved; tamper-evidence chain `[DEFERRED]` (post-MVP). | §7.1 |

---

## Executive summary

NexusOps is a **desktop-first, local-runtime AI engineering control plane** for macOS: one cockpit to **dispatch, supervise, review, deliver, and remember** the work of many AI coding agents (Claude Code + Codex) across multiple local projects (`PRD §1-2`). The **Session** is the atomic operational unit, and a strict chain of ownership (Project → Session → Worktree/Branch → Terminal → Task → Harness → Execution Profile → Diffs → Commits → PR → Brain episode → Events) is preserved on every surface.

The system is **three local processes** plus the agent subprocesses they supervise (§4): (1) the **NexusOps Daemon** (Rust, detached, long-lived) — the trust core and the *only* mutator of state, owning the append-only SQLite event store (sole writer), the Action Gateway (single mutation chokepoint), the harness adapters, PTYs, git/worktree ops, integration syncers, the lease-lock manager, the projection engine, and the Brain sidecar's lifecycle; (2) the **NexusOps UI** (Tauri 2.x — Rust host + system WebView) — a *reattaching client* that reads projections and submits intents over a Unix-domain-socket `GatewayPort`, renders xterm.js terminals, and never writes the DB; (3) the **Project Brain sidecar** (Python/FastMCP, stdio MCP) — a *sibling product* that reasons over project memory and **proposes** action plans but never executes (Brain internals are out of scope here; only the seam is specified).

It is an **event-sourced, projection-driven, capability-gated** design (§7): every important fact is an immutable event in append-only SQLite; the UI reads rebuildable projections; every mutation is a typed, risk-classified, previewed, approved, audited `ActionRequest`/`ActionPlan` through the Action Gateway (§6). Three architectural laws hold (§4.2): single mutation chokepoint; events-are-facts-UI-reads-projections; reason (Brain) vs execute (platform). Both harnesses expose a structured supervision + approval surface that maps onto the Gateway (Claude's `can_use_tool`; Codex's `app-server` approvals) so status and worktree/task association are reliable without scraping terminals (§9.1) — with the important caveat that `can_use_tool` is *not* a complete chokepoint (subagent/background/permission-mode holes, §9.1, O-13), mitigated by default-mode-only + `PreToolUse` defense-in-depth.

The MVP proves the full thesis via the **PRD §25 demo** (§19.1) and is deliberately **comprehensive** (owner decision): both harnesses live, full session survival, full Brain action plans. The hardest risks — terminal survival fidelity, Codex protocol churn, the Claude Agent-SDK credit pool, and the macOS Python-sidecar notarization — are tracked as managed risks and pre-build spikes (§24), not silently assumed away.

---

## §1 — Goals & non-goals

**Goals (MVP)** `[LOCKED]` (`PRD §15`, scoped by the §25 demo):
- One cockpit supervising many local agent sessions (Claude Code + Codex) with reliable, non-scraped status and worktree/task association.
- Session atomic; full ownership chain everywhere; attention-first ordering (`PRD §5.2`).
- Every mutation typed + previewed + risk-classified + approved + audited through the Action Gateway; **no mutation path bypasses it** (INV-SEC-1, §15).
- Append-only event store + rebuildable projections; immutable audit; fail-closed on audit-write; crash-recoverable; UI restart-safe; agents survive UI quit AND resume/replay across daemon restart (O-2).
- Worktree-isolated agent work; git basics + PR creation; manual task/plan/session/worktree/PR linking.
- Project Brain drawer: read + evidence chips + **multi-step action-plan propose/preview/approve** (O-3); Brain never executes directly.
- Workflow-pack **detection** (cc-crew first); platform fully usable on **basic** projects (no pack).
- Local trust boundary; secrets in OS keychain; redaction before persist/embed/sync; consented setup.

**Non-goals (MVP)** `[DEFERRED]` (`PRD §15 Non-Goals`, §17, DFR §6): web app; cloud/remote runner; multi-user collaboration; iOS companion (seam designed, not built); full IDE replacement; bidirectional Linear sync; fully autonomous merges; Brain policy-automation; full cc-crew personalization/upgrade UI; full transcript→commit linking; Windows/Linux; multi-repo projects; hash-chain tamper-evidence; **agent egress isolation** (largest residual risk, §15); Project Brain internals (sibling product).

---

## §2 — Product definition & scope

Authoritative: `PRD §1-7`, `PRODUCT_CANON §2-16` (not restated). Load-bearing framings the architecture honors: five responsibilities (Dispatch · Supervise · Review · Deliver · Remember/Reason); six product layers + two cross-cutting systems (Brain, Workflow Packs); 20 screens (§11); MVP/P1/P2 tiers (§19).

---

## §3 — Locked architecture decisions

See **§0.2** (ADR spine, with status) and `docs/planning/DECISIONS.md` (full ADRs: options, rationale, fallbacks, what-would-change-this). Product decisions: `PRODUCT_CANON §17`.

---

## §4 — System overview

### §4.1 — Process topology `[LOCKED — ADR-001/002]`
```text
macOS host = trust boundary (DFR §5)
 ┌ NexusOps UI (Tauri: Rust host + WebView) ─ reattaching client; xterm.js; 20 screens
 │     ↓↑  UDS GatewayPort (JSON-RPC: intents + projection reads + subscriptions; Terminal Channel)
 ┌ NexusOps Daemon (Rust, detached, long-lived) ─ THE TRUST CORE, sole mutator & sole DB writer
 │   Action Gateway · SQLite event store (WAL) · projection engine · harness adapters ·
 │   terminal mgr (PTY) · git/worktree mgr · GitHub/Linear syncers · lease-lock mgr ·
 │   brainclient · ipc(UDS) · policy · usage · notifier
 │     ↓↑ stdio MCP (tools=propose; notifications→events)   → Project Brain sidecar (Python/FastMCP)
 │     ↓↑ PTY + SDK/app-server (can_use_tool / approvals)   → Agent subprocesses (Claude / Codex)
 │     ↓↑ octocrab / git CLI (tokens ← keychain)            → GitHub / Linear / git remotes
```

### §4.2 — The three architectural laws `[LOCKED]`
1. **Single mutation chokepoint.** All state change flows through the Action Gateway in the daemon; agents/Brain/UI/future-RemoteClient submit *intents* only; only the Gateway executes and only the daemon writes the DB. Enforced + tested as **INV-SEC-1** (§15).
2. **Events are facts; the UI reads projections.** Immutable append-only events; rebuildable projections drive the UI; source-of-truth discipline per §7.2.
3. **Reason vs execute split.** Brain plans/queries/proposes; the platform executes; Brain has zero direct privileged operations (`PBI §1`).

### §4.3 — Why three processes `[LOCKED — ADR-002]`
A PTY child dies (SIGHUP) when its owner exits, so "agents survive UI quit" (`PRD DESK-7`) requires a detached daemon. The daemon keeps the trust core (credentials, mutations, audit) a small memory-safe Rust surface separate from the restartable UI, and forces the Gateway behind an explicit IPC seam from day one. The future iOS companion reuses the **GatewayPort trait + ActionRequest model** (not the UDS transport, which is local-only; a future relay/tunnel terminates at a daemon-side adapter that re-injects intents — §13.1). Cost: daemon lifecycle (single-instance via `pidlock`, stale-socket reclaim, UI↔daemon version handshake, orphan/zombie kill) — mitigated (§16, §17).

---

## §5 — Domain model

Authoritative object catalog: `SOM` (~30 objects). Persistence: §7, `docs/planning/DATA_MODEL.md`. Four canonical chains (`SOM §35`) are invariants the event/data model keeps traceable: ticket→merge, plan→implementation, brain-action, workflow-personalization.

### §5.0 — Contract source-of-truth & propagation `[LOCKED — owner 2026-06-07; OQ-DATA-SPIKE-5]`
The cross-language representation of every shared contract (the §5.1 status enums, the §5.2 IDs + ULID format, the §7.1 actor/source enums, §6.1/§6.4 GatewayPort + wire schemas, the §7.1 `EventTypeRegistry`, the §6.3 `ActionTypeCatalog`, Appendix A models) follows **one mechanism** — **Option A**, locked by the owner (Phase 0.5 / `OQ-DATA-SPIKE-5`):

1. **Rust `shared` crate = the native contract authority.** The trust core owns its own types as idiomatic Rust (load-bearing IDs are **newtypes**, enums are `serde`-closed). No type in the trust core is generated; the daemon, as the single authority + sole mutator, authors the contract.
2. **`schemars` → JSON Schema = a first-class, published, versioned interchange artifact** (not a throwaway build output): checked into `shared/contracts/schema/`, stamped with `CONTRACT_VERSION`, regenerated from the Rust authority, and **CI-diff-gated** (a test fails when the checked-in schema ≠ what Rust emits — the same drift-gate pattern as the Codex app-server schema, `OQ-HARN-SPIKE-4`). This neutral artifact is what the **sibling Python Brain** and any external consumer bind to (no Rust import).
3. **TS (Zod) + Python (Pydantic) are generated, drift-caught consumers** of the published schema; the UI's "parse, don't trust" boundary uses the generated Zod validators.
4. **Closed-enum / reject-unknown end-to-end** — unknown contract values are rejected at every language boundary (`serde` closed enums → JSON-Schema `enum` → `z.enum` → Pydantic), preserving the fail-closed posture (§15/§17).

Rationale (single-authority thesis · preserves the newtype typing posture · schemars is already the IPC-schema mechanism · the reject-unknown safety property maps cleanly; an external IDL/codegen-into-the-trust-core was rejected as inverting authority + generating bare types in the safety-critical module). Recorded as a direct architecture-doc note on the owner's lock; a user-invoked `/arch-finalize` re-validation (natural at Phase-0 exit) would re-scrutinize it like any other anchor.

### §5.1 — Status state machines (canonical, reconciled) `[LOCKED — R-4..R-9]`
Nine machines (the "8" of the draft + ActionRequest). Enums are binding; each declares terminal states; illegal transitions → degraded marker (R-9, §17). Stored as `TEXT status` on the named projection/registry row.

| Machine | Canonical states (terminal in **bold**) | Stored on |
|---|---|---|
| **Session** (17, R-4) | creating, starting, active, thinking, running_command, editing_files, running_tests, waiting_on_permission, waiting_on_human_input, waiting_on_external_service, idle, stale, changes_ready, **failed**, **completed**, **archived**, **killed** | `proj_session.status` |
| **Task** (R-8 superset) | unassigned, queued, assigned, ready, in_progress, blocked, needs_clarification, in_review, changes_ready, pr_opened, needs_review, requested_changes, done, deferred, **merged**, **closed**, **abandoned** | `tasks.status` (kind-scoped subsets) |
| **Worktree** (derived, R-7) | git-axis {clean,dirty,untracked_files,conflicts,behind_base,ahead_of_base} + overlay {creating,locked,pr_open,merged,prunable,**deleted**} via precedence fn | `proj_worktree.status` (derived) |
| **PullRequest** | draft, open, checks_pending, checks_failing, needs_review, changes_requested, approved, mergeable, conflict, **merged**, **closed** | `proj_pull_request.status` (GitHub-authoritative cache, §7.2) |
| **WorkflowInstance** (R-7) | not_detected, pack_available, needs_personalization, personalization_in_progress, generated_review_required, active, ready_for_team_run, degraded, drift_detected, upgrade_available, **archived**, **detached** | `workflow_instances.status` |
| **ProjectBrain** | not_configured, indexing, ready, partial_index, stale, graph_degraded, transcript_ingestion_off, transcript_ingestion_active, reindex_required, **error** | daemon-cached, `brain_status_reported_at` staleness (§7.2) |
| **Approval** (R-5) | requested, previewed, awaiting_approval, **approved**, **denied**, **edited**, **auto_approved_by_policy**, **expired**, **cancelled**, **escalated** | `approvals.status` + `proj_approval_queue.status` |
| **ActionRequest** (R-5, NEW) | submitted, previewed, policy_decided, awaiting_approval, approved, denied, queued, executing, **succeeded**, **failed**, **partially_succeeded**, **rolled_back**, **rollback_failed**, **cancelled**, **expired** | `action_requests.status` |
| **ExecutionProfile** (R-7) | available, active, in_use, rate_limited, auth_expired, misconfigured, **disabled**, unknown | `execution_profiles.status` (config vs runtime split) |
| **AgentTeam** (R-6) | draft, starting, active, waiting_on_human, blocked, reconciling_outputs, **completed**, **failed**, **archived** | `agent_teams.status` / `proj_agent_team` |

**Time-derived transitions** (R-9): `stale` (Session/Brain/Profile) and git-cache staleness are read-time derivations from a liveness clock + threshold (configurable policy; Session default = heartbeat-age > 3× `refreshInterval`). On projection rebuild, `stale` is **recomputed** from `last_heartbeat_at` (not replayed) — §7.2.

### §5.2 — Shared IDs + ID format `[LOCKED — R-1; PBI §3]`
The **22 shared IDs** (PBI §3) are the cross-product contract: `workspace_id, project_id, repo_id, worktree_id, branch_name, commit_sha, session_id, agent_team_id, execution_profile_id, workflow_pack_id, workflow_instance_id, workflow_command_id, implementation_plan_id, plan_task_id, architecture_anchor, linear_issue_id, github_issue_number, pr_number, action_request_id, event_id, artifact_id, evidence_item_id`. Platform-minted IDs use **prefixed ULIDs** (`sess_…`, `evt_…`, `wt_…`, `act_…`); external IDs keep native values. **Harness→session_id:** Claude id is settable (1:1); Codex has no settable id → platform mints `sess_<ULID>`, maps the returned `thread_id` keyed on `(cwd, thread_id)` via `harness_session_map`; re-association after restart via `thread/list?cwd=` (harness is authoritative; the map is a re-derivable cache, §7.2).

### §5.3 — Desktop-addendum objects `[LOCKED status; objects PROPOSED→adopted]`
Closes the SOM gap (`DFR §7`). MVP-live: **LocalRunner** (the daemon's execution surface; sessions bind to it; minted per daemon start), **EventProjection** (projection catalog/metadata, §7). Dormant scaffolding `[DEFERRED]` for iOS: **Device**, **RemoteClient** (maps to `requester_type=remote_client`/`actor_type=remote_client`, R-2). `OQ-DATA-SPIKE-5` reconciles these into SOM. Full fields: Appendix A, `DATA_MODEL §6`.

---

## §6 — Daemon module architecture

Rust modules; clear ownership; all writes funnel through the Gateway → event store. (`docs/planning/ARCHITECTURE_DRAFT §6` table, corrected: the harness module detail is **§9.1**.)

`gateway` (§6.1/§6.2/§6.3) · `eventstore` (§7) · `projections` (§7) · `harness` (§9.1) · `terminal` (§9, ADR-009) · `git` (§9) · `integrations` (§9) · `locks` (§5.1/§7) · `brainclient` (§13.1) · `workflow` (§13.2) · `ipc` (§6.4) · `policy` (§15) · `usage` (§11/§18) · `notifier` (§10, DESK-8).

### §6.1 — `GatewayPort` contract `[LOCKED — ADR-004]`
A Rust trait implemented **in the daemon process** (the executor runs in-daemon, *not* in a separate process and *not* in the UI); all out-of-daemon callers (UI, future RemoteClient) reach it **only** over the UDS transport (§6.4). JSON-RPC method surface (binding; Appendix A):
| Method | Params → Result | Class |
|---|---|---|
| `submit_action` | `ActionRequest` → `ActionAck{action_request_id,status}` | mutation-intent |
| `submit_action_plan` | `ActionPlan` → `PlanAck{plan_id,...}` | mutation-intent (O-3) |
| `preview_action` | `action_request_id` → `ActionPreview` | query |
| `approve` | `approval_id, edits?, step_id?` → `Ack` | decision |
| `deny` | `approval_id, reason` → `Ack` | decision |
| `get_projection` | `name, scope, page` → `ProjectionPage` | query (read-only) |
| `subscribe` | `{projection|events|terminal, filter}` → stream handle | subscription |
| `get_capabilities` | → `Capabilities{protocol_version,...}` | query |
Only `submit_*` are mutation entrypoints; `get_*`/`subscribe` are read-only. Agents/Brain feed intents *into* these (MCP tools/approvals → `submit_*`); they are never the transport.

### §6.2 — Gateway core data model `[LOCKED — AG §9]`
Inlined contracts (full fields: Appendix A, sourced AG §9.1-§9.9/§12.2): `ActionRequest`, `ActionPlan` + `ActionPlanStep` + `ActionDependency` (bundled plans, MVP per O-3), `ActionPreview`, `Approval`, `ActionResult`, `ActorRef` (R-2), `ResourceRef` (21 types), `EvidenceRef`, `PolicyDecision{allow|require_approval|require_step_approval|deny|downgrade|needs_more_context}`. **Risk 0-4** (AG §7) drives preview depth + approval; risk ranges (e.g. `git.delete_worktree` 3-4) are resolved per-action in §6.3. Critical (4) actions are never in approve-all by default. **Stale-precondition re-check** (AG §16.4): after lease+fencing acquisition and immediately before `execute()`, re-resolve resources + re-read the live source (§7.2); on mismatch → `ActionFailed(stale_precondition)`, regenerate preview, **require fresh approval if the previewed diff/resource changed** (never execute a different mutation than was approved).

### §6.3 — MVP action-type catalog `[LOCKED — AG §28.2]`
The MVP action types, each with a **binding per-type contract** (params schema, locked risk, required preview class, idempotency-key formula, executor) — full table is Appendix A row `ActionTypeCatalog`. The MVP set (owner kept comprehensive scope, so this is the full ~21, not a trimmed Tier-0): `brain.ask, brain.sync, brain.summarize_session, project.rescan, workflow.detect, workflow.command.invoke, plan.link_task, session.create, session.attach_terminal, session.send_message, session.pause, session.resume, git.status, git.diff, git.create_worktree, git.create_branch, github.create_pr_draft, github.create_pr, linear.link_issue, linear.create_issue, code.open_file, review.request_agent_fix`. `workflow.command.invoke` with a null `input_schema` is risk-floored to require approval (cannot be standing-granted) — preserving the typed-actions invariant at the approval gate (OQ-WP-5).

### §6.4 — IPC wire contract `[LOCKED — ADR-004]`
One framing: **4-byte big-endian length prefix + JSON body** (newline-framing dropped). Handshake: `HelloFrame{protocol_version,client_kind,app_version}` → `HelloAck{protocol_version,daemon_version,capabilities}` | `VersionSkewError`. Fixed `MAX_FRAME_SIZE`. JSON-RPC error codes: `version_skew, frame_too_large, unknown_method, unauthorized_peer, policy_denied, precondition_stale`. The **Terminal Channel** (ADR-009) multiplexes over the same UDS socket by a frame-type tag: output `{terminal_id,seq,bytes}`, input `{terminal_id,bytes}`, and explicit backpressure control frames (pause/resume + high/low watermark) realizing app-level flow control. Peer auth: §15 (`getpeereid`).

---

## §7 — Data & state model

Authoritative: `docs/planning/DATA_MODEL.md` (full DDL). One daemon-owned SQLite DB (WAL) at `~/Library/Application Support/NexusOps/nexusops.db`; single writer = the daemon; large artifacts content-addressed on disk; harness transcripts referenced in place (Codex rollout dir pre-created 0700, files 0600 — §15). Tables: `events`, `object_refs`, the MVP projections (`proj_*`), `projection_offsets`, `outbox`, `leases`, `artifacts`, `tasks` (R-8), registry tables (projects/repositories/execution_profiles/workflow_instances/integration_connections/command_registry/agent_teams), `action_requests`/`approvals`, `harness_session_map`, FTS5.

**MVP projections** (rebuildable; tracked by `projection_offsets`): ProjectActivity, Session, ApprovalQueue, Worktree, **PullRequest** (added — §7.2), PlanProgress, ProjectGraph, **AgentTeam** (R-6), AuditTrail, UsageLedger. A single event may update multiple projections **within the one event-commit transaction**; the UI subscription pushes deltas so all subscribed surfaces (sidebar, graph, queue) reflect the same event coherently (demo step 7 fan-out, §14 test).

### §7.1 — Event envelope contract `[LOCKED — EM §6; R-2/R-3]`
Required: `event_id, seq, event_type, event_version, occurred_at, recorded_at, workspace_id, actor_type, actor_id, source_type, source_id, correlation_id, sensitivity, payload_json, schema_version`. Optional: `project_id, session_id, agent_team_id, causation_id, action_request_id, approval_id, workflow_run_id, object_refs, idempotency_key, visibility, payload_hash, previous_event_hash` (last two reserved, R-10). `seq` is the canonical order (not `occurred_at`; clocks skew — both kept). **Enums** (inlined): `actor_type` (R-2, 10 values incl. `remote_client`); `source_type` (EM §8); `sensitivity` = `public|internal|confidential|secret|restricted` (EM §9; terminal output defaults `restricted`); `visibility` = `user|project|workspace|system`. `correlation_id` mandatory; `causation_id` = immediate prior event. The **MVP event-type registry** (enumerated types + per-type payload schema + version + consuming projection + sensitivity default) is Appendix A row `EventTypeRegistry` — the contract golden-log tests bind to.

### §7.2 — Source-of-truth matrix + re-read invariant `[LOCKED]`
Every state has exactly one authoritative source; the daemon never treats a projection as truth when a live source exists. **General invariant:** *for every state class whose SoT is a live/remote source, the Gateway MUST re-read the authoritative source after acquiring the lease and before executing; the cached projection is never sufficient for a mutation decision* (realizes AG §16.4).

| State class | Source of truth | Re-read / staleness rule |
|---|---|---|
| Event-derived (`proj_*`) | the `events` log | rebuildable; `projection_offsets` |
| Durable registry | the row | not rebuildable; mutations still emit audit events |
| Gateway state | `action_requests`/`approvals` rows | canonical for execution; `proj_approval_queue` is a may-lag read-only projection; row + projection updated in one txn |
| Locks | `leases` table | authoritative; expired → reclaim w/ new fencing token |
| **Git/FS** | the repo/worktree | **re-read git2 live before any mutation**; `git_checked_at` staleness on cache |
| **PullRequest** | **GitHub (octocrab)** | local `proj_pull_request` is a synced cache w/ own `pr_checked_at`; **re-fetch before merge/checks decision** |
| **PlanTask structure** | the parsed plan file (MVP_TASKS.md) | `last_parsed_sha`; re-parse on change → `ImplementationPlanUpdated`; status is event-derived in `proj_plan_progress`; `tasks` row is the durable spine |
| **WorkflowInstance pack contents** | `.scaffolding/manifest.json` on disk (read-only to platform) | re-hash on scan/git events → `WorkflowInstanceDriftDetected`; re-validate before any `workflow.command.invoke`/personalization |
| Harness session/usage | the harness transcript/thread + SDK/app-server stream | **never scrape PTY for machine state**; resume via `claude --resume`/`codex thread/resume`; `harness_session_map` is a re-derivable cache (harness authoritative) |
| ExecutionProfile **config** | the row | canonical |
| ExecutionProfile **runtime** (active/in_use/rate_limited/auth_expired) | live: session state + adapter telemetry + keychain self-test | **re-derived on daemon restart** (not trusted from the persisted row) |
| Secrets | OS keychain | only `keychain_ref` pointers in DB; never in events/rows |
| Brain memory/index | Brain's own store | daemon caches `last reported` status + `brain_status_reported_at`; failed ping downgrades to degraded |

Field-level note on `proj_session`: `status`=event-derived (replayable); `stale`=daemon-time-derived (recomputed on rebuild, not replayed); `context/token/cost`=harness telemetry with `metric_quality ∈ {exact,estimated,unavailable}` (NULL context-% allowed for Codex — UI renders "unknown", not 0%).

---

## §8 — User & system flows

Authoritative: `PRD §9` (14 flows) + `UX §10` (A-M). Each MVP requirement maps to a flow (the requirements→flow matrix is the §14 coverage gate). Component-path table (the audit-added flows are included):

| Flow | Component path |
|---|---|
| **First-time setup** (PRD §9.1, was unmapped) | UI wizard → `gateway` intents → detection executors (`harness`/`git`/shell probes) + `integrations`(gh/Linear auth bootstrap) + `brainclient`(status) + `policy`(approval/profile defaults) → consent steps (§16 consent map); idempotent/reversible/skippable |
| Add local project (PRD §9.2) | UI → `gateway` → `git`(detect, git2) + `workflow`(detect→readiness) + `brainclient`(status) → registry rows + events → projections |
| Start from plan task (PRD §9.4) | UI → `gateway`(create_worktree=git CLI, session.create) → `locks`(lease+token) → `harness`(launch) → `terminal`(PTY) → events |
| **Start Codex session** (was unmapped; O-1) | UI → `gateway`(session.create, harness=codex) → `harness`(app-server `thread/start{cwd}` → map id → `harness_session_map`) → `locks` → events; restart re-assoc via `thread/list?cwd=` |
| **Start blank session** (PRD §9.5, was unmapped) | UI(New Session) → `gateway`(session.create) → same executors minus plan-link |
| Respond to permission (PRD §9.8) | `harness`(can_use_tool / app-server approval) → `gateway`(ActionRequest, risk) → `proj_approval_queue` → UI approve → execute → events |
| Review diff (PRD §9.9) | UI → `git`(git2 diff) → review workspace; ask-agent = `gateway` intent |
| **Commit & push** (was unmapped) | UI → `gateway`(commit/push, risk per GIT-5; force-push/protected ≥3) → `git`(CLI) → events |
| **Manual linking** (TASK-6, was unmapped) | UI(inspector link) → `gateway`(link intent risk 1) → link event + `object_refs` → projections |
| **GitHub/Linear intake** (PRD §9.3, was unmapped) | `integrations`(octocrab/Linear read) → `tasks`(external_task rows) cached as projection → dispatch via session.create |
| Ask Brain + action plan (PRD §9.11/§9.12; O-3) | UI → `brainclient`(MCP tool) → Brain proposes **multi-step ActionPlan** → `gateway`(preview/step-approve) → execute → events → Brain re-indexes |
| Create PR (PRD §9.10) | `gateway`(github.create_pr) → `integrations`(octocrab) → outbox → events; PR linked to session/worktree/task |
| **Recovery: UI restart** (DESK-7) | UI → `ipc`(handshake) → projection re-read → terminal re-stream (daemon alive) |
| **Recovery: daemon restart** (O-2) | replay projections → re-read git2 → ping Brain → reclaim leases(new token) → `harness` resume (`--resume`/`thread/resume`) else scrollback replay+relaunch w/ banner; reconcile orphaned `executing` actions (§17) |
| **Agent / PTY / app-server failure (daemon alive)** (§17) | supervisor detects child exit → `SessionFailed`+`TerminalProcessExited` → fail in-flight action + release lease → resume affordance in Human Input Queue |

---

## §9 — Integration architecture

`[LOCKED — ADR-007]`. **Git:** `git2-rs` for hot structured reads (status/diff/log/branch/worktree-list); **git CLI for ALL mutations + worktree lifecycle** (terminal parity + single chokepoint). `OQ-INT-SPIKE-6` (2026-06-07) **resolved the relative-worktree read question empirically: libgit2 ≥ 1.9.4 (git2 0.21) CAN fully read `extensions.relativeWorktrees` repos** (open/statuses/branches/head/worktree-list/find_worktree/diff) — the earlier ADR-007 "fix unreleased" premise is superseded; relative-worktree repos **no longer force CLI reads**. The CLI-read fallback is **retained** only for the separate, still-unverified **sparse-checkout misreport** gap. Mutations remain CLI-only regardless (the chokepoint invariant, not a libgit2 limitation). **GitHub:** `octocrab` (typed REST+GraphQL) for issues/PRs/checks/merges; auth bootstrap = reuse `gh auth token` else OAuth Device Flow. **Linear:** `@linear/sdk`/GraphQL; auth-code+PKCE (loopback) or pasted key; 24h refresh; budget query complexity. **Integration-failure contract** (§17): transient (429 Retry-After / 5xx → outbox backoff) vs terminal (401/403 → `*SyncFailed` + profile→auth_expired + "re-authenticate" card; keychain-unavailable → hard fail). Staged sync (inherited): link (P0) → one-way (P1) → bidirectional (P2). **Credentials:** `keyring` crate (covers iOS future), explicit per-OS feature flags + startup self-test; macOS keychain ACLs need a stable Developer ID (§16).

### §9.1 — Harness adapter layer `[LOCKED — ADR-006; Claude mode PENDING-SPIKE]`
One **`HarnessAdapter` trait** — `{launch, stream_status, intercept_mutation, read_transcript, telemetry_heartbeat, resume, capabilities}` — with **normalized return types** (Appendix A): `NormalizedStatus` (the 17 Session states), `TelemetrySample{tokens_in,tokens_out,context_pct:Option<f32>,cost,quality}`, `MutationIntercept{tool,params,decision_sink}` (both Claude `can_use_tool` and Codex `requestApproval` implement it), `TranscriptRef{path,hash,is_in_place}`, `ResumeResult`, and a **`HarnessCapabilities`** struct (PRD HARN-5's 10 fields: supportsTerminal/Resume/TranscriptRead/ToolCallParsing/UsageMetadata/ContextMetadata/CommandInjection/Subagents/Hooks/CloudTasks) driving per-capability UI degradation (e.g. `supportsContextMetadata=false` for Codex → render "unknown").

**Per-harness mutation-coverage matrix** (binding; the conformance suite asserts per category per harness, §14):
| Tool category | Claude (`can_use_tool`) | Codex (`app-server` approval) |
|---|---|---|
| direct bash / file-edit | intercepted **in `default` mode only** | intercepted (`commandExecution`/`applyPatch`) |
| subagent (Task) | **NOT guaranteed** (inherits parent mode; bypassed under acceptEdits/bypass) | n/a |
| background subagent | **bug #27203** — bypasses callback | n/a |
| MCP tools (`mcp__*`) | falls through to mode+callback (direct only) | covered (`mcp_tool_call` elicitation) |

**O-13 (locked regardless of O-4):** NexusOps-driven Claude sessions run **`default` permission mode only** (never acceptEdits/bypass/auto, so subagents can't inherit a bypass); add **`PreToolUse` hooks + deny-rules as a redundant interception layer**; **forbid background subagents** until #27203 is fixed or a pinned-version proves coverage. Mutation interception is **defense-in-depth**, not solely `can_use_tool`.

- **Claude adapter:** drive-mode (SDK streaming `can_use_tool` vs interactive-PTY + `PreToolUse` hooks) is **`[PENDING-SPIKE]`** (`OQ-HARN-SPIKE-7`, §24) — the spike must also weigh the **Agent-SDK credit-pool** (2026-06-15: SDK/`-p` draws a capped pool that hard-stops; interactive terminal is exempt) which may make interactive-PTY the sustainable default. Status from SDK stream + `Notification` hooks; transcript JSONL replay; telemetry merged from `ResultMessage.usage` + statusLine (`refreshInterval`) + transcript.
- **Codex adapter** (O-1, live in MVP): `codex app-server --stdio` JSON-RPC. **MVP depends only on stable methods** (`thread/start`, `thread/resume`, `thread/list`, `turn/start`, `thread/status/changed`, `item/commandExecution/requestApproval`) — *not* `experimentalApi`-gated ones. Handle modern + legacy approval shapes (or pin a min Codex version). Treat `-32001` overload as transient/retryable. Pin Codex version + regenerate the app-server schema bundle in CI on every bump (`OQ-HARN-SPIKE-4`). Rollout dir hardened (§15).
- **Pinned version tuple** (validated by spikes, CI golden-fixture + schema-diff gate): `{Claude Code CLI, @anthropic-ai/claude-agent-sdk, Codex CLI}` recorded in §16 version-compat matrix; documented min-supported floor.

---

## §10 — Automation / background jobs

Daemon Tokio tasks: projection workers (fold events, advance offsets in-txn); outbox drainers (Brain MCP, GitHub/Linear syncers, **notifier**, optional JSONL mirror; classify retryable vs terminal); heartbeat/status pollers (Claude statusLine `refreshInterval` + Codex push; derive `stale` by age); lease reaper (expire + new fencing token); git watcher (refresh worktree/PR caches via git2 + git hooks); WAL checkpointer (`wal_checkpoint(TRUNCATE)`); sidecar supervisor (Brain MCP ping + restart/backoff + process-group kill; in-flight `brain.*` calls carry a timeout and fail on EOF). **`notifier`** (DESK-8, MVP-nice): consumes `SessionWaitingOnHumanInput/Permission`, `CheckFailed`, `SessionCompleted` → macOS UserNotification; notification permission is a setup-wizard consent step (§16); lock-screen previews are redacted (§15).

---

## §11 — Frontend architecture

`[LOCKED — ADR-001]`. Tauri 2.x (Rust host + WebView). **Projection-driven reattaching client**: reads projections + submits intents over UDS `GatewayPort`; holds no authoritative state; never writes the DB. Terminals: xterm.js (WebGL) over the Terminal Channel with app-level backpressure (§6.4, ADR-009); approval prompts are **structured cards outside terminal text**. This section folds in the prototype review (O-5): the binding UI contracts surfaced by the 6-lens review of `NexusOps-ui-kit` (detail: `docs/ui-review/UI_RECONCILIATION.md` + the six `docs/ui-review/*.json` lenses).

### §11.1 — Canonical design system `[LOCKED — O-5]`
The **`NexusOps-ui-kit`** (tokens + component library + the "Graphite Arc" direction) is the canonical design system `tasks-gen` references and the build implements against. Two-tier oklch tokens (primitive→semantic) keep color a **re-hueable first pass** (a re-hue touches ~13 hue-family primitives only); product code references the semantic layer. Component inventory (carry forward): Button/IconButton; StatusPill/RiskBadge/UsageMeter/AttentionMarker; Badge/HarnessBadge/ProfileBadge/MetaChip; SessionRow/GraphNode/DiffHunk/EvidenceChip. **Never color alone** is a binding invariant — every status renders on ≥3 non-color channels (glyph + text label + intensity/motion); critical uses a grayscale-safe hazard hatch.

### §11.2 — Screen → surface map (20 screens) `[LOCKED]`
Present in the prototype and carried forward (16/20), with sound folds: Worktree/Git/PR Control Center → Code/Diff tabs; Usage Dashboard → Settings tab. **Build/expand:** **(1) First-Launch Setup Wizard** (absent — §11.4, §16); **(10) PR Review Workspace = FULL surface** (O-6) — a PR-review *mode* in Code/Diff with PR header, checks, reviews/comments, mergeability, risk summary, Brain evidence, agent-session summary, and Approve/Merge/Squash/Rebase/Request-changes/Ask-agent-to-fix-checks; the **Merge control triggers a fresh GitHub re-fetch** (§7.2) and routes merge/force-push/protected-branch through the Gateway at risk≥3 (§6.3, §15); **(4) Sessions List/Board** as a dense table (not only Command-Center groups); **Events/Audit detail inspector** (correlation/causation/sensitivity/payload). **Deferred:** Workflow Personalization Review UI `[P1]` (§19.2) — but the MVP Gateway still renders the multi-step personalization plan (§11.5); Remote Access / iOS pairing `[P2]`.

### §11.3 — Status rendering binding `[LOCKED]`
StatusPill (and every status surface) binds its **status keys to the §5.1 canonical enum strings verbatim** (`proj_*.status`), snake_case; display labels are a separate copy layer. It must render **every state of all 9 machines** — notably the full **17 Session states incl. `changes_ready`** (kit currently uses ~7). **One canonical `status → attention-rank` table** (covering all 9 machines) is the single source for sidebar weight, queue membership, and sort order — **no silent fall-through to idle** (today `waiting_on_permission`/conflict/stale/blocked floor to 0 and never enter "Needs my attention"); ordering per PRD §5.2. Approval and ActionRequest render as **two distinct status surfaces** (R-5). Worktree status is the **derived two-axis precedence** value (§7.2).

### §11.4 — Net-new UI surfaces `[LOCKED]` (the prototype predates these)
- **Daemon-connection indicator** (connected/reconnecting/disconnected), distinct from LocalRunner health, + a **global READ-ONLY degraded mode** that disables every intent-submitting control (Gateway approve/deny, Dispatch, Brain Run-via-Gateway, commit/push) with a "daemon unavailable — reconnecting" banner + Retry/Repair (§4/§12/§16).
- **Survival/recovery UX** (O-2): post-restart recovery banner; per-session **resumed-(live) vs replayed-(relaunched)** indicator; "Restart session" affordance for recovery-failed sessions (§8/§17).
- **Codex context-% = "unknown"** (never a number/0%) per `supportsContextMetadata=false`; carry `metric_quality` on all telemetry (§9.1/§7.2).
- **First-Launch Setup Wizard + macOS consent/TCC map** — stepper of idempotent/reversible/skippable Gateway intents; consent card + denied-degraded + repair for keychain ACL, notification permission, Full Disk Access, launchd Background Item (SMAppService), AppleEvents (§16).
- **Fencing/hard-conflict card** in the Human Input Queue (never auto-resolved); **fail-closed/audit-integrity alert** + "unknown outcome"/"partially succeeded"/"rollback failed" treatments in Audit + HIQ (§15/§17).
- **Agent-SDK credit-pool meter** in Usage + on Claude profiles (near-exhaustion + hard-stop), distinct from token spend (§9.1).
- **Native desktop notification settings** (per-type toggles for the UX §11.5 types; permission state; "previews redacted") wired to the notifier (§10/§16); **version-skew + update states** (§6.4/§16); **degraded/offline/stale** variants as first-class (§17/§13.1).

### §11.5 — Action Gateway & Brain UI `[LOCKED — O-3]`
The **Gateway Review Modal accepts an `ActionPlan` (1..N steps)** (single ActionRequest = N=1) and renders per-step rows (step #, action type, target, **risk 0-4**, preconditions, **preview status** incl. pending/unavailable+reason/stale, rollback, step status) + affected ResourceRefs/EvidenceRefs/permissions/audit-note, with the **full Screen-16 controls** (approve-all-eligible [critical/4 excluded], approve-step `step_id`, edit-before-approve [re-preview+re-risk; stale-precondition→fresh approval], remove-step, require-manual-execution, deny-with-reason, save-as-policy→`policy_grant`). **Brain "Run via Gateway" submits the exact reviewed `plan.steps`**; Brain stays propose-only. Brain drawer: add **`Actions` mode**; make modes functional; full scope chips that constrain retrieval; header shows live ProjectBrain status + grounded-at/staleness + privacy/transport; per-answer confidence/verification. Human Input Queue: all **7 groups** (add Needs-clarification, Project-Brain-action-plans, Agent-team-escalations) + full card actions + **expiration**; stamp `actor_type=project_brain` on Brain-originated items.

### §11.6 — Accessibility invariants `[LOCKED — PRD §14.8 MUST; tested §14]`
- **Project Graph list/table fallback** — functionally equivalent (same nodes+edges, status, ownership, attention), Graph|List toggle, keyboard-reachable (OBS-6; absent today).
- **Global `:focus-visible` ring** on every interactive control (tokens exist, unapplied today).
- **Every drag semantic has a non-drag equivalent** (TASK-5): task-chip overflow {Dispatch new / Send to session / Delegate to team}, session-row "Add task as context", Dispatch-dialog target selector.
- Graph nodes keyboard-operable OR the list fallback is the designated keyboard surface; node names include type+status+attention. Capacity meters show threshold on a non-color channel. AttentionMarker rail co-locates glyph+label.

### §11.7 — Component contract fixes `[LOCKED]`
UsageMeter renders the exact/estimated/unavailable accuracy label in **all variants** (ring drops it today) and shows "unknown" (not 0%/empty) when unavailable/NULL. SessionRow adds `model` + team/role (when `agent_team_id` set) and maps every waiting_on_* state into its attention rank. GraphNode adds team_lead/orchestrator/Task/Workflow-command node types (full OBS-2/UX §9 set). EvidenceChip models the 5-state freshness lifecycle (live/stale/moved/unverified/unavailable, each text/aria-labeled) + `confidence`. **External-IDE open** (EDIT-10) as a low-risk action in the Code/Diff flow.

---

## §12 — Backend / daemon strategy

`[LOCKED]`. The Rust daemon is the backend; no server. Detached (launchd/`setsid`), single-instance (`pidlock`), survives UI restarts. Concurrency: one serialized write-actor owns SQLite writes (= the chokepoint); projection reads use read-only WAL connections; long-lived work is Tokio tasks (§10). The daemon owns the Brain sidecar + all agent subprocesses (process-group kill on shutdown). **Extension points:** the `HarnessAdapter` trait (§9.1) and the executor-adapter interface (AG §15.3) — new harnesses/action-types bind here. **Determinism-for-testability** (§14): all ID minting, time, and randomness flow through injectable `IdGen`/`Clock` providers (seedable in tests).

---

## §13 — Brain seam & Workflow Packs

### §13.1 — Project Brain seam `[LOCKED-PENDING-SPIKE — ADR-005; PBI]`
Brain = stdio MCP sidecar; daemon-owned lifecycle. **Brain tools = proposals/queries** fed into the Gateway as intents (R-2 actor `project_brain`); **never direct writes** (INV-SEC-1). Opens no port. **MCP-notification→event mapping** (binding; Appendix A row `BrainEventMapping`): each Brain signal (`notifications/resources/updated`, `…/list_changed`, Tasks status, ProjectBrain status report) → a platform `event_type` (`BrainIndexStarted/Completed/Failed`, `BrainSourceIngested`, ProjectBrain status) with the read-back call the adapter makes and `actor_type=project_brain` stamping. **Brain outbox payload** = the event envelope minus restricted/secret fields, `object_refs` preserved by shared ID, redacted per §15 (`OQ-BR-9`). In-flight Brain crash → timeout → fail the `brain.*` action (retryable for idempotent queries); partial proposals discarded. **Degrades gracefully when absent/stale** (platform never hard-depends on Brain) — BUT the **§25 demo requires a reachable Brain** (precondition, §19.1). **Spike `OQ-PLAT-SPIKE-1`** (macOS sidecar notarization #11992); **fallback = FastMCP streamable-HTTP on 127.0.0.1 + per-launch loopback token** (same server code, keeps the demo working — chosen over the under-specified user-installed-CLI path). iOS reuses the **trait + intent model**, not the UDS transport (§4.3).

### §13.2 — Workflow Packs `[LOCKED]`
Optional; basic projects fully work; detection **advisory** → explicit readiness checks; cc-crew is the first pack (parsers for `MVP_TASKS.md` + `ARCHITECTURE.md §N` anchors). Personalization runs as Gateway action plans (`[P1]` UI, but the action model is MVP-ready). Workflow-owned manifests (`.scaffolding/manifest.json`) read-only to the platform (re-validated before invoke, §7.2). **Trust gate (§15):** any pack command of type `shell_command`/`hook`, or any pack with `trustLevel != bundled_trusted`, routes every script/binary exec through the Gateway at risk≥3 with explicit approval; the policy engine reads the pack `securityProfile` and DENIES forbidden-by-default ops (WORKFLOW_PACKS §16.3). MVP: cc-crew **detection** only (personalization/team/TDD-tracker deferred, §19).

---

## §14 — Testing strategy

Tiered taxonomy with explicit CI gates; **every architecture invariant has ≥1 falsifiable test**; the RISKS.md `TR-*`/`PR-*` test signals map into these tiers (traceability table maintained alongside `tasks-gen`).
- **Unit** (merge gate): state-machine legal-transition tables; idempotency-key canonicalization (same semantics → same key); fencing-token monotonicity.
- **Contract-with-fixtures** (merge gate): **HarnessAdapter conformance suite** run against **recorded fixtures** — Claude (golden SDK-message-stream + statusLine JSON + transcript JSONL per pinned CLI) and Codex (recorded app-server JSON-RPC replayed over a fake stdio child); asserts the **per-harness coverage matrix** (§9.1) per category, not a single shared pass. Gateway per-action tests (risk, preview/dry-run, idempotency dedup) bind to the §6.3 catalog.
- **Daemon-integration-with-fakes** (merge gate): event-store golden-log → rebuild projections → assert; **rebuild-equivalence** (full replay == incremental fold); crash-recovery via a **fault-injection hook** (SIGKILL/abort the write-actor at named checkpoints: mid-apply, post-event-pre-projection, mid-execute) → replay → consistent; **fail-closed** (inject audit-write failure → assert mutation did NOT apply); **fencing-conflict** (fake-clock lease expiry → stale-token write rejected); multi-projection fan-out coherence (one `SessionStarted` updates `proj_session`+`proj_project_graph` in one txn); **FakeHarness/FakePty** for survival + orphan-reaping + resume-vs-replay without real agents.
- **Frontend** (merge gate): projection-view component tests over fixtures (attention-ordering, degraded states, approval cards, graph list-fallback **a11y equivalence**, drag→non-drag paths); UI-restart reattach (mock GatewayPort); terminal backpressure UX.
- **Security** (merge gate): redaction **property/fuzz** test (secret-shaped inputs through every event/preview/audit path → zero unredacted leaks + `SensitiveOutputRedacted` emitted); **architecture-invariant test** proving the Brain-seam crate cannot reach the eventstore-writer or executor crates (INV-SEC-1 / TR-10); injection via stdin-not-argv + 10MB cap; Codex rollout `stat==0600`.
- **Performance** (gate vs §18 budgets): assert each PRD §19.6 metric against its committed budget; the **SQLite single-writer load test** (`OQ-DATA-SPIKE-3`) with committed thresholds (target N=20 concurrent mutating agents; intent-commit p95 budget; reads sub-100ms) — run before single-writer freeze, ceiling documented.
- **Live-agent smoke** (nightly, NOT a merge gate): real pinned CLIs.
- **Demo e2e** (release gate): the PRD §25 path driven via a Tauri driver against a daemon seeded with FakeHarness (or live, with demo preconditions §19.1).
- **Determinism prerequisites** (§12): injectable `Clock`/`IdGen` make all of the above non-flaky.

---

## §15 — Security & trust boundaries

Consolidated model: `docs/planning/THREAT_MODEL.md`. Binding invariants:
- **INV-SEC-1 (no-bypass):** *there exists no code path by which an agent, the Brain, or the future phone mutates FS/git/external/session state except via a typed Action that passed policy + approval and produced an audit event.* Enforcement points: Claude `can_use_tool`+`PreToolUse` (§9.1/O-13), Codex host-routed approvals, Brain proposal-only seam, UI intents-only, fail-closed-on-audit-write. Tested by the §14 architecture-invariant test (no executor reachable except via the Gateway pipeline; an event row exists for every mutation).
- **Redaction-before-persist (wired, not prose):** the eventstore single-writer routes every payload through a shared `Redactor` (owned by `policy`) before INSERT; an event may **never** persist with `redaction_status='unredacted'`. The SAME redactor gates all three sinks — **persist, embed (Brain), sync** — so unredacted content never crosses the Brain `[c]` or integration boundaries. MVP engine: curated high-recall token-prefix set (`ghp_`,`github_pat_`,`sk-`,`xox`,`AKIA`,PEM,JWT) + Shannon-entropy fallback on `KEY=value` lines (`OQ-SEC-2`). On a high-confidence secret that can't be safely redacted → **quarantine the event** + emit `SensitiveOutputRedacted` (EM §23).
- **Secrets never in events/payloads:** enforced by the redactor + a §14 test; only `keychain_ref` pointers in DB.
- **UDS peer-auth (macOS):** **`getpeereid()`** (NOT `SO_PEERCRED`, which is Linux-only); reject `uid != daemon-uid`; socket perms (0700 dir / 0600 node) are defense-in-depth, not the primary control. Capability-token in handshake `[DEFERRED]` (single-user MVP).
- **Dangerous-command / credential-read policy** (PRD §13.4, named + testable): shell tool-calls matching credential/secret-read patterns (`~/.ssh`, `security find-generic-password`, token-shaped env dumps, keychain-helper output) or non-allowlisted network egress are risk 3-4 requiring approval, emitting `CredentialAccessAttempted`/`DangerousCommandDetected`.
- **Execution-profile binding** (no silent account-hopping, ToS): profile resolved at approval time, recorded in `SessionStarted`; any change requires a new approval (re-preview+re-risk, AG §14.4 critical-field rule); Brain may never set/change a profile; usage-transparency-only, no auto-routing (`OQ-INT-6`).
- **Workflow-pack script gating:** per §13.2.
- **Codex rollout:** **pre-create + own `~/.codex/sessions` at 0700** before launching Codex (closes the 0644 TOCTOU window, #21660), plus first-read chmod 0600 belt-and-suspenders; §14 asserts no rollout file is ever observable at 0644.
- **Largest residual (accepted, MVP):** **no agent egress isolation** — Gateway gating bounds *mutations*, not data exfiltration over the agent's own allowed model-API/network egress; the only MVP reduction is keeping secrets off agent-reachable surfaces (redaction + keychain-only). Recorded as a security non-goal; egress firewalling `[DEFERRED]`.
- MVP threat non-goals: multi-user RBAC, remote/network attackers beyond localhost, compromised OS, cloud runner, agent sandboxing, hash-chain tamper-proofing.

---

## §16 — Deployment, bootstrap & lifecycle

`[LOCKED — ADR-001/011]`. macOS-only. Tauri bundler → signed/notarized `.app` + first-party updater.
- **Build/sign/notarize** (release gate, `OQ-PLAT-SPIKE-1`): Developer ID Application cert; hardened runtime on; entitlements incl. `com.apple.security.cs.allow-unsigned-executable-memory` for the PyInstaller sidecar (evaluate `disable-library-validation` for spawning child processes); **deep-sign order**: inner `.dylib`/`.so` → sidecar exe → daemon binary → `.app`; `notarytool` submit + `stapler staple`; `spctl`/`codesign --deep` CI gate. State whether the detached daemon is signed within the `.app` or as a standalone Developer-ID binary launched by launchd.
- **First-run bootstrap** (ordered, binding): (1) bundler installs a launchd plist (Background Item via SMAppService) OR the UI double-fork+setsid spawns the daemon; (2) daemon acquires `pidlock`, reclaims stale UDS socket, creates app-support dir, creates+migrates DB, registers desktop-host `Device`+`LocalRunner`, binds UDS; (3) UI connects + handshake; (4) setup wizard (the PRD §7.1 surface, now in §11) runs detection + profile creation as idempotent Gateway intents. Daemon-start-failure → explicit degraded UX.
- **Consent / OS-permission map** (PRD §9.1 "consented", §14.7 degraded): enumerate each macOS gate the locked architecture triggers — keychain ACL (post-signing), **notification permission** (DESK-8), **Full Disk Access** (daemon reads arbitrary repos + `~/.claude`/`~/.codex` transcripts), **launchd Background Item approval** (SMAppService), AppleEvents (external-IDE) — each with its setup-wizard step + degraded-state on denial.
- **App-update while daemon running** (`OQ-PLAT-7`, promoted to MVP-blocking): UI stages the new `.app` → sends `prepare_for_update` intent → daemon checkpoints (`wal_checkpoint(TRUNCATE)`), persists scrollback, then **gracefully drains + exits so the new bundle's daemon relaunches via launchd** with resume-or-replay (O-2); refuses if a high-risk action is mid-execute. MVP may require "quit live sessions before update" if hot-swap proves unsafe — state the chosen guarantee.
- **DB migrations** (forward-only `user_version`) **+ backup/rollback** (binding): before any migration that raises `user_version`, copy `nexusops.db` → `nexusops.db.bak-<from_version>`; on failure, restore + surface "update failed, rolled back to vN"; an `app_version ↔ min/max user_version` floor lets a downgraded binary detect "DB newer than I understand" and refuse-safely. Raw events are the irreplaceable spine — never migrate without the backup.
- **Version-compatibility matrix** (binding): `app_version` → IPC handshake range → min/max DB `user_version` → event-envelope version → pinned agent-CLI/SDK tuple (§9.1). Mismatch rules: UI↔daemon handshake refuse+relaunch; daemon↔DB migrate-up-or-refuse; daemon↔sidecar MCP `initialize` version check.
- **Uninstall/reset** (`[P1]`): `shutdown_and_uninstall` (unload launchd, process-group-kill daemon+sidecar+agents, optionally purge app-support + keychain refs) with a "preserve local data" option.

---

## §17 — Failure-mode contract

The single consolidated table (extends EM §23) — each row has an **owning module** and a **binding behavior**. (Detail: `docs/gap-audits/D03-failures.json`.)
| Failure | Owner | Behavior |
|---|---|---|
| Event-write fails (audit-required = all risk≥1) | `eventstore`/`gateway` | **Fail closed**: commit the authoritative `ActionExecution*` event in the same txn boundary as / strictly before acknowledging the mutation; write-failure aborts with a typed `GatewayError`. If a side effect already applied but its terminal event can't be written → emit `ActionPartiallySucceeded` best-effort + hard audit-integrity alert. |
| Agent / PTY / app-server dies (**daemon alive**) | `harness`/`terminal` | Supervisor detects child exit → `SessionFailed`+`TerminalProcessExited`/`TerminalPTYFailed`; in-flight `ActionRequest` → `ActionFailed`, release lease/token; offer "restart session" in Human Input Queue. Codex: distinguish pipe-drop (reconnect + `thread/list?cwd=`) from process crash (relaunch). |
| Daemon crash mid-action (side effect applied, terminal event not) | `gateway`/daemon recovery | On restart, scan `action_requests WHERE status='executing'`; re-derive real-world state via idempotency key (git2 re-read / octocrab GET / lease check) → transition succeeded/failed + emit the missing terminal event; un-reconcilable → surface "unknown outcome". |
| Projection update fails / corruption | `projections` | Mark `projection_offsets.state='degraded'`, skip bad event, never corrupt raw events; startup validates `schema_version` vs projector version → auto-rebuild on mismatch; MVP ships a debug/CLI rebuild (UI rebuild `[P1]`); silent logical divergence is NOT auto-detected in MVP (documented limitation). |
| Stale precondition (preview≠reality at execute) | `gateway` | Re-read live source after lock; on mismatch → `ActionFailed(stale_precondition)`, regenerate preview, require fresh approval if the diff/resource changed (§6.2). |
| Lease expiry of paused holder / fencing conflict | `locks`/`gateway` | Stale-token write → `ActionFailed(fencing_conflict)` + conflict event; rejected session → blocked/conflict state; **hard-conflict card in Human Input Queue (never auto-resolved)**. |
| Integration auth-expiry / rate-limit | `integrations` | Transient (429 Retry-After / 5xx) → outbox backoff; terminal (401/403) → `*SyncFailed` + profile→auth_expired + "re-authenticate" card; outbox row → dead only after bounded retry budget; keychain-unavailable → hard fail. |
| Brain sidecar crash mid-call | `brainclient` | MCP call timeout/EOF → fail the `brain.*` action (retryable for idempotent queries); supervisor respawns (backoff); partial proposals discarded; never blocks the core loop, never leaves a dangling `executing` Brain action. |
| Network loss (offline) | `integrations` | Local cockpit (sessions/terminals/git/Gateway/Brain) fully operational; integration reads serve last projection with "stale (offline)" badge; integration writes queue in outbox → drain on reconnect (idempotent); originating action shows "queued (offline)", not failed. |
| Duplicate / unknown-type / corrupt-payload / clock-skew events | `eventstore` | dedup via `idempotency_key`/`event_id`; unknown version → store raw + degraded marker, don't crash; corrupt `payload_json` → quarantine + audit-integrity event; use both `occurred_at`+`recorded_at`. |

---

## §18 — Performance / non-functional budgets

PRD §19.6 success metrics get committed budgets, an owning module, and a §14 performance-tier assertion (numbers are MVP targets; remaining `[OPEN]` exact values confirmed by the §14 load test + a measurement spike). The **event-store rows below are MEASURED + committed** by the single-writer load test `OQ-DATA-SPIKE-3` (resolved 2026-06-07; `docs/spikes/OQ-DATA-SPIKE-3.md`).
| Metric | Budget (MVP target) | Owner |
|---|---|---|
| App launch time | < 2 s to interactive shell | UI + daemon bootstrap |
| Project scan time | < 3 s typical repo | `git`/`workflow`/`brainclient` |
| Terminal attach latency | < 250 ms | `terminal`/`ipc` |
| Graph render time | < 500 ms for a typical project graph | UI/`projections` |
| Diff open latency | < 500 ms | `git`(git2)/UI |
| Event write latency (intent→committed) | p95 < 100 ms at N=20 concurrent agents (SLO) — **MEASURED 5.35 ms fresh / 8.44 ms @1M events** | `eventstore`/`gateway` |
| Reader latency under write load | p95 < 100 ms at N=20 (SLO) — **MEASURED ≤ 0.38 ms** | `eventstore`/`projections` |
| Brain drawer response latency | < 1.5 s first token (Brain-bound) | `brainclient`/Brain |
The event-write and drawer budgets bound the central performance bet of the projection-driven UDS + single-writer design (§14 load test quantifies the single-writer ceiling).

**Event-store load test — committed thresholds (`OQ-DATA-SPIKE-3`, resolved 2026-06-07).** The single writer holds at the N=20 design target with ~12–19× headroom; p95 stays < 100 ms through **N=100** (5× target), so the single writer is not the bottleneck at any realistic local-agent count. The user-facing SLO stays at the PRD number; tighter **§14 CI regression guards** sit at the measured baseline + margin so a regression surfaces long before the 100 ms ceiling:

| §14 CI regression guard | Committed threshold | Basis (measured) |
|---|---|---|
| Event-write p95 @ N=20 | < 30 ms | 5.35 ms fresh / 8.44 ms @1M (catches ~4× regression) |
| Event-write p99 @ N=20 | < 75 ms | ~47 ms @1M incl. WAL-checkpoint stalls |
| Reader p95 under write load @ N=20 | < 10 ms | ≤ 0.38 ms |
| Sustained single-writer throughput | floor ≥ 1,500 commits/s | ~4,000/s @1M, ~5,350/s fresh |
| Documented single-writer ceiling | p95 < 100 ms holds through ≥ N=100 | sweep, not saturated at 5× target |

WAL config confirmed: `synchronous=NORMAL`, `fullfsync=OFF` (ADR-003). **Phase-1 (1.1) implementation note:** the p99/max tail is the inline `wal_autocheckpoint` (not lock contention) — consider a background-checkpoint thread (`wal_autocheckpoint=0` + periodic manual `PASSIVE` checkpoint off the hot path) to flatten it; not a blocker. Caveat: macOS `fsync()` ≠ `F_FULLFSYNC`, so power-loss could lose the last WAL frames since the previous checkpoint (durable against app/OS crash; hash-chain tamper-evidence is post-MVP).

---

## §19 — MVP boundaries & deferred work

Tiers: `PRD §15-17`. The owner chose a **comprehensive MVP** (O-1/O-2/O-3): both harnesses, full survival, full Brain action plans. This is a large slice; sequence per §19.1 and treat the §24 spikes as gates.

### §19.1 — MVP technical slice (built backward from PRD §25) `[MVP]`
Build order (invariants → spine → demo path), revised for the audit (bootstrap precedes the daemon skeleton; harness steps reflect both-in-MVP):
0. **Pre-build spikes** (§24): `OQ-PLAT-SPIKE-1` (sidecar notarization), `OQ-HARN-SPIKE-7` (Claude mode + credit pool), `OQ-DATA-SPIKE-3` (write-contention numbers), `OQ-HARN-SPIKE-4` (Codex schema pin), `OQ-INT-SPIKE-6` (git2/octocrab), `OQ-DATA-SPIKE-5` (gap objects).
1. **First-run bootstrap + signing + version-compat** (§16) — must exist before the daemon can ship.
2. **Daemon skeleton + SQLite event store + projections + UDS GatewayPort + pidlock** (§4/§6/§7).
3. **Action Gateway** (risk, preview, approval **incl. ActionPlan + step-approval per O-3**, audit, idempotency, lease/fencing) — the §6.3 action catalog.
4. **Tauri shell** reading projections (Command Center, Project Home/Graph, Sessions, Human Input Queue, Gateway modal, Setup Wizard, Usage Dashboard).
5. **Project add + detection** (git2/CLI; workflow/cc-crew detect; Brain status) + **Execution Profiles** (keychain).
6. **Claude adapter** (mode per `OQ-HARN-SPIKE-7`; `can_use_tool`/`PreToolUse` per O-13) + **Codex adapter** (`app-server`, O-1) behind the one contract; **terminal** (portable-pty→xterm.js).
7. **Full session survival** (O-2): UI-restart reconnect + daemon-restart resume-or-replay + the §17 failure contract + the §14 deterministic test seams.
8. **Permission flow → Human Input Queue → approve via Gateway**; **code/diff review**; **commit/push**.
9. **Project Brain drawer** (read + evidence + **full multi-step action-plan** propose/preview/step-approve, O-3) — demo Brain beats; **demo precondition: reachable Brain + non-exhausted Claude profile**.
10. **PR creation** (octocrab) + task/session/worktree/PR linking + events; **GitHub/Linear read/link**.

### §19.2 — Deferred `[P1]/[P2]`
Agent Team View + `/team-start` orchestration; cc-crew personalization/upgrade UI + TDD slice tracker; PR checks + agent-fix; one-way/bidirectional Linear sync; Brain policy-automation; conflict resolver; usage budgets; iOS companion; Windows/Linux; multi-repo; hash-chain; egress isolation; UI projection-rebuild; uninstall/reset UI.

---

## §20 — Alternatives considered

Per-decision options/fallbacks: `docs/planning/DECISIONS.md` + `RESEARCH.md`. Roads not taken: Electron/Node stack (footprint, ABI tax, JS chokepoint, no iOS reuse) — fallback if Rust velocity/Tauri sidecar signing blocks; in-process topology (can't survive UI quit) — rejected per ADR-002, survives only as a theoretical regression path *for MVP*; Codex `exec --json` (coarser) — fallback to `app-server`; Brain embedded/loopback-HTTP — HTTP is the documented iOS/notarization-fallback; DuckDB/KV/libSQL — rejected vs SQLite.

---

## §21 — Diagrams

Tiering is authoritative from `docs/planning/DIAGRAM_PLAN.md` (corrected from the draft's flat list). **P0 — embed inline in this doc:** D0 system/trust map (§4.1), D1 Gateway pipeline + event/approval chain (§6), **D1a** the synchronous in-harness mutation-intercept round-trip (agent ↔ adapter ↔ Gateway ↔ approval ↔ allow/deny/rewrite — the load-bearing safety mechanic, §9.1), D2 event/projection/outbox flow (§7), D3 Session lifecycle + status-derivation (§5.1; **17 states**, no PTY scraping), D8 PRD §25 demo sequence (§19.1). **P1 — linked:** D4 adapter two-model seam, D5 the 4 canonical chains, D6 trust boundaries, D7 Brain seam, **fencing-token rejection** sequence, **daemon-restart resume-or-replay branch** (promoted from P2 given O-2's risk). **P2:** D9b UI-restart/orphan-kill, D10 deploy.

---

## §22 — Repo scaffold

`[OPEN]` confirmed at scaffold-generate. Daemon (Rust) crates: `gateway, eventstore, projections, harness (+claude/+codex), terminal, git, integrations, locks, brainclient, workflow, ipc, policy, usage, notifier, model (shared types/IDs/enums)`. `ui/` (Tauri: thin Rust host + WebView frontend). `shared/` (IPC JSON-RPC schema, the 22-ID contract, the event-type registry). `brain/` referenced (sibling, not built here). cc-crew personalizes this after this doc is finalized.

---

## §23 — Open questions & spikes

Build-gating spikes (must resolve/plan before/within the relevant `tasks-gen` phase; full register `docs/planning/OPEN_QUESTIONS.md`):
- `OQ-PLAT-SPIKE-1` — macOS notarization of the bundled Python sidecar (Tauri externalBin #11992); fallback = loopback-HTTP Brain (§13.1).
- `OQ-HARN-SPIKE-7` (NEW) — `can_use_tool` mutation-path coverage (direct/subagent/background/MCP × permission mode) **and** the Agent-SDK credit-pool impact → resolves O-4 (Claude SDK-vs-PTY primary).
- `OQ-HARN-SPIKE-4` — Codex `app-server` version-pin + CI schema-regen; modern-vs-legacy approval handling.
- `OQ-DATA-SPIKE-3` — SQLite single-writer write-contention load test with committed thresholds (§18).
- `OQ-DATA-SPIKE-5` — reconcile the 4 desktop-addendum objects into SOM.
- `OQ-INT-SPIKE-6` — git2 read-path survival on `extensions.relativeworktrees` repos; octocrab `pulls().merge()` + GitHub-App token spot-check.
Other `[OPEN]`: redaction engine specifics (`OQ-SEC-2`, MVP answer in §15), event-schema versioning (`OQ-DATA-9`), exact §18 budget numbers, App-update hot-swap-vs-quit guarantee (§16).

---

## Appendix A — Model / contract inventory

The canonical home for every cross-doc-invariant model — mirrored in the area `CLAUDE.md` cross-doc-invariants table. A field change on any model here requires editing this appendix **and** the model's `§` section in the same commit round.

> **Frozen in code (0.5 / `OQ-DATA-SPIKE-5`, 2026-06-07).** The four foundational contract surfaces below — the status state machines (§5.1), the 22 shared IDs + prefix map (§5.2), the actor enum (§7.1/R-2), and the desktop-addendum objects (§5.3) — are now codified in the `shared/` Rust authority crate per the **§5.0 Option-A** mechanism (Rust = authority → schemars JSON-Schema artifact → generated Zod/Pydantic). **Exception:** the **ExecutionProfile** runtime-state enum is deliberately **held for 0.5b** (pending the cat-4 SDK-vs-PTY call) — frozen everything else.

| Model | Section | Fields (summary) |
|---|---|---|
| **Event envelope** | §7.1 | event_id, seq, event_type, event_version, occurred_at, recorded_at, workspace_id, actor_type, actor_id, source_type, source_id, correlation_id, causation_id?, action_request_id?, approval_id?, session_id?, agent_team_id?, workflow_run_id?, idempotency_key?, sensitivity, visibility, payload_json, payload_hash?(rsvd), previous_event_hash?(rsvd), schema_version, app_version |
| **Actor/Requester enum + mapping** | §5.1/§7.1 | audit actor_type (10, incl. remote_client); requester_type aliases agent_session→session_adapter, workflow_pack→workflow_runtime, system_policy→automation_policy |
| **GatewayPort** | §6.1 | submit_action, submit_action_plan, preview_action, approve, deny, get_projection, subscribe, get_capabilities |
| **ActionRequest** | §6.2 | action_request_id, project_id?, action_type, requester_type, requester_id, resource_refs[], inputs, risk_level(0-4), idempotency_key, fencing_token?, status(15, §5.1), preview, created_at |
| **ActionPlan / ActionPlanStep** | §6.2 | plan_id, title, steps[], dependencies[], rollback_plan?, overall_risk, approval_mode(approve_all|step_by_step|mixed|blocked) |
| **Approval** | §6.2/§5.1 | approval_id, action_request_id?/plan_id?, required_approver, status(10, §5.1), risk_level, scope(single_action|plan|policy_grant), constraints?, decided_by, decided_at, expires_at |
| **ActionResult** | §6.2 | action_request_id, status(succeeded|failed|partially_succeeded|cancelled), created/changed_resources, emitted_events, error?, rollback_available |
| **ResourceRef / EvidenceRef / PolicyDecision** | §6.2 | ResourceRef{type(21),id,uri?}; EvidenceRef{type,id,confidence}; PolicyDecision{status(6),reasons,requiredApprovals,constraints,saferAlt?} |
| **ActionTypeCatalog** (per-type) | §6.3 | per action_type: params schema, locked risk, required preview class, idempotency-key formula, executor, resource_refs required |
| **EventTypeRegistry** (MVP subset) | §7.1 | per event_type: event_version, payload schema, sensitivity default, consuming projection(s) |
| **HarnessAdapter trait + normalized types** | §9.1 | methods{launch,stream_status,intercept_mutation,read_transcript,telemetry_heartbeat,resume,capabilities}; NormalizedStatus(17), TelemetrySample{…,context_pct:Option}, MutationIntercept, TranscriptRef, ResumeResult |
| **HarnessCapabilities** | §9.1 | 10 fields (PRD HARN-5) |
| **Per-harness mutation-coverage matrix** | §9.1 | tool-category × harness → guaranteed|best-effort|unsupported |
| **Status state machines (10 total)** | §5.1 | Session(17), Task(superset), Worktree(2-axis: git+overlay, derived), PullRequest, WorkflowInstance, ProjectBrain, Approval(10), ActionRequest(15), AgentTeam — **9 frozen in `shared/` (0.5)**; the 10th, **ExecutionProfile, is HELD for 0.5b** (cat-4). _(§5.1 prose header still says "Nine" — header/table count mismatch flagged for the Phase-0-exit /arch-finalize.)_ |
| **22 shared IDs + ID format** | §5.2 | the 22 IDs; prefixed-ULID newtypes; **canonical id_kind prefix map (frozen 0.5)** — 16 platform-minted: `ws_ proj_ repo_ wt_ sess_ team_ prof_ pack_ wfi_ cmd_ plan_ task_ act_ evt_ artf_ evid_`; 4 desktop: `dev_ rc_ lr_ eprj_`; 6 external (branch_name, commit_sha, architecture_anchor, linear_issue_id, github_issue_number, pr_number) = native, no prefix. harness_session_map{session_id,harness,harness_native_id,cwd,rollout_path} |
| **Desktop-addendum objects** | §5.3 | LocalRunner, EventProjection (MVP); Device, RemoteClient (deferred) |
| **BrainEventMapping** | §13.1 | MCP signal → platform event_type + read-back call + actor stamping |
| **Version-compatibility matrix** | §16 | app_version ↔ IPC handshake range ↔ DB user_version range ↔ event-envelope version ↔ agent-CLI/SDK tuple |
| **Lease** | §5.1/§7.2 | resource_id, owner_id, fencing_token(monotonic), acquired_at, heartbeat_at, expires_at, lease_kind |

---

## §24 — (reserved — see §23 for open questions & spikes)

*Anchor reserved to keep §23 = Open Questions stable; spike list lives in §23.*

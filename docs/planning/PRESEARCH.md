# PRESEARCH — NexusOps Architecture Planning Hub

> **Project:** NexusOps — a desktop-first, local-runtime AI engineering control plane.
> **Status:** Planning draft (Brain 1 / `arch-draft`). Rough draft for adversarial finalization by `arch-finalize` (Brain 2).
> **Planning mode:** Expanded, **de-duplicated** — full expanded rigor, but this hub *references* the already-rich `docs/` set instead of restating product/users/domain/requirements. Net-new energy goes to research, decision-locking, threat model, data/persistence model, the architecture draft, diagrams, and handoff.
> **Naming decision (user, 2026-06-06):** Platform is **NexusOps** across all artifacts. Project Brain keeps the working name **Project Brain** (candidate brand *Anchorlight*); brand-name/legal clearance remains `open question`. Supersedes `docs/product/NAMING.md`'s "Switchyard" recommendation for the platform.
> **Project Brain scope (user, 2026-06-06):** This architecture covers the **platform + the Project Brain integration seam only**. Brain internals (vector store, embeddings, CodeGraph federation) are the sibling product's own architecture (`../project-brain`).
> **Tag vocabulary:** `locked decision` · `proposed recommendation` · `open question` · `MVP simplification` · `deferred work` · `research required`.

---

## 0. Doc Coverage Map (why this hub is de-duplicated)

The existing `docs/` set already satisfies most of the Expanded artifact slots. This hub does **not** re-derive them; it points to them and records only the net-new synthesis. The downstream `ARCHITECTURE_DRAFT.md` binds to these as upstream inputs.

| Expanded artifact | Satisfied by (authoritative source) | Treatment here |
|---|---|---|
| PRODUCT_BRIEF | `docs/product/PRODUCT_CANON.md`, `docs/product/PRD.md` §1–2 | reference; §1 below |
| USERS | `docs/product/PRD.md` §3 (3 personas), `PRODUCT_CANON.md` | reference; §3 below |
| STAKEHOLDERS | implicit in PRD risks/principles; **thin** | net-new mini-table §3 |
| USER_FLOWS | `docs/product/PRD.md` §9 (14 flows), `docs/ux/UX_INFORMATION_ARCHITECTURE.md` §10 (flows A–M) | reference; §4 below |
| DOMAIN_MODEL | `docs/architecture/SHARED_OBJECT_MODEL.md` (~30 objects, lifecycles, 4 canonical chains) | reference; §5 below. `DATA_MODEL.md` will make it persistence-concrete |
| REQUIREMENTS | `docs/product/PRD.md` §10 (module reqs, MUST/SHOULD/MAY) | reference; §6 below |
| CONSTRAINTS | PRD §22, `DESKTOP_FIRST_RUNTIME.md`, cross-doc | synthesized §7 below |
| EVALUATION_CRITERIA | `docs/product/PRD.md` §19 (success metrics), §25 (demo) | synthesized §7 below |
| ASSUMPTIONS | scattered | net-new §9 below |
| OPEN_QUESTIONS | every doc has a list; **large & consistent** | consolidated → `OPEN_QUESTIONS.md` (net-new) |
| RESEARCH | none yet | net-new → `RESEARCH.md` |
| DECISIONS | `PRODUCT_CANON.md` §17 (product decisions, D1–D17); **no tech ADRs** | net-new tech ADRs → `DECISIONS.md` |
| RISKS | `docs/product/PRD.md` §20 (10 product risks) | architecture-risk register → `RISKS.md` |
| THREAT_MODEL | `ACTION_GATEWAY.md` + `EVENT_MODEL` security sections; **no consolidated threat model** | net-new → `THREAT_MODEL.md` |
| DATA_MODEL | `SHARED_OBJECT_MODEL.md` + `EVENT_MODEL_AND_AUDIT_TRAIL.md` §12 | persistence-concrete → `DATA_MODEL.md` |
| ARCHITECTURE_DRAFT | none yet | net-new → `ARCHITECTURE_DRAFT.md` |
| DIAGRAM_PLAN | none yet | net-new → `DIAGRAM_PLAN.md` |
| CLAUDE_CODE_HANDOFF | none yet | net-new → `CLAUDE_CODE_HANDOFF.md` |

**Companion specs the draft must honor (subsystem-level, already authored):**
`ACTION_GATEWAY.md` (typed actions, risk 0–4, executors, idempotency, locks, audit) · `EVENT_MODEL_AND_AUDIT_TRAIL.md` (append-only SQLite events, projections, outbox, sensitivity, redaction) · `WORKFLOW_PACKS.md` + `CC_CREW_WORKFLOW_PACK.md` (pack/instance/personalization, cc-crew) · `PROJECT_BRAIN_INTERFACE.md` (platform↔Brain boundary, 22 shared IDs) · `DESKTOP_FIRST_RUNTIME.md` (desktop = trust boundary, iOS routing) · `UX_INFORMATION_ARCHITECTURE.md` + `UI_COMPONENT_INVENTORY.md` (20 screens, 8 status enums).

> **Authoritative-source note:** `docs/product/ARTIFACT_REGISTER.md` is a **stale snapshot** (marks many now-existing docs "not created yet"). Live docs override it. Legacy `docs/archive/v0.1/*` and any "AgentOps Studio" handoff are **not** sources of truth (per `CLAUDE_DESIGN_SYSTEM_PROMPT.md`).

---

## 1. Phase 0 — PRD Intake

### Product in one sentence
A desktop-first, local-runtime cockpit that lets one developer dispatch, supervise, review, and ship the work of many AI coding agents (Claude Code, Codex, future harnesses) across multiple local projects — with every action permissioned and auditable, and an optional project-memory brain that reasons over it all. `locked decision` (`PRODUCT_CANON.md` §2, `PRD.md` §1)

### What the product IS
A local desktop application **and** local execution runtime with five responsibilities: **Dispatch · Supervise · Review · Deliver · Remember/Reason** (`PRD.md` §2.1). The control layer *above* Claude Code / Codex / git / GitHub / Linear — not a replacement for any of them.

### What the product IS NOT
Not a generic IDE, not a chatbot wrapper, not a cloud SaaS, not a hosted/remote shell, not a generic RAG chatbot, not a PM tool, not a replacement for the harnesses or git/GitHub/Linear, not an autonomous system that mutates repos/tickets/credentials without permission (`PRD.md` §2.2). No web app in MVP (`DESKTOP_FIRST_RUNTIME.md` §1).

### Primary problem
Once AI coding agents become parallel workers, the developer becomes a *manager of distributed work*, but existing tools still expose each agent as an isolated terminal/chat/branch/PR. There is no single place where every task, session, terminal, worktree, branch, PR, approval, workflow, and memory object is visible, actionable, and auditable (`PRD.md` §1 thesis).

### Primary user
Portfolio Builder / Solo Technical Lead running many local projects with Claude Code/Codex + custom workflows + worktrees. Secondary: small-team tech lead/reviewer. Future: mobile supervisor (P2). (`PRD.md` §3)

### Core workflow (canonical work path)
Task/PlanTask/Issue/Ticket → **Dispatch** → Session or Agent Team → Worktree+Branch → Terminal execution → Code changes → Tests/checks → Human review → Commit → PR → Merge → Archive → Project Brain memory. (`UX_INFORMATION_ARCHITECTURE.md` §4.2)

### Explicit PRD requirements
Module requirement set in `PRD.md` §10 (PROJ, PROF, HARN, SESS, TERM, EDIT, TASK, PLAN, GIT, PR, TEAM, WF, BRAIN, AG, EVT, OBS, HIQ, USAGE, SET) with MUST/SHOULD/MAY. MVP must-haves / nice-to-haves / non-goals in `PRD.md` §15. See §6.

### Implied requirements (not explicit, MVP must still handle)
See Phase 8 inferences (§8): local persistence/crash-recovery, a structured plan-parser as a platform capability, derived "attention" + "risk" computed attributes per session/worktree, redaction pipeline, ID strategy across platform+Brain, projection rebuild, fail-closed audit write.

### External dependencies
Claude Code (local auth), Codex CLI/Cloud (local auth), git + worktrees, GitHub (issues/PRs/checks, `gh`/OAuth), Linear (OAuth/API), OS keychain, Project Brain service (sibling), optional CodeGraph (code intel) / Context7 (docs) MCPs. `research required` on the *current* realities of each adapter (see `RESEARCH.md`).

### Ambiguities / open terms
"Local runner" decomposition; "session status detection" reliability; "minimal useful code editor"; "Action Gateway transport"; Brain deployment topology. All `open question` — consolidated in `OPEN_QUESTIONS.md`.

### Initial risk areas
Scope too large (RISK-1); brittle terminal/status detection (RISK-2); dangerous git automation (RISK-3); Brain over-powered too early (RISK-4); workflow overfit to cc-crew (RISK-5); execution-profiles look like subscription circumvention (RISK-6); editor scope creep (RISK-7); mobile security (RISK-8); transcript privacy (RISK-9); decorative graph (RISK-10). (`PRD.md` §20) → architecture-risk register in `RISKS.md`.

### Recommended planning mode
**Expanded, de-duplicated** — confirmed by user.

---

## 2. Phase 1 — Product Mechanics

### Core object of value
The **Session** — the atomic operational unit; every meaningful unit of agent work resolves to one or more sessions. It binds ~19 entities (project, repo, worktree, branch, task, plan task, GitHub/Linear issue, workflow command, agent team, execution profile, terminal, transcript, diffs, commits, PR, test results, approvals, Brain episode cards). `locked decision` (`SHARED_OBJECT_MODEL.md` §12, `PRODUCT_CANON.md` §8)

### State-changing actions
All mutations are typed `ActionRequest`s through the **Action Gateway** (the *only* mutation chokepoint), classified risk 0–4, previewed/dry-run, approved (single/step/plan), executed by adapters, audited as immutable `Event`s. `locked decision` (`ACTION_GATEWAY.md` §2, §7, §8)

### Lifecycle (the spine)
Session: 15–16 states (`creating → starting → active → thinking → running_command → editing_files → running_tests → waiting_on_permission → waiting_on_human_input → waiting_on_external_service → idle → stale → failed → changes_ready/completed → archived → killed`). Worktree (11–13 states), Task (13), PR (11), WorkflowInstance (12), Approval (12), Brain index (10), Execution Profile (8) — full enums in `SHARED_OBJECT_MODEL.md` / `UX_INFORMATION_ARCHITECTURE.md` §8. **8 distinct state machines** the data model must own.

### Units / records
Events (immutable, correlation/causation-chained), Approvals, Episode cards, Usage records (accuracy: exact/estimated/unavailable), Evidence chips (10 types), Decisions (locked/proposed/open/deferred).

### Who/what creates the main objects
User (UI/palette), Project Brain (proposes plans), Workflow runtime (commands/recipes), local runner (sessions/terminals), integration syncers (tasks). All converge to ActionRequests + Events.

### Who/what resolves them
Action Gateway executes + emits authoritative `ActionExecution*` events (sole emitter). Human resolves approvals via the Human Input Queue (attention-sorted). Sessions end explicitly with a completion summary.

### Hidden mechanics (load-bearing, easy to miss)
- **Attention-first ordering** is computed: waiting-on-human/blocked/high-risk float above active work (`PRD.md` §5.2, `UX_IA` §5.1). → derived attribute on every session/worktree.
- **Risk visibility** is a set of derived/computed states (high context, high cost, stale, dirty, failing checks, conflicts, dangerous commands, credential access, unreviewed code).
- **Stale-precondition re-check**: state can change between preview and execute; gateway must re-check after lock acquisition (`ACTION_GATEWAY.md` §16.4).
- **Fail-closed audit**: if the audit event write fails, the action does not proceed (`EVENT_MODEL` §23).
- **Detection vs readiness**: detecting workflow files ≠ ready; readiness is an explicit verifying step (`WORKFLOW_PACKS.md` §3.3).

### Confirmed mechanics / Still ambiguous
**Confirmed:** the Session spine, the Gateway-as-chokepoint, append-only events + projections, pack/instance/personalization, Brain-plans-platform-executes. **Still ambiguous (→ Decisions/Research):** how sessions are *spawned & attached* (wrap existing terminal vs platform-launched PTY) and how fine-grained statuses are *derived* from terminal/tool-call observation (flagged fragile, `SHARED_OBJECT_MODEL.md` §37 Q5–Q6).

---

## 3. Phase 2–3 — Users, Actors, Stakeholders

**Human users:** Primary = Portfolio Builder/Solo Lead; Secondary = small-team lead/reviewer; Future = mobile supervisor (P2). Full personas: `PRD.md` §3.

**Non-human actors** (must be first-class in event/actor model): `user`, `project_brain`, `action_gateway`, `workflow_runtime`, `local_runner`, `session_adapter`, `integration_syncer`, `system`, `remote_device`, `automation_policy` (`EVENT_MODEL` §7).

**Stakeholders (net-new mini-table — was thin):**

| Stakeholder | Cares about | Would reject if | Architecture must address |
|---|---|---|---|
| Solo builder (user=buyer) | Attention triage, safety, speed | Dangerous automation, opaque state | Attention-first IA, Gateway gating, ownership chain |
| Security-minded self / future reviewer | Credentials never leak, auditable | Secrets in logs, silent mutation | Trust boundary, redaction, immutable audit (`THREAT_MODEL.md`) |
| Anthropic/OpenAI (ToS) | No subscription circumvention | Auto account-hopping | Execution Profiles explicit/auditable, no silent switching |
| Future teammates (P2) | Shared context, provenance | No decision trail | Decision log, episode cards, events |

---

## 4. Phase 4 — User & System Flows
Authoritative: `PRD.md` §9 (14 flows incl. setup, add project, start-from-ticket/plan/blank, agent team, monitor, human-input, review, PR, ask-Brain, Brain-action-plan, workflow personalization, remote stretch) and `UX_INFORMATION_ARCHITECTURE.md` §10 (flows A–M). **Stop-condition check (every MVP requirement maps to a flow):** to be verified during gap audit in `arch-finalize`; flagged here as `research required` for the requirements→flow matrix.

## 5. Phase 5 — Domain Model
Authoritative: `SHARED_OBJECT_MODEL.md` (~30 objects with Definition/Key fields/Relationships/Lifecycle/UI surfaces/Events; object containment tree; **4 canonical chains** — ticket→merge, plan→implementation, brain-action, workflow-personalization). **Net-new gap:** the desktop addendum requires `Device`, `RemoteClient`, `LocalRunner`, `EventProjection` objects that the model does **not** yet define (only `Terminal` exists) — `open question` to close in `DATA_MODEL.md`/draft. Actor/risk enums need a remote-client value.

## 6. Phase 6 — Requirements
Authoritative: `PRD.md` §10 (module MUST/SHOULD/MAY) and §15 (MVP must/nice/non-goals). `DECISIONS.md`/draft will not restate these; the draft references requirement IDs and maps them to components + the MVP technical slice.

---

## 7. Phase 7 — Constraints & Evaluation (synthesized)

### Hard constraints (`locked decision` unless noted)
- **Desktop-first**; web app a non-goal for MVP. Local machine = execution & trust boundary; desktop app brokers all FS/git/PTY/credential/Brain access.
- **All mutation through the Action Gateway** (incl. Brain, workflows, remote/iOS). Brain never executes directly.
- **Events immutable & append-only**; corrections appended; UI reads projections; fail-closed on audit write.
- **Workflow Packs optional**; Basic Projects must fully work; cc-crew first-class but never required.
- **Execution Profiles explicit/visible/auditable**; no silent account-hopping (ToS-sensitive).
- **Local-first persistence**; no cloud runner in MVP; per-project Brain stores local by default.
- **Secret redaction before persist/embed/sync**; terminal output defaults to `restricted` sensitivity.
- **iOS companion** P2; observability→approvals only; never a remote shell; routes through Gateway.
- **Decoupled from Project Brain**: thin contract (shared IDs + doc-format spec), no code imports, degrade gracefully when Brain absent/stale.

### Evaluation criteria
Success metrics: `PRD.md` §19 (activation, operational value, trust, review quality, Brain, performance). Demo target: `PRD.md` §25 (17-step scenario). → confirm/lock the **first demo workflow** with user (intake question).

### Intake constraints (resolved with user, 2026-06-06)
- **Timebox/resourcing:** solo build driven largely by AI agents (likely via cc-crew itself); optimize for a credible, demoable MVP slice over a hard calendar deadline. `locked decision`
- **MVP harness scope:** **Claude Code AND Codex both** are MVP-blocking adapters (bolder than the PRD's "Codex if feasible"). Raises the criticality of the harness-adapter abstraction and of verifying Codex's session/transcript/status realities. `locked decision` → `RESEARCH.md` R-CC / R-CODEX, RISK-2.
- **Desktop stack:** no hard constraint; **research & recommend**, then lock in `DECISIONS.md`. `open question` → `RESEARCH.md` R-STACK.
- **First demo / MVP proof:** PRD §25 full 17-step scenario (exercises the whole spine). The MVP technical slice is built backward from it. `locked decision`

---

## 8. Phase 8 — MVP-Scoped Inferences (net-new)

| Inference | Why it matters | Classification | Architecture impact |
|---|---|---|---|
| Local persistence = SQLite (WAL) event store + projections + outbox + artifact refs | Everything (audit, graph, queue, usage) reads from it; crash-safe | MVP-critical | Foundational storage layer; recommended in `EVENT_MODEL` §12 but not locked → ADR |
| A structured **plan parser** is a platform capability (MVP_TASKS.md etc.) | Plan View, dispatch-from-plan, cc-crew rely on it | MVP-critical | Generic parser + pack-provided parser seam |
| **Derived attributes** (attention rank, risk states) computed from events/state | Attention-first IA is load-bearing | MVP-critical | Projection/derivation layer, not stored truth |
| **Redaction pipeline** runs before persist/embed/sync | Privacy + ToS; terminal=restricted | MVP-critical | Cross-cutting subsystem, sensitivity-classified |
| **Shared ID strategy** (22 IDs) agreed before schemas harden | Brain evidence + provenance + integration | MVP-critical | ID/reference contract is foundational |
| Session **spawn/attach + status derivation** from PTY/tool-calls | Core supervision value; flagged fragile | research required | Adapter capability model + degraded states |
| Projection **rebuild + crash recovery** path | Reliability; UI restart safe | MVP-critical | projection_offsets, idempotent rebuild |
| Codex adapter modeled even if launch is P1 | "Codex-ready data model from day one" | MVP simplification | Adapter abstraction now, full Codex launch later |
| iOS substrate (redacted projections, typed actions, device metadata) | Keep P2 possible without building it | deferred work | Reserve projection/redaction seam now |
| Cryptographic hash-chain of events | Tamper-evidence | deferred work | Reserve columns (`payload_hash`,`previous_event_hash`), not enforced MVP (`EVENT_MODEL` §24) |

---

## 9. Phase 9 — Assumptions & Open Questions

### Assumptions (with fallback)
- **A1** Desktop-first is final (resolves the Canon's "open" framing). *Fallback:* none; user-confirmed direction.
- **A2** Single-user, single-machine in MVP (multi-user/cloud = P2). *Fallback:* design seams don't preclude later.
- **A3** SQLite (WAL) is the event store unless research surfaces a blocker. *Fallback:* SQLite + append-only JSONL mirror.
- **A4** Project usually maps to one primary repo (multi-repo later). *Fallback:* keep Worktree↔Repo cardinality open in data model.
- **A5** Brain runs as a local service/sidecar the platform talks to over a defined IPC/API (topology TBD). *Fallback:* embedded library mode.

### Load-bearing open questions (full register → `OPEN_QUESTIONS.md`)
1. Desktop **stack & language/runtime**? (`research required`) 2. **Process model** — single process vs UI + local daemon + runner? 3. Brain **deployment topology + IPC** (embedded/sidecar/service/library)? 4. Event-store engine **lock-in**? 5. **PTY/terminal capture + status detection** mechanism & reliability? 6. ~~MVP harness scope~~ → **resolved: Claude Code AND Codex both in MVP** (adapter realities for both still `research required`). 7. Minimal useful **code editor**? 8. Gateway **transport** (IPC/HTTP/in-process/MCP)? 9. **Locks across app restarts**? 10. **Packaging** (Node CLI vs Python core vs both, given Brain is Python/FastMCP)? 11. Worktree/branch **co-ownership & AgentTeam PR reconciliation**? 12. ~~Timebox/resourcing/first-demo~~ → **resolved**: solo+AI, credible demoable MVP, demo = PRD §25.

---

## 10. Next phases (this hub feeds)
- **Phase 10 Research** → `RESEARCH.md` (current facts for OQ #1–10, fan-out planned).
- **Phase 11–12 Decisions** → `DECISIONS.md` (ADR-lock the load-bearing tech decisions).
- **Phase 14 Security** → `THREAT_MODEL.md`; **Phase 5/persistence** → `DATA_MODEL.md`; **risks** → `RISKS.md`.
- **Phase 15 Draft** → `ARCHITECTURE_DRAFT.md`; **Phase 17** → `DIAGRAM_PLAN.md`; **Phase 16** → `CLAUDE_CODE_HANDOFF.md`.

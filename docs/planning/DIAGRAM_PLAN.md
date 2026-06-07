# DIAGRAM PLAN — NexusOps Architecture

> **Phase 17 output.** Plans the diagrams the architecture needs, after the architecture (not before). For each: purpose, what it must show, the spec anchors it maps to, priority, and recommended format. Diagrams are produced during/after `arch-finalize`; this plan tells the finalizer/illustrator exactly what to draw and what each must prove. Favor diagrams that clarify hard mechanics, lifecycle flows, trust boundaries, and implementation seams (the things prose hides).
> **Anchors** reference `ARCHITECTURE_DRAFT.md §N` (AD) and the upstream specs (SOM/EM/AG/PBI/DFR/UX/DATA_MODEL).
> **Format key:** Mermaid is the default (text-diffable, lives in-repo); call out where a hand-drawn/Excalidraw or a sequence/state diagram fits better.

---

## Full-Scope Architecture Diagram

**D0 — System & trust-boundary map.**
- **Purpose:** the one picture that shows the whole system: the macOS trust boundary, the three processes (Tauri UI, Rust daemon, Python Brain sidecar), the agent subprocesses, and the external systems — with every connection labeled by transport.
- **Must show:** trust boundary box (host); UI ↔ daemon over **UDS `GatewayPort`** (projection reads + intent submits + terminal Channel); daemon ↔ Brain over **stdio MCP** (tools=propose, notifications→events); daemon ↔ agents over **PTY + SDK/app-server** (can_use_tool / approvals); daemon ↔ GitHub/Linear/git (octocrab/CLI, tokens from keychain); the daemon's internal modules as a cluster (Gateway, event store, projections, adapters, terminal, git, integrations, locks, brainclient, ipc); the "daemon = sole writer / sole mutator" annotation.
- **Anchors:** AD `§4.1`, `§4.2`, `§6`; `DFR §5`.
- **Priority:** P0 (the cover diagram). **Format:** Mermaid `flowchart` (or Excalidraw for the polished version).

---

## Sub-Diagrams (prioritized)

### D1 — Action Gateway pipeline + event/approval chain  ·  P0
- **Purpose:** show the single mutation chokepoint end to end — how an intent becomes an audited, approved, executed action with events.
- **Must show:** the ordered pipeline (normalize → resolve resources → policy/risk 0–4 → preview/dry-run → approval manager → action queue → executor adapter → event bus/audit → Brain ingestion); the canonical event chain (`ActionPlanProposed → ActionRequestCreated → ActionPreviewGenerated → PolicyDecisionRecorded → ApprovalRequested → ApprovalGranted/Denied → ActionExecutionStarted → Succeeded/Failed → RolledBack`); the rule that the Gateway is the **only** emitter of `ActionExecution*` events; stale-precondition re-check after lock acquire; fencing-token check.
- **Anchors:** AD `§6`; `AG §8/§15.1/§16/§17`; `EM §16`; `DATA_MODEL §2.6/§2.9`.
- **Format:** Mermaid `flowchart` for the pipeline + a `sequenceDiagram` for the event chain.

### D2 — Event store / projection / outbox data flow  ·  P0
- **Purpose:** make the event-sourced persistence concrete and show crash-recoverability.
- **Must show:** intent → daemon write-actor → single SQLite txn writing `events` + `projections` + `outbox` + advancing `projection_offsets`; UI reading projections (read-only WAL); outbox drainers (Brain/GitHub/Linear/notifier/JSONL-mirror); projection rebuild path (replay `events WHERE seq > last_seq`); large artifacts as path+hash refs (not BLOBs); the "single writer = the daemon" invariant.
- **Anchors:** AD `§7`, `§10`; `DATA_MODEL §1/§2/§7.2`; `EM §12/§13`.
- **Format:** Mermaid `flowchart`.

### D3 — Session lifecycle + status derivation  ·  P0
- **Purpose:** the highest-value mechanics diagram — how a session moves through its 16 states and, critically, **where each status comes from** (never from PTY scraping).
- **Must show:** the Session state machine (`creating→starting→active` with runtime/waiting sub-states → `idle→stale` → terminal states); annotations on each transition's *source* — SDK message stream / `Notification` hooks (Claude), app-server push status (Codex), heartbeat-age → `stale` (time-derived); the PTY shown as display-only, explicitly NOT a status source.
- **Anchors:** AD `§5`, `§9.1`; `DATA_MODEL §4.1`; `SOM §12`, `UX §8.1`; `ADR-006`.
- **Format:** Mermaid `stateDiagram-v2`.

### D4 — Harness-adapter two-model seam  ·  P1
- **Purpose:** show how one `HarnessAdapter` contract spans two very different lifecycle models, keeping the Gateway/event store agent-agnostic.
- **Must show:** the trait surface (`launch/stream-status/intercept-mutation/read-transcript/telemetry-heartbeat/resume`); Claude lane (SDK streaming + can_use_tool + PTY mirror + JSONL + statusLine) vs Codex lane (app-server JSON-RPC: thread/start{cwd}→id, push status, host-routed approvals, rollout JSONL@0600); both feeding the same Gateway/event store; the Codex gaps (no settable id → cwd+thread_id mapping; no context %).
- **Anchors:** AD `§9.1`; `ADR-006`; `RESEARCH R-CC/R-CODEX`; `DATA_MODEL §5`.
- **Format:** Mermaid `flowchart` (two lanes converging).

### D5 — The 4 canonical object chains  ·  P1
- **Purpose:** show the traceability invariants the data/event model must keep intact.
- **Must show:** (1) ticket→merge (Task→Session/Team→Worktree→Branch→Diff→Commit→PR→Review→Merge→Brain episode); (2) plan→implementation; (3) brain-action (Brain query→Evidence→Action plan→ActionRequest→Approval→Execution→Event→Re-index); (4) workflow-personalization (Pack→PersonalizationRun→Generated diff→Approval→Instance→Command Registry→Session/Team launch); all keyed by the 22 shared IDs.
- **Anchors:** AD `§5`; `SOM §35`; `PBI §3`.
- **Format:** Mermaid `flowchart` (4 stacked chains) — high value for Brain evidence + audit.

### D6 — Trust boundaries & threat surface  ·  P1
- **Purpose:** the security view — every boundary, what crosses, where validation/redaction happens.
- **Must show:** the 6 boundaries (host; UI↔daemon UDS peer-auth; daemon↔Brain stdio no-port; daemon↔agents stdin-not-argv + can_use_tool/approvals; daemon↔integrations TLS + keychain tokens; future iOS→relay→Gateway); redaction-before-persist/embed/sync; secrets in keychain (pointers-only in DB); sensitivity levels on data at rest; the largest residual risk (no agent egress isolation).
- **Anchors:** AD `§15`; `THREAT_MODEL §2/§4`; `EM §9`; `DFR §5/§6`.
- **Format:** Mermaid `flowchart` with boundary subgraphs (or Excalidraw for clarity).

### D7 — Project Brain seam  ·  P1
- **Purpose:** show reason-vs-execute concretely across the MCP seam.
- **Must show:** daemon `brainclient` owning the stdio sidecar lifecycle (spawn/ping/restart/process-group kill); Brain tool calls → Gateway intents (never direct writes); events → outbox → MCP-notification adapter → Brain ingestion; Brain citing platform objects by shared ID; graceful degradation when Brain absent/stale; the future HTTP-transport upgrade path (dotted, deferred).
- **Anchors:** AD `§13`; `PBI §1/§6/§7`; `ADR-005`.
- **Format:** Mermaid `flowchart` + a small `sequenceDiagram` for the action-plan loop.

### D8 — PRD §25 demo sequence  ·  P0 (for the build + the actual demo)
- **Purpose:** the end-to-end happy path the MVP slice is built backward from — proves the spine in one sequence.
- **Must show:** the 17-step demo as a sequence across UI / daemon (Gateway+store) / Claude adapter / git / GitHub / Brain: add project → detect → execution profile → plan task → worktree+session → sidebar/graph → permission → Human Input Queue → approve → edit → review diff → ask Brain (evidence chips) → Brain proposes PR plan → approve → PR created → links + events.
- **Anchors:** AD `§18.1`; `PRD §25`.
- **Format:** Mermaid `sequenceDiagram` (lifelines = UI, Daemon/Gateway, Claude session, Git, GitHub, Brain).

### D9 — Process/survival & lifecycle  ·  P2
- **Purpose:** clarify the detached-daemon lifecycle and the survival policy (the riskiest mechanics).
- **Must show:** daemon detach (launchd/setsid) + pidlock single-instance; UI restart → reconnect-live; daemon restart → resume (`claude --resume`/`codex thread/resume`) else serialized-scrollback replay + relaunch; lease reclaim + new fencing token on restart; orphan/zombie kill via process groups.
- **Anchors:** AD `§4.3`, `§12`; `ADR-002/008/010`; `RESEARCH R-PTY/R-PROC`.
- **Format:** Mermaid `sequenceDiagram` (restart scenarios) + `stateDiagram` for the runner.

### D10 — Deployment / packaging / signing  ·  P2
- **Purpose:** the release pipeline and the sharpest packaging risk.
- **Must show:** Tauri bundle (daemon + UI + PyInstaller Brain sidecar) → deep-sign all → notarize → updater; the externalBin notarization spike (#11992); keychain ACL dependency on Developer ID; Codex schema-pin + CI regen.
- **Anchors:** AD `§16`; `ADR-005/011`; `OPEN_QUESTIONS OQ-PLAT-SPIKE-1`.
- **Format:** Mermaid `flowchart`.

---

## Diagram priority summary
- **P0 (build-critical):** D0 system map, D1 Gateway pipeline, D2 event/projection flow, D3 session lifecycle, D8 demo sequence.
- **P1 (clarify seams):** D4 adapter seam, D5 canonical chains, D6 trust boundaries, D7 Brain seam.
- **P2 (operational detail):** D9 survival/lifecycle, D10 deployment.

`[PROPOSED]` Keep all diagrams as in-repo Mermaid in the finalized `ARCHITECTURE.md` (text-diffable, reviewable in PRs); promote D0/D6 to Excalidraw only if the Mermaid version is too dense to read.

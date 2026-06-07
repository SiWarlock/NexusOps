# Claude Code Handoff — NexusOps (Brain 1 → Brain 2)

> **Phase 16 output.** The instruction set for the **next** stage of the cross-model planning chain. This `arch-draft` pass (Brain 1) produced a *rough draft* architecture + supporting artifacts. A **different model** (Claude / Opus, via `/arch-finalize`) now adversarially audits and finalizes it — two brains on purpose.
>
> **Chain:** `arch-draft` (THIS, done) → **`arch-finalize`** (you, next) → `tasks-gen` → `scaffold-generate` → `/tdd` build engine.

---

## Goal

Read the architecture draft + ALL supporting planning artifacts + the upstream specs, run a second-pass gap audit and adversarial scrutiny against the PRD, resolve load-bearing open questions with the human, and produce the **binding `ARCHITECTURE.md`** (repo root) from the project's `templates/ARCHITECTURE.md`. **Then** (and only then) let `tasks-gen` create `MVP_TASKS.md`.

## Do NOT
- Do **not** start implementation or write application code.
- Do **not** generate `MVP_TASKS.md` here (that is `tasks-gen`, after finalization).
- Do **not** silently resolve a load-bearing open question — confirm with the human.
- Do **not** treat `docs/product/ARTIFACT_REGISTER.md` or any `docs/archive/v0.1/*` / "AgentOps Studio" doc as source of truth (stale/legacy; the live `docs/` specs override).

## Inputs (read all, end-to-end)

**This planning chain (`docs/planning/`) — produced by `arch-draft`:**
1. `PRESEARCH.md` — Phase 0 intake, de-dup doc-coverage map, mechanics, MVP-scoped inferences, assumptions, resolved intake constraints.
2. `RESEARCH.md` — sourced, confidence-tagged tech findings (8 clusters) feeding the ADRs.
3. `DECISIONS.md` — **11 locked ADRs** (stack, topology, store, gateway transport, Brain seam, adapters, git/integrations, locks, terminal, survival, signing) with options/fallbacks/what-would-change-this + the 5 carry-to-finalize spikes.
4. `DATA_MODEL.md` — concrete SQLite persistence (tables/DDL sketches, 8 state machines, 22 shared IDs, the 4 gap objects, migrations/recovery/retention, 8 open data questions).
5. `THREAT_MODEL.md` — assets, 6 trust boundaries, Gateway-as-control, 5-level sensitivity/redaction, 13-row threat table, MVP non-goals, 7 spikes.
6. `RISKS.md` — top-5 + 10 product (PR-01..10) + 13 technical (TR-01..13) risks with mitigations/fallbacks/test-signals.
7. `OPEN_QUESTIONS.md` — consolidated, deduplicated register; RESOLVED-by-decision vs OPEN vs SPIKE, by area, with the 6 build-gating spikes called out.
8. `ARCHITECTURE_DRAFT.md` — the anchored first-draft spec (§1–§23) you will finalize.
9. `DIAGRAM_PLAN.md` — D0–D10 diagram plan (produce the P0 ones into the final doc).

**Upstream product/specs (authoritative; the draft references, doesn't restate):**
`docs/product/PRD.md`, `docs/product/PRODUCT_CANON.md`, `docs/architecture/SHARED_OBJECT_MODEL.md`, `docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md`, `docs/domains/ACTION_GATEWAY.md`, `docs/domains/WORKFLOW_PACKS.md`, `docs/domains/CC_CREW_WORKFLOW_PACK.md`, `docs/architecture/PROJECT_BRAIN_INTERFACE.md`, `docs/architecture/DESKTOP_FIRST_RUNTIME.md`, `docs/ux/UX_INFORMATION_ARCHITECTURE.md`, `docs/ux/UI_COMPONENT_INVENTORY.md`.

**Context repos (read-only, for grounding):** `../project-brain` (the sibling Brain product — seam only), `claude-code-tdd-agent-crew-scaffold/` (cc-crew = the build engine + the `templates/ARCHITECTURE.md` you finalize into).

## Instructions
1. Read all inputs end-to-end. Do not implement.
2. Run the **gap audit** (below) across the ~13 dimensions.
3. Identify inconsistencies, missing decisions, unclear boundaries, untestable requirements, scope creep, and missing anchors.
4. Propose precise edits to the architecture.
5. **Confirm every load-bearing change + every still-open question with the human** (use `AskUserQuestion`).
6. Apply confirmed edits; produce the binding `ARCHITECTURE.md` at the repo root from `claude-code-tdd-agent-crew-scaffold/templates/ARCHITECTURE.md`, **preserving the stable `§N` anchors** the draft established (`ARCHITECTURE_DRAFT §22`).
7. Produce the P0 diagrams (`DIAGRAM_PLAN.md`: D0, D1, D2, D3, D8) inline (Mermaid).
8. Only after the architecture is finalized, hand to `tasks-gen` → `MVP_TASKS.md`; every task must reference architecture anchors; do not invent architecture in tasks.

## Gap-Audit Prompt (run this)
Perform a second-pass architecture gap audit. Look for:
- missing user flows · missing lifecycle states · missing failure modes · missing interfaces/schemas · unclear source-of-truth boundaries · unresearched external dependencies · inconsistent decisions · overbuilt scope · missing tests · missing deployment/demo path · missing security/trust boundaries · missing diagram needs · missing anchors for task planning.

Return: (1) Critical gaps, (2) Important gaps, (3) Nice-to-haves, (4) Proposed architecture edits, (5) Questions requiring human decision.

## Things this draft explicitly flags FOR the finalize pass

**Build-gating spikes (resolve/plan before tasks):**
- **OQ-PLAT-SPIKE-1** — macOS notarization of the bundled PyInstaller Brain sidecar via Tauri `externalBin` (#11992) on a real signed build. *Sharpest packaging risk.*
- **OQ-HARN-SPIKE-2** — the human-interactive-PTY vs SDK-driven-session handoff model for Claude (when does the human "own" a session vs the Gateway).
- **OQ-DATA-SPIKE-3** — SQLite single-writer write-contention load test under many concurrent agents (before freezing single-writer).
- **OQ-HARN-SPIKE-4** — Codex `app-server` schema-pin + CI schema-regen policy (protocol churn).
- **OQ-DATA-SPIKE-5** — reconcile the 4 gap objects (Device/RemoteClient/LocalRunner/EventProjection) into the Shared Object Model.
- **OQ-INT-SPIKE-6** — octocrab merge-PR / GitHub-App ergonomics + libgit2 relative-worktrees spot-check.

**Specific consistency checks owed:**
- The **requirements→flow matrix**: verify every MVP requirement (`PRD §10/§15`) maps to a flow (`ARCHITECTURE_DRAFT §8`, `PRD §9`, `UX §10`) — the draft flagged this `RESEARCH`/unverified (playbook stop-condition).
- The **8 status state machines** reconcile across `SOM`, `UX §8`, and `DATA_MODEL §4` (note any drift).
- **Actor-enum naming**: `EM §7` uses `remote_device`; the draft/`DATA_MODEL §6.2` proposes `remote_client`; pick one canonical token across events + gateway.
- **ID format**: confirm prefixed-ULID (`DATA_MODEL §5`) vs UUIDv7 before any schema freezes.
- **Task vs PlanTask** modeling (one table + discriminator vs two; `SOM §37 Q2`, `DATA_MODEL §8`).
- **Worktree↔Repo cardinality / multi-repo**, **branch co-ownership**, **AgentTeam PR reconciliation** (`SOM §37 Q1/Q3/Q4`).

**Adversarial scrutiny targets (where this draft is most likely wrong/thin):**
- The detached-daemon lifecycle (single-instance, stale socket, UI↔daemon version skew, orphan kill) — is the mitigation set sufficient? (`RISKS TR-13`)
- Session-survival across daemon restart, esp. lossy alt-screen/raw-mode TUI re-attach (`RISKS TR-01`) — is resume-or-replay the right policy, or should MVP scope to replay-only?
- Codex parity claims — is `app-server` stable enough to be MVP-blocking, or should Codex be P1 with `exec --json` as the MVP stopgap? (User chose both-in-MVP; pressure-test it.)
- Whether "both harnesses MVP-blocking" + "detached daemon day one" overloads the MVP slice vs the credible-demo goal.

## Artifacts written by this pass (inventory)
`docs/planning/`: `PRESEARCH.md`, `RESEARCH.md`, `DECISIONS.md`, `DATA_MODEL.md`, `THREAT_MODEL.md`, `RISKS.md`, `OPEN_QUESTIONS.md`, `ARCHITECTURE_DRAFT.md`, `DIAGRAM_PLAN.md`, `CLAUDE_CODE_HANDOFF.md` (this file).

## Still-open / research-required for finalize to resolve
Full register: `OPEN_QUESTIONS.md`. The headline unresolved items: the 6 build-gating spikes above; the requirements→flow matrix; the 8 open data questions (`DATA_MODEL §8`); the 7 threat-model spikes (`THREAT_MODEL`); P1/P2 deferrals (`ARCHITECTURE_DRAFT §18.2`). Naming is **NexusOps** (user-locked); Brain scope is **seam-only** (user-locked); demo is **PRD §25** (user-locked) — do not re-open these.

---
*Generated by `arch-draft` (Brain 1) on 2026-06-06. Next: run `/arch-finalize`.*

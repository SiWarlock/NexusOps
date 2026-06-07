# UI/UX Reconciliation — NexusOps-ui-kit ↔ binding specs

> **Purpose:** Consolidate the 6-lens prototype review (`docs/ui-review/{A-screens,B-status,C-objects,D-gateway-brain,E-netnew,F-visual-a11y}.json`) into tasks-gen-ready UI/UX requirements, so the prototype's design system AND the gaps both flow into `MVP_TASKS.md`. Reviewed 2026-06-07 against the finalized `ARCHITECTURE.md` + PRD + UX docs.
> **Overall verdict:** **Strong, build-worthy prototype, substantially aligned.** It is the canonical *visual* pass the design-system prompt deferred. Gaps are mostly **(1) net-new architecture surfaces it predates**, **(2) status vocabulary undersized vs the reconciled 9 state machines**, **(3) Gateway = single-action → must become multi-step ActionPlan (owner decision O-3)**, **(4) a few accessibility MUSTs**. Tally: 16 critical · 24 important · 9 nice-to-have.
> **Tag legend:** `[KIT-CHANGE]` build the kit differently · `[ADD-SURFACE]` net-new screen/affordance to build · `[P1]` defer · `[DOC]` update a binding doc · `[DECISION]` needs owner.

---

## 0. What the prototype gets RIGHT (carry forward as the design baseline)
NexusOps naming + "air-traffic control" thesis; the **Attention Ladder** (attention-first ordering); the full core object model; **two-tier, oklch, re-hueable tokens** (primitive→semantic); a genuine **four-channel "never color alone"** status system (color + glyph + label + motion) with grayscale-safe hazard hatch; **EvidenceChip = all 10 evidence types**; **RiskBadge = 5 levels + mandatory text label**; **UsageMeter = exact/estimated/unavailable**; DiffHunk per-hunk actions; **Brain = propose-not-execute** with evidence + freshness; approval as a **structured card outside terminal text**; centralized grouped **Human Input Queue**; **pack≠instance** lock; Plan view with architecture anchors; the **Usage dashboard**; Audit trail + activity dock; **prefers-reduced-motion** handled. This is a strong foundation — reconcile, don't restart.

---

## 1. Status & state-machine binding `[KIT-CHANGE]` (lens B, C) — CRITICAL
The kit's status vocabulary is a small, hyphenated, free-text set; bind it to the canonical §5.1 enums.
- **Session: render all 17 states** incl. the newly-added **`changes_ready`** (+ creating, starting, thinking, running_command, editing_files, running_tests, waiting_on_external_service, idle, stale, killed). Add a Command Center/Sessions "Changes ready" grouping + the Flow-H "session reports changes ready → open review" affordance.
- **Status keys = canonical enum strings** (`proj_session.status` verbatim, snake_case); display labels are a separate copy layer ("Waiting on you" is fine copy for `waiting_on_human_input`).
- **One canonical status→attention-rank table** covering every state of all 10 machines (incl. ExecutionProfile via ProfileBadge); sidebar weight + queue membership + sort order all derive from it. **No silent fall-through to idle** (today `waiting_on_permission`, conflict, stale, blocked, changes_ready all floor to rank 0 and never enter "Needs my attention"). Order per PRD §5.2.
- **Split Approval vs ActionRequest** status (R-5): two distinct pills; render executing / partially_succeeded / rolled_back / rollback_failed / policy_decided / edited / auto_approved_by_policy / escalated / expired.
- **Worktree = derived** two-axis status (git-axis + lifecycle overlay) with the precedence collapse (§7.2); surface all UX §8.3 states, not {dirty, clean, conflict}.
- **PR = canonical 11 states** (drop the duplicated `lane`+`status` fields); "mergeable" gated on a fresh GitHub re-fetch (§7.2).
- **ExecutionProfile = 8 states** (add in_use, misconfigured, unknown) with config-vs-runtime split (runtime re-derived on restart).
- **WorkflowInstance = 12 / ProjectBrain = 10 / AgentTeam = 9** canonical enums (replace ad-hoc project-row strings; AgentTeam gets its OWN status, not member-session status — incl. reconciling_outputs/blocked).
- **Task/PlanTask = R-8 superset**, kind-scoped subsets (Plan View shows the plan_task subset; not "Backlog/Todo").
- **`stale` is daemon-time-derived** (heartbeat-age), recomputed on projection rebuild — not a stored event status; exercise it in sample data.
- *Test:* one StatusPill rendering per state of every machine; every Session state resolves to a ladder level.

## 2. Net-new architecture surfaces `[ADD-SURFACE]` (lens E, A) — CRITICAL
The prototype predates these; they are now binding (ARCH §4/§7.2/§9.1/§15/§16/§17):
- **Daemon-connection indicator** (connected / reconnecting / disconnected), **distinct from** LocalRunner health (today one static "runtime healthy" chip conflates them), + a **global READ-ONLY degraded mode** that disables every intent-submitting control (Gateway approve/deny, Dispatch, Brain Run-via-Gateway, commit/push) with a "daemon unavailable — reconnecting" banner + Retry/Repair. (ARCH §4/§12/§16; UX Screen 2 error.)
- **Survival/recovery UX (O-2):** post-restart recovery banner ("Reconnected — view restored; N resumed, M replayed"); per-session **resumed-(live) vs replayed-(relaunched)** indicator; "Restart session" affordance for sessions that failed during recovery. (ARCH §8 recovery rows, §17, ADR-010.)
- **Codex context-% = "unknown"** — `[KIT-CHANGE]` set Codex sessions' context to null and render "unknown"/n/a (never a number or 0%) per `supportsContextMetadata=false`; carry metric_quality on telemetry. (ARCH §9.1/§7.2; kit currently shows Codex s2=96/200, s4=58/200 — a live contradiction.)
- **First-Launch Setup Wizard + macOS consent/TCC map** — stepper (welcome, runtime check, Claude/Codex detection, Execution Profiles, Brain, git/GitHub/Linear, Workflow Pack library, security/approval policy, finish/add-project), each as an idempotent/reversible/skippable Gateway intent; consent card + denied-degraded + repair for keychain ACL, **notification permission, Full Disk Access, launchd Background Item (SMAppService), AppleEvents**. (ARCH §11/§16/§8; UX Screen 1.) **Entirely absent today.**
- **Fencing/hard-conflict card** in the Human Input Queue (new "Conflicts" group) showing the contended resource + competing lease owners/fencing tokens; **never auto-resolved**, no silent defer. (ARCH §17.)
- **Fail-closed / audit-integrity alert** (loud system banner when an authoritative event fails to write/is quarantined) + "unknown outcome" / "partially succeeded" / "rollback failed" result treatments in Audit + HIQ. (ARCH §15/§17.)
- **Agent-SDK credit-pool meter** in Usage + on Claude profiles (near-exhaustion + exhausted/hard-stop states), distinct from token spend + interactive usage (effective 2026-06-15). (ARCH §9.1.)
- **Native desktop notifications** settings (per-type toggles for the 9 UX §11.5 types; permission state + request; "previews redacted" note) wired to the notifier triggers; keep in-app bell as the mirror. (ARCH §10 DESK-8/§16.)
- **Version-skew + update states:** blocking UI↔daemon mismatch ("update required"/relaunch); DB-migration progress + "update failed, rolled back to vN"; app-update-while-running affordance. (ARCH §6.4/§16.)
- **Degraded/offline/stale variants** as first-class: integration "stale (offline)" badge + queued-writes, Brain-degraded, projection-rebuilding markers. (ARCH §17/§13.1; UX §12 item 20.)

## 3. Action Gateway → multi-step ActionPlan + Brain handoff `[KIT-CHANGE]` (lens D) — CRITICAL (O-3)
- **Gateway Review Modal accepts an `ActionPlan` (1..N steps)** and renders the UX Screen-16 layout: per-step rows (step #, action type, target, **risk 0-4**, preconditions, **preview/dry-run status** incl. pending/unavailable+reason/stale, rollback availability, step status), affected ResourceRefs, EvidenceRefs, permissions-required, audit note. A single ActionRequest is the N=1 case of the same component.
- **Full Screen-16 controls:** approve-all-eligible (critical/4 auto-excluded), approve-individual-step (`step_id`), edit-before-approve (re-preview+re-risk; never execute an unapproved diff — stale-precondition → fresh approval), remove-step, require-manual-execution, deny-with-reason, save-as-policy (→ `policy_grant`; "Always allow" maps here).
- **Brain "Run via Gateway" submits the exact reviewed `plan.steps`** (today it opens an unrelated static single action) — preserve step count/risk/targets/evidence; Brain stays propose-only.
- **Brain drawer:** add the **`Actions` mode** (proposed/pending/executed plans w/ live approval state) + reconcile `Dispatch`; make modes **functional** (each renders its own view; Decisions/Memory reachable in the drawer form); full **scope chips** (workspace/project/session/file/diff-selection/PR/plan-task/agent-team — and actually constrain retrieval); header shows **live ProjectBrain index status + grounded-at/staleness + privacy/transport indicator**; per-answer **confidence/verification** + "unverified claim" treatment.
- **Human Input Queue:** add the missing 3 of 7 groups — **Needs clarification, Project Brain action plans, Agent team escalations** — and full card actions (Edit instruction, Open terminal, Open diff, Ask Brain, Approve once, Set rule, Escalate) + **expiration** field; Brain-plan cards open the multi-step Gateway modal; stamp **`actor_type=project_brain`** on Brain-originated items.
- **Risk = numeric 0-4** resolved per action-type from the §6.3 ActionTypeCatalog (not ad-hoc strings); support a risk range where declared.

## 4. Component contract fidelity `[KIT-CHANGE]` (lens C) — IMPORTANT
- **UsageMeter ring variant** must also render the accuracy label (it drops it today) and show "unknown" (not 0%/empty ring) when unavailable/NULL — the ring is what SessionRow + graph nodes use.
- **SessionRow** add `model` + team/role (when `agent_team_id` set); add `waiting_on_permission` (and all waiting_on_* ) to its attention map (no fall-through to idle).
- **GraphNode** add team_lead, orchestrator, generic Task (vs PlanTask), Workflow command node types (→ full OBS-2/UX §9 set).
- **EvidenceChip** model the full 5-state freshness lifecycle (live/stale/moved/unverified/unavailable), each text/aria-labeled; surface `confidence`; bind freshness to live events.

## 5. Accessibility MUSTs `[ADD-SURFACE]`/`[KIT-CHANGE]` (lens F) — CRITICAL/IMPORTANT
- **Project Graph list/table fallback** — functionally equivalent (same nodes+edges, status, ownership, attention), Graph|List toggle, keyboard-reachable. **Binding + tested (OBS-6); absent today.**
- **Global `:focus-visible` ring** on every interactive control (tokens exist but are never applied; readme promises a 2px azure ring). Keyboard users currently can't see focus.
- **Every drag semantic gets a non-drag equivalent** (TASK-5): task-chip overflow (Dispatch new / Send to session / Delegate to team), session-row "Add task as context", Dispatch dialog target selector {new session, existing session, team}. (Today only the "new session" path has a click equivalent.)
- Graph nodes keyboard-operable (roving tabindex + Enter/Space) OR the list fallback is the designated keyboard surface; node accessible names include type+status+attention.
- Capacity meters convey threshold crossing on a non-color channel (tick or normal/approaching/hard-stop label).
- AttentionMarker rail must co-locate glyph+label so attention is ≥3 non-color channels; beacon animates on the level-5 rail.

## 6. Screen coverage & canonical mapping (lens A)
Canonical screen → kit surface (full table in `A-screens.json`):
- **Present (16/20):** Command Center, Project Graph, Session Terminal, Code/Diff Review, Editor, Plan View, Task Inbox, Agent Team, Workflow Packs, Brain drawer+page, Human Input Queue, Gateway modal, Execution Profiles, Usage (folded into Settings — sound), Events/Audit + Activity dock, Worktree/Git/PR (folded into Code/Diff tabs — sound).
- **Gaps:** **Setup Wizard ABSENT** `[ADD-SURFACE]` (§2 above); **PR Review Workspace** collapsed to a single-PR diff — missing checks/reviews/mergeability/Brain-evidence panels + merge controls `[DECISION]` (own surface vs expand Code "Review" mode); **Sessions List/Board** folded into Command Center — no dense table/board with column set + per-row actions `[KIT-CHANGE]`; **Workflow Personalization Review** minimal `[P1]` (keep deferred per ARCH §19.2, but the MVP Gateway must still render the multi-step personalization plan); **Audit detail inspector** (correlation/causation/sensitivity/payload) absent `[KIT-CHANGE]`; **Remote Access / iOS pairing** absent `[P1/P2]` (keep deferred per ARCH §19.2).

## 7. Visual system ratification `[DECISION]` (lens F)
The kit IS the later visual pass the design-system prompt deferred; "Graphite Arc" is a documented, two-tier, re-hueable first-pass palette that legitimately resolves the "no color taxonomy in the first pass" constraint and satisfies "never color alone." **Owner decision:** adopt the NexusOps-ui-kit (tokens + components + Graphite Arc) as the **canonical design system** that `tasks-gen` references, treating remaining color as a tunable first pass.

---

## 8. Disposition summary for tasks-gen
- **Adopt** the kit as the canonical design-system + component inventory (pending §7 ratification).
- **Bind** all status rendering to the §5.1 enums + one attention-rank table (§1).
- **Build the net-new surfaces** (§2) — daemon health/read-only, survival banners, fencing/fail-closed cards, Setup Wizard + consent map, notifications, version-skew, credit-pool.
- **Upgrade the Gateway to ActionPlan + step approval** and wire the Brain handoff (§3, O-3).
- **Fix the component contracts + a11y MUSTs** (§4, §5) — graph list fallback + focus ring + drag→non-drag are binding/tested.
- **Resolve the screen scope calls** (§6, §7) with the owner.
- Per-requirement detail + `for_tasks_gen` strings live in the six `docs/ui-review/*.json` lens files.

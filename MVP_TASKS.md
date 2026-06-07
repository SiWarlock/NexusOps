# MVP_TASKS.md — NexusOps

> **Phase note.** Spec-anchored task tracker for the NexusOps MVP, decomposed from the binding `ARCHITECTURE.md` (every phase cites its `§N` anchors; cross-doc-invariant models mirror `ARCHITECTURE.md` Appendix A). Scope is the **comprehensive MVP** the owner locked at arch-finalize (`§0.1`): both Claude Code **and** Codex live (O-1), **full resume-or-replay** survival (O-2), **full multi-step Brain action plans** (O-3), Claude supervision mode pending-spike (O-4), `NexusOps-ui-kit`/"Graphite Arc" as the canonical design system (O-5), full **PR Review Workspace** (O-6). macOS-only. Built backward from the **PRD §25 demo** (`§19.1`). Build order = invariants → lifecycle correctness → tests → local demo → polish. Living-state sections below start empty and accrete through `/tdd` work.

> **Session protocol:**
> - **At session start** — orchestrator runs `/orchestrate-start`; implementer runs `/session-start`. Confirm what's targeted this session. **Re-read the phase's `Spec anchors:` sections of `ARCHITECTURE.md` first.**
> - **At session end** (only when the user says we're done):
>   - **Implementer** runs `/session-end` — TDD audit + cross-doc audit + Step-9 list + session doc + `/preflight`. Does NOT touch this doc.
>   - **Orchestrator** runs `/orchestrate-end` — verify hot routing landed, reconcile checkbox state, append Log entry, update Decisions / Carry-forward / Currently in progress, triage Carry-forward, round commit + push.

> **Reference deadlines:**
> - Solo + AI build (cc-crew); no hard calendar deadline — optimize for the PRD §25 demo as the proof.
> - **Phase 0 spikes gate their dependent phases** — do not start a phase whose gating spike is unresolved (see Phase 0).

> **Spec-anchor convention (architecture-as-contract).** Each phase header carries a `**Spec anchors:**` block listing the `ARCHITECTURE.md` sections it implements. Orchestrator + implementer re-read those anchors at session start. If a slice surfaces a behavior the anchors don't cover, that's a cross-doc invariant flag at TDD Step 9 — either the anchor is missing (→ re-run `/arch-finalize`) or the implementation drifted. Architecture is contract; drift surfaces structurally.

---

## Currently in progress

**PHASE 0 — DETERMINISTIC WORK COMPLETE (2026-06-07).** All 4 spikes landed (`docs/spikes/*.md`) + the **0.5 contract freeze landed (06f9576)** — the serial neck. Single-writer holds (0.4 → §18 written MEASURED); git2 reads relative-worktrees (0.3 → §9 corrected); #27203 confirmed, bg subagents forbidden (0.1); shared/ contracts frozen Option-A (0.5, §5.0). Toolchain RESOLVED (Lesson §1).

**PHASE BOUNDARY — awaiting direction (surfaced to lead):** the fan-out trigger is met. Decision forks: **(a)** user-invoked **Phase-0-exit `/arch-finalize` re-validation** (reconcile upstream planning drift — DATA_MODEL §4/§6.4, EM §7 — before Phase 1 binds); **(b)** open the **ui track** (now unblocked by the frozen contracts); **(c)** next daemon-core slice = **Phase 1 / 1.1** (event store; deps 0.4+0.5 met). Recommend `/orchestrate-end` first (commit + push the accumulated Phase-0 round) so any `/arch-finalize` runs against a clean state.

**Still open (non-blocking for Phase 1):** HITL — notarization run (0.2, Apple creds), credit-pool drain ≥6/15 (0.1) → then cat-4 SDK-vs-PTY + **0.5b** (re-freeze ExecutionProfile).

---

## Carry-forward to upcoming briefs

Items the orchestrator MUST fold into upcoming slice briefs. **Triaged at every `/orchestrate-end`** — NOT append-only. New entries carry `(origin: YYYY-MM-DD <slice-id>)`.

_(Empty for next-brief purposes — 1.1 has no carry-forward dependencies; it binds to the frozen `shared/` contracts + `ARCHITECTURE.md` directly. 0.5b inlined as a Phase-0 task. Two cross-track op-items spread to their consumer slices below.)_

**Spread (kept; auto-resolve at the consumer slice):**
- **ui track generates Zod from `contracts/schema/`** (`pnpm` broken → use `npx json-schema-to-zod`, per the 0.3 env note). `last-consumer-slice: ui-track open (Phase 6)` `(origin: 2026-06-07 P0.5)`
- **Wire the §5.0 contract gates into CI** — schema-diff (test 9) + 3-way verify (test 8) + the Codex schema gate — no `.github/workflows/` exists yet; they run in `cargo test`/harness locally meanwhile. `last-consumer-slice: Phase 10 (release infra), or earlier if CI is stood up` `(origin: 2026-06-07 P0.5)`

_(LOCKED-text corrections + planning-doc reconciles are tracked in "Architecture-doc corrections" + the Phase-0-exit `/arch-finalize` reconcile list below.)_

---

## Deliverable map

| Deliverable | Status | Delivered by |
|---|---|---|
| PRD §25 demo runs end-to-end (add project → launch session → permission → approve → review → Brain PR plan → PR created + linked) | ❌ | Phase 10 |
| Detached Rust daemon: event store + projections + IPC GatewayPort + leases | ❌ | Phase 1 |
| Action Gateway: typed actions, risk 0-4, ActionPlan + step approval, audit, fail-closed | ❌ | Phase 2 |
| Claude **and** Codex adapters behind one HarnessAdapter contract + embedded terminals | ❌ | Phase 3 |
| Full session survival (resume-or-replay) + failure-mode contract | ❌ | Phase 4 |
| Project registry + dual-git/worktrees + Execution Profiles (keychain) | ❌ | Phase 5 |
| Tauri shell + projection-driven UI (design system, status binding, a11y) | ❌ | Phase 6 |
| GitHub/Linear integration + full PR Review Workspace + Task Inbox | ❌ | Phase 7 |
| Project Brain stdio-MCP seam + drawer (multi-step action plans, evidence) | ❌ | Phase 8 |
| Workflow Pack detection + cc-crew + Plan view | ❌ | Phase 9 |
| Signed/notarized macOS app + first-run bootstrap + consent map | ❌ | Phase 10 |

---

## Phase exit checklist (template — applies to every phase)

Before ticking a phase complete:

- [ ] **All phase task checkboxes ticked.** Conservative — partial work stays unchecked with a Log note.
- [ ] **Acceptance criterion met.** `/preflight` clean + manual smoke if there's runtime behavior.
- [ ] **`/preflight` clean.** Includes architecture-invariant tests (esp. INV-SEC-1, §15).
- [ ] **Cross-doc invariants verified.** No model field change without an `ARCHITECTURE.md` Appendix A edit in the same round.
- [ ] **Session doc(s) for this phase exist** and list every file created/modified.
- [ ] **Commits pushed to the git remote.**

---

## Final-submission acceptance criteria (project-level)

The MVP is "done" when:

- [ ] The **PRD §25 demo** runs end-to-end locally (`§19.1`).
- [ ] **INV-SEC-1** holds and is tested: no mutation path bypasses the Action Gateway (`§15`).
- [ ] Both **Claude Code and Codex** sessions launch, are supervised with reliable non-scraped status, and survive UI restart + resume-or-replay across daemon restart (`§9.1`, `§17`, O-1/O-2).
- [ ] The app is **signed + notarized** (incl. the Python Brain sidecar) and first-run bootstrap + consent map work (`§16`).
- [ ] Accessibility MUSTs pass: graph list/table fallback, focus ring, drag→non-drag (`§11.6`, tested `§14`).

---

## Parallelization & track plan

Running this with multiple agent teams. The architecture is "daemon = single source of truth + sole mutator," so most surfaces consume daemon contracts — but **Phase 0.5 freezes those contracts into `shared/` up front**, which lets tracks build in parallel against the frozen *interface* (mocking the other side; the arch even mandates a mock GatewayPort for UI tests, §14). **The fan-out trigger is the end of Phase 0 — not daemon progress.**

**Sequence:** spikes (parallel) → **freeze contracts (serial neck, 0.5)** → `daemon-core` ∥ `ui` [∥ `edges`] → converge → demo.

**Tracks** (`/team-start <track>`; track-prefixed `<track>-<area>-<role>` so peer DMs don't cross-bleed):

| Track | Phases | Area | Runs independently because… | Mocks | Gated by |
|---|---|---|---|---|---|
| **shared** (the neck) | 0 | `shared/` | one team; 6 spikes (parallel) then the contract freeze | — | — |
| **daemon-core** (critical path / long pole) | 1 → 2 → 4 | `daemon/` | it *is* the foundation everyone consumes | — | 0.4, 0.5 |
| **ui** (biggest independent arm) | 6 | `ui/` | projection-driven; the NexusOps-ui-kit already exists | GatewayPort + projections (fixtures) | 0.5 |
| **edges** (optional 3rd) | 3, 5, 7 | `daemon/` (+`ui/` for PR review) | executor edges submit intents to the frozen Gateway iface | Gateway executor iface | 0.1, 0.3, 0.5 |
| **converge** | 8, 9 | `daemon/`+`ui/` | layer on once core + edges exist | — | per-phase |
| **integration** (final gate) | 10 | all | the demo needs everything | — | ~all (esp. 0.2) |

**Recommended cadence:**
1. **One team — `/team-start daemon` → all of Phase 0.** Run the spikes in parallel; then **0.5 contract freeze** (the daemon team owns `shared/`). This is the only serial neck.
2. **0.5 lands → `/team-start ui`** (parallel). Two arms now: daemon-core (1→2→4) + ui (6). The UI builds against the frozen enums/projections + a mock gateway-client + the NexusOps-ui-kit; it integrates with the real daemon per slice once that slice's daemon-side contract is live.
3. **Bandwidth + 0.1/0.3 resolved → optionally `/team-start edges`** (3/5/7).
4. **Converge** (8/9), then the **demo/deploy gate** (10) — all hands, not parallelizable.

**Ceiling:** 2 tracks is the sweet spot, 3 if you have bandwidth. You are the escalation conduit for every team — past ~3 you become the bottleneck (the product's own "human attention is the scarce resource" thesis applies to *you*). **Don't fan out before 0.5** — contract churn after fan-out thrashes every track. The per-phase `Track / deps:` lines below make the dependency graph explicit.

---

## Phase 0 — Pre-build spikes & contract freeze

**Goal:** Resolve the build-gating spikes and freeze the shared contracts so downstream phases bind to stable enums/IDs. These are validation/decision tasks, not TDD code; each ends in a recorded decision (→ Decisions tabled / a `DECISIONS.md` note) or a `ARCHITECTURE.md` confirmation. **Gates the phases noted.**

**Spec anchors:** `ARCHITECTURE.md §23` (spikes), `§0.3` (reconciliations), `§5.1`, `§5.2`, `§9.1`, `§13.1`, `§16`.

**Track / deps:** `shared` (the serial neck — one team). **Deps:** none. **Parallel:** the 6 spikes run concurrently; **0.5 (contract freeze) is gated on 0.1 + 0.3** and is what unlocks every downstream track.

### 0.1 — Claude supervision-mode spike (O-4) — *gates Phase 3 Claude adapter*
- [x] Empirically map `can_use_tool` coverage across direct bash/Write/Edit, MCP tools, Task subagents (fg/bg), and each permission mode; confirm the #27203 background-subagent gap. — **#27203 CONFIRMED present on CC 2.1.168** (closed won't-fix); bg subagents stay forbidden (matches §9.1, no change).
- [ ] Measure Agent-SDK credit-pool behavior (2026-06-15) vs interactive-PTY; decide SDK-driven vs interactive-PTY-primary; record the criterion + decision. — **PARTIAL:** criterion + both branches recorded (`OQ-HARN-SPIKE-7.md §3`); drain measurement = HITL checklist (§5), user runs ≥ 2026-06-15. **Decision NOT made (cat-4 — see Decisions tabled).**
- [x] Files: `docs/spikes/OQ-HARN-SPIKE-7.md` (NEW)
- [x] Cross-doc invariant: none (resolves O-4; #27203 confirmed → §9.1 unchanged)

### 0.2 — macOS sidecar notarization spike (OQ-PLAT-SPIKE-1) — *gates Phase 10 packaging / Phase 8 Brain bundling*
- [ ] Validate deep-signing + notarizing a bundled PyInstaller sidecar via Tauri `externalBin` (#11992) on a real signed build; if blocked, confirm the loopback-HTTP Brain fallback (§13.1). — **PARTIAL:** turnkey checklist + loopback-HTTP fallback decision tree drafted (`OQ-PLAT-SPIKE-1.md`); **HITL run pending** (user's Developer ID / notary creds).
- [x] Files: `docs/spikes/OQ-PLAT-SPIKE-1.md` (NEW)
- [x] Cross-doc invariant: none

### 0.3 — Codex app-server schema-pin + git2/octocrab spot-check (OQ-HARN-SPIKE-4, OQ-INT-SPIKE-6) — *gates Phase 3 Codex / Phase 5 git / Phase 7 GitHub*
- [ ] Pin the Codex version; generate + diff the app-server JSON-RPC schema bundle; confirm the stable method set + modern/legacy approval shapes; wire the CI schema-diff gate. — **PARTIAL:** codex CLI absent locally → pin procedure + CI schema-diff gate scaffolded (`OQ-HARN-SPIKE-4.md`); **live schema capture is HITL** (run once codex installed).
- [x] Verify git2 read-path survives `extensions.relativeworktrees` repos (else CLI-read fallback); spot-check `octocrab` `pulls().merge()` + GitHub-App token flow. — git2 1.9.4 **CAN** read relative-worktree repos (ADR-007 premise stale → cross-doc flag); octocrab 0.53 merge + `gh auth token` adequate (`OQ-INT-SPIKE-6.md`).
- [x] Files: `docs/spikes/OQ-HARN-SPIKE-4.md`, `docs/spikes/OQ-INT-SPIKE-6.md` (NEW)
- [ ] Cross-doc invariant: ~~none~~ → **libgit2 relative-worktree read-gap closed (libgit2 ≥ 1.9.4)** contradicts LOCKED ADR-007 + §9 aside → route `/arch-finalize` (see Carry-forward; escalated to lead).

### 0.4 — SQLite single-writer load test (OQ-DATA-SPIKE-3) — *gates Phase 1 event-store freeze*
- [x] Load-test the single-writer event-store path at N=20 concurrent mutating agents; record intent-commit p95 + reader latency; confirm/adjust the §18 budgets; document the ceiling. — single-writer **holds** (p95 5.35 ms fresh / 8.44 ms @1M; reader ≤ 0.38 ms; ceiling > N=100).
- [x] Files: `docs/spikes/OQ-DATA-SPIKE-3.md` (NEW)
- [x] Cross-doc invariant: none (§18 numbers written into `ARCHITECTURE.md §18` — MEASURED + §14 CI guards committed).

### 0.5 — Contract freeze: enums, IDs, gap objects (OQ-DATA-SPIKE-5) — ✅ **LANDED 06f9576** (Option A, §5.0)
- [x] Freeze the canonical enums (§5.1 nine machines), the 22 shared IDs + prefixed-ULID format (§5.2), the actor enum (R-2), and the 4 desktop-addendum objects (§5.3) as `shared/` constants; reconcile into SOM. — **ExecutionProfile runtime-state enum HELD for 0.5b** (cat-4); everything else frozen.
- [x] Files: `shared/` Rust authority crate (status/actor/ids/objects/schema + emit_schema bin + contract tests) + `contracts/schema/nexusops-contract.schema.json` (published) + `contracts/verify/` (3-way harness). 14 files.
- [x] Cross-doc invariant: Appendix A annotated (frozen-in-`shared/` + prefix map) + `daemon/CLAUDE.md` cross-doc rows + LESSONS §2. (Orchestrator-written; rides the round commit.)
- [x] Tests: 11 Rust contract tests + the 3-way (Rust/Zod/Pydantic) equality harness + schema-diff gate — all green; clippy `-D warnings` clean.

### 0.5b — Re-freeze the ExecutionProfile runtime-state enum — ⏳ **GATED (cat-4, ≥2026-06-15)**
- [ ] Re-freeze the `ExecutionProfile` runtime-state enum (held out of 0.5 — credit-pool-adjacent: `rate_limited`/`auth_expired` + possible SDK-credit-exhaustion value) into `shared/` via the same Option-A mechanism (§5.0); regenerate schema + Zod/Pydantic consumers; extend the 3-way verify.
- [ ] **Gate:** the cat-4 SDK-vs-PTY decision (see Decisions tabled) — needs the ≥6/15 Claude credit-pool drain data first.
- [ ] Also narrow the deferred `#[allow(unreachable_patterns)]` in `shared/src/status.rs` (0.5 code-quality low) while touching the crate.
- [ ] Cross-doc invariant: add the ExecutionProfile machine to Appendix A's frozen set + the `daemon/CLAUDE.md` cross-doc row (flip "10 total: 9 frozen + ExecutionProfile held" → "10 frozen").

### Acceptance criteria (0)
- [x] All 0.X spikes resolved with a recorded decision (`docs/spikes/*.md`). _Architecture changes recorded as direct anchored edits (§5.0, §9, §18) — a **user-invoked Phase-0-exit `/arch-finalize` re-validation** is the natural gate to reconcile the upstream planning-doc drift (DATA_MODEL §4/§6.4, EM §7) before Phase 1 binds._ **HITL executions still open** (notarization run; credit-pool drain ≥6/15) — staged, non-blocking for Phase 1.
- [x] Shared contracts frozen in `shared/` (06f9576); Appendix A models codified + annotated. (ExecutionProfile held → 0.5b.)

---

## Phase 1 — Daemon foundation: event store, projections, IPC, locks, bootstrap

**Goal:** The trust-core spine — a detached Rust daemon owning a single-writer WAL SQLite event store with rebuildable projections, an outbox, persistent lease locks, a UDS `GatewayPort`, single-instance + first-run bootstrap, and migrations with backup/rollback. Everything downstream writes through this.

**Spec anchors:** `ARCHITECTURE.md §4`, `§6.1`, `§6.4`, `§7`, `§7.1`, `§7.2`, `§5.2`, `§8` (recovery rows), `§16` (bootstrap/migration/version-compat), `§12`.

**Track / deps:** `daemon-core` (critical path / long pole). **Deps:** 0.4, 0.5. **Parallel-with:** Phase 6 (ui, via mock GatewayPort + fixture projections).

### 1.1 — Event store: append-only events + envelope + FTS5
- [ ] WAL SQLite opened with the §1 pragmas; `events` table per `§7.1` envelope (incl. reserved `payload_hash`/`previous_event_hash`); `seq` canonical order; FTS5 over the redaction-safe audit projection; `user_version` migration runner.
- [ ] Files: `daemon/src/eventstore/` (NEW: schema, writer, migrations), `shared/contracts/event_envelope` (extended)
- [ ] Cross-doc invariant: NEW — Event envelope + actor/source/sensitivity/visibility enums (Appendix A, §7.1)
- [ ] Tests: happy (append + read back by seq); edge (out-of-order occurred_at vs seq; clock skew uses both timestamps); error (unknown event_version → degraded marker, no crash; corrupt payload → quarantine); integration (golden event log replays deterministically via injectable IdGen/Clock).

### 1.2 — Projection engine + the MVP projections + offsets
- [ ] Projection workers fold events → the MVP `proj_*` tables (ProjectActivity, Session, ApprovalQueue, Worktree, PullRequest, PlanProgress, ProjectGraph, AgentTeam, AuditTrail, UsageLedger) + `object_refs`; advance `projection_offsets` in the same txn; startup replay + full rebuild; degraded handling.
- [ ] Files: `daemon/src/projections/` (NEW)
- [ ] Cross-doc invariant: extended — projection rows mirror the §5.1 status enums
- [ ] Tests: happy (event → projection update); edge (rebuild-equivalence: full replay == incremental fold); error (projector exception → `state='degraded'`, skip, raw events intact); integration (single SessionStarted fans out to proj_session + proj_project_graph in one txn — demo step 7).

### 1.3 — Transactional outbox
- [ ] `outbox` table; event + projection + outbox commit in one txn; drainers for destinations (brain_mcp, github, linear, notifier, jsonl_mirror) with backoff + retryable/terminal classification.
- [ ] Files: `daemon/src/eventstore/outbox.rs` (NEW), `daemon/src/projections/` (extended)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (drain delivers once); edge (retryable 429/5xx backoff vs terminal 401/403 → dead after budget); error (crash between event-commit and delivery → redelivered, no double-apply); integration (outbox is the only path events reach external destinations).

### 1.4 — Lease locks + fencing tokens + single-instance
- [ ] `leases` table (resource_id, owner_id, monotonic fencing_token, heartbeat, expires_at); owner-guarded renew; expired-lease reclaim mints a new token; `pidlock` single-instance; lease reaper background task.
- [ ] Files: `daemon/src/locks/` (NEW)
- [ ] Cross-doc invariant: NEW — Lease (Appendix A, §5.1/§7.2)
- [ ] Tests: happy (acquire/renew/release); edge (paused holder's stale token rejected after reclaim — fencing, fake clock); error (PID reuse doesn't grant a false single-instance); integration (survives daemon restart — reclaim + new token).

### 1.5 — UDS GatewayPort transport + handshake (read/subscribe surface)
- [ ] UDS server with 4-byte length-prefixed JSON-RPC; `HelloFrame`→`HelloAck`|`VersionSkewError`; `getpeereid()` peer-auth (uid==daemon-uid); `MAX_FRAME_SIZE`; the read-only methods (`get_projection`, `subscribe`, `get_capabilities`); Terminal Channel frame-type multiplexing reserved. (Mutation methods land in Phase 2.)
- [ ] Files: `daemon/src/ipc/` (NEW), `shared/contracts/ipc` (NEW: JSON-RPC method/error schema)
- [ ] Cross-doc invariant: NEW — GatewayPort method surface (Appendix A, §6.1)
- [ ] Tests: happy (handshake + projection read); edge (version skew → structured error + disconnect); error (wrong-uid peer rejected; oversized frame rejected); integration (UI client reads a projection over UDS).

### 1.6 — First-run bootstrap + migrations backup/rollback + version-compat
- [ ] Cold-start ordering (§16): install launchd Background Item / detached spawn → pidlock → reclaim stale socket → create app-support dir → create+migrate DB → register desktop-host Device + LocalRunner → bind UDS. Pre-migration DB backup + restore-on-failure; `app_version↔user_version` floor (refuse-if-too-new); version-compat matrix.
- [ ] Files: `daemon/src/bootstrap.rs` (NEW), `daemon/src/eventstore/migrations.rs` (extended)
- [ ] Cross-doc invariant: NEW — Device, LocalRunner (Appendix A, §5.3); Version-compatibility matrix (§16)
- [ ] Tests: happy (clean first run creates DB + runs migrations); edge (stale socket reclaimed; second instance refused); error (bad migration → restore `.bak`, refuse-safely; downgraded binary sees newer DB → refuse); integration (daemon restart resumes against existing DB).

### Acceptance criteria (1)
- [ ] Daemon starts detached, single-instance, survives UI restart; event store is the sole writer.
- [ ] Projections rebuild deterministically; offsets crash-safe; `/preflight` clean.
- [ ] Lease fencing rejects stale holders; UDS peer-auth + handshake enforced.

---

## Phase 2 — Action Gateway (the mutation chokepoint)

**Goal:** The single, audited mutator. Typed `ActionRequest`/`ActionPlan` over the GatewayPort, the staged pipeline (normalize → resolve → policy/risk → preview/dry-run → approval → execute → audit), risk 0-4, **bundled ActionPlan + step-by-step approval (O-3)**, idempotency, fencing-guarded execution, stale-precondition re-check, fail-closed-on-audit-write, and the MVP executor adapters' interface.

**Spec anchors:** `ARCHITECTURE.md §6`, `§6.1`, `§6.2`, `§6.3`, `§5.1` (Approval/ActionRequest), `§15` (INV-SEC-1, fail-closed), `§17` (stale-precondition, fencing-conflict, daemon-crash-mid-action).

**Track / deps:** `daemon-core`. **Deps:** Phase 1, 0.5. **Parallel-with:** Phase 6 (ui); Phase 5 (git, against the frozen Gateway iface). Unblocks the `edges` track's intercept→intent path.

### 2.1 — Gateway pipeline + ActionRequest/ActionPlan model + mutation methods
- [ ] `submit_action`/`submit_action_plan`/`preview_action`/`approve`/`deny` GatewayPort methods; the staged pipeline; `action_requests`/`approvals` durable rows; the two split state machines (R-5); authoritative `ActionExecution*` events emitted ONLY by the Gateway.
- [ ] Files: `daemon/src/gateway/` (NEW: pipeline, request, approval), `daemon/src/ipc/` (extended)
- [ ] Cross-doc invariant: NEW — ActionRequest, ActionPlan/ActionPlanStep, Approval, ActionResult, ActorRef, ResourceRef, EvidenceRef, PolicyDecision (Appendix A, §6.2)
- [ ] Tests: happy (single action: submit→preview→approve→execute→succeeded events); edge (bundled plan: step-by-step approval, approve-all excludes critical/4); error (deny-with-reason; expired approval); integration (Brain/agent/UI all reach mutation only via submit_*).

### 2.2 — Policy engine + risk 0-4 + per-type action catalog
- [ ] Policy engine returning `{allow|require_approval|require_step_approval|deny|downgrade|needs_more_context}`; per-action risk resolution from the §6.3 ActionTypeCatalog (params schema, locked risk, required preview class, idempotency formula, executor); critical never in approve-all by default.
- [ ] Files: `daemon/src/policy/` (NEW), `daemon/src/gateway/catalog.rs` (NEW)
- [ ] Cross-doc invariant: NEW — ActionTypeCatalog (Appendix A, §6.3)
- [ ] Tests: happy (each MVP action type resolves to its locked risk + preview class); edge (risk-range action resolves by resource state, e.g. git.delete_worktree 3-4); error (workflow.command.invoke w/ null schema is approval-floored, never standing-granted); integration (policy decision drives approval requirement).

### 2.3 — Preview/dry-run + idempotency + executors interface
- [ ] `ActionExecutor` interface (validate/preview/execute/optional rollback); preview classes (command/diff/git/api/session/workflow/rollback) + `cannotPreviewReason`; idempotency-key derivation + dedup store; MVP executor stubs wired to later phases.
- [ ] Files: `daemon/src/gateway/executor.rs`, `daemon/src/gateway/preview.rs` (NEW)
- [ ] Cross-doc invariant: extended — ActionPreview (Appendix A, §6.2)
- [ ] Tests: happy (dry-run produces a preview); edge (preview-impossible escalates risk + sets cannotPreviewReason); error (duplicate idempotency key deduped); integration (executor only invoked post-approval).

### 2.4 — Fail-closed, stale-precondition re-check, fencing-conflict, crash reconciliation
- [ ] Fail-closed: audit-required (risk≥1) actions abort if the terminal event can't be written (one txn boundary); stale-precondition re-read after lock + before execute → fresh approval if the diff/resource changed; stale fencing token → `ActionFailed(fencing_conflict)` + hard-conflict surface; on restart reconcile orphaned `executing` actions via idempotency key.
- [ ] Files: `daemon/src/gateway/` (extended), `daemon/src/gateway/recovery.rs` (NEW)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (normal execute commits event + side effect atomically); edge (state changed between preview and execute → re-approval required); error (audit-write fails → mutation NOT applied; fencing conflict → ActionFailed, never auto-resolved); integration (daemon crash mid-action → restart reconciles to succeeded/failed/unknown).

### Acceptance criteria (2)
- [ ] **INV-SEC-1 architecture-invariant test passes** (no executor reachable except via the Gateway pipeline; every mutation has an event).
- [ ] Bundled ActionPlan + step approval works (O-3); critical excluded from approve-all.
- [ ] Fail-closed + fencing-conflict + stale-precondition behaviors tested deterministically (fault-injection + fake clock).

---

## Phase 3 — Harness adapters & embedded terminal

**Goal:** One `HarnessAdapter` contract over two lifecycle models — Claude Code (drive-mode per the 0.1 spike; `can_use_tool`/`PreToolUse` defense-in-depth, default-mode-only, no background subagents) and Codex (`app-server` stdio). Embedded PTY terminals (portable-pty → xterm.js) with backpressure. Status derived from structured streams, never from PTY scraping.

**Spec anchors:** `ARCHITECTURE.md §9.1`, `§5.1` (Session machine), `§6` (intercept→Gateway intent), `§7.2` (harness-derived SoT), `§9` (terminal/ADR-009), `§0.1` O-1/O-4, `§0.2` O-13.

**Track / deps:** `edges`. **Deps:** 0.1 (Claude mode), 0.3 (Codex schema), 0.5, Phase 2 (Gateway intercept→intent; can mock the Gateway iface to start). **Parallel-with:** daemon-core (Phase 4), ui (Phase 6).

### 3.1 — HarnessAdapter contract + normalized types + capabilities
- [ ] The trait `{launch, stream_status, intercept_mutation, read_transcript, telemetry_heartbeat, resume, capabilities}` + normalized types (NormalizedStatus[17], TelemetrySample{context_pct:Option, metric_quality}, MutationIntercept, TranscriptRef, ResumeResult) + `HarnessCapabilities` (10 fields); per-harness coverage matrix.
- [ ] Files: `daemon/src/harness/mod.rs` (NEW), `shared/contracts/harness` (NEW)
- [ ] Cross-doc invariant: NEW — HarnessAdapter trait + normalized types, HarnessCapabilities, Per-harness coverage matrix (Appendix A, §9.1)
- [ ] Tests: happy (both adapters satisfy the trait); edge (Codex `supportsContextMetadata=false` → context None); error (unsupported capability surfaced, not faked); integration (a shared conformance suite runs against recorded fixtures for both).

### 3.2 — Claude Code adapter (drive-mode per 0.1)
- [ ] Launch + status (SDK stream or PTY+hooks per spike) → Session 17-state machine; `can_use_tool`/`PreToolUse` → Gateway intent (default mode only; forbid background subagents); transcript JSONL (`~/.claude/projects/.../<id>.jsonl`) tail; telemetry merge (ResultMessage + statusLine `refreshInterval` + transcript); settable session_id (1:1).
- [ ] Files: `daemon/src/harness/claude/` (NEW)
- [ ] Cross-doc invariant: extended — NormalizedStatus
- [ ] Tests: happy (launch → status transitions → completed); edge (permission request → MutationIntercept → Gateway); error (no-TTY hang guarded; SDK schema pinned); integration (mutation interception covers direct tool calls; coverage-matrix gaps asserted).

### 3.3 — Codex adapter (app-server, O-1)
- [ ] `codex app-server --stdio` JSON-RPC (stable methods only); `thread/start{cwd}` → mint `sess_` ULID + `harness_session_map`; `thread/list?cwd=` re-association; push status → Session machine; host-routed approvals → Gateway; rollout dir pre-created 0700, files 0600; context-% = unknown.
- [ ] Files: `daemon/src/harness/codex/` (NEW)
- [ ] Cross-doc invariant: NEW — harness_session_map (Appendix A, §5.2)
- [ ] Tests: happy (thread start → status push → turn completed); edge (no settable id → cwd+thread_id mapping; -32001 overload retried); error (rollout never observable at 0644; legacy approval shape handled); integration (re-association after restart via thread/list).

### 3.4 — Embedded terminal (portable-pty → Tauri Channel → xterm.js)
- [ ] Daemon owns each PTY via portable-pty; headless VT state; Terminal Channel frames over UDS with explicit backpressure (pause/resume watermarks, ~30fps batch, XON/XOFF); scrollback serialize; **PTY is display-only, never a status source**.
- [ ] Files: `daemon/src/terminal/` (NEW), `ui/src/terminal/` (NEW: xterm.js host)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (PTY output streams to xterm.js); edge (high-output backpressure bounds buffer, no OOM/stall); error (PTY death → TerminalProcessExited event); integration (alt-screen TUI scrollback serialize/replay fidelity vs golden corpus).

### Acceptance criteria (3)
- [ ] Both harnesses launch + are supervised with reliable, non-scraped status + worktree/task association.
- [ ] Mutation interception routes to the Gateway; coverage-matrix gaps documented + compensated (O-13).
- [ ] Terminal backpressure + scrollback fidelity tested with FakePty/recorded corpus.

---

## Phase 4 — Session lifecycle, survival & failure-mode contract

**Goal:** Full session survival (O-2) and the consolidated failure-mode contract: UI-restart reconnect-live; daemon-restart resume-or-replay (`claude --resume`/`codex thread/resume` else scrollback replay + relaunch); supervised-child-death recovery; the background jobs; the §17 failure table behaviors and their UI-feeding events.

**Spec anchors:** `ARCHITECTURE.md §17`, `§10`, `§8` (recovery rows), `§0.1` O-2, `§5.1` (stale, Session terminals).

**Track / deps:** `daemon-core`. **Deps:** Phase 1, Phase 3 (sessions/adapters must exist to survive/recover). **Parallel-with:** ui (Phase 6), edges (Phase 5/7).

### 4.1 — Survival: UI-restart reconnect + daemon-restart resume-or-replay
- [ ] UI restart → reconnect live (daemon alive). Daemon restart → rebuild projections, re-read git2, ping Brain, reclaim leases, then per session: resume (harness `--resume`/`thread/resume`) else serialized-scrollback replay + relaunch; emit resumed-vs-replayed signal.
- [ ] Files: `daemon/src/harness/resume.rs` (NEW), `daemon/src/bootstrap.rs` (extended)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (UI reload reconnects live); edge (resume succeeds for one harness, replay+relaunch for the other); error (resume fails → relaunch + "restart session" affordance); integration (kill daemon mid-run via fault-injection → recover, no orphaned PTY/sidecar).

### 4.2 — Supervised-child-death recovery (daemon alive)
- [ ] Detect agent/PTY/app-server child exit (process-group reaper) → `SessionFailed`/`TerminalProcessExited`/`TerminalPTYFailed`; fail in-flight ActionRequest + release lease; Codex pipe-drop (reconnect) vs crash (relaunch) distinction.
- [ ] Files: `daemon/src/harness/supervisor.rs` (NEW)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (child exit → SessionFailed); edge (Codex pipe drop reconnects); error (in-flight action of dead session → ActionFailed + lease released); integration (no orphan processes after SIGKILL).

### 4.3 — Background jobs + failure-mode contract surfaces
- [ ] Heartbeat/status pollers (derive `stale` by age); WAL checkpointer; sidecar supervisor (ping/restart/backoff/timeout); failure-table events for: fail-closed/audit-integrity, integration auth/rate-limit, projection-degraded, network-offline, fencing-conflict — emitted so the UI (Phase 6) can render them.
- [ ] Files: `daemon/src/jobs/` (NEW)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (heartbeat keeps session live); edge (heartbeat age > threshold → stale, recomputed on rebuild not replayed); error (each §17 row emits its event); integration (offline → local cockpit operational, integration reads marked stale).

### Acceptance criteria (4)
- [ ] Resume-or-replay verified deterministically (injectable clock + FakeHarness + fault-injection).
- [ ] Every §17 failure row has an owner + a test + a UI-feeding event.

---

## Phase 5 — Projects, git/worktrees & Execution Profiles

**Goal:** Project registry + detection, the dual git backend (git2 reads / git-CLI mutations) with worktree lifecycle as Gateway actions, and Execution Profiles (keychain-backed, explicit, no silent account-hopping).

**Spec anchors:** `ARCHITECTURE.md §9` (git/integrations), `§7.2` (git/profile SoT + re-read), `§5.1` (Worktree derived, ExecutionProfile), `§6.3` (git.* actions), `§15` (profile binding, keychain).

**Track / deps:** `edges`. **Deps:** 0.3 (git2/octocrab), 0.5, Phase 2 (git mutations are Gateway actions; mock to start). **Parallel-with:** daemon-core, ui.

### 5.1 — Project registry + detection
- [ ] `projects`/`repositories` registry; `project.rescan` action detects git state, GitHub remote, workflow/cc-crew signals, Brain status, plan files; emits events → projections.
- [ ] Files: `daemon/src/git/detect.rs`, `daemon/src/workflow/detect.rs` (NEW), `daemon/src/eventstore` (registry tables)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (add project → detect git/remote/signals); edge (non-git path; basic project w/ no pack still works); error (missing path → degraded); integration (detection feeds proj_project_activity + graph).

### 5.2 — Dual git backend + worktree lifecycle (Gateway actions)
- [ ] git2 for status/diff/log/branch reads (projection refresh, live re-read before mutate); git CLI for `git.create_worktree`/`create_branch`/commit/checkout/merge as Gateway actions; derived worktree status precedence (§7.2); git watcher (hooks + git2).
- [ ] Files: `daemon/src/git/` (NEW: reads, cli_mutations, worktree, watcher)
- [ ] Cross-doc invariant: extended — Worktree status (Appendix A, §5.1)
- [ ] Tests: happy (create worktree + branch via Gateway); edge (relative-worktrees repo → CLI-read fallback per 0.3; precedence collapse locked/conflicts>dirty); error (dirty/conflict worktree mutation gated); integration (worktree status matches the user's terminal git).

### 5.3 — Execution Profiles (keychain, binding)
- [ ] `execution_profiles` registry (config vs runtime split); keychain (`keyring`) secrets w/ startup self-test; profile resolved at session.create approval time + recorded in SessionStarted; no Brain/silent switching; runtime status re-derived on restart.
- [ ] Files: `daemon/src/profiles/` (NEW), `daemon/src/policy/` (extended)
- [ ] Cross-doc invariant: extended — ExecutionProfile (Appendix A, §5.1)
- [ ] Tests: happy (create profile → launch session under it); edge (rate_limited/auth_expired runtime states; keychain self-test misconfig → 'misconfigured'); error (profile change requires new approval); integration (profile recorded in SessionStarted; never silently switched).

### Acceptance criteria (5)
- [ ] Worktree mutations are Gateway actions matching terminal git; reads are fast via git2.
- [ ] Execution Profiles explicit + auditable; secrets only in keychain.

---

## Phase 6 — Frontend shell & projection-driven UI

**Goal:** The Tauri shell as a projection-driven reattaching client implementing the canonical design system (O-5), the status-rendering binding, attention-first ordering, the core screens, the net-new daemon/survival/degraded surfaces, and the accessibility MUSTs.

**Spec anchors:** `ARCHITECTURE.md §11`, `§11.1`–`§11.7`, `§4` (UI=client), `§5.1` (status binding), `§7.2` (degraded SoT), `§14` (frontend tests).

**Track / deps:** `ui` (biggest independent arm — open it the moment 0.5 lands). **Deps:** 0.5 only (frozen enums/projections/GatewayPort) + the existing NexusOps-ui-kit. **Parallel-with:** ALL of daemon-core + edges — builds against a mock gateway-client + fixture projections; integrate with the real daemon per slice once that slice's daemon-side contract is live (team-protocol rule).

### 6.1 — App shell + design-system integration + daemon-connection/read-only mode
- [ ] Tauri shell (top bar, project switcher, sidebar, right drawer stack, activity dock, status bar); link `NexusOps-ui-kit` tokens + components; **daemon-connection indicator** (connected/reconnecting/disconnected) distinct from LocalRunner health; **global READ-ONLY degraded mode** disabling all intent-submitting controls + banner/Repair.
- [ ] Files: `ui/src/shell/` (NEW), `ui/src/lib/gateway-client.ts` (NEW: UDS client)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (shell renders from projections); edge (daemon disconnected → read-only mode disables approve/dispatch/commit); error (version-skew → "update required"); integration (UI never writes the DB; reconnect restores live state).

### 6.2 — Status rendering binding + attention-rank table
- [ ] StatusPill keys == §5.1 enum strings (all 9 machines, incl. 17 Session states + `changes_ready`); one canonical status→attention-rank table (no fall-through to idle) driving sidebar weight + queue membership + sort; four-channel "never color alone"; Approval vs ActionRequest as two surfaces.
- [ ] Files: `ui/src/status/` (extended from kit), `shared/contracts/attention.ts` (NEW)
- [ ] Cross-doc invariant: extended — the 9 status enums (Appendix A, §5.1)
- [ ] Tests: happy (one pill per state of each machine); edge (waiting_on_permission/conflict/stale enter "Needs my attention"); error (unknown status → visible, not idle); integration (attention ordering matches PRD §5.2).

### 6.3 — Core screens (Command Center, Project Home/Graph, Sessions, Terminal, Editor/Diff)
- [ ] Command Center triage (needs-attention/working/settled incl. a Changes-ready grouping); Project Graph **with list/table fallback**; Sessions list/board (dense table); Session Terminal (PTY + inline permission card); Code/Diff review with per-hunk actions.
- [ ] Files: `ui/src/views/{command,graph,sessions,terminal,code}/` (extended from kit)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (each screen renders fixtures); edge (graph list-fallback a11y equivalence — every node/edge appears as a row); error (empty/degraded states); integration (graph node → Inspector; session → terminal).

### 6.4 — Survival/failure UI surfaces + Usage dashboard + Settings + accessibility
- [ ] Recovery banner + per-session resumed/replayed indicator + "restart session"; fencing/hard-conflict + fail-closed/audit-integrity surfaces; Usage dashboard (exact/estimated/unavailable, **Codex context=unknown**, **credit-pool meter**); Settings (Integrations health, Execution Profiles, Security & policy, Notifications); **global `:focus-visible` ring**; **drag→non-drag equivalents**; reduced-motion.
- [ ] Files: `ui/src/views/{usage,settings}/`, `ui/src/recovery/`, `ui/src/a11y/` (NEW/extended)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (usage labels render; Codex shows "unknown"); edge (every drag has a button/menu path — TASK-5; focus ring on all controls); error (credit-pool near-exhaustion + hard-stop states); integration (recovery banner after restart; audit-integrity alert renders).

### Acceptance criteria (6)
- [ ] Projection-driven UI; read-only degraded mode works; status binding covers all 9 machines.
- [ ] Accessibility MUSTs pass (graph list fallback, focus ring, drag→non-drag) — frontend merge-gate tests green.

---

## Phase 7 — Integrations, PR Review Workspace & Task Inbox

**Goal:** GitHub (octocrab + gh-token bootstrap) and Linear (PKCE/key) read/link; the **full PR Review Workspace (O-6)**; Task Inbox + Dispatch; manual linking. GitHub-authoritative PR with re-fetch-before-merge.

**Spec anchors:** `ARCHITECTURE.md §9`, `§11.2` (PR Review), `§7.2` (PR SoT), `§6.3` (github/linear actions), `§17` (integration-failure contract), `§8` (intake/PR flows).

**Track / deps:** `edges`. **Deps:** 0.3, Phase 2 (Gateway), Phase 5 (git). **Parallel-with:** ui (Phase 6); the PR-Review-Workspace UI lands in the ui track and consumes this track's GitHub data.

### 7.1 — GitHub + Linear integration (read/link first)
- [ ] octocrab (issues/PRs/checks) + gh-token bootstrap else Device Flow; Linear SDK (PKCE/key, 24h refresh); `integration_connections` (keychain); reads cached as projections; integration-failure contract (§17).
- [ ] Files: `daemon/src/integrations/{github,linear}/` (NEW)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (list issues/PRs); edge (token expiry → re-auth card; rate-limit backoff); error (offline → stale projection + queued writes); integration (auth bootstrap order: gh token → device flow → keychain).

### 7.2 — Full PR Review Workspace (O-6)
- [ ] PR-review mode in Code/Diff: header (number/branch/base/author/session/task), checks, reviews/comments, mergeability, risk summary, Brain evidence, agent-session summary; Approve/Merge/Squash/Rebase/Request-changes/Ask-agent-to-fix-checks; **merge re-fetches GitHub (§7.2)** + routes through Gateway at risk≥3.
- [ ] Files: `ui/src/views/pr-review/` (NEW), `daemon/src/integrations/github/pr.rs` (extended)
- [ ] Cross-doc invariant: extended — PullRequest status (Appendix A, §5.1)
- [ ] Tests: happy (open PR → review panels render); edge (mergeable gated on fresh re-fetch); error (checks failing → "fix via agent" routes a Gateway action); integration (merge is a risk≥3 Gateway action with re-fetch).

### 7.3 — Task Inbox + Dispatch + manual linking
- [ ] Task Inbox (GitHub/Linear/plan-task chips); Dispatch dialog (target {new session, existing session, team}, harness, profile → Gateway); `plan.link_task`/`linear.link_issue` manual linking; the 4 canonical chains stay traceable.
- [ ] Files: `ui/src/views/tasks/` (extended from kit), `daemon/src/gateway/catalog.rs` (link actions)
- [ ] Cross-doc invariant: extended — `tasks` table kind-scoped (Appendix A, §5.1 R-8)
- [ ] Tests: happy (dispatch a task → session via Gateway); edge (drag + non-drag both dispatch); error (dispatch under offline → queued); integration (linking keeps ticket→session→worktree→PR traceable).

### Acceptance criteria (7)
- [ ] Full PR Review Workspace with re-fetch-before-merge; integrations degrade gracefully offline.
- [ ] Task intake dispatches via the Gateway; manual linking preserves the canonical chains.

---

## Phase 8 — Project Brain seam & drawer

**Goal:** The Brain stdio-MCP sidecar seam (daemon-owned lifecycle, propose-not-execute) and the drawer with full modes, multi-step action plans (O-3), evidence freshness/confidence, and graceful degradation. Brain internals are out of scope (sibling product).

**Spec anchors:** `ARCHITECTURE.md §13.1`, `§11.5`, `§6` (Brain→Gateway intents), `§7.1` (Brain outbox payload), `§5.1` (ProjectBrain status), `§0.1` O-3, `§13.1` (notarization spike/fallback).

**Track / deps:** `converge`. **Deps:** 0.2 (sidecar notarization/fallback), Phase 1 (events/outbox), Phase 2 (Gateway). **Parallel-with:** Phase 9; the Brain-drawer UI lands in the ui track.

### 8.1 — Brain MCP sidecar lifecycle + notification→event adapter
- [ ] Spawn/own the Brain stdio MCP sidecar (`rmcp`); ping/restart/backoff/process-group-kill; MCP-notification→event mapping (BrainEventMapping); Brain outbox payload (redacted envelope, object_refs by shared ID); degrade gracefully when absent/stale (`brain_status_reported_at`).
- [ ] Files: `daemon/src/brainclient/` (NEW)
- [ ] Cross-doc invariant: NEW — BrainEventMapping (Appendix A, §13.1)
- [ ] Tests: happy (sidecar starts, notifications → events); edge (Brain stale → degraded banner, platform unaffected); error (in-flight call timeout → fail brain.* action, respawn); integration (Brain reaches mutations only via Gateway — INV-SEC-1).

### 8.2 — Brain drawer (modes, evidence, multi-step action plans)
- [ ] Drawer with full modes (Ask/Plan/Review/Decisions/Memory/Actions) each functional; scope chips that constrain retrieval; header (live index status + grounded-at/staleness + privacy/transport); per-answer confidence/verification; EvidenceChip 5-state freshness + confidence; **"Run via Gateway" submits the exact reviewed plan.steps** → multi-step Gateway modal; `actor_type=project_brain` stamped.
- [ ] Files: `ui/src/views/brain/` (extended from kit), `ui/src/views/gateway-modal/` (extended for ActionPlan)
- [ ] Cross-doc invariant: extended — EvidenceChip freshness (Appendix A, §11.7)
- [ ] Tests: happy (ask → grounded answer + evidence + plan); edge (each mode renders its own view; unverified-claim treatment); error (Brain unreachable → drawer degraded, not blocking); integration (Brain plan → step-by-step Gateway approval → execute — demo steps 12-16).

### Acceptance criteria (8)
- [ ] Brain proposes multi-step action plans executed only via the Gateway; never executes directly.
- [ ] Drawer modes functional; evidence freshness/confidence rendered; degrades gracefully.

---

## Phase 9 — Workflow Packs, cc-crew detection & Plan view

**Goal:** The generic Workflow Pack abstraction (detection-advisory → readiness), cc-crew as the first pack (plan + architecture-anchor parsers), the command registry, the Plan view, and the AgentTeam object/projection (modeling only; orchestration deferred). Trust-gated pack script execution.

**Spec anchors:** `ARCHITECTURE.md §13.2`, `§11.2` (Plan/Packs screens), `§5.1` (WorkflowInstance, AgentTeam R-6), `§15` (pack trust gate), `§19.2` (orchestration deferred).

**Track / deps:** `converge`. **Deps:** Phase 1, Phase 2; the Plan-view + Workflow-Packs UI depends on Phase 6 (ui). **Parallel-with:** Phase 8.

### 9.1 — Workflow Pack detection + readiness + command registry
- [ ] Pack/Instance model (pack≠instance); detection-advisory → explicit readiness checks; `command_registry`; workflow-owned manifests read-only (re-hash on scan → drift); trust-gated script execution (untrusted → Gateway risk≥3).
- [ ] Files: `daemon/src/workflow/` (extended)
- [ ] Cross-doc invariant: extended — WorkflowInstance status (Appendix A, §5.1)
- [ ] Tests: happy (detect cc-crew → readiness states); edge (template pack not-ready until personalized; manifest drift detected); error (untrusted pack script gated behind approval); integration (basic project w/ no pack fully works).

### 9.2 — cc-crew parsers (plan + architecture anchors) + Plan view
- [ ] MVP_TASKS.md/ARCHITECTURE.md §N anchor parsers → ImplementationPlan + PlanTask; Plan view (Phase→Track→PlanTask, anchors, AC, dispatch); plan-task linking stored in platform metadata (no write-back in MVP).
- [ ] Files: `daemon/src/workflow/parsers/` (NEW), `ui/src/views/plan/` (extended from kit)
- [ ] Cross-doc invariant: extended — PlanTask kind-scoped (Appendix A, §5.1 R-8)
- [ ] Tests: happy (parse a plan → PlanTasks with anchors); edge (ambiguous structure → raw-preview fallback); error (missing anchors degrade w/ warnings); integration (dispatch a plan task → session linked to the plan task — demo step 5).

### 9.3 — AgentTeam object + projection (modeling only)
- [ ] `agent_teams` registry + `proj_agent_team` projection over the 9-state machine (R-6); team membership/role on sessions; `active_teams` counter; graph team/lead/orchestrator/worker nodes. (Full `/team-start` orchestration is P1, §19.2.)
- [ ] Files: `daemon/src/eventstore` (agent_teams table), `daemon/src/projections/agent_team.rs` (NEW), `ui/src/views/team/` (extended from kit)
- [ ] Cross-doc invariant: extended — AgentTeam status (Appendix A, §5.1 R-6)
- [ ] Tests: happy (team object created + status renders); edge (worker session shows team + role); error (orchestration actions are P1-gated/absent); integration (active_teams counter derives from team status).

### Acceptance criteria (9)
- [ ] Detection-advisory + readiness; cc-crew parsers feed the Plan view; basic projects unaffected.
- [ ] AgentTeam modeled (object/projection/graph nodes); orchestration explicitly deferred to P1.

---

## Phase 10 — Demo integration, deployment, signing & consent

**Goal:** The First-Launch Setup Wizard + consent/TCC map, native notifications, signed/notarized packaging (incl. the Python sidecar), and the PRD §25 demo wired end-to-end as the release gate.

**Spec anchors:** `ARCHITECTURE.md §16`, `§11.4` (setup wizard, consent, notifications), `§10` (notifier), `§19.1` (demo), `§14` (demo e2e), `§13.1` (sidecar notarization, 0.2 spike).

**Track / deps:** `integration` (all hands — final gate, NOT parallelizable). **Deps:** ~everything (esp. 0.2 notarization, Phase 2 Gateway, Phase 6 ui, Phase 8 Brain). The Setup Wizard + consent map can be drafted in the ui track earlier; the demo e2e + signing are the convergence gate.

### 10.1 — First-Launch Setup Wizard + consent/TCC map
- [ ] Stepper (welcome, runtime check, Claude/Codex detection, Execution Profiles, Brain, git/GitHub/Linear, Workflow Pack library, security/approval policy, finish/add-project) as idempotent/reversible/skippable Gateway intents; consent card + denied-degraded + repair for keychain ACL, notification permission, Full Disk Access, launchd Background Item, AppleEvents.
- [ ] Files: `ui/src/views/setup-wizard/` (NEW), `daemon/src/bootstrap.rs` (extended)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (first run → all-passed → add project); edge (tool missing → installable/manual states; consent denied → degraded + repair); error (re-run is idempotent); integration (wizard completes → Command Center).

### 10.2 — Native notifications (notifier) + settings
- [ ] `notifier` module → macOS UserNotifications for SessionWaitingOnHumanInput/Permission, CheckFailed, SessionCompleted; per-type toggles; permission state + request; redacted lock-screen previews; in-app bell mirror.
- [ ] Files: `daemon/src/notifier/` (NEW), `ui/src/views/settings/notifications.tsx` (NEW)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (waiting session → notification); edge (permission not granted → degraded, in-app only); error (previews redacted — no secrets); integration (notifier consumes outbox events).

### 10.3 — Packaging, signing & notarization (incl. Python sidecar, per 0.2)
- [ ] Tauri bundle + deep-sign order (inner libs → sidecar → daemon → .app) + notarytool + staple; entitlements; `spctl`/`codesign` CI gate; updater; app-update-while-running (prepare_for_update → drain → relaunch).
- [ ] Files: `ci/sign-notarize.sh`, `src-tauri/entitlements.plist`, `tauri.conf.json` (NEW)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (signed+notarized build passes spctl); edge (sidecar deep-signed; if #11992 blocks → loopback-HTTP Brain fallback per 0.2); error (keychain ACL no-prompt with Developer ID); integration (app-update drains daemon + relaunches).

### 10.4 — PRD §25 demo end-to-end (release gate)
- [ ] Wire + verify the 17-step demo: add project → detect → execution profile → plan task → worktree + Claude session → sidebar/graph → permission → Human Input Queue → approve via Gateway → edit → review diff → ask Brain (evidence) → Brain proposes PR action plan → approve → PR created + task/session/worktree/PR linked + events. Demo preconditions checked (reachable Brain + non-exhausted Claude profile).
- [ ] Files: `tests/e2e/prd-25-demo.spec.ts` (NEW)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (full 17-step path green via Tauri driver + FakeHarness or live); edge (demo-precondition check fails gracefully); error (each step's failure surfaces correctly); integration (the whole spine exercised end-to-end).

### Acceptance criteria (10)
- [ ] Signed/notarized app installs + runs first-run bootstrap + consent map.
- [ ] **PRD §25 demo passes end-to-end** — the project-level "done" condition.

---

## Trims / Nice-to-Haves Catalog

Deferred items with come-back guidance. (Seeded from `ARCHITECTURE.md §19.2`; expanded as scope cuts surface.)

- **Agent Team orchestration / `/team-start`** `[P1]` — modeling only in MVP (Phase 9.3); full orchestration deferred (§19.2). Belongs in `daemon/src/workflow/` + `ui/src/views/team/`.
- **cc-crew personalization/upgrade UI + TDD slice tracker** `[P1]` (§19.2) — MVP Gateway still renders the multi-step personalization plan.
- **PR checks + agent-fix flow** `[P1]`; **one-way/bidirectional Linear sync** `[P1/P2]`; **Brain policy-automation** `[P2]`; **conflict resolver** `[P1]`; **usage budgets** `[P1]`.
- **iOS companion** `[P2]`; **Windows/Linux** `[P2]`; **multi-repo projects** `[P2]`; **hash-chain tamper-evidence** `[P2]`; **agent egress isolation** `[P2]`; **UI projection-rebuild** `[P1]`; **uninstall/reset UI** `[P1]`.

---

## Decisions tabled

Open scope/design questions awaiting resolution.

- **[RESOLVED 2026-06-07 — cat-4, user-locked] Cross-language contract source-of-truth = Option A.** Rust `shared` crate = native authority (newtypes, serde-closed enums); `schemars` → **JSON Schema as a first-class, versioned, CI-diff-gated published artifact** (`shared/contracts/schema/`, `CONTRACT_VERSION`); TS Zod + Python Pydantic generated from it (drift-caught); reject-unknown end-to-end. Documented at **`ARCHITECTURE.md §5.0`** (direct anchored edit, owner-locked). This is the mechanism for ALL contract surfaces, not just 0.5. `(origin: 2026-06-07 P0.5)`
- **[cat-4 / load-bearing — PENDING ≥6/15 DRAIN] SDK-vs-PTY primary driver for Claude (O-4 / ADR-006).** The 2026-06-15 Anthropic policy gives SDK/`-p` a separate **capped** Agent-SDK credit pool that **hard-stops with no fallback**; the interactive terminal is exempt (`RESEARCH.md:65` [VERIFIED]). This **may invert ADR-006**'s Option-C-Hybrid lean (SDK-driven primary). **Not decided agent-only.** Phase-0 0.1 LANDED the measurable half: **decision criterion + both branches recorded** (`docs/spikes/OQ-HARN-SPIKE-7.md §3`); **#27203 confirmed** present on CC 2.1.168 (bg subagents stay forbidden — §9.1 unchanged). **Still open:** the deciding **drain measurement = HITL checklist** (`§5`), user runs ≥ 6/15**. Orchestrator carries the resolved call back to the lead/user with the drain data. **Blocks** freezing any 0.5 supervision-touching contract surface. `(origin: 2026-06-07 P0.1)`
- **[tracked — Phase 10, not blocking] D14 demo-viability contradiction.** Brain-optional (design) vs demo-mandatory (PRD §25) vs SDK-can-halt (credit-pool) are in tension for the end-to-end demo. Carry as a known Phase-10 concern; resolve before the integration/deploy gate. `(origin: 2026-06-07, D14 audit)`
- _(Phase 0 also populated §18 perf numbers (0.4 → written into `ARCHITECTURE.md §18`). §13.1 Brain transport still resolves later.)_

---

## Architecture-doc corrections (applied as direct edits)

> **`/arch-finalize` is user-invoked, on-demand — NOT an auto-recurring batch** (user, 2026-06-07). It produced the binding `ARCHITECTURE.md`; there is no scheduled "next run" to queue work into. Locked-decision records + factual corrections + numbering fixes land as **direct atomic anchored edits** (Step-9 arch-doc-note hot-routing), committed with the round. A full `/arch-finalize` **re-validation** is a deliberate user-invoked gate, natural at **Phase-0 exit before Phase 1**.

Applied this round (2026-06-07), committed atomically with the round commit:
- [x] **`ARCHITECTURE.md §5.0` — Contract source-of-truth & propagation** written (Option A, owner-locked). The contract *format* is now itself contracted. `(P0.5)`
- [x] **`ARCHITECTURE.md §9` — libgit2 relative-worktree read-gap CLOSED.** Empirical (0.3 / `OQ-INT-SPIKE-6.md`): libgit2 ≥ 1.9.4 (git2 0.21) **CAN** fully read `extensions.relativeWorktrees` repos; the §9 aside corrected. Dual-git posture UNCHANGED (CLI for ALL mutations + the sparse-checkout fallback retained). `(P0.3)`
- [x] **OQ-HARN-SPIKE numbering reconciled.** Canonical = **`-7`** (used across the finalized artifacts: gap-audits/D06, ui-review/E-netnew, MVP_TASKS, the spike file). `OPEN_QUESTIONS.md` `-2` (legacy /arch-draft) cross-ref'd to `-7`. `(P0.1)`
- [x] **Appendix A + `daemon/CLAUDE.md` cross-doc rows + LESSONS §2** — the 0.5 frozen contracts recorded (cross-doc invariant routing). `(P0.5)`

**Upstream PLANNING-doc reconciles for the user-invoked Phase-0-exit `/arch-finalize`** (the binding `ARCHITECTURE.md` is already correct; these are /arch-draft drafts the re-validation should sweep — do NOT hot-edit planning artifacts):
- [ ] `docs/planning/DECISIONS.md` **ADR-007** still carries the stale "libgit2 can't do relativeworktrees (fix unreleased)" premise → reconcile to the §9 correction. `(P0.3)`
- [ ] `docs/planning/DATA_MODEL.md` **§4** (rough draft) is superseded by §5.1 (LOCKED): `ready_for_team_mode`→`ready_for_team_run`; Approval predates the R-5 split; lacks ActionRequest/AgentTeam machines. `(P0.5)`
- [ ] `DATA_MODEL.md` **EM §7** actor enum `remote_device` → unify on **`remote_client`** (§7.1/R-2; DATA_MODEL §6 line 576 already recommends this). `(P0.5)`
- [ ] `DATA_MODEL.md` **§6.4** EventProjection prefix `prj_` → **`eprj_`** (frozen value, de-collided from `proj_`). `(P0.5)`
- [ ] **Ratify `§5.0`** (it's `[LOCKED — owner]`, recorded as a direct edit; the re-validation should scrutinize it like any anchor). `(P0.5)`
- [ ] **`ARCHITECTURE.md §5.1` header/table count mismatch** — prose says "**Nine** machines (the 8 of the draft + ActionRequest)" but the table lists **10** (the 10th appears to be **AgentTeam**, R-6). Reconcile the header to 10 + correct the parenthetical (not direct-fixed — header framing inferred, left for the re-validation to confirm authoritatively). `(P0.5)`

---

## Log

Append-only, date-stamped, the orchestrator's framing of each round.

### 2026-06-07 — Phase 0: spikes + 0.5 contract freeze (the serial neck)

- **Completed:** all 4 Phase-0 spikes (0.1–0.4) resolved with recorded decisions (`docs/spikes/*.md`); **0.5 shared-contract freeze landed (06f9576)** — the serial neck. Single-writer holds (§18 written MEASURED, 12–19× headroom); git2 reads relative-worktrees (§9 corrected); #27203 confirmed (bg subagents stay forbidden); contracts frozen Option-A in `shared/` (9 status machines, 22 IDs + prefix map, actor enum, 4 desktop objects).
- **Decisions made:** **contract SoT = Option A** (cat-4, user-locked) — Rust authority → first-class versioned CI-diff-gated JSON Schema → generated Zod/Pydantic (recorded `§5.0`). 10 new ID prefixes defined + ratified (orchestrator TWEAK de-collided `art_`→`artf_`, `prj_`→`eprj_`). §18 budgets MEASURED + committed (+ §14 CI guards).
- **Scope shifts:** **ExecutionProfile runtime-state enum held → 0.5b** (excluded from the freeze pending cat-4). HITL items staged (notarization, credit-pool drain ≥6/15, codex schema capture) — none block Phase 1.
- **New blockers / open questions:** cat-4 **SDK-vs-PTY** pending the ≥6/15 credit-pool drain; **D14 demo-viability** contradiction tracked for Phase 10.
- **Convention fixes:** `cargo fmt --check` added to the daemon preflight gate (Step-8 was clippy-only → 06f9576 needed fmt follow-up 407be7c). Banked LESSONS §1 (broken cargo shims) + §2 (wire-value-is-contract / §5.0 SoT pattern). Architecture corrections direct-edited (§5.0, §9, §18, Appendix A); planning-doc reconciles queued for the Phase-0-exit `/arch-finalize`.
- **Next session target:** **user-invoked Phase-0-exit `/arch-finalize` re-validation** (sweep the planning-doc reconciles) → then fan-out: **Phase 1 / 1.1** (event store, daemon-core) ∥ **ui track**. **HELD on 1.1** until the re-validation completes + any moved contracts are reconciled into `shared/` (a 0.5b if frozen enums/IDs/§5.0 shifted).
- **Reference:** implementer session doc `001-2026-06-07-phase0-spikes-and-contract-freeze.md`; commits 06f9576 (0.5) + 407be7c/94e4894/2bf198d/4f572dc (close-out).

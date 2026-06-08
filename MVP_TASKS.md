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

- **⏸️ TEAM PAUSED 2026-06-08** (`/team-end`) — handoff `docs/team-handoffs/001-2026-06-08-phase1-eod.md` · last round-seal `41db7f0` (origin/main) · resume: 1.6a-L3 (ruling B) → 1.6c (ruling C) → 1.6d → 1.7 (briefs 027/028/029 ready, rulings baked). Next `/team-start daemon`.

**PHASE 0 — DETERMINISTIC WORK COMPLETE (2026-06-07).** All 4 spikes landed (`docs/spikes/*.md`) + the **0.5 contract freeze landed (06f9576)** — the serial neck. Single-writer holds (0.4 → §18 written MEASURED); git2 reads relative-worktrees (0.3 → §9 corrected); #27203 confirmed, bg subagents forbidden (0.1); shared/ contracts frozen Option-A (0.5, §5.0). Toolchain RESOLVED (Lesson §1).

**Phase-0-exit `/arch-finalize` re-validation COMPLETE + committed (2026-06-07, 1efff4b).** No frozen `shared/` contract moved (0 release-blockers; no 0.5b forced; ui track ungated). Drift swept forward to `ARCHITECTURE.md` (§5.1→Ten, §5.0 ratified, ADR-004 getpeereid/length-prefix, ADR-007 libgit2, EM §7 remote_client, DATA_MODEL/SOM/OQ/UI sweep). Audit: `docs/gap-audits/R2-phase0-exit-revalidation.json`. Hold released.

**PHASE 1 — IN PROGRESS (daemon-core fan-out).** **1.1 event store ✅ LANDED** (df753aa+b998f20+61089ea; 0.7.0; 31 tests; §15 fail-closed redaction gate; entropy → blocking 1.7). **1.2 projection engine ✅ LANDED** (c8c6c72+a1dc482+e66c659; in-band fold in the event-commit txn + offsets + `object_refs` + catch-up-replay/full-rebuild/degraded-skip + the 4 Phase-1-feedable projectors; `SessionStarted`/CONTRACT_VERSION 0.8.0; 6 projector bodies re-homed Option-A; brief 004). **1.3 transactional outbox ✅ LANDED** (343cc09+707845a; in-txn write + §15 sync-sink gate + at-least-once `drain_once` [backoff/retryable/terminal/bounded dead-letter] + crash-recovery + `jsonl_mirror`; 4 destination adapters re-homed; brief 005). **1.4 lease locks + fencing + pidlock + reaper ✅ LANDED** (b3f7612+442149a+0347ac6; `leases` per `(resource_id,lease_kind)` + persisted monotonic fencing high-water + Option-B live-lease `validate_held` authority + std-file-lock pidlock + `reap_once`; daemon-internal/no CONTRACT_VERSION bump [stays 0.8.0]; brief 006; LESSON §6; acquire/validate→P2 gateway, pidlock+reaper-spawn→1.6). **67 tests; security PASS §15/§17/§7.2/rule-#6.**

**1.5 LANDED + ROUND SEALED → 1.6 FRESH TEAM (2026-06-08).** `track/ui` merged to main (`48d9931`); **1.5 UDS GatewayPort transport LANDED** (4-commit `ae27f1d`/`fb941e8`/`c1c54e5`/`6690e41`; 15 ipc tests; workspace 83 green; rule #7 [getpeereid] + single-writer PASS; CONTRACT_VERSION **0.11.0** [+`protocol_error`, lead-ratified away-authority]; §6.1 read surface [get_projection/subscribe/get_capabilities] + the full §6.4 wire contract; `ServerFrame` frame-mux; brief 007; LESSONS §7). **Round sealed at the clean 1.5 boundary** (away-authority context cycle — impl 70% WARN mid-L4) + pushed. **1.6 is next but ON HOLD** — the lead spins down both teammates + spawns a **fresh team** for 1.6 (full budget; **1.6 is large** — first-run bootstrap + the runtime-task spawns [outbox drainer / reaper / IPC accept-loop / subscribe delta-source] + the §17 corrupt-row degradable replay + migrations backup/rollback + version-compat; **consider splitting 1.6a cold-start/bootstrap + 1.6b runtime-spawns + §17**). **Open Finding → 1.6:** §17 corrupt-row-on-`open` (`catch_up_replay` strict-reconstruct → daemon-won't-start; human-ratified defer Option A; 1.6 decides skip-vs-refuse + audit-integrity). The **ui track** consumes the 1.5 IPC contract @ 0.11.0 at its resume (cross-track-timed by the user; Carry-forward `ui↔daemon-1.5` 🟢 — part (a) `SUPPORTED_PROTOCOL_RANGE {1,1}` confirmed).

**1.6 SPLIT 3-WAY + IN PROGRESS (2026-06-08, fresh team).** Split (orch-scoped, lead-nodded): **1.6a = cold-start bootstrap/migrations/version-compat** (brief 027; task #11 → daemon-implementer; 3 layers — **L1+L2 ✅ LANDED `48b7a2c`/`f1de088`** [92 workspace tests green; security-reviewer PASS (pidlock-before-open ordering, §15/§16/forbidden-#3); both reviewers; cq-highs fixed in-slice; daemon-internal — no CONTRACT_VERSION bump]: L1 eventstore typed-error cleanups [`user_version()→Result`, typed `RestoreFailed` w/ preserved migration-error context; 1.1 L2/L3 carry-forward] · L2 `bootstrap.rs` cold_start orchestration [pidlock→app-support dir→`EventStore::open`→`DaemonContext`+`DaemonVersionInfo`]; LESSON §8 · L3 Device/LocalRunner registration [**UNBLOCKED — USER-RULED Option B** 2026-06-08: system-event append `actor_type=System` + reserved system-workspace sentinel + `DeviceId`/`LocalRunnerId` newtypes off `DesktopObjectKind`; brief 027 updated; CONTRACT_VERSION bump; security-reviewer]) · **1.6b = runtime-task spawns** (outbox drainer + bounded pass · lease reaper · IPC `bind()`/accept-loop · live subscribe delta-source [writer↔IPC seam] · platform cfg-guard · concurrency cap · per-session WAL conn · `subscription_id` · the production `main.rs` entry; consumes 1.6a-L2's `DaemonContext`; brief pending) · **1.6c = §17 degradable catch-up replay** (safety-critical, atomized; **UNBLOCKED — USER-RULED Option C** 2026-06-08; brief 029; the `DegradableEvent`/`read_all_degradable` primitives already exist [1.1/1.2] — 1.6c WIRES them into `replay`/`open` + adds the quarantine record + the audit-integrity event [CONTRACT_VERSION bump] + the unredacted-replay-skip + the `-1`-seq cleanup; security-reviewer). **Scope finding:** `EventStore::open` + `migrations.rs` ALREADY do migrate + backup/rollback + DB-version floor (landed 1.1) — so 1.6a's new work is the *orchestration* + registration + version-info + the two typed-error cleanups, not the migration mechanism. **Two cat-1 safety calls USER-RULED 2026-06-08:** §17 corrupt-row = **Option C** (quarantine + audit-integrity event + continue; gates 1.6c) · registration = **Option B** (system-event append; gates 1.6a-L3) — both resolved in Decisions-tabled. **Build progress:** 1.6a L1+L2 ✅ + **1.6b ✅ LANDED** (`26898a8`/`9448035`/`8d06ed7`/`f9c31b1`/`39f756e`; 105 workspace green; security PASS forbidden-#3 [sole-writer + publish-after-commit + reader-never-blocks-writer] + rule-#7 [getpeereid through the accept path]; **no contract bump — stays 0.11.0**; write-actor [dedicated blocking thread + mpsc] + `main.rs` `#[tokio::main]` + drainer/reaper interval loops + UDS bind/accept-loop [cap + unlink-before-bind] + broadcast subscribe-**SOURCE** [publish-after-commit]; LESSON §9; the deferral chain `main.rs`→cold_start/drain_once/reap_leases/serve_connection closes). **Carved-out:** the socket subscribe-**SERVE** push (brief 028 test 11) → **1.6d** (task #13; daemon serve-layer wiring; closes the ui live-subscribe path). **Ctx-check ACTION 79% (the LEAD's context) → LEAD RULED CLOSE-OUT at the 1.6b clean boundary** (2026-06-08; matches the user's call). 1.6a-L3 was opened (Step-2.5 design only, no commit) → **HELD + cleanly paused** (design blessed by the orch + recorded in session doc 006 for instant resumption; L3 RED reverted for a clean tree). **Round sealing:** impl `/session-end` (doc 006) → orch `/orchestrate-end` + **push to origin** (banks 1.6a-L1/L2 + 1.6b commits + Decisions-tabled C/B + the de-dup'd Carry-forward + all hot-routing) → lead `/team-end`. **NEXT session's first slices (all unblocked; briefs ready 027/028/029; rulings baked):** **1.6a-L3** (Option B; design blessed: `desktop_minted_id!` macro + `SYSTEM_WORKSPACE_ID` sentinel + object_refs-from-payload) → **1.6c** (Option C) → **1.6d** (subscribe-serve #13) → **1.7** (entropy redactor, brief TBA). **1.6b deferrals → Carry-forward** (triage at this `/orchestrate-end`): `subscription_id` on the subscribe ack (when >1 subscription/connection); macOS `getpeereid` cfg-guard (when Linux CI exists); the `"SessionStarted"` event-type string-literal dup across `runtime/writer.rs` + the 1.2 projectors → a shared const (desync risk; spans 1.2). **Brief numbering:** lead away-ruling — new briefs both tracks `027+` (global-unique NNN; no renames of the 29 committed); daemon 1.6a = **027**.

**Still open (non-blocking for Phase 1):** HITL — notarization run (0.2, Apple creds), credit-pool drain ≥6/15 (0.1) → then cat-4 SDK-vs-PTY + **0.5b** (re-freeze ExecutionProfile).

**UI TRACK (`track/ui`) — round 6 SEALED (2026-06-08): ✅ PHASE 6 COMPLETE + active-project selection + the FULL daemon-independent polish set → ⏸️ UI TRACK PAUSED at its parallel-track limit.** ✅ **6.1** · ✅ **6.2** · ⏳ **6.3 PARTIAL** — 6.3a–c landed (+ sessions-table filtering `9a7945f`); **6.3d/6.3e ⏸️ PARKED on daemon-1.5** (Decision C) · ✅ **6.4 DONE** (6.4a–e + 6.4d-2 §17 safety + checking-banner; + round-6 closures: TopBar history-nav `25ef3ed`, tablist roving + roving-aware §9 audit `128714e`, sidebar resume-mode `64ffa42`) · ✅ **6.5 Graphite Arc theme pass DONE + LEAD-ACCEPTED visual gate** (FINAL aesthetic sign-off flagged for the user) · ✅ **P7.3 active-project selection (`86727ec`) + dropdown-popover widget (`0af44a5`+`9f2c91f`)** — daemon-independent (Lesson §13). **193 tests green** (`012ee5a..9f2c91f`; round 6 = 142→193, +51), tsc/oxlint/prettier clean; built against frozen `shared/` 0.5.0 + `MockGatewayPort` + fixtures + `NexusOps-ui-kit`. Lessons banked/extended: `ui/LESSONS.md §8`/`§9`/`§11`/`§12`/`§13`; new shared `ui/src/a11y/roving.ts` (orientation-aware). **⏸️ UI TRACK PAUSED** — Phase 6 (logic + visual) + active-project + the entire daemon-independent polish tail are DONE; **everything remaining is daemon/integration-gated** (6.3d/e + intent seam on daemon mutation/Terminal; Phase 7/8 PR-Review/Task-Inbox/Brain/Gateway-modal; provisional→generated reconcile; ExecutionProfile 0.5b) — **resumes on daemon contract-freeze OR user direction.** Round-6 terminal commit: _(this `/orchestrate-end` commit)_; pushed to `origin track/ui`. Session docs: `ui-001`–`ui-006` (r6 `e1225a4`). Briefs `022`–`026`.

---

## Carry-forward to upcoming briefs

Items the orchestrator MUST fold into upcoming slice briefs. **Triaged at every `/orchestrate-end`** — NOT append-only. New entries carry `(origin: YYYY-MM-DD <slice-id>)`.

_(Re-triaged at the 2026-06-08 **`track/ui` → main merge** re-orient (daemon-1.5 resume). The merge unioned both tracks' Carry-forwards (12 items). **De-duped:** dropped the daemon "ui track generates Zod" spread (DONE in P6.1a — `ui/src/contracts/generated.ts` + `gen-contracts.mjs`) + the daemon duplicate of "§5.0 contract gates into CI" (kept the ui **EXPANDED superset** above, which adds the ui-side TS drift test + `CONTRACT_VERSION`===`x-contract-version` pin + the corepack note). **Result: 10 items**, all cross-track / future-phase **SPREADs with valid `last-consumer-slice` markers** (none KEEP-active; none >3-slices-old-without-a-consumer) — the ~7 cap is exceeded only by **parked, consumer-gated cross-track integration points**, not active working-set. **Brief 007 (1.5) folds in** the `ui ↔ daemon-1.5 integration` spread (reconcile `SUPPORTED_PROTOCOL_RANGE {min:1,max:1}` + build the real `UdsGatewayPort` the ui's `MockGatewayPort` swaps to) as the consumer contract to match; the item STAYS in Carry-forward until the ui-side swap (cross-track-timed by the user). Triage: **2 deleted / 0 inlined / 0 deferred / 10 spread / 0 kept-active.** **Doc-numbering (parallel-track reality, corrected 2026-06-08 after the merge):** the two tracks numbered briefs **independently from 003** — daemon `003–007` (P1.x) + ui `003–026` (P6.x/P7.x) — so the NNN-prefixes **003–007 overlap**, BUT every **full filename `NNN-<task-id>-<topic>` is unique** (the `P1` vs `P6/P7` task-id disambiguates). NNN is therefore **per-track** when parallel tracks run (NOT global); each track continues its own sequence (daemon next = `008-P1-6`; ui next = `027`). The overlap is **benign — no renames needed** (forcing global-unique NNN on 29 committed briefs is busywork; the filenames already disambiguate). Session docs already namespaced (daemon `NNN`, ui `ui-NNN`). *(Flagged to the lead — the earlier "ui = 022+" was incomplete; if a global-unique scheme is wanted, new briefs both-tracks go `027+`, decided lead/user-side.)* )_

**1.1 L1 arch-note refinements (deferred; non-blocking — SPREAD, consumer-marked at 2026-06-08 triage):**
- **`Timestamp` newtype (`format:date-time`)** across the envelope + future event types — `occurred_at`/`recorded_at` are currently RFC3339 `String` (valid, but no schema `format`). Production refinement; apply when the event-type registry accretes. `last-consumer-slice: 2.1` (first new event-type payload schemas beyond `SessionStarted`) `(origin: 2026-06-07 P1.1)`
- **`seq` schema `minimum:1`** annotation (the L2 writer enforces monotonic ≥1; document it in the schema). Minor. `last-consumer-slice: 2.1` `(origin: 2026-06-07 P1.1)`
- **`workflow_run_id` reconcile** — ∉ the LOCKED 22 IDs; `String` placeholder in the envelope. Resolve: ≡ `workflow_instance_id` (wfi_) vs a genuinely distinct concept needing a new kind (**if new kind → LOCKED-22 contract escalation**). Route to a `/arch-finalize` reconcile or a documented String. `last-consumer-slice: 9.2` (workflow events accrete) `(origin: 2026-06-07 P1.1)`
**Deferred perf (at scale, not yet triggered):**
- **6.3/6.4 perf-at-scale memoization** — `useMemo` the Shell's `sidebarItems`/`commandItems`; memoize the graph's `parentLabelOf` (O(n²) child→parent-label lookup in `ProjectGraph`). Both only matter once **real subscriptions + larger data** land (fixtures are tiny now). `last-consumer-slice: real subscriptions land (post daemon-integration)` `(origin: 2026-06-08 P6.2/P6.3)`

**Cross-track spreads (auto-resolve at the consumer slice):**
- **Reconcile UI provisional object types → generated** when the daemon freezes object schemas. Replace `ui/src/contracts/provisional.ts` shapes (`SessionRow` + `ProjectActivityRow`/`PullRequestRow`/`ApprovalQueueRow`/`AuditEventRow` + the `ProjectionPageByName` registry) with generated ones + delete the PROVISIONAL banners; includes the `ProjectionDelta.row` projection-discrimination + closed projection-name set, the **object-KEY strictness** harden — **RATIFIED (human) A-now → strict-at-freeze** (tolerant reads now; `.strict()` at freeze; enum-value reject-unknown stays strict throughout) — **and narrow `ProjectionItem.machine`/`status` (`ui/src/projections/items.ts`, intentionally `string` now — descriptor table is string-keyed w/ unknown-fallback, [[6]]) → the generated machine-name + status enum unions** (ProjectionItem-narrow origin: 2026-06-07 P6.3b). **Also (P6.4b): the Usage provisional shapes — `UsageRow`/`MetricQuality`/`Harness`/`CreditPool`/`UsageProjectionPage` (`provisional.ts`) → generated when the daemon freezes the telemetry/usage schema (`MetricQuality`=`exact|estimated|unavailable`, `Harness`=`claude|codex`); AND the credit-pool meter thresholds (UI-provisional `near_exhaustion ≤15% remaining` / `hard_stop` at exhausted — §11.4 pins no numbers) reconcile when the daemon defines credit-pool semantics** (Usage-shapes origin: 2026-06-07 P6.4b). **Also (P6.4d): the survival/recovery provisional shapes — `RecoveryState`/`ResumeMode`/`RecoveryStatus` + the optional `resume_mode` on `SessionRow` (`provisional.ts`) → generated when the daemon freezes the survival/failure-mode schema (§8/§17); the real recovery-state source swaps the fixture default at that integration** (recovery-shapes origin: 2026-06-08 P6.4d). **Also (P6.4d-2): the safety-state provisional shapes — `ConflictReason`/`FencingConflict`/`SafetyState` + `AuditIntegrityKind`/`AuditIntegrityState` (`provisional.ts`; `AuditOutcomeStatus` REUSES frozen `ActionRequest` via `.extract` → drift-pinned, no reconcile) → generated when the daemon freezes the §17 failure-mode/conflict schema; the real conflict + audit-integrity source swaps the fixture default at that integration** (safety-shapes origin: 2026-06-08 P6.4d-2). `last-consumer-slice: first daemon object-schema bump (Phase 1/2)` `(origin: 2026-06-07 P6.1a)`
- **ui ↔ daemon-1.5 integration** — **🟢 DAEMON SIDE LANDED 1.5 (`UdsGatewayPort` @ CONTRACT_VERSION 0.11.0; part (a) RESOLVED — `SUPPORTED_PROTOCOL_RANGE {min:1,max:1}` is daemon-authored + agrees with the ui's provisional pin).** Remaining ui-side work at the ui-track resume: (a) ~~reconcile SUPPORTED_PROTOCOL_RANGE~~ ✅ confirmed (matches); **regen the ui Zod against the 0.11.0 schema** + adopt the new IPC types (`ServerFrame` frame-mux, `ProjectionDelta`/`DeltaKind`, `protocol_error`, the canonical **`UsageLedger`** projection name — rename the ui's provisional `Usage`→`UsageLedger`); (b) swap the Shell's gateway default `MockGatewayPort` → the real client (mock → dev/test-only); **(c) the connected+version-unknown "checking" degraded-banner window — wired + unit-tested (P6.4) but trigger-pending: the real daemon-1.5 reconnect re-handshake drives connected→version-unknown past the `!data` load gate to surface the checking banner** (checking-trigger origin: 2026-06-08 P6.4-checking-banner). **NOTE:** the live `subscribe` delta-source is daemon-1.6 (mechanism @ 1.5; the ui's `subscribe()` path is live once 1.6 wires the broadcast + binds the socket). `last-consumer-slice: ui ↔ daemon-1.5/1.6 (UDS GatewayPort live)` `(origin: 2026-06-07 P6.1c; daemon side 2026-06-08 P1.5)`
- **⏸️ PARKED daemon-1.5 ui slices (Decision C)** — **6.3d (Session Terminal + inline permission card)** + **6.3e (Code/Diff review + per-hunk actions)** are deferred until the daemon freezes the contracts they consume — the **Terminal Channel (§6.4)** + the **mutation/intent-submission surface** (the gateway-client's `submit_action`/`approve` methods, today read-only — `gateway-client/types.ts`). **Rationale (Decision C, human-delegated):** 6.3d's permission card is the UI's FIRST mutation path (INV-SEC-1 / forbidden #6) — a safety contract is the worst thing to build provisionally + reconcile; the contract-freeze-before-consume foundation (Option-A+ discipline) + the parallel-track rule (consume a daemon contract only once stable — reads qualified, this mutation surface does not) both forbid building it now. **Build ONCE against the real frozen contract.** The intent seam, when it lands, serves ALL consumers (permission card, per-hunk actions, Dispatch, Brain Run-via-Gateway, notifications toggle, restart-session, policy grants). **Unblocks at:** daemon mutation/intent-submission + Terminal-Channel contract freeze (cross-track timing the human controls). `last-consumer-slice: daemon mutation/Terminal-Channel contract freeze` `(origin: 2026-06-07 P6.3d-escalation / Decision C)`
- **⏸️ Stale-precondition re-approval treatment (deferred, P6.4d-2 Step-2.5 TWEAK)** — `stale_precondition` (preview≠reality at execute, §17/§6.2) is a **RE-APPROVABLE** flow (regenerate preview + require fresh approval), **NOT** the never-auto-resolved hard-conflict card — so it was scoped OUT of 6.4d-2 L1 (fencing-only; the card's "never auto-resolved" copy must be true of every state it shows). Its display treatment belongs with the **approval/preview surface + the parked intent seam** (Gateway modal / 6.3d-e) — build it there when the intent seam lands. `last-consumer-slice: daemon mutation/intent-submission contract freeze (with the approval/preview surface)` `(origin: 2026-06-08 P6.4d-2 / Step-2.5 TWEAK)`
- **Wire the §5.0 contract gates into CI** — schema-diff (test 9) + 3-way verify (test 8) + the Codex schema gate (no `.github/workflows/` yet; run via `cargo test`/harness locally meanwhile) **+ the ui-side TS drift test** (generated Zod `.options` === frozen `$defs`) + the `CONTRACT_VERSION`===`x-contract-version` pin + the pnpm/corepack-restore note (corepack shim broken → `npm i -g pnpm`; `ui/.npmrc verify-deps-before-run=false`). `last-consumer-slice: Phase 10 (release/CI infra), or earlier if CI is stood up` `(origin: 2026-06-07 P0.5; ui additions 2026-06-08)`
- **1.6b deferrals (daemon, consumer-marked SPREADs)** — (i) `subscription_id` on the subscribe ack (`last-consumer-slice: 1.6d` — the subscribe-serve slice); (ii) macOS-only `getpeereid` cfg-guard so a non-macOS build links (`last-consumer-slice: Phase 10` CI; reviewer-sanctioned — no Linux CI yet); (iii) the `"SessionStarted"` event-type string-literal dup across `runtime/writer.rs` + the 1.2 projectors → a shared event-type-name const (desync risk; `last-consumer-slice: 1.6c` — next daemon event-touching slice); (iv) await in-flight serve tasks on shutdown via a `JoinSet` (low-risk — serve tasks use own read-only conns; `last-consumer-slice: 1.6d`). `(origin: 2026-06-08 P1.6b)`
- _(✅ DELETED at the 2026-06-08 1.6 round close-out: the **1.1 L3 §16 restore-failure-typing** item — DONE in 1.6a-L1 `48b7a2c` (typed `EventStoreError::RestoreFailed{from,migration_error,source}` via the pure `on_migration_failure` classifier, original error preserved).)_

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
- [x] Freeze the canonical enums (nine of §5.1's ten machines — ExecutionProfile held → 0.5b), the 22 shared IDs + prefixed-ULID format (§5.2), the actor enum (R-2), and the 4 desktop-addendum objects (§5.3) as `shared/` constants; reconcile into SOM. — **ExecutionProfile runtime-state enum HELD for 0.5b** (cat-4); everything else frozen.
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

### 1.1 — Event store: append-only events + envelope + FTS5 — ✅ **LANDED** (df753aa + b998f20 + 61089ea)
- [x] WAL SQLite opened with the §2 pragmas; `events` table per `§7.1` envelope (incl. reserved `payload_hash`/`previous_event_hash`); `seq` canonical order (atomic, `BEGIN IMMEDIATE`); `user_version` migration runner + §16 backup/rollback. — **§15 redaction gate fail-closed** (never persists `unredacted`; `redaction_status` + prefix Redactor); single-write-actor (read-only WAL readers); CONTRACT_VERSION 0.7.0. _(FTS5 scaffolding → populated with the AuditTrail projection in 1.2.)_
- [x] Files: root Cargo workspace + `daemon/` crate; `daemon/src/{lib,clock,idgen}` + `eventstore/{mod,schema,migrations,writer,redaction}` + tests; `shared/` envelope contract (extended, §5.0).
- [x] Cross-doc invariant: Event envelope + 4 enums (`source_type`[15 closed]/`sensitivity`/`visibility`/`redaction_status`) + `redaction_status`/`redaction_engine_version` columns — written to Appendix A / §7.1 / DATA_MODEL §2.1 / `daemon/CLAUDE.md` (0.6.0 envelope → 0.7.0 redaction). LESSONS §3 (single-write-actor).
- [x] Tests: 31 workspace (append/seq-canonical/single-writer/fail-closed-redaction[test 4]/secret-masked/audit-write-fail-closed/idempotency/migration+backup-restore/degrade/quarantine/golden-log/wal-pragmas) + 3-way verify @ 0.7.0; security-reviewer PASS all invariants.

**Spawned:** entropy fallback + quarantine→`SensitiveOutputRedacted` + §15 property/fuzz → **1.7** (blocking); §16 migration restore-failure typing → **1.6**; AppendIntent/object_refs → **1.2** (Carry-forward).

### 1.2 — Projection engine + the MVP projections + offsets — ✅ **LANDED** (c8c6c72 + a1dc482 + e66c659)
- [x] Projection workers fold events → the MVP `proj_*` tables (ProjectActivity, Session, ApprovalQueue, Worktree, PullRequest, PlanProgress, ProjectGraph, AgentTeam, AuditTrail, UsageLedger) + `object_refs`; advance `projection_offsets` in the same txn; startup replay + full rebuild; degraded handling. — **engine + offsets + `object_refs` + catch-up-replay/full-rebuild/degraded-skip + the 4 Phase-1-feedable projectors landed in-band in the event-commit txn; 6 projector bodies re-homed (scope below).**
- [x] **Projector scope (Option-A, human-approved 2026-06-07):** the engine + **all 10 table DDLs** + `object_refs` + `projection_offsets` land here; **projector BODIES = the 4 Phase-1-feedable** (Session, ProjectGraph/object_refs, AuditTrail+FTS, ProjectActivity). The other **6 re-homed to their producing phases** (each lands with its real event contract + tests; nothing dropped): ApprovalQueue→**2.1**, UsageLedger→**3.1**, Worktree→**5.2**, PullRequest→**7.1**, PlanProgress→**9.2**, AgentTeam→**9.3**. All 10 DDLs kept in 1.2 so the ui track fixtures every projection schema. `(origin: 2026-06-07 P1.2, brief 004)`
- [x] Files: `daemon/src/projections/` (NEW) — `{mod,schema,session,graph,audit,activity,object_refs}.rs`; `shared/src/events.rs` (`SessionStarted`, CONTRACT_VERSION 0.8.0); `eventstore/{mod,schema,migrations}.rs` (extended)
- [x] Cross-doc invariant: extended — projection status columns bind the frozen §5.1 enums; **EventTypeRegistry** (`SessionStarted`/0.8.0) + projection-surface rows written to Appendix A / `daemon/CLAUDE.md` cross-doc / DATA_MODEL §2.3. LESSONS §4.
- [x] Tests: happy (event → projection update); edge (rebuild-equivalence: full replay == incremental fold); error (projector exception → `state='degraded'`, skip, raw events intact); integration (single SessionStarted fans out to proj_session + proj_project_graph in one txn — demo step 7). — **15 tests; §17 strict-reconstruct-on-`open` Finding → 1.6.**

### 1.3 — Transactional outbox — ✅ **LANDED** (343cc09 + 707845a)
- [x] `outbox` table; event + projection + outbox commit in one txn; drainers for destinations (brain_mcp, github, linear, notifier, jsonl_mirror) with backoff + retryable/terminal classification. — **outbox table (migration 4) + in-txn write + §15 sync-sink gate + `drain_once` (backoff/retryable/terminal/bounded dead-letter) + crash-recovery (`reset_in_flight` on `open`) + `jsonl_mirror` landed; the 4 external destination adapters re-homed (Option-A): brain_mcp→8.1, github/linear→7.1, notifier→10.2; Tokio drainer spawn + bounded drain-pass → 1.6.**
- [x] Files: `daemon/src/eventstore/outbox.rs` (NEW), `daemon/src/{clock,idgen}.rs` (extended), `daemon/Cargo.toml` (time "parsing")
- [x] Cross-doc invariant: none frozen — `out_` id + `destination`/`status` enums are **daemon-internal** (no CONTRACT_VERSION bump); recorded in Appendix A / `daemon/CLAUDE.md` cross-doc / DATA_MODEL §2.5. LESSONS §5.
- [x] Tests: happy (drain delivers once); edge (retryable 429/5xx backoff vs terminal 401/403 → dead after budget); error (crash between event-commit and delivery → redelivered, no double-apply); integration (outbox is the only path events reach external destinations). — **9 tests; full suite 56 green; security PASS.**

### 1.4 — Lease locks + fencing tokens + single-instance — ✅ **LANDED** (b3f7612 + 442149a + 0347ac6)
- [x] `leases` table (one row per **`(resource_id, lease_kind)`**: owner_id, fencing_token, acquired_at, heartbeat_at, expires_at — Appendix A field set; MVP_TASKS abbreviation reconciled); owner-guarded renew (token unchanged); expired-lease reclaim mints a new token; `pidlock` single-instance; `reap_once` reaper unit. — **`fencing_token` = persisted monotonic high-water mark (+1 on acquire/reclaim under `BEGIN IMMEDIATE`; release/reap KEEP it; survives restart). Authority = a LIVE lease (Option B, human-ratified): `validate_held` = owner + token-match + `expires_at > now`; "stale" = expired OR superseded → `fencing_conflict` (safety rule #6). pidlock = std advisory file lock (OS-fd-held, auto-released on death, immune to PID reuse). Reaper Tokio spawn + pidlock cold-start call site → 1.6.**
- [x] Files: `daemon/src/locks/` (NEW — `mod.rs` + `lease.rs` + `pidlock.rs`); `eventstore/{schema,migrations,mod}.rs` (MIGRATION_5_LEASES, `SUPPORTED_USER_VERSION` 4→5, 5 EventStore delegating methods over the single `Connection`); `lib.rs`.
- [x] Cross-doc invariant: NEW — Lease (Appendix A / §5.1/§7.2/§17 / `daemon/CLAUDE.md` cross-doc / LESSONS §6). **Daemon-internal — no `shared/` surface, no CONTRACT_VERSION bump** (outbox-analogous).
- [x] Tests: happy (acquire/renew/release w/ token-preserve); edge (paused holder's stale token rejected after reclaim — fencing, fake clock) + **expired-holder-rejected-even-if-unsuperseded (Option-B safety pin)**; error (PID reuse doesn't grant a false single-instance — OS-fd oracle, not `kill(pid,0)`); integration (survives daemon restart — reclaim + new token, monotonicity persisted). — **11 lease/pidlock/reaper tests; workspace 67 green; security-reviewer PASS every layer.**

### 1.5 — UDS GatewayPort transport + handshake (read/subscribe surface) — ✅ **LANDED** (ae27f1d + fb941e8 + c1c54e5 + 6690e41)
- [x] UDS server with 4-byte length-prefixed JSON-RPC; `HelloFrame`→`HelloAck`|`VersionSkewError`; `getpeereid()` peer-auth (uid==daemon-uid, **FIRST** in the handler, **NOT `SO_PEERCRED`** — rule #7); `MAX_FRAME_SIZE` (**8 MiB**, pre-alloc reject); the read-only methods (`get_projection`, `subscribe`, `get_capabilities`); **`ServerFrame`** frame-type multiplexing (`rpc_response`|`subscription_push`; Terminal reserved). (Mutation methods → Phase 2.) — **CONTRACT_VERSION 0.11.0; +`protocol_error` (lead-ratified); two version axes (`CONTRACT_VERSION` vs `protocol_version` `{1,1}` daemon-authored); read-only WAL reads (single-writer); bind/accept-spawn + live subscribe delta-source → 1.6.**
- [x] Files: `daemon/src/ipc/{mod,transport,peer,server,methods,subscribe}.rs` + `shared/src/ipc.rs` (NEW) + `shared/src/{lib,schema}.rs` + `contracts/schema/*.json` (regenerated); +`libc 0.2`.
- [x] Cross-doc invariant: NEW — GatewayPort method surface + §6.4 IPC wire contract (Appendix A / §6.1 / §6.4 / §5.0 / `daemon/CLAUDE.md` cross-doc / LESSONS §7). **Cross-language `shared/` contract @ 0.11.0** (the ui consumes it; ui regen + `ui/CLAUDE.md` row → ui-track resume, Carry-forward).
- [x] Tests: happy (handshake + projection read); edge (version skew → structured error + disconnect); error (wrong-uid peer rejected; oversized frame rejected); integration (read over UDS). — **15 ipc tests; workspace 83 green; rule #7 + single-writer security-PASS'd; L4 `ServerFrame` frame-mux + subscribe mechanism.**

### 1.6 — First-run bootstrap + migrations backup/rollback + version-compat — 🔄 **IN PROGRESS (split 3-way + 1.6d)**
> **Round 2026-06-08:** **1.6a-L1/L2 ✅ + 1.6b ✅ LANDED** (`48b7a2c`/`f1de088` + `26898a8`/`9448035`/`8d06ed7`/`f9c31b1`/`39f756e`; 105 workspace green; security PASS; daemon-internal — CONTRACT_VERSION stays 0.11.0; LESSONS §8/§9; session doc 006). **HELD for next cycle:** **1.6a-L3** (registration, USER-RULED Option B; closes #11) · **1.6c** (§17 degradable replay, USER-RULED Option C; brief 029) · **1.6d** (subscribe-SERVE push, #13). See Currently-in-progress + Decisions-tabled.
- [ ] Cold-start ordering (§16) — 🔶 PARTIAL: the orchestration (create_dir_all → pidlock → `EventStore::open` → `DaemonVersionInfo`) ✅ LANDED 1.6a-L2 (`f1de088`); **register desktop-host Device + LocalRunner → 1.6a-L3 (held)**; bind UDS ✅ 1.6b; launchd install → Phase 10. Pre-migration DB backup + restore-on-failure + `app_version↔user_version` floor were already in place (1.1); the **enforcing** version-compat floor at bootstrap = the DB `user_version` check.
- [ ] **Degradable catch-up replay on `open` (§17 — re-homed from the 1.2 L3 Finding; human-ratified defer 2026-06-07, Option A)** — 1.2's `catch_up_replay` reconstructs via the **strict** read path, so one corrupt event row aborts `open` → daemon won't start (no in-product recovery; `rebuild_projections` sits behind the failing `open`). §17 (line 391) requires the read path to **quarantine/degrade + continue** (`read_all_degradable`), not crash. **1.6 must DECIDE + TEST the semantics (not just swap readers):** the §17-correct behavior for a corrupt EVENT row — **degrade/skip-and-continue** (`read_all_degradable`) **vs refuse-safely + restore `.bak`** — carries an **audit-integrity dimension** (skip-and-continue could *silently drop a real event*). **Flag to the human at 1.6 if the skip-vs-refuse call turns load-bearing.** **Bundle:** a replay-time `redaction_status='unredacted'` **quarantine-skip** (security-medium; non-producible via the write gate today — defense-in-depth on replay). Tests: corrupt row → `open` recovers per the chosen semantics (degrades-not-crashes, or refuses-safely + restores `.bak`); legacy unredacted row → quarantine-skipped. **Bundle (1.6a-L1 cq-medium, routed 2026-06-08):** the `-1` unreadable-seq quarantine sentinel → a named const / `Option<i64>` typing (belongs with the §17 read-path that owns `DegradableEvent`; not fixed in 1.6a — scope). `(origin: 2026-06-07 P1.2 L3 — security+cq HIGH Finding; human-ratified defer + skip-vs-refuse design mandate)`
- [x] **Wire the daemon-runtime Tokio tasks (re-homed from 1.3 + 1.4)** ✅ **LANDED 1.6b L2 (`9448035`)** — spawn the outbox `drain_once` loop on an interval (the 1.3 `drain_once` unit + `reset_in_flight`-on-`open` are ready; the *spawn* lands here, §12); **bound a single drain pass (LIMIT/batch)** so a large backlog can't starve the writer (1.3 security-low deferral). **Spawn the lease reaper `reap_once` loop on an interval** (the 1.4 `reap_once` unit is ready; only the spawn is here, §12). `(origin: 2026-06-08 P1.3 + P1.4)`
- [x] **Wire the 1.4 `PidLock` into the cold-start ordering** ✅ **LANDED 1.6a-L2 (`f1de088`)** — the §16 cold-start bullet above ("pidlock → reclaim stale socket → …") is satisfied by the 1.4 `locks::PidLock` mechanism (std advisory file lock); this lands the *call site* (acquire the pidlock first; on `AlreadyHeld` refuse the second instance). `(origin: 2026-06-08 P1.4 L2)`
- [ ] **Wire the 1.5 UDS GatewayPort runtime (re-homed from 1.5)** — 🔶 PARTIAL: `bind()` (unlink-before-bind) + accept-loop + concurrency cap + the broadcast subscribe-**SOURCE** (publish-after-commit) ✅ LANDED 1.6b (`8d06ed7`/`f9c31b1`); **the socket subscribe-SERVE push → 1.6d (#13)**; the platform cfg-guard + `subscription_id` handle + one-read-only-WAL-conn-per-session → Carry-forward. Original scope: `bind()` + the accept-loop spawn (the §16 cold-start "bind UDS" step; `serve_connection` is ready); the **live `subscribe` delta-source** (`EventStore::append` → a `tokio::sync::broadcast` → the subscriber push, via the `try_clone` read/write split — a **writer↔IPC seam**, slightly more than a spawn); a **platform cfg-guard** (`getpeereid` is macOS/BSD-only → gate the UDS stack to the macOS-MVP target so a Linux build links — security-MEDIUM, reviewer-sanctioned defer); an accept-loop **concurrency cap** (bound concurrent 8 MiB frame buffers + live connections — anti-DoS at scale); **one read-only-WAL conn per session** (today `get_projection` opens per-request); a `subscription_id` handle on the subscribe ack if multiple concurrent subscriptions per connection are supported. `(origin: 2026-06-08 P1.5)`
- [x] **`eventstore::user_version()` → `Result<u32>` (inlined from 1.1 L2 via 1.2 triage)** ✅ **LANDED 1.6a-L1 (`48b7a2c`)** — typed `Result<u32,_>` (out-of-u32 → `Migration`, not `Reconstruct`); the `-1` sentinel removed. `(origin: 2026-06-07 P1.1 L2; triaged 2026-06-08; done 1.6a-L1)`
- [ ] Files: `daemon/src/bootstrap.rs` (NEW), `daemon/src/eventstore/migrations.rs` (extended)
- [ ] Cross-doc invariant: NEW — Device, LocalRunner (Appendix A, §5.3); Version-compatibility matrix (§16)
- [ ] Tests: happy (clean first run creates DB + runs migrations); edge (stale socket reclaimed; second instance refused); error (bad migration → restore `.bak`, refuse-safely; downgraded binary sees newer DB → refuse); integration (daemon restart resumes against existing DB).

### 1.7 — Redactor: entropy fallback (OQ-SEC-2) — 🔒 **BLOCKING Phase-1 acceptance** (human, 2026-06-07)
- [ ] Extend the 1.1 `Redactor` (token-prefix high-recall set) with the **Shannon-entropy fallback on `KEY=value` lines** (OQ-SEC-2, §15) so full secret-detection recall doesn't drift. The 1.1 fail-closed gate + `redaction_status` already enforce the invariant; this raises *recall*. Same redactor gates all 3 sinks (persist/embed/sync, §15).
- [ ] Files: `daemon/src/eventstore/redaction.rs` (extended) or `daemon/src/policy/redactor.rs` (per the 1.1 placement decision).
- [ ] Cross-doc invariant: none (refines the §15 redactor; OQ-SEC-2 resolves).
- [ ] Tests: happy (entropy fallback catches a high-entropy `KEY=value` secret the prefix set misses); edge (low-entropy config value NOT redacted — no false-positive storm); error (unredactable high-confidence secret → quarantine + `SensitiveOutputRedacted`); integration (all 3 sinks gated by the same redactor).
- [ ] **Absorbs the L3-deferred §15 items** (1.1's prefix Redactor redacts-or-passes, never quarantines): wire the **quarantine → `SensitiveOutputRedacted` emission** path + the **§15 property/fuzz test** (no secret persists `unredacted` across fuzzed payloads). `(origin: 2026-06-07 P1.1 L3)`
- [ ] **Human chose this as a blocking task** (not a loose Step-9 flag) specifically so secret-detection recall can't silently drift below the §15 bar.

### Acceptance criteria (1)
- [ ] Daemon starts detached, single-instance, survives UI restart; event store is the sole writer.
- [ ] Projections rebuild deterministically; offsets crash-safe; `/preflight` clean.
- [ ] Lease fencing rejects stale holders; UDS peer-auth (`getpeereid`) + handshake enforced.
- [ ] **§15 redaction: 1.1 fail-closed gate + `redaction_status` AND the 1.7 entropy fallback (OQ-SEC-2) both landed** — no event persists `unredacted`; full prefix+entropy recall. (Gate is the blocking bar.)

---

## Phase 2 — Action Gateway (the mutation chokepoint)

**Goal:** The single, audited mutator. Typed `ActionRequest`/`ActionPlan` over the GatewayPort, the staged pipeline (normalize → resolve → policy/risk → preview/dry-run → approval → execute → audit), risk 0-4, **bundled ActionPlan + step-by-step approval (O-3)**, idempotency, fencing-guarded execution, stale-precondition re-check, fail-closed-on-audit-write, and the MVP executor adapters' interface.

**Spec anchors:** `ARCHITECTURE.md §6`, `§6.1`, `§6.2`, `§6.3`, `§5.1` (Approval/ActionRequest), `§15` (INV-SEC-1, fail-closed), `§17` (stale-precondition, fencing-conflict, daemon-crash-mid-action).

**Track / deps:** `daemon-core`. **Deps:** Phase 1, 0.5. **Parallel-with:** Phase 6 (ui); Phase 5 (git, against the frozen Gateway iface). Unblocks the `edges` track's intercept→intent path.

### 2.1 — Gateway pipeline + ActionRequest/ActionPlan model + mutation methods
- [ ] `submit_action`/`submit_action_plan`/`preview_action`/`approve`/`deny` GatewayPort methods; the staged pipeline; `action_requests`/`approvals` durable rows; the two split state machines (R-5); authoritative `ActionExecution*` events emitted ONLY by the Gateway.
- [ ] **proj_approval_queue projector body (re-homed from 1.2, Option-A)** — fold the Gateway's `ActionRequested`/`Approval*` events → `proj_approval_queue` (the Human Input Queue read model, AG §8; `sort_key` risk DESC, age). Table DDL pre-created in 1.2 (migration 3); this lands the projector + its event contract + tests. `(origin: 2026-06-07 P1.2)`
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
- [ ] **Gateway fencing oracle = 1.4's `validate_held` (live-lease authority) + heartbeat/renew long actions** — Option B (human-ratified 2026-06-08): "stale" = NOT a live lease (**expired OR superseded**). The gateway calls `locks::validate_held(resource_id, lease_kind, owner, token, now)` after lock + before execute → false → `fencing_conflict`. It **must renew/heartbeat (or re-acquire)** a long-running action so a legitimately-long action isn't fenced by its **own** lease expiry (self-fence → false `fencing_conflict`). `(origin: 2026-06-08 P1.4 — fencing Option-B ruling)`
  - [ ] **Define the idempotent same-owner re-acquire contract** — 1.4 returns `Held{owner:self}` on ANY live re-acquire incl. the *same* owner (1.4 deliberately did NOT pin a guessed semantics). The gateway decides whether a same-owner re-acquire is idempotent (renew-like) or an error. `(origin: 2026-06-08 P1.4 L1 flag)`
  - [ ] **Account for the strict expiry boundary** — `validate_held`/renew use `expires_at > now` (strict) + reject TTL≤0; the gateway heartbeat loop must renew strictly *before* the boundary, not at it. `(origin: 2026-06-08 P1.4 L1 flag)`
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
- [ ] **proj_usage_ledger projector body (re-homed from 1.2, Option-A)** — fold telemetry/usage events (derived from `TelemetrySample`) → `proj_usage_ledger` rollups (tokens/context/cost per day|project|session|model, `metric_quality`, §18). Table DDL pre-created in 1.2 (migration 3); this lands the projector + its event contract + tests. `(origin: 2026-06-07 P1.2)`
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
- [ ] **proj_worktree projector body (re-homed from 1.2, Option-A; incl. the Q6 two-axis precedence fn)** — fold worktree events + live git2 reads → `proj_worktree` with the git+overlay status precedence (§7.2; §5.1 R-7). Table DDL pre-created in 1.2 (migration 3); this lands the projector + precedence fn + tests. `(origin: 2026-06-07 P1.2)`
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

### 6.1 — App shell + design-system integration + daemon-connection/read-only mode ✅ (6.1a fd9738b + 6.1b 39a87c6 + 6.1c 402f4c5)
- [x] Tauri shell (top bar, project switcher, sidebar, right drawer stack, activity dock, status bar); link `NexusOps-ui-kit` tokens + components; **daemon-connection indicator** (connected/reconnecting/disconnected) distinct from LocalRunner health; **global READ-ONLY degraded mode** disabling all intent-submitting controls + banner/Repair (fail-safe `canSubmitIntent`; security-reviewer PASS).
- [x] Files: `ui/src/{shell,contracts,gateway-client,connection}/` (NEW). _(Corrected: built `ui/src/gateway-client/` [interface + boundary + MockGatewayPort], NOT `ui/src/lib/gateway-client.ts`. The **real `UdsGatewayPort`** is deferred to the daemon-1.5 integration — mock-backed until then; see Carry-forward.)_
- [x] Cross-doc invariant: none new (the generated Zod layer is a drift-caught consumer; rows added to `ui/CLAUDE.md`)
- [x] Tests: happy (shell renders from projections); edge (daemon disconnected → read-only mode disables intent controls via `canSubmitIntent`); error (version-skew → "update required" + precedence); integration (UI never writes the DB; reconnect restores live state).

### 6.2 — Status rendering binding + attention-rank table ✅ (6.2a b32c3c0 + 6.2b e2cebbc)
- [x] StatusPill keys == §5.1 enum strings (all **9 frozen** machines — 113 states, incl. 17 Session states + `changes_ready`; ExecutionProfile held 0.5b → deferred); one canonical status→attention-rank table (drift-pinned, no fall-through to idle) driving sidebar weight + queue membership + sort; four-channel "never color alone"; Approval vs ActionRequest as two surfaces.
- [x] Files: `ui/src/status/{attention,descriptors,worktree}.ts` (model, P6.2a) + `{StatusPill,AttentionMarker}.tsx` (rendering, P6.2b). _(Corrected from `shared/contracts/attention.ts`: attention-rank is UI render policy, not a frozen cross-language contract; the ui track does not write the frozen `shared/` crate — P6.2a Q4.)_
- [x] Cross-doc invariant: extended — the status enums + the attention-rank table (rows in `ui/CLAUDE.md`, §5.1/§11.3)
- [x] Tests: happy (one pill per state of each machine); edge (waiting_on_permission/conflict/stale enter "Needs my attention"); error (unknown status → visible, not idle — incl. the kit `||idle` guard); integration (attention ordering matches PRD §5.2).
- [ ] **Widen `ProjectionDelta.row` to projection-discriminated** (currently `SessionRow`-specific) when non-Session subscriptions land here — not widened in 6.1b to avoid prematurely breaking the delta typing (cq-medium; tracked in Carry-forward → provisional reconcile). _(origin: 2026-06-07 P6.1b)_

### 6.3 — Core screens (Command Center, Project Home/Graph, Sessions, Terminal, Editor/Diff) — ⏳ PARTIAL (6.3a–6.3c landed; 6.3d–6.3e PARKED on daemon-1.5)
> **Decomposed:** 6.3a Command Center → 6.3b Graph+list/table-fallback → 6.3c Sessions → 6.3d Terminal → 6.3e Code/Diff. **Landed:** 6.3a (`144b6b6`), 6.3b (`c420cd7` L1 + `885cc0d` L2), 6.3c (`23fbda3`). **⏸️ PARKED (daemon-1.5):** 6.3d (Terminal + permission card) + 6.3e (Code/Diff per-hunk actions) — both consume daemon contracts that are NOT frozen (the **Terminal Channel §6.4** + the **mutation/intent-submission surface**; 6.3d's permission card is the UI's FIRST mutation path, INV-SEC-1/forbidden #6). See Carry-forward "parked daemon-1.5 ui slices". _(**Decision C, 2026-06-07** [human, delegated-on-architectural-correctness]: build the first mutation seam ONCE against the real frozen contract — never provisionally; reorder to the daemon-independent 6.4 work. **Next = 6.4a (§11.6 a11y MUSTs).**)_
- [ ] Command Center triage (needs-attention/working/settled incl. a Changes-ready grouping) ✅ **6.3a (144b6b6)**; Project Graph **with list/table fallback** ✅ **6.3b (c420cd7+885cc0d)**; Sessions list/board (dense table) ✅ **6.3c (23fbda3)**; Session Terminal (PTY + inline permission card) ⏸️ **PARKED — daemon-1.5 (6.3d)**; Code/Diff review with per-hunk actions ⏸️ **PARKED — daemon-1.5 (6.3e)**.
- [ ] Files: `ui/src/views/{command,graph,sessions,terminal,code}/` (extended from kit)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (each screen renders fixtures); edge (graph list-fallback a11y equivalence — every node/edge appears as a row); error (empty/degraded states); integration (graph node → Inspector; session → terminal).
- [x] **Graph zero-projects degraded state** ✅ **RESOLVED by P7.3(fwd) active-project (`86727ec`)** — the Graph now re-roots at the active project (Q3 resolved) + an explicit `graph-no-project` guard renders when there's no active project (distinct from the per-project `graph-empty`); no more empty-pid root. _(origin: 2026-06-07 P6.3b)_
- [x] **Sessions-table filtering** ✅ **LANDED — polish slice 3 (brief `024`, hash at round close)** — pure `filterSessionRows` (status-exact AND ci name|project substring, fields OR'd independently — cross-boundary false-positive caught + pinned in review) + `distinctStatuses` (from the unfiltered set); composed **filter→sort** (attention order holds in the subset); labeled status `<select>` + search `<input>`; filtered-empty distinct from truly-empty. §13 family; Lesson §13 extended; §11.2 facets pinned. **Project facet deferred** as redundant — project scoping is owned by the active-project switcher (the Shell pre-filters rows); re-add only on a real unscoped need. _(origin: 2026-06-07 P6.3c)_

### 6.4 — Survival/failure UI surfaces + Usage dashboard + Settings + accessibility — ✅ DONE (6.4a–e + 6.4d-2 + checking-banner; **Phase-6 LOGIC COMPLETE** — only 6.5 theme/visual pass remains)
> **Decomposed (Decision C, 2026-06-07):** split by daemon-dependence. **Daemon-INDEPENDENT (active, fixture-drivable) → do now:** ✅ **6.4a §11.6 a11y MUSTs** (global `:focus-visible` ring + reduced-motion — LOCKED merge-gate; **`f70757e`**) → ✅ **6.4b Usage dashboard** (accuracy labels / Codex=unknown / credit-pool meter; **`db9b89b`** — interim 4th content-view, **relocates to a Settings tab at 6.4c**) → ✅ **6.4c Settings tabbed surface + Usage relocation** (`765923f` — ARIA tablist; honest pending stubs; ExecutionProfile 0.5b-gated; **nav-reconcile §11.2 TopBar tracked**) → ✅ **6.4d Survival/recovery DISPLAY** (`290381a` — recovery banner + resumed/replayed indicator; **split:** the **§17 safety-state display [6.4d-2: fencing/hard-conflict + fail-closed/audit-integrity — touches §15/§17 → security-reviewer]** is the **NEXT** sub-slice) → ✅ **TopBar slice (P6.4e §11.2 nav rewire + accessible-names — `823d16e`)** · ✅ **6.4d-2** (§17 safety-state display — L1 fencing/hard-conflict card #6 `ff2f8d6` + L2 fail-closed/audit-integrity alert #5 `503b6a2`; security-reviewer PASS both layers; 128 green) · ✅ **checking/handshaking banner** (`5f40149` — silent-read-only closed; connected+unknown trigger-pending → daemon-1.5 spread) · **Phase-6 LOGIC COMPLETE** → remaining: **6.5 the theme/visual pass (below).** **PARKED/GATED:** the **intent-submitting controls** scattered here (restart-session, Notifications toggles, policy/`save-as-policy`) share the **daemon-1.5 intent seam** (parked with 6.3d/6.3e — see Carry-forward); **drag→non-drag equivalents** (TASK-5) are **forward-looking** — the named drag surfaces (task-chip/Dispatch-dialog) are intent-coupled + don't exist yet, so the non-drag MUST lands with them (forbidden #5 still pins the rule); the **ExecutionProfile Settings tab** stays **0.5b-gated** (don't hard-bind).
- [ ] Recovery banner + per-session resumed/replayed indicator + "restart session"; fencing/hard-conflict + fail-closed/audit-integrity surfaces; Usage dashboard (exact/estimated/unavailable, **Codex context=unknown**, **credit-pool meter**); Settings (Integrations health, Execution Profiles, Security & policy, Notifications); **global `:focus-visible` ring**; **drag→non-drag equivalents**; reduced-motion.
- [ ] Files: `ui/src/views/{usage,settings}/`, `ui/src/recovery/`, `ui/src/a11y/` (NEW/extended)
- [ ] Cross-doc invariant: none
- [ ] Tests: happy (usage labels render; Codex shows "unknown"); edge (every drag has a button/menu path — TASK-5; focus ring on all controls); error (credit-pool near-exhaustion + hard-stop states); integration (recovery banner after restart; audit-integrity alert renders).
- [ ] **Accessible names on shell controls + kit closed-props (§11.7)** — kit components have **closed prop types with no `HTMLAttributes`/`aria-*`/`data-*` passthrough** (confirmed across `Button` [aria-label], `StatusPill`/`AttentionMarker` [data-*]). Back/forward + icon-only controls need accessible names; the surface chip / markers route `aria-*`/`data-*` onto NexusOps wrapper elements as the current workaround. Resolve via a §11.7 kit component-contract addition (HTMLAttributes passthrough) or keep the wrapper pattern. _(origin: 2026-06-07 P6.1b; broadened P6.2b)_
- [ ] **ExecutionProfile status descriptors/pill** — add the 10th machine's `(machine,status)` attention-rank descriptors + StatusPill mapping once its enum freezes (0.5b); surfaces in the Settings → Execution Profiles tab. The 6.2a completeness test iterates only the generated layer, so it won't fail until the enum lands. _(inlined from Carry-forward; origin: 2026-06-07 P6.2a → 0.5b gate)_
- [ ] **"Checking/handshaking" degraded-banner variant** — `deriveDegradedState(connected, version=unknown)` → "ok" (no banner) while `canSubmitIntent` is FALSE; add a checking/handshaking banner so read-only is never silently unexplained once intent controls exist (currently unreachable-through-Shell + security-confirmed). _(inlined from Carry-forward; origin: 2026-06-07 P6.1c)_
- [ ] **Real Repair / update-relaunch UI** — the DegradedBanner Repair affordance currently aliases `reconnect()` (named-distinct + TODO); wire the real repair (daemon-1.5) + the version-skew update/relaunch flow (§16, with packaging in Phase 10). _(inlined from Carry-forward; origin: 2026-06-07 P6.1c)_

- [x] **Settings nav reconcile (§11.2)** — ✅ **RESOLVED by P6.4e** (TopBar Settings wired → `contentView="settings"`; "Settings" dropped from the view-switch). _(FINDING — orch brief error; §11.2 nav model human-CONFIRMED 2026-06-08; arch-doc note added (`ARCHITECTURE.md §11.2`); origin: 2026-06-08 P6.4c)_
- [x] **TopBar back/forward history nav** ✅ **LANDED — polish slice 1 (brief `022`, hash at round close)** — the previously named-but-INERT back/forward now drive a real content-view navigation history: a pure `viewHistoryReducer` (browser semantics — forward-truncation, collapse-on-same) + `useViewHistory()` hook (Lesson §13 family); back/forward `disabled` at the history boundaries (wire-or-disable — never a dead click, §11.6); §11.2 history semantics pinned in `ARCHITECTURE.md`. The global `.sr-only` utility already replaced the inline `SR_ONLY` at 6.5a. _(origin: 2026-06-08 P6.4e → landed polish slice 1)_
- [ ] **Brain TopBar trigger (named-but-placeholder)** — the TopBar "Brain" `<Button>` is named but inert (no drawer to open); lands with the **Brain drawer (Phase 8, §11.5)**. Until then it's a known placeholder, not a wire-or-disable miss (the affordance is Phase-8 scope, not a closable gap now). _(split from the P6.4e TopBar item; origin: 2026-06-08 P6.4e)_
- [x] **Tablist arrow-key roving + §9 audit refinement** ✅ **LANDED — polish slice 2 (brief `023`, hash at round close)** — WAI-ARIA roving tabindex on the Settings tablist (one tabstop; Arrow/Home/End move + auto-activate; focused index read at event time — high stale-closure race caught + fixed in-slice); pure `nextTabIndex` core (widget-agnostic, slice-5 reuse). The §9 audit extracted to vitest-free `a11y/reachability.ts` + taught roving-awareness (a `tabIndex=-1` `role="tab"` reachable iff its tablist has exactly one tabstop) + `role="tabpanel"` coverage. Lesson §9 extended; §11.6 arch note pinned. _(boundary: the audit assumes flat tablists — nested-composite hardening YAGNI; origin: 2026-06-08 P6.4c)_
- [x] **Sidebar resume-mode indicator** ✅ **LANDED — polish slice 4 (brief `025`, hash at round close)** — the resumed/replayed indicator (▶/⟳, reusing `describeResumeMode`) now shows on the sidebar item via an **id-keyed side map** (`resumeModesBySessionId`, machine-guarded lookup) passed as a prop — the shared `ProjectionItem` is NOT widened (§8 verified by a no-map test). Glyph + `.sr-only` label (never color alone). Lesson §8 extended (entity side-map pattern). _(origin: 2026-06-08 P6.4d; §8-respecting follow-up)_

### 6.5 — Graphite Arc theme/visual layer + automated visual gate — ✅ DONE (LEAD-ACCEPTED visual gate 2026-06-08; 6.5a `c439379` + 6.5b `b5618af` + 6.5c `3b6135d`; final aesthetic sign-off flagged for the user)
> **Decomposition (orchestrator, 2026-06-08 — from the 6.5 recon):** 3 visual layers (**non-`/tdd`** — design/visual-gate slices with an optional lightweight wiring-guard; the kit `styles.css` tokens are ALREADY imported in `main.tsx` — the gap is APPLYING them) + the gate, driven layer→layer (Lesson §7). Reference the kit **SEMANTIC** token layer only (never primitives — §11.1 re-hue promise). Target = the assembled prototype `NexusOps-ui-kit/ui_kits/control-plane/index.html` (dark cockpit, Geist, a 3-area CSS Grid, token chrome).
> - **6.5a — base/global layer:** `ui/src/theme/global.css` — `html,body` reset (`--surface-window` bg + Geist `--font-sans` + `--text-primary` + overflow), scrollbar chrome, a global `.sr-only` utility (replaces TopBar's inline `SR_ONLY`). Imported in `main.tsx` after the kit `styles.css`. Wiring-guard: assert the stylesheet is imported + applies `body` styling.
> - **6.5b — shell chrome + grid:** `ui/src/theme/shell.css` — refactor `Shell.tsx` flexbox→the prototype's CSS Grid (`.shell` 3-area: top / sidebar+main / dock); `.topbar`/`.sidebar`/`.main`/`.status-bar`/`.safety-host` token surfaces+borders+elevation; lift `Shell/TopBar/Sidebar/StatusBar` inline structural styles → token classes.
> - **6.5c — view/component panels:** `ui/src/theme/components.css` — `.command-center`/`.sessions`/`.project-graph`/Settings/Usage + the banner/safety surfaces; lift remaining view inline styles → classes (semantic surfaces + type scale + `--sp-*` spacing).
> - **Visual gate (acceptance):** gstack-browser comparison — running dev server vs the prototype — verify dark-cockpit/Geist/grid/chrome MATCH (**human-judged** per Lesson §10). ~700–900 lines CSS + TSX refactor total.
>
> **Why this exists (FINDING, 2026-06-07):** 6.1–6.4 shipped functionally-correct but **completely unstyled** — the kit `styles.css` only DEFINES tokens (`:root` vars); nothing in the app APPLIES them (no `body{}` bg/font, no layout grid, no panel chrome). Only the kit's self-theming components (StatusPill/AttentionMarker) render themed. The Graphite Arc visual layer was never authored, and **no rendered-product verification existed** (jsdom/tsc/oxlint/build don't check appearance — see `ui/LESSONS.md §10`). Human-sequenced as a **dedicated pass at the END of Phase-6 logic** (NOT next); unstyled surfaces are accepted to accumulate until then.
- [x] **Theme/base layer** ✅ **6.5a (`c439379`)** — `ui/src/theme/global.css` (dark `--surface-window` bg + Geist + base type + scrollbar chrome + global `.sr-only`); 6.5b added the cockpit layout grid + panel chrome. Semantic tokens only (§11.1). _(gate caught a `*/`-in-CSS-comment bug — Lesson §12.)_
- [x] **Theme the shell + views** ✅ **6.5b (`b5618af`, shell chrome + flexbox→CSS-Grid) + 6.5c (`3b6135d`, view panels + severity surfaces)** — TopBar/Sidebar/ActivityDock/StatusBar + CC/Graph/Sessions/Settings/Usage themed via `theme/{shell,components}.css`; inline structural styles lifted; region surfaces grounded to the prototype. The right-hand Human Input Queue stays PARKED (Phase 8 / intent-seam — the §17 safety surfaces render Shell-level meanwhile, §11.4 note). DrawerStack = `:empty` overlay (Brain = Phase 8).
- [x] **Automated visual gate (acceptance mechanism + standing gate)** ✅ — ran the gstack-browser comparison (dev server over HTTP vs the prototype, all views + driven safety/degraded fixtures, 6 screenshots); **standing gate established** for UI styling work going forward. _(origin: 2026-06-07 theme-finding / human decision)_
- [x] **Acceptance = VISUAL** ✅ **LEAD-ACCEPTED (best-effort, production-grade) 2026-06-08** — the foundation token-matches the prototype (dark canvas / Geist / grid / chrome / additive-severity surfaces); every divergence = an unbuilt Phase-7/8 feature or an approved app-specific adaptation, no theme defects. **Final aesthetic sign-off flagged for the user on return.**

### Acceptance criteria (6) — ✅ MET (Phase 6 logic + visual complete; modulo parked 6.3d/e on daemon-1.5)
- [x] Projection-driven UI; read-only degraded mode works; status binding covers all **9 frozen** machines (ExecutionProfile = the 10th, 0.5b-gated — deferred, not Phase-6-blocking).
- [x] Accessibility MUSTs pass (graph list fallback, focus ring) — frontend merge-gate tests green. _(drag→non-drag: the named drag surfaces are intent-coupled + don't exist yet; the non-drag MUST lands with them — forbidden #5 still pins the rule.)_
- [x] **Graphite Arc theme/visual layer applied (6.5) + the automated visual gate passes** — the dev server foundation visually matches `NexusOps-ui-kit/ui_kits/control-plane/index.html` (LEAD-ACCEPTED; not implied by green unit tests — `ui/LESSONS.md §10`/`§12`).

---

## Phase 7 — Integrations, PR Review Workspace & Task Inbox

**Goal:** GitHub (octocrab + gh-token bootstrap) and Linear (PKCE/key) read/link; the **full PR Review Workspace (O-6)**; Task Inbox + Dispatch; manual linking. GitHub-authoritative PR with re-fetch-before-merge.

**Spec anchors:** `ARCHITECTURE.md §9`, `§11.2` (PR Review), `§7.2` (PR SoT), `§6.3` (github/linear actions), `§17` (integration-failure contract), `§8` (intake/PR flows).

**Track / deps:** `edges`. **Deps:** 0.3, Phase 2 (Gateway), Phase 5 (git). **Parallel-with:** ui (Phase 6); the PR-Review-Workspace UI lands in the ui track and consumes this track's GitHub data.

### 7.1 — GitHub + Linear integration (read/link first)
- [ ] octocrab (issues/PRs/checks) + gh-token bootstrap else Device Flow; Linear SDK (PKCE/key, 24h refresh); `integration_connections` (keychain); reads cached as projections; integration-failure contract (§17).
- [ ] **proj_pull_request projector body (re-homed from 1.2, Option-A)** — fold GitHub PR sync events → `proj_pull_request` (the GitHub-authoritative synced cache w/ `pr_checked_at`; status ∈ PullRequest[11], §7.2). Table DDL pre-created in 1.2 (migration 3); this lands the projector + its event contract + tests. `(origin: 2026-06-07 P1.2)`
- [ ] **github + linear outbox-drainer adapters (re-homed from 1.3, Option-A)** — the `Destination` impls the 1.3 outbox drains: GitHub/Linear delivery (octocrab / Linear SDK) with the §17 retryable/terminal classification feeding the drainer's backoff + bounded dead-letter. The drainer framework + `drain_once` already landed in 1.3 (`343cc09`/`707845a`). `(origin: 2026-06-08 P1.3)`
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
- [ ] **Expand Command Center triage sources to include tasks** — P6.3a's Command Center sources = sessions+PRs+approvals only (no `proj_task` in Phase 6); add Task items to the CC triage groups once the Task/PlanProgress projection lands here. _(inlined from Carry-forward; origin: 2026-06-07 P6.3a)_
- [x] **Active-project model + selection + view re-rooting** ✅ **LANDED P7.3(fwd) (`86727ec`)** — UI active-project state (`active-project.ts` + `ActiveProjectProvider`/`useActiveProject`, daemon-independent — Lesson §13) + single-select ProjectSwitcher (`aria-pressed` + ✓Active) + graph re-root + Sessions filter (CC global). Resolved 6.3b Q3 + the zero-projects guard. **The dropdown-popover WIDGET ✅ LANDED — polish slice 5 (brief `026`, 2 commits `0af44a5`+L2, hash at round close):** trigger (`aria-haspopup=listbox` + active name + caret + disabled "No project") + roving `role="listbox"` popover (Arrow/Home/End/Enter/Escape; open-focuses-active; Enter/Escape return focus to trigger; click-outside closes); reuses the shared **orientation-aware** `nextTabIndex` (`a11y/roving.ts`); the §9 audit generalized to {tab-in-tablist, option-in-listbox}. Lesson §9 extended (point 5). _(origin: 2026-06-08 P6.5c → active-project landed P7.3(fwd); dropdown widget landed polish slice 5)_

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
- [ ] **brain_mcp outbox-drainer adapter (re-homed from 1.3, Option-A)** — the `Destination` impl delivering the redacted Brain outbox payload (envelope minus restricted/secret, §7.1/§13.1) the 1.3 drainer drains. Drainer framework already in 1.3. `(origin: 2026-06-08 P1.3)`
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
- [ ] **proj_plan_progress projector body (re-homed from 1.2, Option-A)** — fold plan/task events (`ImplementationPlanUpdated` + task-status changes) → `proj_plan_progress` (status ∈ Task[17] R-8). Table DDL pre-created in 1.2 (migration 3); this lands the projector + its event contract + tests. `(origin: 2026-06-07 P1.2)`
- [ ] Files: `daemon/src/workflow/parsers/` (NEW), `ui/src/views/plan/` (extended from kit)
- [ ] Cross-doc invariant: extended — PlanTask kind-scoped (Appendix A, §5.1 R-8)
- [ ] Tests: happy (parse a plan → PlanTasks with anchors); edge (ambiguous structure → raw-preview fallback); error (missing anchors degrade w/ warnings); integration (dispatch a plan task → session linked to the plan task — demo step 5).

### 9.3 — AgentTeam object + projection (modeling only)
- [ ] `agent_teams` registry + `proj_agent_team` projection over the 9-state machine (R-6); team membership/role on sessions; `active_teams` counter; graph team/lead/orchestrator/worker nodes. (Full `/team-start` orchestration is P1, §19.2.)
- [ ] **proj_agent_team projector body (re-homed from 1.2, Option-A)** — the `proj_agent_team` table DDL was pre-created in 1.2 (migration 3); this task builds the projector body over it (already in the deliverable above). `(origin: 2026-06-07 P1.2)`
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
- [ ] **notifier outbox-drainer adapter (re-homed from 1.3, Option-A)** — the `Destination` impl delivering notifier events the 1.3 outbox drains → macOS UserNotifications (redacted previews). Drainer framework already in 1.3. `(origin: 2026-06-08 P1.3)`
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

- **[RESOLVED 2026-06-07 — §15 safety-design, human-decided] 1.1 redaction sequencing = Option (a+).** 1.1 ships the **`redaction_status` column + `Redactor` trait + fail-closed gate** (never persist `redaction_status='unredacted'`) + the **high-recall token-prefix Redactor**; the **entropy fallback (OQ-SEC-2) is a BLOCKING Phase-1 task (1.7)**, not a fast-follow — chosen so secret-detection recall can't drift. Pin the §15 invariant with the fail-closed test (the load-bearing assertion). `redaction_status` on the envelope = cross-doc invariant → Appendix A / §7.1 / DATA_MODEL §2.1 atomic edit at 1.1 Step 9. `(origin: 2026-06-07 P1.1)`
- **[RESOLVED 2026-06-07 — cat-4, user-locked] Cross-language contract source-of-truth = Option A.** Rust `shared` crate = native authority (newtypes, serde-closed enums); `schemars` → **JSON Schema as a first-class, versioned, CI-diff-gated published artifact** (`shared/contracts/schema/`, `CONTRACT_VERSION`); TS Zod + Python Pydantic generated from it (drift-caught); reject-unknown end-to-end. Documented at **`ARCHITECTURE.md §5.0`** (direct anchored edit, owner-locked). This is the mechanism for ALL contract surfaces, not just 0.5. `(origin: 2026-06-07 P0.5)`
- **[RESOLVED 2026-06-08 — §17 / safety-rule-#6, human-decided] Lease fencing authority = Option B (a LIVE lease).** §17 (line 387) pinned "stale-token write → `fencing_conflict`" but left **"stale" undefined** for the *expired-but-not-yet-superseded* holder (the zombie-holder window) — and §17's daemon-crash recovery (line 384) re-derives an old `executing` action via the same "lease check." Human ruled **Option B**: authority = a **live lease** — `locks::validate_held` = owner-match AND `token == fencing_token` AND `expires_at > now`; **"stale" = expired OR superseded**. Rejected Option A (textbook supersession-only — leaves the zombie window) + Option C (defer to Phase 2 — would ship the safety semantic untested). Pinned by the 1.4 L1 test `test_expired_holder_rejected_even_if_unsuperseded`. Recorded: §17 lines 387/384 clarified + Appendix A `Lease` row + `daemon/CLAUDE.md` cross-doc row + LESSONS §6. **Phase-2 dependency (task 2.4):** the gateway must heartbeat/renew (or re-acquire) a long-running action so it doesn't self-fence on its own lease expiry. `(origin: 2026-06-08 P1.4)`
- **[RESOLVED 2026-06-08 — §6.4 contract gap, LEAD-decided (away-authority)] Add `protocol_error` to the §6.4 error-code set (Option B).** P1.5 L2 surfaced that the LOCKED §6.4 6-code set (`version_skew, frame_too_large, unknown_method, unauthorized_peer, policy_denied, precondition_stale`) has **no code for a bad-first-frame / handshake-required / protocol violation** (a non-Hello or malformed first frame). L2 shipped `unknown_method` as a safe, test-pinned placeholder; the lead ruled **Option B** (add `protocol_error`) on the user's behalf — production-grade / correct-the-first-time (the IPC contract is new at 0.9.0 + **UNCONSUMED** by the ui, so correcting before consumption is ~free). Rejected Option A (ratify `unknown_method` — imprecise; the ui + future RemoteClient couldn't distinguish a protocol violation from an unknown method). **Execute within the 1.5 round (before seal):** impl folds `protocol_error` into `IpcErrorCode` + swaps the bad-first-frame emission + re-pins test 7 + bumps **CONTRACT_VERSION 0.9.0→0.10.0** + regen + 3-way verify; orchestrator writes the §6.4 + Appendix-A + cross-doc edits hot at `/orchestrate-end`. **Away-authority transparency:** in the lead's away-log + here for the user's review; re-validated at the next user-invoked `/arch-finalize`. `(origin: 2026-06-08 P1.5 L2 Finding)`
- **[cat-4 / load-bearing — PENDING ≥6/15 DRAIN] SDK-vs-PTY primary driver for Claude (O-4 / ADR-006).** The 2026-06-15 Anthropic policy gives SDK/`-p` a separate **capped** Agent-SDK credit pool that **hard-stops with no fallback**; the interactive terminal is exempt (`RESEARCH.md:65` [VERIFIED]). This **may invert ADR-006**'s Option-C-Hybrid lean (SDK-driven primary). **Not decided agent-only.** Phase-0 0.1 LANDED the measurable half: **decision criterion + both branches recorded** (`docs/spikes/OQ-HARN-SPIKE-7.md §3`); **#27203 confirmed** present on CC 2.1.168 (bg subagents stay forbidden — §9.1 unchanged). **Still open:** the deciding **drain measurement = HITL checklist** (`§5`), user runs ≥ 6/15**. Orchestrator carries the resolved call back to the lead/user with the drain data. **Blocks** freezing any 0.5 supervision-touching contract surface. `(origin: 2026-06-07 P0.1)`
- **[tracked — Phase 10, not blocking] D14 demo-viability contradiction.** Brain-optional (design) vs demo-mandatory (PRD §25) vs SDK-can-halt (credit-pool) are in tension for the end-to-end demo. Carry as a known Phase-10 concern; resolve before the integration/deploy gate. `(origin: 2026-06-07, D14 audit)`
- **[RESOLVED 2026-06-08 — §17/§15 audit-integrity safety call, USER-RULED Option C] Corrupt-event-row recovery on `open` (the deferred 1.2 L3 Finding). Unblocks 1.6c (the §17 degradable-replay sub-slice).** **USER RULING: Option C** — on a corrupt event row in `catch_up_replay`: quarantine (preserve) the raw row + emit a loud audit-integrity event + mark the offset degraded + continue (daemon starts); never *silently* drop; raw spine untouched (§17 as written). Adds the audit-integrity event type → CONTRACT_VERSION bump in 1.6c; security-reviewer applies. 1.2's `catch_up_replay` rebuilds projections via the *strict* read path, so one corrupt event row aborts `open()` → daemon won't start. Audit-integrity dimension: a naive skip could *silently* drop a real event. **What §17 already says (locked):** eventstore row — *corrupt `payload_json` → quarantine + audit-integrity event*; *unknown version → store raw + degraded marker, don't crash*; projections row — *mark offset degraded, skip bad event, never corrupt raw events.* Options:
  - **A — silent skip-and-continue:** ✗ violates audit-integrity (projection silently diverges from true history, no record). Reject.
  - **B — refuse-to-start + restore `.bak`:** ✓ integrity, ✗ availability (one corrupt row → daemon won't start), ✗ may lose more events than it saves (stale pre-migration `.bak`). This is §16's *migration-failure* semantics mis-applied to a corrupt event row.
  - **C — quarantine + audit-integrity event + continue (= §17 as written). ⭐ ORCHESTRATOR RECOMMENDS.** Quarantine (preserve) the corrupt raw row, emit a loud audit-integrity event recording the gap, mark the offset degraded, continue replay, daemon starts. No *silent* drop · daemon starts · raw spine untouched. **Rationale:** for an audited single-mutator system the only unacceptable failure is *silent* divergence — C makes the gap loud + recorded + recoverable; it is **conformance to locked §17, not a fresh architectural call** (surfaced because it is audit-integrity-sensitive). Cost: adds an audit-integrity event type → CONTRACT_VERSION bump in the 1.6c sub-slice. `(origin: 2026-06-08 P1.2 L3 Finding; lead-parked-for-user 2026-06-08)`
- **[RESOLVED 2026-06-08 — INV-SEC-1/§15 single-mutator safety call, USER-RULED Option B] Cold-start Device/LocalRunner registration mechanism. Unblocks 1.6a-L3 (the registration layer).** **USER RULING: Option B** — direct system-event append via the write-actor (`actor_type=System`), through the §15 redaction gate + projector fold (immutable + audited), NOT policy-gated. **Both sub-leans ADOPTED:** (i) reserved system-workspace sentinel + (ii) `DeviceId`/`LocalRunnerId` `minted_id!` newtypes off `DesktopObjectKind` (NOT new `IdKind` variants). Adds `DeviceRegistered`/`LocalRunnerRegistered` → CONTRACT_VERSION bump in 1.6a-L3; security-reviewer applies (INV-SEC-1-touching). §16 puts desktop-host `Device` + per-start `LocalRunner` registration at bootstrap step (2) — **before the UI connects and before the Action Gateway exists (Phase 2).** INV-SEC-1: "*no code path mutates … session state except via a typed Action that passed policy+approval.*" Options:
  - **A — Gateway Action:** ✗ circular (Gateway is Phase 2) → forces registration to defer to P2, blocking the §16 cold-start ordering. Reject for P1.
  - **B — direct system-event append via the write-actor (`actor_type=System`). ⭐ ORCHESTRATOR RECOMMENDS.** Bootstrap self-registration is the daemon establishing its *own* runtime identity — substrate (like the event log recording system facts / `SessionStarted` in 1.2), not an intent from an untrusted proposer (agent/Brain/UI) mutating external state. It still produces an immutable, **redaction-gated, audited** event (the §15 append gate + projector fold both run); it is simply not a *policy-gated Action* because there is no untrusted intent to gate. **INV-SEC-1 governs proposer-driven mutations; the daemon's own lifecycle events are the layer those Actions are recorded *into*.** Cost: adds `LocalRunnerRegistered` + `DeviceRegistered` event types → the CONTRACT_VERSION bump 1.6a-L3 already carries.
  - **Two contract-shaping sub-decisions that apply IF Option B (surfaced at 1.6a Step-2.5; settle with the mechanism call):** (i) **`workspace_id` on bootstrap events** — the envelope's `workspace_id` is NOT NULL, but cold-start runs before any workspace. Orch lean: a **reserved system-workspace sentinel** (a well-known `ws_` constant for System-actor lifecycle events — the "system scope" pattern), NOT minting a host-workspace (conflates Workspace=project-container with the daemon host) and NOT a nullable-column schema change to the frozen envelope. Sets the workspace-less-event precedent. (ii) **`lr_`/`dev_` id newtypes** — no `DeviceId`/`LocalRunnerId` newtype exists (the 22 `IdKind` = 16 minted + 6 external; the 4 desktop prefixes live only on `DesktopObjectKind::id_prefix()`). dev_/lr_ are **consumer-facing** (the UI reads LocalRunner). Orch lean: add `DeviceId`/`LocalRunnerId` `minted_id!` newtypes in `shared/` off the frozen `DesktopObjectKind` prefixes (keyed via `DesktopObjectKind`, NOT new `IdKind` variants — don't touch the frozen-22), rather than the bare-String `out_` precedent (which is for genuinely-internal ids). (Also an L3 build wrinkle, non-user: object_refs must source the lr_/dev_ id from the payload since it isn't an envelope typed column.) `(origin: 2026-06-08 P1.6a-L3; lead-parked-for-user 2026-06-08)`
- _(Phase 0 also populated §18 perf numbers (0.4 → written into `ARCHITECTURE.md §18`). §13.1 Brain transport still resolves later.)_

---

## Architecture-doc corrections (applied as direct edits)

> **`/arch-finalize` is user-invoked, on-demand — NOT an auto-recurring batch** (user, 2026-06-07). It produced the binding `ARCHITECTURE.md`; there is no scheduled "next run" to queue work into. Locked-decision records + factual corrections + numbering fixes land as **direct atomic anchored edits** (Step-9 arch-doc-note hot-routing), committed with the round. A full `/arch-finalize` **re-validation** is a deliberate user-invoked gate, natural at **Phase-0 exit before Phase 1**.

Applied this round (2026-06-07), committed atomically with the round commit:
- [x] **`ARCHITECTURE.md §5.0` — Contract source-of-truth & propagation** written (Option A, owner-locked). The contract *format* is now itself contracted. `(P0.5)`
- [x] **`ARCHITECTURE.md §9` — libgit2 relative-worktree read-gap CLOSED.** Empirical (0.3 / `OQ-INT-SPIKE-6.md`): libgit2 ≥ 1.9.4 (git2 0.21) **CAN** fully read `extensions.relativeWorktrees` repos; the §9 aside corrected. Dual-git posture UNCHANGED (CLI for ALL mutations + the sparse-checkout fallback retained). `(P0.3)`
- [x] **OQ-HARN-SPIKE numbering reconciled.** Canonical = **`-7`** (used across the finalized artifacts: gap-audits/D06, ui-review/E-netnew, MVP_TASKS, the spike file). `OPEN_QUESTIONS.md` `-2` (legacy /arch-draft) cross-ref'd to `-7`. `(P0.1)`
- [x] **Appendix A + `daemon/CLAUDE.md` cross-doc rows + LESSONS §2** — the 0.5 frozen contracts recorded (cross-doc invariant routing). `(P0.5)`

**Upstream PLANNING-doc reconciles — ✅ SWEPT at the Phase-0-exit `/arch-finalize` re-validation (2026-06-07).** The binding `ARCHITECTURE.md` was already correct + AHEAD; these /arch-draft + upstream-spec drafts were swept FORWARD to match it. **A 5-dim adversarial gap audit confirmed NO frozen `shared/` contract moved (0 release-blockers) → no 0.5b forced; the UI track is NOT gated by drift** (`docs/gap-audits/R2-phase0-exit-revalidation.json`).
- [x] `docs/planning/DECISIONS.md` **ADR-007** — stale "libgit2 can't do relativeworktrees (fix unreleased)" premise reconciled to the §9 correction (git2 ≥1.9.4 reads `relativeWorktrees`; sparse-checkout misreport is the only retained CLI-read fallback; mutations CLI-only regardless). `(P0.3)`
- [x] `docs/planning/DATA_MODEL.md` **§4** marked SUPERSEDED → §5.1 (10 machines): `ready_for_team_mode`→`ready_for_team_run` (R-7); §4.2 Task-vs-PlanTask resolved (R-8); §4.7 Approval R-5 split noted (ActionRequest = the new execution machine). `(P0.5)`
- [x] **EM §7** (`docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md:469,485`) actor enum **and** example `remote_device` → **`remote_client`** (R-2) + reconciliation note; DATA_MODEL §6/§8-Q5 flag resolved. `(P0.5)`
- [x] `DATA_MODEL.md` **§6.4** EventProjection prefix `prj_` → **`eprj_`** (frozen value, de-collided from `proj_`). `(P0.5)`
- [x] **Ratify `§5.0`** — re-scrutinized + **upheld unchanged**; the frozen `shared/` crate matches it byte-for-value (CI schema-diff gate green); ratification stamp added to §5.0 + Appendix A flag resolved. `(P0.5)`
- [x] **`ARCHITECTURE.md §5.1` header** reconciled "Nine" → "**Ten** machines (8 draft + **ActionRequest** [R-5] + **AgentTeam** [R-6])". ⚠️ The count has **two meanings** — **9 = frozen in 0.5** (ExecutionProfile held) · **10 = canonical §5.1 / UI-binding set** — so this was swept **per-occurrence, NOT uniformly**: §11.3 + the Phase-6 UI lines (6.2/AC-6) + UI_RECONCILIATION → **10**; freeze-scoped refs (0.5 line, briefs/002 freeze test) **stay 9**. `(P0.5)`

**Additional stragglers the 6-item list missed (found + swept by the re-validation gap audit):**
- [x] **DATA_MODEL §2.3 + §6.4** "8 MVP projections" → **10** (added PullRequest §7.2 + AgentTeam R-6). `(STR-02/03)`
- [x] **DATA_MODEL §2.9** `action_requests.status` column comment was pre-R-5 → corrected to the frozen 15-value ActionRequest set. `(STR-01)`
- [x] **DECISIONS.md ADR-004** "newline-framed JSON-RPC" → length-prefixed only (§6.4); "SO_PEERCRED" → `getpeereid()` (macOS; SO_PEERCRED is Linux-only — CLAUDE.md safety rule #7). `(STR-04)`
- [x] **DATA_MODEL §5** ID-format `[PROPOSED]`→`[LOCKED]`; **§8 Q1/Q5/Q7** + **OPEN_QUESTIONS** OQ-DATA-SPIKE-5/OQ-DATA-6 (→ frozen 06f9576), OQ-INT-SPIKE-6/OQ-INT-9 (→ libgit2 read-path resolved; octocrab spot-check still open), OQ-DATA-7 (→ R-8). `(STR-05/06/07)`
- [x] **SHARED_OBJECT_MODEL.md** supersession pointer added (§5.1/§7.1 authoritative for state machines + actor enum); **UI_RECONCILIATION.md** "9"→"10". `(COMP-1/COMP-3)`
- [ ] _Intentionally LEFT (historical, non-binding — tasks-gen binds to `ARCHITECTURE.md`):_ `ARCHITECTURE_DRAFT.md` + `PRESEARCH.md` (Brain-1 rough drafts, self-labeled non-binding) + `docs/ui-review/B-status.json` (point-in-time audit record). `(STR-08/09)`

---

## Log

Append-only, date-stamped, the orchestrator's framing of each round.

### 2026-06-08 — Phase 1.6 (round 1): cold-start bootstrap (1.6a-L1/L2) + daemon runtime (1.6b)

- **Completed:** **1.6a-L1/L2 + 1.6b LANDED** (fresh team, away-cycle #1). 1.6a: `48b7a2c` (typed `user_version()→Result` + non-swallowed `RestoreFailed` via a pure `on_migration_failure` classifier) · `f1de088` (`cold_start` orchestration + `DaemonContext` + `DaemonVersionInfo`). 1.6b: `26898a8` (write-actor + `#[tokio::main]` `main.rs`) · `9448035` (drainer/reaper interval loops + `DRAIN_BATCH_LIMIT` bounded pass) · `8d06ed7` (UDS bind + accept-loop + concurrency cap) · `f9c31b1` (broadcast publish-after-commit subscribe-source) · `39f756e` (cq review fixes). **105 workspace green; security-reviewer PASS both slices** (forbidden #3 sole-writer + publish-after-commit + reader-never-blocks-writer; rule #7 through the accept path). **Daemon-internal — CONTRACT_VERSION stays 0.11.0.** Session doc `006`; LESSONS §8 (cold-start ordering + FS-failure determinism) + §9 (runtime/write-actor). `main.rs` closes the 1.3/1.4/1.5 "wire-the-runtime" deferral chain.
- **Scope — 1.6 SPLIT 3-way (+1.6d):** orchestrator-scoped, lead-nodded (driver = "never bundle a safety-critical slice"): **1.6a** cold-start bootstrap (brief 027) · **1.6b** runtime-task spawns (brief 028) · **1.6c** §17 degradable replay (brief 029, atomized safety sub-slice). **Scope finding:** `EventStore::open`+`migrations.rs` already do migrate/backup-rollback/floor (1.1), and `DegradableEvent`/`read_all_degradable` already exist (1.1/1.2) — so 1.6a's real work was the *orchestration*+registration+version-info; 1.6c WIRES the existing primitives + adds the audit-integrity event.
- **Decisions made:** **Write-actor = dedicated OS thread + mpsc `WriteHandle`** (user-ratified Q1) — concretization within the locked single-write-actor (LESSON §3), not a new arch call. Bounded drain = const `DRAIN_BATCH_LIMIT` 128. Stale-socket reclaim = unlink-before-bind. Lag policy = subscriber-detects-`Lagged`+resync (no wire frame → no CONTRACT bump). Restore-failure tested via a pure classifier (determinism-for-testability).
- **Findings + escalations (2 cat-1 — lead-parked, then USER-RULED):** **§17 corrupt-row = Option C** (quarantine + audit-integrity event + continue; = §17 as written; gates 1.6c) · **registration = Option B** (daemon self-registration = a System-actor system-event append via the write-actor — audited + redaction-gated, NOT a policy-gated Gateway Action [circular pre-Phase-2]; gates 1.6a-L3) + 2 adopted sub-leans (reserved `SYSTEM_WORKSPACE_ID` sentinel for workspace-less System events; `DeviceId`/`LocalRunnerId` `desktop_minted_id!` newtypes off `DesktopObjectKind`, NOT new `IdKind`). The lead conservatively parked both for the user (refused to rule INV-SEC-1/audit-integrity on paraphrases under budget); user ruled C/B. Both resolved in Decisions-tabled.
- **Scope shifts:** **1.6a-L3** (registration) + **1.6c** (§17 replay) HELD for next cycle (design blessed + recorded in session doc 006 "Resumption" for instant rebuild). **1.6d** (#13) carved from 1.6b L4 — the socket subscribe-SERVE push (broadcast SOURCE live; serve-layer wiring deferred rather than rush a 1.5-safety-adjacent signature change under budget). Carry-forward: +1.6b deferrals (subscription_id / cfg-guard / `"SessionStarted"`-const dup / JoinSet-shutdown); −1.1-L3 §16 restore-typing (✅ done 1.6a-L1).
- **Process note (orchestrator self-correction):** at the post-1.6b ctx-check (ACTION, lead=79%) the orch briefly over-rotated to *forcing* a close-out, then corrected to the per-slice protocol (orch sends the tier line + dispatches the next slice; **the lead adjudicates the cycle**). The lead then explicitly ruled close-out at the clean 1.6b boundary — complied; 1.6a-L3 (just opened, Step-2.5 only) cleanly held + reverted. Reinforced: a deferral concern goes to the lead, never a direct-to-impl countermand of a lead/user close-out.
- **Round close trigger:** **lead ACTION-tier (79%) + user-signaled** cycle at the clean 1.6b boundary. Impl `/session-end` (doc 006) → orch `/orchestrate-end` + push → lead `/team-end`.
- **Next session target:** **1.6a-L3** (registration, Option B; closes #11) → **1.6c** (§17 replay, Option C) → **1.6d** (subscribe-serve, #13) → **1.7** (redactor entropy fallback) — all unblocked; briefs 027/028/029 ready; rulings baked.
- **Reference:** implementer session doc `006-2026-06-08-cold-start-bootstrap-and-runtime.md`; briefs `027`/`028`/`029`.

### 2026-06-08 — Phase 1.5: UDS GatewayPort transport + handshake (read surface)

- **Completed:** **1.5 LANDED** (4-commit: `ae27f1d` framing + getpeereid peer-auth [rule #7] / `fb941e8` handshake + the `shared/` IPC contract @ 0.9.0 / `c1c54e5` JSON-RPC dispatch + read methods + the ratified `protocol_error` @ 0.10.0 / `6690e41` subscribe + `ServerFrame` frame-mux @ 0.11.0). 15 ipc tests; workspace 83 green; rule #7 (getpeereid) + single-writer (read-only WAL) security-PASS'd; session doc `005`. The §6.1 **read surface** (get_projection/subscribe/get_capabilities) + the full §6.4 wire contract.
- **Decisions made:** **getpeereid peer-auth FIRST** (rule #7; NOT SO_PEERCRED; isolated unsafe + fail-closed; the *enforcement* path test-pinned via an injectable peer_uid — orch ADD). **Two version axes:** `CONTRACT_VERSION` (the §5.0 schema/codegen artifact, 0.8.0→0.11.0 across the slice) vs `protocol_version` (the wire handshake; `SUPPORTED_PROTOCOL_RANGE {1,1}` daemon-authored). **Canonical projection-name enum = the architecture labels** (orch TWEAK: `UsageLedger`, not the ui's provisional `Usage` — 5/6 ui names already matched; the ui reconciles its outlier). **`ServerFrame` frame-mux = an internally-tagged JSON discriminant** over the unchanged codec (rejected the 1-byte binary prefix — premature for a reserved channel). tokio + libc confirmed **architecture-mandated** (stack table + §12 + ADR-004; first async surface). Read-only WAL reads (single-writer); closed-enum table map (no SQL injection).
- **Findings + escalations:** **§6.4 error-code gap → lead-ratified Option B (away-authority): added `protocol_error`** (the LOCKED 6-code set had no bad-first-frame/handshake/malformed/bad-params code; `unknown_method` reserved for a genuine unknown method NAME). Folded within the round (0.9.0→0.10.0→0.11.0). Recorded in Decisions-tabled + the §6.4/Appendix-A edits.
- **Scope shifts (ship-the-mechanism / wire-at-1.6 pattern):** the whole IPC server's production wiring → **1.6** — `bind()`/accept-loop spawn (the §16 "bind UDS"), the live subscribe delta-source (writer→broadcast→subscriber), the platform cfg-guard (getpeereid macOS-only), the accept-loop concurrency cap, per-session read-only conn. **Deferred far-future (recorded for the consuming team):** `get_projection` `scope.project_id` enforcement (post-MVP — accepted-but-not-enforced, test-pinned) → ui-scoping consumer; `SELECT *`→column-allowlist hardening before a sensitive projection column lands; the §6.4 `ServerFrame` **Terminal-variant encoding** (JSON-base64 vs binary fast-path) → **Phase 3** (Terminal Channel, decided with throughput data).
- **Cross-track:** the daemon side of the ui's pre-pinned `GatewayPort` contract — part (a) `SUPPORTED_PROTOCOL_RANGE {1,1}` **CONFIRMED** (Carry-forward `ui↔daemon-1.5` updated 🟢); the ui regens against 0.11.0 + adopts ServerFrame/ProjectionDelta/protocol_error/`UsageLedger` + swaps MockGatewayPort→real at the ui-track resume (cross-track-timed). Carry-forward re-triaged at the merge re-orient (2 deleted, 10 spread). Doc-numbering namespaced (daemon 001+/007; ui 022+).
- **⚠️ 1.6 scope note:** 1.6 has accumulated substantial runtime-wiring scope (outbox drainer + reaper + IPC accept-loop + subscribe-delta-source spawns; pidlock cold-start; the §17 corrupt-row degradable replay; migrations backup/rollback + version-compat) — the fresh 1.6 team should consider **splitting** (e.g. 1.6a cold-start/bootstrap/migrations + 1.6b runtime-task spawns + the §17 replay).
- **Round close trigger:** **away-authority context cycle** — daemon-implementer at 70% [WARN] mid-L4; proactive close-out at the clean 1.5 boundary so 1.6 (big) starts on a fresh budget. Both teammates cycle; the lead spawns a fresh team for 1.6.
- **Next session target:** **1.6 — first-run bootstrap + runtime-task spawns + §17 degradable replay** (ON HOLD pending the fresh-team spawn).
- **Reference:** implementer session doc `005-2026-06-08-uds-gatewayport-transport.md`; brief `007-P1-5-uds-gatewayport-transport.md`.

### 2026-06-08 — Phase 1.4: lease locks + fencing + pidlock + reaper

- **Completed:** **1.4 LANDED** (3-commit: `b3f7612` `leases` table [migration 5] + primitive + monotonic fencing + Option-B `validate_held` authority / `442149a` pidlock single-instance [std advisory file lock] / `0347ac6` reaper `reap_once` + restart-survival). 11 lease/pidlock/reaper tests; workspace 67 green; security-reviewer PASS every layer; session doc `004`.
- **Decisions made:** **Fencing authority = Option B (a LIVE lease)** — human-ratified safety-rule-#6 call (escalated mid-Step-2.5): `validate_held` = owner + token-match + `expires_at > now`; "stale" = expired OR superseded (rejected supersession-only A / defer-to-P2 C). `fencing_token` = persisted monotonic high-water mark per `(resource_id, lease_kind)` (release/reap keep it; survives restart). pidlock = std `File::try_lock` (OS-fd-held, auto-released on death, immune to PID reuse — no `kill(pid,0)`; zero new deps on the 1.93 toolchain). `leases` daemon-internal → no CONTRACT_VERSION bump (stays 0.8.0). `validate_held` subsumed `is_current_token` (single oracle). LESSONS §6.
- **Cross-doc reconciles:** Appendix A `Lease` row + `daemon/CLAUDE.md` cross-doc row written; §17 lines 387/384 clarified ("stale = not a live lease"); **DATA_MODEL §2.6 drift caught + reconciled** — the sketch had PK=`resource_id` + all-NOT-NULL holder cols + a single global counter; as-built is composite PK `(resource_id, lease_kind)` + NULLable holder fields + per-key high-water (the composite PK resolves the latent exclusive-vs-shared conflict; designed in brief 006 Q2, approved at Step-2.5). Brief 006 test-number map reconciled.
- **Scope shifts:** Phase-2 gateway deps → task 2.4 (use `validate_held` as the fencing oracle; heartbeat/renew long actions to avoid self-fence; same-owner idempotent-acquire contract; strict expiry-boundary). 1.6 captures: reaper Tokio spawn (joins outbox drainer) + pidlock cold-start call site. Carry-forward Step-5.5 triage: 3 spread (consumer-marked), 3 kept, 0 deferred.
- **Process:** safety-rule-#6 fencing-semantics **escalated to the human** via the lead (AskUserQuestion A/B/C) before Step-2.5 sign-off → Option B; added `validate_held` + the expired-holder test as `ADD:`. Layer-drive held — no wake-gap across L1→L2→L3. Orch **re-verified the suite** at L1 (impl's reported "44" was a partial count → actual 62) + at slice-complete (67) before authoring commits.
- **Round close trigger:** **user-on-demand** (via lead) — seal the 1.4 round + push at this clean boundary so the lead can merge `track/ui` → main before 1.5.
- **Next session target:** **1.5 — UDS GatewayPort transport + handshake** — **ON HOLD** pending the `track/ui` → main merge; `/orchestrate-start`-refresh against merged main before authoring brief 007.
- **Reference:** implementer session doc `004-2026-06-08-lease-locks-and-fencing.md`; brief `006-P1-4-lease-locks-and-fencing.md`.

### 2026-06-08 — Phase 1.2 (projections) + 1.3 (outbox) — read-model + reliable side-effects

- **Completed:** **1.2 projection engine LANDED** (3-commit: `c8c6c72` schema+AppendIntent identity fields + migration 3 [10 proj_* + object_refs + projection_offsets] / `a1dc482` in-band apply [4 feedable projectors + `SessionStarted`/0.8.0 + §15 regression guard] / `e66c659` recovery [catch-up replay + full rebuild + degraded-skip]). **1.3 transactional outbox LANDED** (2-commit: `343cc09` table+in-txn-write+§15-sync-sink-gate / `707845a` drainer [at-least-once, backoff, retryable/terminal, bounded dead-letter] + crash-recovery + jsonl_mirror). 56 tests; security PASS §15/§17/§7.2 on every layer; session doc `003`.
- **Decisions made:** **Projections fold in-band in the event-commit txn** (locked by §7/§2.4), savepoint-isolated per projector (logic error degrades+skips; Db error fails the append closed); offsets advance same-txn (never ahead of rows); status columns bind the frozen §5.1 enums (LESSONS §4). **object_refs are DERIVED** from typed fields, not caller-supplied (rebuild-safety — impl finding, corrected the brief default). **`SessionStarted` = first EventTypeRegistry payload in `shared/`** per §5.0 → CONTRACT_VERSION 0.8.0. **Outbox = the §15 sync sink** (payload from the already-redacted event; LESSONS §5); `out_` id daemon-internal (not a frozen contract ID); at-least-once via in_flight-claim-before-deliver + reset-on-open.
- **Scope shifts (Option-A sequencing, human-ratified):** 1.2 ships the engine + all 10 table DDLs + the 4 Phase-1-feedable projector bodies; **6 projector bodies re-homed** to producing phases (ApprovalQueue→2.1, UsageLedger→3.1, Worktree→5.2, PullRequest→7.1, PlanProgress→9.2, AgentTeam→9.3). 1.3 ships the engine + jsonl_mirror; **4 destination adapters re-homed** (brain_mcp→8.1, github/linear→7.1, notifier→10.2) + the drainer Tokio-spawn/bounded-pass→1.6. All origin-marked (nothing dropped).
- **Findings:** **§17 corrupt-row-on-`open`** — `catch_up_replay` reconstructs via the strict read path → one corrupt event row aborts `open` (daemon-won't-start), contradicting §17's degrade/quarantine read path. Both reviewers HIGH; not producible by current code (writer can't persist a corrupt row). **Human-ratified defer (Option A) → task 1.6** (decide skip-vs-refuse semantics + audit-integrity; bundle the replay-time legacy-`unredacted` quarantine-skip).
- **Process:** 3rd ship→next-layer wake-gap (1.3 L2→L3) caught by the lead's watchdog; `drive-multicommit-slices` lesson sharpened to 3 mechanisms (fold next-layer into SHIP + re-wake on any "proceeding" + roll straight into next-layer RED). Per-LAYER context monitoring held (no repeat of the 1.1 86% blowout).
- **Round close trigger:** **proactive cycle** at the clean 1.3 boundary (impl 63%, below ACTION 75%) — recommended by orch + approved by the lead/user — so 1.4 runs on a fresh budget instead of cycling mid-slice.
- **Next session target:** **1.4 — lease locks + fencing tokens + single-instance** (`leases` table + monotonic fencing + pidlock + reaper; deps 1.1 met). ui track remains the user's to open (ungated).
- **Reference:** implementer session doc `003-2026-06-08-projections-and-outbox.md`; briefs `004-P1-2-projections.md` + `005-P1-3-outbox.md`.

### 2026-06-07 — Phase 1.1: event store (the trust-core spine) + HARD-STOP cycle

- **Completed:** **1.1 event store LANDED** as a 3-commit slice — `df753aa` (§7.1 envelope contract + source/sensitivity/visibility enums, 0.6.0) + `b998f20` (root Cargo workspace + `daemon/` crate + single-writer WAL store: atomic `seq` in `BEGIN IMMEDIATE`, read-only WAL readers, `user_version` migrations + §16 backup/rollback, injectable Clock/IdGen, degrade/quarantine on read §17) + `61089ea` (**§15 redaction gate**: `redaction_status` + `RedactionStatus` + `Redactor` trait + fail-closed writer gate + prefix Redactor + migration 2, 0.7.0). 31 workspace tests + 3-way verify; security-reviewer PASS on every invariant.
- **Decisions made:** **§15 redaction sequencing = Option (a+)** (human-decided) — `redaction_status` column + fail-closed gate + prefix Redactor in 1.1; the entropy fallback (OQ-SEC-2) made a **BLOCKING Phase-1 task (1.7)** so recall can't drift. Build-layout: a **root Cargo workspace** (shared+daemon). `CONTRACT_VERSION` = semver-on-contract (0.5.0→0.6.0→0.7.0). `source_type` declared **closed** (15, EM §8 "Examples"→closed). `causation_id` typed `EventId`.
- **Scope shifts:** entropy fallback + quarantine→`SensitiveOutputRedacted` + §15 property/fuzz test → **1.7**; §16 migration restore-failure typing → **1.6**; AppendIntent-grows + object_refs table → **1.2**. None weaken the shipped §15 gate (the blocking bar).
- **Process:** 1.1's multi-commit cadence surfaced two team-comms lessons — (a) I second-guessed an in-flight freeze-once/then-extend directive (4 crossed messages; resolved by anchoring on the committed artifact); (b) the impl idled after each layer commit (wake-gap ×2) → orchestrator must **drive layer→layer** (banked to auto-memory). LESSONS §3 (single-write-actor + atomic seq). `cargo fmt --check` added to the preflight gate (1.1-prior).
- **Round close trigger:** **HARD-STOP context cycle** — daemon-implementer hit **86%** mid-slice (the per-slice gate missed a long multi-commit climb). 1.1 finished clean first (slice-atomicity), then both teammates cycle (impl `/session-end` → session doc `002`/`59d39ad`; orch `/orchestrate-end` = this round commit + push). Fresh successors spawn; **1.2 halted** for them.
- **Next session target:** **1.2 — projection engine + MVP projections + offsets** (deps 1.1 met; consumes the event log, fills the AuditTrail FTS5; folds in the Carry-forward 1.2 items). The **ui track** remains the user's to open (parallel, ungated).
- **Reference:** implementer session doc `002-2026-06-07-event-store.md`; brief `003-P1-1-event-store.md`.

### 2026-06-07 — Phase 0: spikes + 0.5 contract freeze (the serial neck)

- **Completed:** all 4 Phase-0 spikes (0.1–0.4) resolved with recorded decisions (`docs/spikes/*.md`); **0.5 shared-contract freeze landed (06f9576)** — the serial neck. Single-writer holds (§18 written MEASURED, 12–19× headroom); git2 reads relative-worktrees (§9 corrected); #27203 confirmed (bg subagents stay forbidden); contracts frozen Option-A in `shared/` (9 status machines, 22 IDs + prefix map, actor enum, 4 desktop objects).
- **Decisions made:** **contract SoT = Option A** (cat-4, user-locked) — Rust authority → first-class versioned CI-diff-gated JSON Schema → generated Zod/Pydantic (recorded `§5.0`). 10 new ID prefixes defined + ratified (orchestrator TWEAK de-collided `art_`→`artf_`, `prj_`→`eprj_`). §18 budgets MEASURED + committed (+ §14 CI guards).
- **Scope shifts:** **ExecutionProfile runtime-state enum held → 0.5b** (excluded from the freeze pending cat-4). HITL items staged (notarization, credit-pool drain ≥6/15, codex schema capture) — none block Phase 1.
- **New blockers / open questions:** cat-4 **SDK-vs-PTY** pending the ≥6/15 credit-pool drain; **D14 demo-viability** contradiction tracked for Phase 10.
- **Convention fixes:** `cargo fmt --check` added to the daemon preflight gate (Step-8 was clippy-only → 06f9576 needed fmt follow-up 407be7c). Banked LESSONS §1 (broken cargo shims) + §2 (wire-value-is-contract / §5.0 SoT pattern). Architecture corrections direct-edited (§5.0, §9, §18, Appendix A); planning-doc reconciles queued for the Phase-0-exit `/arch-finalize`.
- **Next session target:** **user-invoked Phase-0-exit `/arch-finalize` re-validation** (sweep the planning-doc reconciles) → then fan-out: **Phase 1 / 1.1** (event store, daemon-core) ∥ **ui track**. **HELD on 1.1** until the re-validation completes + any moved contracts are reconciled into `shared/` (a 0.5b if frozen enums/IDs/§5.0 shifted).
- **Reference:** implementer session doc `001-2026-06-07-phase0-spikes-and-contract-freeze.md`; commits 06f9576 (0.5) + 407be7c/94e4894/2bf198d/4f572dc (close-out).

### 2026-06-08 — UI track: Phase 6 shell + status binding + Command Center (round 1)

- **Completed (planning level):** opened the `ui` track on `track/ui` and built **6.1 (shell)** + **6.2 (status binding)** to completion and **6.3a (Command Center)** — 6 `/tdd` slices, all test-first, **51 tests green**, tsc/oxlint/vite-build clean. Built entirely against the frozen `shared/` 0.5.0 contract + a `MockGatewayPort` + fixtures + the `NexusOps-ui-kit` — the parallel-track plan validated (no daemon dependency consumed; real `UdsGatewayPort` integrates at daemon-1.5). Commits: `fd9738b`/`39a87c6`/`402f4c5` (6.1a/b/c), `b32c3c0`/`e2cebbc` (6.2a/b), `144b6b6` (6.3a), `de5b71d` (session doc).
- **Decisions made:** contract enums **generated** from the frozen schema (checked-in artifact + drift + `CONTRACT_VERSION` pins), never hand-declared; kit consumed via **`@ui-kit` source alias + `resolve.dedupe`** (not the global bundle/vendored); read-only gate is **fail-safe + defense-in-depth** (canSubmitIntent = connected && version-compatible; daemon Gateway remains the INV-SEC-1 guard, security-reviewer PASS); attention-rank table is **UI render policy in `ui/src/status/`** (keyed by (machine,status), drift-pinned, no fall-through to idle); Command Center `changes_ready` extracted (disjoint, surfaced first). Banked `ui/LESSONS.md §1–6`.
- **Escalations resolved:** **object-key strictness posture RATIFIED (human): A-now → strict-at-freeze** (tolerant reads now, `.strict()` at the daemon object-schema freeze; enum reject-unknown stays strict). User-directed clean round close-out at the 6.1+6.2+6.3a boundary (before the heavier 6.3b).
- **Scope shifts:** decomposed 6.1 → 6.1a/b/c, 6.2 → 6.2a/b, 6.3 → 6.3a–e (orchestrator decomposition latitude). `attention.ts` resited `shared/` → `ui/src/status/` (UI render policy, not frozen contract). Carry-forward triaged (1 deleted / 4 inlined to 6.4+7.3 / 3 spread / 3 kept = the 6.3b working set).
- **New blockers / open questions:** none new. Cross-track integration points tracked as spreads (object-schema-freeze reconcile, daemon-1.5 `UdsGatewayPort` + `SUPPORTED_PROTOCOL_RANGE`, §5.0 CI gates). ExecutionProfile (0.5b) + the closed-kit-props §11.7 a11y item inlined to their phases.
- **Next session target:** **6.3b — Project Graph + a11y list/table fallback** (the §11.6 MUST), then 6.3c–e.
- **Reference:** implementer session doc `ui-001-2026-06-08-phase6-shell-status-command-center.md`; briefs `docs/briefs/003–008`.

### 2026-06-08 — UI track: Phase 6 round 2 (Graph/Sessions/a11y/Usage/Settings/Survival/TopBar) + Decision-C + two Findings

- **Completed:** **7 ui slices** on `track/ui` (`347c31f..823d16e`), all test-first, **116 tests green**, tsc/oxlint/vite-build clean throughout: **6.3b** Project Graph + §11.6 list/table a11y fallback (2-commit layer slice) · **6.3c** Sessions dense sortable table · **6.4a** a11y MUSTs (global focus-visible ring + `useReducedMotion` + the multi-view §9 reachability merge-gate audit) · **6.4b** Usage dashboard (forbidden #4 Codex-`"unknown"` + #5 non-color credit-pool) · **6.4c** Settings tablist + Usage relocation · **6.4d** Survival/recovery display (distinct RecoveryBanner + parked restart + resume-mode indicator) · **6.4e** TopBar §11.2 nav reconcile + accessible-names. Built entirely against frozen `shared/` 0.5.0 + MockGatewayPort + fixtures + the kit (no daemon dependency consumed); **no frozen contract field changed** (only provisional banner-marked shapes → reconcile spread).
- **Decisions made:** **Decision C (human-delegated on "pick the architecturally correct choice")** — the 6.3 tail (6.3d Terminal+permission-card, 6.3e Code/Diff) crosses the daemon-1.5 mutation/intent + Terminal-Channel boundary; the permission card is the UI's **FIRST mutation path** (INV-SEC-1/forbidden #6) → **build the intent seam ONCE against the real frozen contract, never provisionally.** Parked 6.3d/e + the intent seam; reordered to the daemon-independent 6.4 work. Banked `ui/LESSONS.md §7` (multi-commit layer-driving) + `§8` (mapper + namespaced-locator) + `§9` (a11y foundation) + `§10` (green-tests ≠ looks-right / visual gate) + a **§6 refinement** (a closed-prop kit control's accessible NAME = a visually-hidden child INSIDE it, never a wrapper `aria-label`).
- **Findings (both surfaced to the human, both handled):** **(1) theme-layer Finding** — 6.1–6.4 shipped functionally-correct but **completely unstyled** (kit ships tokens-only; the app never authored the Graphite Arc base/layout/chrome; no rendered-product verification existed — green tests/build never check appearance). Human-sequenced: a **dedicated 6.5 theme pass at the END of Phase-6 logic** + an **automated gstack-browser visual gate** (dev server vs `NexusOps-ui-kit/ui_kits/control-plane/index.html` — CONFIRMED to exist); tracked as the **§6.5 task** with VISUAL acceptance + Lesson §10. **(2) nav Finding (orchestrator brief error)** — 6.4c placed Settings on the content-view-switch vs §11.2's TopBar nav model → **RESOLVED by 6.4e** (TopBar Settings wired, view-switch Settings dropped) + an `ARCHITECTURE.md §11.2` nav-model note (human-confirmed).
- **Scope shifts:** decomposed 6.4 → 6.4a–e by daemon-dependence; split 6.4d → survival-display (done) + **6.4d-2** safety-state display (next, security-reviewer). Carry-forward triaged: 2 deleted (consumed by §8), 6.4 follow-ups inlined as `[ ]` tasks, 1 speculative narrow dropped, perf-nits spread to real-subscriptions, 4 cross-track spreads kept (5 items, under cap).
- **New blockers / open questions:** none new. The parked 6.3d/e + intent seam unblock at the **daemon mutation/intent-submission + Terminal-Channel contract freeze** (cross-track timing the human controls).
- **Next session target (FRESH team):** finish the last Phase-6 logic — **6.4d-2** (§17 safety-state display — security-reviewer) + the **checking-banner** — THEN the **6.5 theme pass + automated browser visual gate.** Carry forward: the parked 6.3d/e + intent seam, the 6.5 theme deliverable, the §11.2 nav-note, the provisional→generated reconcile (Usage + Recovery shapes), and the inlined 6.4 a11y/nav follow-ups.
- **Reference:** implementer session doc `ui-002-2026-06-08-...` (`731c355`); slice commits `c420cd7`/`885cc0d` (6.3b) · `23fbda3` (6.3c) · `f70757e` (6.4a) · `db9b89b` (6.4b) · `765923f` (6.4c) · `290381a` (6.4d) · `823d16e` (6.4e). Briefs `docs/briefs/009–015`.

### 2026-06-08 — UI track: Phase 6 round 3 (§17 safety-state display + checking-banner) — PHASE-6 LOGIC COMPLETE

- **Completed:** **3 ui slices** on `track/ui` (`5db... → 5f40149`; 116→**131 green**, tsc/oxlint/vite-build clean throughout): **6.4d-2** the §17 safety-state display — a **2-commit safety slice**, the FIRST §15/§17-touching UI surface: **L1** fencing/hard-conflict card (`ff2f8d6`, #6 never-auto-resolved) + **L2** fail-closed/audit-integrity alert (`503b6a2`, #5 non-dismissible), **security-reviewer PASS both layers** · **P6.4 checking/handshaking banner** (`5f40149`) — closes the silent-read-only gap (`deriveDegradedState("connected","unknown")`→new `"checking"` state). **Phase-6 LOGIC is now COMPLETE** (6.4 fully done; only the 6.5 theme/visual pass remains; 6.3d/e stay parked on daemon-1.5 per Decision C).
- **Decisions made:** **Step-2.5 TWEAK (orchestrator)** — scoped 6.4d-2 L1 to `fencing_conflict` ONLY; `stale_precondition` is a §17/§6.2 RE-APPROVABLE flow (regenerate preview + require fresh approval), NOT a never-auto-resolved hard conflict (mislabeling it would misrepresent a safety state) → deferred to the approval/preview surface. **Step-2.5 ADD** — completeness render coverage over the full audit-integrity discriminated union (a future state is forced to render, never silently dropped — #5). Modeling: **reuse the frozen `ActionRequest` enum** (`partially_succeeded`/`rollback_failed`) via `z.enum().extract` (drift-pinned), provisional ONLY for the net-new; `SafetyState.integrity` **required-nullable** (fail-closed-by-construction). Banked **`ui/LESSONS.md §11`** (degraded+safety states always explained/fail-closed — folds the safety-surface + read-only-never-silent conventions into one). `ARCHITECTURE.md §11.4` safety-surface rendering note added (MVP renders at Shell level; full HIQ host = Phase 8).
- **Scope shifts:** none new. 3 cross-track spreads added (safety provisional shapes → provisional→generated spread; stale-precondition treatment → daemon mutation/approval surface; checking-banner trigger → ui↔daemon-1.5 spread). Carry-forward triaged: 0 deleted / 6 spread / under cap.
- **New blockers / open questions:** none. The checking-banner's connected+unknown trigger is **wired-but-masked** until the real daemon-1.5 reconnect re-handshake (the mock resolves `version` with `data`); tracked on the ui↔daemon-1.5 spread.
- **Round seal (Option A, lead-decided, user away):** seal the logic milestone + push BEFORE the qualitatively-different 6.5 visual pass (isolates the clean logic round from 6.5's human-in-the-loop gate). **NO teammate cycle** (ctx ~32%) — same team continues into 6.5.
- **Next session target:** **6.5 Graphite Arc theme pass** (decomposed 6.5a base → 6.5b shell-chrome+grid → 6.5c view-panels + the automated gstack visual gate, §6.5) — **non-/tdd, human-judged acceptance**; drive to a lead-accepted best-effort production match, FINAL visual sign-off flagged for the user on return.
- **Reference:** implementer session doc `ui-003-2026-06-08-safety-state-display-and-checking-banner.md` (`c9acb49`); slice commits `ff2f8d6` (6.4d-2 L1) · `503b6a2` (6.4d-2 L2) · `5f40149` (P6.4 checking-banner). Briefs `docs/briefs/016` (6.4d-2) + `017` (checking-banner).

### 2026-06-08 — UI track: Phase 6 round 4 (6.5 Graphite Arc theme pass + automated visual gate) — ✅ PHASE 6 COMPLETE

- **Completed:** the **6.5 Graphite Arc theme pass** — 3 **non-/tdd VISUAL** slices on `track/ui` (131→**133 green**; the kit tokens were already imported, the gap was APPLYING them): **6.5a** base/global (`c439379` — dark `--surface-window` canvas + Geist + scrollbar chrome + global `.sr-only`) · **6.5b** shell chrome + **flexbox→CSS-Grid** cockpit (`b5618af` — 5-row grid; `--surface-panel` regions grounded to the prototype; sidebar 240→264 fix) · **6.5c** view panels + banner/safety **severity surfaces** (additive to glyph+label) + per-panel scroll (`3b6135d`). The **full automated gstack-browser visual gate** ran (dev server over HTTP vs the prototype, every view + driven safety/degraded fixtures, 6 screenshots) → **LEAD-ACCEPTED** (best-effort, production-grade). **Phase 6 is now COMPLETE** (logic + visual; modulo parked 6.3d/e on daemon-1.5).
- **Decisions made:** Option-A round seals throughout (logic before visual; this round seals the visual pass). **Visual-slice methodology** ratified: design/visual-gate flow (visual Step-2.5 → apply → rendered check vs prototype), not RED/GREEN; GROUND surfaces against the prototype's actual rendering (caught the 6.5b `--surface-canvas`→`--surface-panel` divergence). **ProjectSwitcher**: compacted to an honest CSS row this pass; the prototype's single-select **dropdown** needs an active-project model (behavioral) → routed to **§7.3 (daemon-INDEPENDENT, buildable now)**; a presentational-only dropdown was REJECTED (fake selector). Banked **`ui/LESSONS.md §12`** (theme/visual layer — apply/ground/gate; extends §10). `ARCHITECTURE.md`: no new edit (the §11.4 safety-surface note landed round 3).
- **Gate value (Lesson §10/§12 validated):** the visual gate caught what tsc/oxlint/jsdom structurally CANNOT — the **6.5a `*/`-in-CSS-comment bug** (a comment token-glob silently closes the comment + drops the next CSS rule → the whole `body` reset) + the **6.5b surface divergence**. Green tests would have shipped both.
- **Scope shifts:** none cut. Divergences from the prototype at the gate = unbuilt **Phase 7/8** features (switcher dropdown, two-column HIQ rail, command palette, Brain drawer, Gateway modal, Task inbox, PR review) or **approved app-specific adaptations** (compact switcher, the dedicated 5-row banner grid) — **no theme defects**.
- **New blockers / open questions:** **PAUSE-OR-CONTINUE call pending (lead, user away).** Most remaining ui work is **daemon/integration-gated** (6.3d/e + the intent seam on daemon mutation/Terminal; Phase 7/8 needing GitHub/Gateway/Brain contracts). **Daemon-INDEPENDENT, fixture-buildable now:** active-project selection (the ProjectSwitcher dropdown, §7.3) + a small a11y/nav **polish set** (TopBar history-nav, tablist arrow-roving + §9 audit refinement, sidebar resume-indicator, graph zero-projects guard, sessions-table filtering). **Final aesthetic sign-off on the theme flagged for the user on return.**
- **Next session target:** lead's continue-vs-pause call. If CONTINUE (daemon-independent): active-project selection is the highest-value next ui slice; else the polish set; else PAUSE (the track has delivered Phase 6 + the rest converges on daemon contracts).
- **Reference:** implementer session doc `ui-004-2026-06-08-graphite-arc-theme-pass-and-visual-gate.md` (`7dc2874`); slice commits `c439379` (6.5a) · `b5618af` (6.5b) · `3b6135d` (6.5c). Briefs `docs/briefs/018` (6.5a) + `019` (6.5b) + `020` (6.5c).

### 2026-06-08 — UI track: Phase 6 round 5 (active-project selection) — measured continue + AUTO-CYCLE at WARN

- **Completed:** **1 slice** (lead-directed measured continue, post-Phase-6): **P7.3(fwd) active-project selection** (`86727ec`) — a daemon-INDEPENDENT `/tdd` slice (133→**142 green**) completing the previously-INERT ProjectSwitcher: a UI active-project model (`active-project.ts` + `ActiveProjectProvider`/`useActiveProject`, mirroring `ReadOnlyProvider`) + single-select (`aria-pressed` + ✓Active glyph+label) + graph re-root + Sessions filter (Command Center stays global). Resolved the **6.3b graph project-source Q3** + the **zero-projects guard** (`graph-no-project`); stale-id re-scope guard (no ghost id).
- **Decisions made:** the slice is **UI selection state over the FROZEN projects projection** — NOT a mutation/provisional/Gateway-intent (no `canSubmitIntent` gate, no daemon dep) → banked **`ui/LESSONS.md §13`** (UI selection/scope over a frozen projection; distinct from the parked daemon-1.5 mutation path). The prototype's dropdown-popover WIDGET (caret + popover + WAI-ARIA radiogroup/roving) is a deferred presentation polish.
- **Scope shifts:** the §7.3 active-project item LANDED (model + selection + view re-rooting); the §6.3b zero-projects + Q3 graph-source items RESOLVED + ticked.
- **AUTO-CYCLE:** canonical `/context-check` hit **ui-implementer 70% [WARN]** on this slice → **lead-authorized seal + auto-cycle** (no mid-slice interrupt — the slice landed first). This round seals + the lead spawns the FRESH team.
- **New blockers / open questions:** **FRESH-TEAM directive = POLISH-SET vs PAUSE** (lead, user away). Daemon-independent polish set (all production-grade closures): TopBar back/forward history-nav (named-but-inert), tablist arrow-roving + §9 audit refinement, sidebar resume-mode indicator, sessions-table filtering, the ProjectSwitcher dropdown-popover widget. Everything else daemon/integration-gated (6.3d/e + intent seam; Phase 7/8). **Final aesthetic sign-off on the 6.5 theme still flagged for the user on return.**
- **Next session target:** fresh team — the lead's polish-vs-pause call. If polish: the inert-control/a11y closures (highest-value = TopBar history-nav, a real dead-affordance gap). Else PAUSE the ui track at its parallel-track limit (the rest converges on daemon contracts).
- **Reference:** implementer session doc `ui-005-2026-06-08-active-project-selection.md` (`d6816e4`); slice commit `86727ec`. Brief `docs/briefs/021`.

### 2026-06-08 — UI track: round 6 (daemon-independent polish round) — ✅ COMPLETE → **ui track PAUSES at its parallel-track limit**

- **Completed:** the **FINAL daemon-independent polish set — 5/5 slices** (lead-directed, user away), closing every remaining dead-affordance / a11y gap that didn't need a daemon contract: **(1)** TopBar back/forward history-nav `25ef3ed` (brief 022 — pure `viewHistoryReducer` + `useViewHistory`; disabled-at-boundary, the real dead-affordance closed); **(2)** Settings tablist roving + roving-aware §9 audit `128714e` (023 — WAI-ARIA roving; the audit taught roving-awareness + `role=tabpanel`); **(3)** Sessions-table status/free-text filtering `9a7945f` (024); **(4)** sidebar resume-mode indicator `64ffa42` (025 — §8 id-keyed side map, no `ProjectionItem` widening); **(5)** ProjectSwitcher dropdown-popover `0af44a5`+`9f2c91f` (026, 2-layer — trigger + roving `role=listbox`; orientation-aware roving re-homed to shared `a11y/roving.ts`; §9 audit generalized to {tab-in-tablist, option-in-listbox}). **142 → 193 green** (+51); tsc+oxlint+prettier clean. All test-first (slice 5 layer-driven); 3 high reviewer findings caught + fixed in-slice (2 focus stale-closure/out-of-bounds races + 1 phantom focus-jump); zero safety-invariant touches.
- **Decisions made:** all 5 are §13/§8/§9-family **pure UI state** (selection/nav/filter/presentation over frozen/local state — no mutation/provisional/Gateway-intent, no daemon dep). New shared **`ui/src/a11y/roving.ts`** (orientation-aware `nextTabIndex`) consumed by Settings (horizontal) + ProjectSwitcher (vertical). Lessons **§8** (entity-specific display data rides an id-keyed side map, never a widened `ProjectionItem`), **§9** (roving tabindex + the roving-aware audit generalized to {tab→tablist, option→listbox} + the dropdown/popover composite), **§13** (view-history nav + the table-filter pattern incl. the OR-independent-fields cross-boundary gotcha) all extended. Arch notes: §11.2 (history semantics + Sessions filter facets), §11.6 (composite-widget roving). Within-table **project facet intentionally NOT built** (redundant — owned by the active-project switcher).
- **Scope shifts:** all 5 polish items ticked complete (§6.4 history-nav + tablist roving + sidebar resume-mode; §6.3 sessions filtering; §7.3 dropdown widget). The Brain TopBar trigger split off as a Phase-8 residual (named-but-placeholder, lands with the Brain drawer — NOT a wire-or-disable miss). Out-of-scope TODOs documented (not Carry-forward'd, no consumer): history keybindings (Cmd-[/]); `isRovingMember` nested-container hardening (YAGNI); within-table project facet.
- **New blockers / open questions:** none. **The ui track now PAUSES at its parallel-track limit** — Phase 6 (logic + Graphite Arc visual) + active-project selection + the entire daemon-independent polish tail are DONE. Everything remaining is **daemon/integration-gated**: parked 6.3d/e (Terminal + Code/Diff) + the mutation/intent seam; Phase 7/8 (PR-Review Workspace / Task-Inbox / Brain drawer / Gateway modal); the provisional→generated reconcile; ExecutionProfile 0.5b. The track **resumes when the daemon freezes those contracts OR the user directs**. **Final 6.5 aesthetic sign-off still flagged for the user on return.**
- **Next session target:** _(ui track PAUSED)_ — resume on daemon contract-freeze (mutation/intent seam + Terminal Channel §6.4 → unparks 6.3d/e) or user direction. No daemon-independent ui work remains.
- **Reference:** implementer session doc `ui-006-2026-06-08-daemon-independent-polish-round.md` (`e1225a4`); slice commits `25ef3ed`·`128714e`·`9a7945f`·`64ffa42`·`0af44a5`+`9f2c91f`. Briefs `docs/briefs/022`–`026`.

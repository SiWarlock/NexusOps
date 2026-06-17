# /tdd brief — gateway_path_deltas (D4b) — the gateway-emitted-event delta COMPLETENESS SWEEP

## Feature
**Fix the CLASS (lead-ruled), not just one instance:** the gateway `emitted_events` path
(`pipeline.rs` terminal txn-B, `gtx.append`) publishes NO `ProjectionDelta` — so EVERY projection mutated
by a gateway-emitted event is orphaned from its nudge. The surfaced instance: production `SessionStarted`
(emitted via `EmittedEvent`, NOT `Command::Append`) never nudges → the L2 session card is stale on creation
(D3's `deltas_for_append` Session arm is dead in prod). **Sweep every nudge-able gateway-fed projection**
and pin EACH with a **production-path test** (driving the REAL gateway execute, not direct-append) so the
orphaned-nudge bug cannot recur for another projection.

**Design (the LESSONS §51-consistent class fix):** route every gateway-emitted event through a shared
**`deltas_for_event(...)`** — the SAME mapping `deltas_for_append` uses (extract it; `deltas_for_append`
becomes a thin wrapper). One mapping, both paths → no 2nd drift-prone delta-source. **CONTRACT-neutral**,
**NON-cat-1** (non-mutation post-commit fire-and-forget deltas) — **security-reviewer opt-in YES**
(gateway-pipeline-touching: confirm the delta-threading doesn't perturb the fail-closed txn-B).

### The sweep scope (BOUNDED by `ProjectionName` — only the 10-variant closed subscribe set can be nudged)
| Gateway-emitted event | Live emitter (reachable) | Nudge-able projection(s) → `ProjectionName` | delta `id` |
|---|---|---|---|
| `SessionStarted` | `session.create` (SessionExecutor) ✅ | `Session` · `ProjectActivity` · `ProjectGraph` (all fold it) | `session_id` (Session); `project_id` (Activity, Graph) |
| `WorktreeCreated` | `git.create_worktree` (GitExecutor) ✅ | `Worktree` | `worktree_id` (payload) |
| `PullRequestSynced` | `github.create_pr` (GithubExecutor) ✅ | `PullRequest` | `pr_number`/`pr_id` (payload — see Step-2.5 Q1) |
| *(every gateway command)* | submit/approve/deny/plan | `AuditTrail` (blanket, one per command) | `None` |

**Out of the per-projection sweep (no `ProjectionName` variant → NOT subscribe-able → audit-blanket only):**
`proj_project` + `proj_repository` (← `ProjectRescanned`) and `proj_integration_connection`
(← `IntegrationConnectionRegistered`) have NO `ProjectionName` variant — the ui can't subscribe to them, so
there's nothing to nudge (the AuditTrail blanket still covers them). **Flag (Step-9 / arch note):** if the
ui later needs to live-subscribe to projects / repositories / integration-connections, adding those
`ProjectionName` variants is a **CONTRACT change** — out of scope here; record it.
**`BranchCreated`** is emitted but has **no projector** (live-read overlay, LESSONS §47) → no delta.
**`ApprovalQueue`** already nudges (`approval_queue_delta`, 2.1c) — do NOT regress it.

## Use case + traceability
- **Task ID:** D4b (the user's UI-unblock work order) — **P4.5** the gateway-emitted-event delta
  completeness sweep + the D3-SessionStarted production-gap Finding fix (lead-ruled fold + sweep).
- **Architecture sections it implements:** `ARCHITECTURE.md §7`/`§7.2` (the gateway-fed read models +
  the projection-delta source), **§6.1** (the subscribe/`subscription_push` closed `ProjectionName` set),
  **§6.2** (the gateway pipeline the delta-publish threads through), **§11** (the ui session / worktree /
  PR / activity views the nudges keep live).
- **Widens phase scope because** the delta-source feeds the §7 read models + the §6.1 subscribe push for
  ui-track live-refresh (§11) and touches the §6.2 gateway pipeline — cross-cutting beyond Phase 4's primary
  §8/§17 anchors (the D3/D4a/4.0b-ui precedent).
- **Related context:** the **FINDING** (verified: `session_executor.rs:114-125` emits SessionStarted via
  `EmittedEvent`; `pipeline.rs:1001-1003`/`1053-1055` append emitted_events via `gtx.append` but publish NO
  delta; `deltas_for_append` runs only in `Command::Append`). · the lead RULING (fold the fix; sweep the
  class; production-path test required; security opt-in). · `EmittedEvent` enum (`gateway/executor.rs:32-53`:
  `SessionStarted{session_id,payload}` + `Namespaced{event_type,payload_json}`). · the live executors +
  their emits (`session_executor.rs`, `git/executor.rs`, `integrations/executor.rs`) registered in
  `main.rs:203-252`. · the projectors: `session.rs` / `activity.rs:27` / `graph.rs:44` (all fold
  SessionStarted), `worktree.rs:46` (WorktreeCreated), `pull_request.rs:47` (PullRequestSynced),
  `audit.rs:26` (blanket). · `ProjectionName` (`shared/src/ipc.rs:126-137`, 10 variants — the bound). ·
  the gateway delta accumulator (`approval_queue_delta` `pipeline.rs:79`, `publish_after_commit`
  `writer.rs:669`, the gateway command handlers `writer.rs:557-614`). · D3 (`019a4b1`) + D4a (`e3d82ad`) —
  the `deltas_for_append` arms this extracts. · LESSONS §17 (gateway delta accumulator), §51 (the one-mapping
  two-list agreement).

## Acceptance criteria (what "done" means)
- [ ] `deltas_for_event(...)` is the single shared event→`Vec<ProjectionDelta>` mapping; `deltas_for_append`
      is a thin wrapper — the D3/D4a `Command::Append` behavior is unchanged (all existing runtime delta
      tests pass).
- [ ] The gateway emitted-events loop (`pipeline.rs` txn-B, BOTH `Succeeded` + `FailedWithEvents` arms)
      routes each emitted event through `deltas_for_event` and publishes the result post-commit
      (`publish_after_commit`, `result.is_ok()`-gated; fail-closed txn-B unperturbed).
- [ ] **(Finding fix)** A `session.create` through the REAL gateway (SessionExecutor + FakeLauncher,
      auto-execute → SessionStarted emitted) publishes a `Session` AND a `ProjectActivity` AND a
      `ProjectGraph` `Upsert` delta — pinned by a **production-path integration test** (NOT a direct
      `store.append`).
- [ ] A gateway-emitted `WorktreeCreated` publishes a `Worktree` `Upsert` delta (id: `worktree_id`).
- [ ] A gateway-emitted `PullRequestSynced` publishes a `PullRequest` `Upsert` delta (id per Step-2.5 Q1).
- [ ] Every committed gateway COMMAND publishes ONE `AuditTrail` `Upsert` (id: None) — the gateway half of
      the D4a blanket; `ApprovalQueue` deltas are NOT regressed.
- [ ] Each reachable swept projection (Session/ProjectActivity/ProjectGraph via session.create; Worktree;
      PullRequest) has a **production-path test driving the gateway execute path** (a fake/test executor
      emitting the event THROUGH the real gateway execute+publish is acceptable for the edges-emitted
      events; Session uses the real SessionExecutor).
- [ ] A LESSONS §51 guard pins the gateway-fed folded-event set ↔ `deltas_for_event` agreement (every
      gateway-emitted event maps to its nudge-able projection(s), or is audit-only / no-projector).
- [ ] Publish-after-commit preserved: a rolled-back gateway txn publishes none of these.
- [ ] All tests pass (`runtime.rs` + `gateway*.rs`); `/preflight` clean; CONTRACT-neutral (no `shared/`
      change, no version bump); security-reviewer pass (txn-B / INV-SEC-1 unperturbed).

## Wiring / entry point (Step 7.5)
The gateway emitted-events loop (`pipeline.rs:1001-1003` + `1053-1055`) → `deltas_for_event` per emitted
event → accumulate → `publish_after_commit` (`writer.rs:669`, post-commit, `result.is_ok()`-gated). The
AuditTrail per-command nudge lands in `publish_after_commit` / the gateway command handlers. Live callers:
`session.create` (risk-0 auto-execute), `git.create_worktree`, `github.create_pr` — all registered in
`main.rs`. **Reachable by construction**; the production-path tests drive the real gateway execute.

## Files expected to touch
**Modified:**
- `daemon/src/runtime/writer.rs` — extract `deltas_for_event`; `deltas_for_append` → thin wrapper; thread
  the emitted-event deltas from the gateway `execute` result to the publish site; the AuditTrail
  per-gateway-command nudge.
- `daemon/src/gateway/pipeline.rs` — accumulate `deltas_for_event(...)` per emitted event in BOTH txn-B
  arms; extract payload-keyed ids (worktree_id / pr_number) from the `EmittedEvent` at the push site.
- `daemon/tests/runtime.rs` and/or `daemon/tests/gateway*.rs` — the production-path tests per projection +
  the LESSONS §51 guard + the behavior-preserving wrapper test.

The `execute()` signature likely changes to surface deltas to the publish site — **flag the exact approach
at Step 2.5** (keep txn-B fail-closed; deltas post-commit/fire-and-forget).

## RED test outline (Step 2)
1. **`test_gateway_session_create_publishes_session_activity_graph_deltas`** (THE Finding pin) — `session.create`
   through the real gateway (SessionExecutor + FakeLauncher) → assert Session + ProjectActivity + ProjectGraph
   `Upsert` deltas among those published. Why: the Finding — production SessionStarted must nudge all 3 folds.
2. **`test_gateway_worktree_created_publishes_worktree_delta`** — a gateway-executed `WorktreeCreated`
   (via the real or a test GitExecutor through the gateway execute) → a `Worktree` Upsert (id: worktree_id).
3. **`test_gateway_pull_request_synced_publishes_pr_delta`** — a gateway-emitted `PullRequestSynced` →
   a `PullRequest` Upsert (id per Q1).
4. **`test_gateway_command_publishes_audit_trail_delta`** — a committed gateway command → one `AuditTrail`
   Upsert (id: None); `ApprovalQueue` deltas still fire (no regression).
5. **`test_deltas_for_append_behaviour_unchanged`** — the extraction is behavior-preserving (D3/D4a deltas
   identical pre/post-refactor; drain-and-find tests stay green).
6. **`test_rolled_back_gateway_txn_publishes_no_deltas`** — a refused/rolled-back gateway action publishes
   none of the new deltas (fail-closed txn-B unperturbed).
7. **`test_gateway_folded_events_match_delta_source`** (LESSONS §51 guard) — every gateway-emitted event maps in
   `deltas_for_event` to its nudge-able projection(s), OR is audit-only (ProjectRescanned /
   IntegrationConnectionRegistered / *SyncFailed), OR has no projector (BranchCreated). Why: LESSONS §51 —
   ONE mapping; a future gateway-emitted event folding a nudge-able projection without an arm is a silent
   stale-UI bug (the exact class this slice closes).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. `ProjectionDelta`/`ProjectionName` frozen @0.11.0; all swept projections
  use EXISTING `ProjectionName` variants. (The no-variant projections are explicitly OUT of scope — a future
  CONTRACT change, flagged.)
- **Orchestrator doc rows (Step 9):** the MVP-projections row gains a `[D4b]` AS-BUILT note + the Finding
  note; LESSONS §51 extends (one mapping across both paths) + a NEW lesson on orphaned-nudge-on-path-migration
  + test-must-drive-the-production-path (lead-requested). Orchestrator territory, at the seal.
- **§2.5-seam model touched?** No. No schema-snapshot test.

## Things to flag at Step 2.5
1. **PR delta id — `pr_number` vs the computed `pr_id` (`{repo_id}#{pr_number}`).** `proj_pull_request` keys
   on `pr_id` (repo_id sibling-read in the projector). My default vote: **whatever the row's PK is — match
   the projector's key** (likely `pr_id`); parse `pr_number` from the emitted payload + derive/parse the
   same key the projector uses, so the nudge id matches the row. If `repo_id` isn't reachable at the push
   site, fall back to `id: None` (the PR workspace re-reads). Flag your resolution.
2. **AuditTrail gateway nudge — one-per-command (`publish_after_commit`).** Default: one per committed
   gateway command (Action* events append outside the emitted-events loop → per-command is the single clean
   chokepoint; the subscriber re-reads the page). Flag if you see a cleaner single site.
3. **The `execute()` delta-threading (LOAD-BEARING).** Terminal txn-B `execute` returns an `ActionAck`, no
   accumulator. Default: accumulate the emitted-event deltas (build them AFTER `gtx.append`, inside the
   committed success arm) and surface them to the existing `publish_after_commit` (return alongside the ack
   or take a `&mut Vec`). A delta-build error must NEVER affect the commit (deltas are post-commit /
   fire-and-forget). **Surface your exact approach — security-reviewer confirms txn-B stays fail-closed.**
4. **`deltas_for_event` signature across paths.** The `Command::Append` path has envelope ids (session_id /
   project_id) only; the gateway path additionally has payload ids (worktree_id / pr_number from the
   `EmittedEvent`). Default: `deltas_for_event(event_type, &Ids)` where `Ids` carries the available
   Option-ids; each caller fills what it has (the wrapper passes envelope ids; the gateway loop adds the
   payload-extracted id). Keep ONE mapping (LESSONS §51). Flag a cleaner factoring if you find one.
5. **No-`ProjectionName` projections — confirm OUT of scope.** proj_project / proj_repository /
   proj_integration_connection have no subscribe-name → audit-blanket only; **do NOT add `ProjectionName`
   variants** in this slice (that's a CONTRACT change). Default: confirm + I record the future-CONTRACT flag.

## Dependencies + sequencing
- **Depends on:** D3 (`019a4b1`), D4a (`e3d82ad`) — the `deltas_for_append` arms this extracts; 4.0b-1
  (SessionExecutor emitted_events), edges P5.2/P7.1 (Git/Github executors + their events), 2.1c (the gateway
  delta accumulator + `publish_after_commit`). All landed.
- **Blocks:** the ui live-refresh of session-create / worktree / PR / activity views (the cockpit go-live);
  completes AuditTrail (cross-path) started in D4a.

## Estimated commit count
**1** (lead-ruled — fold the Session fix in; isolating it is "commit cosmetics for no real gain"). One
cohesive slice: the shared-`deltas_for_event` extraction + the gateway emitted-events publish + the sweep +
the production-path tests. The implementer MAY split a `fix:`-first commit if it genuinely aids bisection,
but it's not required. **Gateway-pipeline-touching → security-reviewer RUNS (opt-in).** Not a §15 pin.

## Lessons-logged candidates anticipated
- **NEW LESSON (lead-requested)** — **orphaned-nudge-on-path-migration + test-must-drive-the-production-path:**
  when an event's append PATH migrates (here SessionStarted moved from `Command::Append` to the gateway
  `emitted_events` path at 4.0b-1), any path-specific side-channel (its `deltas_for_append` nudge) is
  silently orphaned; a test that drives the NON-production path (direct `store.append`) masks it. Discipline:
  route side-channels through a path-agnostic shared function; pin behavior with a test that drives the REAL
  production entry, not a convenient proxy.
- **Convention candidate (extend LESSONS §51)** — a projection event may append via EITHER path; the delta
  mapping must be ONE shared function (a per-path duplicate re-introduces the two-list drift LESSONS §51 forbids).
- **Architecture-doc note** — §6.2/§7: the gateway emitted-events path publishes projection deltas via the
  shared `deltas_for_event`; the audit nudge is per-command. + the future-CONTRACT flag (no-subscribe-name
  projections).
- **Future TODO (carry-forward)** — `ProjectionName` variants for project/repository/integration_connection
  IF the ui needs them (CONTRACT change); the seq-cursor audit-delta enrichment.

## How to invoke
1. **Read this brief end-to-end** (the sweep table + Step-2.5 Q3 the `execute()` threading are load-bearing).
2. **Run `/tdd gateway_path_deltas`**.
3. **Step 0 (Restate)** — confirm the completeness sweep + the Finding fix + the shared-`deltas_for_event`.
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5** — answer the 5 design questions; surface the `execute()` threading + the LESSONS §51 sweep coverage.
6. **Step 9** — categorized flags + the security-reviewer result + the no-subscribe-name future-CONTRACT flag.

> **Step-8 reviewer policy:** `code-quality-reviewer` runs (`every-slice`). **`security-reviewer` runs
> (orchestrator opt-in)** — no §15 invariant, but it threads deltas through the §6.2 gateway pipeline / txn-B;
> the review confirms the delta-publish stays post-commit + fire-and-forget and the fail-closed txn-B /
> INV-SEC-1 commit semantics are unperturbed. NON-cat-1.

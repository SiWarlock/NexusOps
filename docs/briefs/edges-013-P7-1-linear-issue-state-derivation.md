# /tdd brief — linear_issue_state_derivation

## Feature
The **Linear issue-state derivation** — the deterministic foundation of the Linear read vertical (analogous to edges-004's PR-status derivation, simpler). Map Linear's workflow-**state-type** (`triage`/`backlog`/`unstarted`/`started`/`completed`/`canceled`) → the frozen §5.1 **`Task`** status (the external-task subset), via `parse_linear_state_type` (GraphQL string → daemon enum, edges-008-conservative) + `derive_task_status_from_linear` (state-type → `Task`). Pure, deterministic; no network (the Linear read client that fetches the issue is the next slice).

## Use case + traceability
- **Task ID:** P7.1 (in-lane, Approach A — the Linear read foundation; the Linear read client + the `proj_*`/`tasks` registry + the `linear.link_issue` executor wiring stay gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (Linear = GraphQL read; integration-failure §17 contract), `§8` (the GitHub/**Linear intake** flow: `integrations(Linear read) → tasks(external_task rows) cached as projection`).
- **Widens phase scope because** the derivation targets the frozen **§5.1 `Task`** machine (R-8 superset) — the external-task status a Linear issue maps into; this slice **consumes** the frozen `Task` enum read-only (it does not modify it), the same way edges-004/006 consumed the frozen `PullRequest` enum. (Widens to §5.1; same waiver posture as the GitHub PR slices.)
- **Related context:** edges-004 (`897a9f2`) `derive_pull_request_status` (the GitHub analog — a daemon-defined derivation into a frozen §5.1 machine). edges-008 (`0eb60d4`) the conservative-floor + case-insensitive decode convention. §5.1 R-8: one `tasks` table, `kind ∈ {plan_task, external_task}`, a **superset** Task machine, external tasks render the GitHub/Linear subset. The frozen `Task` (`shared/src/status.rs:58`): `Unassigned, Queued, Assigned, Ready, InProgress, Blocked, NeedsClarification, InReview, ChangesReady, PrOpened, NeedsReview, RequestedChanges, Done, Deferred, Merged, Closed, Abandoned` (terminal: Merged/Closed/Abandoned). **Linear's `WorkflowState.type` is a closed 6-value set** (the impl confirms the exact GraphQL enum at Step-2.5, like the octocrab spikes).

## Acceptance criteria (what "done" means)
- [ ] `LinearStateType` (daemon-internal enum): `Triage, Backlog, Unstarted, Started, Completed, Canceled`.
- [ ] `parse_linear_state_type(Option<&str>) -> LinearStateType` maps Linear's GraphQL `WorkflowState.type` values (`"triage"/"backlog"/"unstarted"/"started"/"completed"/"canceled"`, case-insensitive per edges-008) → the enum; `None`/`null`/unrecognized → a conservative default (Step-2.5 Q1 — lean `Backlog`).
- [ ] `derive_task_status_from_linear(state_type) -> Task` maps each state-type → a frozen §5.1 `Task` status (the external-task subset), total + exhaustive (a new `LinearStateType` variant forces a reconcile here — the LESSON-2 exhaustive-match discipline).
- [ ] The mapping (Step-2.5-reviewed defaults): `Backlog→Queued`, `Unstarted→Ready`, `Started→InProgress`, `Completed→Done`, `Canceled→Abandoned`, `Triage→NeedsClarification` (the ambiguous ones — Triage / Unstarted / Canceled — are Step-2.5 Q2).
- [ ] **Terminal mapping:** `Completed→Done`, `Canceled→Abandoned` land on frozen-terminal `Task` states (Done is non-terminal in the machine but is the completed-task state; Abandoned IS terminal — confirm Done-vs-terminal at Step-2.5 Q2).
- [ ] An end-to-end pin: `parse_linear_state_type(Some("started"))` → `Started` → `derive_task_status_from_linear` → `Task::InProgress` (the parse→derive chain).
- [ ] Both fns are **total** (no panic / no `unwrap`; every input resolves to exactly one variant) and pure (no `Clock`/IO).
- [ ] All tests pass; `/preflight` clean. Cross-doc invariant: **none** (daemon-internal derivation over the frozen `Task` enum — no new `shared/` model; the Linear→Task mapping is daemon-defined, recorded as an arch-note like the §7.2 PR-derivation precedence).

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated Linear read client + the `tasks`(external_task)/`proj_*` projector + the `linear.link_issue` executor.** The derivation is consumed by (a) the **next in-lane slice** — the Linear read client (Linear GraphQL fetch → the issue model → this derivation) — and (b) the gated `tasks` registry projector + the §7.3 Task Inbox. Tested-but-unwired **by design** (Approach A) — Step 7.5 grep-confirms only the module + test reference the new symbols. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**New:**
- `daemon/src/integrations/linear.rs` — `LinearStateType` + `parse_linear_state_type` + `derive_task_status_from_linear`. `use nexusops_shared::status::Task`.
- `daemon/tests/linear_issue_state.rs` — the parse / derive / parse→derive tests.

**Modified:**
- `daemon/src/integrations/mod.rs` — `pub mod linear;`

If implementation needs files beyond this list, flag at Step 2.5.

## RED test outline (Step 2)
Tests in `daemon/tests/linear_issue_state.rs`:

1. **`parse_linear_state_type_known_values`** — Asserts: each of the 6 GraphQL strings → its `LinearStateType`. Why: §9 Linear state-type decode.
2. **`parse_linear_state_type_none_and_unknown`** — Asserts: `None`/`null`/`"FOO"`/`""` → the conservative default (Q1). Why: edges-008 floor (total, no fabrication).
3. **`parse_linear_state_type_case_insensitive`** — Asserts: `"Started"`/`"BACKLOG"` decode case-folded. Why: edges-008/009 case convention.
4. **`derive_task_status_all_state_types`** — Asserts: each of the 6 `LinearStateType` → its mapped `Task` (the Q2 table). Why: §5.1 R-8 external-task derivation (exhaustive).
5. **`derive_terminal_states`** — Asserts: `Completed→Done`, `Canceled→Abandoned` (the issue-closed mappings). Why: §5.1 terminal/completed semantics.
6. **`parse_then_derive_chain`** — Asserts: `"started"` → `Started` → `InProgress`; `"completed"` → `Completed` → `Done`. Why: the parse→derive composition (end-to-end).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none — `LinearStateType` is daemon-internal; the derivation targets the **frozen** `Task` enum (no change to it).
- **Orchestrator doc rows to write hot (Step 9 routing):** none for `daemon/CLAUDE.md`/Appendix A. **Anticipated (integration-owned — FLAG, I route):** a §5.1/§9 arch-note pinning the **daemon-defined Linear-state-type → Task-status mapping table** (the architecture lists the Task machine + the R-8 subset rule but does not pin the Linear mapping — exactly like the §7.2 PR-derivation precedence I routed for edges-004) + a C-list lesson (Linear external-task derivation = state-type→Task, conservative-floor + exhaustive-match, daemon-defined).
- **Shared-contract seam model touched?** **NO** — the frozen `Task` enum is consumed read-only; `LinearStateType` is daemon-internal. No envelope/ID/status-machine/catalog change → no schema-snapshot, no CONTRACT_VERSION. (Per LESSON-2, if the derivation ever needs to PRODUCE a `Task` value, it binds the frozen wire value — pin parity if a wire-string surfaces; here it returns the Rust enum, no wire-string.)

## Things to flag at Step 2.5
1. **Unknown `WorkflowState.type` default.** Linear's 6 state-types are a closed, stable set, so an unknown is a genuine API anomaly. Default to a least-salient parked state (`Backlog`, edges-008-consistent) vs. surfacing it (`NeedsClarification`, "needs a human look")? **Default vote: `Backlog`** — consistent with the established conservative-floor convention (don't fabricate human-attention from a parse miss); but the surface-the-anomaly argument is real — your call. Confirm Linear's exact `WorkflowState.type` enum values against the SDK/GraphQL while you're here.
2. **The ambiguous state-type → Task mappings.** `Triage` → `NeedsClarification` (untriaged, needs human triage) vs `Unassigned`/`Queued`? `Unstarted` → `Ready` (todo, ready to start) vs `Assigned`? `Canceled` → `Abandoned` (terminal won't-do) vs `Closed`? **Default votes: Triage→NeedsClarification, Unstarted→Ready, Canceled→Abandoned.** The unambiguous ones: Backlog→Queued, Started→InProgress, Completed→Done. These are daemon-defined (I route the chosen table as a §5.1/§9 arch-note) — push back if a mapping reads wrong for the §7.3 Task Inbox.
3. **State-type only, or secondary signals?** A Linear issue also has an assignee (could distinguish Unassigned vs Ready/Assigned), a `startedAt`/`completedAt`, etc. **Default vote: state-type only** for the foundation — the workflow-state-type is the authoritative lifecycle signal; assignee/timestamps are refinements for a later slice (the read client can layer them if §7.3 needs them). Keep this slice the pure state-type derivation.
4. **Module layout.** A new `integrations/linear.rs` (mirrors `integrations/github.rs` from edges-009). **Default vote: new `linear.rs`** — the Linear read client (next slice) extends this module, same as github.rs.

## Dependencies + sequencing
- **Depends on:** the frozen `Task` enum (`shared/`, landed); edges-004's derivation pattern; edges-008's decode convention.
- **Blocks:** the **next in-lane slice** — the Linear read client (Linear GraphQL fetch → an issue model → this derivation, behind a `LinearReadClient` trait + `FakeLinearReadClient` + the real client, errors via edges-003's `classify` — mirrors the edges-009 octocrab client). Then the gated `tasks`(external_task) projector + the §7.3 Task Inbox + the `linear.link_issue`/`linear.create_issue` executors.

## Estimated commit count
**1** — a focused deterministic derivation in one new module; no network, no safety invariant, no cross-doc change. ~50–80 lines + tests. (Opens the Linear vertical; the read client follows.)

## Lessons-logged candidates anticipated
- **Convention candidate** — "A Linear issue is an **external_task** (§5.1 R-8): its status derives from the Linear **`WorkflowState.type`** (the 6-value closed set), NOT the team's custom state name — `parse_linear_state_type` (conservative floor) + `derive_task_status_from_linear` (exhaustive match into the frozen `Task` subset; daemon-defined mapping, recorded as an arch-note). Mirrors the GitHub `derive_pull_request_status` two-stage pattern." (extends the edges-004/008 line).
- **Architecture-doc note candidate** — the daemon-defined Linear-state-type → §5.1 Task-status mapping table (under §5.1 R-8 / §9), which the architecture leaves unpinned.

## How to invoke
1. **Read this brief end-to-end** — esp. the Step-2.5 mapping questions (the daemon-defined Linear→Task table) + confirm Linear's `WorkflowState.type` enum.
2. **Run `/tdd linear_issue_state_derivation`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (files)** — confirm the new `integrations/linear.rs` + the test file + the `mod.rs` line.
5. **Step 2.5** — send the test-design write-up + the mapping-table + unknown-default decisions (+ the confirmed Linear enum) before GREEN; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 9** — surface cross-doc "none" + the daemon-defined Linear→Task mapping table (§5.1/§9 arch-note) + the C-list lesson (integration-owned — flag, I route).

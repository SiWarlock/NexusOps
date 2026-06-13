# /tdd brief — phase_5_7_wiring_event_types

## Feature
Freeze the **Phase-5/7 wiring event-type contract** in `shared/` — the events edges' (dormant) executors will emit and its projectors will consume — in ONE batched additive `CONTRACT_VERSION` bump (edges regenerates once). `shared/`-only (payload structs + `EVENT_TYPE` registry entries + schema bundle + snapshots + 3-way verify); **no daemon emission code** (edges' executors emit these via the `EmittedEvent` mechanism when each namespace lands at P5/P7). The second half of the cross-track **edges-R1** unblock. §2.5-seam freeze.

## Use case + traceability
- **Task ID:** P4.0b-R1b
- **Architecture sections it implements:** `ARCHITECTURE.md §7.1` (the `EventTypeRegistry` + event envelope; the new payload types). Touches §5.1 (`PullRequest` enum reused by `PullRequestSynced`) + §15 (the `remote_url`/`keychain_ref` redaction contract); §5.0/§2.5 (the contract-propagation mechanism + the dependency seam) + LESSON §15/§23 are referenced for the pattern.
- **Phase-scope note — this brief WIDENS phase scope because** the **§7.1** event types it freezes are **cross-phase Phase-5/7 wiring enablers** (project/worktree + integration/PR), not in Phase 4's nominal Spec-anchor set. They are batched into Phase 4 purely by the cross-track **edges-R1** timing (the lead-confirmed R1-A → R1-B → 4.0b-2 order, folding the edges unblock into the P4 executor-seam work). No P4 session/survival surface is implemented here — this is the `shared/` event-type contract edges consumes.
- **Related context:**
  - **edges-R1 specs (read-only, cross-worktree):** `../NexusOps-edges/docs/planning/edges-R1-wiring-seam-and-event-specs.md` **Part 2** (the field-level tables — the binding field spec) + `edges-R1-routing-packet.md` Part 2. The **daemon finalizes names/fields/version**; the field tables are edges' requirement.
  - **Lead rulings (away-authority):** `ProjectRescanned` = **ONE coarse event** (the projector splits projects/repositories rows). `*SyncFailed` = **split, non-auth variant ONLY** in R1b (the `auth_expired` variant deferred — its 0.5b gate is now LIFTED, but it needs its own §17/INV-SEC re-review when it lands).
  - **Decided edges-side (NOT in R1b):** `WorktreeStatusRefreshed` is **NOT an event** (live-read projection cache — edges reads git2 on demand, `git_checked_at` stamp). So R1b adds the worktree LIFECYCLE events only, not a per-poll status event.
  - **Pattern to follow:** the existing `shared/src/events.rs` event structs (`TelemetrySampled`, `TerminalProcessExited`, the `ActionExecution*` family) — each a `pub struct` (serde + schemars + `#[serde(deny_unknown_fields)]`) with `impl { pub const EVENT_TYPE: &'static str = "<Name>"; }` (the single-home registry name); identity (actor/project_id/resource_refs) on the **envelope**, payload = **delta only**. LESSON §15 (the schemars/§5.0 freeze traps), §23 (observation events are non-Gateway).

## Acceptance criteria (what "done" means)
- [ ] **P5.1 — `ProjectRescanned`** (one coarse event): the edges Part-2 field set (`is_git`, `repo_root?`, `remote_url?`, `branch?`, `detached`, `is_dirty`, `workflow_pack`, `cc_crew`, `plan_file?`, `brain`, `scanned_at: Timestamp`); `deny_unknown_fields`; `EVENT_TYPE = "ProjectRescanned"`.
- [ ] **`remote_url` §15 contract documented on the field** (rule #5/#3/#4): the URL **userinfo (`user:token@`) MUST be stripped at the emit source** (edges' project executor) — the §15 Redactor is the *backstop*, not the primary control (a generic URL password has no prefix → outside the redactor's recall envelope, LESSON 13). The field doc states this; the strip-at-source enforcement + its test are **edges' P5.1 emission slice** (cross-track note).
- [ ] **P5.2 — worktree/branch lifecycle:** `WorktreeCreated` (`worktree_id`, `path`, `branch_name`, `base_branch?`) · `BranchCreated` (`branch_name`, `base?`) · the overlay-axis transitions `WorktreeMerged` / `WorktreePrunable` / `WorktreeDeleted` / `WorktreeLocked` (shape per Step-2.5 #2 — default empty-payload, identity on the envelope, the `ActionStarted{}` precedent). Each `deny_unknown_fields` + `EVENT_TYPE`.
- [ ] **P7.1 — integration reads + sync failures:** `PullRequestSynced` (`pr_number`, `status: PullRequest` [the frozen §5.1 enum], `branch`, `base`, `mergeable?`, `checks_summary?`, `pr_checked_at`) · `IntegrationConnectionRegistered` (`connection_id`, `provider: github|linear`, `keychain_ref`, `account?`) · `GithubSyncFailed` + `LinearSyncFailed` (`provider`, `reason` [a redaction-safe STRUCTURAL class-name string, NOT raw API text], `failed_at`) — **non-auth variant only**.
- [ ] **`keychain_ref` is a NON-SECRET POINTER only** (§15 rule #4 — never the token); the field doc states it.
- [ ] All new types added to the **schema bundle** (`shared/src/schema.rs`) → published schema regenerated (`shared/contracts/schema/*`).
- [ ] **ONE `CONTRACT_VERSION` bump** (0.25.0 → 0.26.0) covering the whole batch (edges regenerates once).
- [ ] **Schema snapshots** (§2.5-seam) for every new type in `shared/tests/contract.rs` (field-name set == checked-in snapshot) + the **3-way verify GREEN** (the gate restored at 4.0b-T now guards this bump — run `shared/contracts/verify/run.sh`).
- [ ] `shared/` builds; `cargo test --workspace` green; `/preflight` clean. **`provider` enum** (`github|linear`) is a new closed enum → reject-unknown + a flat `enum` schema (LESSON 15 trap 2).

## Wiring / entry point (Step 7.5)
**none — wiring lands in the edges P5/P7 slices.** R1b freezes the `shared/` event-type CONTRACT (the consumable + emittable shapes). The EMITTERS (edges' Project/Git/Github/Linear executors via `EmittedEvent` variants) and the CONSUMERS (edges' projectors: `proj_project_activity`, the project graph, `proj_worktree`, `proj_pull_request`, the private registries) are edges' P5/P7 work against this frozen contract. The `EVENT_TYPE` consts + the published schema are the reachable contract surface (the §5.0 mechanism + the golden-log/snapshot tests bind them).

## Files expected to touch
**Modified:**
- `shared/src/events.rs` — the ~11 new payload structs + `EVENT_TYPE` consts.
- `shared/src/schema.rs` — register the new types in the schema bundle.
- `shared/src/lib.rs` — `CONTRACT_VERSION` 0.25.0 → 0.26.0.
- `shared/contracts/schema/*` — regenerated published schema.
- `shared/tests/contract.rs` — schema snapshots for the new types (+ the count/registry assertions).

No daemon files (no emission yet). If the `provider` enum or a shared `Timestamp`/`PullRequest` reuse needs a helper beyond this list — **flag at Step 2.5**.

## RED test outline (Step 2)
Tests in `shared/tests/contract.rs` (the §2.5-seam snapshot + the registry pattern the existing event types use):

1. **`test_projectrescanned_snapshot`** — the `ProjectRescanned` field-name set == the checked-in snapshot; `EVENT_TYPE == "ProjectRescanned"`. Why: §7.1/§2.5-seam freeze.
2. **`test_worktree_lifecycle_snapshots`** — `WorktreeCreated`/`BranchCreated` field sets + the 4 overlay transitions' shapes == snapshots; `EVENT_TYPE`s correct. Why: §7.1.
3. **`test_p7_integration_snapshots`** — `PullRequestSynced` (incl. `status: PullRequest` reuse) / `IntegrationConnectionRegistered` / `GithubSyncFailed` / `LinearSyncFailed` field sets == snapshots. Why: §7.1.
4. **`test_provider_enum_closed_reject_unknown`** — `provider` deserializes `github`/`linear`, rejects an unknown value (closed enum). Why: §5.0/§15 reject-unknown (LESSON 15 trap 2).
5. **`test_contract_version_bumped_0_26_0`** — `CONTRACT_VERSION == "0.26.0"` and the published schema's `x-contract-version` matches. Why: §5.0 single batched bump.
6. **`test_deny_unknown_fields_on_new_types`** — each new struct rejects an extra field. Why: §5.0 reject-unknown end-to-end.

**Acceptance-by-run (the restored 4.0b-T gate):** `bash shared/contracts/verify/run.sh` → `schema==pydantic==zod` PASS (the 3-way verify now covers the 0.26.0 bump).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **YES — ~11 new event types + the `provider` enum** (a contract addition). The orchestrator writes hot at the round: the **`EventTypeRegistry` rows** (`daemon/CLAUDE.md` cross-doc table) + the **`ARCHITECTURE.md` Appendix A `EventTypeRegistry` rows + §7.1** + the CONTRACT bump record.
- **§2.5-seam model touched?** **YES** — every new event type crosses the §2.5 seam (consumed cross-track by edges) → the schema-snapshot tests are mandatory (above), authored this cycle.
- **Reviewer policy:** **`security-reviewer` = YES** — the slice introduces the §15-sensitive contract fields (`remote_url` cred-carrying; `keychain_ref` pointer-not-secret). The pass confirms the field docs state the §15 contract correctly and no field invites a secret into an event payload/row.

## Things to flag at Step 2.5
1. **Batch structure / commit count.** ~11 event types is a large freeze. My default vote: **land the structs in layers (L1 = P5.1+P5.2, L2 = P7.1) but ONE `CONTRACT_VERSION` bump + schema regen + snapshots + 3-way verify in the FINAL layer** (the lead's "one batched bump → edges regens once"). If you'd rather one commit or a 3-way split, flag it — but keep a single bump.
2. **Overlay lifecycle event shapes.** `WorktreeMerged`/`WorktreePrunable`/`WorktreeDeleted`/`WorktreeLocked` are underspecified in the edges spec (the field tables stop at `WorktreeCreated`/`BranchCreated`). My default vote: **empty-payload events (`struct WorktreeMerged {}` …), worktree identity on the envelope `resource_refs`** — the `ActionStarted{}`/`ActionSucceeded{}` precedent; the transition IS the `event_type`. Flag if edges' `derive_worktree_status` overlay axis needs a payload field (e.g. a reason).
3. **`reason` on `*SyncFailed`.** My default vote: **`reason: String` documented as a STRUCTURAL class-name** (the edges classifier's terminal class, redaction-safe — NOT raw API text). A closed enum would be cleaner but the classifier's class set is edges-owned + still accreting → a documented structural string now; tighten to an enum when edges' classifier set freezes (a P7.1 reconcile). Flag if you'd rather freeze an enum now.
4. **`remote_url` §15 — backstop test now or defer to edges' emission?** My default vote: **document the field contract (strip-userinfo-at-source + redactor-backstop, rule #5) in R1b; the strip-at-source enforcement + test is edges' P5.1 emission slice** (R1b ships no emitter). Optionally add ONE daemon redactor-backstop test asserting a prefixed token in a URL is caught — but the PRIMARY control is strip-at-source (edges), so don't over-invest here. If the security-reviewer wants the backstop pinned now, add it.

## Dependencies + sequencing
- **Depends on:** R1-A (✅ `e5c8811`/`c653121` — the seam exists, so these events have a future emit home) + 4.0b-T (✅ — the restored 3-way verify guards this bump).
- **Blocks:** the **edges-track full resume** (R1-on-main = R1-A + R1-B). On landing, the orchestrator signals the lead → edges `/team-start`.

## Estimated commit count
**1–2.** One batched §2.5-seam contract freeze (the lead's "one bump"). Layer the struct additions if it reads cleaner, but the `CONTRACT_VERSION` bump + schema regen + snapshots + 3-way verify are atomic (the freeze is one logical unit). A cross-doc invariant change is involved → the orchestrator's Appendix-A/registry rows ride the round commit (staggered, per the cadence).

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — the ~11 new `EventTypeRegistry` rows (Appendix A + §7.1) + the `provider` enum.
- **Future TODO — cross-track** — (a) the `*SyncFailed` `auth_expired` variant (deferred; its 0.5b gate is lifted but it needs a §17/INV-SEC re-review) `last-consumer-slice: edges P7.1 auth-failure slice`; (b) the `remote_url` strip-userinfo-at-source enforcement `last-consumer-slice: edges P5.1 emission`; (c) `reason` structural-string → enum tighten `last-consumer-slice: edges classifier freeze`.
- **Convention candidate** — likely none (follows the established event-type pattern).

## How to invoke
1. **Read this brief end-to-end** + the edges Part-2 field tables (the binding field spec).
2. **Run `/tdd phase_5_7_wiring_event_types`**.
3. **Step 0/1** — confirm against the Feature line + the `shared/`-only file list.
4. **Step 2.5** — answer the 4 questions (esp. #2 overlay shapes + #4 the `remote_url` §15 backstop). Dispatch `security-reviewer` at Step 8 (the §15-sensitive fields).
5. **Step 9** — surface the EventTypeRegistry rows for the orchestrator's hot Appendix-A/§7.1 writes + confirm the 3-way verify GREEN at 0.26.0.

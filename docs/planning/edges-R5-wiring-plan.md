# edges R5 — P5/P7.1 PHASE-EXIT WIRING PLAN

> **Status:** post-merge plan (R5, 2026-06-13). Authored by `edges-daemon-orchestrator` after merging the
> main R1 seal `0c637a8` into `track/edges` (merge commit `bd3ee31`). Refines step 3 of
> `docs/planning/edges-phase-exit-readiness.md` (the 10-step merge-reconciliation checklist) into an
> ordered, TDD-sliced wiring sequence. **The implementer builds against this + per-slice briefs.**

---

## MERGE RESULT (done)

- **Merged:** `0c637a8` ("seal the 4.0b-T + edges-R1 round") → `track/edges`, merge commit **`bd3ee31`**.
  Targeted the **last sealed** main commit, NOT the in-flight `00d82c0` (4.0b-2 L1, CONTRACT 0.27.0) above it.
- **Absorbed:** CONTRACT **0.20.0 → 0.26.0** (daemon P3.1/3.2/3.4/3.5 + P4.0a/4.0b-1 + R1). `shared/` was
  edges-untouched → all bumps came in clean.
- **Conflicts (3, all additive — kept both sides):** `daemon/src/lib.rs` (+`session`/`terminal` mods),
  `daemon/Cargo.toml` (+`base64`/`portable-pty` alongside edges' `git2`/`octocrab`/`async-trait`/`reqwest`),
  `Cargo.lock` (regenerated).
- **Green-verified:** `cargo check` + **530 tests / 0 failed** + `clippy -D warnings` + `fmt --check`, all pass.

## R1-COMPLETENESS — COMPLETE (no Finding)

| Deliverable | State on `bd3ee31` |
|---|---|
| Part 1 — executor registration seam | ✅ `CatalogExecutor { handlers: HashMap<ExecutorKind, Arc<dyn ActionExecutor>> }` + `register()` + stub-fallback; INV-SEC-1 `Adjudication` guard + `requires_resource_refs` precondition run BEFORE dispatch. |
| Part 1 — design-choice **3a (async model)** | ✅ trait kept **SYNC** (`fn execute(&self, req) -> ExecutionOutcome`); daemon drives via `spawn_blocking` (P4.0a). **The frozen 2.3 trait was NOT reopened.** |
| Part 2 — 11 wiring event types + `Provider` | ✅ `ProjectRescanned`, `WorktreeCreated`, `BranchCreated`, `Worktree{Merged,Prunable,Deleted,Locked}` (empty-payload), `PullRequestSynced`, `IntegrationConnectionRegistered`, `Github/LinearSyncFailed`. CONTRACT 0.26.0. |
| Part 4 — `test-support` cargo-feature | ✅ in `daemon/Cargo.toml` (the mechanism; edges gates its 2 fakes). |
| **Bonus — catalog entries** | ✅ all edges action types already catalogued with `ExecutorKind` + locked risk: `project.rescan`→`Project`(r0), `git.status/diff`→`Git`(r0), `git.create_worktree/create_branch`→`Git`(r2), `github.create_pr/_draft`→`Github`(r2), `linear.link_issue/create_issue`→`Linear`(r2). **No further `shared/` change needed.** |
| H1 — `ExecutionProfile` enum freeze | ✅ frozen on main (4.0b-1, CONTRACT 0.24.0); `SessionStarted.execution_profile_id` present. 5.3 un-gated; `auth_expired` variant still deferred (needs §17/INV-SEC re-review). |
| Design-choice 3b (`ProjectRescanned` granularity) | ✅ one coarse event (edges' lean). |
| Design-choice 3c (worktree status) | ✅ live-read cache, empty-payload lifecycle events (edges' lean). |

## Emission + migration facts (verified on `bd3ee31`)

- **Emission mechanism:** each executor returns `ExecutionOutcome { emitted_events: Vec<EmittedEvent>, applied, ... }`;
  the Gateway appends `emitted_events` **in-txn through the §15 redaction gate** (the `SessionExecutor`/`SessionStarted`
  precedent, LESSON §28). Edges' executors emit the new event types this way — never a direct write.
- **Migration sequence:** `SUPPORTED_USER_VERSION = 8` (MIGRATION_1..8). Edges' D5 migration = **MIGRATION_9** (→ v9).
  ⚠️ **Coordination:** daemon P4 work on main (above the seal) could also claim MIGRATION_9 → collision at the eventual
  edges→main final merge. Flag the claimed number to the daemon track, or re-number to next-free at the final merge.

---

## WIRING SLICE SEQUENCE (each its own TDD slice; **`security-reviewer` on every wiring slice** — all are INV-SEC-1-touching, the first real edges mutations)

Ordered lowest-risk-first; reads/risk-0 before mutators; the migration foundation before the connection event.

**Wave A — risk-0 executors (no FS/external mutation):**
1. **`project.rescan` executor (P5.1, X::Project, risk-0)** — `ProjectExecutor` runs the existing detection engine
   → emits **`ProjectRescanned`** + projector → `projects`/`repositories` rows. **§15 carry:** `remote_url` =
   **strip `user:token@` userinfo at source + redactor backstop** (rule #5; a generic URL password has no prefix
   → outside the recall envelope). Register `ExecutorKind::Project`.
2. **`git.status` / `git.diff` read executors (P5.2, X::Git, risk-0)** — `GitExecutor` read arms serve status/diff
   over the existing git2 **read-only** backend (no events, no mutation). Register `ExecutorKind::Git` (read arms).

**Wave B — git mutators (git CLI, NOT git2 — forbidden #6):**
3. **`git.create_worktree` / `git.create_branch` executor (P5.2, X::Git, risk-2)** — `GitExecutor` mutate arms via
   the **git CLI** (forbidden #6 — git2 is read-only) → emits **`WorktreeCreated`** / **`BranchCreated`** + the
   overlay-axis lifecycle transitions + projector → `proj_worktree`. **First real FS mutation through the Gateway.**

**Wave C — integration connection + the D5 migration:**
4. **`integration_connections` migration + `IntegrationConnectionRegistered` (P7.1)** — **MIGRATION_9** (the private
   table, daemon-internal — `conn_` not a frozen-22 IdKind) + the connect path → emits
   **`IntegrationConnectionRegistered`** + projector. **§15 carry:** `keychain_ref` = **non-secret POINTER only**
   (rule #4 — never the token).

**Wave D — external sync executors (the design-choice-3a caveat is LOAD-BEARING here):**
5. **github sync executor (`github.create_pr` / `_draft`, P7.1, X::Github, risk-2)** — `GithubExecutor` via octocrab.
   **3a caveat:** the trait is SYNC + octocrab is async → run on a **dedicated blocking context**
   (`spawn_blocking` + `Handle::block_on`), **never `block_on` on a tokio worker thread (panics)** — the
   `SessionExecutor` spawn_blocking precedent (LESSON §28) is the template. → emits **`PullRequestSynced`** +
   **`GithubSyncFailed`** (non-auth) + projector → `proj_pull_request`. **§15 carry:** `*SyncFailed.reason` =
   redaction-safe **structural class-name**, never raw API text. §17 `AuthFailed`-branch is the deferred carry.
6. **linear sync executor (`linear.link_issue` / `create_issue`, P7.1, X::Linear, risk-2)** — `LinearExecutor` via
   the reqwest GraphQL adapter; same 3a `spawn_blocking`/`block_on` discipline → emits the link/create outcome +
   **`LinearSyncFailed`** (non-auth) + projector. Same §15 `reason`-class carry.

**Wave E — phase-exit close-out (mix of small slices + orchestrator/bench rows):**
7. **5.3 ExecutionProfile binding (H1 resolved)** — enum is frozen on main (merged); wire profile resolution at
   approval-time + `SessionStarted` binding (§15 #8). **DEFER** the `auth_expired` `*SyncFailed` variant (needs the
   §17/INV-SEC re-review; non-auth shipped first). *Scope to confirm — much may already be daemon-side at 4.0b-1.*
8. **`test-support` feature consumption** — gate edges' 2 fakes (`FakeGithubReadClient`/`FakeLinearReadClient`)
   behind `#[cfg(feature = "test-support")]` now that the feature exists. Small slice.
9. **P5.4 `project.rescan` bench** — `[[bench]] harness=false`; baseline **1.029 ms ≪ 3 s** SLO (re-author with the
   known number; LESSON §22 cadence — /phase-exit + nightly, never `cargo test --workspace`).
10. **`cargo audit`** — reqwest / octocrab / async-trait vs the Phase-2 baseline (`docs/audits/P2-cargo-audit.txt`);
    record new-vs-baseline; escalate any finding.
11. **`/phase-exit 5` + `/phase-exit 7`** — row-by-row; the previously-gated `§6.3`/`§15`/`§8` wiring anchors are now
    LIVE (drop the gated-waivers as each lands); tick the phases only on CLEAR. Push row = verify-only / user-gated.

## PLAN-DELTA / lessons numbering (post-merge)

- **Lessons → `daemon/LESSONS.md` start at §30** (daemon took §26–§29 in P3/P4; merge brought §28/§29 in — **NOT** §28
  as the pre-merge readiness doc said). Renumber edges' R4 §C lesson (epoch-ms-reset trap) + any new wiring lessons §30+.
- **`IMPLEMENTATION_PLAN.md`** is now the merged main tracker (auto-merged, main-only-changed). The phase-exit ticks
  5.x/7.x rows in it directly (this is the merge event — the worktree copy lands on main at phase-exit completion).
- **Arch-notes:** §9 read-client boundaries + R4 NotFound/epoch-ms (016) + richer §9 read model (018).

## Key design decisions to confirm with the lead before fan-out
1. **Slice granularity** — 6 wiring slices (waves A–D) + 5 close-out items (wave E). Bundle any? (e.g. the two git-read
   arms with the git mutators; or github+linear sync as one bundled brief). Lean: keep mutators atomic (security), bundle reads.
2. **MIGRATION_9 collision** — claim it now on `track/edges` vs. coordinate next-free with the daemon track at the final merge.
3. **`auth_expired` defer holds** — non-auth `*SyncFailed` only this round; the auth variant stays gated on §17/INV-SEC re-review.

---

## R5 round progress + accumulated hot-routing (apply at the phase-exit merge)

> Edges does NOT edit the shared root docs (`IMPLEMENTATION_PLAN.md`/`ARCHITECTURE.md`/`daemon/CLAUDE.md`/
> `daemon/LESSONS.md`) in-worktree mid-round (cross-track rule). Doc-deltas accumulate here + apply at the
> phase-exit merge reconciliation. Lead rulings (R5 open): slice granularity = orch's call (atomic mutators,
> bundle reads); `auth_expired` defer CONFIRMED; **MIGRATION_9 = D8 DEFER** to the final edges→main merge
> (daemon's in-flight 4.0b-2 may hold v9 → claiming now risks a hard collision; consumer-less forward-laying →
> deferral is free, D5-aligned; Wave-C takes the then-next-free number, and whether to build+test against a
> test-schema now vs. defer the connection slice is orch's call); **bridge gateway/ edits = edges OWNS them**
> as phase-exit integration (lead-confirmed; variant shape = orch's call, ruled B).

**Slice ledger:**
- **edges-019** P5.1 `project.rescan` executor — LANDED `c739278` (543/0, security CLEAR). Executor + emission
  + §15 strip-at-source; read-model projector deferred (needs MIGRATION_9). Q1 ruled **B (generic
  `EmittedEvent::Namespaced{event_type, payload_json}`)** — one gateway/ edit serves all ~11 edges events.
- **edges-020** P5.2 `git.create_worktree` executor — DISPATCHED (task #5). First real edges FS mutation.

**Slice-plan revision (orch, slice-sequencing authority):** the separate "git read executors" slice
(Wave-A slice 2) is **DISSOLVED** — `git.status`/`git.diff` have NO consumer (reads served via
`get_projection(Worktree)→proj_worktree` + the in-lane diff backend), and `ExecutorKind::Git` is ONE handler
for all `git.*`, so `GitExecutor` (edges-020) handles `create_worktree` + delegates status/diff/create_branch
to the inner stub. Net Wave-B/C/D unchanged; one fewer slice.

**Accumulated PLAN-DELTA for the merge reconciliation:**
- **LESSON 30** (next-free; daemon took ≤§29) — edges executors emit via the in-txn `EmittedEvent::Namespaced`
  bridge through the §15 gate (SessionExecutor precedent); credential-bearing URL fields (`remote_url`)
  stripped AT THE EMIT SOURCE — authority-scoped, **last-`@`-in-authority** delimiter, ALL scheme-URL userinfo
  stripped (a token can ride the bare-username slot), scp-style intact; the Redactor is the backstop only.
- **Arch-doc notes (edges-019):** (a) `ExecutorKind::Project` registered in production main.rs; (b)
  `ProjectRescanned` has a live edges emitter; (c) the new daemon-internal `EmittedEvent::Namespaced{event_type,
  payload_json}` generic bridge (object_ref dropped — `AppendIntent` has no generic slot; identity rides the
  envelope `project_id`/`correlation_id`, LESSON §10/§17); `SessionStarted` stays typed.
- **Carry-forward — §15-backstop/repo_root (origin edges-019):** the §15 entropy backstop masks high-entropy
  `repo_root` path components in the persisted `ProjectRescanned` (defense-in-depth working). Implication for
  the **registry-projector / MIGRATION_9 slice:** a high-entropy real repo path → masked repo_root → the
  projector can't locate the repo from the event alone → re-derive identity from `project_id`, OR exempt
  path-fields from entropy masking (arch consideration). NOT blocking emit+strip.
- **Completed-work ticks (hold):** P5.1 = PARTIAL (executor + emission landed edges-019; registry projector
  pending MIGRATION_9). P5.2 = in progress (edges-020 mutator; proj_worktree projector + create_branch pending).
- **Cross-track surfaces (lead-aware):** edges touches `gateway/executor.rs` + `gateway/request.rs` (the
  `EmittedEvent::Namespaced` bridge — 1 variant + 1 arm, additive, edges-owned per the lead) + MIGRATION_9
  **deferred to the final merge (D8)** — Wave-C takes the then-next-free number after the daemon's schema settles.
  Both lead-ruled + logged to the cross-track ledger for the final merge.

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

**Wave D — external sync executors (the design-choice-3a mechanism is LOAD-BEARING here):**

> **3a mechanism — REFINED to the as-built (R6 orch trace, lead-endorsed 2026-06-13; supersedes the pre-merge
> "spawn_blocking + Handle::block_on, never on a worker thread" framing).** `execute()` runs on the
> **write-actor's dedicated raw `std::thread`** (`runtime/writer.rs:273-326` → `gateway/pipeline.rs:969`) —
> NOT a tokio worker, **no entered runtime**. So `Handle::current()` PANICS there and `spawn_blocking` is the
> wrong tool (the OPPOSITE footgun from the ledger's worker-thread framing). **CORRECT:** inject a captured
> `tokio::runtime::Handle` from `main.rs`'s `#[tokio::main]` runtime (`Handle::current()` works in the async
> `run()`) → `handle.block_on(async{…})` in `execute()` (block_on from a non-runtime/non-worker thread does
> NOT panic; the reactor runs on the runtime's workers). → **Wave-D LESSON (§32 candidate).**
>
> **STANDING REQUIREMENT (lead-MANDATED 2026-06-13, every external-call executor — github + linear + any
> future):** wrap the network future in a **`tokio::time::timeout`**. The single write-actor serializes ALL
> daemon mutations → an unbounded octocrab/Linear hang would freeze the entire mutation path (liveness).
> NOT optional. A timeout → `Failed` (structural reason).

5. **github sync executor (`github.create_pr` risk-3 / `_draft` risk-2, P7.1, X::Github)** — `GithubExecutor` via
   octocrab over the captured-`Handle` `block_on` + timeout (above). → emits **`PullRequestSynced`** (success) +
   **`GithubSyncFailed`** (terminal non-auth) + projector → `proj_pull_request` (projector = a separate slice).
   **§15 carry:** `*SyncFailed.reason` = redaction-safe **structural class-name**, never raw API text. §17
   `AuthFailed`-branch is the deferred carry. Emitting `GithubSyncFailed` on a FAILED action needs an additive
   **`ExecutionOutcome::FailedWithEvents`** (edges-owned gateway bridge — daemon-internal, additive). Brief
   edges-023 (spec-lint PASS). *(catalog risk: create_pr=3, create_pr_draft=2 — both approval-gated; the
   pre-merge "r2" summary was imprecise, no functional diff.)*
6. **linear sync executor (`linear.link_issue` / `create_issue`, P7.1, X::Linear, risk-2)** — `LinearExecutor` via
   the reqwest GraphQL adapter; SAME 3a captured-`Handle` `block_on` + MANDATORY timeout + the `FailedWithEvents`
   reuse → emits the link/create outcome + **`LinearSyncFailed`** (non-auth) + projector. Same §15 `reason`-class
   carry. (GraphQL already uses variables, not interpolation — injection-safe.)

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
- **edges-020** P5.2 `git.create_worktree` executor — LANDED `7dabab5` (554/0). First real edges FS mutation
  (git-CLI seam, `WorktreeCreated` via the Namespaced bridge, `WorktreeId::new()`, side_effect=true). **Security
  HIGH caught + CLOSED in-slice:** git **argument-injection** (leading-`-` operand parsed as a git flag →
  on-disk worktree diverges from the approved+audited Action — audit-integrity, INV-SEC-1-adjacent); fixed
  fail-closed + canonical arg order + regression test. security-reviewer else PASS.
- **edges-021** P5.2 `git.create_branch` executor — LANDED `51f5586` (563/0, security CLEAR). Extends
  `GitExecutor`; `BranchCreated`; the arg-injection guard extracted to a SHARED `reject_dash_operands` helper
  across both git arms (repo_path exempt = cwd, not a git operand). **P5.2 git mutators complete.**
- **edges-022** P5.2 `proj_worktree` projector — LANDED `c666dc0` (571/0, security CLEAR). Folds
  `WorktreeCreated`→`proj_worktree`; `repo_id` via the LESSON-17 immutable sibling-read of
  `action_requests.resource_refs` (repo_id ULID §15-allowlisted → survives redaction); `status="creating"` via
  `wire_value` (LAYER-CORRECT — persistence-core must NOT import the `git/` edge's `DerivedWorktreeStatus`);
  live-read cols NULL; rebuild-equivalent. **P5.2 READ VERTICAL CLOSED** (mutator→event→projection→IPC).

**P5 STATUS (end of R5-so-far):** P5.1 = executor+emit done (registry projector MIGRATION_9-deferred); P5.2 =
mutators + read vertical COMPLETE (live-read status refresh deferred — see TODO). **Remaining for the
phase-exit:** P7.1 Wave-D (github/linear executors + proj_pull_request projector) · the MIGRATION_9-deferred
P5.1 registry projector + Wave-C `integration_connections` · the §7.2 + subscribe-delta + live-read hardening ·
P5.4 bench · cargo audit · `/phase-exit 5`+`7`.

**Slice-plan revision (orch, slice-sequencing authority):** the separate "git read executors" slice
(Wave-A slice 2) is **DISSOLVED** — `git.status`/`git.diff` have NO consumer (reads served via
`get_projection(Worktree)→proj_worktree` + the in-lane diff backend), and `ExecutorKind::Git` is ONE handler
for all `git.*`, so `GitExecutor` (edges-020) handles `create_worktree` + delegates status/diff/create_branch
to the inner stub. Net Wave-B/C/D unchanged; one fewer slice.

---

## R6 round progress (the Wave-D external mutators) — accumulated hot-routing

**R6 slice ledger:**
- **edges-023** P7.1 **github sync executor** (Wave-D slice 5) — LANDED `498bd21`
  (585/0; security-reviewer PASS 0 findings; code-quality 4-fixed-in-slice; reachability YES). `GithubExecutor`
  (`ExecutorKind::Github`, registered `main.rs`) for `github.create_pr`(risk-3)/`_draft`(risk-2) → an injected
  async octocrab WRITE-client seam (`integrations/github_write.rs`: `GithubWriteClient`/`CreatePrArgs`/domain
  `CreatedPr`/`GithubWriteError`/`OctocrabGithubWriteClient`/`FakeGithubWriteClient`[test-support]), driven
  from the SYNC trait over a **captured `tokio::runtime::Handle` + `handle.block_on` + a mandatory 30s
  timeout** (the as-built 3a; execute() on the write-actor std::thread, non-worker — `Handle::current()`/
  `spawn_blocking` is wrong). Success → `PullRequestSynced` (Namespaced bridge, §15-gated, atomic w/
  `ActionSucceeded`, `side_effect_applied:true`); terminal-non-auth → `GithubSyncFailed` via the new
  `FailedWithEvents`; AuthFailed/transient → plain `Failed`. §15 `reason` = structural class-name. Reuses
  `derive_pull_request_status` + `classify_octocrab_error`(→pub(crate)) + `extract_pr_signals`.
- **edges-024** P7.1 **linear sync executor** (Wave-D slice 6 — LAST Wave-D mutator) — LANDED
  `f908424` (602/0; security-reviewer PASS 0 findings; code-quality 1-high+3-fixed
  in-slice; reachability YES; **all edges-023 github tests stay green** — the extraction refactor is
  behavior-preserving). `LinearExecutor` (`ExecutorKind::Linear`, registered `main.rs`) for
  `linear.link_issue`(requires_resource_refs)/`linear.create_issue`(FromInputs, no ref) → an injected async
  Linear-GraphQL WRITE-client seam (`integrations/linear_write.rs`), driven over the SAME captured-`Handle`
  `block_on` + the SHARED 30s `NETWORK_TIMEOUT`. **Success → `Succeeded{emitted_events:[]}` — NO Linear
  domain event** (Q1: intentional asymmetry; the frozen contract has none; Linear read on-demand via
  `fetch_issue`, §7.3; `ActionSucceeded` is the record; `changed_resources` carries the resource_refs).
  Terminal-non-auth → `LinearSyncFailed` via the landed `FailedWithEvents`; auth/transient → `Failed`. New
  pure `classify_linear_write_response` (GraphQL `errors[].extensions.code` over HTTP `classify`).
  **`classify_sync_failure`/`SyncFailure` EXTRACTED** — the §17 disposition (Auth/TerminalNonAuth/Transient)
  now in ONE exhaustive place, used by BOTH executors (edges-023 github refactored to call it;
  behavior-preserving). **WAVE-D EXTERNAL MUTATORS COMPLETE.**
- **edges-025** P7.1 **`proj_pull_request` projector** (Wave-E) — LANDED `8db6cc7`
  (612/0; code-quality 1-med-fixed; security-reviewer NOT required [read-model fold]; reachability YES).
  `PullRequestProjector` folds `PullRequestSynced`→`proj_pull_request` (§7.2 cache); `pr_id` = the
  rebuild-safe `{repo_id}#{pr_number}` composite (`#` ULID-safe); `repo_id` via the LESSON-17 sibling-read;
  `status` via `wire_value(&PullRequest)`; `title` NULL (no payload field); `mergeable`/`checks_summary`
  not projected (fed `status`); the edges-022 3-case taxonomy; rebuild-equivalent. **GITHUB READ VERTICAL
  CLOSED** (`github.create_pr`→`PullRequestSynced`→`proj_pull_request`→`get_projection(PullRequest)`).
  _Deferred nits (LOW, accepted): `pr_id` `#`-collision (repo_id past the Gateway trust boundary, ULID-safe);
  no `resource_refs_json` Decode-degrade test (mirrors the edges-022 gap)._

**R6 PLAN-DELTA additions (apply at the phase-exit merge; held — cross-track rule):**
- **`classify_sync_failure`/`SyncFailure` (edges-024)** — a SHARED exhaustive §17 failure→outcome disposition
  in `integrations/executor.rs` used by both `GithubExecutor` + `LinearExecutor` (github + linear can't
  diverge on the Auth/TerminalNonAuth/Transient routing — a correctness guard, not just DRY). edges-023's
  `GithubExecutor::classify_failure` refactored to call it (behavior-preserving; github #5/#6/#7/#12 confirm).
  **Behavior-change note (MED, intentional, UNREACHABLE path):** the `Success`-input arm of the shared fn folds
  to `Transient`, changing github's unreachable-path `Failed` message string (bespoke "unreachable" → "(transient):
  success"); github tests unaffected (the path is unreachable). Noted for the merge ledger.
- **Linear-success-no-event asymmetry (edges-024 Q1; arch-doc note)** — `linear.link_issue`/`create_issue`
  success emits NO domain event (only `ActionSucceeded`), INTENTIONAL: Linear issues read on-demand via
  `fetch_issue` (§7.3) — no write-event-fed projection, vs github's `proj_pull_request`-fed `PullRequestSynced`.
  No §7.3/§8 gap found (impl confirmed); the architecture-as-contract rule HELD (no improvised event). §6.3
  `linear.*` LIVE · §17 `LinearSyncFailed` emit path LIVE.
- **SPREAD (edges-024)** — **Linear live-client mutation-payload parsing:** the live client treats 2xx +
  no-GraphQL-`errors[]` as success (doesn't parse the mutation `success:false` payload); a soft failure would
  mis-report `ActionSucceeded` (honest-audit: a false success is worse than a false failure). MVP-accept
  (orch-approved; rare, the live client is the fake-covered non-deterministic edge; the deterministic mapper
  handles errors[]/HTTP). `last-consumer-slice: a Linear live-client hardening slice`. `(origin: 2026-06-13
  edges-024)` _(the write-actor-offload + Linear-auth-bootstrap + deferred-`auth_expired` SPREADs REUSE
  edges-023's — same consumer markers.)_

**R6 PLAN-DELTA additions (apply at the phase-exit merge; held — cross-track rule) [edges-023]:**
- **3a mechanism REFINED (lead-endorsed 2026-06-13)** — see the Wave-D block above. The pre-merge
  "spawn_blocking + Handle::block_on, never on a worker thread" framing is SUPERSEDED by the as-built
  (write-actor raw std::thread, no entered runtime → captured `Handle::block_on`, never `Handle::current()`).
- **LESSON 32 (next-free; daemon took ≤§29, edges §30/§31)** — the **external-network-mutator pattern**: a SYNC
  `ActionExecutor` driving an async client via a CAPTURED `Handle::block_on` + a **mandatory `tokio::time::
  timeout`** on the write-actor std::thread (never `Handle::current()` there — panic; the timeout is liveness,
  the single write-actor serializes ALL mutations); the §17 classifier→`*SyncFailed` (terminal-non-auth ONLY,
  structural §15 `reason`, never raw API text); auth/transient → plain `Failed`; the LESSON-31 injection guard
  ADAPTS to fail-closed operand validation for a typed (non-CLI) API; `FailedWithEvents` for the atomic
  failure+observation-event. **TEST-HARNESS pin:** the `execute()`-path tests are plain `#[test]` + a built
  `Runtime` handle (NOT `#[tokio::test]` — `block_on` inside a runtime context panics).
- **Arch-doc note (ARCHITECTURE.md §6.2 / Appendix-A-adjacent):** `ExecutionOutcome::FailedWithEvents{detail,
  emitted_events}` — a daemon-internal gateway-contract extension (edges-owned bridge, phase-exit integration;
  additive — existing `Failed(String)` sites untouched; pipeline txn-B records `ActionFailed` + appends the
  events atomic; `side_effect_applied()==false`). Plus: §6.3 `github.create_pr*` LIVE; §7.2/§17
  `PullRequestSynced`/`GithubSyncFailed` emit path LIVE.
- **SPREADs (consumer-marked):**
  - **write-actor execute-phase OFFLOAD** — a slow external executor blocks the single write-actor for
    ≤NETWORK_TIMEOUT (bounded; security-reviewer info-only). `last-consumer-slice: a gateway execute-phase-
    offload hardening slice` (daemon-core write-actor territory — routed to the lead/cross-track ledger, like
    subscribe-delta). `(origin: 2026-06-13 edges-023)`
  - **github auth bootstrap** — `main.rs` registers an UNAUTHENTICATED octocrab handle (a real create →
    401→AuthFailed→Failed, no event — fail-closed-correct); the gh-token/Device-Flow + keychain slice + the
    deferred `auth_expired` `*SyncFailed` variant. `last-consumer-slice: edges P7.1 auth slice`. `(origin:
    2026-06-13 edges-023)`
  - **`proj_pull_request` projector** folding `PullRequestSynced`→the §7.2 read cache — the deferred PR
    read-vertical close (the edges-022 `proj_worktree` precedent). `last-consumer-slice: a proj_pull_request
    projector slice (Wave-D follow-on)`. `(origin: 2026-06-13 edges-023)`

**Accumulated PLAN-DELTA for the merge reconciliation:**
- **LESSON 30** (next-free; daemon took ≤§29) — edges executors emit via the in-txn `EmittedEvent::Namespaced`
  bridge through the §15 gate (SessionExecutor precedent); credential-bearing URL fields (`remote_url`)
  stripped AT THE EMIT SOURCE — authority-scoped, **last-`@`-in-authority** delimiter, ALL scheme-URL userinfo
  stripped (a token can ride the bare-username slot), scp-style intact; the Redactor is the backstop only.
- **LESSON 31** (edges-020) — edges git mutators run via an injected git-CLI seam (forbidden #6 — never git2;
  structural grep-pin on `git/executor.rs`+`cli.rs`, NOT the git2-read backend); `side_effect_applied: true`
  (→ honest `ActionPartiallySucceeded` on a txn-B fault, LESSON 21); mint the `wt_` id via `WorktreeId::new()`
  (domain-id-via-::new(), persisted-once + replay-safe); **reject leading-`-` operands fail-closed + canonical
  option-before-operand arg order (argument-injection guard — a leading-`-` operand becomes a CLI flag → the
  executed mutation diverges from the approved+audited Action: audit-integrity)**; STRUCTURAL failure reasons
  (no raw git stderr → §15).
- **STANDING REQUIREMENT (from the edges-020 arg-injection HIGH):** EVERY external mutator that takes inputs
  (edges-021 `create_branch`; Wave-D github/linear) MUST guard against argument/parameter injection (leading-`-`
  for CLI args; the analogous vector for octocrab/Linear params) fail-closed BEFORE the call + a regression test.
  Fold into each mutator brief. Cross-track-relevant (the daemon's own future external mutators want it too).
- **Arch-doc notes (edges-019):** (a) `ExecutorKind::Project` registered in production main.rs; (b)
  `ProjectRescanned` has a live edges emitter; (c) the new daemon-internal `EmittedEvent::Namespaced{event_type,
  payload_json}` generic bridge (object_ref dropped — `AppendIntent` has no generic slot; identity rides the
  envelope `project_id`/`correlation_id`, LESSON §10/§17); `SessionStarted` stays typed.
- **Carry-forward — §15-backstop/repo_root (origin edges-019):** the §15 entropy backstop masks high-entropy
  `repo_root` path components in the persisted `ProjectRescanned` (defense-in-depth working). Implication for
  the **registry-projector / MIGRATION_9 slice:** a high-entropy real repo path → masked repo_root → the
  projector can't locate the repo from the event alone → re-derive identity from `project_id`, OR exempt
  path-fields from entropy masking (arch consideration). NOT blocking emit+strip.
- **FINDING — approve-path redacts executor operational inputs (§7.2/§15; origin edges-020; lead-ENDORSED
  MVP-accept; CATEGORY-1, on the human's return-review ledger):** the §7.2 approve-path runs an executor off
  the DURABLE row's §15-REDACTED inputs (pipeline.rs:671-675's deferred "real-input-fidelity" concern, now
  LIVE). `git.create_worktree`'s operational inputs are FS paths → a high-entropy component (macOS tempdir
  hash) is masked → broken git op on the approve path. Production LOW (real paths low-entropy survive); §15
  invariant HOLDS (over-redaction FP, not a leak). MVP-accept edges-020 (the 9a real-CLI/raw + 9b approve-path
  low-entropy split + an in-code comment). **Cross-cutting proper-fix (human, future hardening slice, own
  security pass):** (a) a non-redacted operational-input channel [§15 bypass → INV-SEC re-review] vs (b) exempt
  path/operational fields from entropy redaction [§15-policy change]. **Wave-D note (lead):** the same MVP-accept
  default extends to the github/linear mutators (operational inputs = repo/PR/issue identifiers, low-entropy) —
  CONFIRM per-slice, flag if any hits a genuinely high-entropy operational field.
- **LESSON 17 generalization (edges-022):** a gateway-event projector sources non-payload identity (repo_id)
  from the IMMUTABLE sibling `action_requests` row via `env.action_request_id` (LESSON 17 generalized from
  object_refs/graph to the gateway-emitted-event case); the §15 ID-allowlist (LESSON 13) lets a repo_id ULID
  survive the redacted sibling-read; status bound via `wire_value` (the layer-correct producer — persistence-core
  must NOT import the `git/` edge). *Process note: a Step-2.5 approval (orch) endorsed importing `git/` into
  `projections` (layer-reverse) — impl caught it at GREEN + used `wire_value` [same output]. Step-2.5 should
  check layer-direction for cross-module refs.*
- **FINDING (subscribe-delta; origin edges-022; → lead cross-track ledger; CROSS-CUTTING, daemon-owned):** the
  `emitted_events` append loop (pipeline.rs:994-996) threads NO `ProjectionDelta`, so EVERY emitted-event
  projector write lacks a live subscribe-push — `WorktreeCreated`→`proj_worktree` AND the daemon's own
  `SessionStarted`→`proj_session`. Reads work via `get_projection`; subscribers see it on reconnect. A
  gateway/pipeline delta-threading fix (daemon-track-owned; benefits its own emitted events). NOT blocking.
- **TODO (live-read status refresh; P5.2 follow-on) — ✅ CONSUMED edges-026 (R7):** `read_worktree_status`
  → `proj_worktree` git-axis cache via a non-Gateway/non-event write-actor command + a git-watcher task. See
  the R7 block below.
- **Completed-work ticks (hold):** P5.1 = PARTIAL (executor + emission landed edges-019; registry projector
  MIGRATION_9-deferred). P5.2 = mutators (edges-020/021) + read vertical (edges-022) COMPLETE; live-read status
  refresh deferred (TODO above).
- **Cross-track surfaces (lead-aware):** edges touches `gateway/executor.rs` + `gateway/request.rs` (the
  `EmittedEvent::Namespaced` bridge — 1 variant + 1 arm, additive, edges-owned per the lead) + MIGRATION_9
  **deferred to the final merge (D8)** — Wave-C takes the then-next-free number after the daemon's schema settles.
  Both lead-ruled + logged to the cross-track ledger for the final merge.

---

## R7 round progress (the thin in-lane drain → then PAUSE) — accumulated hot-routing

**R7 slice ledger:**
- **edges-026** P5.2 **§7.2 worktree-status live-read cache refresh** — LANDED `c195c7f`
  (620/0; code-quality 2-fixed-in-slice [usize→i64 saturating; git-watcher error/panic logging]; security NOT
  required [read-only git2, no event, no secret]; reachability YES). `read_worktree_status` (git2) →
  `proj_worktree` git-axis cache (`dirty_state`/`ahead_count`/`behind_count`/`last_commit_sha`/`git_checked_at`
  + recomputed `status`) via a NEW **non-Gateway, non-event** write-actor command `RefreshWorktreeStatus` (the
  drain_once/reap_leases precedent; single-writer forbidden #3; §7.1 — **NO `WorktreeStatusRefreshed` event**,
  the git-axis is a live-read cache) triggered by a 30s **git-watcher** interval task (drainer/reaper precedent,
  ARCHITECTURE.md:340). **Layer-clean:** persistence-core stays git-free (the UPDATE takes plain computed
  values; the git read + `derive_worktree_status` live in the runtime layer — the edges-022 LESSON-17 rule).
  Rebuild RESETS the live-read cache (not event-sourced); the edges-022 rebuild-equivalence holds.
  _(+ `style(edges-026)` follow-up `d800ef1`: cargo-fmt the 5 reflow files — c195c7f shipped
  unformatted, the 407be7c precedent; ZERO behavior change.)_
- **edges-027** P5.4 **§18 `project.rescan` detection-latency benchmark** (NON-TDD bench) — LANDED
  `44ce907` (the bench IS the coverage, no RED→GREEN; the event_write.rs precedent).
  NEW `benches/project_rescan.rs` (`fn main()`, harness=false) + the `[[bench]]` Cargo.toml entry; drives the
  AS-BUILT detection core (`detect_git`+`detect_workflow`) over a representative committed temp repo, 1000
  warm-cache iters → **median 0.44 ms** (p95 0.50/p99 0.60/max 1.07; the sub-ms band of the ~1.029 ms
  edges-007 baseline; ~6800× under the §18 3 s SLO). CI guard = **median < 50 ms** (LESSON 22 — tighter than
  the SLO, median-gated, gate-last). Invisible to `cargo test --workspace`; runs via `cargo bench`.

**R7 PLAN-DELTA additions (apply at the phase-exit merge; held):**
- **Arch-doc note (edges-027):** the §18 `project.rescan` perf budget is benched + guarded (median ~0.44 ms
  ≪ 3 s; guard median < 50 ms, calibrated tighter than the SLO per LESSON 22).
- **Held-for-merge (edges-027):** register `project_rescan` in the `/phase-exit` perf row + `.github/
  nightly.yml` at the edges→main merge (CI files are shared-root; the bench target + file land now, the CI
  registration is the merge note).
- **Convention candidate (process gap, edges-026/027 — held-for-merge):** `/tdd` Step 8 runs `check`+`clippy`
  but NOT `cargo fmt --check`, so a slice can ship unformatted (edges-026 `c195c7f` did → the `style` follow-up;
  the 0.5/407be7c precedent). FIX: add `cargo fmt --check` to `/tdd` Step 8 (or run `/preflight` per-slice) +
  an enforcement note in daemon/CLAUDE.md (the "fmt-check is FIRST" note exists for `/preflight`; extend it to
  the per-slice gate). NOT a lead escalation (code correct; fmt cosmetic; the round-seal fmt-check is the net).
  `(origin: 2026-06-13 edges-027)`
- **LESSON candidate (live-read-cache refresh pattern, edges-026)** — a non-Gateway/non-event write-actor
  command (the DrainOnce/ReapLeases family) + a git-watcher interval trigger + read-time `git_checked_at`
  staleness; a rebuild RESETS the cache (live-read, not event-sourced); persistence-core stays git-free
  (read+derive in the runtime layer). (LESSON 33 candidate — next-free after edges took §30/§31/§32.)
- **Arch-doc note (edges-026):** §7.2 worktree live-read cache is LIVE (the git-watcher task wired,
  ARCHITECTURE.md:340); `WorktreeStatusRefreshed`-is-NOT-an-event confirmed as-built (§7.1, the git-axis is a
  live-read projection cache).
- **Future TODO (overlay-source follow-on, MIGRATION_9-deferred, edges-026):** `status` recompute uses a
  hardcoded `Creating` overlay (the only emitted overlay). When `WorktreeMerged`/`Locked`/`Prunable`/… emitters
  land, a clean overlay source is needed (an `overlay` column = MIGRATION_9, or an event-sourced overlay read)
  — else a merged/locked worktree's status would wrongly re-derive to a git-axis value each watcher tick.
  Not testable without an overlay emitter. `last-consumer-slice: a worktree-overlay-emitter slice (post-merge)`.
- **SPREAD (UNIFIED, edges-026 + edges-023/024) — write-actor-I/O-offload hardening:** move slow write-actor
  I/O off-thread — covers the git-watcher git reads (edges-026; a BOUNDED local read, not the unbounded-network
  class) + `drain_once` outbox I/O + the edges-023/024 external executors (the bounded-network case, already
  timeout-guarded). ONE item. `last-consumer-slice: a write-actor-I/O-offload hardening slice`. `(origin:
  2026-06-13 edges-023; unified edges-026)`
- **Carry (edges-026):** the git-watcher reads `proj_worktree.path`, which is §15-redaction-masked for a
  high-entropy component (tempdir hash) → the SAME over-redaction FP class as the edges-020 §7.2 return-review
  item (invariant HOLDS, production-low — real worktree paths low-entropy survive). NOT a new finding; folds
  into the existing §7.2-redacted-operational-inputs return-review.

- **FINDING — `cargo audit` (R7 ops task, orch-run): 1 NEW MEDIUM vs the P2 0-baseline** — RUSTSEC-2023-0071
  (`rsa` 0.9.10, Marvin Attack timing sidechannel, no fix), transitive via **octocrab → jsonwebtoken → rsa**
  (GitHub-App JWT auth). **Exposure LOW** — edges never exercises it (auth deferred; the planned `gh auth
  token`/OAuth model is bearer-token, NOT GitHub-App RS256-JWT; local trust boundary). **Disposition: accept-
  and-document** (medium, no fix, unexercised, local) → **human return-review** (surfaced to the lead in the
  R7 seal report). Full report: `docs/audits/edges-P5-P7-cargo-audit.md`. Preferred fix (a follow-up slice):
  octocrab `default-features = false` feature-prune to drop the unused jsonwebtoken/rsa app-auth path. CI:
  add the RUSTSEC-2023-0071 ignore + rationale to the `/phase-exit` dep-audit row + `.github/` at the merge.

**R7 COMPLETE (then PAUSE):** §7.2 live-read (edges-026) · P5.4 bench (edges-027, median 0.44 ms) · `cargo
audit` (1 new medium, accept-and-documented) — all DONE → seal R7 → edges PAUSES for the user-gated
`/phase-exit 5`+`7` + the edges→main merge (NOT run by edges — the user drives it with the daemon track +
the D8/MIGRATION_9 items: Wave-C `integration_connections` + the P5.1 registry projector).

---

## R8 — the phase-exit/merge round, STEP 1: main→edges re-sync (USER topology A)

**User chose topology A** (finish edges phase-exit → user coordinates edges→main → main→ui). R8 STEP 1 =
re-sync edges with main's latest before the phase-exit (the slices are R9, a fresh post-cycle pair).

- **MERGE `536ac04`** (2 parents `1f1f14f` edges + `df19f89` main; 47 main commits) — absorbed **CONTRACT
  0.26→0.32** (daemon Phase-4 [4.0b-2 interception · 4.0c telemetry · 4.1a/b survival+tmux · 4.2 SessionFailed
  · 4.3 bg jobs] + Codex-3.3). Edges' `shared/` untouched → clean absorb. **7 conflicts** = additive unions +
  2 reconciliations (git/mod.rs add/add: edges' git submodules + main's `read_diff` ui-backend COEXIST ·
  main.rs CAT-1: edges' 4 executors folded into main's live INV-SEC-1 drive loop under AgentMutationPolicy +
  SessionExecutor + alarm/breaker). **2 beyond-plan resolutions (impl-caught):** a `[dependencies]` git2 TOML
  dup-key dedup (kept edges' vendored) + Cargo.lock `--theirs`+regen. **Green 760/0/0** (edges 620 + main's
  arc); both cat-1 pins pass; no semantic drift.
- **SECURITY (the load-bearing gate) — `INV-SEC-1: PASS (no-bypass confirmed)`** (6 criteria, file:line):
  edges' 4 mutators reachable ONLY via Gateway policy→approval→execute→audit (sole exec `pipeline.rs:976`;
  executors hold no WriteHandle/SQL); AgentMutationPolicy only raises agent.*→Deny else CatalogPolicy → edges'
  risk-2/3 STILL approval-gated, risk-0 project.rescan auto = read-only intentional; FailedWithEvents audited
  atomic+breaker-gated; §15 secrets clean.
- **D8 RESOLVED:** main holds **MIGRATION_9 = `MIGRATION_9_POLICY_DECISION`** (②-mini, 0.30.0) → edges' Wave-C
  `integration_connections` = **MIGRATION_10** (next-free; no renumber — Wave-C unbuilt). The R5 deferral vindicated.
- **cargo audit (merged tree):** same single finding (RUSTSEC-2023-0071, rsa) — NO new advisories from main's
  P4/Codex deps; the R7 accept-and-document still covers it.
- **PLAN-DELTA STAYS HELD:** the R5–R7 PLAN-DELTA (LESSONs 30/31/32/33-cand · arch-notes · SPREADs · ticks ·
  the cargo-audit FINDING · the fmt-gap convention) is UNCHANGED — the R8 main→edges merge did NOT apply it
  (cross-track rule holds: the daemon track is live on main at 3.3c; edges editing the merged-in shared root
  docs would re-conflict at edges→main). The integration owner applies it at the edges→main merge.
- **R8 sealed (merge clean cut) + CYCLE BOTH** → fresh R9 pair. Seal: merge `536ac04` + impl doc `ae1106e`
  (edges-015) + orch seal edges-016. NOT pushed, NOT merged.

**R9 (fresh phase-exit pair) target:** Wave-C `integration_connections` (MIGRATION_10) + `IntegrationConnectionRegistered`
(keychain_ref pointer-ONLY §15 #4; security-reviewer + INV-SEC-1 + LESSON-31 guard) + the P5.1 registry projector
(projects/repositories) + `/phase-exit 5`+`7` (verify-only — the gated §6.3/§15/§8 anchors now LIVE) → SEAL →
HOLD for the user's edges→main coordination (NOT run by edges).

---

## R9 — the FINAL phase-exit round (P5/P7.1 wiring COMPLETE + the gate) — accumulated hot-routing (apply at the edges→main merge)

**Round shape (USER topology A — finish edges phase-exit before the user coordinates edges→main):** 3 TDD slices + `/phase-exit 5`+`7` (verify-only) → seal → HOLD. All slices test-first, Step-2.5-reviewed; sealed LOCAL on `track/edges`, **NOT pushed, NOT merged**.

**Slice ledger:**
- **edges-028** P5.1 **project-registry projector** — LANDED `8788210` (CONTRACT-neutral; 9 tests; security SKIP). Event-fed `ProjectRescanned` → `proj_project` (identity axis) + `proj_repository` (git-detection axis, 1:1 MVP keyed by project_id) + **MIGRATION_10** (SUPPORTED_USER_VERSION 9→10); both tables in `REBUILD_TABLES`, rebuild-equivalent (LESSON 4/17). NO sibling-read (the payload is self-contained, identity on the envelope — simpler than edges-022). Healthy-skip on missing project_id; Decode-degrade (offset degraded, not advanced) on unbindable payload + the non-git/all-NULL boundary pinned. `remote_url` folds through the already-stripped+redacted committed value.
- **edges-029** P7.1 **Wave-C `integration.connect` mutator** — LANDED `355eddf` (**CONTRACT 0.32→0.33**; 8 tests; **security-reviewer full invariant PASS**; 1 commit — the §15 #4 pin is STRUCTURAL/inseparable, the LESSON-39 carve-out). `ExecutorKind::Integration` (NEW) + `integration.connect` (risk-2) + `IntegrationExecutor` + `IdGen::new_connection_id` (`conn_`). **REGISTRATION-ONLY** (§15+LESSON-20-forced — a risk-2 action executes off the §15-redacted durable row → a token in inputs is masked by execute-time → the mutator STRUCTURALLY cannot carry the token): inputs `{provider, keychain_ref pointer, account?}`, NO token; §15 #4 holds by construction. Defense-in-depth: a secret-shaped keychain_ref rejected via the canonical `PrefixRedactor` read-only (LESSON 13, no parallel detector). INV-SEC-1 no-bypass (holds only `Box<dyn IdGen>`, emits via `emitted_events`, reachable ONLY via the catalog-gated pipeline, risk-2→approval, NOT on the risk-0 auto-execute allowlist). `side_effect_applied=false`.
- **edges-030** P7.1 **Wave-C integration-connections projector** — LANDED `25e0833` (CONTRACT-neutral; 8 tests; security SKIP). `IntegrationConnectionRegistered` → `proj_integration_connection` (keyed by **payload.connection_id**, not the envelope) + **MIGRATION_11** (SUPPORTED_USER_VERSION 10→11); in `REBUILD_TABLES`, rebuild-equivalent. `status='connected'` plain TEXT (no frozen §5.1 Connection machine). **P5.1 + P7.1 Wave-C verticals CLOSED.**

**`/phase-exit 5` + `/phase-exit 7` — VERDICT: CLEAR (verify-only); NO BLOCKED.**
- **arch-drift (both): CLEAR — 0 drift** (reports `docs/audits/P5-arch-drift.md`, `docs/audits/P7-arch-drift.md`). All known-deferred items confirmed deferred-NOT-drift.
- **reachability (both): CLEAR** (reports `docs/audits/P5-reachability.md` [20 reachable / 2 gated / 9 forward-laid `git/reads.rs` helpers], `docs/audits/P7-reachability.md` [53 reachable / 0 unreachable / 7 intentionally-gated]).
- **spec-coverage:** tests 5 PASS; tests 7 PASS on §9/§17/§7.2/§6.3/§8 — **§11.2 needs a ui-track waiver** (below).
- **dependency (cargo audit):** unchanged from R8 — RUSTSEC-2023-0071 (rsa, medium, no-fix) the single accept-and-documented finding; R9 added NO deps.
- **test-count verification (lead-requested):** workspace `cargo test --workspace` = **785/0** = R8's 760 + the 25 R9 additions (028:9 · 029:8 · 030:8); the impl's per-slice "707" = daemon-crate-only (`-p nexusopsd`). Coverage UP, none lost. BENIGN scope difference.

### HELD-for-merge PLAN-DELTA (R9 — apply at the user-gated edges→main merge; edges does NOT edit the shared root docs in-worktree)

- **🟡 PROMINENT merge-ledger (USER-directed):** **CONTRACT 0.32 (main) → 0.33 (edges)** — the `integration.connect` catalog action_type + `ExecutorKind::Integration` (the ONLY edges catalog add since the merge — confirmed `git log 536ac04..HEAD -- shared/src/catalog.rs` = `355eddf` only; the session.kill/profile_change + git.stage/unstage/discard_hunk came IN via the R8 merge). `MVP_ACTION_TYPES.len()` = 28 (edges) vs 24 (main per the auditor count). **The daemon (catalog/CONTRACT owner) RATIFIES `integration.connect` + assigns the FINAL CONTRACT version at the merge** (like the MIGRATION numbers).
- **Completed-work ticks (HELD):** P5.1 read vertical CLOSED (executor edges-019 + emission + the registry projector edges-028) · P7.1 Wave-C connection vertical CLOSED (mutator edges-029 + projector edges-030). **Phases 5 + 7 stay OPEN** (5.3 ExecutionProfile = daemon-side/H1-gated; P7.2/7.3 = ui-track; the deferrals below) → the phase checkboxes do NOT tick.
- **MIGRATION_10 + MIGRATION_11** registered; `SUPPORTED_USER_VERSION` 9→11.
- **🔴 §6.2-floor FINDING (lead-endorsed; for the user's return-review ratification):** `integration.connect` `standing_grant_eligible=FALSE` (non-grantable) — security-reviewer-recommended (the eligibility axis is irreversibility/authorization-establishing blast-radius, NOT risk, LESSON 32; a credential/auth-establishing action always gets a per-action approval, the discard_hunk precedent). Fail-safe. Joins git.discard_hunk + workflow.command.invoke on the non-grant set; every-risk-4⇒non-grant invariant untouched (this is a risk-2 added on the authorization axis). USER ratifies: keep FALSE / relax to true.
- **§11.2 ui-track WAIVER (for the P7 Spec-anchor line at the merge):** §11.2 (PR Review Workspace UI) is ui-track; the edges read backend is covered by §7.2/§9. Add the waiver so `spec-lint tests 7` passes (mechanical; §11.2's ui-track scope is pre-established).
- **Arch-notes (DATA_MODEL §3 SoT-rule + §2.8 reconciliation):** MVP `projects`/`repositories`/`integration_connections` are realized as **event-fed projections** (`proj_project`/`proj_repository`/`proj_integration_connection`, rebuild-equivalent, proj_ naming), NOT the §2.8 durable-registry direct-write rows; the fuller durable-registry models (canonical rows + `register_project` mutator + workspace_id/scopes/expires + disconnect/refresh lifecycle) are DEFERRED. Lead-ruled at the R1b `ProjectRescanned` freeze. · The §15 #4 registration-only data-flow (the token never flows through the connect action). · **Stale-doc (arch-drift):** §6.3 MVP count "~21" → as-built 28; §18 bench 0.44/0.45 ms; §9 auth-bootstrap-deferred not noted; §11.2 mergeable/checks_summary not persisted (MVP trade-off).
- **Lesson candidates → `daemon/LESSONS.md` (renumber §44+ at the merge — edges' R5–R7 §30–§33 candidates COLLIDE with daemon's merged-in §30–§43):** (1) **event-fed registry projection** — a `proj_*` registry projection is a projection, not the §2.8 durable registry: fold the coarse event, key by the available identity (envelope for edges-028 / payload for edges-030), both tables in `REBUILD_TABLES`, defer the canonical-row/mutator model. (2) **credential-registration mutator = registration-ONLY** — the secret never flows through a risk-≥1 action (LESSON 20 masks it at execute-time); the action carries the keychain_ref POINTER, the token→keychain write is a separate non-Gateway mechanism; reuse the canonical §15 detector read-only as the defense-in-depth pointer-shape reject. (3) **migration-floor-test convention** — a per-migration "applies" test asserts its FLOOR (`user_version >= N`) + table existence, NOT exact-latest (else every migration breaks the prior's test); ONE exact-latest runtime pin (gateway_plan.rs) is the double-bump guard; `assert!(CONST >= N)` trips clippy's const-assertion lint → use the runtime `store.user_version()`.
- **Carry-forward (consumer-gated, HELD):** the §9 read-set IPC RPCs (the 9 forward-laid `git/reads.rs` helpers — `list_linked_worktrees`/`read_diff` ref-vs-ref/`read_file_hunks`/`read_log` — Phase-6 IPC territory) · the IPC read RPCs for `proj_project`/`proj_integration_connection` (a `ProjectionName::Project`/`::IntegrationConnection` variant → CONTRACT-bumping; lands with a ui consumer) · the **token→keychain credential-storage** (the `keyring` crate + a non-Gateway secret-store path + the live macOS-keychain write — HITL/live-integration follow-on, folds with H1; + a fresh `cargo audit` when keyring lands) · the durable-registry fuller models · the `ProjectRescanned` → `proj_project_activity` + graph folds · the `auth_expired` `*SyncFailed` variant (non-auth shipped) · the prior R5–R8 parks (the §7.2 redacted-inputs over-redaction return-review · write-actor-I/O-offload · subscribe-delta [daemon-owned]).

**R9 SEALED (final edges in-lane seal — P5/P7.1 wiring + phase-exit COMPLETE).** Slices `8788210`/`355eddf`/`25e0833` + impl doc edges-017 `074465f` + this orch seal (edges-018). **NOT pushed, NOT merged.** edges HOLDS at COMPLETE for the user's edges→main coordination (the user drives it with the daemon track — the §5.0 contract reconcile [0.33 ratify] + the held PLAN-DELTA above + the D8/MIGRATION numbers apply at that merge).

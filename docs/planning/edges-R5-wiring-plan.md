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
- **TODO (live-read status refresh; P5.2 follow-on):** `read_worktree_status` (git/reads.rs exists) →
  `proj_worktree` dirty_state/ahead/behind/git_checked_at via `derive_worktree_status` (§7.2 live-read cache).
  Its own slice; the projector's ON CONFLICT DO UPDATE preserves those columns on re-fold/rebuild. The
  rebuild-compare coverage boundary (5 always-NULL columns) revisits here.
- **Completed-work ticks (hold):** P5.1 = PARTIAL (executor + emission landed edges-019; registry projector
  MIGRATION_9-deferred). P5.2 = mutators (edges-020/021) + read vertical (edges-022) COMPLETE; live-read status
  refresh deferred (TODO above).
- **Cross-track surfaces (lead-aware):** edges touches `gateway/executor.rs` + `gateway/request.rs` (the
  `EmittedEvent::Namespaced` bridge — 1 variant + 1 arm, additive, edges-owned per the lead) + MIGRATION_9
  **deferred to the final merge (D8)** — Wave-C takes the then-next-free number after the daemon's schema settles.
  Both lead-ruled + logged to the cross-track ledger for the final merge.

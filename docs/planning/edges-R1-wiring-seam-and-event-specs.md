# edges → daemon-track cross-track ask (R1): the executor registration seam + wiring event-type specs

> **Status:** consumer-driven DESIGN PROPOSAL from the `edges` track (Approach A, lead-decided 2026-06-12). The **daemon track owns the final shape** of everything here — `gateway/` dispatch + `shared/` `EventTypeRegistry` + `CONTRACT_VERSION` are daemon-owned. This doc is what edges needs in order to land its **wiring** slices (the deferred half of P5/P7.1); edges builds all in-lane logic + private registry migrations meanwhile.
>
> **Why the seam is shared infra (not just an edges convenience):** the same per-namespace registration the edges executors need (`Project`/`Git`/`Github`/`Linear`) is what the **daemon's own Phase-3 `session.*` arms** (`ExecutorKind::Session`) need to stop being stubs. Building it once unblocks both tracks.
>
> **Decision log (lead rulings 2026-06-12, edges-automated authority):** the open choices below are tagged: **PROVISIONAL-pending-daemon** (daemon-owned gateway/shared surface — edges' lean is a recommended default only) vs **DECIDED** (edges-side in-lane modeling, within edges authority). (a) async = PROVISIONAL (+ caveat: a sync+`block_on` handler MUST run on a dedicated blocking context — `spawn_blocking` + `Handle::block_on`; `block_on` on a tokio worker panics). (b) `ProjectRescanned` granularity = PROVISIONAL (either acceptable). (c) worktree status-refresh = **DECIDED live-read cache** (lead-endorsed; edges read/projector territory). `*SyncFailed` = **DECIDED split** — the non-auth variant lands first; the `auth_expired` variant is DEFERRED with H1 (0.5b `ExecutionProfile` gate).

---

## Part 1 — The per-namespace `ActionExecutor` registration seam

### Current state (`daemon/src/gateway/executor.rs`)
`CatalogExecutor::execute(req)` resolves the catalog entry, then dispatches by `entry.executor` (`ExecutorKind`) via an **inline match that returns a side-effect-free stub** ("would execute via {namespace} (Phase N)"). To make a namespace real today, you'd edit that match — i.e. edit sealed `gateway/`. That's the contamination Approach A forbids.

### Proposed shape (consumer-driven; daemon owns final)
Turn `CatalogExecutor` into a **registry** that delegates to per-namespace handlers, falling back to the current stub for any unregistered namespace (so registration is **incremental** — a namespace goes live exactly when its handler is registered; everything else behaves as today):

```rust
// gateway/executor.rs (daemon-owned)
pub struct CatalogExecutor {
    // ExecutorKind → the namespace's real handler; absent ⇒ stub (today's behavior)
    handlers: HashMap<ExecutorKind, Arc<dyn ActionExecutor>>,
}

impl CatalogExecutor {
    pub fn new() -> Self { Self { handlers: HashMap::new() } }

    /// Register a namespace's real executor. Called by the daemon at startup wiring.
    pub fn register(&mut self, kind: ExecutorKind, handler: Arc<dyn ActionExecutor>) {
        self.handlers.insert(kind, handler);
    }
}

impl ActionExecutor for CatalogExecutor {
    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        let entry = match self.resolve(req) { Ok(e) => e, Err(e) => return ExecutionOutcome::Failed(e.to_string()) };
        match self.handlers.get(&entry.executor) {
            Some(h) => h.execute(req),          // real namespace arm
            None    => /* today's structured stub, unchanged */,
        }
    }
    // validate/preview/rollback delegate the same way (registered handler else stub/catalog default)
}
```

**Edges' side of the contract:** each edges namespace module exposes a constructor returning `Arc<dyn ActionExecutor>` carrying its own deps — e.g. `git::executor::GitExecutor::new(cli_runner) -> Arc<dyn ActionExecutor>`, `integrations::github::GithubExecutor::new(octocrab, conn_registry)`. **Edges writes these IMPLs in its own modules** (`git/`, `integrations/`) — no `gateway/` edit. The daemon does the one-line `register(...)` wiring at startup.

### Two open design points for the daemon to decide
1. **Async. [PROVISIONAL-pending-daemon]** `ActionExecutor::execute` is **sync** (runs off the write-actor). Git-CLI = blocking (fine). **octocrab/Linear = async** → the github/linear handler must either `block_on` via a runtime handle held in the handler, **or** the trait gains an `async fn execute` variant. Edges' lean: keep the sync trait + `block_on` inside the handler (no trait churn, executor already runs off-actor) — **but the daemon owns this call** (if you prefer an async trait, edges adapts). **Caveat (lead):** a sync+`block_on` handler MUST run on a **dedicated blocking context** — `spawn_blocking` + `Handle::block_on`; calling `block_on` on a tokio worker thread **panics**. The executor already running off the write-actor makes this natural, but the daemon owns the placement.
2. **Registration ownership.** `register()` mutation vs. a builder/`Vec<(ExecutorKind, Arc<dyn ActionExecutor>)>` passed to `CatalogExecutor::new(...)`. Either works for edges; daemon picks.

**What edges needs delivered:** the seam (registry + `register`/builder + the stub fallback) merged on the daemon track, so edges' `register(ExecutorKind::Project, …)` etc. compiles against it.

---

## Part 2 — Event-type payload specs (`shared/src/events.rs` `EventTypeRegistry` additions)

Per the established pattern (identity — actor/`project_id`/`resource_refs`/etc. — on the **envelope columns**; **payload = delta only**; `deny_unknown_fields`; emitted ONLY via the Gateway write-actor append through the §15 redaction gate). Each new type = a `CONTRACT_VERSION` bump the **daemon track** authors. Edges supplies these specs; daemon finalizes names/fields/version.

### P5.1 wiring — project detection → projections
**`ProjectRescanned`** (emitted by the `project.rescan` executor after running edges' detection engine; consumed by `proj_project_activity` + the project graph + a new private `projects`/`repositories` registry).
| field | type | notes |
|---|---|---|
| `is_git` | `bool` | from `git::detect` |
| `repo_root` | `Option<String>` | canonical path; `None` for non-git |
| `remote_url` | `Option<String>` | **§15: route through the Redactor** — can embed `https://user:token@host` creds (edges-001 Step-9 flag) |
| `branch` | `Option<String>` | |
| `detached` | `bool` | |
| `is_dirty` | `bool` | |
| `workflow_pack` | `bool` | `.scaffolding/manifest.json` |
| `cc_crew` | `bool` | `.claude/` |
| `plan_file` | `Option<String>` | `MVP_TASKS.md`\|`IMPLEMENTATION_PLAN.md` |
| `brain` | `bool` | `.brain` presence (provisional; Phase-8 reconcile) |
| `scanned_at` | `Timestamp` | |

- **Sensitivity:** `internal` (the `remote_url` redaction is the load-bearing §15 point).
- **Open choice [PROVISIONAL-pending-daemon]:** one `ProjectRescanned` carrying both project + repo facts (projector splits into `projects`/`repositories` rows) **vs.** separate `ProjectRegistered` + `RepositoryDetected`. Edges' lean: **one event** (a rescan is atomic; simpler). Daemon owns the modeling call (lead: either acceptable).

### P5.2 wiring — worktree lifecycle + status
- **`WorktreeCreated`** (from `git.create_worktree`): `worktree_id`, `path`, `branch_name`, `base_branch?`. → `proj_worktree` row (status via edges' `derive_worktree_status`).
- **`BranchCreated`** (from `git.create_branch`): `branch_name`, `base?`.
- **`WorktreeStatusRefreshed`** (from the git watcher / a refresh pass): the git-axis (`dirty_state`), `ahead_count?`, `behind_count?`, `last_commit_sha?`, `git_checked_at`. → updates `proj_worktree` (the projector applies edges' precedence fn against the overlay axis).
  - **[DECIDED — live-read cache, lead-endorsed 2026-06-12]:** the git-axis status is a **live-read projection-cache update** — the projector reads git2 on demand, `git_checked_at` is the cache stamp, **no event per poll** (§7.2 already models exactly this; avoids watcher event-log spam AND reduces contract surface). Lifecycle transitions (`created`/`merged`/`deleted`/overlay) remain **events** (`WorktreeStatusRefreshed` is NOT emitted per status poll). This is edges read/projector territory → treated as the in-lane modeling default, not pending-daemon.
- Overlay-axis transitions (`WorktreeMerged`/`WorktreePrunable`/`WorktreeDeleted`/`WorktreeLocked`) — emit as the lifecycle warrants; feed the overlay axis of `derive_worktree_status`.

### P7.1 wiring — integration reads + sync failures
- **`PullRequestSynced`** (from a github PR-sync executor): `pr_number`, `status` (the §5.1 `PullRequest` enum value), `branch`, `base`, `mergeable?`, `checks_summary?`, `pr_checked_at`. → `proj_pull_request` (GitHub-authoritative cache, §7.2). **Sensitivity:** `internal`.
- **`IntegrationConnectionRegistered`** (from connecting GitHub/Linear): `connection_id`, `provider` (`github`\|`linear`), `keychain_ref` (**a pointer only — §15 rule #4, never the secret**), `account?`. → a private `integration_connections` registry. **Sensitivity:** `internal` (keychain_ref is a non-secret pointer).
- **`GithubSyncFailed` / `LinearSyncFailed`** (the §17 line-450 `*SyncFailed`, driven by the edges classifier's terminal class): `provider`, `reason` (a **redaction-safe STRUCTURAL** string — the classifier's class name, not raw API text), `failed_at`. **[DECIDED — split, lead 2026-06-12]:** the **non-auth** `*SyncFailed` (a terminal `ClientError` sync failure, no profile mutation) **lands first**; the **`auth_expired` variant** (driven by the classifier's `AuthFailed` class → `ExecutionProfile→auth_expired` + "re-authenticate" card) is **DEFERRED with H1** (the 0.5b `ExecutionProfile` unfreeze, cat-4 HITL gate). **Hard constraint (security):** when the auth variant lands, it MUST branch on `IntegrationOutcomeClass::AuthFailed` — NOT the collapsed `DeliveryOutcome::Terminal` (edges-003 security forward-note) — and warrants a §17/INV-SEC security re-review.

---

## Part 3 — What each real namespace executor will need (deps the daemon constructs at startup)
| `ExecutorKind` | edges module (impl) | deps to inject |
|---|---|---|
| `Project` | `git::detect` + a `project` executor | the detection engine (landed edges-001); a `Clock`/`IdGen` for the event |
| `Git` | `git` (CLI runner) | a git-CLI runner (mutations are CLI per forbidden #6); edges' `reads`/`precedence` (landed edges-002) for the re-read-before-mutate (§7.2) |
| `Github` | `integrations::github` | `octocrab` client + the `integration_connections` registry + the classifier (landed edges-003) |
| `Linear` | `integrations::linear` | the Linear client + connections registry + the classifier |

(Edges builds these IMPLs + their private migrations in-lane now; they stay unregistered/dormant until the seam lands.)

---

## Summary — what edges needs from the daemon track (the R1 ask)
1. **The registration seam** (Part 1) merged on the daemon track — the single highest-leverage unblock (serves the daemon's own `Session` arms too).
2. **The event types** (Part 2) added to `shared/src/events.rs` + a `CONTRACT_VERSION` bump — batched or per-namespace, daemon's call. Edges regenerates + consumes via the §5.0 mechanism.
3. **Resolutions** on the flagged open choices (async trait, one-vs-split project event, status-refresh event-vs-cache, SyncFailed-vs-ExecutionProfile-gate) — daemon-owned.

Edges keeps producing in-lane logic + private migrations against these specs; nothing here blocks edges' current slice flow.

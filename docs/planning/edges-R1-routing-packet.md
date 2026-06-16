# R1 ROUTING PACKET — edges → daemon track (cross-track unblock)

> **One-page hand-off** the edges lead routes (user → daemon track). Condenses the full spec
> `docs/planning/edges-R1-wiring-seam-and-event-specs.md` (field-level detail lives there). **The daemon
> track owns the final shape** of everything here — `gateway/` dispatch, `shared/` `EventTypeRegistry`,
> `CONTRACT_VERSION` are daemon-owned. This is a **consumer-driven design proposal** (Approach A,
> lead-decided 2026-06-12), not a directive.
>
> **Why it's shared infra, not an edges favor:** the per-namespace registration seam edges needs
> (`Project`/`Git`/`Github`/`Linear`) is the **same** seam the daemon's own Phase-3 `session.*` arms
> (`ExecutorKind::Session`) need to stop being stubs. Build once → unblocks both tracks.
>
> **Staleness note:** edges is based `a40ac00`; main has advanced to the daemon Phase-3 seals. If the
> daemon track has **already** introduced per-namespace executor dispatch for its `session.*` arms, Part 1
> may be partly delivered — reconcile the actual seam shape at the P5/P7.1 phase-exit merge. The shapes
> below are edges' requirement regardless of where the daemon surface currently sits.

---

## THE ASK (4 deliverables)
1. **The executor registration seam** (Part 1) merged on the daemon track — highest-leverage unblock; serves the daemon's own `Session` arms too.
2. **The wiring event types** (Part 2) added to `shared/src/events.rs` `EventTypeRegistry` + a `CONTRACT_VERSION` bump (batched or per-namespace, daemon's call). Edges regenerates + consumes via the §5.0 mechanism.
3. **Resolutions** on the 3 daemon-owned design choices (below). The other two open points are already DECIDED edges-side (worktree status = live-read cache; `*SyncFailed` = split, non-auth first).
4. **A `test-support` cargo-feature** (Part 4 — folded in at R4) — introduce `[features] test-support` in `daemon/Cargo.toml` gating the crate's test doubles (`FakeHarness` + edges' two read-client fakes) out of the release binary; edges then consumes it for its own two fakes.

---

## PART 1 — the per-namespace `ActionExecutor` registration seam

**Current state (`daemon/src/gateway/executor.rs`, edges base `a40ac00`):** `CatalogExecutor` is a **unit
struct** whose `execute` returns **one uniform side-effect-free stub for every namespace** — there is **no
per-`ExecutorKind` dispatch arm at all** (the kind only labels the `detail` string via
`preview::namespace_label`). To make a namespace real today you'd edit sealed `gateway/` — the
contamination Approach A forbids.

**Proposed shape (daemon owns final):** make `CatalogExecutor` a registry that delegates to per-namespace
handlers, **falling back to today's stub for any unregistered kind** (so registration is *incremental* — a
namespace goes live exactly when its handler registers; everything else is unchanged):

```rust
// gateway/executor.rs (daemon-owned)
pub struct CatalogExecutor { handlers: HashMap<ExecutorKind, Arc<dyn ActionExecutor>> }
impl CatalogExecutor {
    pub fn register(&mut self, kind: ExecutorKind, h: Arc<dyn ActionExecutor>) { self.handlers.insert(kind, h); }
}
// execute(): resolve catalog entry → handlers.get(entry.executor) → Some(h) ? h.execute(req) : <today's stub>
// validate/preview/rollback delegate the same way (registered handler else stub/catalog default)
```

**Edges' side of the contract:** each edges namespace module exposes a constructor returning
`Arc<dyn ActionExecutor>` carrying its own deps (`git::executor::GitExecutor::new(cli_runner)`,
`integrations::github::GithubExecutor::new(octocrab, conn_registry)`, …). **Edges writes these impls in
`git/`+`integrations/` — no `gateway/` edit.** The daemon does the one-line `register(...)` at startup wiring.

---

## PART 2 — wiring event-type payload specs (envelope identity; payload = delta; `deny_unknown_fields`; emitted ONLY via the Gateway append through the §15 redaction gate)

| Phase | Event | Key payload fields | Load-bearing |
|---|---|---|---|
| P5.1 | **`ProjectRescanned`** | `is_git, repo_root?, remote_url?, branch?, detached, is_dirty, workflow_pack, cc_crew, plan_file?, brain, scanned_at` | **`remote_url` MUST route through the Redactor** — can embed `https://user:token@host` creds (§15; edges-001 flag). Sensitivity `internal`. |
| P5.2 | **`WorktreeCreated`** · **`BranchCreated`** · overlay-axis lifecycle (`WorktreeMerged`/`Prunable`/`Deleted`/`Locked`) | `worktree_id, path, branch_name, base_branch?` / `branch_name, base?` | git-axis status is a **live-read cache, NOT an event** (DECIDED — see below). Lifecycle transitions stay events. |
| P7.1 | **`PullRequestSynced`** | `pr_number, status (§5.1 PullRequest enum), branch, base, mergeable?, checks_summary?, pr_checked_at` | → `proj_pull_request` (GitHub-authoritative cache §7.2). Sensitivity `internal`. |
| P7.1 | **`IntegrationConnectionRegistered`** | `connection_id, provider (github\|linear), keychain_ref, account?` | **`keychain_ref` = a non-secret POINTER only (§15 rule #4 — never the token).** Backs the private `integration_connections` table (deferred — D5 migration gate). |
| P7.1 | **`GithubSyncFailed` / `LinearSyncFailed`** | `provider, reason (STRUCTURAL class name — redaction-safe, not raw API text), failed_at` | **Non-auth variant first** (DECIDED split). The `auth_expired` variant is DEFERRED with H1 (0.5b `ExecutionProfile` gate) and MUST branch on `IntegrationOutcomeClass::AuthFailed`, never the collapsed `DeliveryOutcome::Terminal` — warrants a §17/INV-SEC re-review when it lands. |

*(Full field tables + projector/consumer mapping: the source spec doc Part 2.)*

---

## THE 3 DAEMON-OWNED DESIGN CHOICES (edges' leans — daemon decides)

1. **Async model.** `ActionExecutor::execute` is **sync** (runs off the write-actor). Git-CLI is blocking
   (fine); octocrab/Linear are **async**. **Edges' lean:** keep the sync trait + `block_on` inside the
   github/linear handler (no trait churn). **Hard caveat:** a sync+`block_on` handler MUST run on a
   **dedicated blocking context** (`spawn_blocking` + `Handle::block_on`) — `block_on` on a tokio worker
   thread **panics**. Alternative: add an `async fn execute` trait variant (cleaner, but reopens the frozen
   2.3 executor trait). Neither is free — daemon owns this (touches the frozen trait).
2. **`ProjectRescanned` granularity.** **Edges' lean:** one coarse event (rescan is atomic; the projector
   splits into `projects`/`repositories` rows). Alternative: split `ProjectRegistered` + `RepositoryDetected`
   (mild event-sourcing argument given two registry aggregates). Either acceptable — daemon's modeling call.
3. **Registration ownership.** `register()` mutation vs. a builder / `Vec<(ExecutorKind, Arc<dyn ActionExecutor>)>`
   passed to `CatalogExecutor::new(...)`. Either works for edges — daemon picks.

**Already DECIDED edges-side (not pending daemon):** (a) worktree git-axis status = **live-read projection
cache** (`git_checked_at` stamp, no per-poll event — §7.2 already models this; reduces contract surface);
(b) `*SyncFailed` = **split**, non-auth lands first, `auth_expired` deferred with H1.

---

---

## PART 4 — `test-support` cargo-feature (folded in at R4; daemon-owned, covers all three fakes)

**Problem:** the crate's test doubles ship in the RELEASE binary. `FakeHarness` (`harness/mod.rs`,
daemon-track-owned), `FakeLinearReadClient` (`integrations/linear.rs`) + `FakeGithubReadClient`
(`integrations/github.rs`) are all `pub` + ungated — because the integration tests in `daemon/tests/`
link the lib as an external crate (WITHOUT `cfg(test)`) and need them public. Result: release dead-weight +
an unintended public test surface.

**Ask (daemon-owned — the same shared-manifest surface as the existing `fault-injection` feature):**
introduce `[features] test-support = []` in `daemon/Cargo.toml` and enable it on the self-dev-dependency —
`nexusopsd = { path = ".", features = ["fault-injection", "test-support"] }` (the LESSON §21 idiom already
in place for `fault-injection`). Gate each fake `#[cfg(feature = "test-support")]`. Then
`cargo build --release` excludes all three; `cargo test` (the dev-dep enables it) includes them.

**Why daemon-owned / cross-track (not an edges-unilateral slice):** the `[features]` block + the self-dev-dep
line are a SHARED `daemon/Cargo.toml` surface both tracks edit (D5-analogous merge contention), AND
`FakeHarness` is daemon-track-owned (outside edges' `git/`+`integrations/` lane). Introducing the feature
ONCE, covering all three fakes, avoids both a half-measure and a merge conflict. **Division of labor:**
the daemon adds the manifest stanza + the `FakeHarness` cfg; **edges gates its own two `integrations/`
fakes** once the feature exists. (Routed R4 per the edges lead/user — folded here rather than shipped as
a separate edges in-lane slice.)

---

**Bottom line:** Part 1 (the seam) is the single unblock that lights up ALL edges wiring slices *and* the
daemon's own `session.*` arms. Until it lands, edges stays on in-lane read/refinement work. Nothing here
blocks edges' current slice flow.

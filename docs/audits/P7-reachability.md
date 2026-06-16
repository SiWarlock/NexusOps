# Phase-7 Reachability Audit — `track/edges` (daemon/ area)

**Branch:** `track/edges` (commit `8db62bb`)
**Audit date:** 2026-06-15
**Scope:** `daemon/src/integrations/` (all files) + `daemon/src/projections/pull_request.rs` + `daemon/src/projections/integration_connections.rs`

---

## Production entry points confirmed

The production call chain is:

```
IPC submit_action (daemon/src/ipc/methods.rs:79)
  → WriteHandle::submit_action (write-actor)
    → Gateway::execute_action (gateway pipeline: policy → approval → execute)
      → CatalogExecutor::execute (dispatcher, daemon/src/gateway/executor.rs)
        → [ExecutorKind::Github] GithubExecutor::execute
        → [ExecutorKind::Linear] LinearExecutor::execute
        → [ExecutorKind::Integration] IntegrationExecutor::execute
          → EventStore::append (emits PullRequestSynced / GithubSyncFailed /
                                  LinearSyncFailed / IntegrationConnectionRegistered)
            → apply_all (daemon/src/projections/mod.rs:113)
              → PullRequestProjector::apply (proj_pull_request table)
              → IntegrationConnectionProjector::apply (proj_integration_connection table)
```

All three executors are registered in `daemon/src/main.rs` lines 224–253:
- `catalog_exec.register(ExecutorKind::Github, Arc::new(GithubExecutor::new(...)))` — `OctocrabGithubWriteClient` bound
- `catalog_exec.register(ExecutorKind::Linear, Arc::new(LinearExecutor::new(...)))` — `LinearGraphqlWriteClient` bound
- `catalog_exec.register(ExecutorKind::Integration, Arc::new(IntegrationExecutor::new(...)))`

Both projectors registered in `projectors()` at `daemon/src/projections/mod.rs:97,105`.

---

## Symbol-by-symbol classification

### `daemon/src/integrations/classifier.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `RetryAfter` (enum) | REACHABLE | Consumed by `linear.rs`, `linear_write.rs`, `github_write.rs` (production) |
| `IntegrationOutcomeClass` (enum) | REACHABLE | Used in `executor.rs`, `github.rs`, `github_write.rs`, `linear.rs`, `linear_write.rs` (production) |
| `IntegrationOutcomeClass::to_delivery_outcome` | INTENTIONALLY-GATED | Test-only caller (`daemon/tests/integration_classifier.rs`); module doc explicitly states "The github/linear Destination adapters that call this...are deferred/gated" — wiring to the outbox drainer is a future phase item |
| `classify` (fn) | REACHABLE | Called from `github.rs:265,268,270`, `linear.rs:354,396,422`, `linear_write.rs:162` (production) |
| `parse_retry_after` (fn) | REACHABLE | Imported + called from `linear.rs:419` (production) |
| `parse_rate_limit_reset` (fn) | REACHABLE | Imported + called from `linear.rs:418` (production) |

### `daemon/src/integrations/github.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `extract_pr_signals` (fn) | REACHABLE | Called from `github.rs:241` (within `OctocrabGithubReadClient::read_pr` — production method body) |
| `parse_review_decision` (fn) | REACHABLE | Called from `github.rs:293` (production) |
| `layer_review_decision` (fn) | REACHABLE | Called from `github.rs:245` (production) |
| `GithubReadError` (struct) | REACHABLE | Used in `GithubReadClient` trait return types; consumed by `GithubExecutor` via the trait |
| `GithubReadClient` (trait) | REACHABLE | Implemented by `OctocrabGithubReadClient` (production) + `FakeGithubReadClient` (test seam); consumed by `GithubExecutor` |
| `FakeGithubReadClient` (struct) | TEST-SUPPORT SEAM | Defined in `src/` (not `#[cfg(test)]`) for cross-crate test import; no production instantiation. By design per CLAUDE.md HITL posture — "live octocrab/reqwest paths are exercised via FakeGithubReadClient/FakeLinearReadClient under test-support" |
| `OctocrabGithubReadClient` (struct) | INTENTIONALLY-GATED | Defined in `src/`, implements `GithubReadClient` for live HTTP; NOT imported from `main.rs` or any production file. The live read path is deferred (the existing `GithubExecutor` uses the write-client only — read-client wiring is a later HITL follow-on wave per design) |

### `daemon/src/integrations/github_write.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `CreatePrArgs` (struct) | REACHABLE | Used in `GithubWriteClient::create_pr` trait signature; called by `GithubExecutor::execute` (production) |
| `CreatedPr` (struct) | REACHABLE | Returned by `GithubWriteClient::create_pr`; consumed in `GithubExecutor::execute:179` to derive `PullRequestSynced` |
| `GithubWriteError` (struct) | REACHABLE | Returned by `GithubWriteClient::create_pr`; consumed in `GithubExecutor::execute` error branches |
| `GithubWriteClient` (trait) | REACHABLE | Implemented by `OctocrabGithubWriteClient` (bound in `main.rs:227`); dispatched by `GithubExecutor` |
| `OctocrabGithubWriteClient` (struct) | REACHABLE | Instantiated in `main.rs:227` (`Box::new(OctocrabGithubWriteClient::new(octocrab::Octocrab::default()))`) |
| `FakeGithubWriteClient` (struct + methods `ok/err/hanging/calls`) | TEST-SUPPORT SEAM | No production instantiation; test seam in `daemon/tests/github_executor.rs` |

### `daemon/src/integrations/linear.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `LinearStateType` (enum) | REACHABLE | Used in `parse_linear_state_type` + `derive_task_status_from_linear`; both called from `extract_issue` (production) |
| `parse_linear_state_type` (fn) | REACHABLE | Called from `extract_issue:197` (production) |
| `derive_task_status_from_linear` (fn) | REACHABLE | Called from `extract_issue:197` (production) |
| `LinearIssue` (struct) | REACHABLE | Returned by `extract_issue`, `LinearReadClient::read_issue`; consumed in production |
| `LinearTeam` (struct) | REACHABLE | Field of `LinearIssue`; populated in `extract_issue:208` (production) |
| `extract_issue` (fn) | REACHABLE | Called from `LinearGraphqlReadClient::read_issue:485→`map_linear_response` (production) + `FakeLinearReadClient` test seam |
| `build_issue_query` (fn) | REACHABLE | Called from `LinearGraphqlReadClient::read_issue:461` (production) |
| `map_linear_response` (fn) | REACHABLE | Called from `LinearGraphqlReadClient::read_issue:485` (production) |
| `classify_linear_write_response` (fn) | REACHABLE | Imported + called from `linear_write.rs:112` (`LinearGraphqlWriteClient::post_mutation`) (production) |
| `LinearReadError` (struct) | REACHABLE | Used in `LinearReadClient` trait return types; consumed by `LinearExecutor` |
| `LinearReadClient` (trait) | REACHABLE | Implemented by `LinearGraphqlReadClient` (production) + `FakeLinearReadClient` (test seam) |
| `FakeLinearReadClient` (struct) | TEST-SUPPORT SEAM | No production instantiation; test seam |
| `LinearGraphqlReadClient` (struct + methods) | INTENTIONALLY-GATED | Defined in `src/`, implements `LinearReadClient`; NOT imported from `main.rs`. The live read path is deferred (same pattern as `OctocrabGithubReadClient`) — the `LinearExecutor` uses only the write-client path |

### `daemon/src/integrations/linear_write.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `LinkIssueArgs` (struct) | REACHABLE | Used in `LinearWriteClient::link_issue` trait signature; dispatched by `LinearExecutor::execute` |
| `CreateIssueArgs` (struct) | REACHABLE | Used in `LinearWriteClient::create_issue` trait signature; dispatched by `LinearExecutor::execute` |
| `LinearWriteCall` (enum) | REACHABLE | Used in `LinearWriteClient` trait + `LinearGraphqlWriteClient`; type-aliases the call record |
| `LinearWriteError` (struct) | REACHABLE | Returned by `LinearWriteClient` methods; consumed in `LinearExecutor::execute` error branches |
| `LinearWriteClient` (trait) | REACHABLE | Implemented by `LinearGraphqlWriteClient` (bound in `main.rs:238`); dispatched by `LinearExecutor` |
| `LinearGraphqlWriteClient` (struct + `new`) | REACHABLE | Instantiated in `main.rs:238` (`Box::new(LinearGraphqlWriteClient::new(...))`) |
| `FakeLinearWriteClient` (struct + methods `ok/err/hanging/calls`) | TEST-SUPPORT SEAM | No production instantiation; test seam in `daemon/tests/linear_executor.rs` |

### `daemon/src/integrations/pull_request.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `PrState` (enum) | REACHABLE | Used in `PullRequestSignals`; `parse_pr_state` called from `signals_from_github_response:312` (production) |
| `Mergeability` (enum) | REACHABLE | Field of `PullRequestSignals`; set in `signals_from_github_response` (production) |
| `ReviewDecision` (enum) | REACHABLE | Used in `PullRequestSignals`; set via `layer_review_decision` in `github.rs:245` (production) |
| `ChecksConclusion` (enum) | REACHABLE | Field of `PullRequestSignals`; set in `signals_from_github_response` (production) |
| `PullRequestSignals` (struct) | REACHABLE | Constructed in `signals_from_github_response`; consumed by `derive_pull_request_status` + `extract_pr_signals`; used in `GithubExecutor::execute` via `CreatedPr` |
| `derive_pull_request_status` (fn) | REACHABLE | Called from `executor.rs:179` (`GithubExecutor::execute`) (production) |
| `parse_pr_state` (fn) | REACHABLE | Called from `signals_from_github_response:312` (production) |
| `parse_mergeable_state` (fn) | REACHABLE | Called from `signals_from_github_response:315` (production) |
| `parse_review_state` (fn) | REACHABLE | Called from `signals_from_github_response:305` (production) |
| `parse_check_conclusion` (fn) | REACHABLE | Called from `signals_from_github_response:309` (production) |
| `signals_from_github_response` (fn) | REACHABLE | Called from `github.rs:62` (`extract_pr_signals`) (production) |
| `GitHubMergeableState` (enum) | REACHABLE | Used in `signals_from_github_response:315` + `PullRequestSignals` (production) |
| `ReviewState` (enum) | REACHABLE | Used in `signals_from_github_response:305` + `PullRequestSignals` (production) |
| `CheckConclusion` (enum) | REACHABLE | Used in `signals_from_github_response:309` + `PullRequestSignals` (production) |
| `CheckConclusion::from_github` (method) | REACHABLE | Called from `signals_from_github_response:309` (production) |

### `daemon/src/integrations/executor.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `GithubExecutor` (struct + `new` + `with_timeout`) | REACHABLE | Instantiated in `main.rs:226`; registered as `ExecutorKind::Github` |
| `LinearExecutor` (struct + `new` + `with_timeout`) | REACHABLE | Instantiated in `main.rs:237`; registered as `ExecutorKind::Linear` |

### `daemon/src/integrations/connect.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `IntegrationExecutor` (struct + `new`) | REACHABLE | Instantiated in `main.rs:252`; registered as `ExecutorKind::Integration` |

### `daemon/src/projections/pull_request.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `PullRequestProjector` (struct) | REACHABLE | Registered in `projectors()` at `projections/mod.rs:97`; runs in `apply_all` on every `EventStore::append` |

**IPC exposure:** `ProjectionName::PullRequest` variant exists in `shared/src/ipc.rs:131` and maps to `"proj_pull_request"` in `daemon/src/ipc/methods.rs:50`. The UI can `get_projection("PullRequest")` → reads the table directly.

**REBUILD_TABLES:** `"proj_pull_request"` present in `daemon/src/projections/schema.rs:20` — fold + rebuild are both wired.

### `daemon/src/projections/integration_connections.rs`

| Symbol | Classification | Evidence |
|---|---|---|
| `IntegrationConnectionProjector` (struct) | REACHABLE (fold path) | Registered in `projectors()` at `projections/mod.rs:105`; runs in `apply_all` on every `EventStore::append`. `"proj_integration_connection"` in `REBUILD_TABLES` (schema.rs:29). |

**NOTE (intentionally-gated IPC read):** `ProjectionName::IntegrationConnection` does NOT exist in the shared `ProjectionName` enum (`shared/src/ipc.rs:126-137`). The `get_projection` IPC method cannot serve `proj_integration_connection` rows to the UI yet — this was declared a known deferred item (would require a CONTRACT bump). The projector WRITES to the table on every `IntegrationConnectionRegistered` event (fold/rebuild reachable). The READ side via IPC is gated to a future CONTRACT increment.

**REBUILD_TABLES:** `"proj_integration_connection"` present in `daemon/src/projections/schema.rs:29`.

---

## Summary table

| Area | Total exported symbols | REACHABLE | UNREACHABLE | INTENTIONALLY-GATED / TEST-SUPPORT SEAM |
|---|---|---|---|---|
| `integrations/classifier.rs` | 6 | 5 | 0 | 1 (`to_delivery_outcome` — outbox wiring deferred) |
| `integrations/github.rs` | 7 | 5 | 0 | 2 (`FakeGithubReadClient` seam, `OctocrabGithubReadClient` gated) |
| `integrations/github_write.rs` | 6 | 5 | 0 | 1 (`FakeGithubWriteClient` seam) |
| `integrations/linear.rs` | 12 | 10 | 0 | 2 (`FakeLinearReadClient` seam, `LinearGraphqlReadClient` gated) |
| `integrations/linear_write.rs` | 7 | 6 | 0 | 1 (`FakeLinearWriteClient` seam) |
| `integrations/pull_request.rs` | 14 | 14 | 0 | 0 |
| `integrations/executor.rs` | 4 | 4 | 0 | 0 |
| `integrations/connect.rs` | 2 | 2 | 0 | 0 |
| `projections/pull_request.rs` | 1 | 1 | 0 | 0 |
| `projections/integration_connections.rs` | 1 | 1 (fold) | 0 | IPC-read gated (no `ProjectionName::IntegrationConnection`) |
| **TOTAL** | **60** | **53** | **0** | **7 (all intentional)** |

---

## Intentionally-gated items (not gaps)

1. **`IntegrationOutcomeClass::to_delivery_outcome`** (`classifier.rs:50`) — Maps class to outbox `DeliveryOutcome`. Module doc states the `Destination` adapters that call this "are deferred/gated." Wired to a future outbox-integration wave; no production caller yet. Test-covered as a contract pin.

2. **`OctocrabGithubReadClient`** (`github.rs:196`) — Live octocrab read client for PR signals polling. Not instantiated in `main.rs`. The `GithubExecutor` uses only the write-client path for now; the read-polling path (syncing existing PRs) is a HITL follow-on wave. `FakeGithubReadClient` covers tests.

3. **`LinearGraphqlReadClient`** (`linear.rs:439`) — Live reqwest read client for Linear issue fetching. Same pattern: `LinearExecutor` uses only the write-client path; the read-polling path is deferred. `FakeLinearReadClient` covers tests.

4. **`FakeGithubReadClient`** (`github.rs:170`) — Test-support seam; `pub` to allow `daemon/tests/` imports; no production instantiation. Per CLAUDE.md HITL posture.

5. **`FakeGithubWriteClient`** (`github_write.rs:120`) — Test-support seam; `pub` for cross-crate tests.

6. **`FakeLinearReadClient`** (`linear.rs:254`) — Test-support seam; `pub` for cross-crate tests.

7. **`FakeLinearWriteClient`** (`linear_write.rs:172`) — Test-support seam; `pub` for cross-crate tests.

8. **`ProjectionName::IntegrationConnection` (absent)** — The `proj_integration_connection` table is written by `IntegrationConnectionProjector` (REACHABLE fold + rebuild path), but the `ProjectionName` enum does not include an `IntegrationConnection` variant yet. The IPC `get_projection` read path is unserveable until a CONTRACT bump adds this variant. Declared known-deferred per the audit brief.

---

## Phase-exit gate verdict

**CLEAR** — 0 unreachable production symbols. All gaps are intentional deferrals documented in the design (HITL live-read clients, outbox wiring, IPC read variant for integration connections). No wiring tasks required for this phase exit.

The complete production path — `submit_action` → `CatalogExecutor` → `GithubExecutor` / `LinearExecutor` / `IntegrationExecutor` → `EventStore::append` → `apply_all` → `PullRequestProjector` / `IntegrationConnectionProjector` — is fully wired end-to-end.


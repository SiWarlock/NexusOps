# Session 027 — UI-unblock arc: P7.2 + the D-series (D2→D3→D4a→D4b)

- **Date:** 2026-06-16
- **Phase:** Phase 7 tail (P7.2) + Phase 4.4/4.5 (the D-series, the §8.1/§11.4 survival-display + live-delta surface)
- **Predecessor:** [026-2026-06-14-background-jobs-and-codex-arc-3.3ab.md](026-2026-06-14-background-jobs-and-codex-arc-3.3ab.md)
- **Successor:** [028-2026-06-16-rich-pr-workspace-and-structured-reviews.md](028-2026-06-16-rich-pr-workspace-and-structured-reviews.md)
- **Branch:** `main` (single-track, daemon). All commits LOCAL — push is user-gated, never pushed here.

## Why this session existed

The user re-prioritized a **UI-unblock work order** ahead of the held cat-1 3.3c: a chain of cross-track read-surface slices so the ui cockpit reads live, typed projections and live-refreshes after every state change. Started after the team un-idled (post the user's ui↔edges merge → main `95df2e0`, CONTRACT 0.33.0). Closed by a HARD-STOP context cycle at a clean boundary (D-series complete, nothing in flight).

## What was built (5 slices, 5 commits)

- **P7.2 — `PullRequestRow` freeze** (`e748874`, CONTRACT 0.33.0→0.34.0). The 2nd frozen projection-row in `shared/src/projections.rs` (after `ApprovalQueueRow`) + a fail-closed `read_pull_request_typed` serving `get_projection(PullRequest)` typed. BASIC columns only (`mergeable`/`checks_summary` = a later SPREAD). Unblocks the ui PR Review Workspace.
- **D2 (P4.4) — `SessionRecovered` fold + `SessionRow` freeze** (`7fc8bc7`, CONTRACT 0.34.0→0.35.0). Folds the previously-DEAD `SessionRecovered` event into `proj_session` (a `SessionProjector` arm + MIGRATION_12, SUPPORTED_USER_VERSION 11→12) + the 3rd frozen projection-row `SessionRow` (typed-served via `read_session_typed`, a retain-whitelist). The §11.4 resumed-vs-replayed recovery display.
- **D3 (P4.5) — Session live-delta nudge** (`019a4b1`, CONTRACT-neutral). `deltas_for_append` now nudges on `SessionFailed`/`SessionRecovered` (not just `SessionStarted`), keyed on the projector's `EVENT_TYPE` consts (wire-value-drift-safe) + a §50/§51 keep-two-lists guard.
- **D4a (P4.5) — observation-path nudges** (`e3d82ad`, CONTRACT-neutral). `UsageLedger` nudge on `TelemetrySampled` (selective, §51-guarded with a negative arm) + a blanket `AuditTrail` nudge on every event. Migrated the existing/D3 delta tests to drain-and-find.
- **D4b (P4.5) — gateway-emitted-event delta sweep + the SessionStarted production-gap Finding fix** (`e2f6deb`, CONTRACT-neutral). The largest slice. See below.

### Files modified (by slice)
- **P7.2:** `shared/src/{projections,schema,lib}.rs` + `contracts/schema/*` (regen) · `daemon/src/ipc/{methods,mod}.rs` · `shared/tests/contract.rs` + `daemon/tests/projections.rs`.
- **D2:** `daemon/src/projections/session.rs` (fold arm) · `daemon/src/eventstore/{schema,migrations}.rs` (MIGRATION_12) · `shared/src/{projections,schema,lib}.rs` + schema regen · `daemon/src/ipc/{methods,mod}.rs` (`read_session_typed`) · `shared/tests/contract.rs` + `daemon/tests/{projections,gateway_plan}.rs` (the latter = the exact-latest version-pin 11→12).
- **D3:** `daemon/src/runtime/writer.rs` (`deltas_for_append` arms) · `daemon/tests/runtime.rs`.
- **D4a:** `daemon/src/runtime/writer.rs` (UsageLedger + blanket AuditTrail) · `daemon/tests/runtime.rs`.
- **D4b:** `daemon/src/projections/mod.rs` (the shared `deltas_for_event` + `EventDeltaIds` + §51 unit tests) · `daemon/src/runtime/writer.rs` (`deltas_for_append`→thin wrapper, `audit_trail_delta`, per-command AuditTrail) · `daemon/src/gateway/pipeline.rs` (`execute()` `deltas` param + post-commit build + `emitted_event_deltas`) · `daemon/tests/{session_executor,runtime}.rs`.

## Decisions made

- **D4b: `deltas_for_event` lives in `projections/`, NOT `writer.rs`** (the brief's file-list). Gateway is below runtime → gateway can't import runtime; `projections/` is below both (the principled home, mirrors the projectors the §51 guard references). Orchestrator confirmed.
- **D4b: gateway delta-threading = `execute()` gains a `deltas: &mut Vec` param; the emitted-event deltas build POST-commit** in execute's `Ok(ack)` arm (after the txn-B closure), fire-and-forget into the same Vec `publish_after_commit` gates on `result.is_ok()`. Double-gated; fail-closed txn-B / INV-SEC-1 unperturbed (security-reviewer CLEAR).
- **D4b: PR delta id = `pr_id` = `{repo_id}#{pr_number}`** (the `proj_pull_request` row PK, `pull_request.rs:92`) — not the bare `pr_number`. Matches the row so the ui targets the exact PR; `id:None` fallback if no Repo resource_ref.
- **D4b: AuditTrail = per-command blanket** in `publish_after_commit` (the single chokepoint for submit/plan/approve/deny); the projection-specific nudges go through the shared `deltas_for_event`.
- **D2: nullability = match-the-DDL** (`pr_id`/`session_id`/`project_id`/`status` non-Option where the column is PK/NOT-NULL; rest Option). `project_id` non-Option deviates from the ui provisional (DDL NOT NULL + the fold guarantees it).
- **D2: MIGRATION_12 = ALTER ADD COLUMN ×3** (the historical CREATE untouched — editing it would duplicate-column-fail fresh DBs; the MIGRATION_9 precedent).
- **D3: keyed on `EVENT_TYPE` consts (not literals)** for SessionFailed/Recovered — closes wire-value drift with the projector (the orchestrator's TWEAK).

## Decisions explicitly NOT made (deferred)

- **The `mergeable`/`checks_summary` SPREAD** (PullRequestRow enrichment) — no projection column yet; freeze when the ui workspace needs them.
- **`SessionStarted::EVENT_TYPE` const** in `shared/` — would close the §51 literal exception uniformly (the projector + `deltas_for_event` both use the `"SessionStarted"` literal today — §51-*consistent*, but a const is rename-safe). Deferred to avoid a `shared/` touch on a no-shared-change slice.
- **`ProjectionName` variants for project/repository/integration_connection** — they have no subscribe-name → audit-blanket only; adding them is a future CONTRACT change (flagged).
- **The seq-cursor audit-delta enrichment** — the per-event AuditTrail full-re-read is fine for MVP (§9 fire-and-forget); a cursor is a future throughput optimization.

## TDD compliance

- **P7.2 / D2 / D3 / D4a — clean RED-first.** Tests written first, confirmed RED for the right reason (missing type/fn compile errors; D3's positive tests hung waiting for an un-published delta), then GREEN.
- **D4b — RED confirmed, with a noted ordering nuance.** The §51 mapping unit tests were RED-first (missing `deltas_for_event` → compile error). The 3 **production-path integration tests** were authored after the gateway threading (a structural refactor across 3 files) — but RED was **retroactively confirmed via a revert-check** (neutering the gateway delta build → all 3 fail). Not a violation (RED proven), but recorded honestly: for a structural slice the integration harness was coupled to the confirmed threading. The revert-check is exactly the LESSON §52 "test must drive the production path" discipline demonstrated.
- No safety-critical TDD skips.

## Reachability (per /tdd Step 7.5)

- **P7.2:** `get_projection(PullRequest)` (live §6.1 RPC dispatch → serve_connection) → `read_pull_request_typed` → `PullRequestRow`. Source = the edges-P7.1 `PullRequestSynced`→projector on main.
- **D2:** `get_projection(Session)` → `read_session_typed` → `SessionRow`; the `SessionRecovered` fold runs in-band via the `SessionProjector` (emitted by 4.1b-1's `recover_sessions_on_restart`).
- **D3/D4a:** `deltas_for_append` @ the write-actor `Command::Append` arm (publish-after-commit); `SessionFailed`/`SessionRecovered`/`TelemetrySampled` production emitters.
- **D4b:** the gateway `execute()` post-commit build → `emitted_event_deltas` → `deltas_for_event` → `publish_after_commit`; reachable from `session.create` (auto-execute), git.create_worktree/github.create_pr (approve); AuditTrail per committed gateway command. **No tested-but-unwired gaps.**

## Open follow-ups

- **Cross-doc (orch-routed, /orchestrate-end):** the Appendix-A + daemon/CLAUDE.md MVP-projections rows for `PullRequestRow` (0.34.0) + `SessionRow` (0.35.0); the `SessionRecovered`-now-CONSUMED EventTypeRegistry note; the [D4a]/[D4b] AS-BUILT notes (incl. the `projections/` home); LESSON §51 extension + LESSON §52 (orphaned-nudge-on-path-migration / test-must-drive-the-production-path). All flagged at Step-9; daemon/CLAUDE.md + LESSONS.md hot-edited in the working tree (single-track happy path).
- **🔴 Cross-track → lead/ui (orch-routed):** the ui regenerates from 0.35.0 — (a) `ResumeMode` 2→4 variants (a Relaunched/ReattachedLive value fails the ui's Zod shadow); (b) `pr_number` string→number; (c) `toPrItems` `id: pr.pr_number`→`pr.pr_id`; (d) `display_name` vs the ui's `title`; (e) `project_id` optional→non-Option.
- **🔴 Future-CONTRACT flag:** `ProjectionName` variants for project/repository/integration_connection (if the ui needs to live-subscribe).
- **Convention/minor (deferred):** `SessionStarted::EVENT_TYPE` const; the `pr_number as i64` → `try_from` uniform hardening (P7.2); the seq-cursor audit enrichment; `drain`/`drain_deltas` test-helper dup (test-crate boundary); a `tracing::warn` on the `emitted_event_deltas` parse-failure (the coarse-None fallback is by-design).
- **Future TODO:** the mergeable/checks_summary SPREAD; the remaining projection-row freezes (ProjectActivityRow/AuditEventRow).
- **Held next:** 3.3c (the cat-1 Codex INV-SEC-1 interception, brief 066 ready) — or D5a.

## State at close

Workspace **816/0 green**; clippy clean; fmt clean; 3-way verify GREEN @0.35.0 (the contract slices). CONTRACT_VERSION = 0.35.0. SUPPORTED_USER_VERSION = 12 (MIGRATION_12). All 5 slice commits LOCAL on `main` (never pushed). security-reviewer CLEAR on every invariant-touching slice (D2 LIGHT, D4b opt-in); D3/D4a no §15 surface (security-reviewer not triggered).

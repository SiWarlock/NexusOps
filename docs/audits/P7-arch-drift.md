# P7 arch-drift audit — `/phase-exit 7` (edges track) — 2026-06-15

**Auditor:** arch-drift-auditor · **Branch:** `track/edges` · **Verdict: CLEAR**

Scope note: over-approximated track-diff audit (the 7.1 daemon side; §11.2 UI + §8 intake = ui-track / deferred, read-backend coverage only). 6 anchors — **0 DRIFT / 3 STALE-DOC / 0 ambiguous**. All 71 `shared/tests/contract.rs` snapshot tests GREEN (verified-by-test for CONTRACT 0.33 + the integration.connect catalog entry + the schema artifact).

## Anchor results (no drift)
- **§9 (integrations):** octocrab GitHub + Linear GraphQL read/write clients; auth-bootstrap deferred (spec-consistent); staged sync (one-way P1). CONFIRMED.
- **§11.2 (PR Review read backend):** `proj_pull_request` GitHub-authoritative cache (keyed `{repo_id}#{pr_number}`, rebuild-safe); `ProjectionName::PullRequest` exposed. The merge re-fetch + the UI surface = ui-track deferred. CONFIRMED (read backend).
- **§7.2 (PR SoT):** `proj_pull_request` synced cache + `pr_checked_at`; in REBUILD_TABLES; `keychain_ref` pointers only (the IntegrationExecutor `PrefixRedactor` reject). CONFIRMED.
- **§6.3 (catalog):** `integration.connect` risk-2, `ExecutorKind::Integration` (wire `integration`), `requires_resource_refs=false`, `standing_grant_eligible=false`, idempotency `FromInputs` — all VERIFIED-BY-TEST. CONFIRMED.
- **§17 (integration-failure contract):** classifier maps 429/5xx/transport→Retryable, 401/403→AuthFailed(Terminal), 4xx/404→Terminal; `*SyncFailed.reason` = STRUCTURAL class-name (never raw API text — daemon-log-only); `FailedWithEvents` emits the non-auth `*SyncFailed`; `GithubSyncFailed`/`LinearSyncFailed` distinct `{provider,reason,failed_at}`; `parse_retry_after`/`parse_rate_limit_reset`. CONFIRMED.
- **§8 (intake flows):** the `tasks(external_task)` intake → session.create is explicitly gated/deferred (code flags it, no silent partial). CONFIRMED-deferred.

## Known-deferred (NOT drift)
integration.connect registration-only (token→keychain write deferred) · standing_grant=FALSE (security-recommended, pending ratification) · MVP integration_connections = event-fed projection (proj_integration_connection) · CONTRACT 0.33 edges-local (daemon ratifies at merge) · `auth_expired` *SyncFailed variant deferred · §11.2 UI ui-track · the IPC RPC for proj_integration_connection · GitHub/Linear live network = HITL (fakes wired).

## Stale-doc notes (code correct → merge-ledger)
1. **§6.3 MVP count** "~21" → as-built 28 (CONTRACT 0.33; additive entries documented in test assertions).
2. **§9 auth-bootstrap deferral** not noted in arch text (code correctly defers; §19 mentions it generally).
3. **§11.2 `mergeable`/`checks_summary`** not persisted as separate proj columns (feed `derive_pull_request_status` in the executor; the status field encodes the result — MVP trade-off, not a gap).

## 🟡 MERGE-LEDGER (prominent)
CONTRACT 0.32 (main) → 0.33 (edges). `MVP_ACTION_TYPES.len()` = 28 (edges) vs 24 (main per the count) — the additive entries reconcile at the edges→main merge; the daemon (catalog/CONTRACT owner) ratifies `integration.connect` + assigns the final version.

**VERDICT: CLEAR** (0 drift; the landed edges P7.1 surfaces match the spec; deferrals confirmed).

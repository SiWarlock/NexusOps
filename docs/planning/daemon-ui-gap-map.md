# Daemon → UI Gap-Map (the "wire every daemon feature into the cockpit" goal)

> **Owner:** daemon-orchestrator (authoritative daemon inventory + sequencing). **Method:** daemon surface read directly from source (`shared/src/catalog.rs`, `daemon/src/ipc/methods.rs`, `shared/src/ipc.rs` ProjectionName, `daemon/src/projections/`); UI-wiring classified by grep over `ui/src` (intent builders, gateway-client method calls, projection consumers). First-pass, **to be reconciled with ui-orchestrator's parallel ui-side map** (functional-vs-mock nuances grep can't fully confirm). _(The fan-out workflow was platform-throttled — rate-limited 3× on subagent spawns — so this was produced inline.)_
>
> **Status legend:** ✅ wired+functional · ⚠️ wired-but-broken/partial/stub · ❌ not-wired · 🔒 machine-internal or phase-gated (not expected wired).

## Tally
- **Gateway actions:** 33 MVP (+7 machine-internal `agent.*`). UI submits **6** as intents; **~3 functional**, **~3 stub/broken**; **~24 not-wired**.
- **IPC methods:** 12. UI calls **9 functional**, **2 partial** (connect_via_gh / set_secret+set_live_writes), **1 unused** (submit_action_plan).
- **Projections:** 11. UI consumes **7 functional**, **2 degraded** (AuditTrail, UsageLedger), **2 daemon-gated** (PlanProgress, AgentTeam).

---

## 1. Gateway actions (the catalog — what a user can *do*)

| action_type | risk | daemon | UI intent | status | close-by |
|---|---|---|---|---|---|
| **project.rescan** | 0 | ✅ | `rescan-project-request.ts` | ⚠️ **BROKEN** (add-project; empty id + no project_id) | **IN FLIGHT** 089+090+ui-080 |
| **github.merge_pr** | 3 | ✅ | `pr-mutation-request.ts` | ✅ functional (go-live) | — |
| **github.submit_review** | 3 | ✅ | `pr-mutation-request.ts` | ✅ functional (go-live) | — |
| **git.diff** (read) | 0 | ✅ | `get_diff` RPC | ✅ functional (diff viewer) | — |
| git.stage_hunk | 2 | ⚠️ stub (Phase-5) | `hunk-resource-ref.ts` | ⚠️ UI-wired, daemon executor STUB | daemon (Phase 5 git exec) |
| git.unstage_hunk | 2 | ⚠️ stub | `hunk-resource-ref.ts` | ⚠️ UI-wired, daemon STUB | daemon (Phase 5) |
| git.discard_hunk | 3 | ⚠️ stub | `hunk-resource-ref.ts` | ⚠️ UI-wired, daemon STUB | daemon (Phase 5) |
| git.create_worktree | 2 | ⚠️ stub | referenced | ⚠️ stub | daemon (Phase 5) |
| github.create_pr | 3 | ✅ | referenced (types/tests) | ⚠️ confirm live button | ui verify |
| **session.create** | 0 | ✅ | none | ❌ **NOT-WIRED** (can't launch an agent from cockpit) | **ui (HIGH)** |
| **session.send_message** | 2 | ✅ | none | ❌ **NOT-WIRED** (can't message an agent) | **ui (HIGH)** |
| session.kill | 0 | ✅ | none | ❌ not-wired | ui |
| session.pause / resume | 1/2 | ✅ | none | ❌ not-wired | ui |
| session.attach_terminal | 1 | ✅ | terminal view exists | ⚠️ view yes, attach-intent no | ui |
| session.profile_change | 2 | ✅ | none | ❌ not-wired | ui |
| profile.set_keychain_ref | 2 | ✅ | `set_secret` ref (settings) | ⚠️ partial | both verify |
| integration.connect | 2 | ✅ | `connect_via_gh` ref | ⚠️ partial (auth trigger) | both verify |
| integration.set_live_writes | 2 | ✅ | `set_live_writes` ref | ⚠️ partial (governance toggle) | both verify |
| github.create_pr_draft | 2 | ✅ | none | ❌ not-wired | ui |
| github.sync_reviews | 1 | ✅ | reviews consumed, no sync intent | ⚠️ partial | ui |
| linear.link_issue | 2 | ⚠️ exec | none | ❌ not-wired | both (Phase 7) |
| linear.create_issue | 2 | ⚠️ exec | none | ❌ not-wired | both (Phase 7) |
| plan.link_task | 2 | ⚠️ (Plan proj gated) | none | ❌ not-wired | both |
| brain.ask / sync / summarize_session | 0/2/2 | ⚠️ 8.1 gated | brain view exists | ❌ not-wired | both (Phase 8) |
| workflow.detect | 0 | ✅ (rescan-internal) | none | ❌ not-wired | ui |
| workflow.command.invoke | 4 | ⚠️ (Phase 9) | packs view | ❌ not-wired | both (Phase 9) |
| code.open_file | 0 | ⚠️ stub | code/editor view | ❌ not-wired (intent) | ui |
| review.request_agent_fix | 3 | ⚠️ stub | none | ❌ not-wired | both |
| git.status / git.create_branch | 0/2 | ✅/stub | none | ❌ not-wired | ui/daemon |
| `agent.*` (bash/file_edit/file_read/mcp_tool/todo_write/web_fetch/web_search) | 0-2 | ✅ | — | 🔒 machine-internal (Claude/Codex interception — NOT a user UI surface) | n/a |

## 2. IPC methods (the wire surface)

| method | kind | UI | status |
|---|---|---|---|
| get_capabilities | read | ✅ (51) | ✅ |
| get_projection | read | ✅ (110) | ✅ |
| subscribe | subscribe | ✅ (175) | ✅ |
| get_diff | read | ✅ (69) | ✅ |
| get_pr_diff | read | ✅ (38) | ✅ |
| submit_action | mutation | ✅ (47) | ✅ |
| approve | mutation | ✅ (274) | ✅ |
| deny | mutation | ✅ (97) | ✅ |
| preview_action | read | ✅ (23) | ✅ |
| connect_via_gh | trigger | ⚠️ (2) | partial (auth; needs runtime verify) |
| submit_action_plan | mutation | ❌ (0) | ❌ N-step plan modal not wired (Phase-8 ui-ahead) |
| intercept | daemon-internal | — | 🔒 (hook receiver — not a UI method) |

## 3. Projections (the read surface)

| projection | UI refs | status |
|---|---|---|
| ApprovalQueue | 151 | ✅ live subscription (ui-059) |
| PullRequest | 175 | ✅ PR Workspace |
| Review | 393 | ✅ PR review |
| ProjectActivity | 97 | ✅ (Row provisional/not-frozen) |
| Worktree | 79 | ✅ |
| ProjectGraph | 52 | ✅ graph view |
| Session | (Shell/sidebar) | ✅ refetch-on-nudge (ui-062) |
| **UsageLedger** | 32 | ⚠️ **DEGRADED** — daemon doesn't serve `creditPool` (Mock-vs-real gap) |
| **AuditTrail** | 31 | ⚠️ **DEGRADED** — daemon doesn't serve `event_type` (namespace filter/icons broken) |
| **AgentTeam** | 20 | ❌ daemon Team projector NOT built (UI view, no live data) |
| **PlanProgress** | 1 | ❌ daemon Plan projector NOT built (daemon-gated) |

---

## 4. Actionable gaps + proposed sequence

**ARC 0 — add-project (IN FLIGHT).** 089 (CLI project_id mint) + 090 (daemon submit hardening + error de-collapse) + ui-080 (UI mints act_+proj_ ULIDs). ⚠️→✅. _Owner: daemon-impl + ui-impl._

**WAVE 1 — the core "air-traffic-control" loop (highest user value).**
1. **Session lifecycle control** — wire cockpit buttons for `session.create` (launch agent), `session.send_message`, `session.kill`/`pause`/`resume`, `attach_terminal`. Daemon is READY (Phase 3/4). _Owner: ui-impl (daemon surface confirmed)._ This is the biggest single gap — the cockpit can *view* sessions but not *drive* them.
2. **Git hunk executors** — make the already-UI-wired `git.stage_hunk`/`unstage_hunk`/`discard_hunk`/`create_worktree` functional (they're audited but the daemon executor is a Phase-5 stub → nothing happens on disk). _Owner: daemon-impl (Phase 5 git executors)._

**WAVE 2 — projection honesty.**
3. **AuditTrail `event_type`** — daemon persists `event_type` on `proj_audit_trail` (+ migration) → un-degrade the Audit tile. _Owner: daemon-impl._ (user already chose "daemon adds event_type".)
4. **UsageLedger `creditPool`** — daemon serves `creditPool` in the projection page. _Owner: daemon-impl._
5. **Plan + Team projectors** — daemon builds production `proj_plan`/`proj_team` → the Plan/Team views get live data. _Owner: daemon-impl (8.1 / 9.x)._

**WAVE 3 — integrations + auth.**
6. **Auth/governance settings** — finish the `connect_via_gh` + `integration.set_live_writes` + `profile.set_keychain_ref`/`set_secret` settings surface + runtime-validate. _Owner: both._
7. **Linear** (`linear.link_issue`/`create_issue`) + **Brain** (`brain.ask`/`sync`/`summarize` + drawer, Phase 8 / daemon 8.1). _Owner: both._

**WAVE 4 — advanced.**
8. **N-step ActionPlan** (`submit_action_plan` + the plan modal, Phase-8 ui-ahead). **Workflow packs** (`workflow.*`, Phase 9). **code.open_file / review.request_agent_fix** (stub executors). _Owner: both._

## 5. Coverage notes (runtime verification needed)
- "wired_functional" for github.create_pr, the auth/governance surface (connect_via_gh/set_live_writes/set_secret), and the git hunk UI is **static-evidence only** — needs a runtime smoke pass to confirm the live data/exec path (esp. the git executors which are known daemon stubs).
- The `agent.*` family is the Claude/Codex interception (machine-internal) — correctly NOT a user-facing UI surface; excluded from the goal.
- Reconcile against ui-orchestrator's ui-side map for Mock-vs-real on the projection consumers (ProjectActivity/Worktree/ProjectGraph rows that are provisional/not-frozen).

## 6. Reconciliation with the ui-side map (MERGED 2026-06-26)

> Folds `docs/planning/ui-wiring-gap-map-ui-side.md` (ui-orchestrator's Mock-vs-real verification half) into this authoritative map. The ui-side file is retained as the source artifact; this section is the consolidated truth. Verified de-duped count: **35 features — wired_functional 6 · wired_but_broken 8 (all Mock-masked) · not_wired 21 · mock_only_suspect 1 (`preview_action`) · n/a_internal 2 (`agent.*` + intercept).**

**Agrees + sharpened (ui-side confirms daemon-orch's doc):**
- The empty-`action_request_id` mutation-builder class is **Mock-masked** (`Mock.submit_action` returns a canned ack, never validates the id) → only surfaces on the live daemon. Collapsed to 2 mint-id slices (ui-080 add-project + ui-081 the 3 siblings — git hunks ×3 + merge_pr + submit_review). **All landed.**
- AuditTrail: daemon serves `headline`/`actor_label` (`schema.rs:239`); the UI `AuditEventRow` requires `actor_type`+`event_type` → every populated row fails the boundary parse. → **W2-audit** (daemon adds `event_type` + UI reconcile).

**ui-side NEW findings folded into the WAVEs:**
- **UsageLedger is broader than `creditPool`** — the whole `UsageRow` shadow mismatches the served columns (`session_id`≠`subject_id`, `tokens_in/out`≠`tokens`, `cost_estimate`≠`cost`, no `harness`); currently masked because `TelemetrySampled` ingress is P4-dormant (table empty). → **W2-usage** = row-reconcile to `schema.rs:253` names **+** serve `creditPool` (both halves, was under-scoped to just creditPool).
- **Transport gaps** — `connect_via_gh` (`methods.rs:93`) + `profile.set_secret` (`methods.rs:101`) are live daemon IPC methods **absent from `gateway-client/types.ts` + the Tauri `lib.rs` allowlist** → no consumer can call them. The W3-connect + profile verticals must add the transport FIRST. _(daemon dep: none — daemon side shipped.)_
- **Worktree** — daemon serves+nudges `proj_worktree` but the UI has no consumer (not in Shell's 7 `get_projection`s, no shadow). Pure-UI add when a worktree panel is wanted (no daemon dep).
- **Plan/Team dead on BOTH sides** — daemon `projectors()` registers no projector for either (always-empty tables) and the UI renders static `planFixture`/`teamFixture` (Mock-risk: look live, no backing). → **W2-plan-team** is daemon-gated (needs the projectors) before any UI un-fixture.

**ui-side corroborations (NOT gaps — recorded so they aren't re-raised):**
- **ApprovalQueue live refresh = OK.** The nudge is pushed via the gateway pipeline path (`pipeline.rs:79` `approval_queue_delta(row:None)` on submit/approve/deny/expire); ui-059 already subscribes + refetches. No daemon fix.
- **PlanModal per-step `approve(approval_id, step_id)` = latent Phase-8 UX gap, not urgent.** The daemon `approve()` ignores `step_id` (`methods.rs:229`, "RESERVED in 2.1c") → per-step buttons resolve the whole approval; unreachable today (no live plan feed). Folds into the WAVE-4 `submit_action_plan`/plan-modal slice (daemon dep: per-step targeting in `approve()`).
- **`preview_action` = mock_only_suspect** — referenced by PlanModal/GatewayModal but PlanModal is partly `samplePlan`-driven → reachability re-verify owed (is preview reached from a real submit or only the fixture demo?).

# UI-Wiring Gap-Map — ui-orchestrator reconciliation (ui-side)

> **Pairs with** `docs/planning/daemon-ui-gap-map.md` (daemon-orchestrator, the authoritative daemon-anchored half). This is the **ui-side verification + deltas** — Mock-vs-real reads that grep can't confirm. Produced from the serial gap-map workflow (`wf_fab30ff5-9f6`; 4/9 group agents + synthesis completed — `read-rpcs`/`approval-plan-flow`/`streaming-terminal`/`brain`/`census` rate-limited, covered by daemon-orch's doc + direct reads) + direct file verification. daemon-orchestrator merges this into the single authoritative version.
>
> **Headline:** the 5 wired-but-broken mutation builders (the empty-`action_request_id` class) collapse to 2 mint-id slices; the **3 broken projection shadows reject every REAL daemon row** (Mock-masked) — and one of them (ProjectActivity) is why **Add-project still won't show the project after ui-080**.

## A. CONFIRMED + sharpened (agrees with daemon-orch's doc)

- **3 sibling mutation builders carry the empty-`action_request_id` bug** (= ui-080's class, **Mock-masked** because `Mock.submit_action` returns a canned ack and never validates the id):
  - `buildHunkActionRequest` (`hunk-resource-ref.ts:57`) → `git.stage_hunk`/`unstage_hunk`/`discard_hunk` (approval-gated; approve→`request::load("")`→`ActionRequestId::parse("")` WrongPrefix derail; 2nd mutation collides on the empty PK).
  - `buildMergePrActionRequest` (`pr-mutation-request.ts:69`) + `buildSubmitReviewActionRequest` (`:106`) → `github.merge_pr`/`submit_review` — **ENABLED in production** (ui-075 go-live, `Shell.tsx:150-151`) → the live PR-mutation surface derails on the 2nd mutation. → **queued as ui-081** (one bundled mint-id slice; daemon dep: none).
- **AuditTrail broken**: daemon serves `headline`/`actor_label` (`schema.rs:239`), the UI `AuditEventRow` (`provisional.ts:282`) requires `actor_type`+`event_type` → every populated row fails the boundary parse (table IS populated — audit folds every event). → UI reconcile + **daemon adds `event_type`** (user already chose this).

## B. NEW findings (beyond daemon-orch's doc — the value-add)

1. **🔴 ProjectActivity rejects every real row + no auto-appear → Add-project not complete after ui-080.**
   - `provisional.ts:162` `ProjectActivityRow` requires `name`; daemon `proj_project_activity` (`schema.rs:101`) has **no `name` column** (counters + `project_id` + `updated_at_seq`) → `BoundaryValidationError` → ProjectActivity tile degrades → **empty project switcher + empty ProjectGraph** (it derives from `data.projects`). Mock-masked (`fixtures/proj_project_activity.ts` fakes `name`).
   - **Worse — auto-appear:** ProjectActivity folds only `SessionStarted` (`mod.rs:89-92`); `ProjectRescanned` is audit-blanket-only (`mod.rs:192-196`, **no ProjectActivity delta**). And `proj_project` (which HAS the rescanned project) has **no `ProjectionName` variant** → not serve-able. So a session-less newly-added project is invisible to the cockpit regardless of the `name` fix.
   - **UI half (fast, pure-UI):** relax `name`→optional + id-fallback label so real rows parse → the switcher renders projects-with-sessions. **Daemon half (auto-appear):** expose the project registry (`ProjectionName::Project` serving `proj_project` w/ name) **or** fold `ProjectRescanned`→ProjectActivity + a name source.
2. **🔴 `integration.set_live_writes` has NO UI control → PR-mutations not functional e2e.** The daemon `live_writes_enabled` toggle defaults OFF (`catalog.rs:311`, daemon shipped) and is the real live-write gate; even with the UI PR-mutation gate enabled (`Shell.tsx:151`) and ui-081's id fix, a real merge/review can't reach GitHub. Only flippable via a raw IPC `submit_action` today. → a **new Settings governance toggle** completes the cat-1 go-live (daemon dep: none).
3. **UsageLedger is more broken than just `creditPool`:** the whole `UsageRow` shadow (`provisional.ts:315`) mismatches the served columns (`schema.rs:253`): `session_id`≠`subject_id`, `tokens_in/out`≠`tokens`, `cost_estimate`≠`cost`, no `harness` → every non-empty row fails parse (currently masked — `TelemetrySampled` ingress is P4-dormant so the table is empty). `creditPool` is never served (bare-array reply, no envelope). → UI row-reconcile to `schema.rs:253` names + daemon serves `creditPool`.
4. **Transport gaps (not in GatewayPort / Tauri allowlist):** `connect_via_gh` (`methods.rs:93`) and `profile.set_secret` (`methods.rs:101`) are live daemon IPC methods but absent from `gateway-client/types.ts` + the Tauri `lib.rs` allowlist → no consumer can call them (the GitHub-Connect + profile-credential verticals need the transport added first).
5. **Worktree:** daemon serves+nudges `proj_worktree` (`methods.rs:50`, `mod.rs:274/97`), but the UI has **no consumer** (not in Shell's 7 `get_projection`s, no shadow, not in `boundary.ts` PAGE_SCHEMAS). Pure-UI add when a worktree panel is wanted.
6. **PlanProgress + AgentTeam — dead on BOTH sides:** daemon `projectors()` (`mod.rs:261-292`) registers NO projector for either → the tables are always-empty, no delta; the UI renders static `planFixture`/`teamFixture` (Mock-risk: look live, no backing). Daemon-gated (needs the projectors).

## C. Corrections / nuance vs daemon-orch's doc

- **`project.rescan` is now `wired_functional`** (was the broken anchor) — ui-080's mint-id fix is in the working tree (uncommitted at workflow time); the builder now mints both ids matching the daemon shape. Residual: the live "project auto-appears" is gated on **B.1** (daemon half), not on ui-080.
- The 5 mutation builders' derail is on the **approve→execute load path** (`request::load("")`→`parse("")`), NOT the risk-0 auto-execute path (which runs off the in-memory req) — the original add-project mechanism note was imprecise; the real breaks were (a) `project_id` None→projector skip and (b) the empty-PK collision. (Moot for ui-080; relevant framing for the sibling builders, which ARE approval-gated.)
- **`preview_action` = mock_only_suspect** — referenced by PlanModal/GatewayModal but PlanModal is partly `samplePlan`-fixture-driven → flag a reachability re-verify (is preview reached from a real submit, or only the fixture demo?).

## D. Reconciled top sequence (post-ui-080 — to confirm with daemon-orch's WAVE plan)

| # | Slice | Fixes | Owner | Daemon dep |
|---|---|---|---|---|
| ui-081 | sibling-builders mint-id (authored) | git hunks ×3 + merge_pr + submit_review submit-derail | ui | none |
| ui-082 | **ProjectActivity `name`-relax** (pure-UI) | project switcher renders real rows | ui | none (UI half) |
| ui-083 | **`integration.set_live_writes` toggle** | completes the cat-1 PR go-live | ui | none (daemon shipped) |
| WAVE-1 A/B | session.create "Launch" + session.kill | the air-traffic-control loop | ui | none |
| — | **D: ProjectActivity auto-appear** (registry read / ProjectRescanned-fold + name) | new project appears w/o a session | **daemon** | — |
| — | **D: AuditTrail `event_type` col** → then ui-08x reconcile | un-degrade Audit tile | daemon→ui | — |
| — | **D: UsageLedger `creditPool` serve** + ui row-reconcile | usage tile | daemon→ui | — |
| later | connect_via_gh + integration.connect (Connect vertical) · profile vertical · Worktree consumer · PlanProgress/AgentTeam (daemon projectors first) · submit_action_plan | — | both | varies |

**Counts (verified, de-duped 35 features):** wired_functional 6 · **wired_but_broken 8** (the urgent set; all Mock-masked) · not_wired 21 · mock_only_suspect 1 (`preview_action`) · n/a_internal 2 (`agent.*` family + `intercept`).

## E. Addendum — fuller serial run (`wdr2syycz`, all 7 groups completed)

A second fully-serial pass completed every group (the first partial run's 5 rate-limited groups now covered). It corroborated every finding above and added two:

- **ApprovalQueue live refresh — VERIFIED OK (not a gap).** One pass flagged "no ApprovalQueue arm in `deltas_for_event` → queue goes stale"; another found the nudge IS pushed via the **gateway pipeline path** (`pipeline.rs:79` `approval_queue_delta(row:None)` on submit/approve/deny/expire), which ui-059 already relies on (Session + ApprovalQueue are the two live-nudged streams). Resolution: ApprovalQueue **is** live-nudged; the UI subscribe + refetch-on-nudge is correct; **no daemon fix needed.** (Recorded so the methodology artifact isn't re-raised.)
- **PlanModal per-step `approve(approval_id, step_id)` — latent UX gap (Phase 8, NOT urgent).** The N per-step "Approve step" buttons thread `step_id` (`PlanModal.tsx`/`submit-intent.ts`/`uds.ts`), but the daemon `approve()` currently IGNORES it (`methods.rs:229` — "accepted at the §6.1 boundary but RESERVED in 2.1c") → every per-step button resolves the WHOLE approval; the granularity is illusory. **Unreachable today** (PlanModal is exposed-ahead, no live plan-data feed). UI fix when Phase-8 plans go live: disable/relabel the per-step control to "resolves the whole plan" until the daemon implements per-step targeting (keep plan-level Approve-all/Deny). Daemon dep: per-step targeting in `approve()`. → folds into the WAVE-4 `submit_action_plan`/plan-modal slice.


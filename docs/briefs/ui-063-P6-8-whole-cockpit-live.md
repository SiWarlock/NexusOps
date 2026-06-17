# /tdd brief — whole_cockpit_live (the refetch-on-nudge spread to the rest)

## Feature
Spread the proven **refetch-on-nudge** live-subscribe pattern (Session @ui-062, ApprovalQueue @ui-059) to the remaining live-relevant served projections: **ProjectActivity, PullRequest, UsageLedger**. Each gets its own subscribe effect in `Shell.tsx` mirroring the Session/ApprovalQueue 2nd-stream exactly — `runSubscriptionSupervisor` → `client.subscribe({projection})` → a daemon `row:None` nudge → `coalescer.nudge()` → a coalesced `get_projection(<X>)` re-read → `setData`, with a per-stream `notifyConnectionState("<X>", …)` registering into the port's worst-of connection authority. This closes **whole-cockpit-live** (the 6.8-tail): the whole cockpit — projects/graph counts, PR list, usage dashboard — now updates live as the daemon mutates, not just on reconnect. **NON-cat-1, read-only.** No shadow change, no contract regen.

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport — the **live-delta spread to the REST**, now UNGATED by daemon D3/D4; the tracker checkbox at `IMPLEMENTATION_PLAN.md` line 765 — `[✅ Session DONE @ui-062]` → ProjectActivity/PullRequest/UsageLedger = ui-063)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the live `ProjectionDelta` transport — refetch-on-nudge), `§11` (the projection-driven cockpit display)
- **Related context:** the **ui-062 Session brief** (`docs/briefs/ui-062-P6-8-session-live-and-survival-shadow.md`) + the **ui-059 ApprovalQueue** live-subscribe — this slice is their mechanical replication; `ui/LESSONS.md §29` (the `row:None` id-nudge → coalesced refetch, NEVER a row-apply reducer; the per-stream worst-of connection aggregation); `ui/src/gateway-client/refetch-on-nudge.ts` (`createNudgeCoalescer` — at-most-one-in-flight + one-trailing, timer-free); the daemon delta source `daemon/src/projections/mod.rs` `deltas_for_event` (lines 80–114) — **VERIFIED emits nudges for all three:** ProjectActivity (on `SessionStarted`, keyed by project_id), PullRequest (on `PullRequestSynced`, keyed by pr_id), UsageLedger (on `TelemetrySampled`, id None).

## The two live effects to mirror (the exact template)
Both already in `Shell.tsx`: the **Session** effect (lines 226–280) and the **ApprovalQueue** effect (lines 290–340). Each new projection effect copies that structure verbatim, swapping the projection name + the recount function:

```
useEffect(() => {
  let cancelled = false;
  const recountFromX = (prev, rows) => ({ ...prev, <stateField>: rows, counts: deriveProjectSwitcherCounts({...}) });  // see "recount discipline"
  const refetchX = async () => { const page = await client.get_projection("<X>"); if (cancelled) return; setData(prev => prev ? recountFromX(prev, page.rows) : prev); };
  const coalescer = createNudgeCoalescer(refetchX);
  runSubscriptionSupervisor({
    subscribe: () => client.subscribe({ projection: "<X>" }),
    onDelta: () => coalescer.nudge(),
    refetch: refetchX,
    setConnection: (next) => client.notifyConnectionState("<X>", next),
    delay: (attempt) => <copy the bounded backoff verbatim>,
    shouldContinue: () => !cancelled,
  }).catch((e) => console.error("<X> subscription supervisor exited unexpectedly", e));
  return () => { cancelled = true; };
}, [client]);
```

## Recount discipline (the one real correctness point — do NOT get this wrong)
`deriveProjectSwitcherCounts({projects, sessions, pullRequests, approvals})` (used at initial load `Shell.tsx:194` + in `recountFrom`/`recountFromApprovals`) is what feeds `ShellData.counts`. So a refetch must recompute `counts` **iff** the refetched projection is one of its inputs:
- **ProjectActivity** → updates `projects`; `projects` IS an input → **recompute `counts`** (a new/changed project must re-key the switcher counts). `recountFromProjects(prev, projects) = {...prev, projects, counts: deriveProjectSwitcherCounts({projects, sessions: prev.sessions, pullRequests: prev.pullRequests, approvals: prev.approvals})}`.
- **PullRequest** → updates `pullRequests`; `pullRequests` IS an input → **recompute `counts`**. `recountFromPullRequests(prev, pullRequests) = {...prev, pullRequests, counts: deriveProjectSwitcherCounts({projects: prev.projects, sessions: prev.sessions, pullRequests, approvals: prev.approvals})}`.
- **UsageLedger** → updates `usage` + `creditPool`; **NOT** a `deriveProjectSwitcherCounts` input → **plain replace, NO recount**. `refetchUsage` sets `{...prev, usage: page.rows, creditPool: page.creditPool ?? null}` (the `UsageProjectionPage` carries `creditPool` — mirror the initial-load `usage.creditPool ?? null` at `Shell.tsx:209`).

Verify this input-set against `deriveProjectSwitcherCounts` during GREEN (flag at Step 2.5 if it reads more than the 4 inputs above).

## Acceptance criteria (what "done" means)
- [ ] **ProjectActivity refetch-on-nudge** — a new subscribe effect; a daemon `row:None` ProjectActivity nudge → coalesced `get_projection("ProjectActivity")` re-read → `projects` updates + `counts` recomputed. Per-stream `notifyConnectionState("ProjectActivity", …)`.
- [ ] **PullRequest refetch-on-nudge** — a new subscribe effect; a `row:None` PullRequest nudge → re-read → `pullRequests` updates + `counts` recomputed. Per-stream `notifyConnectionState("PullRequest", …)`.
- [ ] **UsageLedger refetch-on-nudge** — a new subscribe effect; a `row:None` UsageLedger nudge → re-read → `usage` + `creditPool` updated (no recount). Per-stream `notifyConnectionState("UsageLedger", …)`.
- [ ] All three consume the daemon nudge via **refetch-on-nudge** (the `createNudgeCoalescer` + `get_projection` re-read), **NEVER a row-apply reducer** (the `row:None` no-op footgun — `ui/LESSONS.md` §29). Pin each against a daemon-SHAPED Mock (`row` omitted).
- [ ] **AuditTrail stays load-once (NOT in the spread)** — confirm AuditTrail is deliberately excluded (rationale in Step-2.5 #2); the initial `get_projection("AuditTrail")` at `Shell.tsx:189` is unchanged, no AuditTrail subscribe effect added.
- [ ] The full ui suite stays green (the ~376 + the 3 new tests), `tsc --noEmit` + `oxlint` clean, `/preflight` clean.
- [ ] Cross-doc flagged at Step 9 (orchestrator writes the `ui/CLAUDE.md` transport-row update hot: the refetch-on-nudge spread is COMPLETE for the live-relevant served set — Session+ApprovalQueue+ProjectActivity+PullRequest+UsageLedger — whole-cockpit-live; AuditTrail excluded-by-design). **Implementer does NOT edit `ui/CLAUDE.md`.**

## Deferred / out of scope (flagged, not built)
- **AuditTrail live subscribe** — DEFERRED by design (see Step-2.5 #2): the daemon emits a **BLANKET** AuditTrail nudge on **every** event (`deltas_for_append` audit-blanket) → a subscribe would trigger a whole-page re-read on every system event (a refetch storm on a paged/forensic projection). AuditTrail is refresh-on-open; the proper fix is the daemon's flagged **seq-cursor audit-delta enrichment** (fetch only new rows) — a daemon carry-forward, not this slice.
- **The UsageLedger LIVE producer** — the UI handler is built + correct, but live `TelemetrySampled` emission in production is **daemon P4-dormant** (daemon `LESSONS` §35 — the telemetry pump ticks but the live `UsageSource` ingress is a deferred daemon follow-on). The subscribe stream still connects (`connected`, just quiet) and refetches correctly once telemetry flows. Build it anyway (identical pattern, future-proof) — do NOT special-case or skip it.
- **ProjectGraph / Worktree / Review live subscribe** — not in the Shell's initial load set (no static read to make live); out of scope. (Review lands with the Phase-7 PR Workspace.)

## Wiring / entry point (Step 7.5)
The 3 new subscribe effects are mounted in `Shell.tsx` exactly like the Session/ApprovalQueue effects (the production path: Shell mount → `runSubscriptionSupervisor` → `client.subscribe({projection})` → the daemon's deltas → `onDelta`/`refetch`). No new exported symbol beyond the 3 effects; each is reachable-from-mount the moment the Shell renders. `/wired` target: each new subscribe effect (reachable from the Shell mount, the Session/ApprovalQueue precedent).

## Files expected to touch
**Modified:**
- `ui/src/shell/Shell.tsx` — 3 new subscribe effects (ProjectActivity / PullRequest / UsageLedger), each mirroring the Session/ApprovalQueue effect with its recount function.
- `ui/src/shell/Shell.subscribe.test.tsx` — 3 new `<X>_subscribe_refetches_on_row_none_nudge` tests + their `Gated<X>RefetchGateway` helpers (mirror `GatedSessionRefetchGateway` / `GatedApprovalRefetchGateway`).
- `ui/src/projections/fixtures/proj_project_activity.ts` — add a `projectActivityDeltaFixture` (a `row:None` ProjectActivity nudge).
- `ui/src/projections/fixtures/proj_pull_request.ts` — add a `pullRequestDeltaFixture` (`row:None`).
- `ui/src/projections/fixtures/proj_usage.ts` — add a `usageDeltaFixture` (`row:None`).
- `ui/src/gateway-client/mock.ts` — register the 3 new delta fixtures in `MockGatewayPort.subscribe()` (extend the `if/else if` chain; keep the unknown-projection throw).

**Not touched:** `ui/src/contracts/provisional.ts` + `generated.ts` (no shadow/contract change — these 3 rows already parse via `boundary.ts` PAGE_SCHEMAS) · `ui/src/gateway-client/refetch-on-nudge.ts` (reused as-is) · `ui/src/connection/state.ts` + `uds.ts` (the worst-of aggregation is unchanged — new streams just register) · `ui/CLAUDE.md` (orchestrator territory).

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
In `ui/src/shell/Shell.subscribe.test.tsx`, mirror the existing `session_subscribe_refetches_on_row_none_nudge` (lines 105–135) / `approvalqueue_nudge_refetches_not_row_apply` (lines 211–237) pattern — a `Gated<X>RefetchGateway` holding the nudge until released, serving an augmented page on the post-nudge re-read:

1. **`projectactivity_subscribe_refetches_on_row_none_nudge`** — baseline loads without the extra project; `releaseNudge()` fires the `row:None` ProjectActivity nudge → the supervisor refetches `get_projection("ProjectActivity")` → the new project appears (DOM/state) + the switcher `counts` reflect it. **RED:** no ProjectActivity subscribe effect exists → no refetch → the new project never appears. [§6.1/§11/`LESSONS.md` §29]
2. **`pullrequest_subscribe_refetches_on_row_none_nudge`** — `row:None` PullRequest nudge → refetch → the new PR appears + `counts` recomputed. **RED:** no PullRequest subscribe effect. [§6.1/§11/`LESSONS.md` §29]
3. **`usageledger_subscribe_refetches_on_row_none_nudge`** — `row:None` UsageLedger nudge → refetch → the updated usage row / `creditPool` reflected (assert on a usage-surface value or `gateway.usageReads > baseline`). **RED:** no UsageLedger subscribe effect. [§6.1/§11/`LESSONS.md` §29]

Optional (implementer's call — the supervisor recovery is shared + already pinned for ApprovalQueue at `approvalqueue_subscribe_recovers_on_lag_close`): one `<X>_subscribe_recovers_on_lag_close` if you want a per-stream recovery pin; not required (the `runSubscriptionSupervisor` recovery path is identical + covered).

## Cross-doc invariant impact
- **Model field changes:** none. No shadow (`provisional.ts`) change, no contract regen (`generated.ts`); the 3 projection pages already validate via `boundary.ts` PAGE_SCHEMAS.
- **Orchestrator doc row (Step 9, I write hot):** `ui/CLAUDE.md` — update the live-`UdsGatewayPort`-transport row: the refetch-on-nudge spread is COMPLETE for the live-relevant served set (whole-cockpit-live); AuditTrail excluded-by-design (blanket-nudge refetch-storm → seq-cursor enrichment is the daemon follow-on). No new generated value-set.
- **2.5-seam:** none touched (no shadow/contract change).

## Things to flag at Step 2.5
1. **Recount discipline per projection** (the correctness crux). My default: **ProjectActivity + PullRequest recompute `counts`** (both are `deriveProjectSwitcherCounts` inputs); **UsageLedger is a plain replace** (`usage` + `creditPool`, not a count input). Confirm `deriveProjectSwitcherCounts` reads exactly `{projects, sessions, pullRequests, approvals}` (it does at `Shell.tsx:194`/`recountFrom`) — if it reads more, widen accordingly.
2. **AuditTrail deliberately EXCLUDED.** My default: **exclude** — the daemon emits a blanket AuditTrail nudge on every event → subscribing causes a whole-page refetch storm; AuditTrail is a forensic/refresh-on-open surface; the daemon's flagged seq-cursor delta enrichment is the right fix. Flag if you'd rather include it (then it needs throttling/cursoring — out of scope here).
3. **UsageLedger producer is P4-dormant.** My default: **build the handler anyway** (identical pattern, future-proof; the stream connects `connected` and stays quiet until the daemon's live telemetry ingress lands). Flag if you'd rather defer the UsageLedger effect until the producer is live (I recommend against — it's mechanical + harmless).
4. **Connection aggregation now spans 5 streams.** My default: **no change** — this is the established worst-of single-authority model (ui-059; `ui/LESSONS.md` §29); any of the 5 degrading ⇒ `canSubmitIntent` FALSE is correct (a cockpit with a dead projection stream is degraded). The new streams just register via `notifyConnectionState`; the aggregation logic is untouched. Flag if you see a concern with the broadened degraded surface.

## Dependencies + sequencing
- **Depends on:** ui-062 (Session refetch-on-nudge — the immediate precedent) + ui-059 (ApprovalQueue — the 2nd-stream + worst-of aggregation) + daemon D4 (the other-projection delta emission — **verified on track/ui:** `deltas_for_event` emits ProjectActivity/PullRequest/UsageLedger nudges).
- **Blocks:** nothing hard. Next in the round is the **Phase-7-UI L2 read-only PR Workspace shell** (independent surface). This slice completes whole-cockpit-live (the 6.8-tail).

## Estimated commit count
**1** — a cohesive, mechanical spread (3 effects sharing the identical pattern + the identical test structure; one logical unit = "make the rest of the served cockpit live"). NON-cat-1, read-only, no safety pin. If the recount/fixture work feels large at GREEN, a per-projection 2–3 commit split is acceptable (your call) — but they're one logical unit, so a single commit is preferred.

## Lessons-logged candidates anticipated
- **Convention candidate** — a one-line reinforcement of `LESSONS.md §29`: refetch-on-nudge is now the uniform mechanism across the whole live-relevant served cockpit (Session/ApprovalQueue/ProjectActivity/PullRequest/UsageLedger); the **recount-iff-a-counts-input** discipline; the **AuditTrail blanket-nudge exclusion** (a paged/forensic projection that nudges on every event needs a seq-cursor, not a whole-page refetch).
- **Future TODO (carry-forward / daemon asks):** the AuditTrail seq-cursor delta enrichment (daemon) before AuditTrail can go live; the UsageLedger live producer (daemon P4) — both already tracked.

## How to invoke
1. Read this brief end-to-end — especially the **recount discipline** + the **AuditTrail exclusion**.
2. Confirm RED: `pnpm test src/shell/Shell.subscribe.test.tsx`.
3. `/tdd whole_cockpit_live`.
4. Step 2.5 → the 4 default calls (recount set / AuditTrail exclude / UsageLedger-dormant build / 5-stream aggregation).
5. GREEN → full suite + `/preflight`.
6. Step 9 → the `ui/CLAUDE.md` transport-row update (I write hot) + the AuditTrail/UsageLedger daemon-ask carry-forwards.

# ui-015 — the live ApprovalQueue subscription (refetch-on-nudge + per-stream connection aggregation)

- **Date:** 2026-06-15
- **Phase:** Phase 6 (ui-resume) — **P6.8 / §6.1 + §11.1** (the live `UdsGatewayPort` transport — the 052-Q3 live-delta spread; ApprovalQueue is the first/highest-value served-delta projection)
- **Predecessor:** [ui-014](ui-014-2026-06-15-boundary-merge-0.33-regen.md)
- **Successor:** [ui-016](ui-016-2026-06-16-boundary-merge-regen-0.38.md)
- **Track:** `track/ui` · implementer `ui-implementer` · orchestrator `ui-orchestrator` · lead `team-lead`

## Why this session existed

After the 0.33 boundary-merge regen (ui-014), the cockpit's **approval queue was still load-time only** — it read `ApprovalQueue` once at mount and never updated as the daemon mutated. The daemon publishes an `ApprovalQueue` `ProjectionDelta` post-commit on **every** submit/approve/deny (`gateway/pipeline.rs:79`, verified), but as a **`row:None` id-nudge** (it signals "this projection changed", not the new row). This slice (ui-059) makes the queue live — the cockpit's **action surface** now reflects an approve/deny immediately, not on reload — completing the L2 live loop. Two design constraints reshaped it: consumption must be **refetch-on-nudge** (a coalesced re-read of `get_projection`), NOT the 052 `applySessionDelta` row-apply (which no-ops on `row:None`); and a 2nd live stream must compose with the single connection-state authority (054) via **per-stream aggregation**, never a 2nd writer.

## What was built (2 commits — multi-commit slice)

| Layer | Commit | What | reviewers |
|---|---|---|---|
| **L1** — the testable core | `20bad27` | The per-stream connection aggregation (054 → N-stream worst-of) + the refetch-on-nudge coalescer. The §11.1 fail-safe surface. | security CLEAR · code-quality 2 med fixed-in-slice + 3 low addressed |
| **L2** — the Shell wiring | `db4a246` | The 2nd `ApprovalQueue` subscribe effect (coalesced refetch-on-nudge) + the daemon-shaped `row:None` integration pins. **Approval queue LIVE.** | security CLEAR · code-quality 2 med fixed-in-slice + 2 low addressed/deferred |

### Files created
- `ui/src/gateway-client/refetch-on-nudge.ts` (+ `.test.ts`) — the pure nudge coalescer: at-most-one-in-flight + one-trailing, timer-free (injectable async `read`). For the daemon's `row:None` id-nudges → bounded coalesced re-reads.
- `docs/sessions/ui-015-2026-06-15-approvalqueue-live-subscription.md` (this doc).

### Files modified
- `ui/src/connection/state.ts` (+ `.test.ts`) — NEW pure `worstOfConnection(states)` aggregate (disconnected > reconnecting > connecting > connected; null on empty) — the connection state-machine's natural home (DRY: both ports import it; no drift). + a direct severity-ordering unit pin.
- `ui/src/gateway-client/uds.ts` — `notifyConnectionState(streamId, next)`: a per-stream `streamStates` Map feeds the worst-of aggregate **target**; `streamDegraded` still derives from the **committed** global post-guard (054's rejected-hop guard preserved verbatim).
- `ui/src/gateway-client/mock.ts` — mirrors the per-stream aggregation signature; the `subscribe` now handles `ApprovalQueue` (yields a daemon-shaped `row:None` nudge, stays open).
- `ui/src/gateway-client/types.ts` — the `GatewayPort.notifyConnectionState` signature (`streamId, next`).
- `ui/src/shell/Shell.tsx` — the 2nd `runSubscriptionSupervisor` for `ApprovalQueue` (onDelta → coalescer.nudge(); refetch → re-read + `recountFromApprovals`; setConnection → `notifyConnectionState("ApprovalQueue", …)`); the Session call-site passes `streamId="Session"`.
- `ui/src/projections/fixtures/proj_approval_queue.ts` — NEW `approvalQueueDeltaFixture` (a daemon-shaped `row:None` nudge).
- Test updates: `uds.test.ts` (054 family → 2-arg + the per-stream aggregation pins #3/#4/#5), `mock.test.ts` (2-arg + a multi-stream aggregation pin), `Shell.test.tsx` (the spy → `("Session", …)`), `Shell.subscribe.test.tsx` (the `row:None` refetch pin #1 + the lag-recovery pin #6).

## Decisions made

- **Refetch-on-nudge, not row-apply (the load-bearing reshape).** A daemon `ProjectionDelta` is a `row:None` id-nudge → the live queue re-reads `get_projection`. A row-apply reducer no-ops on `row:None` against the real daemon (the 052 Session reducer only passes because the Mock fixture carried a row). The lead-mandated pin drives a daemon-shaped `row:None` Mock and proves the new approval surfaces ONLY via the re-read.
- **Coalescing = at-most-one-in-flight + one-trailing** (Step-2.5 Q1) — timer-free, deterministic, can't miss the final state; a burst ⇒ ≤2 reads (the daemon isn't hammered). Over a fixed-window debounce (which adds latency + a timer).
- **Per-stream aggregation = worst-of** (Step-2.5 Q2) — disconnected > reconnecting > connecting > connected. **Load-bearing, not cosmetic:** `canSubmitIntent` reads the global `connection`, so any-stream-degraded ⇒ global non-connected ⇒ the §11.1 fail-safe stays FALSE; only the disconnected-vs-reconnecting *label* is cosmetic.
- **`streamDegraded` derives from the COMMITTED global, NOT the per-stream Map** (a corrected refinement of my own Step-2.5 note). Deriving from the Map would have broken the 054 rejected-hop guard (`notify("reconnecting")` from `connecting` is illegal → must stay first-contact-capable). The Map feeds only the aggregate **target**; the committed-derive preserves 054 verbatim over N streams. Orchestrator-approved.
- **`worstOfConnection` lives in `connection/state.ts`** (one file beyond the brief's list, orchestrator-approved) — the connection state-machine module is its natural home; one copy for a fail-safe-critical aggregate (no drift vs duplicating in uds.ts + mock.ts).
- **`security-reviewer` on the connection-aggregation** (Step-2.5 Q3, YES) — it gates `canSubmitIntent`; both layers reviewed CLEAR.
- **Session call-site = `streamId` only; the Session refetch-on-nudge fix is a carry-forward** (Step-2.5 Q5) — not folded here.

## Decisions explicitly NOT made (deferred)

- **The Session refetch-on-nudge fix + the daemon-side Session delta-emission** — the 052 Session "live" is Mock-validated only: it needs BOTH a UI refetch-on-nudge for Session (it still uses `applySessionDelta`, which no-ops on the real daemon's `row:None`) AND a daemon-side Session status-change delta-emission (`deltas_for_append` only fires on `SessionStarted`). Carry-forward (lead surfacing the Session Finding to the user).
- **The other 4 served projections** (ProjectActivity / PullRequest / AuditTrail / UsageLedger) — they have NO daemon delta-emission (only Session + ApprovalQueue emit deltas today, verified) → a daemon follow-on, out of scope.
- **A machine-checkable `parseDelta(approvalQueueDeltaFixture)` assertion** (code-quality low) — already runtime-checked: the Mock's `subscribe` `parseDelta`s it, and the integration tests exercise that path.

## TDD compliance

**Clean.** L1: the coalescer tests (#2/#7) and the per-stream aggregation tests (#3/#4/#5 + the 054 2-arg conversions) were written FIRST — RED confirmed (the coalescer module missing; the 1-arg impl mishandling the 2-arg calls) — then `refetch-on-nudge.ts` / `state.ts` / `uds.ts` / `mock.ts` turned them GREEN. L2: the integration pins (#1 `row:None` refetch + #6 lag-recovery) were written FIRST — RED confirmed (no 2nd subscribe effect → no refetch / no recovery read) — then the Shell effect turned them GREEN. The `worstOfConnection` unit pin + the mock multi-stream pin were added at the Step-8 code-quality review to harden already-green code (standard review-hardening, not test-after-implementation of new behavior). No safety-critical TDD skips.

## Reachability (Step 7.5)

- **L1:** no new reachable production symbol — the coalescer + ApprovalQueue stream had no production caller at L1 (exposed-ahead by design; the Session `streamId` call-site is the only wiring); reviewers confirmed.
- **L2 — LIVE:** `Shell.tsx` mounts a 2nd `useEffect` → `client.subscribe({projection:"ApprovalQueue"})` → (real `UdsGatewayPort`) the `gateway_subscribe` Tauri command → the daemon's post-commit `ApprovalQueue` deltas; each nudge → `coalescer.nudge()` → `get_projection("ApprovalQueue")` (boundary-parsed) → `setData` (approvals + recomputed switcher counts). The connection drive → `notifyConnectionState("ApprovalQueue", …)` → the per-stream worst-of aggregate → the §11.1 `canSubmitIntent` gate. Pin #6 proves the stream reuses the 052 recovery (re-`get_projection` on lag-close). No tested-but-unwired gaps introduced.

## Open follow-ups (Step-9 categorized — already routed hot to the orchestrator)

1. **Cross-doc (orchestrator writes hot at `/orchestrate-end`):** the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport client" subscribe row → multi-stream subscribe + per-stream connection aggregation (any-stream-degraded, worst-of) + refetch-on-nudge for the daemon's `row:None` `ProjectionDelta` contract.
2. **Carry-forward / Finding (orchestrator records; lead surfacing to user):** the 052 **Session** "live" is Mock-validated only — needs the UI Session refetch-on-nudge (a) AND the daemon-side Session status-change delta-emission (b).
3. **Carry-forward:** the other 4 served projections have no daemon delta-emission → daemon follow-on (out of scope).
4. **LESSON candidate (orchestrator records):** "a daemon `ProjectionDelta` is a `row:None` id-NUDGE → consume via coalesced refetch-on-nudge, never a row-apply reducer (which no-ops on the real daemon's `row:None`); a Mock fixture carrying a row masks this — pin `row:None` against a daemon-shaped Mock. A 2nd live stream composes with the single connection authority via per-stream worst-of aggregation, never a 2nd writer."
5. **Note (no action):** code-quality low deferred — the `refetchApprovals`/Session-effect `refetch` setData-after-unmount pattern; the new ApprovalQueue effect's `refetchApprovals` is now `cancelled`-guarded (the Session effect's `refetch` retains the pre-existing unguarded pattern — a harmless no-op in React 18+; future cleanup if desired).

## Cross-doc invariant audit

**Clean (multi-track memory check).** No frozen `shared/` model field changed — ui-059 consumes the frozen `ProjectionDelta` + `ApprovalQueueRow`; the `GatewayPort.notifyConnectionState` signature change is **ui-internal** (not a `shared/` contract). The cross-doc subscribe-row note was flagged at Step 9 (follow-up #1; orchestrator confirmed + writing hot). No un-flagged drift.

## How to use what was built

With a connected + version-compatible daemon, the cockpit's approval queue is now live: when a session submits a mutation (or a human approves/denies), the daemon commits the event and publishes an `ApprovalQueue` nudge; the cockpit coalesced-refetches the snapshot and the waiting-on-you counts + queue update immediately — no reload. If the ApprovalQueue stream degrades, the per-stream worst-of aggregate drops the global connection so `canSubmitIntent` goes fail-safe FALSE (no stale queue shown as live, §11.7); the supervisor recovers via reconnect → re-`get_projection` → re-subscribe. The UI still only reads + submits intents; the daemon Gateway remains the single INV-SEC-1 mutator.

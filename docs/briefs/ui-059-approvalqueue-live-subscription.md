# /tdd brief — approvalqueue_live_subscription

## Feature
Make the cockpit's **approval queue live**: add a 2nd subscribe stream for the `ApprovalQueue` projection so a submit/approve/deny reflects immediately (today the ui subscribes live to **only** `Session` and one-shot `get_projection`s ApprovalQueue at load). The daemon publishes an `ApprovalQueue` delta post-commit on **every** approval mutation (`gateway/pipeline.rs:79`, verified) — but as a `row:None` **id-nudge**, so consumption is **refetch-on-nudge** (re-read `get_projection`, coalesced), NOT the 052 row-apply reducer (which no-ops on `row:None`). Extend the port's connection-state authority to **per-stream aggregation** so the 2nd stream can't mask the 1st's degrade (054 preserved). NON-cat-1.

## Use case + traceability
- **Task ID:** P6.8 (the live `UdsGatewayPort` transport — the 052-Q3 live-delta spread tail; ApprovalQueue is the first/highest-value of the served-delta projections)
- **Architecture sections it implements:** `ARCHITECTURE.md §6.1` (the subscribe + `get_projection` read surface), `§11.1` (the read-only/degraded gate — `canSubmitIntent`), `§11.7` (honest degradation — never stale-as-live), `§6.4` (the `ProjectionDelta` wire)
- **Related context:** slice 052 (`ui/src/gateway-client/{subscribe-recovery,delta-reducer}.ts` + `Shell.tsx:226-273` — the proven Session subscribe machinery this reuses); slice 054 (`ui/src/gateway-client/uds.ts:251-287` — the single connection-state authority + `streamDegraded`); `ui/LESSONS.md` §22/§23/§25. **Verified surfaces (do NOT re-assume):** the daemon emits deltas for **Session + ApprovalQueue ONLY** (`runtime/writer.rs:725` + `gateway/pipeline.rs:79`); both are `row:None` nudges (the documented "re-read via get_projection" contract). The other one-shot projections (ProjectActivity/PullRequest/AuditTrail/UsageLedger) have **no daemon delta emission** → out of scope (daemon follow-on).

## Acceptance criteria (what "done" means)
- [ ] The ui opens a 2nd live subscribe stream for `ApprovalQueue` (a 2nd `runSubscriptionSupervisor`, reusing the 052 supervisor + recovery), in addition to the existing Session stream.
- [ ] **Refetch-on-nudge:** an `ApprovalQueue` delta (even `row:None`) triggers a **coalesced** `get_projection("ApprovalQueue")` re-read that updates `ShellData.approvals` + the derived switcher counts. Rapid nudges coalesce to a bounded number of re-reads (no get_projection spam per burst).
- [ ] **Per-stream connection aggregation:** the port's `notifyConnectionState` becomes per-stream (`streamId`-keyed); `streamDegraded = ANY stream degraded`; the read-path upgrade (`markConnected`) stays suppressed while ANY stream is degraded. A healthy ApprovalQueue stream can NEVER clear a degraded Session stream (and vice-versa) — no second writer; `onConnectionChange` stays the Shell's ONE React connection writer (054).
- [ ] **The row:None nudge test (lead-mandated):** a test drives a **daemon-shaped Mock** (a delta with `row:None`, like the real daemon) through the ApprovalQueue path → the queue updates from the **re-read**, NOT the delta's row. This pins the class of Mock-only-green the Session reducer hit (`sessionDeltaFixture` carried a row the real daemon doesn't).
- [ ] The full ui suite stays green; `tsc --noEmit` + `oxlint` clean; `/preflight` clean.
- [ ] `security-reviewer` runs on the connection-aggregation change (see Step-2.5 Q3 — it touches the §11.1 fail-safe `canSubmitIntent` gate that L2 relies on).
- [ ] Cross-doc + carry-forward flagged at Step 9 (the `ui/CLAUDE.md` subscribe-transport row gains the multi-stream/per-stream-aggregation note; the Session refetch-on-nudge fix + the daemon-side Session delta-emission carry-forward — orchestrator writes).

## Wiring / entry point (Step 7.5)
Production entry: `ui/src/shell/Shell.tsx` — a new `useEffect` runs a 2nd `runSubscriptionSupervisor({ subscribe: () => client.subscribe({ projection: "ApprovalQueue" }), onDelta: <coalesced refetch>, refetch: get_projection("ApprovalQueue"), setConnection: (next) => client.notifyConnectionState("ApprovalQueue", next), … })`. The existing Session effect (`Shell.tsx:238`) updates its call to `notifyConnectionState("Session", next)`. The port (`UdsGatewayPort` + `MockGatewayPort`) is the live reach; the daemon Gateway/event-store is the authoritative source. `/wired`: the ApprovalQueue stream reaches the real `gateway_subscribe` Tauri command (the 052 path, parameterized by projection) → the daemon's post-commit `ApprovalQueue` deltas.

## Files expected to touch
**New:**
- `ui/src/gateway-client/refetch-on-nudge.ts` (or similar) — the pure coalescing helper: an injectable "nudge → at-most-one-in-flight + one-trailing re-read" (timer/microtask-injectable, unit-testable timer-free, the pure-injectable-helper discipline). + its `.test.ts`.

**Modified:**
- `ui/src/gateway-client/uds.ts` — `notifyConnectionState(streamId, next)` per-stream aggregation (`streamStates` map; `streamDegraded = any degraded`; the global `connection` reflects the aggregate). + `.test.ts` (the 054 fail-safe family extended).
- `ui/src/gateway-client/mock.ts` — mirror the `notifyConnectionState(streamId, …)` signature; the Mock `subscribe` accepts `ApprovalQueue` (and yields a **daemon-shaped `row:None`** nudge for the test — see Step-2.5 Q4). + `.test.ts`.
- `ui/src/gateway-client/types.ts` — the `GatewayPort.notifyConnectionState` interface signature (+ `subscribe` already accepts any `ProjectionName`).
- `ui/src/shell/Shell.tsx` — the 2nd subscribe effect (ApprovalQueue refetch-on-nudge) + the Session call's `streamId`. + `Shell.subscribe.test.tsx` extension (the daemon-shaped row:None integration pin).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
1. **`approvalqueue_nudge_refetches_not_row_apply`** (the lead's pin) — Asserts: a `row:None` ApprovalQueue delta drives a `get_projection("ApprovalQueue")` re-read whose rows replace `ShellData.approvals` (the queue reflects the re-read, NOT the absent delta row). Why: `ARCHITECTURE.md §6.1` the daemon's id-nudge→re-read contract (`ui/LESSONS.md` §22/§23; the row:None gap).
2. **`refetch_on_nudge_coalesces_burst`** — Asserts: N rapid nudges → ≤ a bounded number of re-reads (at-most-one-in-flight + one-trailing). Why: don't hammer the daemon (the 052 backoff ethos).
3. **`approvalqueue_stream_degrade_suppresses_read_upgrade`** — Asserts: with the ApprovalQueue stream degraded, a successful ad-hoc read (`markConnected`) does NOT re-assert `connected`. Why: `§11.1`/`§11.7` fail-safe (`ui/LESSONS.md` §25 generalized to N streams).
4. **`healthy_stream_never_clears_other_streams_degrade`** — Asserts: ApprovalQueue `connected` while Session is `disconnected` → the port stays degraded (`streamDegraded` true; `connection` not `connected`). Why: the 054 single-authority must not be masked by a 2nd stream.
5. **`both_streams_healthy_clears_degrade`** — Asserts: all streams `connected` → `streamDegraded` false, read-upgrades allowed again. Why: the aggregate-recovery path.
6. **`approvalqueue_subscribe_recovers_on_lag_close`** — Asserts: the ApprovalQueue stream reuses the 052 reconnect→re-`get_projection`→re-subscribe recovery (degraded-gated). Why: `§11.7` (the 052 supervisor reuse).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/` (consumes the frozen `ProjectionDelta` + `ApprovalQueueRow`). The `GatewayPort.notifyConnectionState` signature change is **ui-internal** (not a `shared/` contract).
- **Orchestrator doc rows to write hot (Step 9):** the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport client" subscribe row — note multi-stream subscribe + the per-stream connection aggregation (any-stream-degraded) + refetch-on-nudge for the `row:None` daemon contract.
- **2.5-seam (shared-contract) model touched?** No — no NEW/extended ui-authored invariant on a subsystem-boundary model; the change is ui-side consumption + the ui-local connection authority.

## Things to flag at Step 2.5
1. **Coalescing semantics — at-most-one-in-flight + one-trailing, or a fixed debounce window?** My default vote: **at-most-one-in-flight + one-trailing re-read** (a pending-refetch flag; if a nudge arrives during an in-flight re-read, do exactly one more after) — no wall-clock timer, deterministically testable, and it can't miss the final state. A fixed debounce adds latency + a timer.
2. **Per-stream aggregation — global-connection precedence when streams disagree.** When one stream is `disconnected` and another `reconnecting`, what does the global banner show? My default vote: **worst-of precedence** (`disconnected` > `reconnecting` > `connected`); the load-bearing invariant is only "any degraded → suppress upgrade," the exact banner state is cosmetic. Confirm `canTransition` legality still holds on the driven hops.
3. **`security-reviewer` on the connection-aggregation?** My default vote: **YES, run it** — `notifyConnectionState`/`streamDegraded` is the §11.1 fail-safe gate that makes `canSubmitIntent` FALSE on a degraded stream (the L2 defense-in-depth, `ui/LESSONS.md` §4/§25). A regression that let a healthy stream clear another's degrade would re-open the 052 masking (an INV-SEC-1-adjacent fail-safe). Treat the connection change as invariant-touching.
4. **The daemon-shaped Mock nudge — `row:None` for the test.** The current `sessionDeltaFixture` carries a row (which masked the gap). My default vote: the Mock's ApprovalQueue subscribe yields a **`row:None`** nudge (real-daemon-shaped) so test #1 proves refetch-on-nudge against the actual daemon contract; keep a row-bearing fixture only if a separate test needs it.
5. **Session call-site — update its `streamId` only, do NOT change its onDelta this slice.** My default vote: pass `"Session"` to `notifyConnectionState` (required by the new signature) but leave Session's `applySessionDelta` onDelta untouched — the Session refetch-on-nudge fix is a **carry-forward** (lead-ruled), not folded here.

## Dependencies + sequencing
- **Depends on:** 052 (the subscribe supervisor + recovery), 054 (the single connection-state authority), ui-058 (0.33 contract — landed).
- **Blocks:** nothing hard. The Session refetch-on-nudge fix + the daemon-side Session/other-projection delta-emission are the follow-ons (carry-forward).

## Estimated commit count
**2** (multi-commit slice, `ui/LESSONS.md` §7 — orchestrator drives layer→layer): **(1)** the per-stream connection aggregation (`uds.ts`/`mock.ts`/`types.ts` + the refetch-on-nudge coalescing helper) — the testable core + the `security-reviewer` surface; **(2)** the Shell 2nd-subscription wiring + the daemon-shaped row:None integration pin. NON-cat-1; no safety-critical pin gets bundled (the connection-aggregation is its own commit so the reviewer surface is clean).

## Lessons-logged candidates anticipated
- **Convention candidate** — "a daemon `ProjectionDelta` is a `row:None` id-NUDGE → consume via coalesced refetch-on-nudge (re-read `get_projection`), NEVER a row-apply reducer (which no-ops on the real daemon's row:None); a Mock fixture carrying a row masks this → pin the row:None path against a daemon-shaped Mock." + "a 2nd live stream composes with the single connection authority via per-stream aggregation (any-stream-degraded), never a 2nd writer."
- **Finding / carry-forward (orchestrator records)** — the 052 Session "live" is Mock-validated, not real-daemon: it needs BOTH the UI refetch-on-nudge AND a daemon-side Session status-change delta-emission (`deltas_for_append` only fires on `SessionStarted`). The daemon half routes with the other daemon follow-ons.

## How to invoke
1. **Read this brief end-to-end** — especially Step-2.5 Q1 (coalescing) + Q3 (security-reviewer on the fail-safe gate) + Q4 (the row:None daemon-shaped Mock).
2. **Confirm the verified surfaces:** the daemon emits ApprovalQueue deltas (`gateway/pipeline.rs:79`) as `row:None`; the ui currently subscribes only Session (`Shell.tsx:238`).
3. **Run `/tdd approvalqueue_live_subscription`.**
4. **Step 2.5** — send the coverage map (each acceptance bullet → its test) + the 5 design calls. Layer-1 (connection aggregation + coalescing helper) RED first.
5. **Step 9** — surface the cross-doc subscribe row + the Session-gap Finding/carry-forward.

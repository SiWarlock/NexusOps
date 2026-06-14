# /tdd brief — connection_state_single_authority

## Feature
**L2-prep, the two-writer `connection`-state reconcile (NON-cat-1; resolves the 052 routed Finding) —
a hard pre-L2 gate.** Today the Shell's `connection` React state has **two independent writers** that
are never reconciled: (1) the **read path** — `client.onConnectionChange(setConnection)`
(`Shell.tsx:152`), where `UdsGatewayPort.markConnected/markDisconnected` fire on read success/fault;
and (2) the **subscribe supervisor** — `runSubscriptionSupervisor`'s `setConnection` dep
(`Shell.tsx:243-244`), which drives `disconnected/reconnecting/connected` from the live stream's
health straight into the SAME React setter. A successful unrelated single-shot read (e.g. a `get_diff`
from the Code view) fires `markConnected` and **momentarily masks a supervisor stream-degrade** →
`canSubmitIntent` reads `true` while the live subscription is actually down — the exact inversion the
fail-safe gate forbids (forbidden #6 / LESSON 4). Collapse to a **single connection-state authority**
(the port): the supervisor drives the port via a port method (not a second raw React `setConnection`),
the read-path **upgrade** is suppressed while the stream is supervisor-degraded (fail-safe DEGRADE
preserved), and the Shell has exactly ONE writer into React connection state (`onConnectionChange`).
**NON-cat-1** (tightens a defense-in-depth gate — no new mutation surface; the daemon Gateway stays
the load-bearing INV-SEC-1 chokepoint), but **`security-reviewer` REQUIRED** (it touches the
`canSubmitIntent` fail-safe safety gate — the `invariant` policy).

## Use case + traceability
- **Task ID:** P6.8 L2-prep (the 052 connection-state Finding; lead-RULED reconcile-before-L2; NON-cat-1)
- **Architecture sections it implements:** `ARCHITECTURE.md §11.4` (the global READ-ONLY/degraded gate — fail-safe `canSubmitIntent`), `§11.1` (degraded display surface / never stale-as-live), `§6.1` (the `GatewayPort` transport-liveness surface; NOT the frozen RPC set). _(INV-SEC-1 — the load-bearing mutation enforcement — is daemon-side; this UI gate is defense-in-depth only.)_
- **Reference:**
  - **The Finding:** `docs/planning/052-connection-state-two-writer-finding.md` (the full statement + the lead ruling: reconcile-before-L2 as its own slice; the entanglement with the `MockGatewayPort` degraded-banner contract).
  - **The two writers:** `ui/src/shell/Shell.tsx:152` (`onConnectionChange(setConnection)`) + `Shell.tsx:233-244` (the supervisor's `setConnection` dep). **The port authority:** `ui/src/gateway-client/uds.ts:238-253` (`markConnected`/`markDisconnected`/`setConnection`, the guarded transition). **The supervisor:** `ui/src/gateway-client/subscribe-recovery.ts` (pure orchestration over an injected `setConnection: (state)=>void` dep — keep it pure; only the Shell BINDING changes). **The machine:** `ui/src/connection/state.ts` (`canTransition`). **The gate:** `ui/src/connection/read-only.ts` (`canSubmitIntent` = connected + version-compatible). **The mock contract:** `ui/src/gateway-client/mock.ts:208-221` (`reconnect` + the **raw** `setConnectionState` test-staging setter — KEEP it raw for the §11 DegradedBanner tests).
  - LESSON 4 (the fail-safe read-only gate — FALSE on unknown/degraded; defense-in-depth, never the sole guard), LESSON 22/23 (the read path + subscribe transport drive the port's connection state), forbidden #6 (never offer an intent control without `canSubmitIntent`).

## Acceptance criteria (what "done" means)
- [ ] **Single React writer.** The Shell sets its `connection` React state from EXACTLY ONE source — the port's `onConnectionChange`. The subscribe supervisor no longer calls a raw React `setConnection`; its `setConnection` dep is bound to a **port method** (`Shell.tsx:243-244` no longer writes React state directly). Pin: a Shell-level test that drives the supervisor's degrade and asserts it reaches React state THROUGH the port (`onConnectionChange`), not a second setter.
- [ ] **The masking is closed (the core fix).** With the subscribe stream supervisor-degraded (it drove `disconnected`/`reconnecting`), a subsequent **successful single-shot read** does NOT flip the exposed connection to `connected` / `canSubmitIntent` to `true`. The read-path UPGRADE is suppressed while the stream is supervisor-degraded. Pin both: (a) stream-degrade → read-success → still degraded; (b) the control case — no stream-degrade → read-success → connected (the normal initial-connect path still works).
- [ ] **Fail-safe DEGRADE preserved (both axes).** A read transport fault still degrades (`markDisconnected` → `canSubmitIntent` false). A subscribe stream end/fault still degrades. The gate stays fail-safe-FALSE on any unknown/degraded (LESSON 4).
- [ ] **Recovery still returns to connected.** After the supervisor recovers (re-subscribe + `refetch` succeed → it asserts `connected`), the exposed connection returns to `connected` and `canSubmitIntent` is `true` (when version-compatible). The read-upgrade suppression clears once the stream is healthy again.
- [ ] **The `MockGatewayPort` degraded-banner contract is intact.** The §11 `DegradedBanner` tests (which stage connection states via the mock's **raw** `setConnectionState`) still pass unchanged. The mock implements the new port method (guarded transition + notify), and `setConnectionState` stays the raw, unguarded test-staging setter. No `DegradedBanner.tsx` / §11 contract change.
- [ ] **`GatewayPort` interface gains the supervisor's drive method** (`types.ts`) — both `UdsGatewayPort` and `MockGatewayPort` implement it. Flag at Step 9 (a UI-local `GatewayPort` surface change — NOT a frozen `shared/` contract; no schema-snapshot).
- [ ] **`security-reviewer` REQUIRED:** single authority (no second writer can mask a degrade); the read-upgrade suppression is correct (a read can never assert connected over a degraded stream); fail-safe degrade preserved on both axes; no new mutation surface reachable (the L2 methods still throw not-wired).
- [ ] Whole suite green (337 + the reconcile pins); `/preflight` clean; the cross-doc flag at Step 9.

## Wiring / entry point (Step 7.5)
**REAL, live now (L1).** The running cockpit ALWAYS starts the subscribe supervisor (`Shell.tsx:221`
useEffect) AND the read-path connection wiring (`Shell.tsx:152`). After the reconcile: supervisor →
`client.<driveMethod>(next)` → the port's guarded `setConnection` → `onConnectionChange` → the SINGLE
React `setConnection` → `ReadOnlyProvider` value → `canSubmitIntent`. `/wired`: the supervisor's
connection drive now traces THROUGH the port (`uds.ts` `setConnection`), not a raw React setter in
`Shell.tsx`. The exposed `canSubmitIntent` is the live gate (load-bearing at L2; defense-in-depth now).

## Files expected to touch
**Modified:**
- `ui/src/gateway-client/types.ts` — add the supervisor's connection-drive method to the `GatewayPort` interface (the port becomes the single authority).
- `ui/src/gateway-client/uds.ts` — implement the drive method (route through the guarded `setConnection`); add the **read-upgrade suppression while stream-degraded** (the streamDegraded axis — set by the supervisor's drive, gating `markConnected`). `+ uds.test.ts` (the suppression + the recovery-clear pins).
- `ui/src/gateway-client/mock.ts` — implement the drive method (guarded transition + notify); KEEP `setConnectionState` raw for test staging. `+ mock.test.ts` if touched.
- `ui/src/shell/Shell.tsx` — bind the supervisor's `setConnection` dep to the port method; the Shell's only React-state connection writer stays `onConnectionChange(setConnection)`; drop the now-unneeded inline `canTransition` guard at the supervisor wiring (the port owns the guard). `+ Shell.*.test.tsx` (the single-writer + masking-closed pins).

If `subscribe-recovery.ts` needs a shape change (it should NOT — keep the supervisor pure over its injected `setConnection` dep; only the Shell binding changes), **flag at Step 2.5**.

## RED test outline (Step 2)
1. `read_success_does_not_upgrade_while_stream_degraded` — supervisor drives `disconnected`; then a read succeeds (`markConnected`) → exposed connection stays degraded, `canSubmitIntent` false. — Asserts: the masking is closed (§11.4 fail-safe / forbidden #6).
2. `read_success_upgrades_when_stream_healthy` — no supervisor degrade → a read success → `connected` (the normal initial-connect path). — Asserts: the control case (no false degrade; LESSON 22).
3. `read_fault_still_degrades` — a read transport fault → `markDisconnected` → degraded, `canSubmitIntent` false. — Asserts: fail-safe DEGRADE preserved (LESSON 4).
4. `stream_recovery_returns_to_connected` — supervisor degrades then asserts `connected` (recovery) → exposed `connected`; a subsequent read success is no longer suppressed. — Asserts: recovery + suppression-clear (§11.7).
5. `shell_has_single_connection_writer` — driving the supervisor's degrade reaches React state THROUGH the port's `onConnectionChange` (not a second raw setter). — Asserts: single authority (the Finding's core).
6. `mock_degraded_banner_contract_intact` — the §11 `DegradedBanner` staging via the mock's raw `setConnectionState` still renders the degraded surface; the mock's new drive method applies the guarded transition. — Asserts: the mock contract is preserved (the entanglement the Finding named).
7. `l2_mutation_methods_still_throw_not_wired` — the reconcile adds no mutation reach. — Asserts: INV-SEC-1 / L2-HELD.
Each carries `Asserts: <invariant> (§anchor)`; the coverage map ties each acceptance bullet to a test.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none in `shared/` (no frozen contract touched). The `GatewayPort` interface (UI-local) gains a transport-liveness method — **not** a frozen shared-contract (cross-area) model, so **no schema-snapshot test**.
- **Orchestrator doc rows (Step 9):** the `ui/CLAUDE.md` "Live `UdsGatewayPort` transport client" row → note the connection-state single-authority reconcile (the 052 Finding RESOLVED; the supervisor drives the port, read-upgrade gated while stream-degraded) + likely a LESSON (the single-authority + fail-safe-asymmetric-upgrade pattern). No `ARCHITECTURE.md` edit (the §11.4 gate semantics are unchanged — this makes the implementation match the spec).
- **Shared-contract (cross-area) model touched?** No (the `GatewayPort` connection surface is UI-local; no `shared/` schema).

## Things to flag at Step 2.5
1. **The authority model (the load-bearing design choice).** How does the single authority close the masking? My default vote: **the port holds the single `ConnectionState` + a `streamDegraded` axis** — the supervisor's drive method sets `streamDegraded` (true on `disconnected`/`reconnecting`, false on `connected`) and drives `setConnection`; the read path's `markConnected` is **suppressed while `streamDegraded`** (reads still `markDisconnected` on fault — fail-safe). Minimal extra state, preserves the existing `ConnectionState` machine + the §11 tests, and the initial connect still works (no degrade asserted at startup). **Alternative (more explicit, more code):** a full **two-axis worst-wins** — the port tracks `readLiveness` + `streamLiveness` separately and DERIVES the exposed `ConnectionState` as the fail-safe combination (degraded if either is; connected only if both confirm). Pick the alternative only if the flag-guard reads as implicit. Flag your choice.
2. **The drive-method shape.** Default: ONE method `notifyConnectionState(next: ConnectionState): void` on `GatewayPort` (the supervisor reports its computed lifecycle state; the port infers `streamDegraded` from the arg). Alternative: semantic methods (`onStreamDegraded()`/`onStreamReconnecting()`/`onStreamLive()`). Default vote: **the single `notifyConnectionState`** — it keeps `subscribe-recovery.ts` pure + UNCHANGED (its `setConnection` dep just binds to this method in the Shell). Flag if semantic methods read cleaner.
3. **Keep the supervisor pure.** Default: `runSubscriptionSupervisor` keeps its injected `setConnection: (state)=>void` dep — ONLY the Shell binding changes (bind to the port method instead of raw React state). Do NOT entangle the supervisor with the port directly (it must stay unit-testable with a fake setter). Flag if you see a reason to change its shape.
4. **The mock's two setters.** Default: the mock's existing `setConnectionState` stays the **raw, unguarded** test-staging setter (the §11 DegradedBanner contract); the NEW `notifyConnectionState` applies the guarded transition (mirrors the real port). Two methods, two purposes. Flag if collapsing them is cleaner without breaking the staging tests.

## Dependencies + sequencing
- **Depends on:** 052 (the subscribe supervisor + the read-path connection wiring — both landed) + the live `UdsGatewayPort` (L1 ✅).
- **Blocks:** **L2 (cat-1)** — L2 makes `canSubmitIntent` load-bearing (a real human approving a real, accurately risk-classified action), so the gate must be single-authority + fail-safe-correct BEFORE the mutation path goes live. This is one of the two pre-L2 NON-cat-1 slices (alongside 053b); after both land, the L2 cat-1 checkpoint escalates.

## Estimated commit count
**1** (the focused single-authority reconcile — one concern: the connection-state writer). **NON-cat-1** (no new mutation surface; tightens a defense-in-depth gate) — **`security-reviewer` REQUIRED** (the `canSubmitIntent` fail-safe safety surface; the `invariant` policy).

## Lessons-logged candidates anticipated
- **Convention candidate** — the connection state has a **single authority** (the port): every liveness signal (read success/fault, subscribe stream health) flows through the port's one guarded `setConnection`; the Shell has exactly one React writer (`onConnectionChange`). The fail-safe asymmetry: a signal may DEGRADE freely, but an UPGRADE to `connected` is suppressed while a stronger continuous signal (the subscribe stream) is degraded — so an ad-hoc read can never mask a down stream (forbidden #6 / LESSON 4 hardened). Extends LESSON 22/23.
- **Architecture-doc note candidate** — the §11.4 gate semantics are unchanged; this makes the implementation single-authority so the spec actually holds under concurrent read + stream signals. The only L2 blockers left after this + 053b are the cat-1 checkpoint itself.

## How to invoke
1. **Read this brief end-to-end** — the two-writer Finding + the authority model + the 4 Step-2.5 questions.
2. Pre-flight: `track/ui` (053 landed; 0.31.0; 337 green). Same session — no `/session-start`.
3. **Run `/tdd connection_state_single_authority`**.
4. Step 0/1 — confirm Feature + Files.
5. **Step 2.5** — answer the 4 questions (esp. #1 the authority model) + send the test-design write-up + coverage map; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 8** — `security-reviewer` REQUIRED (the fail-safe gate; single authority; read-upgrade suppression correctness).
7. Step 9 — the cross-doc flag (the 052 Finding RESOLVED; the `ui/CLAUDE.md` transport row note + the LESSON) — then the L2 cat-1 checkpoint escalates (I author that one only after the file-based checkpoint to the lead).

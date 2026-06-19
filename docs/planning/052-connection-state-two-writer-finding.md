# Finding — the two-writer `connection`-state race (052; gates L2)

> **Orchestrator-routed Step-9 Finding** (escalation category #2) for `ui-team-lead` → user.
> Surfaced by the 052 `security-reviewer` whole-boundary pass (rated **MED**; the slice was
> CLEARED on all 5 streaming invariants). **NON-blocking for 052 / L1** (reads-only, non-exploitable
> now) — routed because it touches the `canSubmitIntent` safety surface that **L2 (cat-1) makes
> load-bearing.** The lead rules the sequencing + surfaces to the user.

## What it is (plain language)

The UI tracks one "is the daemon connection healthy?" flag (`connection` in `Shell.tsx`, which
drives `canSubmitIntent` — the fail-safe gate that disables every mutation control when the daemon
is unreachable/degraded). After 052 there are now **two independent writers** to that flag:

1. **The read-path writer** — `client.onConnectionChange(setConnection)`: the `UdsGatewayPort`
   marks the connection `connected` whenever any single-shot READ succeeds (e.g. a `get_diff` from
   the Code/Diff view), `disconnected` on a transport fault. (Pre-existing since 051.)
2. **The supervisor writer** — 052's `runSubscriptionSupervisor` drives `connected/disconnected/
   reconnecting` from the **subscribe stream's** health (close-on-lag → degraded → recovery).

These two views are **not reconciled.** A successful unrelated read can fire `markConnected` and
**momentarily mask a supervisor stream-degrade** → `canSubmitIntent` reads `true` while the live
subscription is actually down.

## Why it matters

- It weakens the **fail-safe `canSubmitIntent` gate** (root `ui/CLAUDE.md` forbidden #6 / LESSON §4 —
  the gate must be FALSE on any unknown/degraded, true only when positively confirmed
  connected + version-compatible). A read masking a stream-degrade is exactly the inversion the
  fail-safe default forbids.
- The gate is **defense-in-depth**, not the real chokepoint — the **daemon Action Gateway is the
  load-bearing INV-SEC-1 enforcement** (§15). So a UI-gate flap does not by itself let a bad
  mutation through.

## Why it is NOT exploitable today

- **052 is reads-only.** The mutation methods (`submit_action`/`approve`/`deny`) are **un-wired**
  (L2 cat-1 HELD) — there is **no production path that submits a mutation** through the UI yet
  (pinned: `subscribe_mutation_methods_still_throw_not_wired`). A flapping `canSubmitIntent` gates
  controls that cannot reach a mutation regardless.
- Even when wired, the daemon Gateway re-validates every intent server-side (the gate is
  defense-in-depth, never the sole guard).

## Why it was NOT fixed in 052

The clean fix is a **single connection-state authority** — the `UdsGatewayPort` owns transport
liveness; the supervisor orchestrates *via port methods* (not a second raw `setConnection`), and a
subscribe-stream end marks the **port** degraded (one writer, one source of truth). That refactor is
**entangled with the `MockGatewayPort`'s degraded-banner test contract** (the mock simulates
connection state for the §11 degraded-surface tests), so doing it correctly is its own small slice —
**not** a hasty refactor folded into the L1-closing commit (correct > expedient; rushing it risks
the degraded-banner contract the §11 tests pin).

## Recommendation

**Reconcile the connection-state authority as a dedicated small slice BEFORE L2** — slot it between
the 0.30.0 regen slice (sequence step 2) and the L2 cat-1 transport (step 3). Rationale: L2 is the
slice that makes `canSubmitIntent` **load-bearing** (a real human approving a real, accurately
risk-classified action), so the gate must be single-authority + fail-safe-correct *before* the
mutation path goes live. The reconcile slice is NON-cat-1 (it tightens a defense-in-depth gate; no
new mutation surface) and naturally precedes the L2 cat-1 checkpoint I escalate separately.

## Ruling requested (lead → user)

1. **Sequencing:** ratify "reconcile-before-L2 as its own slice" (orchestrator + implementer rec), OR
   direct an alternative (fix-before-the-L1-push / fold-into-L2 / defer-with-explicit-acceptance).
2. The Finding is **logged** here + in the `ui-010` session doc Open follow-ups + the
   `IMPLEMENTATION_PLAN.md` carry-forward (consumer-marked `last-consumer-slice: the pre-L2
   connection-state reconcile slice`) — so it is not silently dropped regardless of the ruling.

## References

- 052 brief: `docs/briefs/052-P6-L1-subscribe-streaming-and-reconnect-recovery.md`
- The gate: `ui/src/connection/` + `ui/src/shell/Shell.tsx` (`onConnectionChange` + the supervisor wiring) + `ui/src/gateway-client/uds.ts` (`setConnection`/`markConnected`/`reconnect`).
- The invariants: `ui/CLAUDE.md` forbidden #6 + LESSON §4 ([[4]]) + §11.1/§11.4 (the degraded gate) + §15 (INV-SEC-1, daemon-side).
- The L2 cat-1 rulings it protects: `docs/planning/intent-seam-cat1-safety-design.md` (Q2 — the `canSubmitIntent` fail-safe, defense-in-depth caveat).
